//! Compares two files and shows only the changed lines.

use crate::core::guard::never_worse;
use crate::core::tracking;
use anyhow::{Context, Result};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;

/// A format-patch diffstat body line: ` <path> | <count>[ <+- graph>]`, or the
/// binary form ` <path> | Bin [<n> -> <m> bytes]`. Anchored on the count-and-
/// signs right-hand side rather than a bare ` | `, so a commit message's
/// markdown table (` col | val`, ` --- | ---`) is prose, not a diffstat. The
/// bare-count form has no graph: a pure rename or mode change stats as `0`.
static MBOX_DIFFSTAT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^ .+\| *(?:\d+(?: [-+]+)?|Bin\b.*) *$").unwrap());

/// Column-0 marked shapes a commit message writes: a `- text` / `+ text`
/// bullet, a rule or table separator (`-`, `--`, `---------`), and a long
/// option (`--no-stat`) that prose wrapped to the start of a line. Hunk
/// content carries its marker against the line's own first byte, so a body
/// line takes one of these shapes only by coincidence: over 111k marked
/// lines from 698 real diffs, 3.4% are prose-shaped and no whole diff is
/// entirely so. The long-option arm is what admitting it costs: 4 more of
/// those 111k lines, and still no whole diff. Wrapping a *short* option
/// (`-p`) is left out — `-` plus a letter is the commonest diff line there
/// is, and admitting it would answer the question with "always prose".
static MARKED_PROSE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:[-+] \S|-+\s*$|--[A-Za-z])").unwrap());

const IDENTICAL_FILES_MESSAGE: &str = "[ok] Files are identical\n";
const WHITESPACE_ONLY_DIFF_DETAIL: &str =
    "   files differ only in whitespace or line endings (no line-content change)\n";

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

    let (rtk, exit_code) = render_diff(file1, file2, &diff, content1 == content2);

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

fn render_file_header(file1: &Path, file2: &Path) -> String {
    format!("{} → {}\n", file1.display(), file2.display())
}

fn render_diff(file1: &Path, file2: &Path, diff: &DiffResult, bytes_equal: bool) -> (String, i32) {
    if diff.changes.is_empty() {
        if bytes_equal {
            return (IDENTICAL_FILES_MESSAGE.to_string(), 0);
        }
        // `str::lines()` strips `\r` and drops a trailing newline, so these
        // byte-level differences can leave no line changes to render.
        return (
            format!(
                "{}{}",
                render_file_header(file1, file2),
                WHITESPACE_ONLY_DIFF_DETAIL
            ),
            1,
        );
    }

    let mut rtk = String::new();
    rtk.push_str(&render_file_header(file1, file2));
    rtk.push_str(&format!(
        "   +{} added, -{} removed, ~{} modified\n\n",
        diff.added, diff.removed, diff.modified
    ));
    rtk.push_str(&format_diff_changes(diff));
    (rtk, 1)
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

/// Filter a piped stream: parse strictly (the parser reads structure through
/// an ANSI-stripped view, so a `git diff --color` stream parses instead of
/// condensing to silence, while content lines keep their bytes) and apply
/// the never-worse check — inlined rather than via `guard::never_worse`,
/// which hands back the winning `&str`, where this path needs `None` to
/// mean byte-exact fallback. `None` means the caller must emit its exact
/// input bytes — including for non-UTF-8 input, where filtering would
/// rewrite the user's content bytes to U+FFFD (byte fidelity outranks
/// savings here).
fn condense_stdin(bytes: &[u8]) -> Option<String> {
    let input = std::str::from_utf8(bytes).ok()?;
    // PowerShell 5.1's `>` writes a BOM; without this the first `diff
    // --git` line is prose and a binary first section names itself wrong.
    let input = input.strip_prefix('\u{feff}').unwrap_or(input);
    let condensed = condense_unified_diff_strict(input)?;
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

fn compute_diff(lines1: &[&str], lines2: &[&str]) -> DiffResult {
    let mut changes = Vec::new();
    let mut added = 0;
    let mut removed = 0;
    let mut modified = 0;

    // Simple line-by-line comparison (not optimal but fast)
    let max_len = lines1.len().max(lines2.len());

    for i in 0..max_len {
        let l1 = lines1.get(i).copied();
        let l2 = lines2.get(i).copied();

        match (l1, l2) {
            (Some(a), Some(b)) if a != b => {
                // Check if it's similar (modification) or completely different
                if similarity(a, b) > 0.5 {
                    changes.push(DiffChange::Modified(i + 1, a.to_string(), b.to_string()));
                    modified += 1;
                } else {
                    changes.push(DiffChange::Removed(i + 1, a.to_string()));
                    changes.push(DiffChange::Added(i + 1, b.to_string()));
                    removed += 1;
                    added += 1;
                }
            }
            (Some(a), None) => {
                changes.push(DiffChange::Removed(i + 1, a.to_string()));
                removed += 1;
            }
            (None, Some(b)) => {
                changes.push(DiffChange::Added(i + 1, b.to_string()));
                added += 1;
            }
            _ => {}
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
    /// The prefixes this section's `---`/`+++` names carry, as the
    /// `diff --git X Y` line settled them: `("a", "b")` by default,
    /// `("i", "w")` etc. under `diff.mnemonicPrefix` or `--src-prefix`/
    /// `--dst-prefix`, `("", "")` for `--no-prefix` (X == Y) and for
    /// producers that never prefix (GNU diff, svn). `None` for a section
    /// opened by a header pair alone, where the pair itself decides.
    prefixes: Option<(String, String)>,
    /// Opened by a `diff --git` line. Such a section that closes with
    /// nothing parsed — no hunk, no note — is a stream cut off right after
    /// its header (git never emits one) → the stream falls back raw.
    from_git: bool,
    /// Opened by an svn `Index: <path>` line. Such a section that closes
    /// with nothing parsed is either a copy/move target svn has nothing to
    /// show for (`(no content)`) or a file svn described in words the
    /// parser did not read (a localized binary notice) → raw; `unread`
    /// tells the two apart.
    from_index: bool,
    unread: bool,
}

/// The line as the parser reads it for structural decisions: ANSI escapes
/// removed (a `--color` stream wraps every line, marker included) and a
/// trailing CR dropped (CRLF streams). Content lines are pushed from the raw
/// line, never from this view, so escapes and CRs that are part of the
/// user's content survive verbatim.
fn structural(raw: &str) -> std::borrow::Cow<'_, str> {
    use std::borrow::Cow;
    let view = if raw.contains('\x1b') {
        Cow::Owned(crate::core::utils::strip_ansi(raw))
    } else {
        Cow::Borrowed(raw)
    };
    match view {
        Cow::Borrowed(v) => Cow::Borrowed(v.strip_suffix('\r').unwrap_or(v)),
        Cow::Owned(mut v) => {
            if v.ends_with('\r') {
                v.pop();
            }
            Cow::Owned(v)
        }
    }
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
    /// `-`/`+` lines consumed so far. A hunk that closes with none is not a
    /// shape any unified-diff producer emits (whitespace-ignoring modes
    /// suppress the hunk instead); it is `--word-diff` output whose inline
    /// markers all sit behind an indent, booked as context → `None`.
    marked: usize,
}

impl HunkBudget {
    fn exhausted(&self) -> bool {
        self.new_left == 0 && self.old_left.iter().all(|&n| n == 0)
    }
}

/// hg's per-file echo: `diff -r <12 hex> -r <12 hex> <path>` from `hg
/// export` / `hg log -p`, `diff -r <12 hex> <path>` from `hg diff`.
fn is_hg_echo(line: &str) -> bool {
    let hex12 = |s: &str| s.len() == 12 && s.bytes().all(|b| b.is_ascii_hexdigit());
    let mut t = line.splitn(6, ' ');
    match (t.next(), t.next(), t.next(), t.next(), t.next(), t.next()) {
        (Some("diff"), Some("-r"), Some(a), Some("-r"), Some(b), Some(_)) => hex12(a) && hex12(b),
        (Some("diff"), Some("-r"), Some(a), Some(_), _, _) => hex12(a),
        _ => false,
    }
}

/// git's `Submodule <path> <hex>..<hex>[ (<how>)]:` header in its strict
/// shape — the one form of the fact admitted inside an mbox message.
fn is_submodule_range(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("Submodule ") else {
        return false;
    };
    let Some(rest) = rest.strip_suffix(':') else {
        return false;
    };
    let rest = match rest.rsplit_once(" (") {
        Some((r, q)) if q.ends_with(')') => r,
        _ => rest,
    };
    let hex = |s: &str| s.len() >= 7 && s.bytes().all(|b| b.is_ascii_hexdigit());
    rest.rsplit(' ')
        .next()
        .and_then(|range| range.split_once(".."))
        .is_some_and(|(a, b)| hex(a) && hex(b.trim_start_matches('.')))
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

/// Decode C-style quoting: the `"…"` git wraps around a path under
/// `core.quotepath` (the default for any non-ASCII byte, and always for `"`,
/// `\` and control characters), and the same form GNU diff uses on its
/// `diff …` echo and `---`/`+++` lines. `\ooo` octal bytes, `\"`, `\\` and
/// the C control escapes are decoded so the name is the file's own —
/// `caf\303\251.txt` is a name nothing downstream can open. An unquoted
/// string is returned as is. A name whose decoding is not UTF-8, or would
/// split a `[file]` line (a newline), keeps git's quoted spelling, whole.
fn dequote(s: &str) -> std::borrow::Cow<'_, str> {
    match s.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        Some(inner) if inner.contains('\\') => match decode_backslashes(inner, false) {
            // A name a `[file]` line cannot carry (a newline would split
            // the label) or that is not UTF-8 keeps git's own lossless
            // spelling rather than a decoded one that lies.
            Some(d) if !d.contains(['\n', '\r']) => std::borrow::Cow::Owned(d),
            _ => std::borrow::Cow::Borrowed(s),
        },
        Some(inner) => std::borrow::Cow::Borrowed(inner),
        None => std::borrow::Cow::Borrowed(s),
    }
}

/// Decode GNU diff's shell quoting on `Only in` and `Binary files` names
/// (diffutils ≥ 3.11; 3.10 prints them bare): a run of `'literal'` segments,
/// `$'…'` ANSI-C segments (`\ooo`, `\xHH`, `\\`, `\'`, control escapes),
/// `"…"` segments (chosen when the name holds a `'`; only `\"`, `\\`, `\$`
/// and `` \` `` escape inside), a bare `\'` between segments, and bare
/// characters — `'only '$'\303\251''.txt'` is `only é.txt`, `"Kyle's
/// notes.txt"` is `Kyle's notes.txt`. A string that does not open with a
/// quote, or whose decoding is not UTF-8 or would split a `[file]` line,
/// is returned as is.
fn unquote_shell(s: &str) -> std::borrow::Cow<'_, str> {
    if !(s.starts_with('\'') || s.starts_with("$'") || s.starts_with('"')) {
        return std::borrow::Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while !rest.is_empty() {
        if let Some(r) = rest.strip_prefix("$'") {
            let end = unescaped_quote(r, b'\'');
            match decode_backslashes(&r[..end], true) {
                Some(d) => out.push_str(&d),
                None => return std::borrow::Cow::Borrowed(s),
            }
            rest = r.get(end + 1..).unwrap_or("");
        } else if let Some(r) = rest.strip_prefix('"') {
            let end = unescaped_quote(r, b'"');
            let mut seg = r[..end].chars();
            while let Some(c) = seg.next() {
                match c {
                    '\\' => match seg.next() {
                        Some(e @ ('"' | '\\' | '$' | '`')) => out.push(e),
                        Some(e) => {
                            out.push('\\');
                            out.push(e);
                        }
                        None => out.push('\\'),
                    },
                    c => out.push(c),
                }
            }
            rest = r.get(end + 1..).unwrap_or("");
        } else if let Some(r) = rest.strip_prefix('\'') {
            let end = r.find('\'').unwrap_or(r.len());
            out.push_str(&r[..end]);
            rest = r.get(end + 1..).unwrap_or("");
        } else if let Some(r) = rest.strip_prefix('\\') {
            // A bare `\'` between segments is how the shell style spells a
            // single quote: `'it'\''s'`.
            let mut ch = r.chars();
            if let Some(c) = ch.next() {
                out.push(c);
            }
            rest = ch.as_str();
        } else {
            let end = rest.find(['\'', '\\', '"']).unwrap_or(rest.len());
            out.push_str(&rest[..end]);
            rest = &rest[end..];
        }
    }
    if out.contains(['\n', '\r']) {
        return std::borrow::Cow::Borrowed(s);
    }
    std::borrow::Cow::Owned(out)
}

/// Byte offset of the first `quote` in `s` not preceded by a backslash
/// (`s.len()` if none). Escapes are ASCII, so the offset is a char boundary.
fn unescaped_quote(s: &str, quote: u8) -> usize {
    let b = s.as_bytes();
    let mut i = 0;
    while i < b.len() && b[i] != quote {
        i += if b[i] == b'\\' { 2 } else { 1 };
    }
    i.min(b.len())
}

/// Byte length of the shell-quoted word that opens `s` — the run of `'…'`,
/// `$'…'`, `"…"` and bare-`\x` segments up to the first unquoted character
/// — or `None` when `s` does not open with a quote. Lets `Only in <dir>:
/// <file>` find the `: ` after a quoted directory that itself contains one.
fn shell_word_len(s: &str) -> Option<usize> {
    if !(s.starts_with('\'') || s.starts_with("$'") || s.starts_with('"')) {
        return None;
    }
    let mut i = 0;
    loop {
        let rest = &s[i..];
        if let Some(r) = rest.strip_prefix("$'") {
            i += 2 + (unescaped_quote(r, b'\'') + 1).min(r.len());
        } else if let Some(r) = rest.strip_prefix('"') {
            i += 1 + (unescaped_quote(r, b'"') + 1).min(r.len());
        } else if let Some(r) = rest.strip_prefix('\'') {
            i += 1 + r.find('\'').map_or(r.len(), |e| e + 1);
        } else if let Some(r) = rest.strip_prefix('\\') {
            i += 1 + r.chars().next().map_or(0, char::len_utf8);
        } else {
            return Some(i);
        }
    }
}

/// Decode backslash escapes into bytes: `\ooo` (1–3 octal digits), `\xHH`
/// (ANSI-C only), `\a \b \e \f \n \r \t \v`, and `\<c>` for any other `<c>`
/// (`\"`, `\'`, `\\`). `None` when the bytes are not UTF-8 — the caller
/// keeps the producer's own spelling then, which is lossless.
fn decode_backslashes(s: &str, ansi_c: bool) -> Option<String> {
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'\\' || i + 1 >= b.len() {
            out.push(b[i]);
            i += 1;
            continue;
        }
        let c = b[i + 1];
        i += 2;
        match c {
            b'0'..=b'7' => {
                let mut v = (c - b'0') as u32;
                let mut n = 1;
                while n < 3 && i < b.len() && (b'0'..=b'7').contains(&b[i]) {
                    v = v * 8 + (b[i] - b'0') as u32;
                    i += 1;
                    n += 1;
                }
                out.push(v as u8);
            }
            b'x' if ansi_c => {
                let mut v = 0u32;
                let mut n = 0;
                while n < 2 && i < b.len() && (b[i] as char).is_ascii_hexdigit() {
                    v = v * 16 + (b[i] as char).to_digit(16).unwrap_or(0);
                    i += 1;
                    n += 1;
                }
                if n == 0 {
                    out.extend_from_slice(b"\\x");
                } else {
                    out.push(v as u8);
                }
            }
            b'a' => out.push(7),
            b'b' => out.push(8),
            b'e' if ansi_c => out.push(27),
            b'f' => out.push(12),
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            b'v' => out.push(11),
            other => out.push(other),
        }
    }
    String::from_utf8(out).ok()
}

/// Name a section from its `---`/`+++` pair (already timestamp-stripped and
/// as printed — quoting is decoded here). `prefixes` are the directories
/// the `diff --git` line settled, or `None` for a pair on its own, where
/// the pair decides: two different names are git's `a/`/`b/` prefixes, the
/// same name twice is `--no-prefix`.
fn header_name(minus: &str, plus: &str, prefixes: Option<(&str, &str)>) -> String {
    let minus = dequote(minus);
    let plus = dequote(plus);
    let (minus, plus): (&str, &str) = (&minus, &plus);
    let (px, py) = match prefixes {
        Some(p) => p,
        None if minus != plus => ("a", "b"),
        None => ("", ""),
    };
    if plus == "/dev/null" {
        strip_quoted_prefix(minus, px)
    } else {
        strip_quoted_prefix(plus, py)
    }
}

/// Strip `<prefix>/` from a name — inside the quotes when the name kept
/// git's quoted spelling (`"b/nl\nx.txt"` → `"nl\nx.txt"`), so one file
/// has one spelling across a stream whichever line named it.
fn strip_quoted_prefix(side: &str, prefix: &str) -> String {
    if prefix.is_empty() {
        return side.to_string();
    }
    fn strip<'a>(s: &'a str, prefix: &str) -> Option<&'a str> {
        s.strip_prefix(prefix).and_then(|s| s.strip_prefix('/'))
    }
    match side.strip_prefix('"') {
        Some(inner) => match strip(inner, prefix) {
            Some(rest) => format!("\"{rest}"),
            None => side.to_string(),
        },
        None => strip(side, prefix).unwrap_or(side).to_string(),
    }
}

/// Drop the timestamp GNU diff, hg and svn append after a tab on `---`/`+++`
/// lines (`\t2026-09-07 …`, `\tMon Sep 07 …`, `\t(revision 1)`), and the
/// bare tab git appends after a name containing a space. Split on the LAST
/// tab, and only when what follows is timestamp-shaped: a name containing a
/// tab (diffutils 3.10 prints it bare) is not cut.
fn strip_timestamp(s: &str) -> &str {
    // The shapes real producers append: nothing (git, after a name with a
    // space), `(revision 1)` / `(working copy)` / `(nonexistent)` (svn),
    // `2026-09-07 09:04:54.7 -0400` (GNU), `Mon Sep 07 12:55:24 2026 +0000`
    // (hg, with dates). A name such as `v\t2.txt` or `ta\tYes.txt` under
    // `hg diff --nodates` / `--git` matches none of them.
    let timestamp_shaped = |t: &str| {
        let b = t.as_bytes();
        t.is_empty()
            || t.starts_with('(')
            || (b.len() >= 10
                && b[..4].iter().all(u8::is_ascii_digit)
                && b[4] == b'-'
                && b[7] == b'-'
                && (b.len() == 10 || b[10] == b' '))
            || (b.len() > 8
                && b[..3].iter().all(u8::is_ascii_alphabetic)
                && b[3] == b' '
                && b[4..7].iter().all(u8::is_ascii_alphabetic)
                && b[7] == b' '
                && b[8].is_ascii_digit())
    };
    match s.rsplit_once('\t') {
        Some((name, tail)) if timestamp_shaped(tail) => name,
        _ => s,
    }
}

/// The tail X and Y share at a `/` boundary (or one whole side), as a byte
/// length: `a/x/f` and `b/x/f` share `/x/f`; `./f` and `../new/f` share
/// `/f`; `x.bin` and `y.bin` share nothing. GNU diff and git both name the
/// same path under two roots, so this is what tells the X/Y boundary from
/// a ` and ` or a space inside a name, and what the roots are.
fn shared_tail(x: &str, y: &str) -> Option<usize> {
    let (xb, yb) = (x.as_bytes(), y.as_bytes());
    let mut n = 0;
    while n < xb.len() && n < yb.len() && xb[xb.len() - 1 - n] == yb[yb.len() - 1 - n] {
        n += 1;
    }
    // Two different multi-byte characters can share trailing bytes (`😀`
    // and `🙀`, `é` and `©`): back off to a character boundary on both.
    while n > 0 && !(x.is_char_boundary(x.len() - n) && y.is_char_boundary(y.len() - n)) {
        n -= 1;
    }
    if n == 0 {
        return None;
    }
    // One whole side is the tail: the other must hold it under a `/`.
    let at_boundary = |b: &[u8]| n == b.len() || b[b.len() - 1 - n] == b'/';
    if (n == xb.len() && at_boundary(yb)) || (n == yb.len() && at_boundary(xb)) {
        return Some(n);
    }
    // Otherwise back off to the first `/` inside the common suffix.
    let tail = &x[x.len() - n..];
    tail.find('/').map(|p| n - p).filter(|&m| m > 1)
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
///    `\ No newline at end of file` marker consumes no budget. It describes
///    the line directly above it, so it is kept as content exactly when
///    that line was emitted — in-hunk after a `-`/`+` line, or on the line
///    right after the budget closed on one — because it is the only
///    witness of a newline-only change. After a context line, which the
///    output never carries, it is dropped: kept, it would sit under the
///    last marked line and describe the wrong one. A hunk that closes
///    with no `-`/`+` line at all is not a shape any unified-diff producer
///    emits (whitespace-ignoring modes suppress the hunk): it is
///    `--word-diff` / `--color-words` output whose inline markers sit
///    behind an indent, booked as context → `None`.
///    1b. The new-side marker lands on the line right after its hunk's
///    budget closed and is kept by the same test; a `\` line anywhere
///    else is prose (its text is localized, so position decides).
/// 2. An mbox `From <sha>` separator, an `hg export` changeset header
///    (`# HG changeset patch`) or an `hg log -p` `changeset:` line resets
///    to the prose prologue. The hg region differs in two ways: rule 6's
///    fact arms stay live in it (hg emits nothing that needs suppressing,
///    and a lost fact outranks a phantom entry), and hg's own `diff -r`
///    echo closes it (rule 3b) and counts as reaching its body — every hg
///    file gets one, binary files included.
/// 3. `diff --git` / `diff --cc` opens a file section (git extended headers
///    such as `rename`/`Binary files`/mode lines annotate it). `diff --git
///    X Y` names one path under two prefixes — `a/`/`b/` by default,
///    `i/`/`w/`/`c/`/`o/` under `diff.mnemonicPrefix`, anything under
///    `--src-prefix`/`--dst-prefix`, none under `--no-prefix` (X == Y) —
///    so the sides are split where they share a tail at a `/` boundary,
///    what precedes the tail on each side is that side's prefix, and the
///    header pair strips exactly those. `diff --cc` / `--combined` carry
///    one unprefixed path, kept whole. A section opened by `diff --git`
///    that closes with nothing behind it — no hunk, no note — is a stream
///    cut off right after its header (git never emits one) → `None`.
///    3b. A `diff -<opts> X Y` line that is not git's — GNU diff's per-file
///    echo when it compares directories, or hg's `diff -r <rev> [-r <rev>]
///    <file>` — is consumed as the mark of a `diff -r` stream (rule 9); it
///    is keyed on `diff -` because every echo carries an option and a
///    commit body line under `--format=%B` that starts with `diff ` does
///    not. Inside an mbox message region only hg's form is read; a
///    GNU-shaped echo there is prose, like rule 6's facts. hg's form also
///    closes an hg message region. After a GNU echo the roots diff was
///    given are directories, so header pairs keep them whole and the
///    roots are remembered for rule 6; hg prefixes `a/`/`b/` like git.
///    3c. svn's `Index: <path>` opens a file section; its `===` rule and
///    `svn:mime-type = …` lines are prose, `Cannot display: file marked as
///    a binary type.` is its binary note, and the header pair or `diff
///    --git` line that follows names it again. An `Index:` section that
///    closes with nothing parsed is a copy/move target svn has nothing to
///    show for — `(no content)` — unless a column-0 line was dropped
///    inside it, which is svn describing that file in words the parser
///    did not read → `None`. (Bound: an svn property change is a header
///    pair with no hunk followed by a `## -a,b +c,d ##` block, so rule 8
///    sends the stream raw — noise, not loss.)
/// 4. A `--- X` line immediately followed by `+++ Y` whose next line opens
///    a hunk — as every real producer's does — is a file header: it names
///    a still-hunkless section in place (unless git's `rename to`/`copy
///    to` already named it exactly), or opens a new section. A pair with
///    no hunk behind it is never consumed, open section or not: it falls
///    to rules 7-9, so stray marked lines are never swallowed as a phantom
///    header. The timestamp GNU diff, hg and svn append after a tab is
///    dropped first (the LAST tab, and only when timestamp-shaped, so a
///    bare tab in a name survives), then names are dequoted
///    (`core.quotepath` wraps a non-ASCII path in `"…"`), then the
///    prefixes the section's `diff --git` line settled are stripped — or,
///    for a pair on its own, `a/`/`b/` when the two names differ and
///    nothing when they repeat (`--no-prefix`) or the pair follows a GNU
///    echo (roots, not prefixes). This is what ends the prose prologue —
///    the prologue is positional (everything before the first file
///    header), never keyed on line values. (Bound: mbox prose quoting an unindented, well-formed
///    header-plus-hunk block still fabricates a phantom entry — noise, not
///    loss, since any budget disagreement in it falls back raw.)
/// 5. `@@` after a file header opens a hunk; a malformed `@@` line there is
///    `None`. Before any file section (a hunk quoted in commit prose) it
///    stays prose.
/// 6. File-level facts producers emit outside hunks become note-only
///    entries: `Only in <dir>: <file>` and standalone `Binary files X and Y
///    differ` (GNU `diff -r`), `* Unmerged path <file>` (`git diff --ours`
///    et al. during a merge; folded into the section git emits for the
///    same path right after it), `Submodule <path> <a>..<b>` headers and
///    `Submodule <path> contains … content` dirty lines. GNU names its
///    files under the roots diff was given, never under prefixes, so
///    `Only in` and `Binary files X and Y` keep those roots whole (`Only in
///    g1/: x` is `g1/x`), matching the sibling header pairs; the X/Y
///    boundary of `Binary files X and Y` is the ` and ` where the sides
///    share the longest tail at a `/` boundary, so a filename containing
///    ` and ` is not cut. A bare (diffutils 3.10) `Only in` whose text
///    holds more than one `: ` splits at the root the stream already
///    named on its echo, header pairs or `Binary files` lines, else at
///    the first. The GNU arms read the English spelling only — GNU diff
///    translates its lines (git translates none of its own) — and a
///    translated one is caught by rule 9, never dropped; any GNU arm
///    firing marks the stream as GNU's for rule 9. Names are decoded per
///    producer: git C-quotes (`"caf\303\251.txt"`, `dequote`) on every
///    line it names a file, GNU diff C-quotes its echo and header lines
///    but shell-quotes `Only in` / `Binary files` names from diffutils
///    3.11 on (`'only '$'\303\251''.txt'`, `"Kyle's notes.txt"`,
///    `unquote_shell`; a quoted directory may itself hold `: `, so the
///    `Only in` split follows the quoting) and prints them bare before,
///    and `* Unmerged path` / `Submodule` lines are never quoted. A
///    decoded name that would split a `[file]` line (a newline) or is not
///    UTF-8 keeps the producer's own spelling, which is lossless. hg's
///    `Binary file <path> has changed` is a binary fact, GNU's `Files X
///    and Y are identical` (`-s`) / `differ` (`-q`) are facts by the same
///    `X and Y` shape, and `Common subdirectories: X and Y` (`diff` on
///    directories without `-r`) is informational and dropped. A submodule
///    both dirty and moved gets two lines and one entry. These arms are
///    suppressed inside an mbox message region (from a `From <sha>`
///    separator to that patch's first file header), where column-0 prose
///    is indistinguishable from them by value — except git's own
///    `Submodule <path> <hex>..<hex>[ (<how>)]:` in that strict shape,
///    which `format-patch --submodule=log` puts where a file section
///    would go.
/// 7. In a stream that carried an mbox `From <sha>` separator, a line of
///    exactly `--`/`-- ` outside a hunk is the format-patch signature
///    separator: prose. This is the single value-keyed exclusion, kept
///    because every patch `git format-patch` emits ends with one; its body
///    (`2.54.0`) is unmarked and needs no region. Streams that never had an
///    mbox separator (plain `git diff`, `diff -u`) get no such tolerance —
///    a bare `--` there falls to rule 8. (Bound: in a malformed mbox stream
///    a stale-budget leftover of exactly `--` is swallowed as a signature;
///    every other leftover value still falls through.)
/// 8. Any other `+`/`-` marked line outside a hunk is evidence of a stale
///    or under-declared budget → `None`, with one exemption: inside an mbox
///    message region (rule 2 separator to that patch's first file header),
///    up to format-patch's diffstat, column-0 marked lines are commit
///    prose — bullet lists, quoted hunks, the `---` separator, version
///    notes between it and the diffstat. From the first diffstat line
///    ([`MBOX_DIFFSTAT_RE`]) on, nothing but diffstat precedes the file
///    header, so a marked line there is a hand-edited or reflowed patch's
///    lost content and falls back raw. That covers a stat-ful patch
///    whichever of its file sections lost its headers, first or last.
///    The stream-start prologue earns no exemption at all: a marked line
///    before the first file header of a never-mbox stream is a
///    head-truncated stream's lost content — with one positional
///    exception: a bare `---` directly followed by a diffstat line is the
///    separator `git log --stat -p` / `git show --stat -p` put between
///    the message and the stat, and a `-<TAB>-<TAB><path>` row is `git
///    log --numstat -p`'s binary-file count; lost content never looks
///    like either.
///    A `--no-stat` patch has no diffstat to end the tolerance, so there
///    the exemption is provisional: it is settled when the region closes
///    (the next rule 2 separator, or end of stream), and only for a region
///    that never reached a file header. Such a region is either bodyless
///    by construction — a cover letter, an `--always` empty commit — or a
///    patch whose whole body lost its headers, and only the second kind is
///    loss. A region that quotes whole hunks in its message is never
///    judged, because it does reach its own file header. The two are told
///    apart by shape, per [`MARKED_PROSE_RE`]: if any line the region
///    exempted is not a prose shape → `None`. (Bounds: a message whose own
///    prose imitates a diffstat line (` name | 3`) ends the exemption
///    early; format-patch's own `---` + diffstat is still read as the
///    stat separator, so only a marked line between the two sends the
///    stream raw — noise, not loss; and an
///    orphaned body whose every marked line happens to be prose-shaped is
///    still dropped, as is a `--no-stat` region that lost one section's
///    headers but kept another's; and a bodyless region whose prose wraps
///    a *short* option to column 0 falls back raw — noise, not loss.)
/// 9. Everything else is prose and is dropped — except in a GNU `diff -r`
///    stream, recognised by the per-file `diff <opts> X Y` echo GNU diff
///    prints whenever it compares directories (rule 3b). Such a stream
///    carries no prose, so a column-0 line no arm read there (a translated
///    `Only in`/`Binary files`, a `File X is a fifo …` note) is a fact the
///    parser cannot read: `None`, since dropping it would lose a whole
///    file silently. A line dropped before the stream was known to be
///    GNU's counts the same way once it is, wherever in the stream it was
///    dropped — GNU sorts its output, so an `Only in` or a text section
///    may well parse before the fact it could not read, and the `diff`
///    echo or a fact arm may settle the question only afterwards. hg's
///    echo does not count a prologue drop against the stream (`hg log -p
///    --template` prose is not a fact). (Bounds, noise not loss: a
///    column-0 line of prose shaped `diff -u old new` outside a message
///    region reads as a GNU echo and sends a git stream raw; a templated
///    multi-changeset `hg log -p` goes raw at its second changeset
///    header; a bare two-file `diff -u a/f b/f` with no echo cannot be
///    told from git's `a/`/`b/` prefixes and is stripped; a slash-less
///    or multi-component `--src-prefix`/`--dst-prefix` is cut at the
///    first `/`, so such names keep or lose a component inconsistently.)
///
/// Every rule reads the line through [`structural`] (ANSI- and CR-stripped);
/// content is pushed from the raw line, so a `--color` stream parses while
/// escapes that are part of the user's content survive verbatim.
fn condense_unified_diff_strict(diff: &str) -> Option<String> {
    let mut lines: Vec<&str> = diff.split('\n').collect();
    if lines.last() == Some(&"") {
        lines.pop();
    }

    let mut entries: Vec<FileEntry> = Vec::new();
    let mut current: Option<FileEntry> = None;
    let mut hunk: Option<HunkBudget> = None;
    // Index of the line right after the last hunk's budget closed: the only
    // place a `\ No newline` marker can legitimately sit (rules 1 and 1b):
    // one-past the index of the last line pushed as content.
    let mut emitted_at: Option<usize> = None;
    // Signature tolerance (rule 7) is earned by an mbox separator.
    let mut seen_mbox_from = false;
    // True from a `From <sha>` separator to that patch's first file header:
    // the only region where column-0 prose can imitate the rule-6 facts.
    let mut in_mbox_message = false;
    // True once the mbox message region reached format-patch's diffstat
    // (` <path> | N +-` lines): nothing but more diffstat sits between it
    // and the file header, so a column-0 marked line there is lost content,
    // not prose. Keyed on the diffstat rather than the `---` separator
    // because version notes (`Changes in v2:` + bullets) conventionally go
    // between the two, and on the per-file count-and-signs shape rather
    // than the `N files changed` summary because the latter is localized.
    // Load-bearing beyond the diffstat: it is the parser's only mid-region
    // check, so it alone catches a stat-ful patch whose FIRST section lost
    // its headers while a later one survived — that region does reach a
    // file header, so no region-close check can ever see it.
    let mut past_diffstat = false;
    // Rule 9's two facts about a GNU `diff -r` stream: whether it has been
    // recognised as one (its per-file `diff <opts> X Y` echo, or a GNU fact
    // line, was seen — a stream that carries no prose, so an unread
    // column-0 line there is a fact in a language the parser does not
    // speak), and whether a column-0 line was already dropped as prose
    // before that was settled. GNU sorts its output, so either may come
    // first.
    let mut gnu_diff_r = false;
    let mut dropped_line = false;
    // Which producer's echo was seen: GNU diff names files under the roots
    // it was given (nothing to strip); hg prefixes `a/`/`b/` like git.
    let mut gnu_roots = false;
    let mut hg_stream = false;
    // GNU diff specifically (its echo or one of its fact lines): the only
    // producer whose prose-free stream turns an earlier dropped line into
    // a lost fact. hg's prologue (`hg log -p --template`) is prose.
    let mut gnu_stream = false;
    // The two roots GNU diff was given, as its C-quoted echo / header pairs
    // and `Binary files X and Y` lines reveal them (`d: x` and `e: y`), so
    // a bare diffutils 3.10 `Only in e: y: only.txt` splits at the root
    // the stream already named rather than at the first `: `.
    let mut gnu_root_set: Vec<String> = Vec::new();
    // An `hg export` message region (rule 2): prose-exempt like an mbox
    // message, but rule 6's fact arms stay live in it, and hg's own
    // `diff -r <a> -r <b> <file>` echo — not a header pair — closes it.
    let mut in_hg_message = false;
    // Rule 8's two facts about the open message region, which settle the
    // exemption for a region the diffstat never bounded: whether it reached
    // a file header (then its exempted lines are prose by position and are
    // never judged), and whether any line it exempted broke prose shape.
    let mut region_reached_body = false;
    let mut region_exempt_nonprose = false;

    // `None` when the section closing is one the stream cannot account for:
    // a `diff --git` header with nothing behind it (a stream cut off there)
    // or an svn `Index:` whose description the parser did not read.
    fn flush(entries: &mut Vec<FileEntry>, current: &mut Option<FileEntry>) -> Option<()> {
        if let Some(mut e) = current.take() {
            if e.is_empty() {
                if e.from_git || (e.from_index && e.unread) {
                    return None;
                }
                if e.from_index {
                    // A copy/move target svn has nothing to show for.
                    e.notes.push("no content".to_string());
                } else {
                    return Some(());
                }
            }
            // During a merge `git diff` emits `* Unmerged path X` AND a
            // full section for X; fold the fact into the section so a
            // consumer counting `[file]` lines sees X once. git prints that
            // fact line unquoted, so a name holding a newline arrives cut
            // at the newline while the section keeps the quoted spelling:
            // fold when the fact is the head of the decoded section name.
            let same_file = |p: &FileEntry| {
                p.name == e.name
                    || e
                        .name
                        .strip_prefix('"')
                        .and_then(|q| q.strip_suffix('"'))
                        .and_then(|inner| decode_backslashes(inner, false))
                        .is_some_and(|d| d.starts_with(&format!("{}\n", p.name)))
            };
            if entries
                .last()
                .is_some_and(|p| p.notes == ["unmerged"] && same_file(p))
            {
                entries.pop();
                e.notes.insert(0, "unmerged".to_string());
            }
            entries.push(e);
        }
        Some(())
    }

    let mut i = 0;
    while i < lines.len() {
        let raw = lines[i];
        // Structural decisions read the ANSI-stripped, CR-stripped view;
        // content lines are pushed raw so the user's bytes survive verbatim.
        let view = structural(raw);
        let line: &str = &view;
        i += 1;

        // Rule 1: the open hunk's budget owns the line.
        if let Some(h) = hunk.as_mut() {
            if line.starts_with('\\') {
                // "\ No newline at end of file": consumes no budget. It
                // describes the line right above it, so it is kept only
                // when that line was itself emitted — then a trailing-
                // newline-only change keeps its witness. After a context
                // line (which is never rendered) it would land under the
                // last `-`/`+` line and describe the wrong one: dropped.
                if emitted_at == Some(i - 1) {
                    if let Some(e) = current.as_mut() {
                        e.changes.push(raw.to_string());
                    }
                    emitted_at = Some(i);
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
                emitted_at = Some(i);
                h.marked += 1;
            } else if prefix.contains(&b'+') {
                entry.added += 1;
                entry.changes.push(raw.to_string());
                emitted_at = Some(i);
                h.marked += 1;
            }
            if h.exhausted() {
                // A hunk with no marked line at all is `--word-diff` /
                // `--color-words` output whose inline markers sit behind
                // an indent: its change was just booked as context.
                if h.marked == 0 {
                    return None;
                }
                hunk = None;
            }
            continue;
        }

        // Rule 1b: the new-side no-newline marker lands on the line right
        // after its hunk's budget closed; it belongs to that hunk's section
        // and is kept when the closing line was emitted (rule 1's placement
        // test). Anywhere else a `\` line is prose (rule 9): the marker's
        // text is localized by GNU diff, so position, not value, decides.
        if line.starts_with('\\') {
            if emitted_at == Some(i - 1) {
                if let Some(e) = current.as_mut() {
                    e.changes.push(raw.to_string());
                }
                emitted_at = Some(i);
            }
            continue;
        }

        // Rule 2: mbox patch separator → back to the prose prologue. An
        // `hg export` changeset header, or `hg log -p`'s `changeset:` line,
        // opens the same kind of message region: hg's headers and message
        // precede its `diff -r` echo.
        if is_mbox_from(line) || line == "# HG changeset patch" || line.starts_with("changeset:")
        {
            // Rule 8's shape check: the region this separator closes is judged.
            if region_exempt_nonprose && !region_reached_body {
                return None;
            }
            flush(&mut entries, &mut current)?;
            seen_mbox_from = true;
            in_mbox_message = true;
            in_hg_message = !is_mbox_from(line);
            past_diffstat = false;
            region_reached_body = false;
            region_exempt_nonprose = false;
            continue;
        }

        // Rule 3: git file section with extended headers.
        if let Some(rest) = line
            .strip_prefix("diff --git ")
            .or_else(|| line.strip_prefix("diff --cc "))
            .or_else(|| line.strip_prefix("diff --combined "))
        {
            // An svn `Index:` line names the same file this header opens.
            if current.as_ref().is_some_and(|e| e.from_index && e.is_empty()) {
                current = None;
            }
            flush(&mut entries, &mut current)?;
            in_mbox_message = false;
            in_hg_message = false;
            region_reached_body = true;
            // `diff --cc`/`--combined` carry one unprefixed path; the pair
            // (`--- a/P` / `+++ b/P`) decides the prefixes for that one.
            // `diff --git X Y` names one path under two prefixes — `a/`/`b/`
            // by default, `i/`/`w/` etc. under `diff.mnemonicPrefix` or
            // `--src-prefix`/`--dst-prefix`, none under `--no-prefix` (X ==
            // Y) — so the split is the space where the sides share a tail
            // at a `/` boundary (a path containing ` b/` is not cut there),
            // and what precedes the tail on each side is that side's
            // prefix. Only a rename has no such split, and `rename to`
            // names it exactly. The name here is a fallback the `+++`
            // header refines.
            let (name, prefixes) = if !line.starts_with("diff --git ") {
                (dequote(rest).into_owned(), None)
            } else {
                let split = rest
                    .match_indices(' ')
                    .map(|(p, _)| (dequote(&rest[..p]), dequote(&rest[p + 1..])))
                    // A side that stayed quoted is kept as git spells it.
                    .filter(|(x, y)| !x.starts_with('"') && !y.starts_with('"'))
                    .filter_map(|(x, y)| shared_tail(&x, &y).map(|n| (x, y, n)))
                    .max_by_key(|&(_, _, n)| n);
                match split {
                    Some((x, y, n)) => {
                        // A git prefix is one path component (`a/`, `w/`,
                        // `--src-prefix=old/`); what the sides do not share
                        // beyond it is path (`git diff --no-index d1/f d2/f`
                        // → `a/d1/f.txt` `b/d2/f.txt`), not prefix.
                        let root = |s: &str| {
                            let head = &s[..s.len() - n];
                            let head = head.strip_suffix('/').unwrap_or(head);
                            head.split('/').next().unwrap_or("").to_string()
                        };
                        let (px, py) = (root(&x), root(&y));
                        (
                            header_name(&x, &y, Some((&px, &py))),
                            Some((px, py)),
                        )
                    }
                    None => {
                        // A rename (`rename to` names it exactly) or two
                        // unrelated paths (`--no-index --no-prefix x y`):
                        // the `b/`-prefixed side, else the last word.
                        let y = rest
                            .rfind(" b/")
                            .or_else(|| rest.rfind(" \"b/"))
                            .or_else(|| rest.rfind(' '))
                            .map_or(std::borrow::Cow::Borrowed(rest), |p| dequote(&rest[p + 1..]));
                        (strip_quoted_prefix(&y, "b"), None)
                    }
                }
            };
            current = Some(FileEntry {
                name,
                prefixes,
                from_git: true,
                ..FileEntry::default()
            });
            continue;
        }
        if let Some(e) = current.as_mut().filter(|e| e.header_only()) {
            if line.starts_with("Binary files ")
                || line == "GIT binary patch"
                || line == "Cannot display: file marked as a binary type."
            {
                e.notes.push("binary".to_string());
                continue;
            }
            // Structural, not prose: must not count as a dropped line.
            if line.starts_with("index ")
                || line.starts_with("similarity index ")
                || line.starts_with("dissimilarity index ")
            {
                continue;
            }
            if let Some(from) = line.strip_prefix("rename from ") {
                e.rename_from = Some(dequote(from).to_string());
                continue;
            }
            if let Some(to) = line.strip_prefix("rename to ") {
                e.name = dequote(to).to_string();
                let from = e.rename_from.take().unwrap_or_default();
                e.notes.push(format!("renamed from {}", from));
                continue;
            }
            if let Some(from) = line.strip_prefix("copy from ") {
                e.rename_from = Some(dequote(from).to_string());
                continue;
            }
            if let Some(to) = line.strip_prefix("copy to ") {
                e.name = dequote(to).to_string();
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

        // Rule 3b: GNU diff's per-file command echo (`diff -ru X Y`, printed
        // whenever it compares directories) marks a `diff -r` stream. Such
        // a stream carries no prose, so a column-0 line rule 9 already
        // dropped before this echo was a fact it could not read. hg's echo
        // (`diff -r <rev> -r <rev> <file>`) is the same mark, and it is
        // what closes an `hg export` message region — every hg file,
        // binary ones included, gets one.
        // Keyed on `diff -`: every echo carries an option (`-u`, `-r`,
        // `--recursive`; hg's `-r`), and a commit body line that starts
        // with `diff ` under `--format=%B` does not.
        if line.starts_with("diff -") && (!in_mbox_message || is_hg_echo(line)) {
            if is_hg_echo(line) {
                hg_stream = true;
                // The region this echo closes reached its body.
                region_reached_body = true;
            } else {
                gnu_roots = true;
                gnu_stream = true;
            }
            gnu_diff_r = true;
            in_mbox_message = false;
            in_hg_message = false;
            continue;
        }

        // Rule 3c: svn's `Index: <path>` opens a file section (its `===`
        // rule and `svn:mime-type = …` lines are prose; the header pair or
        // a `diff --git` line that follows names it again). An `Index:`
        // section that closes with nothing parsed is either a copy/move
        // target svn has nothing to show for — `(no content)` — or, when a
        // column-0 line was dropped inside it, a file svn described in
        // words the parser did not read → `None`.
        if let Some(path) = line.strip_prefix("Index: ").filter(|_| !in_mbox_message) {
            flush(&mut entries, &mut current)?;
            current = Some(FileEntry {
                name: path.to_string(),
                prefixes: Some((String::new(), String::new())),
                from_index: true,
                ..FileEntry::default()
            });
            continue;
        }

        // Rule 4: `--- X` + `+++ Y` header pair.
        if let Some(minus) = line.strip_prefix("--- ") {
            let next = lines.get(i).map(|r| structural(r));
            if let Some(plus) = next.as_deref().and_then(|n| n.strip_prefix("+++ ")) {
                // A pair must be followed by a hunk header — every real
                // producer emits one, git included (`---`/`+++` only ever
                // precede the first `@@`). Without this gate a stray marked
                // pair (a lying budget's leftovers) would be consumed as a
                // phantom header and its two lines lost; gated, it falls
                // through to rule 8 instead.
                let opens_hunk = lines
                    .get(i + 1)
                    .is_some_and(|r| structural(r).starts_with("@@"));
                if opens_hunk {
                    let (minus, plus) = (strip_timestamp(minus), strip_timestamp(plus));
                    // A binary section never has a pair: a pair after one is
                    // another producer's next file, not a rename in place.
                    if current
                        .as_ref()
                        .is_some_and(|e| e.header_only() && e.notes.iter().any(|n| n == "binary"))
                    {
                        flush(&mut entries, &mut current)?;
                    }
                    if gnu_roots && !hg_stream {
                        // The roots GNU diff was given, for `Only in` (rule 6).
                        let (x, y) = (dequote(minus), dequote(plus));
                        if let Some(n) = shared_tail(&x, &y) {
                            for side in [&x, &y] {
                                let root = side[..side.len() - n].trim_end_matches('/');
                                if !root.is_empty() && !gnu_root_set.iter().any(|r| r == root) {
                                    gnu_root_set.push(root.to_string());
                                }
                            }
                        }
                    }
                    match current.as_mut().filter(|e| e.header_only()) {
                        // `rename to`/`copy to` is git's exact path; the pair
                        // under `--no-prefix` would re-derive it wrongly.
                        Some(e) if e.notes.iter().any(|n| {
                            n.starts_with("renamed from ") || n.starts_with("copied from ")
                        }) => {}
                        Some(e) => {
                            let p = e.prefixes.as_ref().map(|(x, y)| (x.as_str(), y.as_str()));
                            e.name = header_name(minus, plus, p);
                        }
                        None => {
                            flush(&mut entries, &mut current)?;
                            // A pair on its own after a GNU echo names files
                            // under the roots diff was given — `a`/`b` there
                            // are directories; hg (and a bare pair) prefix.
                            let p = (gnu_roots && !hg_stream).then_some(("", ""));
                            current = Some(FileEntry {
                                name: header_name(minus, plus, p),
                                prefixes: p.map(|(x, y)| (x.to_string(), y.to_string())),
                                ..FileEntry::default()
                            });
                        }
                    }
                    in_mbox_message = false;
                    in_hg_message = false;
                    region_reached_body = true;
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
                    let h = HunkBudget {
                        old_left,
                        new_left,
                        marked: 0,
                    };
                    // `@@ -0,0 +0,0 @@` closes before it opens.
                    if !h.exhausted() {
                        hunk = Some(h);
                    }
                    continue;
                }
                Some(_) => continue, // quoted hunk in prose, no file section
                None if current.is_some() => return None,
                None => continue,
            }
        }

        // Rule 6: file-level facts outside hunks, suppressed in mbox prose
        // (an hg message region keeps them live: hg emits nothing that
        // needs suppressing, and losing a fact outranks a phantom entry;
        // git's own `Submodule` fact is admitted in strict shape: `git
        // format-patch --submodule=log` puts it where a file section
        // would go, and no prose line ends in `<hex>..<hex>:`).
        if !in_mbox_message || in_hg_message || is_submodule_range(line) {
            if let Some(rest) = line.strip_prefix("Only in ") {
                // GNU diff's separator is the `: ` right after the
                // directory; a filename may carry its own. The directory is
                // echoed as given (`diff -ru g1/ g2/` → `Only in g1/:`), and
                // diffutils ≥ 3.11 shell-quotes both parts — a quoted
                // directory may itself contain `: `, so the split follows
                // the quoting when there is any.
                // Bare (diffutils 3.10) with more than one `: `: the root
                // the stream already named wins over the first `: `.
                let split = match shell_word_len(rest) {
                    Some(n) if rest[n..].starts_with(": ") => Some((&rest[..n], &rest[n + 2..])),
                    _ => rest
                        .match_indices(": ")
                        .map(|(p, _)| (&rest[..p], &rest[p + 2..]))
                        .find(|(d, _)| gnu_root_set.iter().any(|r| r == d.trim_end_matches('/')))
                        .or_else(|| rest.split_once(": ")),
                };
                if let Some((dir, file)) = split {
                    let dir = unquote_shell(dir);
                    let name = format!("{}/{}", dir.trim_end_matches('/'), unquote_shell(file));
                    flush(&mut entries, &mut current)?;
                    entries.push(FileEntry {
                        name,
                        notes: vec!["only in one side".to_string()],
                        ..FileEntry::default()
                    });
                    gnu_diff_r = true;
                    gnu_stream = true;
                    continue;
                }
            }
            // `diff` without `-r` on two directories: informational, no file.
            if line.starts_with("Common subdirectories: ") {
                gnu_diff_r = true;
                gnu_stream = true;
                continue;
            }
            // hg's binary fact, which follows its own `diff -r` echo.
            if let Some(path) = line
                .strip_prefix("Binary file ")
                .and_then(|r| r.strip_suffix(" has changed"))
            {
                flush(&mut entries, &mut current)?;
                entries.push(FileEntry {
                    name: path.to_string(),
                    notes: vec!["binary".to_string()],
                    ..FileEntry::default()
                });
                continue;
            }
            // Standalone GNU `diff -r` forms; the git `Binary files` form
            // attaches to its open `diff --git` section in the
            // extended-header block above. `-s` reports identical files and
            // `-q` differing ones by the same `X and Y` shape.
            if let Some((pair, note)) = [
                ("Binary files ", " differ", "binary"),
                ("Files ", " are identical", "identical"),
                ("Files ", " differ", "differs"),
            ]
            .iter()
            .find_map(|(pre, suf, note)| {
                line.strip_prefix(pre)
                    .and_then(|r| r.strip_suffix(suf))
                    .map(|pair| (pair, *note))
            }) {
                // `diff -r` names X and Y as the same path under two roots
                // (`.` and `../new`, `src/old` and `src/new`), so the ` and `
                // at the X/Y boundary is the one where the sides share the
                // longest tail at a `/` boundary — a filename containing
                // ` and ` is then not split. diffutils ≥ 3.11 shell-quotes
                // these names, so each side is decoded first. The roots are
                // directories GNU diff was given, never prefixes: nothing is
                // stripped, which matches the sibling header pairs.
                let (x, y) = pair
                    .match_indices(" and ")
                    .map(|(p, _)| (unquote_shell(&pair[..p]), unquote_shell(&pair[p + 5..])))
                    .filter_map(|(x, y)| shared_tail(&x, &y).map(|n| (x, y, n)))
                    .max_by_key(|&(_, _, n)| n)
                    .map(|(x, y, _)| (x, y))
                    .or_else(|| {
                        pair.rsplit_once(" and ")
                            .map(|(x, y)| (unquote_shell(x), unquote_shell(y)))
                    })
                    .unwrap_or((
                        std::borrow::Cow::Borrowed(pair),
                        std::borrow::Cow::Borrowed(pair),
                    ));
                if let Some(n) = shared_tail(&x, &y) {
                    for side in [&x, &y] {
                        let root = side[..side.len() - n].trim_end_matches('/');
                        if !root.is_empty() && !gnu_root_set.iter().any(|r| r == root) {
                            gnu_root_set.push(root.to_string());
                        }
                    }
                }
                let name = header_name(&x, &y, Some(("", "")));
                flush(&mut entries, &mut current)?;
                entries.push(FileEntry {
                    name,
                    notes: vec![note.to_string()],
                    ..FileEntry::default()
                });
                gnu_diff_r = true;
                gnu_stream = true;
                continue;
            }
            if let Some(path) = line.strip_prefix("* Unmerged path ") {
                flush(&mut entries, &mut current)?;
                entries.push(FileEntry {
                    name: path.to_string(),
                    notes: vec!["unmerged".to_string()],
                    ..FileEntry::default()
                });
                continue;
            }
            if let Some(rest) = line.strip_prefix("Submodule ") {
                // `Submodule <path> contains [untracked and ]modified content`
                // (a dirty submodule, no sha range) or `Submodule <path>
                // <a>..<b> (<how>):` — the LAST token holding `..` is the
                // range, so a path containing `..` keeps its name. Only the
                // path belongs in the name slot.
                let fact = rest
                    .rsplit_once(" contains ")
                    .filter(|(_, what)| what.ends_with(" content"))
                    .map(|(path, what)| (path.to_string(), what.to_string()))
                    .or_else(|| {
                        let toks: Vec<&str> = rest.split(' ').collect();
                        toks.iter()
                            .rposition(|t| t.contains(".."))
                            .filter(|&p| p > 0)
                            .map(|p| {
                                // The range and its direction qualifier —
                                // `(rewind)`, `(new submodule)` — are the
                                // fact: keep them in the note.
                                let range = toks[p..]
                                    .join(" ")
                                    .trim_end_matches(':')
                                    .replace(['(', ')'], "");
                                (toks[..p].join(" "), range)
                            })
                    });
                if let Some((name, what)) = fact {
                    flush(&mut entries, &mut current)?;
                    // A submodule both dirty and moved gets two lines; one
                    // path, one entry.
                    match entries.last_mut() {
                        Some(prev)
                            if prev.name == name
                                && prev.changes.is_empty()
                                && prev.notes.first().is_some_and(|n| n == "submodule") =>
                        {
                            prev.notes.push(what);
                        }
                        _ => entries.push(FileEntry {
                            name,
                            notes: vec!["submodule".to_string(), what],
                            ..FileEntry::default()
                        }),
                    }
                    continue;
                }
            }
        }

        // Rule 7: format-patch signature separator, only in mbox streams.
        if seen_mbox_from && (line == "--" || line == "-- ") {
            continue;
        }

        // Rule 8: content outside any hunk → stale budget, fall back. Only
        // the mbox message region is exempt, and only up to format-patch's
        // diffstat: past it nothing but diffstat precedes the file header,
        // so a marked line there — like one in the stream-start prologue —
        // is a truncated or hand-edited stream's lost content. Before the
        // diffstat, and in a `--no-stat` region that has none, the exemption
        // is provisional: the region's close settles it by shape.
        if line.starts_with('+') || line.starts_with('-') {
            if !in_mbox_message || past_diffstat {
                // `git log --stat -p` / `git show --stat -p` put a bare
                // `---` between the message and the diffstat; the indented
                // ` <path> | N +-` (or ` N files changed`) line right after
                // it is the tell, and lost content never looks like that.
                let stat_follows = line == "---"
                    && lines.get(i).is_some_and(|r| {
                        let n = structural(r);
                        MBOX_DIFFSTAT_RE.is_match(&n)
                            || (n.starts_with(' ') && n.contains(" file") && n.contains(" changed"))
                    });
                if stat_follows {
                    continue;
                }
                // `git log --numstat -p` puts `-<TAB>-<TAB><path>` rows
                // (binary files) before the diff; lost content never
                // carries that tab-separated count shape.
                if line.starts_with("-\t-\t") {
                    continue;
                }
                return None;
            }
            region_exempt_nonprose |= !MARKED_PROSE_RE.is_match(line);
        }

        // Rule 9: prose. A diffstat line in the mbox message region ends
        // rule 8's tolerance.
        if in_mbox_message && MBOX_DIFFSTAT_RE.is_match(line) {
            past_diffstat = true;
        }
        // In a GNU `diff -r` stream there is no prose: GNU diff translates
        // `Only in` and `Binary files` (git does not) and has facts no arm
        // reads (`File X is a fifo …`, `Symbolic links X and Y differ`), so
        // a column-0 line dropped here is a fact, and dropping it would lose
        // a whole file silently. Until the stream is known to be GNU's,
        // remember that a drop happened — wherever it happened: GNU sorts
        // its output, so an entry may well have parsed before it.
        if !in_mbox_message && !line.is_empty() && !line.starts_with(' ') {
            if gnu_diff_r {
                return None;
            }
            dropped_line = true;
            // Rule 3c: words inside an open, still-empty `Index:` section
            // (its `===` rule is not words).
            if let Some(e) = current.as_mut().filter(|e| e.from_index && e.is_empty()) {
                e.unread |= !line.bytes().all(|b| b == b'=');
            }
        }
    }

    // Budget owed at EOF.
    if hunk.is_some() {
        return None;
    }
    // Rule 9: the stream turned out to be GNU diff's after a column-0 line
    // had already been dropped — a fact the parser could not read.
    if gnu_stream && dropped_line {
        return None;
    }
    // Rule 8's shape check: end of stream closes the last message region.
    if region_exempt_nonprose && !region_reached_body {
        return None;
    }
    flush(&mut entries, &mut current)?;

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
        // Column 0: anchored greps (`^[+-]`) must match these. (Holds for
        // uncoloured input; a `--color` stream's body lines are its own
        // bytes and open with git's colour escape — fidelity outranks it.)
        out.extend(e.changes);
    }
    Some(out.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_test_diff(file1: &str, file2: &str, content1: &str, content2: &str) -> (String, i32) {
        let lines1: Vec<&str> = content1.lines().collect();
        let lines2: Vec<&str> = content2.lines().collect();
        let diff = compute_diff(&lines1, &lines2);
        render_diff(
            Path::new(file1),
            Path::new(file2),
            &diff,
            content1 == content2,
        )
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

    // --- render_diff (issue #2364 regression) ---

    #[test]
    fn test_render_modified_only_yaml_not_identical() {
        // "a: 1" vs "a: 2" is classified as modified (similarity > 0.5);
        // the identical check must not ignore modified-only diffs.
        let (out, code) = render_test_diff("one.yaml", "two.yaml", "a: 1\n", "a: 2\n");
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
    fn test_render_modified_only_json_not_identical() {
        let (out, code) = render_test_diff("j1.json", "j2.json", "{\"a\": 1}\n", "{\"a\": 2}\n");
        assert!(
            !out.contains("identical"),
            "modified-only diff reported as identical:\n{}",
            out
        );
        assert_eq!(code, 1);
    }

    #[test]
    fn test_render_identical_files_exit_zero() {
        let (out, code) =
            render_test_diff("a.yaml", "b.yaml", "a: 1\nb: 2\n", "a: 1\nb: 2\n");
        assert!(out.contains("[ok] Files are identical"));
        assert_eq!(code, 0);
    }

    #[test]
    fn test_render_added_removed_exit_one() {
        let (out, code) = render_test_diff("t1.txt", "t2.txt", "x\n", "y\n");
        assert!(out.contains("+1 added, -1 removed"));
        assert_eq!(code, 1);
    }

    // --- byte-different but line-equal files must not be "identical" (issue #3469) ---

    #[test]
    fn test_render_crlf_vs_lf_not_identical() {
        let (out, code) = render_test_diff(
            "a.txt",
            "b.txt",
            "alpha\nbeta\n",
            "alpha\r\nbeta\r\n",
        );
        assert!(
            !out.contains("identical"),
            "CRLF-vs-LF difference reported as identical:\n{}",
            out
        );
        assert!(
            out.contains("whitespace or line endings"),
            "expected the whitespace/line-ending message, got:\n{}",
            out
        );
        assert_eq!(code, 1, "byte-different files must exit 1 (diff convention)");
    }

    #[test]
    fn test_render_trailing_newline_not_identical() {
        let (out, code) = render_test_diff("a.txt", "b.txt", "abc", "abc\n");
        assert!(
            !out.contains("identical"),
            "trailing-newline difference reported as identical:\n{}",
            out
        );
        assert_eq!(code, 1);
    }

    #[test]
    fn test_render_byte_identical_exit_zero_with_crlf() {
        let (out, code) = render_test_diff("a.txt", "b.txt", "a\r\nb\r\n", "a\r\nb\r\n");
        assert!(out.contains("[ok] Files are identical"));
        assert_eq!(code, 0);
    }

    #[test]
    fn test_never_worse_fallback_is_a_classic_diff() {
        let diff = compute_diff(&["alpha beta"], &["alpha zzzz"]);
        let fallback = format_classic_diff(&diff);
        let (rendered, code) =
            render_diff(Path::new("before"), Path::new("after"), &diff, false);
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
        let old_content = old.join("\n");
        let new_content = new.join("\n");
        let both_files = format!("{}\n---\n{}", old_content, new_content);
        let (rendered, _) = render_diff(
            Path::new("a"),
            Path::new("b"),
            &diff,
            old_content == new_content,
        );
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
        // Indented markers make anchored greps (`^[+-]`) match nothing, so a
        // "was anything removed?" audit answers no while the content is there.
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

    // --- overflow indicator ---

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
    fn test_condense_unified_diff_large_no_false_overflow_indicator() {
        // All 200 changes are shown in full (never truncate diff content).
        // No misleading "... +N more" should appear.
        let diff = make_large_unified_diff(100, 100);
        let result = condense_unified_diff(&diff);
        assert!(
            !result.contains("more"),
            "No overflow indicator expected when all lines are shown, got:\n{}",
            result
        );
        assert!(
            result.contains("+new_value_99"),
            "Last added line must be present (no truncation)"
        );
        assert!(
            result.contains("-old_value_99"),
            "Last removed line must be present (no truncation)"
        );
    }

    #[test]
    fn test_condense_unified_diff_no_false_overflow() {
        // Counter-case to the 200-change test above: no indicator at small sizes either.
        let diff = make_large_unified_diff(4, 4);
        let result = condense_unified_diff(&diff);
        assert!(
            !result.contains("more"),
            "No overflow message expected for 8 changes, got:\n{}",
            result
        );
    }

    // --- region parser: real-producer fixture corpus ---
    //
    // Every fixture is captured from a real binary — git 2.54 (Windows) and
    // 2.47 (Debian), GNU diffutils 3.12 (Git for Windows) and 3.10 (Debian),
    // Mercurial 7.0, Subversion 1.14, PowerShell 5.1 — never synthesized:
    // synthetic fixtures with impossible hunk counts masked bugs for five
    // review rounds (claudedocs/diff-classifier-review-2026-08-29.md).
    // `git_diff_bom` is pinned through `condense_stdin` (the BOM strip lives
    // there) and `git_format_patch_quoted_hunk` on its own; the raw-asserting
    // captures are pinned in `raw_corpus_falls_back_byte_exact`.

    const CORPUS: &[(&str, &str)] = &[
        (
            "git_diff_multifile",
            include_str!("../../../tests/fixtures/diff/git_diff_multifile_raw.txt"),
        ),
        (
            "git_diff_submodule_dirty",
            include_str!("../../../tests/fixtures/diff/git_diff_submodule_dirty_raw.txt"),
        ),
        (
            "git_diff_no_newline",
            include_str!("../../../tests/fixtures/diff/git_diff_no_newline_raw.txt"),
        ),
        (
            "hg_export",
            include_str!("../../../tests/fixtures/diff/hg_export_raw.txt"),
        ),
        (
            "hg_export_binary",
            include_str!("../../../tests/fixtures/diff/hg_export_binary_raw.txt"),
        ),
        (
            "git_diff_quotepath",
            include_str!("../../../tests/fixtures/diff/git_diff_quotepath_raw.txt"),
        ),
        (
            "diff_ru_quoted_310",
            include_str!("../../../tests/fixtures/diff/diff_ru_quoted_310_raw.txt"),
        ),
        (
            "diff_ru_quoted_312_c",
            include_str!("../../../tests/fixtures/diff/diff_ru_quoted_312_c_raw.txt"),
        ),
        (
            "diff_ru_quoted_312_utf8",
            include_str!("../../../tests/fixtures/diff/diff_ru_quoted_312_utf8_raw.txt"),
        ),
        (
            "svn_diff",
            include_str!("../../../tests/fixtures/diff/svn_diff_raw.txt"),
        ),
        (
            "svn_diff_git",
            include_str!("../../../tests/fixtures/diff/svn_diff_git_raw.txt"),
        ),
        (
            "git_diff_typechange",
            include_str!("../../../tests/fixtures/diff/git_diff_typechange_raw.txt"),
        ),
        (
            "git_diff_custom_prefix",
            include_str!("../../../tests/fixtures/diff/git_diff_custom_prefix_raw.txt"),
        ),
        (
            "diff_rus",
            include_str!("../../../tests/fixtures/diff/diff_rus_raw.txt"),
        ),
        (
            "git_format_patch_submodule",
            include_str!("../../../tests/fixtures/diff/git_format_patch_submodule_raw.txt"),
        ),
        (
            "diff_ru_312_apostrophe",
            include_str!("../../../tests/fixtures/diff/diff_ru_312_apostrophe_raw.txt"),
        ),
        (
            "diff_ru_312_apostrophe_dirs",
            include_str!("../../../tests/fixtures/diff/diff_ru_312_apostrophe_dirs_raw.txt"),
        ),
        (
            "diff_ru_312_colon_dirs",
            include_str!("../../../tests/fixtures/diff/diff_ru_312_colon_dirs_raw.txt"),
        ),
        (
            "hg_diff",
            include_str!("../../../tests/fixtures/diff/hg_diff_raw.txt"),
        ),
        (
            "diff_ru_dot",
            include_str!("../../../tests/fixtures/diff/diff_ru_dot_raw.txt"),
        ),
        (
            "git_diff_cc_binary",
            include_str!("../../../tests/fixtures/diff/git_diff_cc_binary_raw.txt"),
        ),
        (
            "git_diff_mnemonic",
            include_str!("../../../tests/fixtures/diff/git_diff_mnemonic_raw.txt"),
        ),
        (
            "git_diff_ours_mnemonic",
            include_str!("../../../tests/fixtures/diff/git_diff_ours_mnemonic_raw.txt"),
        ),
        (
            "diff_ru_ab",
            include_str!("../../../tests/fixtures/diff/diff_ru_ab_raw.txt"),
        ),
        (
            "git_diff_latin1",
            include_str!("../../../tests/fixtures/diff/git_diff_latin1_raw.txt"),
        ),
        (
            "git_log_p_stat",
            include_str!("../../../tests/fixtures/diff/git_log_p_stat_raw.txt"),
        ),
        (
            "hg_log_p",
            include_str!("../../../tests/fixtures/diff/hg_log_p_raw.txt"),
        ),
        (
            "git_show_format_B",
            include_str!("../../../tests/fixtures/diff/git_show_format_B_raw.txt"),
        ),
        (
            "git_diff_submodule_dirty_moved",
            include_str!("../../../tests/fixtures/diff/git_diff_submodule_dirty_moved_raw.txt"),
        ),
        (
            "git_show_cc_octopus",
            include_str!("../../../tests/fixtures/diff/git_show_cc_octopus_raw.txt"),
        ),
        (
            "git_diff_tree_combined",
            include_str!("../../../tests/fixtures/diff/git_diff_tree_combined_raw.txt"),
        ),
        (
            "svn_diff_move",
            include_str!("../../../tests/fixtures/diff/svn_diff_move_raw.txt"),
        ),
        (
            "diff_u_common_subdirs",
            include_str!("../../../tests/fixtures/diff/diff_u_common_subdirs_raw.txt"),
        ),
        // `git_format_patch_quoted_hunk` is pinned on its own: the replay
        // in property (a) opens a budget at the hunk the message quotes.
        (
            "git_format_patch_interdiff",
            include_str!("../../../tests/fixtures/diff/git_format_patch_interdiff_raw.txt"),
        ),
        (
            "diff_u_label",
            include_str!("../../../tests/fixtures/diff/diff_u_label_raw.txt"),
        ),
        (
            "git_diff_emoji_rename",
            include_str!("../../../tests/fixtures/diff/git_diff_emoji_rename_raw.txt"),
        ),
        (
            "git_diff_no_index_dirs",
            include_str!("../../../tests/fixtures/diff/git_diff_no_index_dirs_raw.txt"),
        ),
        (
            "diff_ru_310_colon_dirs",
            include_str!("../../../tests/fixtures/diff/diff_ru_310_colon_dirs_raw.txt"),
        ),
        (
            "hg_diff_nodates_tab",
            include_str!("../../../tests/fixtures/diff/hg_diff_nodates_tab_raw.txt"),
        ),
        (
            "git_diff_ours_newline",
            include_str!("../../../tests/fixtures/diff/git_diff_ours_newline_raw.txt"),
        ),
        (
            "git_log_p_quoted_renames",
            include_str!("../../../tests/fixtures/diff/git_log_p_quoted_renames_raw.txt"),
        ),
        (
            "git_format_patch_no_stat_cover",
            include_str!("../../../tests/fixtures/diff/git_format_patch_no_stat_cover_raw.txt"),
        ),
        (
            "git_diff_no_prefix_rename",
            include_str!("../../../tests/fixtures/diff/git_diff_no_prefix_rename_raw.txt"),
        ),
        (
            "git_diff_submodule_diff",
            include_str!("../../../tests/fixtures/diff/git_diff_submodule_diff_raw.txt"),
        ),
        (
            "git_log_numstat_p",
            include_str!("../../../tests/fixtures/diff/git_log_numstat_p_raw.txt"),
        ),
        (
            "hg_log_p_template",
            include_str!("../../../tests/fixtures/diff/hg_log_p_template_raw.txt"),
        ),
        (
            "diff_u_fr_no_newline",
            include_str!("../../../tests/fixtures/diff/diff_u_fr_no_newline_raw.txt"),
        ),
        // `git_diff_color` is pinned on its own: property (a)'s replay
        // reads raw lines, and a coloured body line opens with ESC.
        (
            "git_diff_u0",
            include_str!("../../../tests/fixtures/diff/git_diff_u0_raw.txt"),
        ),
        (
            "git_diff_no_prefix",
            include_str!("../../../tests/fixtures/diff/git_diff_no_prefix_raw.txt"),
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
            "hg_export_binary_only",
            include_str!("../../../tests/fixtures/diff/hg_export_binary_only_raw.txt"),
        ),
        (
            "diff_rq",
            include_str!("../../../tests/fixtures/diff/diff_rq_raw.txt"),
        ),
        (
            "git_diff_no_index_no_prefix_bin",
            include_str!("../../../tests/fixtures/diff/git_diff_no_index_no_prefix_bin_raw.txt"),
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
    /// Body lines are extracted here by replaying only the hunk budgets — a
    /// deliberately dumber walk than the parser under test: it knows nothing
    /// about prose, headers, or file sections beyond "a budget opened at
    /// `@@`". The per-parent presence rule is the same one the parser uses
    /// (there is no second formula for git's combined-diff columns), so the
    /// oracle that keeps this walk honest is the header itself: git's counts
    /// must be consumed exactly — over-consumption underflows and panics,
    /// and under-consumption (a budget still owed when the next `@@`, file
    /// header or EOF arrives) is asserted below.
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
                        prefix.len() == parents
                            && prefix.iter().all(|c| matches!(c, ' ' | '-' | '+')),
                        "{name}: budget still owed ({old:?} / {new}) at a non-body line {line:?} — the presence rule under-consumed"
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
                budget.is_none(),
                "{name}: budget still owed at EOF — the presence rule under-consumed"
            );
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
        // `diff -ru a b`: `a`/`b` are the roots diff was given, so every
        // name keeps them — the binary fact, the header pairs, `Only in`.
        assert!(
            out.contains("[file] b/img.bin (binary)"),
            "standalone binary fact vanished:\n{out}"
        );
        assert!(out.contains("[file] b/f1.txt (+2 -1)"), "got:\n{out}");
    }

    #[test]
    fn unmerged_paths_are_reported() {
        // `git diff --ours` during a merge conflict opens with
        // `* Unmerged path <file>` BEFORE any file header — the fact arm
        // must fire in a plain stream's prologue.
        let fixture = include_str!("../../../tests/fixtures/diff/git_diff_unmerged_raw.txt");
        let out = condense_unified_diff(fixture);
        // The fact line is followed by a full section for the same path;
        // the two fold into one entry so `[file]` counts stay honest.
        assert!(
            out.contains("[file] cfile.txt (unmerged) (+4 -0)"),
            "unmerged fact not folded into its section:\n{out}"
        );
        assert_eq!(
            out.matches("[file] cfile.txt").count(),
            1,
            "unmerged path listed twice:\n{out}"
        );
    }

    #[test]
    fn submodule_log_headers_are_reported() {
        // `git diff --submodule=log` emits a `Submodule <name> <a>..<b>`
        // block whose indented body is prose; the header itself is a fact.
        let fixture = include_str!("../../../tests/fixtures/diff/git_diff_submodule_raw.txt");
        let out = condense_unified_diff(fixture);
        // Only the path goes in the name slot — the shas and `(rewind)`
        // would leave a consumer unable to recover it; the range is the
        // fact, so it rides in the note.
        assert!(
            out.contains("[file] sub (submodule, e139196..b0ac9b1 rewind)"),
            "submodule fact vanished or mis-sliced:\n{out}"
        );
    }

    #[test]
    fn translated_gnu_fact_lines_force_raw_instead_of_vanishing() {
        // GNU diff translates `Only in` and `Binary files` (git's own
        // `Binary files` line is never translated). Real capture:
        // `LC_ALL=fr_FR.UTF-8 diff -ru g1 g2`, diffutils 3.10. Dropping the
        // translated lines as prose lost three of five files while the
        // stream still returned `Some`.
        let french = include_str!("../../../tests/fixtures/diff/diff_ru_fr_raw.txt");
        assert!(
            condense_unified_diff_strict(french).is_none(),
            "translated fact lines were dropped silently"
        );
        let body = "diff -ru g1/f.txt g2/f.txt\n--- g1/f.txt\t2026-09-02 20:00:00.000000000 +0200\n+++ g2/f.txt\t2026-09-02 20:00:00.000000000 +0200\n@@ -1,2 +1,2 @@\n a\n-b\n+c\n";
        // A translated fact BEFORE the first `diff` echo counts once the
        // echo settles that this is a `diff -r` stream.
        let leading = format!("Seulement dans g1: onlyleft.txt\n{body}");
        assert!(condense_unified_diff_strict(&leading).is_none());
        // The English spelling of the same stream still parses in full.
        let english = format!(
            "Binary files g1/bin.dat and g2/bin.dat differ\n{body}Only in g1: onlyleft.txt\nOnly in g2: onlyright.txt\n"
        );
        let out = condense_unified_diff_strict(&english).expect("English diff -ru must parse");
        for label in [
            "[file] g2/bin.dat (binary)",
            "[file] g2/f.txt (+1 -1)",
            "[file] g1/onlyleft.txt (only in one side)",
            "[file] g2/onlyright.txt (only in one side)",
        ] {
            assert!(out.contains(label), "missing {label}:\n{out}");
        }
        // The echo is a context marker, not prose: a `git log -p` stream
        // whose prologue was dropped is unaffected because it has none.
        let log = "commit 0123456789abcdef0123456789abcdef01234567\nAuthor: A <a@b>\n\n    subject\n\ndiff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-x\n+y\n";
        assert!(condense_unified_diff_strict(log).is_some());
    }

    #[test]
    fn dirty_submodule_fact_survives_beside_a_condensed_sibling() {
        // `git diff` (also `--submodule=log`) reports a dirty submodule as a
        // sha-less `Submodule <path> contains modified content` line; with
        // a condensed sibling in the stream it used to be dropped as prose.
        let fixture =
            include_str!("../../../tests/fixtures/diff/git_diff_submodule_dirty_raw.txt");
        let out = condense_unified_diff_strict(fixture).expect("real git diff must parse");
        assert!(out.contains("[file] other.txt (+1 -0)"), "got:\n{out}");
        assert!(
            out.contains("[file] sub (submodule, modified content)"),
            "dirty-submodule fact vanished:\n{out}"
        );
        let out = condense_unified_diff_strict(
            &fixture.replace("modified content", "untracked content"),
        )
        .expect("must parse");
        assert!(out.contains("(submodule, untracked content)"), "got:\n{out}");
        // A submodule path containing `..` keeps its name: the range is
        // the LAST token holding `..`.
        let out = condense_unified_diff_strict(
            &fixture.replace(
                "Submodule sub contains modified content",
                "Submodule a..b e139196..b0ac9b1 (rewind):",
            ),
        )
        .expect("must parse");
        assert!(
            out.contains("[file] a..b (submodule, e139196..b0ac9b1 rewind)"),
            "got:\n{out}"
        );
    }

    #[test]
    fn no_prefix_rename_keeps_gits_exact_path() {
        // `git diff -M --no-prefix`: the pair `--- b/src.txt` / `+++ b/dst.txt`
        // differs, which reads as prefixed and would strip a real `b/`
        // directory; `rename to` already named the file exactly.
        let diff = "diff --git b/src.txt b/dst.txt\nsimilarity index 96%\nrename from b/src.txt\nrename to b/dst.txt\nindex e8823e1..10adcaf 100644\n--- b/src.txt\n+++ b/dst.txt\n@@ -28,3 +28,4 @@\n 28\n 29\n 30\n+31\n";
        let out = condense_unified_diff_strict(diff).expect("must parse");
        assert!(
            out.contains("[file] b/dst.txt (renamed from b/src.txt) (+1 -0)"),
            "got:\n{out}"
        );
        // `diff --cc X` carries one path, so it cannot settle the prefix
        // question; the pair does (`b/x` twice => a real `b/` directory).
        let cc = "diff --cc b/x\nindex 1,2..3\n--- b/x\n+++ b/x\n@@@ -1,3 -1,3 +1,3 @@@\n  ctx one\n- conflict MAIN\n -conflict LEFT\n++conflict RESOLVED\n  ctx two\n";
        let out = condense_unified_diff_strict(cc).expect("must parse");
        assert!(out.contains("[file] b/x (+1 -2)"), "got:\n{out}");
    }

    #[test]
    fn quoted_paths_are_dequoted_before_the_prefix_strip() {
        // `core.quotepath` (git's default) wraps a non-ASCII path in quotes
        // on every header line; the prefix strip has to see through them.
        let diff = "diff --git \"a/caf\\303\\251.txt\" \"b/caf\\303\\251.txt\"\nindex 587be6b..975fbec 100644\n--- \"a/caf\\303\\251.txt\"\n+++ \"b/caf\\303\\251.txt\"\n@@ -1 +1 @@\n-x\n+y\n";
        let out = condense_unified_diff_strict(diff).expect("must parse");
        assert!(
            out.contains("[file] café.txt (+1 -1)"),
            "got:\n{out}"
        );
        // The `diff --git` fallback name (no `+++` follows a binary section)
        // dequotes the same way.
        let bin = "diff --git \"a/caf\\303\\251.bin\" \"b/caf\\303\\251.bin\"\nindex 587be6b..975fbec 100644\nBinary files \"a/caf\\303\\251.bin\" and \"b/caf\\303\\251.bin\" differ\n";
        let out = condense_unified_diff_strict(bin).expect("must parse");
        assert!(
            out.contains("[file] café.bin (binary)"),
            "got:\n{out}"
        );
        // `rename to` / `copy to` name the entry directly and are quoted the
        // same way; otherwise one file gets two spellings across a stream.
        let ren = "diff --git \"a/caf\\303\\251.txt\" \"b/na\\303\\257ve.txt\"\nsimilarity index 100%\nrename from \"caf\\303\\251.txt\"\nrename to \"na\\303\\257ve.txt\"\n";
        let out = condense_unified_diff_strict(ren).expect("must parse");
        assert!(
            out.contains("[file] naïve.txt (renamed from café.txt)"),
            "got:\n{out}"
        );
    }

    #[test]
    fn header_pair_without_a_hunk_is_never_consumed() {
        // A stray marked pair inside an open, hunkless git section is not a
        // header (git only ever emits `---`/`+++` before the first `@@`): it
        // is lost content and forces raw, instead of renaming the section
        // to `leftover added` and dropping both lines.
        let diff = "diff --git a/x b/x\nold mode 100644\nnew mode 100755\n--- leftover removed\n+++ leftover added\ndiff --git a/o b/o\n--- a/o\n+++ b/o\n@@ -1 +1 @@\n-1\n+2\n";
        assert!(condense_unified_diff_strict(diff).is_none());
    }

    #[test]
    fn fact_names_split_at_the_producer_boundary() {
        // GNU diff's `Only in <dir>: <file>` separator is the first `: `;
        // `Binary files X and Y differ` names the same path under two
        // roots, so a filename containing ` and ` must not be split.
        let diff = "Only in docs: notes: draft.md\nBinary files a/black and white.png and b/black and white.png differ\nBinary files x.bin and y.bin differ\n--- a/f.txt\n+++ b/f.txt\n@@ -1 +1 @@\n-x\n+y\n";
        let out = condense_unified_diff_strict(diff).expect("plain diff -r stream must parse");
        assert!(
            out.contains("[file] docs/notes: draft.md (only in one side)"),
            "got:\n{out}"
        );
        // GNU roots are directories, never prefixes: `b/` stays.
        assert!(
            out.contains("[file] b/black and white.png (binary)"),
            "got:\n{out}"
        );
        assert!(out.contains("[file] y.bin (binary)"), "got:\n{out}");
        assert!(out.contains("[file] f.txt (+1 -1)"), "got:\n{out}");
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

    // --- condense_stdin: the decode → parse → guard pipeline ---

    #[test]
    fn stdin_colored_stream_parses_with_content_bytes_intact() {
        // Reproducer 9: `git diff --color` through a pipe used to condense to
        // a silently empty result. Structure is read through the stripped
        // view, so the stream parses and the label is clean; the body lines
        // are the user's bytes, escapes included — the parser cannot tell
        // git's colouring from escapes that are part of the content, and
        // fidelity outranks tidiness.
        let colored = "\u{1b}[1mdiff --git a/x b/x\u{1b}[m\n\u{1b}[1m--- a/x\u{1b}[m\n\u{1b}[1m+++ b/x\u{1b}[m\n\u{1b}[36m@@ -1 +1 @@\u{1b}[m\n\u{1b}[31m-old_line_content\u{1b}[m\n\u{1b}[32m+new_line_content\u{1b}[m\n";
        let out = condense_stdin(colored.as_bytes()).expect("colored diff must parse");
        assert!(out.contains("[file] x (+1 -1)"), "got:\n{out}");
        assert!(out
            .lines()
            .any(|l| l == "\u{1b}[31m-old_line_content\u{1b}[m"));
        // GNU `diff -u --color=always`: a bare header pair whose `@@` is
        // coloured, so rule 4's opens-a-hunk lookahead must read through
        // the stripped view too.
        let gnu = "\u{1b}[1m--- n.txt\t2026-01-01\u{1b}[m\n\u{1b}[1m+++ n.txt\t2026-01-02\u{1b}[m\n\u{1b}[36m@@ -1 +1 @@\u{1b}[m\n\u{1b}[31m-old\u{1b}[m\n\u{1b}[32m+new\u{1b}[m\n";
        let out = condense_stdin(gnu.as_bytes()).expect("coloured GNU diff must parse");
        assert!(out.contains("[file] n.txt (+1 -1)"), "got:\n{out}");
    }

    #[test]
    fn ansi_inside_content_lines_survives_verbatim() {
        // Escapes that are the *content* (snapshot fixtures of colouring
        // tools) were stripped along with git's decoration, so a reviewer
        // could not see whether the escapes were what changed. The file
        // path (`rtk diff a b`) was already byte-faithful on the same
        // content; the stdin path now is too.
        let diff = "diff --git a/s b/s\n--- a/s\n+++ b/s\n@@ -1 +1 @@\n-\u{1b}[31merror\u{1b}[0m old\n+\u{1b}[31merror\u{1b}[0m new\n";
        let out = condense_stdin(diff.as_bytes()).expect("must parse");
        assert!(out.contains("[file] s (+1 -1)"), "got:\n{out}");
        assert!(out.lines().any(|l| l == "-\u{1b}[31merror\u{1b}[0m old"));
        assert!(out.lines().any(|l| l == "+\u{1b}[31merror\u{1b}[0m new"));
        // And a CR that follows the content's own reset sequence still
        // counts as the line's CR for structural purposes.
        let crlf = "--- a/s\n+++ b/s\n@@ -1 +1 @@\n-x\u{1b}[0m\r\n+y\u{1b}[0m\r\n";
        let out = condense_unified_diff(crlf);
        assert!(out.split('\n').any(|l| l == "-x\u{1b}[0m\r"), "got:\n{out:?}");
    }

    #[test]
    fn stdin_non_utf8_non_diff_falls_back_to_exact_bytes() {
        // Reproducer 10: non-UTF-8 stdin used to be a hard error. This pins
        // the no-panic, `None` answer on non-UTF-8 input that is not a
        // diff; the byte-fidelity gate itself is pinned by the diff-shaped
        // variant below.
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

    /// Round trip against real producers: a file whose name needs quoting
    /// (non-ASCII, an embedded `"`, a space, a tab) was created, the real
    /// `git diff` / `diff -ru` was run, and the rendered `[file]` name must
    /// be byte-equal to the actual filename under the producer's own root.
    /// Table-driven so a missing decode and a spurious one both fail.
    #[test]
    fn rendered_names_round_trip_to_the_real_filenames() {
        let cases: &[(&str, &str, &[&str])] = &[
            (
                "git_diff_quotepath",
                include_str!("../../../tests/fixtures/diff/git_diff_quotepath_raw.txt"),
                &[
                    "[file] bin é.dat (binary)",
                    "[file] café.txt (+1 -1)",
                    "[file] naïve.txt (renamed from old é.txt)",
                    "[file] quo\"te.txt (+1 -1)",
                    "[file] sp ace.txt (+1 -1)",
                    "[file] ta\tb.txt (+1 -1)",
                ],
            ),
            // diffutils 3.10: `Only in` / `Binary files` names bare, echo
            // and headers C-quoted; `diff -ru g1/ g2/` echoes `g1/:`.
            (
                "diff_ru_quoted_310",
                include_str!("../../../tests/fixtures/diff/diff_ru_quoted_310_raw.txt"),
                &[
                    "[file] g2/bin é.dat (binary)",
                    "[file] g2/c.txt (+1 -1)",
                    "[file] g2/café.txt (+1 -1)",
                    "[file] g2/only right.txt (only in one side)",
                    "[file] g1/only é.txt (only in one side)",
                    "[file] g2/quo\"te.txt (+1 -1)",
                    "[file] g2/sp ace.txt (+1 -1)",
                ],
            ),
            // diffutils 3.12, C locale: `Only in` / `Binary files` names
            // shell-quoted with `$'\303\251'` for the non-ASCII bytes.
            (
                "diff_ru_quoted_312_c",
                include_str!("../../../tests/fixtures/diff/diff_ru_quoted_312_c_raw.txt"),
                &[
                    "[file] g2/bin é.dat (binary)",
                    "[file] g2/c.txt (+1 -1)",
                    "[file] g2/café.txt (+1 -1)",
                    "[file] g2/only right.txt (only in one side)",
                    "[file] g1/only é.txt (only in one side)",
                    "[file] g2/sp ace.txt (+1 -1)",
                ],
            ),
            // diffutils 3.12, UTF-8 locale: shell-quoted for the space only.
            (
                "diff_ru_quoted_312_utf8",
                include_str!("../../../tests/fixtures/diff/diff_ru_quoted_312_utf8_raw.txt"),
                &[
                    "[file] g2/bin é.dat (binary)",
                    "[file] g2/c.txt (+1 -1)",
                    "[file] g2/café.txt (+1 -1)",
                    "[file] g2/only right.txt (only in one side)",
                    "[file] g1/only é.txt (only in one side)",
                    "[file] g2/sp ace.txt (+1 -1)",
                ],
            ),
        ];
        for (name, fixture, labels) in cases {
            let out = condense_unified_diff_strict(fixture)
                .unwrap_or_else(|| panic!("{name}: fell back to raw"));
            for label in *labels {
                assert!(out.contains(label), "{name}: missing {label:?}:\n{out}");
            }
            assert_eq!(
                out.matches("[file] ").count(),
                labels.len(),
                "{name}: extra or missing entries:\n{out}"
            );
        }
    }

    #[test]
    fn svn_index_sections_keep_every_file_including_binaries() {
        // Real `svn diff` (1.14.5): `Index: <path>` + `===` rule per file, a
        // binary as `Cannot display: file marked as a binary type.` with no
        // hunk. The binary section used to vanish as prose while its text
        // siblings condensed — the same silent-loss class as #3769.
        let fixture = include_str!("../../../tests/fixtures/diff/svn_diff_raw.txt");
        let out = condense_unified_diff_strict(fixture).expect("svn diff must parse");
        for label in [
            "[file] b.dat (binary)",
            "[file] d.txt (+0 -1)",
            "[file] new.txt (+1 -0)",
            "[file] sp ace.txt (+1 -1)",
            "[file] sub/deep.txt (+1 -1)",
            "[file] t.txt (+1 -1)",
        ] {
            assert!(out.contains(label), "missing {label:?}:\n{out}");
        }
        assert_eq!(out.matches("[file] ").count(), 6, "got:\n{out}");
        // `svn diff --git` wraps git sections in the same `Index:` lines.
        let git = include_str!("../../../tests/fixtures/diff/svn_diff_git_raw.txt");
        let out = condense_unified_diff_strict(git).expect("svn diff --git must parse");
        assert!(out.contains("[file] b.dat (binary)"), "got:\n{out}");
        assert!(out.contains("[file] t.txt (+1 -1)"), "got:\n{out}");
        assert_eq!(out.matches("[file] ").count(), 6, "got:\n{out}");
        // A binary notice svn wrote in another language leaves the
        // `Index:` section empty: raw, never a vanished file.
        let localized = fixture.replace(
            "Cannot display: file marked as a binary type.",
            "Impossible d'afficher : fichier considéré comme binaire.",
        );
        assert!(condense_unified_diff_strict(&localized).is_none());
        let last = "Index: z.dat\n===================================================================\nImpossible d'afficher : fichier considéré comme binaire.\n";
        assert!(
            condense_unified_diff_strict(&format!("{fixture}{last}")).is_none(),
            "an unread Index section at EOF must force raw"
        );
        // A property change is a header pair with no hunk and a `##` block:
        // rule 8 sends it raw (noise, not loss).
        let prop = include_str!("../../../tests/fixtures/diff/svn_diff_propchange_raw.txt");
        assert!(condense_unified_diff_strict(prop).is_none());
    }

    #[test]
    fn paths_containing_b_slash_and_custom_prefixes_are_named_whole() {
        // Real `git diff` of a binary under a directory named `x b/`: the
        // `diff --git` fallback name used to cut at the last ` b/`.
        let fixture = include_str!("../../../tests/fixtures/diff/git_diff_typechange_raw.txt");
        let out = condense_unified_diff_strict(fixture).expect("must parse");
        assert!(out.contains("[file] x b/y.bin (binary)"), "got:\n{out}");
        // A symlink turned regular file is two sections for one path, as
        // git prints it.
        assert!(out.contains("[file] link (deleted) (+0 -1)"), "got:\n{out}");
        assert!(out.contains("[file] link (new file) (+1 -0)"), "got:\n{out}");
        // `--src-prefix=old/ --dst-prefix=new/`: the `diff --git X Y` line
        // settles the prefixes, whatever they are, and they are stripped
        // exactly — the same way git's own `diff_header_path` reads them.
        let custom =
            include_str!("../../../tests/fixtures/diff/git_diff_custom_prefix_raw.txt");
        let out = condense_unified_diff_strict(custom).expect("must parse");
        assert!(out.contains("[file] f.txt (+1 -1)"), "got:\n{out}");
        assert!(out.contains("[file] x b/y.bin (binary)"), "got:\n{out}");
        assert!(out.contains("[file] link (deleted) (+0 -1)"), "got:\n{out}");
        // `diff.mnemonicPrefix` (`i/`, `w/`, `c/`, `o/`) is the same
        // mechanism; real capture, and one with `* Unmerged path` facts
        // that must fold into their `i/…`/`w/…` sections.
        let mnemonic = include_str!("../../../tests/fixtures/diff/git_diff_mnemonic_raw.txt");
        let out = condense_unified_diff_strict(mnemonic).expect("must parse");
        assert!(out.contains("[file] indented.py (+1 -1)"), "got:\n{out}");
        let ours =
            include_str!("../../../tests/fixtures/diff/git_diff_ours_mnemonic_raw.txt");
        let out = condense_unified_diff_strict(ours).expect("must parse");
        assert!(
            out.contains("[file] b/inb.txt (unmerged) (+4 -0)"),
            "got:\n{out}"
        );
        assert!(out.contains("[file] café.txt (unmerged) (+4 -0)"), "got:\n{out}");
        assert!(out.contains("[file] b/blob.bin (unmerged)"), "got:\n{out}");
        assert_eq!(out.matches("[file] b/inb.txt").count(), 1, "got:\n{out}");
    }

    #[test]
    fn combined_diff_of_a_binary_keeps_its_directory() {
        // Real `git diff` during a conflict on a binary under a directory
        // literally named `b/`: `diff --cc b/blob.bin` carries one
        // unprefixed path and no header pair, so nothing may strip `b/`.
        let fixture = include_str!("../../../tests/fixtures/diff/git_diff_cc_binary_raw.txt");
        let out = condense_unified_diff_strict(fixture).expect("must parse");
        assert!(out.contains("[file] b/blob.bin (binary)"), "got:\n{out}");
        assert!(out.contains("[file] b/inb.txt (+5 -0)"), "got:\n{out}");
        assert!(out.contains("[file] café.txt.bin (binary)"), "got:\n{out}");
        assert!(out.contains("[file] say \"hi\".txt (+5 -0)"), "got:\n{out}");
        assert!(out.contains("[file] tab\there.txt (+5 -0)"), "got:\n{out}");
        // A name holding a newline keeps git's own quoted spelling, with
        // the prefix stripped inside the quotes: one `[file]` line per
        // file, lossless, and spelled as `diff --cc` spelled it.
        assert!(
            out.contains("[file] \"new\\nline.txt\" (+5 -0)"),
            "got:\n{out}"
        );
        assert_eq!(out.matches("[file] ").count(), 11, "got:\n{out}");
    }

    #[test]
    fn hg_diff_echo_carries_one_revision() {
        // Real `hg diff` (not export): `diff -r <rev> <path>` with a single
        // `-r`, names with `a/`/`b/` prefixes, a tab and a `"` in names,
        // and hg's binary fact.
        let fixture = include_str!("../../../tests/fixtures/diff/hg_diff_raw.txt");
        let out = condense_unified_diff_strict(fixture).expect("hg diff must parse");
        for label in [
            "[file] a and b.txt (+1 -1)",
            "[file] b/blob.bin (binary)",
            "[file] b/inb.txt (+1 -1)",
            "[file] back\\slash.txt (+1 -1)",
            "[file] café.txt (+1 -1)",
            "[file] colon: x.txt (+1 -1)",
            "[file] it's.txt (+1 -1)",
            "[file] say \"hi\".txt (+1 -1)",
            "[file] say \"hi\".txt.bin (binary)",
            "[file] tab\there.txt (+1 -1)",
            "[file] tab\there.txt.bin (binary)",
            "[file] trail .txt (+1 -1)",
        ] {
            assert!(out.contains(label), "missing {label:?}:\n{out}");
        }
        // `hg log -p`: a `changeset:` prologue at column 0 before every
        // changeset, then the two-`-r` echo; used to go raw on the latch.
        let log = include_str!("../../../tests/fixtures/diff/hg_log_p_raw.txt");
        let out = condense_unified_diff_strict(log).expect("hg log -p must parse");
        assert!(out.contains("[file] bin.dat (binary)"), "got:\n{out}");
        assert!(out.contains("[file] f.txt (+1 -1)"), "got:\n{out}");
        assert!(out.contains("[file] new.txt (+1 -0)"), "got:\n{out}");
        assert!(out.contains("[file] old.txt (+0 -1)"), "got:\n{out}");
        assert!(!out.contains("summary"), "hg prologue leaked:\n{out}");
        // A binary-only changeset whose message has a marked, non-prose-
        // shaped line: hg's echo closes the region AND counts as reaching
        // its body, so rule 8's shape check does not send it raw.
        let bin = include_str!("../../../tests/fixtures/diff/hg_export_binary_only_raw.txt");
        let out = condense_unified_diff_strict(bin).expect("must parse");
        assert_eq!(out, "[file] b.dat (binary)");
    }

    #[test]
    fn gnu_identical_and_differ_facts_are_entries() {
        // Real `diff -rus` / `diff -rq`: `Files X and Y are identical` and
        // `Files X and Y differ` share the `Binary files` shape.
        let rus = include_str!("../../../tests/fixtures/diff/diff_rus_raw.txt");
        let out = condense_unified_diff_strict(rus).expect("must parse");
        assert!(out.contains("[file] g2/c.txt (+1 -1)"), "got:\n{out}");
        assert!(out.contains("[file] g2/s.txt (identical)"), "got:\n{out}");
        let rq = include_str!("../../../tests/fixtures/diff/diff_rq_raw.txt");
        let out = condense_unified_diff_strict(rq).expect("must parse");
        assert_eq!(out, "[file] g2/c.txt (differs)");
    }

    #[test]
    fn a_leading_bom_does_not_hide_the_first_section() {
        // Real PowerShell 5.1 `git diff | Out-File -Encoding utf8`: a UTF-8
        // BOM in front of `diff --git`, CRLF line ends. The first section is
        // a binary, so nothing downstream re-opens it: without the strip
        // its `diff --git` line is prose and `blob.bin` has no entry.
        let bom = include_bytes!("../../../tests/fixtures/diff/git_diff_bom_raw.txt");
        assert!(bom.starts_with(b"\xEF\xBB\xBF"));
        assert!(bom.contains(&b'\r'));
        let out = condense_stdin(bom).expect("must parse");
        assert!(out.starts_with("[file] blob.bin (binary)\n"), "got:\n{out}");
        assert!(
            out.contains("[file] doomed.txt (deleted) (+0 -3)\n-line a\r\n"),
            "CRLF content bytes must survive:\n{out}"
        );
        assert!(out.contains("(renamed from renamed_src.txt)"), "got:\n{out}");
    }

    #[test]
    fn unquote_shell_decodes_every_segment_kind() {
        assert_eq!(unquote_shell("'only '$'\\303\\251''.txt'"), "only é.txt");
        assert_eq!(unquote_shell("'sp ace.txt'"), "sp ace.txt");
        assert_eq!(unquote_shell("$'\\x41\\e'"), "A\u{1b}");
        assert_eq!(unquote_shell("'it'\\''s'"), "it's");
        // diffutils ≥ 3.11 picks `"…"` for a name holding a `'`; only
        // `\"`, `\\`, `\$` and `` \` `` escape inside.
        assert_eq!(unquote_shell("\"Kyle's notes.txt\""), "Kyle's notes.txt");
        assert_eq!(unquote_shell("\"say \\\"hi\\\" \\$x\""), "say \"hi\" $x");
        assert_eq!(unquote_shell("bare.txt"), "bare.txt");
        // A decoded name that would split a `[file]` line, or is not
        // UTF-8, keeps the producer's own spelling.
        assert_eq!(unquote_shell("$'\\x41\\n'"), "$'\\x41\\n'");
        assert_eq!(dequote("\"a/new\\nline.txt\""), "\"a/new\\nline.txt\"");
        assert_eq!(dequote("\"a/caf\\351.txt\""), "\"a/caf\\351.txt\"");
        assert_eq!(dequote("\"a/quo\\\"te\\\\x\\t.txt\""), "a/quo\"te\\x\t.txt");
        assert_eq!(dequote("\"\\1\\12\""), "\"\\1\\12\"");
        assert_eq!(dequote("\"\\1\\7\""), "\u{1}\u{7}");
        assert_eq!(dequote("plain"), "plain");
        // The shared-tail split behind `diff --git X Y` and `Binary files
        // X and Y`: one path under two roots, at a `/` boundary only.
        assert_eq!(shared_tail("a/x/f", "b/x/f"), Some(4));
        assert_eq!(shared_tail("./f", "../new/f"), Some(2));
        assert_eq!(shared_tail("old/f", "bold/f"), Some(2));
        assert_eq!(shared_tail("f", "new/f"), Some(1));
        assert_eq!(shared_tail("f", "f"), Some(1));
        assert_eq!(shared_tail("x.bin", "y.bin"), None);
        assert_eq!(shared_tail("b.bin", "a and b.bin"), None);
        assert_eq!(strip_timestamp("g1/ta\tb.txt\t2026-09-07 09:04:54 -0400"), "g1/ta\tb.txt");
        assert_eq!(strip_timestamp("a/tab\there.txt\tMon Sep 07 12:55:24 2026 +0000"), "a/tab\there.txt");
        assert_eq!(strip_timestamp("t.txt\t(revision 1)"), "t.txt");
        assert_eq!(strip_timestamp("a/sp ace.txt\t"), "a/sp ace.txt");
        assert_eq!(strip_timestamp("a/tab\tb.txt"), "a/tab\tb.txt");
    }

    #[test]
    fn gnu_facts_the_parser_cannot_read_force_raw_even_after_an_entry() {
        // Real `LC_ALL=C diff -ru --no-dereference g1 g2` on trees holding a
        // fifo and a symlink: GNU sorts its output, so `Only in` parses
        // first and used to leave the latch unarmed; `b_fifo` and `b_link`
        // then vanished with no fallback.
        let fixture = include_str!("../../../tests/fixtures/diff/diff_ru_fifo_symlink_raw.txt");
        assert!(condense_unified_diff_strict(fixture).is_none());
        // The same hole, with a translated fact after an English one.
        let mixed = "Only in g1: a_only.txt\nSeulement dans g2: b.txt\ndiff -ru g1/c.txt g2/c.txt\n--- g1/c.txt\n+++ g2/c.txt\n@@ -1 +1 @@\n-two\n+TWO\n";
        assert!(condense_unified_diff_strict(mixed).is_none());
        // And after a text section, with no echo left to arm anything.
        let late = "diff -ru g1/c.txt g2/c.txt\n--- g1/c.txt\n+++ g2/c.txt\n@@ -1 +1 @@\n-two\n+TWO\nSymbolic links g1/l and g2/l differ\n";
        assert!(condense_unified_diff_strict(late).is_none());
    }

    #[test]
    fn hg_export_with_a_binary_first_file_keeps_the_binary() {
        // Real `hg export tip` whose first changed file is binary: hg emits
        // its `diff -r` echo for it, then `Binary file bin.dat has changed`.
        // The message region used to stay open until the first header pair
        // and swallow the fact as prose.
        let fixture = include_str!("../../../tests/fixtures/diff/hg_export_binary_raw.txt");
        let out = condense_unified_diff_strict(fixture).expect("must parse");
        assert!(out.contains("[file] bin.dat (binary)"), "got:\n{out}");
        assert!(out.contains("[file] f.txt (+1 -1)"), "got:\n{out}");
        // The fact arms are live inside the hg region as well, so the same
        // line without its echo is still a fact, not prose.
        let no_echo = fixture.replace("diff -r 344a53e211f0 -r f1f7989a6f4d bin.dat\n", "");
        let out = condense_unified_diff_strict(&no_echo).expect("must parse");
        assert!(out.contains("[file] bin.dat (binary)"), "got:\n{out}");
    }

    #[test]
    fn dirty_and_moved_submodule_is_one_entry() {
        // Real `git -c diff.ignoreSubmodules=none diff --submodule=log`: a
        // submodule that is untracked-dirty, modified-dirty and moved is
        // three lines — one path, one entry.
        let fixture =
            include_str!("../../../tests/fixtures/diff/git_diff_submodule_dirty_moved_raw.txt");
        let out = condense_unified_diff_strict(fixture).expect("must parse");
        assert!(
            out.contains(
                "[file] sub (submodule, untracked content, modified content, 7470085..66e6a8c)"
            ),
            "got:\n{out}"
        );
        assert_eq!(out.matches("[file] sub").count(), 1, "got:\n{out}");
        assert!(out.contains("[file] o.txt (+1 -1)"), "got:\n{out}");
        // The direction qualifier rides along.
        let diff = "Submodule café contains modified content\nSubmodule café e920674..659c607 (rewind):\n  < c2\ndiff --git a/o b/o\n--- a/o\n+++ b/o\n@@ -1 +1 @@\n-1\n+2\n";
        let out = condense_unified_diff_strict(diff).expect("must parse");
        assert!(
            out.contains("[file] café (submodule, modified content, e920674..659c607 rewind)"),
            "got:\n{out}"
        );
        // Inside an mbox message the strict `<hex>..<hex>:` shape is a fact
        // too: `format-patch --submodule=log` puts it where a section goes.
        let patch =
            include_str!("../../../tests/fixtures/diff/git_format_patch_submodule_raw.txt");
        let out = condense_unified_diff_strict(patch).expect("must parse");
        assert!(
            out.contains("[file] sub (submodule, e1269cf..4ef7e41)"),
            "submodule bump in patch 1/2 vanished:\n{out}"
        );
        assert!(out.contains("[file] t.txt (+1 -1)"), "got:\n{out}");
        assert!(!out.contains("s2"), "log body leaked:\n{out}");
    }

    #[test]
    fn word_diff_hunks_with_no_marked_line_fall_back_raw() {
        // Real `git diff --word-diff` where every hunk line is indented:
        // the inline `[-…-]{+…+}` markers sit behind the indent, so the
        // whole hunk is context by prefix and the file used to vanish
        // beside a binary sibling. A hunk that closes with no marked line
        // is not a unified-diff shape at all.
        let fixture = include_str!("../../../tests/fixtures/diff/git_diff_word_diff_raw.txt");
        assert!(condense_unified_diff_strict(fixture).is_none());
        // `-w` on a whitespace-only change: git omits the hunk entirely, so
        // an all-context hunk cannot come from there (real capture behind
        // `git_diff_truncated_raw.txt`, a `-w` stream).
    }

    #[test]
    fn gnu_facts_sorted_before_the_first_arm_force_raw() {
        // GNU sorts its output: an unread fact (`File X is a directory
        // while file Y is a regular file`, a fifo line) may come BEFORE the
        // first `Only in` / `Files` line, and `-q` streams never carry an
        // echo, so those arms are the only thing that can settle it.
        let dir_vs_file =
            include_str!("../../../tests/fixtures/diff/diff_ru_dir_vs_file_raw.txt");
        assert!(condense_unified_diff_strict(dir_vs_file).is_none());
        let fifo_first = include_str!("../../../tests/fixtures/diff/diff_rq_fifo_first_raw.txt");
        assert!(condense_unified_diff_strict(fifo_first).is_none());
        let bin_then_fr = "Binary files g1/b.bin and g2/b.bin differ\nSeulement dans g2: x.txt\n";
        assert!(condense_unified_diff_strict(bin_then_fr).is_none());
        // Each arm alone still condenses.
        assert_eq!(
            condense_unified_diff_strict("Files g1/c.txt and g2/c.txt differ\n").as_deref(),
            Some("[file] g2/c.txt (differs)")
        );
    }

    #[test]
    fn truncated_stream_ending_in_a_bare_header_forces_raw() {
        // Real `git diff -w | head -n 11`: the cut lands right after the
        // second file's `diff --git` + `index` lines. git never emits a
        // header-only section, so the file it names is lost content.
        let fixture = include_str!("../../../tests/fixtures/diff/git_diff_truncated_raw.txt");
        assert!(condense_unified_diff_strict(fixture).is_none());
        let after_header = "diff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-x\n+y\ndiff --git a/second b/second\n";
        assert!(condense_unified_diff_strict(after_header).is_none());
    }

    #[test]
    fn diffutils_312_shell_quoting_round_trips() {
        // Real diffutils 3.12 captures: `"…"` for a name holding a `'`,
        // `$'\t'` for a tab, a quoted directory that itself holds `: `.
        let cases: &[(&str, &str, &[&str])] = &[
            (
                "apostrophe",
                include_str!("../../../tests/fixtures/diff/diff_ru_312_apostrophe_raw.txt"),
                &[
                    "[file] q2/Kyle's notes.txt (only in one side)",
                    "[file] q2/bin's.dat (binary)",
                    "[file] q2/it's.txt (+1 -1)",
                    "[file] q2/ta\tb.txt (+1 -1)",
                    "[file] q2/tab\tbin.dat (binary)",
                ],
            ),
            (
                "apostrophe_dirs",
                include_str!("../../../tests/fixtures/diff/diff_ru_312_apostrophe_dirs_raw.txt"),
                &[
                    "[file] Kyle's new/f.txt (+1 -1)",
                    "[file] Kyle's new/only.txt (only in one side)",
                ],
            ),
            (
                "colon_dirs",
                include_str!("../../../tests/fixtures/diff/diff_ru_312_colon_dirs_raw.txt"),
                &[
                    "[file] e: y/b.bin (binary)",
                    "[file] e: y/f.txt (+1 -1)",
                    "[file] e: y/only.txt (only in one side)",
                ],
            ),
            // Roots deeper than one component (`diff -ru . ../new`) and
            // roots literally named `a`/`b` (`diff -ru a b`).
            (
                "dot_roots",
                include_str!("../../../tests/fixtures/diff/diff_ru_dot_raw.txt"),
                &[
                    "[file] ../new/ lead.txt (",
                    "[file] ../new/$'dollar.txt (",
                    "[file] ../new/a/ina.txt (",
                    "[file] ../new/a and b and c.txt (",
                    "[file] ../new/a and b.bin (binary)",
                    "[file] ../new/and/x and y.txt (",
                    "[file] ../new/b/inb.txt (",
                    "[file] ../new/café.bin (binary)",
                    "[file] ./café.only (only in one side)",
                    "[file] ../new/café.txt (",
                    "[file] ../new/it's.bin (binary)",
                    "[file] ../new/it's.only (only in one side)",
                    "[file] ../new/it's.txt (",
                    "[file] ../new/sp ace.txt (",
                    "[file] ./x b/only and me.txt (only in one side)",
                    "[file] ../new/x b/y.txt (",
                ],
            ),
            (
                "ab_roots",
                include_str!("../../../tests/fixtures/diff/diff_ru_ab_raw.txt"),
                &[
                    "[file] b/c.txt (+1 -1)",
                    "[file] b/onlyb.txt (only in one side)",
                    "[file] b/x/bin.dat (binary)",
                ],
            ),
        ];
        for (name, fixture, labels) in cases {
            let out = condense_unified_diff_strict(fixture)
                .unwrap_or_else(|| panic!("{name}: fell back to raw"));
            for label in *labels {
                assert!(out.contains(label), "{name}: missing {label:?}:\n{out}");
            }
            assert_eq!(
                out.matches("[file] ").count(),
                labels.len(),
                "{name}: extra or missing entries:\n{out}"
            );
        }
    }

    #[test]
    fn non_utf8_quoted_names_keep_gits_spelling() {
        // Real git on Latin-1 paths: `"a/caf\350.txt"` and `"a/caf\351.txt"`
        // are two files; decoding them lossily would collapse both to one
        // U+FFFD spelling. git's C-quoted form is kept whole instead.
        let fixture = include_str!("../../../tests/fixtures/diff/git_diff_latin1_raw.txt");
        let out = condense_unified_diff_strict(fixture).expect("must parse");
        assert!(out.contains("[file] \"caf\\350.txt\" (+1 -1)"), "got:\n{out}");
        assert!(out.contains("[file] \"caf\\351.txt\" (+1 -1)"), "got:\n{out}");
        assert!(!out.contains('\u{FFFD}'), "got:\n{out}");
    }

    #[test]
    fn git_stat_separator_and_format_b_prose_condense() {
        // Real `git log --stat -p`: a bare `---` between the message and
        // the diffstat used to read as lost content.
        let stat = include_str!("../../../tests/fixtures/diff/git_log_p_stat_raw.txt");
        let out = condense_unified_diff_strict(stat).expect("--stat -p must parse");
        assert!(out.contains("[file] i.py (+1 -1)"), "got:\n{out}");
        assert!(out.contains("[file] m.txt (mode changed)"), "got:\n{out}");
        // Real `git show --format=%B`: body prose starting with `diff `
        // used to arm the GNU latch and send the stream raw.
        let b = include_str!("../../../tests/fixtures/diff/git_show_format_B_raw.txt");
        let out = condense_unified_diff_strict(b).expect("--format=%B must parse");
        assert_eq!(out, "[file] w.txt (+1 -0)\n+zz");
    }

    #[test]
    fn combined_diffs_with_three_parents_parse() {
        // Real octopus merge: `@@@@` with three old ranges, via `git show
        // --cc` and via `git diff-tree -c` (`diff --combined`).
        for fixture in [
            include_str!("../../../tests/fixtures/diff/git_show_cc_octopus_raw.txt"),
            include_str!("../../../tests/fixtures/diff/git_diff_tree_combined_raw.txt"),
        ] {
            let out = condense_unified_diff_strict(fixture).expect("must parse");
            assert_eq!(out, "[file] x.txt (+1 -1)\n---2\n+++EVIL");
        }
    }

    #[test]
    fn svn_copy_target_without_content_is_listed() {
        // Real `svn diff` after `svn mv`: the target's `Index:` section is
        // empty by construction — listed, not raw, and not dropped.
        let fixture = include_str!("../../../tests/fixtures/diff/svn_diff_move_raw.txt");
        let out = condense_unified_diff_strict(fixture).expect("must parse");
        assert!(out.contains("[file] moved.txt (no content)"), "got:\n{out}");
        assert!(out.contains("[file] t.txt (+0 -1)"), "got:\n{out}");
    }

    #[test]
    fn common_subdirectories_line_is_dropped() {
        // Real `diff -u g1 g2` (no `-r`): informational, no file.
        let fixture = include_str!("../../../tests/fixtures/diff/diff_u_common_subdirs_raw.txt");
        let out = condense_unified_diff_strict(fixture).expect("must parse");
        assert_eq!(out, "[file] g2/c.txt (+1 -1)\n-two\n+TWO");
    }

    #[test]
    fn mbox_prose_quoting_a_hunk_stays_prose() {
        // Real `git format-patch` whose message quotes an old hunk at
        // column 0: rule 5 leaves it prose (no section is open) and rule 8
        // exempts its marked lines.
        let fixture =
            include_str!("../../../tests/fixtures/diff/git_format_patch_quoted_hunk_raw.txt");
        let out = condense_unified_diff_strict(fixture).expect("must parse");
        assert_eq!(out.matches("[file] ").count(), 1, "got:\n{out}");
        assert!(out.contains("[file] f (+1 -1)"), "got:\n{out}");
        // A stat-ful patch whose FIRST section lost its headers while the
        // second survived: `past_diffstat` is what catches it.
        let lost = "From 0e7632a01b00c70cbc9dafcf1f23c71fa6b10de1 Mon Sep 17 00:00:00 2001\nSubject: [PATCH] two files\n\n---\n x.txt | 2 +-\n y.txt | 2 +-\n 2 files changed, 2 insertions(+), 2 deletions(-)\n\n-a\n+b\ndiff --git a/y.txt b/y.txt\n--- a/y.txt\n+++ b/y.txt\n@@ -1 +1 @@\n-p\n+q\n-- \n2.54.0\n";
        assert!(condense_unified_diff_strict(lost).is_none());
        // `format-patch --interdiff`: the interdiff block is indented and
        // stays prose; `diff -u --label OLD --label NEW` names by label.
        let inter =
            include_str!("../../../tests/fixtures/diff/git_format_patch_interdiff_raw.txt");
        let out = condense_unified_diff_strict(inter).expect("must parse");
        assert_eq!(out, "[file] v.txt (new file) (+1 -0)\n+v2");
        let label = include_str!("../../../tests/fixtures/diff/diff_u_label_raw.txt");
        let out = condense_unified_diff_strict(label).expect("must parse");
        assert_eq!(out, "[file] NEW (+1 -1)\n-two\n+TWO");
    }

    #[test]
    fn non_ascii_names_never_panic_in_the_shared_tail_scan() {
        // Real `git diff --cached -M` with `core.quotepath=false` on a
        // rename between two emoji names: `😀` and `🙀` share trailing
        // bytes, and the byte-wise suffix scan used to slice mid-character
        // and abort the process — no output, not even the raw fallback.
        let fixture = include_str!("../../../tests/fixtures/diff/git_diff_emoji_rename_raw.txt");
        let out = condense_unified_diff_strict(fixture).expect("must parse");
        assert_eq!(out, "[file] 🙀.txt (renamed from 😀.txt) (+1 -0)\n+world");
        // The shared bytes of two different characters are not a tail.
        assert_eq!(shared_tail("😀.txt", "🙀.txt"), None);
        assert_eq!(shared_tail("a/é.txt", "b/©.txt"), None);
        assert_eq!(shared_tail("a/😀.txt", "b/😀.txt"), Some(9));
        // GNU 3.12 shell-quotes the same bytes; `diff --no-index` under
        // `--no-prefix` names two unrelated binaries.
        let gnu = "Binary files ''$'\\360\\237\\230\\200''.bin' and ''$'\\360\\237\\231\\200''.bin' differ\n";
        assert_eq!(
            condense_unified_diff_strict(gnu).as_deref(),
            Some("[file] 🙀.bin (binary)")
        );
    }

    #[test]
    fn no_index_paths_keep_their_directories() {
        // Real `git diff --no-index d1/f.txt d2/f.txt`: `a/d1/f.txt` vs
        // `b/d2/f.txt` — the prefix is one component, the rest is path.
        let dirs = include_str!("../../../tests/fixtures/diff/git_diff_no_index_dirs_raw.txt");
        let out = condense_unified_diff_strict(dirs).expect("must parse");
        assert_eq!(out, "[file] d2/f.txt (+1 -1)\n-one\n+two");
        // Real `git diff --no-index --no-prefix x.bin y.bin`: no shared
        // tail, no `b/` — the last word names it.
        let bins =
            include_str!("../../../tests/fixtures/diff/git_diff_no_index_no_prefix_bin_raw.txt");
        assert_eq!(
            condense_unified_diff_strict(bins).as_deref(),
            Some("[file] y.bin (binary)")
        );
    }

    #[test]
    fn bare_only_in_with_a_colon_root_uses_the_streams_roots() {
        // Real diffutils 3.10 `diff -ru 'd: x' 'e: y'`: the `Only in` line
        // is bare, so `e: y: only.txt` is ambiguous on its own; the echo,
        // the header pair and the `Binary files` line already named the
        // roots, and the split follows them.
        let fixture =
            include_str!("../../../tests/fixtures/diff/diff_ru_310_colon_dirs_raw.txt");
        let out = condense_unified_diff_strict(fixture).expect("must parse");
        for label in [
            "[file] e: y/b.bin (binary)",
            "[file] e: y/f.txt (+1 -1)",
            "[file] e: y/only.txt (only in one side)",
        ] {
            assert!(out.contains(label), "missing {label:?}:\n{out}");
        }
        assert_eq!(out.matches("[file] ").count(), 3, "got:\n{out}");
    }

    #[test]
    fn tab_names_survive_hg_nodates() {
        // Real `hg diff --nodates` on `v<TAB>2.txt`: no timestamp follows
        // the tab, and `2.txt` is not timestamp-shaped.
        let fixture = include_str!("../../../tests/fixtures/diff/hg_diff_nodates_tab_raw.txt");
        let out = condense_unified_diff_strict(fixture).expect("must parse");
        assert_eq!(out, "[file] v\t2.txt (+1 -1)\n-two\n+TWO");
        assert_eq!(strip_timestamp("a/ta\tYes.txt"), "a/ta\tYes.txt");
        assert_eq!(strip_timestamp("a/v\t2.txt"), "a/v\t2.txt");
        assert_eq!(
            strip_timestamp("/dev/null\tThu Jan 01 00:00:00 1970 +0000"),
            "/dev/null"
        );
    }

    #[test]
    fn unmerged_fact_cut_by_a_newline_folds_into_its_section() {
        // Real `git diff --ours` on a conflicted `nl\nx.txt`: git prints
        // the fact line unquoted, so the newline splits it; the section
        // keeps the quoted spelling and the fact folds into it.
        let fixture = include_str!("../../../tests/fixtures/diff/git_diff_ours_newline_raw.txt");
        let out = condense_unified_diff_strict(fixture).expect("must parse");
        assert!(
            out.contains("[file] \"nl\\nx.txt\" (unmerged) (+4 -0)"),
            "got:\n{out}"
        );
        assert!(!out.contains("[file] nl "), "phantom entry:\n{out}");
        assert!(!out.contains("[file] \"b/nl"), "prefix left inside the quotes:\n{out}");
    }

    #[test]
    fn quoted_names_have_one_spelling_across_a_stream() {
        // Real `git log -p` (Docker git 2.47) over renames of a Latin-1
        // name, a newline name, a `"`-leading name and a backslash name:
        // the retained quoted spelling never carries the prefix inside the
        // quotes, so `rename to` and the header pair agree.
        let fixture =
            include_str!("../../../tests/fixtures/diff/git_log_p_quoted_renames_raw.txt");
        let out = condense_unified_diff_strict(fixture).expect("must parse");
        assert!(
            out.contains("(renamed from \"caf\\351.txt\")"),
            "got:\n{out}"
        );
        assert!(out.contains("[file] \"caf\\351.txt\" (+1 -1)"), "got:\n{out}");
        assert!(out.contains("(renamed from \"nl\\nx.txt\")"), "got:\n{out}");
        assert!(out.contains("[file] back\\slash2.txt (renamed from back\\slash.txt)"), "got:\n{out}");
        // The file that really lives under `b/` keeps exactly one `b/`.
        assert!(out.contains("[file] \"b/caf\\351.txt\" (+1 -1)"), "got:\n{out}");
        assert!(!out.contains("\"b/b/"), "prefix left inside the quotes:\n{out}");
        assert!(!out.contains("\"a/"), "prefix left inside the quotes:\n{out}");
    }

    #[test]
    fn real_captures_pin_the_remaining_producer_shapes() {
        // `git format-patch --stdout --no-stat --cover-letter -2`: the
        // `--no-stat` region is judged by shape (rule 8) — bullets and a
        // column-0 `--no-stat` stay prose, both sections render.
        let no_stat =
            include_str!("../../../tests/fixtures/diff/git_format_patch_no_stat_cover_raw.txt");
        assert_eq!(
            condense_unified_diff_strict(no_stat).as_deref(),
            Some("[file] bin.dat (binary)\n[file] dst.txt (renamed from src.txt) (+1 -0)\n+three\n[file] f.txt (+1 -1)\n-b\n+B\n[file] f.txt (+1 -1)\n-c\n+C")
        );
        // `git diff --cached --color=always`: git colours the marker and
        // the content separately; content lines keep their own bytes.
        let color = include_str!("../../../tests/fixtures/diff/git_diff_color_raw.txt");
        assert_eq!(
            condense_unified_diff_strict(color).as_deref(),
            Some("[file] bin.dat (binary)\n[file] dst.txt (renamed from src.txt) (+1 -0)\n\u{1b}[32m+\u{1b}[m\u{1b}[32mthree\u{1b}[m\n[file] f.txt (+1 -1)\n\u{1b}[31m-b\u{1b}[m\n\u{1b}[32m+\u{1b}[m\u{1b}[32mB\u{1b}[m")
        );
        // `git diff --cached -M --no-prefix`: `diff --git src.txt dst.txt`
        // shares no tail; `rename to` names it and the pair may not.
        let np = include_str!("../../../tests/fixtures/diff/git_diff_no_prefix_rename_raw.txt");
        let out = condense_unified_diff_strict(np).expect("must parse");
        assert!(
            out.contains("[file] dst.txt (renamed from src.txt) (+1 -0)"),
            "got:\n{out}"
        );
        // `--submodule=diff`: nested column-0 sections under the range.
        let sd = include_str!("../../../tests/fixtures/diff/git_diff_submodule_diff_raw.txt");
        assert_eq!(
            condense_unified_diff_strict(sd).as_deref(),
            Some("[file] o.txt (+1 -1)\n-k\n+k2\n[file] sub (submodule, 0641fef..19e2e29)\n[file] sub/s.txt (+1 -1)\n-s\n+s2\n[file] sub/sb.bin (binary)")
        );
        // `git log --numstat -p`: `-<TAB>-<TAB>b.bin` before the diff.
        let ns = include_str!("../../../tests/fixtures/diff/git_log_numstat_p_raw.txt");
        let out = condense_unified_diff_strict(ns).expect("--numstat -p must parse");
        assert!(out.contains("[file] b.bin (binary)"), "got:\n{out}");
        assert!(out.contains("[file] d/e/f (+1 -0)"), "got:\n{out}");
        // `hg log -p --template '{node|short} {desc}\n'`: a prose prologue
        // with no `changeset:` line, then hg's echo — prose is not a fact.
        let tmpl = include_str!("../../../tests/fixtures/diff/hg_log_p_template_raw.txt");
        let out = condense_unified_diff_strict(tmpl).expect("templated hg log must parse");
        assert!(out.contains("[file] bin.dat (binary)"), "got:\n{out}");
        assert!(out.contains("[file] f.txt (+1 -1)"), "got:\n{out}");
        // `LC_ALL=fr_FR.UTF-8 diff -u`: the marker is localized, position
        // decides — kept after `-x` and after `+y`.
        let fr = include_str!("../../../tests/fixtures/diff/diff_u_fr_no_newline_raw.txt");
        assert_eq!(
            condense_unified_diff_strict(fr).as_deref(),
            Some("[file] n2 (+1 -1)\n-x\n\\ Pas de fin de ligne à la fin du fichier\n+y\n\\ Pas de fin de ligne à la fin du fichier")
        );
        // A git binary section followed by another producer's header pair
        // (two streams concatenated): the pair opens its own section
        // rather than renaming the binary one and stealing its note.
        let concat = "diff --git a/x.bin b/x.bin\nindex 1111111..2222222 100644\nBinary files a/x.bin and b/x.bin differ\n--- g1/m.txt\t2026-01-01 00:00:00.000000000 +0000\n+++ g2/m.txt\t2026-01-01 00:00:00.000000000 +0000\n@@ -1 +1 @@\n-x\n+y\n";
        assert_eq!(
            condense_unified_diff_strict(concat).as_deref(),
            Some("[file] x.bin (binary)\n[file] g2/m.txt (+1 -1)\n-x\n+y")
        );
        assert_eq!(unquote_shell("$'caf\\351.txt'"), "$'caf\\351.txt'");
    }

    #[test]
    fn documented_bounds_hold() {
        // The doc comment's "noise, not loss" bounds, executable.
        // Rule 4: mbox prose quoting a whole well-formed header-plus-hunk
        // block fabricates a phantom entry.
        let phantom = "From 0e7632a01b00c70cbc9dafcf1f23c71fa6b10de1 Mon Sep 17 00:00:00 2001\nSubject: [PATCH] x\n\nThe old change was:\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-x\n+y\n---\n g | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)\n\ndiff --git a/g b/g\n--- a/g\n+++ b/g\n@@ -1 +1 @@\n-p\n+q\n-- \n2.54.0\n";
        let out = condense_unified_diff_strict(phantom).expect("noise, not raw");
        assert_eq!(out.matches("[file] ").count(), 2, "got:\n{out}");
        // Rule 7: a stale `--` in a malformed mbox stream is swallowed.
        let stale = "From 0e7632a01b00c70cbc9dafcf1f23c71fa6b10de1 Mon Sep 17 00:00:00 2001\nSubject: [PATCH] x\n\n---\n g | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)\n\ndiff --git a/g b/g\n--- a/g\n+++ b/g\n@@ -1 +1 @@\n-p\n+q\n--\n-- \n2.54.0\n";
        assert!(condense_unified_diff_strict(stale).is_some());
        // Rule 8: prose imitating a diffstat line ends the exemption early,
        // but format-patch's own `---` + diffstat right after is read as
        // the stat separator, so the patch still condenses; a marked line
        // between the two is lost content and falls back raw.
        let fake_stat = "From 0e7632a01b00c70cbc9dafcf1f23c71fa6b10de1 Mon Sep 17 00:00:00 2001\nSubject: [PATCH] x\n\n rows | 3\n---\n g | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)\n\ndiff --git a/g b/g\n--- a/g\n+++ b/g\n@@ -1 +1 @@\n-p\n+q\n-- \n2.54.0\n";
        let out = condense_unified_diff_strict(fake_stat).expect("noise, not raw");
        assert!(out.contains("[file] g (+1 -1)"), "got:\n{out}");
        assert!(condense_unified_diff_strict(&fake_stat.replace(" rows | 3\n", " rows | 3\n-lost\n")).is_none());
        // Rule 8: a bodyless `--no-stat` region whose prose wraps a short
        // option to column 0 falls back raw.
        let short = "From aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa Mon Sep 17 00:00:00 2001\nSubject: [PATCH 1/2] one\n\nPass\n-p\nto it.\n-- \n2.54.0\n\nFrom bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb Mon Sep 17 00:00:00 2001\nSubject: [PATCH 2/2] two\n\ndiff --git a/y.txt b/y.txt\n--- a/y.txt\n+++ b/y.txt\n@@ -1 +1 @@\n-p\n+q\n-- \n2.54.0\n";
        assert!(condense_unified_diff_strict(short).is_none());
    }

    #[test]
    fn overconsumed_budget_falls_back_to_raw() {
        // One more body line than the `@@` budget allows: the line is
        // budget-owed content the parser cannot place.
        let diff = "--- a/f\n+++ b/f\n@@ -1,2 +1,1 @@\n-a\n+b\n+c\n-d\n";
        assert!(condense_unified_diff_strict(diff).is_none());
    }

    /// Real captures that MUST fall back raw: each pins a producer shape
    /// the parser declines rather than guesses at.
    #[test]
    fn raw_corpus_falls_back_byte_exact() {
        for (name, fixture) in [
            (
                "diff_ru_fr",
                include_str!("../../../tests/fixtures/diff/diff_ru_fr_raw.txt"),
            ),
            (
                "diff_ru_fifo_symlink",
                include_str!("../../../tests/fixtures/diff/diff_ru_fifo_symlink_raw.txt"),
            ),
            (
                "svn_diff_propchange",
                include_str!("../../../tests/fixtures/diff/svn_diff_propchange_raw.txt"),
            ),
            (
                "git_diff_word_diff",
                include_str!("../../../tests/fixtures/diff/git_diff_word_diff_raw.txt"),
            ),
            (
                "diff_ru_dir_vs_file",
                include_str!("../../../tests/fixtures/diff/diff_ru_dir_vs_file_raw.txt"),
            ),
            (
                "diff_rq_fifo_first",
                include_str!("../../../tests/fixtures/diff/diff_rq_fifo_first_raw.txt"),
            ),
            (
                "git_diff_truncated",
                include_str!("../../../tests/fixtures/diff/git_diff_truncated_raw.txt"),
            ),
            (
                "diff_ru_fr_fact_after_section",
                include_str!("../../../tests/fixtures/diff/diff_ru_fr_fact_after_section_raw.txt"),
            ),
        ] {
            assert!(
                condense_unified_diff_strict(fixture).is_none(),
                "{name}: must fall back raw"
            );
            assert_eq!(condense_unified_diff(fixture), fixture, "{name}: not byte-exact");
        }
    }

    #[test]
    fn hg_export_condenses_despite_its_prologue_and_diff_echo() {
        // Real `hg export tip` (Mercurial 7.0.1): `# …` headers and the
        // message at column 0, then a `diff -r <a> -r <b> <file>` echo before
        // each header pair. The prose used to latch `dropped_prologue`, and
        // the echo then turned it into a whole-stream raw fallback: 0%
        // savings for a whole producer. The changeset header now opens a
        // message region, like an mbox `From`.
        let fixture = include_str!("../../../tests/fixtures/diff/hg_export_raw.txt");
        let out = condense_unified_diff_strict(fixture).expect("hg export must parse");
        assert!(out.contains("[file] f.txt (+1 -1)"), "got:\n{out}");
        assert!(out.contains("[file] g.txt (+1 -0)"), "got:\n{out}");
        assert!(!out.contains("bullet"), "message prose leaked:\n{out}");
        // The message region ends at the first file header; the second
        // echo arrives outside it and still marks the `diff -r` context, so
        // an unread column-0 line after it forces raw as in any GNU stream.
        let with_fact = format!("{fixture}Seulement dans b: h.txt\n");
        assert!(condense_unified_diff_strict(&with_fact).is_none());
    }

    #[test]
    fn no_newline_marker_stays_attached_to_the_line_it_describes() {
        // Real `git diff` of two files: `f.txt` changes its first line while
        // its unchanged last line lacks a newline (marker after a context
        // line), and `g.txt` gains a trailing newline (marker after `-y`).
        // The first marker used to land under `+A` and claim the ADDED line
        // had no newline; it must be dropped. The second is the witness of
        // a newline-only change and must stay right under `-y`.
        let fixture = include_str!("../../../tests/fixtures/diff/git_diff_no_newline_raw.txt");
        let out = condense_unified_diff_strict(fixture).expect("must parse");
        let lines: Vec<&str> = out.lines().collect();
        let f = lines
            .iter()
            .position(|l| *l == "[file] f.txt (+1 -1)")
            .expect("f.txt entry");
        assert_eq!(&lines[f..f + 3], &["[file] f.txt (+1 -1)", "-a", "+A"]);
        assert!(
            lines[f + 3].starts_with("[file] g.txt"),
            "marker after a context line leaked under +A:\n{out}"
        );
        let g = f + 3;
        assert_eq!(
            &lines[g..g + 4],
            &[
                "[file] g.txt (+1 -1)",
                "-y",
                "\\ No newline at end of file",
                "+y"
            ]
        );
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
    fn no_prefix_streams_keep_a_and_b_directories() {
        // `git diff --no-prefix` (or `diff.noprefix=true`) emits bare paths,
        // so a leading `a/` or `b/` is a directory, not a prefix to strip.
        // The `diff --git X Y` line settles it — `--no-prefix` repeats one
        // path — including for creations and deletions, where the header
        // pair has a `/dev/null` side and cannot decide on its own. Real
        // `git diff --cached --no-prefix` capture: a modify and a creation
        // under `b/`, a deletion under `a/`, and a path with a space (git
        // appends a tab to those header names).
        let fixture = include_str!("../../../tests/fixtures/diff/git_diff_no_prefix_raw.txt");
        let out = condense_unified_diff(fixture);
        assert_ne!(out, fixture, "--no-prefix stream fell back to raw");
        for want in [
            "[file] a/y.rs (deleted) (+0 -1)",
            "[file] b/has space.txt (+1 -0)",
            "[file] b/x.rs (+1 -1)",
            "[file] b/z.rs (new file) (+1 -0)",
        ] {
            assert!(out.lines().any(|l| l == want), "missing {want:?} in:\n{out}");
        }
        // A prefixed pair with no `diff --git` line still strips once, an
        // unprefixed pair (`diff -u` of two files in a `b/` directory)
        // repeats the path and keeps it, and a prefixed path with a space
        // is still two distinct halves.
        let bare_prefixed = "--- a/b/x.rs\n+++ b/b/x.rs\n@@ -1 +1 @@\n-old\n+new\n";
        assert!(condense_unified_diff(bare_prefixed).contains("[file] b/x.rs (+1 -1)"));
        let bare_plain = "--- b/x.rs\t2026-01-01\n+++ b/x.rs\t2026-01-02\n@@ -1 +1 @@\n-old\n+new\n";
        assert!(condense_unified_diff(bare_plain).contains("[file] b/x.rs (+1 -1)"));
        let spaced = "diff --git a/foo bar b/foo bar\n--- a/foo bar\t\n+++ b/foo bar\t\n@@ -1 +1 @@\n-old\n+new\n";
        assert!(condense_unified_diff(spaced).contains("[file] foo bar (+1 -1)"));
        // `diff --cc <path>` carries one path; a file named `a a` must not
        // read as `a` repeated.
        let cc = "diff --cc a a\n--- a/a a\n+++ b/a a\n@@@ -1,1 -1,1 +1,1 @@@\n- x\n -y\n++z\n";
        let out = condense_unified_diff(cc);
        assert!(out.contains("[file] a a (+1 -2)"), "got:\n{out}");
    }

    #[test]
    fn backslash_line_away_from_a_hunk_is_prose() {
        // Rule 1b keeps the new-side `\ No newline` marker only on the line
        // right after the budget closes. A `\` line anywhere else outside a
        // hunk used to be appended to the section as if it were the marker,
        // rendering prose as diff content.
        let diff = "--- a/f\n+++ b/f\n@@ -1 +1 @@\n-old\n+new\n\\ No newline at end of file\nsome trailing prose\n\\ this is prose, not a diff marker\n";
        let out = condense_unified_diff(diff);
        assert!(out.contains("[file] f (+1 -1)"), "got:\n{out}");
        assert!(out.lines().any(|l| l == "\\ No newline at end of file"));
        assert!(
            !out.contains("this is prose"),
            "stray backslash line rendered as content:\n{out}"
        );
    }

    #[test]
    fn mbox_marked_lines_past_the_diffstat_fall_back_to_raw() {
        // A format-patch series with the second patch's `diff --git` /
        // `---` / `+++` / `@@` block stripped (a hand-edited patch, a mail
        // client's reflow). The orphaned hunk body sits in the mbox message
        // region past the diffstat, where the prose exemption used to
        // swallow it; nothing in the output suggested it was incomplete.
        let series = "From fe6a6d0d8316535b5ac232f5d7cc6d227b187b9e Mon Sep 17 00:00:00 2001\nSubject: [PATCH 1/2] one\n\n- a bullet in the message\n---\n x.txt | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)\n\ndiff --git a/x.txt b/x.txt\n--- a/x.txt\n+++ b/x.txt\n@@ -1 +1 @@\n-a\n+b\n-- \n2.54.0\n\nFrom 0e7632a01b00c70cbc9dafcf1f23c71fa6b10de1 Mon Sep 17 00:00:00 2001\nSubject: [PATCH 2/2] two\n\n- another bullet\n---\n y.txt | 2 +-\n 1 file changed, 1 insertion(+), 1 deletion(-)\n\n-p\n+q\n-- \n2.54.0\n";
        assert!(
            condense_unified_diff_strict(series).is_none(),
            "orphaned hunk body in an mbox region was dropped"
        );
        // The intact series (with the block restored) still parses, bullets
        // and separators included, and the signature is not counted.
        let intact = series.replace(
            "\n-p\n+q\n",
            "\ndiff --git a/y.txt b/y.txt\n--- a/y.txt\n+++ b/y.txt\n@@ -1 +1 @@\n-p\n+q\n",
        );
        let out = condense_unified_diff_strict(&intact).expect("intact series must parse");
        assert!(out.contains("[file] y.txt (+1 -1)"), "got:\n{out}");
        assert!(!out.contains("bullet"), "got:\n{out}");
        // Version notes between `---` and the diffstat (the kernel / b4
        // convention) are column-0 bullets past the separator; they must
        // stay prose, which is why the tolerance ends at the diffstat and
        // not at `---`.
        let notes = intact.replace(
            "---\n y.txt | 2 +-",
            "---\nChanges in v2:\n- rebased\n- dropped the hack\n\n y.txt | 2 +-",
        );
        let out = condense_unified_diff_strict(&notes).expect("version notes must stay prose");
        assert!(out.contains("[file] y.txt (+1 -1)"), "got:\n{out}");
        assert!(!out.contains("rebased"), "got:\n{out}");
    }

    #[test]
    fn a_prose_table_is_not_a_diffstat() {
        // A markdown table in the commit message is space-indented and holds
        // a ` | `, but it is prose: the bullet after it must stay prose too,
        // not read as a marked line past the diffstat.
        let base = "From fe6a6d0d8316535b5ac232f5d7cc6d227b187b9e Mon Sep 17 00:00:00 2001\nSubject: [PATCH] one\n\n col | val\n --- | ---\n row | text\n- a bullet after the table\n---\nSTAT\n 1 file changed, 1 insertion(+), 1 deletion(-)\n\ndiff --git a/x.txt b/x.txt\n--- a/x.txt\n+++ b/x.txt\n@@ -1 +1 @@\n-a\n+b\n-- \n2.54.0\n";
        let out = condense_unified_diff_strict(&base.replace("STAT", " x.txt | 2 +-"))
            .expect("a table in the message must not force raw");
        assert!(out.contains("[file] x.txt (+1 -1)"), "got:\n{out}");
        assert!(!out.contains("bullet"), "got:\n{out}");

        // Every diffstat shape format-patch emits still ends the tolerance,
        // so a marked line after one is still lost content. The bare count
        // covers a pure rename or mode change, which stats as `0`.
        for stat in [
            " x.txt                        |   2 +-",
            " added.txt                    |   1 +",
            " img.png                      | Bin 7 -> 12 bytes",
            " img.png                      | Bin",
            " rename_me.txt => renamed.txt |   0",
            " {a => b}/file.txt            |   2 +-",
            " a | b.txt                    |   2 +-",
        ] {
            let lost = base
                .replace("STAT", stat)
                .replace("\ndiff --git a/x.txt b/x.txt\n--- a/x.txt\n+++ b/x.txt\n@@ -1 +1 @@\n", "\n");
            assert!(
                condense_unified_diff_strict(&lost).is_none(),
                "marked line past the diffstat `{stat}` was dropped"
            );
        }
    }

    #[test]
    fn a_bodyless_region_is_judged_by_shape() {
        // `--no-stat` has no diffstat to end rule 8's tolerance, so a region
        // that never reaches a file header is settled at its close. An
        // orphaned hunk body is not bullet-shaped => raw.
        let series = "From aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa Mon Sep 17 00:00:00 2001\nSubject: [PATCH 1/2] one\n\nBODY\n-- \n2.54.0\n\nFrom bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb Mon Sep 17 00:00:00 2001\nSubject: [PATCH 2/2] two\n\ndiff --git a/y.txt b/y.txt\n--- a/y.txt\n+++ b/y.txt\n@@ -1 +1 @@\n-p\n+q\n-- \n2.54.0\n";
        assert!(
            condense_unified_diff_strict(&series.replace("BODY", "-a\n+b")).is_none(),
            "an orphaned no-stat body must fall back raw"
        );
        // Closing at end of stream, not at a separator, judges it the same.
        let last = "From bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb Mon Sep 17 00:00:00 2001\nSubject: [PATCH 1/2] one\n\ndiff --git a/y.txt b/y.txt\n--- a/y.txt\n+++ b/y.txt\n@@ -1 +1 @@\n-p\n+q\n-- \n2.54.0\n\nFrom aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa Mon Sep 17 00:00:00 2001\nSubject: [PATCH 2/2] two\n\n-a\n+b\n-- \n2.54.0\n";
        assert!(
            condense_unified_diff_strict(last).is_none(),
            "an orphaned body at end of stream must fall back raw"
        );

        // Bodyless by construction - a cover letter or an `--always` empty
        // commit - writes bullets, and bullets keep their shape.
        for prose in [
            "- first, it changes a to b\n- second, it changes b to c",
            "Before and after:\n- old_value = 1\n+ new_value = 2",
            "A rule:\n---------\nand a bullet:\n- done",
            // Prose that wrapped a long option to column 0, which is how
            // 11.7% of this repo's marked-line messages break bullet shape.
            "The relevant invocation is\n--no-stat --cover-letter\nwhich changes nothing here.",
        ] {
            let out = condense_unified_diff_strict(&series.replace("BODY", prose))
                .expect("a bodyless region of prose must not force raw");
            assert!(out.contains("[file] y.txt (+1 -1)"), "got:\n{out}");
        }
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
    fn truncated_prologue_marked_lines_fall_back_to_raw() {
        // A stream whose beginning was cut mid-hunk (a `head`-clipped
        // capture, a pipe that lost its first chunk) puts real hunk-body
        // lines before the first file header. They must force raw
        // passthrough, not be dropped as prologue prose while the intact
        // tail condenses into a complete-looking result.
        let diff = "-lost removal\n+lost addition\ndiff --git a/f b/f\n--- a/f\n+++ b/f\n@@ -1 +1 @@\n-x\n+y\n";
        assert!(condense_unified_diff_strict(diff).is_none());
        assert_eq!(condense_unified_diff(diff), diff);
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
        // this corpus that ranges from under 30% (content-heavy) to near 90%
        // (metadata-heavy) per fixture;
        // on content-heavy single-file diffs it can fall to single digits
        // (~4% on this branch's own self-diff). The 20% admission bar in
        // CONTRIBUTING.md is asserted per-fixture on the metadata-heavy
        // streams in the test below; it is not guaranteed by construction
        // on content-heavy input. What must always hold: the output is never
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
    fn metadata_heavy_fixtures_clear_the_admission_bar() {
        // Pins the CONTRIBUTING.md 20% admission bar on the metadata-heavy
        // corpus fixtures so the percentages cited above can't rot into
        // fiction. Content-heavy fidelity fixtures (single-file `diff -u`
        // shapes) are deliberately absent: their savings are input-dependent
        // and only the never-larger property above binds them.
        for name in [
            "git_log_p",
            "git_diff_multifile",
            "git_format_patch_single",
            "git_format_patch_series",
            "git_format_patch_cover",
            "git_format_patch_sha256",
        ] {
            let fixture = CORPUS
                .iter()
                .find(|(n, _)| *n == name)
                .expect("fixture listed in CORPUS")
                .1;
            let out = condense_unified_diff(fixture);
            let (input, output) = (count_tokens(fixture), count_tokens(&out));
            // output <= 80% of input  <=>  savings >= 20%.
            assert!(
                output * 5 <= input * 4,
                "{name}: savings below the 20% admission bar ({input} -> {output} tokens)"
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
