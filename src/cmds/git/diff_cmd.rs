//! Compares two files and shows only the changed lines.

use crate::core::guard::never_worse;
use crate::core::tracking;
use anyhow::{Context, Result};
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
    let lines1: Vec<&str> = content1.lines().collect();
    let lines2: Vec<&str> = content2.lines().collect();
    let diff = compute_diff(&lines1, &lines2);
    let fallback = format_classic_diff(&diff);
    let both_files = format!("{}\n---\n{}", content1, content2);

    let (rtk, exit_code) = render_file_diff(file1, file2, &content1, &content2, &diff);

    let shown = select_file_diff_output(&diff, &fallback, &rtk);
    print!("{}", shown);
    timer.track(
        &format!("diff {} {}", file1.display(), file2.display()),
        "rtk diff",
        tracking_baseline(&diff, &fallback, &both_files, shown),
        shown,
    );
    Ok(exit_code)
}

/// Renders the condensed file comparison and returns it with the
/// diff-convention exit code (0 = identical, 1 = differences found).
///
/// Byte equality is the only safe basis for claiming identity, and it must be
/// checked before `lines()` touches the input. `str::lines()` strips a
/// trailing `\r` and treats the final newline as optional, so a CRLF-vs-LF or
/// missing-trailing-newline difference collapses to identical line vectors.
/// Reporting "identical" with exit 0 for files that differ silently passes
/// any verification gate built on `diff`.
fn render_file_diff(
    file1: &Path,
    file2: &Path,
    content1: &str,
    content2: &str,
    diff: &DiffResult,
) -> (String, i32) {
    if content1 == content2 {
        return ("[ok] Files are identical\n".to_string(), 0);
    }

    if diff.changes.is_empty() {
        // Bytes differ, lines don't: the difference is exactly what `lines()`
        // normalizes away. Name it rather than rendering an empty change list.
        return (
            describe_invisible_difference(file1, file2, content1, content2),
            1,
        );
    }

    render_diff(file1, file2, diff)
}

fn render_diff(file1: &Path, file2: &Path, diff: &DiffResult) -> (String, i32) {
    if diff.changes.is_empty() {
        return ("[ok] Files are identical\n".to_string(), 0);
    }

    let mut rtk = String::new();
    rtk.push_str(&format!("{} → {}\n", file1.display(), file2.display()));
    rtk.push_str(&format!(
        "   +{} added, -{} removed, ~{} modified\n\n",
        diff.added, diff.removed, diff.modified
    ));
    rtk.push_str(&format_diff_changes(diff));
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

    // Bytes, not String: piped diffs are not guaranteed UTF-8 (patches quote
    // the target file's bytes). Non-UTF-8 input takes the raw-bytes branch
    // below — never a hard error, and never a lossy re-encode of content.
    let mut bytes = Vec::new();
    io::stdin()
        .read_to_end(&mut bytes)
        .context("Failed to read diff from stdin")?;

    match condense_stdin(&bytes) {
        Some(condensed) => {
            println!("{}", condensed);
            timer.track(
                "diff (stdin)",
                "rtk diff (stdin)",
                &String::from_utf8_lossy(&bytes),
                &condensed,
            );
        }
        None => {
            // Structural fallback: the caller's exact bytes (plus println!
            // parity — a terminating newline when the input lacked one).
            use std::io::Write;
            let mut out = io::stdout();
            out.write_all(&bytes)
                .context("Failed to write raw diff to stdout")?;
            if !bytes.is_empty() && !bytes.ends_with(b"\n") {
                writeln!(out).context("Failed to write raw diff to stdout")?;
            }
            let raw = String::from_utf8_lossy(&bytes);
            timer.track("diff (stdin)", "rtk diff (stdin)", &raw, &raw);
        }
    }

    Ok(())
}

/// Filter a piped stream: strip ANSI (a `git diff --color` stream otherwise
/// matches nothing and condenses to silence), parse strictly, and apply the
/// never-worse guard. `None` means the caller must emit its exact input
/// bytes — including for non-UTF-8 input, where filtering would rewrite the
/// user's content bytes to U+FFFD (byte fidelity outranks savings here).
fn condense_stdin(bytes: &[u8]) -> Option<String> {
    let input = std::str::from_utf8(bytes).ok()?;
    let cleaned = crate::core::utils::strip_ansi(input);
    let condensed = condense_unified_diff_strict(&cleaned)?;
    // The never_worse contract, against what the user would otherwise get —
    // the original input, not the ANSI-stripped intermediate.
    if crate::core::tracking::estimate_tokens(&condensed)
        <= crate::core::tracking::estimate_tokens(input)
    {
        Some(condensed)
    } else {
        None
    }
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

fn format_classic_diff(diff: &DiffResult) -> String {
    let mut out = String::new();
    let mut index = 0;

    while index < diff.changes.len() {
        match &diff.changes[index] {
            DiffChange::Modified(start, _, _) => {
                let start = *start;
                let mut end = start;
                let mut old_lines = Vec::new();
                let mut new_lines = Vec::new();

                while let Some(DiffChange::Modified(line, old, new)) = diff.changes.get(index) {
                    if *line != end {
                        break;
                    }
                    old_lines.push(old);
                    new_lines.push(new);
                    end += 1;
                    index += 1;
                }

                out.push_str(&format!(
                    "{}c{}\n",
                    format_line_range(start, end - 1),
                    format_line_range(start, end - 1)
                ));
                for line in old_lines {
                    out.push_str(&format!("< {}\n", line));
                }
                out.push_str("---\n");
                for line in new_lines {
                    out.push_str(&format!("> {}\n", line));
                }
            }
            DiffChange::Removed(start, _) if matches!(
                diff.changes.get(index + 1),
                Some(DiffChange::Added(line, _)) if line == start
            ) => {
                let start = *start;
                let mut end = start;
                let mut old_lines = Vec::new();
                let mut new_lines = Vec::new();

                while let (
                    Some(DiffChange::Removed(old_line, old)),
                    Some(DiffChange::Added(new_line, new)),
                ) = (diff.changes.get(index), diff.changes.get(index + 1))
                {
                    if *old_line != end || *new_line != end {
                        break;
                    }
                    old_lines.push(old);
                    new_lines.push(new);
                    end += 1;
                    index += 2;
                }

                out.push_str(&format!(
                    "{}c{}\n",
                    format_line_range(start, end - 1),
                    format_line_range(start, end - 1)
                ));
                for line in old_lines {
                    out.push_str(&format!("< {}\n", line));
                }
                out.push_str("---\n");
                for line in new_lines {
                    out.push_str(&format!("> {}\n", line));
                }
            }
            DiffChange::Added(start, _) => {
                let start = *start;
                let mut end = start;
                let mut new_lines = Vec::new();

                while let Some(DiffChange::Added(line, new)) = diff.changes.get(index) {
                    if *line != end {
                        break;
                    }
                    new_lines.push(new);
                    end += 1;
                    index += 1;
                }

                out.push_str(&format!(
                    "{}a{}\n",
                    start - 1,
                    format_line_range(start, end - 1)
                ));
                for line in new_lines {
                    out.push_str(&format!("> {}\n", line));
                }
            }
            DiffChange::Removed(start, _) => {
                let start = *start;
                let mut end = start;
                let mut old_lines = Vec::new();

                while let Some(DiffChange::Removed(line, old)) = diff.changes.get(index) {
                    if *line != end {
                        break;
                    }
                    old_lines.push(old);
                    end += 1;
                    index += 1;
                }

                out.push_str(&format!(
                    "{}d{}\n",
                    format_line_range(start, end - 1),
                    start - 1
                ));
                for line in old_lines {
                    out.push_str(&format!("< {}\n", line));
                }
            }
        }
    }
    out
}

fn format_line_range(start: usize, end: usize) -> String {
    if start == end {
        start.to_string()
    } else {
        format!("{start},{end}")
    }
}

/// Baseline the savings are measured against: what `diff` itself would have
/// printed, so the recorded ratio compares like with like and can never go
/// negative -- the guard already caps the shown output at the fallback.
fn tracking_baseline<'a>(
    diff: &DiffResult,
    fallback: &'a str,
    both_files: &'a str,
    shown: &'a str,
) -> &'a str {
    if !diff.changes.is_empty() {
        return fallback;
    }

    // Identical files: `diff` prints nothing, so the dump of both files
    // stands in as the output that would otherwise have to be read. Two
    // near-empty files can make that dump cheaper than the verdict line,
    // which would book a loss against the cheapest possible answer.
    if tracking::estimate_tokens(both_files) >= tracking::estimate_tokens(shown) {
        both_files
    } else {
        shown
    }
}

fn select_file_diff_output<'a>(diff: &DiffResult, raw: &'a str, rendered: &'a str) -> &'a str {
    if diff.changes.is_empty() {
        rendered
    } else {
        never_worse(raw, rendered)
    }
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

/// One parsed file section of the stream.
#[derive(Default)]
struct FileEntry {
    name: String,
    added: usize,
    removed: usize,
    changes: Vec<String>,
    notes: Vec<String>,
    /// A `rename from X` seen while this section's header was still open.
    rename_from: Option<String>,
    /// True once a `@@` hunk header was accepted for this section. Gates the
    /// header-pair rule: a `---`/`+++` pair renames a hunkless section in
    /// place (git's extended header precedes them) but flushes one that
    /// already carries hunks (plain `diff -u` concatenates files that way).
    saw_hunk: bool,
}

impl FileEntry {
    fn header_only(&self) -> bool {
        !self.saw_hunk && self.changes.is_empty()
    }
    fn is_empty(&self) -> bool {
        self.changes.is_empty() && self.notes.is_empty()
    }
}

/// Remaining line budget of an open hunk, from its `@@ -a,b +c,d @@` header
/// (one `old_left` slot per parent; `@@@` combined headers carry two or more).
struct HunkBudget {
    old_left: Vec<usize>,
    new_left: usize,
}

impl HunkBudget {
    fn exhausted(&self) -> bool {
        self.new_left == 0 && self.old_left.iter().all(|&n| n == 0)
    }
}

/// True for the mbox `From <sha> <date>` separator `git format-patch` puts
/// before each patch. The sha is 40 hex digits in SHA-1 repos and 64 in
/// SHA-256 repos; both are accepted.
fn is_mbox_from(line: &str) -> bool {
    line.strip_prefix("From ").is_some_and(|rest| {
        let b = rest.as_bytes();
        [40usize, 64].iter().any(|&n| {
            b.len() > n && b[..n].iter().all(|c| c.is_ascii_hexdigit()) && b[n] == b' '
        })
    })
}

/// `--- <name>` / `+++ <name>` → display name: the `+++` side unless it is
/// `/dev/null` (a deletion), then the `--- ` side. One `b/`/`a/` prefix is
/// stripped (exactly once — `b/b/x` names a file in a `b/` directory), and
/// anything after the first tab (`diff -u` timestamps, svn `(working copy)`)
/// is dropped.
fn header_name(minus: &str, plus: &str) -> String {
    fn clean(side: &str, prefix: &str) -> String {
        let side = side.split('\t').next().unwrap_or(side);
        side.strip_prefix(prefix).unwrap_or(side).to_string()
    }
    let p = clean(plus, "b/");
    if p == "/dev/null" {
        clean(minus, "a/")
    } else {
        p
    }
}

/// Parse `@@ -a[,b] +c[,d] @@ ...` (and `@@@ -a,b -c,d +e,f @@@ ...` with one
/// `-` range per parent) into per-parent old-line budgets and the new-line
/// budget. Omitted counts default to 1 per the unified-diff spec.
fn parse_hunk_header(line: &str) -> Option<(Vec<usize>, usize)> {
    fn parse_range(tok: &str, sign: char) -> Option<usize> {
        let tok = tok.strip_prefix(sign)?;
        match tok.split_once(',') {
            Some((start, count)) => {
                start.parse::<usize>().ok()?;
                count.parse().ok()
            }
            None => {
                tok.parse::<usize>().ok()?;
                Some(1)
            }
        }
    }

    let ats = line.bytes().take_while(|&b| b == b'@').count();
    // 2..=9: parents beyond 8 (an implausible octopus) fall back to raw.
    if !(2..=9).contains(&ats) {
        return None;
    }
    let parents = ats - 1;
    let mut toks = line[ats..].split(' ').filter(|t| !t.is_empty());
    let mut old_left = Vec::with_capacity(parents);
    for _ in 0..parents {
        old_left.push(parse_range(toks.next()?, '-')?);
    }
    let new_left = parse_range(toks.next()?, '+')?;
    let close = toks.next()?;
    if close.len() != ats || !close.bytes().all(|b| b == b'@') {
        return None;
    }
    Some((old_left, new_left))
}

/// Region parser for unified-diff streams. Splits the input into
/// (prose)(file-header)(hunk)* regions and classifies lines only within their
/// region; `None` means the stream disagreed with its own structure and the
/// caller must fall back to raw passthrough.
///
/// Detector precedence, total order — earlier rules own the line:
///
/// 1. Inside a hunk, the `@@` line budget owns every line. An invalid body
///    prefix, a budget over-consumed by one more body line, or EOF with
///    budget still owed → `None`. (Hunks close the moment the budget hits
///    zero, so a `@@` or file header arriving while a hunk is open is itself
///    budget-owed → the invalid-prefix arm returns `None`.) The
///    `\ No newline at end of file` marker consumes no budget but is kept as
///    content — in-hunk (old side) or immediately after the budget closes
///    (new side) — because it is the only witness of a newline-only change.
/// 2. An mbox `From <sha>` separator resets to the prose prologue.
/// 3. `diff --git` / `diff --cc` opens a file section (git extended headers
///    such as `rename`/`Binary files`/mode lines annotate it).
/// 4. A `--- X` line immediately followed by `+++ Y` is a file header: it
///    renames a still-hunkless section in place, or — when the line after
///    `+++ Y` opens a hunk, as every real producer's does — opens a new
///    section. A pair with no hunk behind it is not consumed: it falls to
///    rules 7-9, so stray marked lines are never swallowed as a phantom
///    header. This is what ends the prose prologue — the prologue is
///    positional (everything before the first file header), never keyed on
///    line values. (Bound: mbox prose quoting an unindented, well-formed
///    header-plus-hunk block still fabricates a phantom entry — noise, not
///    loss, since any budget disagreement in it falls back raw.)
/// 5. `@@` after a file header opens a hunk; a malformed `@@` line there is
///    `None`. Before any file section (a hunk quoted in commit prose) it
///    stays prose.
/// 6. File-level facts producers emit outside hunks become note-only
///    entries: `Only in <dir>: <file>` and standalone `Binary files X and Y
///    differ` (GNU `diff -r`), `* Unmerged path <file>` (`git diff --ours`
///    et al. during a merge), and `Submodule <name> <a>..<b>` headers.
///    These arms are suppressed inside an mbox message region (from a
///    `From <sha>` separator to that patch's first file header), where
///    column-0 prose is indistinguishable from them by value.
/// 7. In a stream that carried an mbox `From <sha>` separator, a line of
///    exactly `--`/`-- ` outside a hunk is the format-patch signature
///    separator: prose. This is the single value-keyed exclusion, kept
///    because every patch `git format-patch` emits ends with one; its body
///    (`2.54.0`) is unmarked and needs no region. Streams that never had an
///    mbox separator (plain `git diff`, `diff -u`) get no such tolerance —
///    a bare `--` there falls to rule 8. (Bound: in a malformed mbox stream
///    a stale-budget leftover of exactly `--` is swallowed as a signature;
///    every other leftover value still falls through.)
/// 8. Any other `+`/`-` marked line outside a hunk and after the prologue is
///    evidence of a stale or under-declared budget → `None`. (Well-formed
///    unified diffs have no content outside hunks. The prologue exclusion
///    means a hunk quoted in patch prose stays prose — including its marked
///    lines.)
/// 9. Everything else is prose and is dropped.
fn condense_unified_diff_strict(diff: &str) -> Option<String> {
    let mut lines: Vec<&str> = diff.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }

    let mut entries: Vec<FileEntry> = Vec::new();
    let mut current: Option<FileEntry> = None;
    let mut hunk: Option<HunkBudget> = None;
    // Stream start is the prologue: mbox headers, commit message, diffstat.
    let mut in_prologue = true;
    // Signature tolerance (rule 7) is earned by an mbox separator.
    let mut seen_mbox_from = false;
    // True from a `From <sha>` separator to that patch's first file header:
    // the only region where column-0 prose can imitate the rule-6 facts.
    let mut in_mbox_message = false;

    fn flush(entries: &mut Vec<FileEntry>, current: &mut Option<FileEntry>) {
        if let Some(e) = current.take() {
            if !e.is_empty() {
                entries.push(e);
            }
        }
    }

    let mut i = 0;
    while i < lines.len() {
        let raw = lines[i];
        // Structural decisions ignore a trailing CR (CRLF streams); content
        // lines are pushed raw so the user's bytes survive verbatim.
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        i += 1;

        // Rule 1: the open hunk's budget owns the line.
        if let Some(h) = hunk.as_mut() {
            if line.starts_with('\\') {
                // "\ No newline at end of file": consumes no budget, but the
                // fact must survive — without it a trailing-newline-only
                // change renders as two byte-identical -/+ lines.
                if let Some(e) = current.as_mut() {
                    e.changes.push(raw.to_string());
                }
                continue;
            }
            let parents = h.old_left.len();
            // A line shorter than the prefix width, or with any non-marker
            // prefix column, contradicts the open budget: fall back rather
            // than guess (a padding tolerance here would silently consume
            // mangled lines as context). Markers are ASCII, so the byte view
            // is exact and allocation-free.
            let lb = line.as_bytes();
            if lb.len() < parents {
                return None;
            }
            let prefix = &lb[..parents];
            if !prefix.iter().all(|b| matches!(b, b' ' | b'-' | b'+')) {
                return None;
            }
            let in_result = !prefix.contains(&b'-');
            for (k, left) in h.old_left.iter_mut().enumerate() {
                // Present in parent k: removed relative to it, or unchanged
                // and present in the result (a ' ' column on a line some
                // other parent removed is filler, not presence).
                if prefix[k] == b'-' || (prefix[k] == b' ' && in_result) {
                    *left = left.checked_sub(1)?;
                }
            }
            if in_result {
                h.new_left = h.new_left.checked_sub(1)?;
            }
            let entry = current.as_mut()?;
            if prefix.contains(&b'-') {
                entry.removed += 1;
                entry.changes.push(raw.to_string());
            } else if prefix.contains(&b'+') {
                entry.added += 1;
                entry.changes.push(raw.to_string());
            }
            if h.exhausted() {
                hunk = None;
            }
            continue;
        }

        // Rule 1b: the new-side no-newline marker lands right after its
        // hunk's budget closed; it still belongs to that hunk's section.
        if line.starts_with('\\') {
            if let Some(e) = current.as_mut().filter(|e| e.saw_hunk) {
                e.changes.push(raw.to_string());
            }
            continue;
        }

        // Rule 2: mbox patch separator → back to the prose prologue.
        if is_mbox_from(line) {
            flush(&mut entries, &mut current);
            in_prologue = true;
            seen_mbox_from = true;
            in_mbox_message = true;
            continue;
        }

        // Rule 3: git file section with extended headers.
        if let Some(rest) = line
            .strip_prefix("diff --git ")
            .or_else(|| line.strip_prefix("diff --cc "))
            .or_else(|| line.strip_prefix("diff --combined "))
        {
            flush(&mut entries, &mut current);
            in_prologue = false;
            in_mbox_message = false;
            // Fallback name only; `rename to` or the `+++` header refine it.
            let name = rest
                .rfind(" b/")
                .map(|p| &rest[p + 3..])
                .unwrap_or(rest)
                .to_string();
            current = Some(FileEntry {
                name,
                ..FileEntry::default()
            });
            continue;
        }
        if let Some(e) = current.as_mut().filter(|e| e.header_only()) {
            if line.starts_with("Binary files ") || line == "GIT binary patch" {
                e.notes.push("binary".to_string());
                continue;
            }
            if let Some(from) = line.strip_prefix("rename from ") {
                e.rename_from = Some(from.to_string());
                continue;
            }
            if let Some(to) = line.strip_prefix("rename to ") {
                e.name = to.to_string();
                let from = e.rename_from.take().unwrap_or_default();
                e.notes.push(format!("renamed from {}", from));
                continue;
            }
            if let Some(from) = line.strip_prefix("copy from ") {
                e.rename_from = Some(from.to_string());
                continue;
            }
            if let Some(to) = line.strip_prefix("copy to ") {
                e.name = to.to_string();
                let from = e.rename_from.take().unwrap_or_default();
                e.notes.push(format!("copied from {}", from));
                continue;
            }
            if (line.starts_with("old mode ") || line.starts_with("new mode "))
                && !e.notes.iter().any(|n| n == "mode changed")
            {
                e.notes.push("mode changed".to_string());
                continue;
            }
            // Without these two arms, a hunkless empty-file section has no
            // changes and no notes and would vanish at flush.
            if line.starts_with("new file mode ") {
                e.notes.push("new file".to_string());
                continue;
            }
            if line.starts_with("deleted file mode ") {
                e.notes.push("deleted".to_string());
                continue;
            }
        }

        // Rule 4: `--- X` + `+++ Y` header pair.
        if let Some(minus) = line.strip_prefix("--- ") {
            let next = lines
                .get(i)
                .map(|r| r.strip_suffix('\r').unwrap_or(r))
                .and_then(|n| n.strip_prefix("+++ "));
            if let Some(plus) = next {
                let name = header_name(minus, plus);
                // A pair opening a NEW section must be followed by a hunk
                // header — every real producer emits one. Without this gate
                // a stray marked pair (a lying budget's leftovers) would be
                // consumed as a phantom header and its two lines lost;
                // gated, it falls through to rule 8 instead. An open git
                // section (extended headers already seen) needs no gate.
                let opens_hunk = lines
                    .get(i + 1)
                    .map(|r| r.strip_suffix('\r').unwrap_or(r))
                    .is_some_and(|n| n.starts_with("@@"));
                let renames_in_place =
                    current.as_ref().is_some_and(|e| e.header_only());
                if renames_in_place || opens_hunk {
                    match current.as_mut().filter(|e| e.header_only()) {
                        Some(e) => e.name = name,
                        None => {
                            flush(&mut entries, &mut current);
                            current = Some(FileEntry {
                                name,
                                ..FileEntry::default()
                            });
                        }
                    }
                    in_prologue = false;
                    in_mbox_message = false;
                    i += 1; // consume the `+++` line too
                    continue;
                }
            }
        }

        // Rule 5: hunk header.
        if line.starts_with("@@") {
            match parse_hunk_header(line) {
                Some((old_left, new_left)) if current.is_some() => {
                    if let Some(e) = current.as_mut() {
                        e.saw_hunk = true;
                    }
                    let h = HunkBudget { old_left, new_left };
                    // `@@ -0,0 +0,0 @@` closes before it opens.
                    if !h.exhausted() {
                        hunk = Some(h);
                    }
                    continue;
                }
                Some(_) => continue, // quoted hunk in prose, no file section
                None if current.is_some() && !in_prologue => return None,
                None => continue,
            }
        }

        // Rule 6: file-level facts outside hunks, suppressed in mbox prose.
        if !in_mbox_message {
            if let Some(rest) = line.strip_prefix("Only in ") {
                if let Some((dir, file)) = rest.rsplit_once(": ") {
                    flush(&mut entries, &mut current);
                    entries.push(FileEntry {
                        name: format!("{}/{}", dir, file),
                        notes: vec!["only in one side".to_string()],
                        ..FileEntry::default()
                    });
                    continue;
                }
            }
            // Standalone GNU `diff -r` form; the git form attaches to its
            // open `diff --git` section in the extended-header block above.
            if let Some(pair) = line
                .strip_prefix("Binary files ")
                .and_then(|r| r.strip_suffix(" differ"))
            {
                let name = pair.rsplit_once(" and ").map(|(_, b)| b).unwrap_or(pair);
                let name = name.strip_prefix("b/").unwrap_or(name).to_string();
                flush(&mut entries, &mut current);
                entries.push(FileEntry {
                    name,
                    notes: vec!["binary".to_string()],
                    ..FileEntry::default()
                });
                continue;
            }
            if let Some(path) = line.strip_prefix("* Unmerged path ") {
                flush(&mut entries, &mut current);
                entries.push(FileEntry {
                    name: path.to_string(),
                    notes: vec!["unmerged".to_string()],
                    ..FileEntry::default()
                });
                continue;
            }
            if let Some(rest) = line.strip_prefix("Submodule ") {
                if rest.contains("..") {
                    flush(&mut entries, &mut current);
                    entries.push(FileEntry {
                        name: rest.trim_end_matches(':').to_string(),
                        notes: vec!["submodule".to_string()],
                        ..FileEntry::default()
                    });
                    continue;
                }
            }
        }

        // Rule 7: format-patch signature separator, only in mbox streams.
        if seen_mbox_from && (line == "--" || line == "-- ") {
            continue;
        }

        // Rule 8: content outside any hunk → stale budget, fall back.
        if (line.starts_with('+') || line.starts_with('-')) && !in_prologue {
            return None;
        }

        // Rule 9: prose.
    }

    // Budget owed at EOF.
    if hunk.is_some() {
        return None;
    }
    flush(&mut entries, &mut current);

    if entries.is_empty() {
        // Nothing recognizable as a diff (plain text, --stat output, empty
        // input): pass through rather than emitting nothing.
        return None;
    }

    let mut out: Vec<String> = Vec::new();
    for e in entries {
        let label = if e.notes.is_empty() {
            format!("[file] {} (+{} -{})", e.name, e.added, e.removed)
        } else if e.changes.is_empty() {
            format!("[file] {} ({})", e.name, e.notes.join(", "))
        } else {
            format!(
                "[file] {} ({}) (+{} -{})",
                e.name,
                e.notes.join(", "),
                e.added,
                e.removed
            )
        };
        out.push(label);
        // Column 0: anchored greps (`^[+-]`) must match these.
        out.extend(e.changes);
    }
    Some(out.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte-level render for a pair of contents, the shape the fork's
    /// identity-check tests exercise.
    fn render_contents(
        file1: &Path,
        file2: &Path,
        content1: &str,
        content2: &str,
    ) -> (String, i32) {
        let lines1: Vec<&str> = content1.lines().collect();
        let lines2: Vec<&str> = content2.lines().collect();
        render_file_diff(file1, file2, content1, content2, &compute_diff(&lines1, &lines2))
    }

    /// The filter's contract in one line, used throughout these tests:
    /// condense strictly, and on structural disagreement pass the input
    /// through unchanged rather than risk silent loss. (Production holds the
    /// same contract at the byte level — see [`condense_stdin`] /
    /// [`run_stdin`].)
    fn condense_unified_diff(diff: &str) -> String {
        condense_unified_diff_strict(diff).unwrap_or_else(|| diff.to_string())
    }

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
        let diff = compute_diff(&["a: 1"], &["a: 2"]);
        let (out, code) = render_diff(Path::new("one.yaml"), Path::new("two.yaml"), &diff);
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
        let (out, code) = render_contents(
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
        let (out, code) = render_contents(
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
        let (out, code) = render_contents(
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

        let (out, code) = render_contents(Path::new("a.txt"), Path::new("b.txt"), &plain, &mixed);
        assert!(!out.contains("identical"), "got: {}", out);
        assert_eq!(code, 1, "must exit 1 so a `diff` gate fails");
        assert!(out.contains("0 CRLF vs 24 CRLF"), "got: {}", out);
    }

    #[test]
    fn test_render_modified_only_json_not_identical() {
        let diff = compute_diff(&["{\"a\": 1}"], &["{\"a\": 2}"]);
        let (out, code) = render_diff(Path::new("j1.json"), Path::new("j2.json"), &diff);
        assert!(
            !out.contains("identical"),
            "modified-only diff reported as identical:\n{}",
            out
        );
        assert_eq!(code, 1);
    }

    #[test]
    fn test_render_identical_files_exit_zero() {
        let diff = compute_diff(&["a: 1", "b: 2"], &["a: 1", "b: 2"]);
        let (out, code) = render_diff(Path::new("a.yaml"), Path::new("b.yaml"), &diff);
        assert!(out.contains("[ok] Files are identical"));
        assert_eq!(code, 0);
    }

    #[test]
    fn test_render_added_removed_exit_one() {
        let diff = compute_diff(&["x"], &["y"]);
        let (out, code) = render_diff(Path::new("t1.txt"), Path::new("t2.txt"), &diff);
        assert!(out.contains("+1 added, -1 removed"));
        assert_eq!(code, 1);
    }

    #[test]
    fn test_never_worse_fallback_is_a_classic_diff() {
        let diff = compute_diff(&["alpha beta"], &["alpha zzzz"]);
        let fallback = format_classic_diff(&diff);
        let (rendered, code) = render_diff(Path::new("before"), Path::new("after"), &diff);
        let shown = select_file_diff_output(&diff, &fallback, &rendered);

        assert_eq!(code, 1);
        assert!(shown.contains("1c1"));
        assert!(shown.contains("< alpha beta"));
        assert!(shown.contains("\n---\n"));
        assert!(shown.contains("> alpha zzzz"));
    }

    #[test]
    fn test_tracking_baseline_never_books_a_loss() {
        // Two unrelated files: the classic diff carries both of them plus the
        // "< " / "> " markers, so it is bigger than a plain dump. Measuring
        // against the dump used to record negative savings.
        let old: Vec<String> = (0..40).map(|i| format!("old line {i}")).collect();
        let new: Vec<String> = (0..40).map(|i| format!("brand new content {i}")).collect();
        let r1: Vec<&str> = old.iter().map(|s| s.as_str()).collect();
        let r2: Vec<&str> = new.iter().map(|s| s.as_str()).collect();

        let diff = compute_diff(&r1, &r2);
        let fallback = format_classic_diff(&diff);
        let both_files = format!("{}\n---\n{}", old.join("\n"), new.join("\n"));
        let (rendered, _) = render_diff(Path::new("a"), Path::new("b"), &diff);
        let shown = select_file_diff_output(&diff, &fallback, &rendered);
        let baseline = tracking_baseline(&diff, &fallback, &both_files, shown);

        assert!(
            tracking::estimate_tokens(baseline) >= tracking::estimate_tokens(shown),
            "baseline {} < shown {} would record negative savings",
            tracking::estimate_tokens(baseline),
            tracking::estimate_tokens(shown)
        );
    }

    #[test]
    fn test_tracking_baseline_identical_files_use_both_files() {
        let diff = compute_diff(&["a: 1", "b: 2"], &["a: 1", "b: 2"]);
        let both_files = "a: 1\nb: 2\n\n---\na: 1\nb: 2\n";
        let shown = "[ok] Files are identical\n";

        assert_eq!(
            tracking_baseline(&diff, "", both_files, shown),
            both_files,
            "identical files should still measure against the dump"
        );
    }

    #[test]
    fn test_tracking_baseline_empty_files_do_not_book_a_loss() {
        // Both files empty: the dump is shorter than the verdict line.
        let diff = compute_diff(&[], &[]);
        let shown = "[ok] Files are identical\n";

        assert_eq!(tracking_baseline(&diff, "", "\n---\n", shown), shown);
    }

    #[test]
    fn test_identical_files_keep_the_success_message() {
        let diff = compute_diff(&["same"], &["same"]);
        let rendered = "[ok] Files are identical\n";

        assert_eq!(select_file_diff_output(&diff, "", rendered), rendered);
    }

    #[test]
    fn test_classic_diff_covers_modified_line_boundary_cases() {
        for (old, new) in [
            ("alpha beta gamma delta", "alpha beta XXXXX delta"),
            ("alpha beta gamma", "alpha beta"),
            ("alpha beta gamma delta", "XXXXX beta gamma delta"),
        ] {
            let diff = compute_diff(&[old], &[new]);
            let fallback = format_classic_diff(&diff);

            assert!(fallback.contains(&format!("< {old}")));
            assert!(fallback.contains(&format!("> {new}")));
        }
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
        assert!(result.contains("[file] a.rs (+1 -0)"));
        assert!(result.contains("[file] b.rs (+0 -1)"));
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
        let diff = "diff --git a/a.rs b/a.rs\n--- a/a.rs\n+++ b/a.rs\n@@ -1 +1 @@\n-fn old() {}\n+fn new() {}\ndiff --git a/b.rs b/b.rs\n--- a/b.rs\n+++ b/b.rs\n@@ -1 +1 @@\n-let x = 1;\n+let x = 2;\n";
        let result = condense_unified_diff(diff);
        for want in ["-fn old() {}", "+fn new() {}", "-let x = 1;", "+let x = 2;"] {
            assert!(
                result.lines().any(|l| l == want),
                "missing {want:?} at column 0 in:\n{}",
                result
            );
        }
        // Match on leading whitespace rather than a single space: the indent
        // this guards against is two spaces, so `" +"` / `" -"` would never
        // fire and the assertion would pass on the very code it rejects.
        assert!(
            !result.lines().any(|l| {
                let trimmed = l.trim_start();
                trimmed.len() != l.len()
                    && (trimmed.starts_with('+') || trimmed.starts_with('-'))
            }),
            "change lines must not be indented:\n{}",
            result
        );
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
            format!("@@ -1,{} +1,{} @@", removed, added),
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
    fn test_condense_unified_diff_never_claims_truncation() {
        // The filter never truncates content, so the old "  ... +N more"
        // trailer was a lie: it claimed 190 lines were elided while all 200
        // were printed right above it. Every change line must be present and
        // no truncation claim made.
        let diff = make_large_unified_diff(100, 100);
        let result = condense_unified_diff(&diff);
        assert!(
            !result.contains("more"),
            "trailer claims truncation that never happens:\n{}",
            result
        );
        assert!(result.contains("(+100 -100)"), "got:\n{}", result);
        for want in ["-old_value_0", "-old_value_99", "+new_value_0", "+new_value_99"] {
            assert!(
                result.lines().any(|l| l == want),
                "missing {want:?} in:\n{}",
                result
            );
        }
    }

    // --- region parser: real-producer fixture corpus ---
    //
    // Every fixture is captured from a real binary (git 2.54 / GNU diff),
    // never synthesized — synthetic fixtures with impossible hunk counts
    // masked bugs for five review rounds (claudedocs/
    // diff-classifier-review-2026-08-29.md). svn is not installed on the
    // capture machine, so an `Index:`-style fixture is a known corpus gap;
    // svn's `--- f (revision N)` / `+++ f (working copy)` headers ride the
    // generic header-pair rule.

    const CORPUS: &[(&str, &str)] = &[
        (
            "git_diff_multifile",
            include_str!("../../../tests/fixtures/diff/git_diff_multifile_raw.txt"),
        ),
        (
            "git_diff_u0",
            include_str!("../../../tests/fixtures/diff/git_diff_u0_raw.txt"),
        ),
        (
            "git_diff_function_context",
            include_str!("../../../tests/fixtures/diff/git_diff_function_context_raw.txt"),
        ),
        (
            "git_diff_rename_delete_binary",
            include_str!("../../../tests/fixtures/diff/git_diff_rename_delete_binary_raw.txt"),
        ),
        (
            "git_log_p",
            include_str!("../../../tests/fixtures/diff/git_log_p_raw.txt"),
        ),
        (
            "git_show_cc",
            include_str!("../../../tests/fixtures/diff/git_show_cc_raw.txt"),
        ),
        (
            "git_format_patch_single",
            include_str!("../../../tests/fixtures/diff/git_format_patch_single_raw.txt"),
        ),
        (
            "git_format_patch_series",
            include_str!("../../../tests/fixtures/diff/git_format_patch_series_raw.txt"),
        ),
        (
            "git_format_patch_cover",
            include_str!("../../../tests/fixtures/diff/git_format_patch_cover_raw.txt"),
        ),
        (
            "diff_u",
            include_str!("../../../tests/fixtures/diff/diff_u_raw.txt"),
        ),
        (
            "diff_ru",
            include_str!("../../../tests/fixtures/diff/diff_ru_raw.txt"),
        ),
        (
            "diff_rn",
            include_str!("../../../tests/fixtures/diff/diff_rn_raw.txt"),
        ),
        (
            "diff_u_crlf",
            include_str!("../../../tests/fixtures/diff/diff_u_crlf_raw.txt"),
        ),
        (
            "git_diff_unmerged",
            include_str!("../../../tests/fixtures/diff/git_diff_unmerged_raw.txt"),
        ),
        (
            "git_format_patch_sha256",
            include_str!("../../../tests/fixtures/diff/git_format_patch_sha256_raw.txt"),
        ),
        (
            "git_diff_no_eol",
            include_str!("../../../tests/fixtures/diff/git_diff_no_eol_raw.txt"),
        ),
    ];

    /// Fixtures whose sections carry no hunks (notes only) — excluded from
    /// the body-line survival replay, which asserts it finds body lines, but
    /// still bound by the no-fallback and never-larger properties.
    const HUNKLESS_CORPUS: &[(&str, &str)] = &[
        (
            "git_diff_copy",
            include_str!("../../../tests/fixtures/diff/git_diff_copy_raw.txt"),
        ),
        (
            "git_diff_mode",
            include_str!("../../../tests/fixtures/diff/git_diff_mode_raw.txt"),
        ),
        (
            "git_diff_submodule",
            include_str!("../../../tests/fixtures/diff/git_diff_submodule_raw.txt"),
        ),
        (
            "git_diff_empty_new_deleted",
            include_str!("../../../tests/fixtures/diff/git_diff_empty_new_deleted_raw.txt"),
        ),
    ];

    /// Property (c): the raw-passthrough safety net fires on ZERO corpus
    /// fixtures — every real producer parses strictly.
    #[test]
    fn corpus_never_falls_back_to_raw() {
        for (name, fixture) in CORPUS.iter().chain(HUNKLESS_CORPUS) {
            assert!(
                condense_unified_diff_strict(fixture).is_some(),
                "{name}: strict parse fell back to raw"
            );
        }
    }

    /// Property (a): every `+`/`-` hunk-body line in the input survives to
    /// the output verbatim, at column 0.
    ///
    /// Body lines are extracted here by replaying only the hunk budgets — an
    /// independent (and deliberately dumber) walk than the parser under test:
    /// it knows nothing about prose, headers, or file sections beyond "a
    /// budget opened at `@@`".
    #[test]
    fn corpus_every_marked_body_line_survives() {
        for (name, fixture) in CORPUS {
            let out = condense_unified_diff(fixture);
            let out_lines: std::collections::HashMap<&str, usize> =
                out.split('\n').fold(std::collections::HashMap::new(), |mut m, l| {
                    *m.entry(l).or_default() += 1;
                    m
                });
            let mut expected: std::collections::HashMap<&str, usize> =
                std::collections::HashMap::new();
            let mut budget: Option<(Vec<usize>, usize)> = None;
            for raw in fixture.split('\n') {
                let line = raw.strip_suffix('\r').unwrap_or(raw);
                if let Some((old, new)) = budget.as_mut() {
                    if line.starts_with('\\') {
                        continue;
                    }
                    let parents = old.len();
                    let prefix: Vec<char> = line.chars().take(parents).collect();
                    assert!(
                        prefix.len() == parents,
                        "replay hit a short hunk line — corpus fixture malformed"
                    );
                    let in_result = !prefix.contains(&'-');
                    for (k, left) in old.iter_mut().enumerate() {
                        if prefix[k] == '-' || (prefix[k] == ' ' && in_result) {
                            *left -= 1;
                        }
                    }
                    if in_result {
                        *new -= 1;
                    }
                    if prefix.contains(&'-') || prefix.contains(&'+') {
                        *expected.entry(raw).or_default() += 1;
                    }
                    if *new == 0 && old.iter().all(|&n| n == 0) {
                        budget = None;
                    }
                    continue;
                }
                if line.starts_with("@@") {
                    if let Some(b) = parse_hunk_header(line) {
                        if !(b.1 == 0 && b.0.iter().all(|&n| n == 0)) {
                            budget = Some(b);
                        }
                    }
                }
            }
            assert!(
                !expected.is_empty(),
                "{name}: replay found no body lines — fixture or replay broken"
            );
            for (body, count) in expected {
                assert!(
                    out_lines.get(body).copied().unwrap_or(0) >= count,
                    "{name}: body line {body:?} (x{count}) missing from output:\n{out}"
                );
            }
        }
    }

    /// Property (b): each `[file]` counter equals the number of marked lines
    /// rendered under it.
    #[test]
    fn corpus_counters_equal_rendered_lines() {
        for (name, fixture) in CORPUS {
            let out = condense_unified_diff(fixture);
            let mut counts: Option<(usize, usize)> = None;
            let (mut added, mut removed) = (0usize, 0usize);
            let check = |counts: Option<(usize, usize)>, added, removed| {
                if let Some((a, r)) = counts {
                    assert_eq!(
                        (a, r),
                        (added, removed),
                        "{name}: counter/content mismatch in:\n{out}"
                    );
                }
            };
            for line in out.split('\n') {
                if line.starts_with("[file] ") {
                    check(counts, added, removed);
                    counts = line
                        .rfind("(+")
                        .and_then(|p| line[p + 2..].strip_suffix(')'))
                        .and_then(|c| c.split_once(" -"))
                        .and_then(|(a, r)| Some((a.parse().ok()?, r.parse().ok()?)));
                    (added, removed) = (0, 0);
                } else if line.starts_with('+') {
                    added += 1;
                } else if line.starts_with('-') {
                    removed += 1;
                } else {
                    // combined-diff lines may carry a leading space column
                    if line.trim_start_matches(' ').starts_with('-') {
                        removed += 1;
                    } else if line.trim_start_matches(' ').starts_with('+') {
                        added += 1;
                    }
                }
            }
            check(counts, added, removed);
        }
    }

    // --- region parser: the reproducers from the design brief ---

    #[test]
    fn sql_comment_removals_survive_and_are_counted() {
        // Reproducer 1: a removed line whose content starts `-- ` is `--- `
        // on the wire; the old prefix classifier read it as a file header and
        // dropped it.
        let fixture = include_str!("../../../tests/fixtures/diff/git_diff_multifile_raw.txt");
        let out = condense_unified_diff(fixture);
        for want in ["--- users table", "--- created 2024", "-  -- legacy column"] {
            assert!(
                out.lines().any(|l| l == want),
                "missing {want:?} in:\n{out}"
            );
        }
        assert!(
            out.contains("[file] schema.sql (+0 -3)"),
            "schema.sql counter under-reports:\n{out}"
        );
    }

    #[test]
    fn plus_plus_content_line_is_not_a_file_header() {
        // Reproducer 2: an added line whose content starts `++` is `+++ ` on
        // the wire; the old classifier renamed the [file] label to it and
        // lost the line.
        let fixture = include_str!("../../../tests/fixtures/diff/git_diff_multifile_raw.txt");
        let out = condense_unified_diff(fixture);
        assert!(
            out.lines().any(|l| l == "+++ can also start a line"),
            "added ++ line lost:\n{out}"
        );
        assert!(
            out.contains("[file] notes.md (+1 -0)"),
            "notes.md label corrupted:\n{out}"
        );
        assert!(
            !out.contains("[file] + can also start a line"),
            "file label renamed to user content:\n{out}"
        );
    }

    #[test]
    fn format_patch_signature_and_prose_are_not_counted() {
        // Reproducer 3 + the round-5 lesson: the `-- ` signature is not a
        // removal, and unindented `- ` commit-message bullets (mbox prose)
        // neither count nor trigger the fallback.
        let fixture =
            include_str!("../../../tests/fixtures/diff/git_format_patch_single_raw.txt");
        let out = condense_unified_diff(fixture);
        assert_ne!(out, fixture, "format-patch fell back to raw");
        assert!(!out.contains("-- \n"), "signature counted as content:\n{out}");
        assert!(
            !out.contains("- remove the"),
            "mbox prose bullet leaked into output:\n{out}"
        );
        assert!(out.contains("[file] schema.sql (+0 -3)"), "got:\n{out}");
    }

    #[test]
    fn deletion_names_the_deleted_file_not_dev_null() {
        // Reproducer 7: `+++ /dev/null` must not become the display name.
        let fixture =
            include_str!("../../../tests/fixtures/diff/git_diff_rename_delete_binary_raw.txt");
        let out = condense_unified_diff(fixture);
        assert!(
            out.contains("[file] doomed.txt (deleted) (+0 -3)"),
            "deletion misnamed:\n{out}"
        );
        assert!(!out.contains("/dev/null"), "got:\n{out}");
    }

    #[test]
    fn copy_only_and_mode_only_sections_are_reported() {
        // Reproducer 8's remaining shapes: `git diff -C` copy sections and
        // pure mode changes carry no hunks and used to vanish.
        let copy = include_str!("../../../tests/fixtures/diff/git_diff_copy_raw.txt");
        let out = condense_unified_diff(copy);
        assert!(
            out.contains("[file] copied_main.rs (copied from main.rs)"),
            "got:\n{out}"
        );
        let mode = include_str!("../../../tests/fixtures/diff/git_diff_mode_raw.txt");
        let out = condense_unified_diff(mode);
        assert!(out.contains("[file] main.rs (mode changed)"), "got:\n{out}");
    }

    #[test]
    fn empty_new_and_deleted_files_are_reported_in_multi_file_streams() {
        // A hunkless `new file mode` / `deleted file mode` section (an empty
        // file added or removed) has no changes and no other note; without
        // its own arm it vanished silently whenever another file in the same
        // stream parsed cleanly.
        let fixture =
            include_str!("../../../tests/fixtures/diff/git_diff_empty_new_deleted_raw.txt");
        let out = condense_unified_diff(fixture);
        assert!(
            out.contains("[file] empty_new.txt (new file)"),
            "empty added file vanished:\n{out}"
        );
        assert!(
            out.contains("[file] empty_seed.txt (deleted)"),
            "empty deleted file vanished:\n{out}"
        );
        assert!(out.contains("[file] main.rs (+1 -0)"), "got:\n{out}");
    }

    #[test]
    fn file_level_facts_survive_while_the_stream_condenses() {
        // GNU `diff -r` interleaves `Only in <dir>: <file>` and standalone
        // `Binary files X and Y differ` lines between file sections; both
        // used to vanish silently whenever a sibling file condensed.
        let fixture = include_str!("../../../tests/fixtures/diff/diff_ru_raw.txt");
        let out = condense_unified_diff(fixture);
        assert!(
            out.contains("[file] b/newfile.txt (only in one side)"),
            "Only-in fact vanished:\n{out}"
        );
        assert!(
            out.contains("[file] img.bin (binary)"),
            "standalone binary fact vanished:\n{out}"
        );
    }

    #[test]
    fn unmerged_paths_are_reported() {
        // `git diff --ours` during a merge conflict opens with
        // `* Unmerged path <file>` BEFORE any file header — the fact arm
        // must fire in a plain stream's prologue.
        let fixture = include_str!("../../../tests/fixtures/diff/git_diff_unmerged_raw.txt");
        let out = condense_unified_diff(fixture);
        assert!(
            out.contains("[file] cfile.txt (unmerged)"),
            "unmerged fact vanished:\n{out}"
        );
        assert!(
            out.contains("[file] cfile.txt (+4 -0)"),
            "conflict-marker section missing:\n{out}"
        );
    }

    #[test]
    fn submodule_log_headers_are_reported() {
        // `git diff --submodule=log` emits a `Submodule <name> <a>..<b>`
        // block whose indented body is prose; the header itself is a fact.
        let fixture = include_str!("../../../tests/fixtures/diff/git_diff_submodule_raw.txt");
        let out = condense_unified_diff(fixture);
        assert!(
            out.contains("[file] sub e139196..b0ac9b1 (rewind) (submodule)"),
            "submodule fact vanished:\n{out}"
        );
    }

    #[test]
    fn sha256_format_patch_parses_with_64_hex_mbox_separator() {
        // SHA-256 repos emit 64-hex `From` separators; without accepting
        // them the whole stream fell back raw (the signature never earned
        // its rule-7 tolerance).
        let fixture =
            include_str!("../../../tests/fixtures/diff/git_format_patch_sha256_raw.txt");
        let out = condense_unified_diff(fixture);
        assert_ne!(out, fixture, "sha256 format-patch fell back to raw");
        assert!(out.contains("[file] f.txt (+1 -1)"), "got:\n{out}");
        assert!(
            !out.contains("- upper-case"),
            "mbox prose bullet leaked:\n{out}"
        );
    }

    #[test]
    fn fact_lines_in_mbox_prose_stay_prose() {
        // A commit message can start a column-0 line with `Only in ` or
        // `Submodule `; inside an mbox message region those are prose, not
        // facts (rule 6 suppression).
        let diff = "From 0e7632a01b00c70cbc9dafcf1f23c71fa6b10de1 Mon Sep 17 00:00:00 2001\nSubject: [PATCH] x\n\nOnly in b: spurious.txt\nSubmodule notes 1..2 were rewritten\n---\ndiff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-x\n+y\n";
        let out = condense_unified_diff(diff);
        assert!(
            !out.contains("spurious.txt") && !out.contains("submodule"),
            "mbox prose promoted to fact entries:\n{out}"
        );
        assert!(out.contains("[file] f (+1 -1)"), "got:\n{out}");
    }

    #[test]
    fn stray_header_pair_without_a_hunk_falls_back_to_raw() {
        // A lying budget can leave `--- x` / `+++ y` leftovers outside any
        // hunk; consuming them as a phantom file header would silently lose
        // both lines. A pair not followed by `@@` is not a header.
        let diff =
            "--- a/f\n+++ b/f\n@@ -1 +1 @@\n-old\n+new\n--- stray removed\n+++ stray added\n";
        assert!(condense_unified_diff_strict(diff).is_none());
        assert_eq!(condense_unified_diff(diff), diff);
    }

    #[test]
    fn signature_tolerance_requires_an_mbox_stream() {
        // A bare `--` leftover in a plain (non-mbox) stream is stale-budget
        // evidence, not a signature; only format-patch streams (which always
        // open with `From <sha>`) earn the rule-6 exclusion.
        let plain = "--- a/f\n+++ b/f\n@@ -1 +1 @@\n-old\n+new\n--\n";
        assert!(condense_unified_diff_strict(plain).is_none());
    }

    #[test]
    fn short_line_inside_hunk_falls_back_to_raw() {
        // A line shorter than the prefix width while a budget is open is a
        // mangled patch (mailers strip trailing whitespace); guessing it into
        // context would silently absorb damage, so it must fall back.
        let diff = "--- a/f\n+++ b/f\n@@ -1,2 +1,2 @@\n-old\n\n+new\n ctx\n";
        assert!(condense_unified_diff_strict(diff).is_none());
    }

    // --- condense_stdin: the decode → strip-ANSI → parse → guard pipeline ---

    #[test]
    fn stdin_strips_ansi_before_parsing() {
        // Reproducer 9: `git diff --color` through a pipe used to condense to
        // a silently empty result.
        let colored = "\u{1b}[1mdiff --git a/x b/x\u{1b}[m\n\u{1b}[1m--- a/x\u{1b}[m\n\u{1b}[1m+++ b/x\u{1b}[m\n\u{1b}[36m@@ -1 +1 @@\u{1b}[m\n\u{1b}[31m-old_line_content\u{1b}[m\n\u{1b}[32m+new_line_content\u{1b}[m\n";
        let out = condense_stdin(colored.as_bytes()).expect("colored diff must parse");
        assert!(out.contains("[file] x (+1 -1)"), "got:\n{out}");
        assert!(out.lines().any(|l| l == "-old_line_content"));
    }

    #[test]
    fn stdin_non_utf8_non_diff_falls_back_to_exact_bytes() {
        // Reproducer 10: non-UTF-8 stdin used to be a hard error. When the
        // stream is not a diff, the fallback must signal "emit the exact
        // bytes" — a lossy re-encode of unparsed input would corrupt it.
        let bytes = b"not a diff at all \xff\xfe just text\n";
        assert!(condense_stdin(bytes).is_none());
    }

    #[test]
    fn stdin_non_utf8_diff_takes_the_raw_bytes_path() {
        // Even a parseable diff falls back when its content bytes are not
        // UTF-8: condensing would rewrite the user's bytes to U+FFFD, and
        // byte fidelity outranks savings. (The base code hard-errored here;
        // raw passthrough is strictly better on both counts.)
        let bytes: &[u8] =
            b"diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1 +1 @@\n-caf\xe9 old\n+caf\xe9 new\n";
        assert!(condense_stdin(bytes).is_none());
    }

    #[test]
    fn binary_and_rename_only_files_are_reported() {
        // Reproducer 8: binary and rename-only sections used to vanish.
        let fixture =
            include_str!("../../../tests/fixtures/diff/git_diff_rename_delete_binary_raw.txt");
        let out = condense_unified_diff(fixture);
        assert!(out.contains("[file] blob.bin (binary)"), "got:\n{out}");
        assert!(
            out.contains("[file] renamed_dst.txt (renamed from renamed_src.txt)"),
            "got:\n{out}"
        );
    }

    #[test]
    fn combined_diff_hunks_parse_with_two_parents() {
        let fixture = include_str!("../../../tests/fixtures/diff/git_show_cc_raw.txt");
        let out = condense_unified_diff(fixture);
        assert_ne!(out, fixture, "combined diff fell back to raw");
        for want in [
            "- conflict line MAIN",
            " -conflict line LEFT",
            "++conflict line RESOLVED",
        ] {
            assert!(
                out.lines().any(|l| l == want),
                "missing {want:?} in:\n{out}"
            );
        }
        assert!(out.contains("[file] cfile.txt (+1 -2)"), "got:\n{out}");
    }

    #[test]
    fn no_newline_marker_survives_to_the_output() {
        // A trailing-newline-only change is `-content` / `+content` plus
        // `\ No newline at end of file`; dropping the marker leaves two
        // byte-identical lines and no witness of the actual difference.
        let fixture = include_str!("../../../tests/fixtures/diff/git_diff_no_eol_raw.txt");
        let out = condense_unified_diff(fixture);
        assert_ne!(out, fixture, "no-eol diff fell back to raw");
        assert!(
            out.lines().any(|l| l == "\\ No newline at end of file"),
            "no-newline marker lost:\n{out}"
        );
        assert!(out.contains("[file] eol.txt (+1 -1)"), "got:\n{out}");
    }

    #[test]
    fn crlf_content_bytes_survive_verbatim() {
        // Reproducer 11: `lines()` stripped the `\r`, so a CRLF-only change
        // rendered as two identical lines. Content is now byte-faithful.
        let fixture = include_str!("../../../tests/fixtures/diff/diff_u_crlf_raw.txt");
        let out = condense_unified_diff(fixture);
        assert!(
            out.split('\n').any(|l| l == "-change me\r"),
            "CR byte lost from removed line:\n{out:?}"
        );
        assert!(
            out.split('\n').any(|l| l == "+change me now\r"),
            "CR byte lost from added line:\n{out:?}"
        );
    }

    #[test]
    fn plain_diff_u_timestamps_do_not_pollute_the_name() {
        // Reproducer 12 (second half): `diff -u` appends `\t<timestamp>` to
        // the header names.
        let fixture = include_str!("../../../tests/fixtures/diff/diff_u_raw.txt");
        let out = condense_unified_diff(fixture);
        let label = out.lines().next().unwrap_or("");
        assert!(
            label.starts_with("[file] ") && !label.contains("2026-"),
            "timestamp leaked into name: {label}"
        );
    }

    #[test]
    fn b_prefix_is_stripped_exactly_once() {
        // Reproducer 12 (first half): `trim_start_matches("b/")` stripped
        // repeatedly, so `b/b/x.rs` (a file in a literal `b/` directory)
        // became `x.rs`.
        let diff = "diff --git a/b/x.rs b/b/x.rs\n--- a/b/x.rs\n+++ b/b/x.rs\n@@ -1 +1 @@\n-old\n+new\n";
        let out = condense_unified_diff(diff);
        assert!(out.contains("[file] b/x.rs (+1 -1)"), "got:\n{out}");
    }

    #[test]
    fn u0_and_omitted_counts_parse() {
        // `-U0` produces `@@ -3 +3 @@` (omitted count = 1) and zero-count
        // ranges like `@@ -5,0 +6 @@`.
        let fixture = include_str!("../../../tests/fixtures/diff/git_diff_u0_raw.txt");
        let out = condense_unified_diff(fixture);
        assert_ne!(out, fixture, "-U0 fell back to raw");
        assert!(out.contains("[file] main.rs (+2 -1)"), "got:\n{out}");
    }

    // --- region parser: the safety net must fire on structural disagreement ---

    #[test]
    fn truncated_hunk_falls_back_to_raw() {
        // Reproducer 5 (budget owed at EOF): a stream cut mid-hunk must pass
        // through raw, not render a partial hunk as complete.
        let fixture = include_str!("../../../tests/fixtures/diff/git_diff_multifile_raw.txt");
        let cut: String = fixture
            .split('\n')
            .take(7) // ends inside the first hunk
            .collect::<Vec<_>>()
            .join("\n");
        assert!(
            condense_unified_diff_strict(&cut).is_none(),
            "truncated hunk did not fall back"
        );
        assert_eq!(condense_unified_diff(&cut), cut);
    }

    #[test]
    fn understated_budget_falls_back_to_raw() {
        // Reproducer 5 (stale count, under-declared): leftover marked lines
        // after the budget closes are content outside any hunk.
        let diff = "--- a/f\n+++ b/f\n@@ -1,1 +1,1 @@\n-old\n+new\n+leftover the budget missed\n";
        assert!(condense_unified_diff_strict(diff).is_none());
        assert_eq!(condense_unified_diff(diff), diff);
    }

    #[test]
    fn overstated_budget_falls_back_to_raw() {
        // Reproducer 5 (stale count, over-declared): the budget still owes
        // lines when the next file header arrives; the header's `d` fails the
        // body-prefix check.
        let diff = "--- a/f\n+++ b/f\n@@ -1,3 +1,3 @@\n-old\n+new\ndiff --git a/g b/g\n--- a/g\n+++ b/g\n@@ -1 +1 @@\n-x\n+y\n";
        assert!(condense_unified_diff_strict(diff).is_none());
    }

    #[test]
    fn malformed_hunk_header_falls_back_to_raw() {
        let diff = "--- a/f\n+++ b/f\n@@ garbage @@\n-old\n+new\n";
        assert!(condense_unified_diff_strict(diff).is_none());
    }

    #[test]
    fn non_diff_input_passes_through() {
        // `--color` streams, --stat output, plain text: nothing recognizable
        // means raw passthrough, never a silently empty result.
        let ansi = "\u{1b}[1mbold header\u{1b}[m\nplain text\n";
        assert_eq!(condense_unified_diff(ansi), ansi);
        let stat = " main.rs | 3 ++-\n 1 file changed, 2 insertions(+), 1 deletion(-)\n";
        assert_eq!(condense_unified_diff(stat), stat);
    }

    #[test]
    fn empty_zero_zero_hunk_closes_immediately() {
        // Reproducer 6: `@@ -0,0 +0,0 @@` owes nothing; the next line belongs
        // to the following region.
        let diff = "--- a/f\n+++ b/f\n@@ -0,0 +0,0 @@\ndiff --git a/g b/g\n--- a/g\n+++ b/g\n@@ -1 +1 @@\n-x\n+y\n";
        let out = condense_unified_diff(diff);
        assert!(out.contains("[file] g (+1 -1)"), "got:\n{out}");
    }

    // --- token accounting (fidelity filter: content kept, metadata dropped) ---

    fn count_tokens(s: &str) -> usize {
        s.split_whitespace().count()
    }

    #[test]
    fn condensed_output_is_never_larger_than_input() {
        // This filter is a fidelity filter: it keeps every content line by
        // design, so its savings come only from dropped metadata. Measured on
        // this corpus (metadata-heavy streams) that is 52-87% per fixture;
        // on content-heavy single-file diffs it can fall to single digits
        // (~4% on this branch's own self-diff). The 60% floor in
        // cli-testing.md is therefore not guaranteed by construction — the
        // fidelity-filter exemption is escalated on the ticket as a
        // maintainer decision. What must always hold: the output is never
        // larger than the input (the `never_worse` guard's contract,
        // verified here at the filter level). Percentages above are by this
        // test's whitespace-token metric; the runtime guard uses
        // `estimate_tokens` (bytes/4), which shifts individual numbers.
        for (name, fixture) in CORPUS.iter().chain(HUNKLESS_CORPUS) {
            let out = condense_unified_diff(fixture);
            assert!(
                count_tokens(&out) <= count_tokens(fixture),
                "{name}: output grew"
            );
        }
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
}
