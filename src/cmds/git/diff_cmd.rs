//! Compares two files and shows only the changed lines.

use crate::core::guard::never_worse;
use crate::core::tracking;
use anyhow::Result;
use std::fs;
use std::path::Path;

/// Ultra-condensed diff - only changed lines, no context.
/// Returns the diff-convention exit code: 0 if identical, 1 if files differ.
pub fn run(file1: &Path, file2: &Path, verbose: u8) -> Result<i32> {
    let timer = tracking::TimedExecution::start();

    if verbose > 0 {
        eprintln!("Comparing: {} vs {}", file1.display(), file2.display());
    }

    let content1 = fs::read_to_string(file1)?;
    let content2 = fs::read_to_string(file2)?;
    let raw = format!("{}\n---\n{}", content1, content2);

    let (rtk, exit_code) = render_file_diff(file1, file2, &content1, &content2);

    let shown = never_worse(&raw, &rtk);
    print!("{}", shown);
    timer.track(
        &format!("diff {} {}", file1.display(), file2.display()),
        "rtk diff",
        &raw,
        shown,
    );
    Ok(exit_code)
}

/// Renders the condensed file comparison and returns it with the
/// diff-convention exit code (0 = identical, 1 = differences found).
fn render_file_diff(file1: &Path, file2: &Path, content1: &str, content2: &str) -> (String, i32) {
    // Byte equality is the only safe basis for claiming identity, and it must be
    // checked before `lines()` touches the input. `str::lines()` strips a
    // trailing `\r` and treats the final newline as optional, so a CRLF-vs-LF or
    // missing-trailing-newline difference collapses to identical line vectors.
    // Reporting "identical" with exit 0 for files that differ silently passes
    // any verification gate built on `diff`.
    if content1 == content2 {
        return ("[ok] Files are identical\n".to_string(), 0);
    }

    let lines1: Vec<&str> = content1.lines().collect();
    let lines2: Vec<&str> = content2.lines().collect();
    let diff = compute_diff(&lines1, &lines2);

    if diff.changes.is_empty() {
        // Bytes differ, lines don't: the difference is exactly what `lines()`
        // normalizes away. Name it rather than rendering an empty change list.
        return (
            describe_invisible_difference(file1, file2, content1, content2),
            1,
        );
    }

    let mut rtk = String::new();
    rtk.push_str(&format!("{} → {}\n", file1.display(), file2.display()));
    rtk.push_str(&format!(
        "   +{} added, -{} removed, ~{} modified\n\n",
        diff.added, diff.removed, diff.modified
    ));
    rtk.push_str(&format_diff_changes(&diff));
    (rtk, 1)
}

/// Describe a difference that a line-based diff cannot see.
///
/// Reached only when the bytes differ but `lines()` yields identical vectors,
/// which narrows the cause to the two things `lines()` normalizes: a `\r`
/// before the newline, and the presence of the final newline. Rendering the
/// usual `~ 12 foo → foo` change list here would show two visually identical
/// strings, so state the cause instead.
fn describe_invisible_difference(
    file1: &Path,
    file2: &Path,
    content1: &str,
    content2: &str,
) -> String {
    let crlf1 = content1.matches("\r\n").count();
    let crlf2 = content2.matches("\r\n").count();
    let nl1 = content1.ends_with('\n');
    let nl2 = content2.ends_with('\n');

    let mut notes: Vec<String> = Vec::new();
    if crlf1 != crlf2 {
        notes.push(format!(
            "line endings: {} CRLF vs {} CRLF",
            crlf1, crlf2
        ));
    }
    if nl1 != nl2 {
        notes.push(format!(
            "trailing newline: {} vs {}",
            if nl1 { "present" } else { "absent" },
            if nl2 { "present" } else { "absent" }
        ));
    }
    if notes.is_empty() {
        // Defensive: no known `lines()` normalization explains it. Say so
        // rather than implying the files match.
        notes.push(format!(
            "{} vs {} bytes, invisible to a line-based diff",
            content1.len(),
            content2.len()
        ));
    }

    // Deliberately avoids the word "identical". That string is the signal for
    // the true-identity case, and a reader (or grep) scanning for it must not
    // match a report about files that differ.
    format!(
        "{} → {}\n   differs, text matches ({})\n",
        file1.display(),
        file2.display(),
        notes.join("; ")
    )
}

/// Run diff from stdin (piped command output)
pub fn run_stdin(_verbose: u8) -> Result<()> {
    use std::io::{self, Read};
    let timer = tracking::TimedExecution::start();

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    // Parse unified diff format
    let condensed = condense_unified_diff(&input);
    let shown = never_worse(&input, &condensed);
    println!("{}", shown);

    timer.track("diff (stdin)", "rtk diff (stdin)", &input, shown);

    Ok(())
}

#[derive(Debug)]
enum DiffChange {
    Added(usize, String),
    Removed(usize, String),
    Modified(usize, String, String),
}

struct DiffResult {
    added: usize,
    removed: usize,
    modified: usize,
    changes: Vec<DiffChange>,
}

fn format_diff_changes(diff: &DiffResult) -> String {
    let mut out = String::new();
    for change in &diff.changes {
        match change {
            DiffChange::Added(ln, c) => out.push_str(&format!("+{:4} {}\n", ln, c)),
            DiffChange::Removed(ln, c) => out.push_str(&format!("-{:4} {}\n", ln, c)),
            DiffChange::Modified(ln, old, new) => {
                out.push_str(&format!("~{:4} {} → {}\n", ln, old, new))
            }
        }
    }
    out
}

/// One edit-script step, carrying a 1-based line number in its own file.
enum Op {
    Del(usize, String),
    Ins(usize, String),
}

/// Cap on LCS table cells (rows × cols). At 4M cells the `u32` table is 16MB,
/// which is the most we will spend to align two files. Past it the trimmed
/// middles share so little that any alignment is noise anyway, so we emit a
/// wholesale replacement rather than allocate a multi-gigabyte table.
const LCS_CELL_CAP: usize = 4_000_000;

/// Diff two line sequences by longest-common-subsequence.
///
/// The previous implementation compared **positionally** (`lines1[i]` vs
/// `lines2[i]` for i in 0..max_len), so a single inserted line desynchronized
/// every line after it: each subsequent pair compared unrelated lines, the whole
/// tail rendered as changed, and the output grew large enough that the
/// `never_worse` guard discarded it and dumped both files concatenated instead
/// of showing one insertion.
fn compute_diff(lines1: &[&str], lines2: &[&str]) -> DiffResult {
    // Common prefix and suffix carry no information and dominate real diffs.
    // Trimming them first is what keeps the LCS table affordable.
    let mut lo = 0usize;
    while lo < lines1.len() && lo < lines2.len() && lines1[lo] == lines2[lo] {
        lo += 1;
    }
    let mut hi1 = lines1.len();
    let mut hi2 = lines2.len();
    while hi1 > lo && hi2 > lo && lines1[hi1 - 1] == lines2[hi2 - 1] {
        hi1 -= 1;
        hi2 -= 1;
    }

    let a = &lines1[lo..hi1];
    let b = &lines2[lo..hi2];

    let ops = if a.len().saturating_mul(b.len()) > LCS_CELL_CAP {
        let mut v = Vec::with_capacity(a.len() + b.len());
        for (i, l) in a.iter().enumerate() {
            v.push(Op::Del(lo + i + 1, (*l).to_string()));
        }
        for (j, l) in b.iter().enumerate() {
            v.push(Op::Ins(lo + j + 1, (*l).to_string()));
        }
        v
    } else {
        lcs_ops(a, b, lo)
    };

    ops_to_changes(ops)
}

/// Backtrack an LCS table into an edit script. `offset` maps middle-relative
/// indices back to real file line numbers after prefix trimming.
fn lcs_ops(a: &[&str], b: &[&str], offset: usize) -> Vec<Op> {
    let n = a.len();
    let m = b.len();
    if n == 0 || m == 0 {
        let mut v = Vec::with_capacity(n + m);
        for (i, l) in a.iter().enumerate() {
            v.push(Op::Del(offset + i + 1, (*l).to_string()));
        }
        for (j, l) in b.iter().enumerate() {
            v.push(Op::Ins(offset + j + 1, (*l).to_string()));
        }
        return v;
    }

    // dp[i][j] = LCS length of a[i..] and b[j..]
    let stride = m + 1;
    let mut dp = vec![0u32; (n + 1) * stride];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i * stride + j] = if a[i] == b[j] {
                dp[(i + 1) * stride + (j + 1)] + 1
            } else {
                dp[(i + 1) * stride + j].max(dp[i * stride + (j + 1)])
            };
        }
    }

    let mut ops = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < n && j < m {
        if a[i] == b[j] {
            i += 1;
            j += 1;
        } else if dp[(i + 1) * stride + j] >= dp[i * stride + (j + 1)] {
            ops.push(Op::Del(offset + i + 1, a[i].to_string()));
            i += 1;
        } else {
            ops.push(Op::Ins(offset + j + 1, b[j].to_string()));
            j += 1;
        }
    }
    while i < n {
        ops.push(Op::Del(offset + i + 1, a[i].to_string()));
        i += 1;
    }
    while j < m {
        ops.push(Op::Ins(offset + j + 1, b[j].to_string()));
        j += 1;
    }
    ops
}

/// Fold the edit script into the reported change list, pairing each run of
/// deletions with the run of insertions that follows it so a rewritten line
/// still reads as one `Modified` rather than a remove plus an unrelated add.
fn ops_to_changes(ops: Vec<Op>) -> DiffResult {
    let mut changes = Vec::new();
    let (mut added, mut removed, mut modified) = (0usize, 0usize, 0usize);

    let mut k = 0usize;
    while k < ops.len() {
        let del_start = k;
        while matches!(ops.get(k), Some(Op::Del(..))) {
            k += 1;
        }
        let dels = &ops[del_start..k];

        let ins_start = k;
        while matches!(ops.get(k), Some(Op::Ins(..))) {
            k += 1;
        }
        let inss = &ops[ins_start..k];

        let pairs = dels.len().min(inss.len());
        for p in 0..pairs {
            let (dline, dtext) = match &dels[p] {
                Op::Del(l, t) => (*l, t.as_str()),
                Op::Ins(..) => unreachable!("del run holds only deletions"),
            };
            let itext = match &inss[p] {
                Op::Ins(_, t) => t.as_str(),
                Op::Del(..) => unreachable!("ins run holds only insertions"),
            };
            if similarity(dtext, itext) > 0.5 {
                changes.push(DiffChange::Modified(
                    dline,
                    dtext.to_string(),
                    itext.to_string(),
                ));
                modified += 1;
            } else {
                changes.push(DiffChange::Removed(dline, dtext.to_string()));
                changes.push(DiffChange::Added(dline, itext.to_string()));
                removed += 1;
                added += 1;
            }
        }
        for d in dels.iter().skip(pairs) {
            if let Op::Del(l, t) = d {
                changes.push(DiffChange::Removed(*l, t.clone()));
                removed += 1;
            }
        }
        for ins in inss.iter().skip(pairs) {
            if let Op::Ins(l, t) = ins {
                changes.push(DiffChange::Added(*l, t.clone()));
                added += 1;
            }
        }
    }

    DiffResult {
        added,
        removed,
        modified,
        changes,
    }
}

fn similarity(a: &str, b: &str) -> f64 {
    let a_chars: std::collections::HashSet<char> = a.chars().collect();
    let b_chars: std::collections::HashSet<char> = b.chars().collect();

    let intersection = a_chars.intersection(&b_chars).count();
    let union = a_chars.union(&b_chars).count();

    if union == 0 {
        1.0
    } else {
        intersection as f64 / union as f64
    }
}

/// `@@ -old_start[,old_count] +new_start[,new_count] @@` -> (old_count, new_count).
///
/// `None` means "no budget": the header is a combined diff (`@@@ … @@@`, from
/// `git show --cc`, whose extra parent column this does not model) or is
/// malformed. Callers fall back to the marker rule for those.
fn parse_hunk_counts(line: &str) -> Option<(usize, usize)> {
    // `@@@` fails this prefix, which is what routes combined diffs to `None`.
    let mut parts = line.strip_prefix("@@ ")?.split_whitespace();
    let old = parts.next()?.strip_prefix('-')?;
    let new = parts.next()?.strip_prefix('+')?;
    // An omitted count means exactly one line: `@@ -1 +1 @@`.
    let count = |spec: &str| match spec.split_once(',') {
        Some((_, c)) => c.parse().ok(),
        None => Some(1),
    };
    Some((count(old)?, count(new)?))
}

fn condense_unified_diff(diff: &str) -> String {
    let mut result = Vec::new();
    let mut current_file = String::new();
    let mut added = 0;
    let mut removed = 0;
    let mut changes = Vec::new();

    // Never truncate diff content — users make decisions based on this data.
    // Only strip diff metadata (headers, @@ hunks); all +/- lines shown in full.
    //
    // `---`/`+++` cannot be classified by prefix: `--- a/f` is a header, but a
    // removed line whose content is `-- note` is also `--- note` on the wire.
    // Only position separates them, so track whether we are inside a hunk.
    //
    // What ends a hunk is the line budget the `@@` header declares. A
    // separator-based rule cannot work: `diff --git`, `diff --cc`, `diff -ru`
    // and `Index:` all spell it differently, and a plain `diff -u` patch spells
    // it not at all — its next file opens with `--- a/f`, itself a valid
    // hunk-body marker. Counting the declared lines down settles all of them
    // without naming any. Headers with no usable budget (combined `@@@`) keep
    // the marker rule: a line with no ` `, `+`, `-` or `\` marker has left the
    // hunk.
    //
    // The budget is only as good as the patch. Hand-maintained queues and
    // pasted patches carry stale counts, and trusting them would drop content
    // or misread the next file's header — silently, which is the one outcome
    // this function must not produce. So disagreement is detected rather than
    // guessed around, and answered with the fallback the repo mandates: return
    // the input untouched. Worst case is no compression, never lost lines.
    let mut in_hunk = false;
    let mut budget: Option<(usize, usize)> = None;

    for line in diff.lines() {
        // Evaluated before this line is classified, so a hunk that declared
        // (0,0) is already shut when the line after the header arrives.
        if in_hunk && budget == Some((0, 0)) {
            in_hunk = false;
        }

        if line.starts_with("@@") {
            // A budget still owing lines when the next hunk opens means the
            // declared counts did not match the body.
            if in_hunk && matches!(budget, Some((o, n)) if o > 0 || n > 0) {
                return diff.to_string();
            }
            in_hunk = true;
            budget = parse_hunk_counts(line);
            continue;
        }

        if in_hunk {
            if let Some((old_left, new_left)) = budget.as_mut() {
                match line.as_bytes().first() {
                    Some(b'+') => *new_left = new_left.saturating_sub(1),
                    Some(b'-') => *old_left = old_left.saturating_sub(1),
                    // "\ No newline at end of file" annotates the line above
                    // and consumes no budget of its own.
                    Some(b'\\') => {}
                    // Context line — present on both sides. Empty counts as
                    // one: a context line can lose its lone leading space.
                    _ => {
                        *old_left = old_left.saturating_sub(1);
                        *new_left = new_left.saturating_sub(1);
                    }
                }
            }
        }

        if !line.is_empty() && !line.starts_with([' ', '+', '-', '\\']) {
            in_hunk = false;
        }

        if !in_hunk && line.starts_with("+++ ") {
            if !current_file.is_empty() && (added > 0 || removed > 0) {
                result.push(format!("[file] {} (+{} -{})", current_file, added, removed));
                // Column 0: anchored greps (`^[+-]`) must match these.
                result.append(&mut changes);
            }
            current_file = line
                .trim_start_matches("+++ ")
                .trim_start_matches("b/")
                .to_string();
            added = 0;
            removed = 0;
            changes.clear();
        } else if !in_hunk && line.starts_with("--- ") {
            // Old-file header; the new-file header above carries the name.
        } else if in_hunk && line.starts_with('+') {
            added += 1;
            changes.push(line.to_string());
        } else if in_hunk && line.starts_with('-') {
            removed += 1;
            changes.push(line.to_string());
        } else if line.starts_with('+') || line.starts_with('-') {
            // A marked line outside every hunk. `git format-patch` legitimately
            // emits two: `---` before the diffstat and `-- ` before the version
            // trailer. Anything else means the budget ran out before the body
            // did, so the parse is unreliable — hand back the input rather than
            // drop the line.
            if line != "---" && line.trim_end() != "--" {
                return diff.to_string();
            }
        }
    }

    // A budget still owing lines at EOF means the same mismatch.
    if in_hunk && matches!(budget, Some((o, n)) if o > 0 || n > 0) {
        return diff.to_string();
    }

    // Last file
    if !current_file.is_empty() && (added > 0 || removed > 0) {
        result.push(format!("[file] {} (+{} -{})", current_file, added, removed));
        // Column 0: anchored greps (`^[+-]`) must match these.
        result.append(&mut changes);
    }

    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- similarity ---

    #[test]
    fn test_similarity_identical() {
        assert_eq!(similarity("hello", "hello"), 1.0);
    }

    #[test]
    fn test_similarity_completely_different() {
        assert_eq!(similarity("abc", "xyz"), 0.0);
    }

    #[test]
    fn test_similarity_empty_strings() {
        // Both empty: union is 0, returns 1.0 by convention
        assert_eq!(similarity("", ""), 1.0);
    }

    #[test]
    fn test_similarity_partial_overlap() {
        let s = similarity("abcd", "abef");
        // Shared: a, b. Union: a, b, c, d, e, f = 6. Jaccard = 2/6
        assert!((s - 2.0 / 6.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_similarity_threshold_for_modified() {
        // "let x = 1;" vs "let x = 2;" should be > 0.5 (treated as modification)
        assert!(similarity("let x = 1;", "let x = 2;") > 0.5);
    }

    // --- compute_diff ---

    #[test]
    fn test_compute_diff_identical() {
        let a = vec!["line1", "line2", "line3"];
        let b = vec!["line1", "line2", "line3"];
        let result = compute_diff(&a, &b);
        assert_eq!(result.added, 0);
        assert_eq!(result.removed, 0);
        assert_eq!(result.modified, 0);
        assert!(result.changes.is_empty());
    }

    #[test]
    fn test_compute_diff_added_lines() {
        let a = vec!["line1"];
        let b = vec!["line1", "line2", "line3"];
        let result = compute_diff(&a, &b);
        assert_eq!(result.added, 2);
        assert_eq!(result.removed, 0);
    }

    #[test]
    fn test_compute_diff_removed_lines() {
        let a = vec!["line1", "line2", "line3"];
        let b = vec!["line1"];
        let result = compute_diff(&a, &b);
        assert_eq!(result.removed, 2);
        assert_eq!(result.added, 0);
    }

    #[test]
    fn test_compute_diff_modified_line() {
        // Similar lines (>0.5 similarity) are classified as modified
        let a = vec!["let x = 1;"];
        let b = vec!["let x = 2;"];
        let result = compute_diff(&a, &b);
        assert_eq!(result.modified, 1);
        assert_eq!(result.added, 0);
        assert_eq!(result.removed, 0);
    }

    #[test]
    fn test_compute_diff_completely_different_line() {
        // Dissimilar lines (<= 0.5 similarity) are added+removed, not modified
        let a = vec!["aaaa"];
        let b = vec!["zzzz"];
        let result = compute_diff(&a, &b);
        assert_eq!(result.modified, 0);
        assert_eq!(result.added, 1);
        assert_eq!(result.removed, 1);
    }

    #[test]
    fn test_compute_diff_empty_inputs() {
        let result = compute_diff(&[], &[]);
        assert_eq!(result.added, 0);
        assert_eq!(result.removed, 0);
        assert!(result.changes.is_empty());
    }

    // --- compute_diff: LCS alignment, not positional ---

    #[test]
    fn test_compute_diff_single_insertion_does_not_desync_the_tail() {
        // The bug: positional compare paired a[i] against b[i], so inserting one
        // line at the top made every later pair compare unrelated lines and the
        // whole file rendered as changed.
        let a = vec!["one", "two", "three", "four", "five"];
        let b = vec!["INSERTED", "one", "two", "three", "four", "five"];
        let result = compute_diff(&a, &b);
        assert_eq!(result.added, 1, "exactly one line was added");
        assert_eq!(result.removed, 0, "nothing was removed");
        assert_eq!(result.modified, 0, "nothing was modified");
        assert_eq!(result.changes.len(), 1);
        match &result.changes[0] {
            DiffChange::Added(line, text) => {
                assert_eq!(text, "INSERTED");
                assert_eq!(*line, 1);
            }
            other => panic!("expected a single Added, got {:?}", other),
        }
    }

    #[test]
    fn test_compute_diff_insertion_in_the_middle() {
        let a = vec!["a", "b", "c", "d"];
        let b = vec!["a", "b", "NEW", "c", "d"];
        let result = compute_diff(&a, &b);
        assert_eq!((result.added, result.removed, result.modified), (1, 0, 0));
    }

    #[test]
    fn test_compute_diff_deletion_in_the_middle() {
        let a = vec!["a", "b", "GONE", "c", "d"];
        let b = vec!["a", "b", "c", "d"];
        let result = compute_diff(&a, &b);
        assert_eq!((result.added, result.removed, result.modified), (0, 1, 0));
        match &result.changes[0] {
            DiffChange::Removed(line, text) => {
                assert_eq!(text, "GONE");
                assert_eq!(*line, 3, "line number is the old file's");
            }
            other => panic!("expected Removed, got {:?}", other),
        }
    }

    #[test]
    fn test_compute_diff_reports_line_numbers_after_a_shift() {
        // A change *after* an insertion must still name its own line, not an
        // offset one.
        let a = vec!["a", "b", "let x = 1;"];
        let b = vec!["NEW", "a", "b", "let x = 2;"];
        let result = compute_diff(&a, &b);
        assert_eq!(result.added, 1);
        assert_eq!(result.modified, 1);
        assert!(result
            .changes
            .iter()
            .any(|c| matches!(c, DiffChange::Modified(3, old, new)
                if old == "let x = 1;" && new == "let x = 2;")));
    }

    #[test]
    fn test_compute_diff_wholesale_replacement_still_pairs_by_similarity() {
        let a = vec!["let x = 1;", "aaaa"];
        let b = vec!["let x = 2;", "zzzz"];
        let result = compute_diff(&a, &b);
        // First pair is similar -> Modified; second is not -> Removed + Added.
        assert_eq!(result.modified, 1);
        assert_eq!(result.added, 1);
        assert_eq!(result.removed, 1);
    }

    #[test]
    fn test_compute_diff_over_cell_cap_emits_wholesale_replacement() {
        // 2001 x 2001 = 4_004_001 cells > LCS_CELL_CAP, so the cap branch must
        // fire: no LCS table, every middle line reported as removed + added,
        // with line numbers from each line's own file. Sides share no prefix,
        // suffix, or interior lines, so nothing is trimmed or matched away.
        let a: Vec<String> = (0..2001).map(|i| format!("alpha {}", i)).collect();
        let b: Vec<String> = (0..2001).map(|i| format!("brave {}", i)).collect();
        let a_refs: Vec<&str> = a.iter().map(|s| s.as_str()).collect();
        let b_refs: Vec<&str> = b.iter().map(|s| s.as_str()).collect();
        assert!(
            a_refs.len().saturating_mul(b_refs.len()) > LCS_CELL_CAP,
            "fixture must exceed the cell cap"
        );

        let result = compute_diff(&a_refs, &b_refs);
        assert_eq!(result.removed, 2001, "every old line reported removed");
        assert_eq!(result.added, 2001, "every new line reported added");
        assert!(result
            .changes
            .iter()
            .any(|c| matches!(c, DiffChange::Removed(1, t) if t == "alpha 0")));
        assert!(result
            .changes
            .iter()
            .any(|c| matches!(c, DiffChange::Added(2001, t) if t == "brave 2000")));
    }

    // --- render_file_diff (issue #2364 regression) ---

    #[test]
    fn test_render_modified_only_yaml_not_identical() {
        // "a: 1" vs "a: 2" is classified as modified (similarity > 0.5);
        // the identical check must not ignore modified-only diffs.
        let (out, code) = render_file_diff(
            Path::new("one.yaml"),
            Path::new("two.yaml"),
            "a: 1\n",
            "a: 2\n",
        );
        assert!(
            !out.contains("identical"),
            "modified-only diff reported as identical:\n{}",
            out
        );
        assert!(out.contains("~1 modified"));
        assert!(out.contains("a: 1"));
        assert!(out.contains("a: 2"));
        assert_eq!(code, 1, "differing files must exit 1 (diff convention)");
    }

    #[test]
    fn test_render_crlf_difference_is_not_identical() {
        // `str::lines()` strips a trailing `\r`, so a CRLF-vs-LF file pair used
        // to collapse to identical line vectors and report "[ok] Files are
        // identical" with exit 0. `cmp` says these differ at byte 6.
        let (out, code) = render_file_diff(
            Path::new("a.txt"),
            Path::new("b.txt"),
            "keep1\nkeep2\nkeep3\n",
            "keep1\r\nkeep2\r\nkeep3\n",
        );
        assert!(
            !out.contains("identical"),
            "CRLF-vs-LF reported as identical:\n{}",
            out
        );
        assert_eq!(code, 1, "differing files must exit 1");
        assert!(out.contains("0 CRLF vs 2 CRLF"), "got: {}", out);
    }

    #[test]
    fn test_render_trailing_newline_difference_is_not_identical() {
        // The other thing `lines()` normalizes: the final newline is optional.
        let (out, code) = render_file_diff(
            Path::new("a.txt"),
            Path::new("b.txt"),
            "keep1\nkeep2\n",
            "keep1\nkeep2",
        );
        assert!(
            !out.contains("identical"),
            "trailing-newline diff reported as identical:\n{}",
            out
        );
        assert_eq!(code, 1);
        assert!(out.contains("trailing newline: present vs absent"), "got: {}", out);
    }

    #[test]
    fn test_render_byte_identical_is_identical() {
        // The guard must not flip the true-identity case to a false positive.
        let (out, code) = render_file_diff(
            Path::new("a.txt"),
            Path::new("b.txt"),
            "keep1\nkeep2\n",
            "keep1\nkeep2\n",
        );
        assert!(out.contains("[ok] Files are identical"));
        assert_eq!(code, 0);
    }

    #[test]
    fn test_render_partial_crlf_matches_reported_repro() {
        // The shape actually observed: a 200-line file where a Windows editor
        // touched 24 lines. Text identical, bytes differ, `cmp` exits 1.
        let plain: String = (0..200).map(|i| format!("line {} content here\n", i)).collect();
        let mixed: String = (0..200)
            .map(|i| {
                if (50..74).contains(&i) {
                    format!("line {} content here\r\n", i)
                } else {
                    format!("line {} content here\n", i)
                }
            })
            .collect();
        assert_ne!(plain, mixed, "fixture must actually differ");

        let (out, code) = render_file_diff(Path::new("a.txt"), Path::new("b.txt"), &plain, &mixed);
        assert!(!out.contains("identical"), "got: {}", out);
        assert_eq!(code, 1, "must exit 1 so a `diff` gate fails");
        assert!(out.contains("0 CRLF vs 24 CRLF"), "got: {}", out);
    }

    #[test]
    fn test_render_modified_only_json_not_identical() {
        let (out, code) = render_file_diff(
            Path::new("j1.json"),
            Path::new("j2.json"),
            "{\"a\": 1}\n",
            "{\"a\": 2}\n",
        );
        assert!(
            !out.contains("identical"),
            "modified-only diff reported as identical:\n{}",
            out
        );
        assert_eq!(code, 1);
    }

    #[test]
    fn test_render_identical_files_exit_zero() {
        let (out, code) = render_file_diff(
            Path::new("a.yaml"),
            Path::new("b.yaml"),
            "a: 1\nb: 2\n",
            "a: 1\nb: 2\n",
        );
        assert!(out.contains("[ok] Files are identical"));
        assert_eq!(code, 0);
    }

    #[test]
    fn test_render_added_removed_exit_one() {
        let (out, code) = render_file_diff(Path::new("t1.txt"), Path::new("t2.txt"), "x\n", "y\n");
        assert!(out.contains("+1 added, -1 removed"));
        assert_eq!(code, 1);
    }

    // --- condense_unified_diff ---

    #[test]
    fn test_condense_unified_diff_single_file() {
        let diff = r#"diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,3 +1,4 @@
 fn main() {
+    println!("hello");
     println!("world");
 }
"#;
        let result = condense_unified_diff(diff);
        assert!(result.contains("src/main.rs"));
        assert!(result.contains("+1"));
        assert!(result.contains("println"));
    }

    #[test]
    fn test_condense_unified_diff_multiple_files() {
        // Hunk headers are mandatory in a unified diff; without them this
        // fixture exercised a shape no producer can emit.
        let diff = r#"diff --git a/a.rs b/a.rs
--- a/a.rs
+++ b/a.rs
@@ -0,0 +1 @@
+added line
diff --git a/b.rs b/b.rs
--- a/b.rs
+++ b/b.rs
@@ -1 +0,0 @@
-removed line
"#;
        let result = condense_unified_diff(diff);
        assert!(result.contains("a.rs"));
        assert!(result.contains("b.rs"));
    }

    #[test]
    fn test_condense_unified_diff_markers_at_column_0() {
        // Same silent-false-negative class as compact_diff (#118 / upstream
        // #3646): indented markers make anchored greps (`^[+-]`) match nothing.
        //
        // Two files on purpose. A file's changes are flushed at two separate
        // sites: once per `+++` for the preceding file, once after the loop for
        // the last one. A single-file fixture only ever reaches the second, so
        // the first could be reverted with the whole suite still green.
        let diff = "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1 +1 @@\n\
                    -fn old() {}\n+fn new() {}\n\
                    diff --git a/b.rs b/b.rs\n--- a/b.rs\n+++ b/b.rs\n@@ -1 +1 @@\n\
                    -let x = 1;\n+let x = 2;\n";
        let result = condense_unified_diff(diff);
        for want in ["-fn old() {}", "+fn new() {}", "-let x = 1;", "+let x = 2;"] {
            assert!(
                result.lines().any(|l| l == want),
                "missing {want:?} at column 0 in:\n{}",
                result
            );
        }
        // Scoped to change lines: a `[file] …` header is not one, and only
        // change lines carry the column-0 guarantee this pins.
        assert!(
            !result
                .lines()
                .any(|l| l.starts_with(" +") || l.starts_with(" -")),
            "change lines must not be indented:\n{}",
            result
        );
    }

    #[test]
    fn test_condense_unified_diff_keeps_plusplus_and_minusminus_content() {
        // Only `--- a/f` and `+++ b/f` (trailing space) are headers. A changed
        // line whose content is `---`/`+++` is real content: Markdown rules and
        // front-matter fences, `++i`, SQL `-- comment`. Dropping it loses the
        // line and desyncs the (+N -M) counter.
        let diff = "diff --git a/README.md b/README.md\n--- a/README.md\n+++ b/README.md\n\
                    @@ -1,2 +1,2 @@\n---\n-title: Old\n+++\n+title = \"New\"\n";
        let result = condense_unified_diff(diff);
        assert!(result.lines().any(|l| l == "---"), "got:\n{}", result);
        assert!(result.lines().any(|l| l == "+++"), "got:\n{}", result);
        assert!(
            result.contains("(+2 -2)"),
            "counter must include the ---/+++ lines:\n{}",
            result
        );
    }

    #[test]
    fn test_condense_unified_diff_keeps_dashdash_comment_content() {
        // A removed line whose content is `-- note` arrives as `--- note`, which
        // is prefix-identical to the `--- a/file` header. SQL/Lua/Haskell/Ada
        // comments and mail signature delimiters all land here. Dropping them
        // loses the lines and desyncs the counter.
        let diff = "diff --git a/m.sql b/m.sql\n--- a/m.sql\n+++ b/m.sql\n\
                    @@ -1,3 +0,0 @@\n--- Legacy bootstrap\n--- DO NOT RUN\n-  email TEXT,\n";
        let result = condense_unified_diff(diff);
        assert!(
            result.lines().any(|l| l == "--- Legacy bootstrap"),
            "got:\n{}",
            result
        );
        assert!(
            result.lines().any(|l| l == "--- DO NOT RUN"),
            "got:\n{}",
            result
        );
        assert!(
            result.contains("(+0 -3)"),
            "counter must count all three removals:\n{}",
            result
        );
        assert!(result.contains("[file] m.sql"), "got:\n{}", result);
    }

    #[test]
    fn test_condense_unified_diff_plusplus_content_does_not_rename_file() {
        // An added line whose content is `++ text` arrives as `+++ text`, which
        // is prefix-identical to the `+++ b/file` header. Treating it as one
        // renamed the file to a fragment of the user's own diff and dropped the
        // line — the filename in the output was no longer a filename at all.
        let diff = "diff --git a/c.md b/c.md\n--- a/c.md\n+++ b/c.md\n\
                    @@ -1,1 +1,1 @@\n+++ new section marker\n-- removed a feature\n";
        let result = condense_unified_diff(diff);
        assert!(result.contains("[file] c.md"), "got:\n{}", result);
        assert!(
            !result.contains("[file] new section marker"),
            "content must not become the file name:\n{}",
            result
        );
        assert!(
            result.lines().any(|l| l == "+++ new section marker"),
            "got:\n{}",
            result
        );
        assert!(result.contains("(+1 -1)"), "got:\n{}", result);
    }

    #[test]
    fn test_condense_unified_diff_file_boundary_after_a_hunk() {
        // A hunk must end at the next file for EVERY separator a unified-diff
        // producer emits, not just `diff --git`. Keying it on `diff --git`
        // alone collapsed files 2..N into the first, inflated its counter, and
        // leaked the `---`/`+++` headers into the change list. `diff --cc` is
        // git's own merge output, so this is not an exotic-tool concern.
        for sep in [
            "diff --git a/b.rs b/b.rs",
            "diff --cc b.rs",
            "diff -ru old/b.rs new/b.rs",
            "Index: b.rs",
        ] {
            let diff = format!(
                "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1 +1 @@\n\
                 -old\n+new\n{sep}\n--- a/b.rs\n+++ b/b.rs\n@@ -1 +1 @@\n-x\n+y\n"
            );
            let r = condense_unified_diff(&diff);
            assert_eq!(
                r.lines().filter(|l| l.starts_with("[file]")).count(),
                2,
                "separator {sep:?} did not end the hunk:\n{r}"
            );
            assert!(r.contains("[file] a.rs (+1 -1)"), "sep {sep:?}:\n{r}");
            assert!(r.contains("[file] b.rs (+1 -1)"), "sep {sep:?}:\n{r}");
            assert!(
                !r.lines().any(|l| l == "--- a/b.rs"),
                "header leaked into content for {sep:?}:\n{r}"
            );
        }
    }

    #[test]
    fn test_condense_unified_diff_blank_line_does_not_end_hunk() {
        // A context line whose single leading space was stripped in transit
        // arrives empty. It must stay neutral: ending the hunk there makes the
        // following `---`/`+++` content lines read as headers, which drops them
        // AND renames the file to a fragment of the diff. The lines after the
        // blank are chosen so the two behaviours differ.
        let diff = "diff --git a/m.sql b/m.sql\n--- a/m.sql\n+++ b/m.sql\n@@ -1,3 +1,3 @@\n-before\n\n--- legacy note\n+++ new marker\n+after\n";
        let r = condense_unified_diff(diff);
        assert!(r.contains("[file] m.sql"), "file renamed by content: {}", r);
        assert!(r.lines().any(|l| l == "--- legacy note"), "got: {}", r);
        assert!(r.lines().any(|l| l == "+++ new marker"), "got: {}", r);
        assert!(r.contains("(+2 -2)"), "got: {}", r);
    }

    #[test]
    fn test_condense_unified_diff_empty() {
        let result = condense_unified_diff("");
        assert!(result.is_empty());
    }

    // --- truncation accuracy ---

    fn make_large_unified_diff(added: usize, removed: usize) -> String {
        let mut lines = vec![
            "diff --git a/config.yaml b/config.yaml".to_string(),
            "--- a/config.yaml".to_string(),
            "+++ b/config.yaml".to_string(),
            format!("@@ -1,{} +1,{} @@", removed.max(1), added.max(1)),
        ];
        for i in 0..removed {
            lines.push(format!("-old_value_{}", i));
        }
        for i in 0..added {
            lines.push(format!("+new_value_{}", i));
        }
        lines.join("\n")
    }

    #[test]
    fn test_condense_unified_diff_reports_no_phantom_overflow() {
        // The `... +N more` trailer was a vestige of a display cap that no
        // longer exists: the loop emits every change line, so a trailer
        // claiming N were withheld is false. It cost real tokens too — an
        // agent reading "+190 more" re-runs the raw command to see lines that
        // were already on screen, which is the opposite of the point.
        let diff = make_large_unified_diff(100, 100);
        let result = condense_unified_diff(&diff);
        assert!(
            !result.contains("more"),
            "no truncation happens, so nothing may claim it:\n{}",
            result
        );
        // All 200 changes are present, which is why the trailer was a lie.
        assert_eq!(
            result
                .lines()
                .filter(|l| l.starts_with('+') || l.starts_with('-'))
                .count(),
            200,
            "every change line must be emitted:\n{}",
            result
        );
    }

    #[test]
    fn test_condense_unified_diff_no_false_overflow() {
        // 8 changes total — well under any historical cap, no overflow message
        let diff = make_large_unified_diff(4, 4);
        let result = condense_unified_diff(&diff);
        assert!(
            !result.contains("more"),
            "No overflow message expected for 8 changes, got:\n{}",
            result
        );
    }

    #[test]
    fn test_no_truncation_large_diff() {
        // Verify compute_diff returns all changes without truncation
        let mut a = Vec::new();
        let mut b = Vec::new();
        for i in 0..500 {
            a.push(format!("line_{}", i));
            if i % 3 == 0 {
                b.push(format!("CHANGED_{}", i));
            } else {
                b.push(format!("line_{}", i));
            }
        }
        let a_refs: Vec<&str> = a.iter().map(|s| s.as_str()).collect();
        let b_refs: Vec<&str> = b.iter().map(|s| s.as_str()).collect();
        let result = compute_diff(&a_refs, &b_refs);

        assert!(
            result.changes.len() > 100,
            "Expected 100+ changes, got {}",
            result.changes.len()
        );
        assert!(!result.changes.is_empty());
    }

    #[test]
    fn test_format_diff_shows_all_changes() {
        let mut a = Vec::new();
        let mut b = Vec::new();
        for i in 0..100 {
            a.push(format!("old_line_{}", i));
            b.push(format!("new_line_{}", i));
        }
        let a_refs: Vec<&str> = a.iter().map(|s| s.as_str()).collect();
        let b_refs: Vec<&str> = b.iter().map(|s| s.as_str()).collect();
        let diff = compute_diff(&a_refs, &b_refs);
        let output = format_diff_changes(&diff);

        assert!(output.contains("old_line_0"), "should contain first change");
        assert!(output.contains("new_line_99"), "should contain last change");
    }

    #[test]
    fn test_long_lines_not_truncated() {
        let long_line = "x".repeat(500);
        let a = vec![long_line.as_str()];
        let b = vec!["short"];
        let result = compute_diff(&a, &b);
        match &result.changes[0] {
            DiffChange::Removed(_, content) | DiffChange::Added(_, content) => {
                assert_eq!(content.len(), 500, "Line was truncated!");
            }
            DiffChange::Modified(_, old, _) => {
                assert_eq!(old.len(), 500, "Line was truncated!");
            }
        }
    }

    #[test]
    fn test_condense_unified_diff_plain_patch_multi_file() {
        // POSIX `diff -u` / quilt / a pasted patch: the only thing separating
        // one file from the next is the `---`/`+++` pair. There is no
        // `diff --git` line, and `--- a/bar.c` is itself a valid hunk-body
        // marker, so nothing but the hunk's declared line budget can tell where
        // foo.c ends. Getting this wrong merged bar.c into foo.c and leaked its
        // headers into the change list.
        let diff = "--- a/foo.c\n+++ b/foo.c\n@@ -1,2 +1,2 @@\n ctx\n-old foo\n+new foo\n\
                    --- a/bar.c\n+++ b/bar.c\n@@ -1,2 +1,2 @@\n ctx\n-old bar\n+new bar\n";
        let r = condense_unified_diff(diff);
        assert!(r.contains("[file] foo.c (+1 -1)"), "got:\n{}", r);
        assert!(r.contains("[file] bar.c (+1 -1)"), "got:\n{}", r);
        assert!(
            !r.lines().any(|l| l == "--- a/bar.c" || l == "+++ b/bar.c"),
            "headers leaked into content:\n{}",
            r
        );
    }

    #[test]
    fn test_condense_unified_diff_mbox_separators_are_not_changes() {
        // `git format-patch` output puts a bare `---` before the diffstat and a
        // `-- ` signature delimiter before the git version. Both sit OUTSIDE any
        // hunk and are not changes; counting them inflated the previous file's
        // removal count by one apiece.
        let diff = "From abc123 Mon Sep 17 00:00:00 2001\nSubject: [PATCH] x\n\n\
                    ---\n f.rs | 2 +-\n 1 file changed\n\n\
                    diff --git a/f.rs b/f.rs\n--- a/f.rs\n+++ b/f.rs\n\
                    @@ -1 +1 @@\n-old\n+new\n-- \n2.43.0\n";
        let r = condense_unified_diff(diff);
        assert!(r.contains("[file] f.rs (+1 -1)"), "got:\n{}", r);
        assert!(!r.lines().any(|l| l == "---"), "mbox `---` counted:\n{}", r);
        assert!(!r.lines().any(|l| l == "-- "), "signature counted:\n{}", r);
    }

    #[test]
    fn test_condense_unified_diff_no_newline_marker_stays_in_hunk() {
        // `\ No newline at end of file` annotates the line above. It carries no
        // +/- marker, so it must be recognised as hunk body — otherwise it ends
        // the hunk and the following content lines are misread as headers.
        let diff = "diff --git a/f.rs b/f.rs\n--- a/f.rs\n+++ b/f.rs\n@@ -1,2 +1,1 @@\n\
                    -old\n\\ No newline at end of file\n--- removed note\n+++ added marker\n";
        let r = condense_unified_diff(diff);
        assert!(r.contains("[file] f.rs"), "file renamed by content:\n{}", r);
        assert!(r.lines().any(|l| l == "--- removed note"), "got:\n{}", r);
        assert!(r.lines().any(|l| l == "+++ added marker"), "got:\n{}", r);
        assert!(r.contains("(+1 -2)"), "got:\n{}", r);
    }

    /// rtk's own estimator is bytes/4 (`core::tracking::estimate_tokens`); the
    /// repo's testing rules specify `split_whitespace` counting for filters.
    fn count_tokens(text: &str) -> usize {
        text.split_whitespace().count()
    }

    #[test]
    fn test_condense_unified_diff_real_fixture_savings_and_fidelity() {
        // Real captured `git diff` output, not a synthetic string.
        //
        // This filter strips headers and `@@` lines and emits every change line
        // verbatim — its stated contract is "never truncate diff content" — so
        // its reduction is structurally modest and nowhere near the 60% floor
        // that truncating filters meet. What matters here is that it never
        // grows the output and never drops a change line; the percentage is a
        // regression floor, not a compression claim.
        let raw = include_str!("../../../tests/fixtures/diff_stdin_unified_raw.txt");
        let out = condense_unified_diff(raw);

        let saved = 100.0 - (count_tokens(&out) as f64 / count_tokens(raw) as f64 * 100.0);
        assert!(saved > 8.0, "savings regressed to {:.1}%", saved);

        // Fidelity: every +/- line in the input survives, at column 0.
        let want: Vec<&str> = raw
            .lines()
            .filter(|l| {
                (l.starts_with('+') || l.starts_with('-'))
                    && !l.starts_with("+++ ")
                    && !l.starts_with("--- ")
            })
            .collect();
        assert!(want.len() > 100, "fixture too small to be meaningful");
        for line in &want {
            assert!(
                out.lines().any(|l| l == *line),
                "dropped change line {:?}",
                line
            );
        }
    }

    #[test]
    fn test_condense_unified_diff_combined_diff_multi_file() {
        // `git show --cc` headers are `@@@ … @@@`, which carry a second parent
        // column the line budget does not model, so they parse to no budget and
        // fall back to the marker rule. That fallback is the ONLY thing closing
        // a hunk here — without it the second file folds into the first.
        let diff = "diff --cc a.txt\n--- a/a.txt\n+++ b/a.txt\n@@@ -1,1 -1,1 +1,1 @@@\n- M1\n++A\ndiff --cc b.txt\n--- a/b.txt\n+++ b/b.txt\n@@@ -1,1 -1,1 +1,1 @@@\n- M2\n++B\n";
        let r = condense_unified_diff(diff);
        assert_eq!(
            r.lines().filter(|l| l.starts_with("[file]")).count(),
            2,
            "combined diff collapsed:\n{}",
            r
        );
        assert!(r.contains("[file] a.txt"), "got:\n{}", r);
        assert!(r.contains("[file] b.txt"), "got:\n{}", r);
        assert!(
            !r.lines().any(|l| l == "--- a/b.txt"),
            "header leaked as content:\n{}",
            r
        );
    }

    #[test]
    fn test_condense_unified_diff_budget_shorter_than_body_passes_through() {
        // Hand-maintained patch queues and pasted patches carry stale counts.
        // Here the body outruns the declared budget. Summarising anyway would
        // drop the trailing changes and report a counter to match — silent
        // loss, the one outcome this filter must never produce. Fall back.
        let diff = "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1,1 +1,1 @@\n-old1\n+new1\n-old2\n+new2\n";
        let r = condense_unified_diff(diff);
        assert_eq!(r, diff, "must hand back the input verbatim, got:\n{}", r);
    }

    #[test]
    fn test_condense_unified_diff_budget_longer_than_body_passes_through() {
        // The mirror case, and the dangerous one: on a plain patch the budget
        // is the only close, so an over-declared count swallows the next
        // file's headers as content and loses that file entirely.
        let diff = "--- a/a.rs\n+++ b/a.rs\n@@ -1,20 +1,20 @@\n-old1\n+new1\n--- a/b.rs\n+++ b/b.rs\n@@ -1,20 +1,20 @@\n-old2\n+new2\n";
        let r = condense_unified_diff(diff);
        assert_eq!(r, diff, "must hand back the input verbatim, got:\n{}", r);
    }

    #[test]
    fn test_condense_unified_diff_zero_budget_hunk_closes_immediately() {
        // `@@ -0,0 +0,0 @@` declares an empty body, so the very next line is
        // already outside the hunk. Evaluating the close only after processing
        // a line left it open for exactly one extra line, which ate the next
        // file's `--- ` header as a removal.
        let diff = "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -0,0 +0,0 @@\ndiff --git a/b.rs b/b.rs\n--- a/b.rs\n+++ b/b.rs\n@@ -1 +1 @@\n-x\n+y\n";
        let r = condense_unified_diff(diff);
        assert!(r.contains("[file] b.rs (+1 -1)"), "got:\n{}", r);
        assert!(
            !r.lines().any(|l| l == "--- a/b.rs"),
            "header counted as a change:\n{}",
            r
        );
    }

    #[test]
    fn test_condense_unified_diff_added_line_past_budget_passes_through() {
        // Mirror of the removed-line case, and it needs its own fixture: the
        // ordering here makes an ADDED line the first marked line after the
        // budget closes. Counting it instead of falling back would emit a
        // summary that silently omits the rest of the body.
        let diff = "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1,1 +1,1 @@\n+new1\n-old1\n+new2\n";
        let r = condense_unified_diff(diff);
        assert_eq!(r, diff, "must hand back the input verbatim, got:\n{}", r);
    }
}
