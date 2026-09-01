//! Compares two files and shows only the changed lines.

use crate::core::guard::{self, never_worse};
use crate::core::tracking;
use crate::core::tracking::estimate_tokens;
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
    let both_files = format!("{}\n---\n{}", content1, content2);

    let comparison = compare_files(&content1, &content2);
    let fallback = classic_fallback(&comparison);
    let (rtk, exit_code) = render_diff(file1, file2, &comparison);

    // When `lines()` cannot see the difference, the raw fallback is two blobs
    // that look the same, which is the outcome the rendered message exists to
    // avoid. Fewer bytes does not make it the better answer there — but the
    // message has to earn them: past `INVISIBLE_DIFF_TOKEN_ALLOWANCE` the raw
    // text is short enough to read directly, so the guard keeps its say.
    let affordable = invisible_message_affordable(&rtk, file1, file2, &both_files);
    let shown = select_file_diff_output(&comparison, &fallback, &both_files, &rtk, affordable);
    print!("{}", shown);
    timer.track(
        &format!("diff {} {}", file1.display(), file2.display()),
        "rtk diff",
        tracking_baseline(&fallback, &both_files, shown),
        shown,
    );
    Ok(exit_code)
}

/// What comparing the two files established, before anything is rendered.
///
/// The discriminator matters more than it looks. An empty change list is not a
/// synonym for "identical": it is also what a difference `str::lines()` cannot
/// see produces, and what every refusal to build an over-budget listing
/// produces. Routing any of those through the identical branch reports two
/// different files as the same and exits 0, which is the bug this module exists
/// to close.
enum FileComparison {
    /// Byte-identical.
    Identical,
    /// The bytes differ but `lines()` does not. Carries the description of the
    /// cause, since only the file contents can supply it.
    InvisibleDifference(String),
    /// A line-level comparison ran. Either it produced a change list, or it
    /// refused to build one and said why in `DiffResult::unaligned`.
    Lines(DiffResult),
}

fn compare_files(content1: &str, content2: &str) -> FileComparison {
    // Byte equality is the only safe basis for claiming identity, and it must be
    // checked before `lines()` touches the input. `str::lines()` strips a
    // trailing `\r` and treats the final newline as optional, so a CRLF-vs-LF or
    // missing-trailing-newline difference collapses to identical line vectors.
    // Reporting "identical" with exit 0 for files that differ silently passes
    // any verification gate built on `diff`.
    if content1 == content2 {
        return FileComparison::Identical;
    }

    let lines1: Vec<&str> = content1.lines().collect();
    let lines2: Vec<&str> = content2.lines().collect();
    let diff = compute_diff(&lines1, &lines2);

    if diff.unaligned.is_none() && diff.changes.is_empty() {
        // Bytes differ, lines don't: the difference is exactly what `lines()`
        // normalizes away. Name it rather than rendering an empty change list.
        return FileComparison::InvisibleDifference(describe_invisible_difference(
            content1, content2,
        ));
    }

    FileComparison::Lines(diff)
}

/// What `diff` itself would have printed, or the empty string when there is no
/// such output to compare against.
///
/// A classic diff exists exactly when a change list was built. Identical files
/// make `diff` print nothing, an invisible difference has no lines to list, and
/// a refused listing has no changes to format — for those three the baseline
/// and the guard both fall back to the dump of both files.
fn classic_fallback(comparison: &FileComparison) -> String {
    match comparison {
        FileComparison::Lines(diff) if diff.unaligned.is_none() => format_classic_diff(diff),
        _ => String::new(),
    }
}

/// Baseline the savings are measured against: what `diff` itself would have
/// printed, so the recorded ratio compares like with like and can never go
/// negative -- the guard already caps the shown output at the fallback.
fn tracking_baseline<'a>(fallback: &'a str, both_files: &'a str, shown: &'a str) -> &'a str {
    if !fallback.is_empty() {
        return fallback;
    }

    // No classic diff to measure against, so the dump of both files stands in
    // as the output that would otherwise have to be read. Two near-empty files
    // can make that dump cheaper than the verdict line, which would book a loss
    // against the cheapest possible answer.
    if tracking::estimate_tokens(both_files) >= tracking::estimate_tokens(shown) {
        both_files
    } else {
        shown
    }
}

fn select_file_diff_output<'a>(
    comparison: &FileComparison,
    fallback: &'a str,
    both_files: &'a str,
    rendered: &'a str,
    invisible_affordable: bool,
) -> &'a str {
    match comparison {
        // `diff` prints nothing here, so there is no raw output to be worse
        // than: the verdict line is the whole answer.
        FileComparison::Identical => rendered,
        FileComparison::InvisibleDifference(_) => {
            if invisible_affordable {
                rendered
            } else {
                never_worse(both_files, rendered)
            }
        }
        // No change list means no classic diff, so the refusal is measured
        // against the dump it is refusing to replace.
        FileComparison::Lines(diff) if diff.unaligned.is_some() => {
            never_worse(both_files, rendered)
        }
        FileComparison::Lines(_) => never_worse(fallback, rendered),
    }
}

/// Whether the invisible-difference message may be shown above the raw
/// fallback's token count.
///
/// Measured on what the message states, with the `file1 -> file2` header
/// excluded: the caller typed those paths, so their length says nothing about
/// whether the diagnostic is worth its tokens. Counting them made an absolute
/// path pair eat the whole allowance and drop the message on exactly the
/// invocations that most need it.
fn invisible_message_affordable(rtk: &str, file1: &Path, file2: &Path, raw: &str) -> bool {
    let header = file_pair_header(file1, file2);
    let stated = rtk.strip_prefix(&header).unwrap_or(rtk);
    estimate_tokens(stated) <= estimate_tokens(raw) + guard::INVISIBLE_DIFF_TOKEN_ALLOWANCE
}

/// The `file1 -> file2` line every non-identical render opens with.
fn file_pair_header(file1: &Path, file2: &Path) -> String {
    format!("{} \u{2192} {}\n", file1.display(), file2.display())
}

/// Renders the condensed file comparison and returns it with the
/// diff-convention exit code (0 = identical, 1 = differences found).
fn render_diff(file1: &Path, file2: &Path, comparison: &FileComparison) -> (String, i32) {
    let diff = match comparison {
        FileComparison::Identical => return ("[ok] Files are identical\n".to_string(), 0),
        FileComparison::InvisibleDifference(cause) => {
            return (
                format!("{}   {}\n", file_pair_header(file1, file2), cause),
                1,
            );
        }
        FileComparison::Lines(diff) => diff,
    };

    match &diff.unaligned {
        Some(Unaligned::DifferingLines(n)) => {
            return (
                format!(
                    "{}   {} lines differ, too many to list; use `rtk proxy diff` for the full text\n",
                    file_pair_header(file1, file2),
                    n
                ),
                1,
            );
        }
        Some(Unaligned::RegionBounds {
            differing_floor,
            first,
            last1,
            last2,
        }) => {
            // The floor is measured, not derived from the constant: every script
            // shorter than the round the aligner gave up at was tried and
            // failed, and an in-place rewrite is two operations. Where the
            // differences sit is stated as line bounds in each file, because the
            // size of that region is not a count of anything and a figure shaped
            // like one invites reading it as the amount of change.
            return (
                format!(
                    "{}   at least {} lines differ, too different to align line by line\n   differences fall between lines {}-{} of {} and {}-{} of {}; use `rtk proxy diff` for the full text\n",
                    file_pair_header(file1, file2),
                    differing_floor,
                    first,
                    last1,
                    file1.display(),
                    first,
                    last2,
                    file2.display()
                ),
                1,
            );
        }
        Some(Unaligned::EditScript { removed, added }) => {
            return (
                format!(
                    "{}   -{} lines only in {}, +{} only in {}; too many to list, use `rtk proxy diff` for the full text\n",
                    file_pair_header(file1, file2),
                    removed,
                    file1.display(),
                    added,
                    file2.display()
                ),
                1,
            );
        }
        None => {}
    }

    let mut rtk = String::new();
    rtk.push_str(&file_pair_header(file1, file2));
    rtk.push_str(&format!(
        "   +{} added, -{} removed, ~{} modified\n\n",
        diff.added, diff.removed, diff.modified
    ));
    if diff.positional {
        // The pairing is by line position, not by alignment. Saying so is the
        // difference between a non-minimal diff and a misleading one.
        rtk.push_str("   paired by line position: too different to align\n");
    }
    if let Some(legend) = frame_legend(diff, file1, file2) {
        rtk.push_str(&legend);
    }
    rtk.push_str(&format_diff_changes(diff));
    (rtk, 1)
}

/// The note explaining which file each marker is numbered in, or `None` when
/// the output has only one frame and needs no note.
///
/// Every line is numbered in the file it comes from: `-` and `~` in file1, `+`
/// in file2. The note is owed whenever the output mixes frames, which is any
/// time a `+` sits beside a `-` or a `~` — an insertion above a modification
/// shifts the numbering just as much as a replacement pair does. Output drawn
/// from one file only (`+` alone, `-` alone, `~` alone) has one frame.
///
/// It names the markers actually on screen rather than a fixed `-` and `+`.
/// The note exists solely to stop a line-number misread, so one that describes
/// output the reader is not looking at — announcing a `-` frame with no lines
/// in it while saying nothing about the `~` lines that are there — is worse
/// than none.
///
/// The positional fallback numbers both halves of a pair from the same
/// position, so there is one frame there and the note would misdescribe it as
/// two.
fn frame_legend(diff: &DiffResult, file1: &Path, file2: &Path) -> Option<String> {
    if diff.positional || diff.added == 0 || (diff.removed == 0 && diff.modified == 0) {
        return None;
    }
    let mut file1_markers = Vec::new();
    if diff.removed > 0 {
        file1_markers.push("-");
    }
    if diff.modified > 0 {
        file1_markers.push("~");
    }
    Some(format!(
        "   ({} numbered in {}, + in {})\n",
        file1_markers.join(" and "),
        file1.display(),
        file2.display()
    ))
}

/// 1-based numbers of the lines that `content` terminates with CRLF.
///
/// Positions, not a count: two files can hold the same number of CRLF
/// terminators at different lines, and a count alone cannot tell them apart.
fn crlf_line_numbers(content: &str) -> Vec<usize> {
    // Only the newline-terminated part has line terminators to classify.
    // `split` hands back an unterminated tail as its own segment, so a file
    // ending in a bare `\r` would otherwise count a CRLF that isn't there.
    let terminated = match content.rfind('\n') {
        Some(i) => &content[..=i],
        None => "",
    };
    terminated
        .split('\n')
        .enumerate()
        .filter(|(_, segment)| segment.ends_with('\r'))
        .map(|(i, _)| i + 1)
        .collect()
}

/// Describe a difference that a line-based diff cannot see.
///
/// Reached only when the bytes differ but `lines()` yields identical vectors,
/// which narrows the cause to the two things `lines()` normalizes: a `\r`
/// before the newline, and the presence of the final newline. Rendering the
/// usual `~ 12 foo → foo` change list here would show two visually identical
/// strings, so state the cause instead.
///
/// Returns the cause alone; the caller supplies the `file1 -> file2` header.
fn describe_invisible_difference(content1: &str, content2: &str) -> String {
    let crlf1 = crlf_line_numbers(content1);
    let crlf2 = crlf_line_numbers(content2);
    let nl1 = content1.ends_with('\n');
    let nl2 = content2.ends_with('\n');

    let mut notes: Vec<String> = Vec::new();
    if crlf1.len() != crlf2.len() {
        notes.push(format!(
            "line endings: {} CRLF vs {} CRLF",
            crlf1.len(),
            crlf2.len()
        ));
    } else if crlf1 != crlf2 {
        // Equal counts, different placement. Printing the counts alone would
        // show the same number on both sides, which reads as "no difference".
        let line = crlf1
            .iter()
            .zip(&crlf2)
            .find(|(l1, l2)| l1 != l2)
            .map(|(l1, l2)| (*l1).min(*l2))
            .unwrap_or(0);
        notes.push(format!(
            "line endings: {} CRLF on each side, first differing at line {}",
            crlf1.len(),
            line
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
        // Defensive: `lines()` normalizes only the `\r` before a newline and
        // the final newline, so one of the checks above should have fired.
        notes.push("cause outside the line-ending and trailing-newline checks".to_string());
    }

    // Deliberately avoids the word "identical". That string is the signal for
    // the true-identity case, and a reader (or grep) scanning for it must not
    // match a report about files that differ.
    format!("differs, text matches ({})", notes.join("; "))
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
    /// A line only in file2. `after1` is the last file1 line before the
    /// insertion point, 0 when it sits at the top of the file.
    Added {
        after1: usize,
        line2: usize,
        text: String,
    },
    /// A line only in file1. `after2` is the last file2 line before the
    /// deletion point, 0 when it sits at the top of the file.
    Removed {
        line1: usize,
        after2: usize,
        text: String,
    },
    /// A line rewritten in place, similar enough to show as a single `~` line.
    Modified {
        line1: usize,
        line2: usize,
        old: String,
        new: String,
    },
    /// A line rewritten in place, too dissimilar to read as one line: shown as
    /// a `-` and a `+`, but one edit.
    ///
    /// It stays one change rather than becoming a `Removed` plus an `Added`
    /// because the classic renderer has to group it as a single `NcM`, and the
    /// only other way to recover that pairing is to infer it from equal line
    /// numbers — which stops holding the moment an insertion shifts the two
    /// files' numbering apart.
    Replaced {
        line1: usize,
        line2: usize,
        old: String,
        new: String,
    },
}

struct DiffResult {
    added: usize,
    removed: usize,
    modified: usize,
    changes: Vec<DiffChange>,
    /// Set when no change list was produced, either because the pair could not
    /// be aligned or because the list would be too large to be worth building.
    /// `changes` is empty by design and the counts are zero because none were
    /// computed.
    unaligned: Option<Unaligned>,
    /// Set when the changes were paired by line position rather than aligned,
    /// which happens past `MAX_TRACE_CELLS` on equal-length inputs.
    positional: bool,
}

fn format_diff_changes(diff: &DiffResult) -> String {
    let mut out = String::new();
    for change in &diff.changes {
        match change {
            DiffChange::Added { line2, text, .. } => {
                out.push_str(&format!("+{:4} {}\n", line2, text))
            }
            DiffChange::Removed { line1, text, .. } => {
                out.push_str(&format!("-{:4} {}\n", line1, text))
            }
            DiffChange::Modified {
                line1, old, new, ..
            } => out.push_str(&format!("~{:4} {} → {}\n", line1, old, new)),
            DiffChange::Replaced {
                line1,
                line2,
                old,
                new,
            } => {
                out.push_str(&format!("-{:4} {}\n", line1, old));
                out.push_str(&format!("+{:4} {}\n", line2, new));
            }
        }
    }
    out
}

/// What `diff` itself prints for the same comparison: `NcM`, `NaM` and `NdM`
/// hunks with `<` and `>` bodies.
///
/// Every hunk header names a position in each file, and after an insertion the
/// two files' numbering no longer agrees, so each change carries both. Deriving
/// one frame from the other — the shape a positional comparison made look
/// workable — silently mislabels every hunk past the first shift.
fn format_classic_diff(diff: &DiffResult) -> String {
    let mut out = String::new();
    let mut index = 0;

    while index < diff.changes.len() {
        match &diff.changes[index] {
            // A run of rewritten lines, however each one is displayed: `~` and
            // `-`/`+` differ only in how similar the two texts are, and classic
            // diff groups both as a change.
            DiffChange::Modified { line1, line2, .. }
            | DiffChange::Replaced { line1, line2, .. } => {
                let (start1, start2) = (*line1, *line2);
                let (mut end1, mut end2) = (start1, start2);
                let mut old_lines = Vec::new();
                let mut new_lines = Vec::new();

                while let Some(
                    DiffChange::Modified {
                        line1,
                        line2,
                        old,
                        new,
                    }
                    | DiffChange::Replaced {
                        line1,
                        line2,
                        old,
                        new,
                    },
                ) = diff.changes.get(index)
                {
                    if *line1 != end1 || *line2 != end2 {
                        break;
                    }
                    old_lines.push(old);
                    new_lines.push(new);
                    end1 += 1;
                    end2 += 1;
                    index += 1;
                }

                out.push_str(&format!(
                    "{}c{}\n",
                    format_line_range(start1, end1 - 1),
                    format_line_range(start2, end2 - 1)
                ));
                for line in old_lines {
                    out.push_str(&format!("< {}\n", line));
                }
                out.push_str("---\n");
                for line in new_lines {
                    out.push_str(&format!("> {}\n", line));
                }
            }
            DiffChange::Added { after1, line2, .. } => {
                let (anchor1, start2) = (*after1, *line2);
                let mut end2 = start2;
                let mut new_lines = Vec::new();

                while let Some(DiffChange::Added {
                    after1,
                    line2,
                    text,
                }) = diff.changes.get(index)
                {
                    if *after1 != anchor1 || *line2 != end2 {
                        break;
                    }
                    new_lines.push(text);
                    end2 += 1;
                    index += 1;
                }

                out.push_str(&format!(
                    "{}a{}\n",
                    anchor1,
                    format_line_range(start2, end2 - 1)
                ));
                for line in new_lines {
                    out.push_str(&format!("> {}\n", line));
                }
            }
            DiffChange::Removed { line1, after2, .. } => {
                let (start1, anchor2) = (*line1, *after2);
                let mut end1 = start1;
                let mut old_lines = Vec::new();

                while let Some(DiffChange::Removed {
                    line1,
                    after2,
                    text,
                }) = diff.changes.get(index)
                {
                    if *line1 != end1 || *after2 != anchor2 {
                        break;
                    }
                    old_lines.push(text);
                    end1 += 1;
                    index += 1;
                }

                out.push_str(&format!(
                    "{}d{}\n",
                    format_line_range(start1, end1 - 1),
                    anchor2
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

/// Why a pair produced no change list.
#[derive(Debug, PartialEq, Eq)]
enum Unaligned {
    /// The differing lines could be counted exactly — one side is empty, or the
    /// lengths are equal — and there are too many of them to be worth listing.
    DifferingLines(usize),
    /// Unequal lengths past the trace budget. Nothing was counted, so the only
    /// figures stated are a floor and where the differences are.
    ///
    /// `differing_floor` comes from the round the aligner gave up at: every
    /// shorter script failed, so at least that many lines differ. `first` is the
    /// first differing line in both files, `last1` and `last2` the last in each.
    ///
    /// Line bounds, not a magnitude: the region between the first and last
    /// difference is unrelated to how much changed inside it, so a number
    /// shaped like a count would swing by orders of magnitude on the same
    /// amount of change.
    RegionBounds {
        differing_floor: usize,
        first: usize,
        last1: usize,
        last2: usize,
    },
    /// The pair aligned, but the change list it implies is too long or too
    /// large to build. Both sides are stated because the script knows them
    /// exactly and a single figure would not: `removed` lines of file1 have no
    /// counterpart in file2, and `added` lines of file2 none in file1.
    ///
    /// Not the number of listed lines: the runs have not been paired yet, and a
    /// pairing turns two script steps into either one `~` or one `-` plus one
    /// `+`. `removed + added` bounds that count from above, which is what the
    /// budget needs.
    EditScript { removed: usize, added: usize },
}

/// One edit-script step.
///
/// `Del` and `Ins` carry a position in each file: the 1-based number of the
/// line they name, and the last line consumed on the other side at that point.
/// Both are needed because the classic renderer anchors an append in file1 and
/// a delete in file2, and neither anchor can be derived from the other once an
/// insertion has shifted the two files' numbering apart.
///
/// `Keep` marks a matched line, which is what stops a deletion run and an
/// insertion run on opposite sides of it from folding together.
enum Op {
    Del {
        line1: usize,
        after2: usize,
        text: String,
    },
    Ins {
        after1: usize,
        line2: usize,
        text: String,
    },
    Keep,
}

/// Cap on the aligner's trace, counted in `i32` cells.
///
/// The trace is the only part of the alignment that grows: round `d` records
/// the furthest-reaching `x` on the diagonals that could still be on an optimal
/// path, and the backtrack walks those records.
///
/// Counting cells rather than the edit distance is what lets a lopsided pair
/// through. A one-line file against a five-thousand-line one needs `d = 4999`,
/// but at most one deletion is possible, so each round's window is a few
/// diagonals wide and the whole trace is ~15,000 cells. An edit-distance cap
/// refused that pair and reported an alignment it could have produced in one
/// pass as too different to align.
///
/// A pair whose lengths are close gets the full window, which is where the
/// budget is spent: each round costs `d + 2` cells, so the trace is quadratic
/// in the amount of change and 1,000,000 cells stop the search at `d = 1414`.
/// That is ~707 scattered rewritten lines, the worst case for the window;
/// contiguous change narrows it and reaches further. Past the budget equal
/// lengths fall back to a positional comparison, which cannot run out of it,
/// and unequal ones report bounds.
///
/// ~707 changed lines is the number this constant is really choosing, and it is
/// chosen against `POSITIONAL_CHANGE_CAP` rather than in isolation: an aligner
/// that refuses an order of magnitude below what the listing path prints is a
/// cliff, not a budget. A rewritten line is one listed line and a replaced one
/// is two, so the aligner tops out within a factor of ~3 of the 5,000-line
/// listing cap. Storing only the diagonals the backtrack reads — every other
/// slot, since a round computes one parity and the backtrack reads the other —
/// is what buys half of that reach at no cost in memory.
///
/// The remaining headroom is bounded by RTK's own budgets rather than by the
/// algorithm. 1,000,000 `i32` cells is 4MB, measured at +3.4MB of peak RSS on
/// the pair that fills them, and that search costs ~7ms of user time against
/// the 10ms in CLAUDE.md. Both are spent only by a pair that actually changed
/// that much; a typical diff never allocates a second round. Doubling the
/// constant doubles both and buys 1.41x the reach.
///
/// The cap is on the amount of change, never on the size of the files or on how
/// far apart the edits sit: two lines changed 2000 lines apart cost `d = 4`,
/// whatever the file's length.
const MAX_TRACE_CELLS: usize = 1_000_000;

/// Diff two line sequences.
///
/// The original implementation compared **positionally** (`lines1[i]` vs
/// `lines2[i]` for i in 0..max_len), so a single inserted line desynchronized
/// every line after it: each subsequent pair compared unrelated lines, the whole
/// tail rendered as changed, and the output grew large enough that the
/// `never_worse` guard discarded it and dumped both files concatenated instead
/// of showing one insertion.
fn compute_diff(lines1: &[&str], lines2: &[&str]) -> DiffResult {
    // Common prefix and suffix carry no information and dominate real diffs.
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

    // A pure insertion or deletion needs no search: after trimming, one side is
    // empty and the script is a single run. Myers would spend the whole trace
    // budget reaching `(n, m)` here, so without this an appended chunk — or an
    // empty file against a populated one — reports as too different to align.
    if a.is_empty() || b.is_empty() {
        if let Some(refused) = too_much_to_list(a.iter().chain(b.iter()).map(|l| l.len())) {
            return refused;
        }
        return ops_to_changes(one_sided_ops(a, b, lo));
    }

    let gave_up_at = match myers_ops(a, b, lo) {
        Ok(Aligned::Script(ops)) => {
            if let Some(refused) = script_too_large(&ops) {
                return refused;
            }
            return ops_to_changes(ops);
        }
        Ok(Aligned::TooManySteps { removed, added }) => {
            return unaligned(Unaligned::EditScript { removed, added });
        }
        Err(d) => d,
    };

    if a.len() == b.len() {
        // Too much change for an alignment, but equal lengths make pairing by
        // position a valid edit script: it reconstructs file2 and every line
        // number names the text it claims. It is not minimal — a deletion at
        // the top and an insertion at the bottom keep the lengths equal and
        // report every line between them as rewritten — so the render says the
        // pairing is positional rather than presenting it as an alignment.
        //
        // Counting first costs one pass and no allocation, which is what keeps
        // two wholly different 100,000-line files from building a 200,000-line
        // change list nobody asked for.
        let differing = a.iter().zip(b.iter()).filter(|(x, y)| x != y);
        if let Some(refused) = too_much_to_list(differing.map(|(x, y)| x.len() + y.len())) {
            return refused;
        }
        return positional_changes(a, b, lo);
    }

    unaligned(Unaligned::RegionBounds {
        differing_floor: gave_up_at.div_ceil(2),
        first: lo + 1,
        last1: hi1,
        last2: hi2,
    })
}

/// A result carrying no change list, only the reason there is none. The counts
/// are zero because none were computed, not because nothing changed.
fn unaligned(reason: Unaligned) -> DiffResult {
    DiffResult {
        added: 0,
        removed: 0,
        modified: 0,
        changes: Vec::new(),
        unaligned: Some(reason),
        positional: false,
    }
}

/// Refuse to build a change list that is too long or too large, naming the
/// exact number of differing lines instead.
///
/// `sizes` yields one entry per differing position the list would report,
/// holding the bytes that position contributes — both halves of a replacement,
/// since one carries the old text and the new. Both callers know the count
/// exactly — one side is empty, or the two sides are the same length — so the
/// refusal states a measured number rather than a bound. The third listing
/// path, an aligned edit script, is guarded by `script_too_large`.
fn too_much_to_list(sizes: impl Iterator<Item = usize>) -> Option<DiffResult> {
    let mut count = 0usize;
    let mut bytes = 0usize;
    for size in sizes {
        count += 1;
        bytes += size;
    }
    if !over_listing_budget(count, bytes) {
        return None;
    }
    Some(unaligned(Unaligned::DifferingLines(count)))
}

/// Whether a change list naming `count` differing positions and holding `bytes`
/// bytes of text is too large to be worth building.
fn over_listing_budget(count: usize, bytes: usize) -> bool {
    count > POSITIONAL_CHANGE_CAP || bytes > POSITIONAL_BYTE_CAP
}

/// Refuse an aligned edit script whose change list would hold too many bytes,
/// stating the two counts the script knows exactly instead.
///
/// The third listing path, and the one the band made reachable. While the cap
/// was on the edit distance, a large script meant the aligner had already given
/// up; a banded window keeps the trace cheap while `d` grows, so a one-line file
/// against a 60,000-line one now aligns and would build 59,999 changes — 11x
/// `POSITIONAL_CHANGE_CAP` — from an input the empty-file case refuses at the
/// same size for free.
///
/// The count half of the budget is spent before the script exists: `myers_ops`
/// knows the edit distance the moment it reaches the end, and refuses there
/// rather than materialising a script it is about to throw away. What is left
/// here is the byte half, which genuinely needs the text. `ops` is already
/// built, so this is a pass over what is in hand, and the bytes are exact: each
/// step's text is cloned into a `DiffChange` and again into the render.
fn script_too_large(ops: &[Op]) -> Option<DiffResult> {
    let (mut removed, mut added, mut bytes) = (0usize, 0usize, 0usize);
    for op in ops {
        match op {
            Op::Del { text, .. } => {
                removed += 1;
                bytes += text.len();
            }
            Op::Ins { text, .. } => {
                added += 1;
                bytes += text.len();
            }
            Op::Keep => {}
        }
    }
    if !over_listing_budget(removed + added, bytes) {
        return None;
    }
    Some(unaligned(Unaligned::EditScript { removed, added }))
}

/// The edit script for a pair where one side is empty: every line of the other
/// side, in order. `offset` maps middle-relative indices back to file line
/// numbers after prefix trimming, and is also the other side's cursor — the
/// empty middle consumes nothing, so the whole run sits after the trimmed
/// prefix.
fn one_sided_ops(a: &[&str], b: &[&str], offset: usize) -> Vec<Op> {
    a.iter()
        .enumerate()
        .map(|(i, line)| Op::Del {
            line1: offset + i + 1,
            after2: offset,
            text: (*line).to_string(),
        })
        .chain(b.iter().enumerate().map(|(i, line)| Op::Ins {
            after1: offset,
            line2: offset + i + 1,
            text: (*line).to_string(),
        }))
        .collect()
}

/// Cap on the differing positions either listing path will name.
///
/// Not a cap on rendered lines: a position whose two texts are too dissimilar
/// to read as one `~` prints a `-` and a `+`, so 5,000 positions can render as
/// up to 10,000 lines. `POSITIONAL_BYTE_CAP` is what bounds the memory; this
/// one bounds how much a reader is asked to scan. Past it the pair is not a
/// diff anyone reads line by line, and the exact count says more than the list
/// would.
const POSITIONAL_CHANGE_CAP: usize = 5_000;

/// Cap on the bytes those positions may hold.
///
/// A count says nothing about line length, and each listed line is cloned into
/// an `Op` and again into a `DiffChange` before being formatted. Five thousand
/// ten-thousand-character lines are inside the count cap and cost hundreds of
/// megabytes, so the byte budget is what actually bounds the listing.
const POSITIONAL_BYTE_CAP: usize = 2_000_000;

/// Pair line `i` of each side. Only valid when the two sides are the same
/// length, which is what makes every pair a replacement rather than a shift.
fn positional_changes(a: &[&str], b: &[&str], offset: usize) -> DiffResult {
    let mut changes = Vec::new();
    let (mut added, mut removed, mut modified) = (0usize, 0usize, 0usize);

    for (i, (old, new)) in a.iter().zip(b.iter()).enumerate() {
        if old == new {
            continue;
        }
        let line = offset + i + 1;
        if similarity(old, new) > 0.5 {
            changes.push(DiffChange::Modified {
                line1: line,
                line2: line,
                old: (*old).to_string(),
                new: (*new).to_string(),
            });
            modified += 1;
        } else {
            changes.push(DiffChange::Replaced {
                line1: line,
                line2: line,
                old: (*old).to_string(),
                new: (*new).to_string(),
            });
            removed += 1;
            added += 1;
        }
    }

    DiffResult {
        added,
        removed,
        modified,
        changes,
        unaligned: None,
        positional: true,
    }
}

/// What an alignment produced.
enum Aligned {
    /// The edit script, in forward order.
    Script(Vec<Op>),
    /// The pair aligned, but the script has more steps than the listing budget
    /// admits. Both counts are exact without building it: the script has `d`
    /// steps and deletions minus insertions is the length difference.
    TooManySteps { removed: usize, added: usize },
}

/// Myers' greedy edit script: `O((n + m) * d)` in the edit distance `d`, so the
/// cost tracks how much changed rather than how large the files are.
///
/// `offset` maps middle-relative indices back to real file line numbers after
/// prefix trimming. `Err(d)` when the trace would exceed `MAX_TRACE_CELLS`,
/// carrying the round it gave up at: rounds below it all failed, so the edit
/// distance is at least `d`.
///
/// Cost grows with `d`, but each round still scans the band and runs snakes
/// across the input, so wall time grows with the file length at a fixed `d`.
/// The bound this function keeps is on the trace, not on the total work.
fn myers_ops(a: &[&str], b: &[&str], offset: usize) -> Result<Aligned, usize> {
    let n = a.len() as i32;
    let m = b.len() as i32;
    // Deleting all of `a` and inserting all of `b` always reaches the target,
    // so the search never needs a longer script than this.
    let max_d = a.len() + b.len();

    // `v[k + vo]` is the furthest `x` reached on diagonal `k = x - y`. `k` runs
    // over `[-m, n]`, and the extra slot on each side absorbs the `k +/- 1`
    // reads at the window edges.
    let vo = m + 1;
    let mut v = vec![0i32; (n + m + 3) as usize];
    // Round `d` records `v` over the diagonals it is about to extend, plus one
    // on each side for what the extension reads, before it runs. That window is
    // what the backtrack walks.
    let mut trace: Vec<(i32, Vec<i32>)> = Vec::new();
    let mut cells = 0usize;

    for d in 0..=max_d {
        let di = d as i32;
        // Only the diagonals that could still be on an optimal path. Reaching
        // `k` at round `d` costs `(d + k) / 2` deletions and `(d - k) / 2`
        // insertions, and neither can exceed the side it consumes. On a lopsided
        // pair that window stays a few diagonals wide however large `d` grows,
        // which is what keeps the trace affordable where an edit-distance cap
        // gave up on a trivial alignment.
        let lo = (-di).max(di - 2 * m);
        let hi = di.min(2 * n - di);
        if hi < lo {
            break;
        }

        // Round `d` computes only the diagonals with `d`'s parity, and the
        // backtrack reads this snapshot only at the opposite parity — the
        // values round `d - 1` left behind, which is what a step back from
        // `k` lands on. Storing every other slot is not a sampling: the ones
        // skipped are the ones round `d` is about to overwrite and nothing
        // ever reads from here.
        let slots = ((hi - lo) / 2 + 2) as usize;
        if cells + slots > MAX_TRACE_CELLS {
            return Err(d);
        }
        cells += slots;

        let mut snapshot = Vec::with_capacity(slots);
        let mut ks = lo - 1;
        while ks <= hi + 1 {
            snapshot.push(v[(ks + vo) as usize]);
            ks += 2;
        }
        trace.push((lo - 1, snapshot));

        let mut k = lo;
        while k <= hi {
            let ki = (k + vo) as usize;
            let mut x = if k == -di || (k != di && v[ki - 1] < v[ki + 1]) {
                v[ki + 1]
            } else {
                v[ki - 1] + 1
            };
            let mut y = x - k;
            while x < n && y < m && a[x as usize] == b[y as usize] {
                x += 1;
                y += 1;
            }
            v[ki] = x;
            if x >= n && y >= m {
                // The script has `d` steps, and deletions minus insertions is
                // `n - m`, so both counts are known before a single line is
                // cloned. Refusing here is what keeps a one-line file against
                // 40,000 long ones — a shape the band aligns comfortably — from
                // materialising a script only to throw it away.
                let deletions = ((di + n - m) / 2) as usize;
                let insertions = ((di - n + m) / 2) as usize;
                if deletions + insertions > POSITIONAL_CHANGE_CAP {
                    return Ok(Aligned::TooManySteps {
                        removed: deletions,
                        added: insertions,
                    });
                }
                return Ok(Aligned::Script(myers_backtrack(&trace, a, b, offset)));
            }
            k += 2;
        }
    }

    Err(max_d)
}

/// Walk the trace back from `(n, m)` to the origin, emitting the edit script in
/// forward order.
fn myers_backtrack(trace: &[(i32, Vec<i32>)], a: &[&str], b: &[&str], offset: usize) -> Vec<Op> {
    let mut ops: Vec<Op> = Vec::new();
    let mut x = a.len() as i32;
    let mut y = b.len() as i32;

    for d in (0..trace.len()).rev() {
        let di = d as i32;
        let (base, v) = &trace[d];
        let at = |k: i32| -> i32 {
            // `base` is `lo - 1`, and the snapshot steps by two from there, so
            // only diagonals of `base`'s parity are held. Every read below is
            // at that parity; anything else is off the band, where the
            // furthest-reaching `x` is still the initial 0.
            let i = k - base;
            if i < 0 || i % 2 != 0 {
                return 0;
            }
            let i = (i / 2) as usize;
            if i >= v.len() { 0 } else { v[i] }
        };

        let k = x - y;
        let prev_k = if k == -di || (k != di && at(k - 1) < at(k + 1)) {
            k + 1
        } else {
            k - 1
        };
        let prev_x = at(prev_k);
        let prev_y = prev_x - prev_k;

        while x > prev_x && y > prev_y {
            ops.push(Op::Keep);
            x -= 1;
            y -= 1;
        }
        if d > 0 {
            // A step consumes one side only, so the other side's cursor at that
            // point is the anchor the classic renderer needs: `x` file1 lines
            // stand before an insertion, `y` file2 lines before a deletion.
            if x == prev_x {
                ops.push(Op::Ins {
                    after1: offset + x as usize,
                    line2: offset + y as usize,
                    text: b[(y - 1) as usize].to_string(),
                });
            } else {
                ops.push(Op::Del {
                    line1: offset + x as usize,
                    after2: offset + y as usize,
                    text: a[(x - 1) as usize].to_string(),
                });
            }
        }
        x = prev_x;
        y = prev_y;
    }

    ops.reverse();
    ops
}

/// Fold the edit script into the reported change list, pairing each run of
/// deletions with the insertion run that directly follows it so a rewritten
/// line still reads as one `Modified` rather than a remove plus an unrelated
/// add.
///
/// "Directly follows" is a claim about the files, not about the vector.
/// `Op::Keep` marks every matched line, so a deletion at line 1 and an
/// insertion at line 41 stay separated by the 39 matched lines between them
/// and cannot pair into a change that neither file contains.
fn ops_to_changes(ops: Vec<Op>) -> DiffResult {
    let mut changes = Vec::new();
    let (mut added, mut removed, mut modified) = (0usize, 0usize, 0usize);

    let mut k = 0usize;
    while k < ops.len() {
        if matches!(ops[k], Op::Keep) {
            k += 1;
            continue;
        }
        let del_start = k;
        while matches!(ops.get(k), Some(Op::Del { .. })) {
            k += 1;
        }
        let dels = &ops[del_start..k];

        let ins_start = k;
        while matches!(ops.get(k), Some(Op::Ins { .. })) {
            k += 1;
        }
        let inss = &ops[ins_start..k];

        let pairs = dels.len().min(inss.len());
        for p in 0..pairs {
            let (dline, dtext) = match &dels[p] {
                Op::Del { line1, text, .. } => (*line1, text.as_str()),
                _ => unreachable!("del run holds only deletions"),
            };
            let (iline, itext) = match &inss[p] {
                Op::Ins { line2, text, .. } => (*line2, text.as_str()),
                _ => unreachable!("ins run holds only insertions"),
            };
            // `dline` indexes file1 and `iline` file2, and after any earlier
            // insertion they differ. Both are carried so neither has to be
            // guessed from the other downstream.
            if similarity(dtext, itext) > 0.5 {
                changes.push(DiffChange::Modified {
                    line1: dline,
                    line2: iline,
                    old: dtext.to_string(),
                    new: itext.to_string(),
                });
                modified += 1;
            } else {
                changes.push(DiffChange::Replaced {
                    line1: dline,
                    line2: iline,
                    old: dtext.to_string(),
                    new: itext.to_string(),
                });
                removed += 1;
                added += 1;
            }
        }
        for d in dels.iter().skip(pairs) {
            if let Op::Del { line1, after2, text } = d {
                // The paired insertions were consumed ahead of this deletion,
                // so file2 has advanced past the cursor the script recorded.
                changes.push(DiffChange::Removed {
                    line1: *line1,
                    after2: *after2 + pairs,
                    text: text.clone(),
                });
                removed += 1;
            }
        }
        for ins in inss.iter().skip(pairs) {
            if let Op::Ins {
                after1,
                line2,
                text,
            } = ins
            {
                changes.push(DiffChange::Added {
                    after1: *after1,
                    line2: *line2,
                    text: text.clone(),
                });
                added += 1;
            }
        }
    }

    DiffResult {
        added,
        removed,
        modified,
        changes,
        unaligned: None,
        positional: false,
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

    /// Compare two file contents and render the result, which is the path
    /// `run` takes minus the guard and the tracking.
    fn render_file_diff(
        file1: &Path,
        file2: &Path,
        content1: &str,
        content2: &str,
    ) -> (String, i32) {
        render_diff(file1, file2, &compare_files(content1, content2))
    }

    /// The change list `compare_files` produced, or a panic naming what it
    /// produced instead.
    fn changes_of(content1: &str, content2: &str) -> DiffResult {
        match compare_files(content1, content2) {
            FileComparison::Lines(diff) => diff,
            FileComparison::Identical => panic!("expected a change list, files were identical"),
            FileComparison::InvisibleDifference(cause) => {
                panic!("expected a change list, got an invisible difference: {cause}")
            }
        }
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
            DiffChange::Added { line2, text, .. } => {
                assert_eq!(text, "INSERTED");
                assert_eq!(*line2, 1);
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
            DiffChange::Removed { line1, text, .. } => {
                assert_eq!(text, "GONE");
                assert_eq!(*line1, 3, "line number is the old file's");
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
            .any(|c| matches!(c, DiffChange::Modified { line1: 3, old, new, .. }
                if old == "let x = 1;" && new == "let x = 2;")));
    }

    #[test]
    fn test_compute_diff_adjacent_replacement_pairs_by_similarity() {
        // Two lines replaced in place. First pair is similar -> Modified;
        // second is not -> Removed + Added.
        let a = vec!["let x = 1;", "aaaa"];
        let b = vec!["let x = 2;", "zzzz"];
        let result = compute_diff(&a, &b);
        assert_eq!(result.modified, 1);
        assert_eq!(result.added, 1);
        assert_eq!(result.removed, 1);
        assert!(result.unaligned.is_none());
    }

    #[test]
    fn test_compute_diff_does_not_pair_across_matched_lines() {
        // A deletion at line 1 and an insertion at line 41 are adjacent in the
        // ops vector, because matched lines used to emit nothing. `Op::Keep`
        // keeps the 40 shared lines between them, so they stay unpaired
        // instead of being reported as one modification with counts zeroed.
        let shared: Vec<String> = (0..40).map(|i| format!("shared {}", i)).collect();
        let mut a = vec!["delete_me_x"];
        a.extend(shared.iter().map(|s| s.as_str()));
        let mut b: Vec<&str> = shared.iter().map(|s| s.as_str()).collect();
        b.push("delete_me_y");

        let result = compute_diff(&a, &b);
        assert_eq!(result.modified, 0, "unrelated lines must not pair");
        assert_eq!(result.removed, 1);
        assert_eq!(result.added, 1);
    }

    #[test]
    fn test_compute_diff_reorder_is_not_a_line_modified_into_itself() {
        // Sorting an import block: `use zeta::A;` moves, it is not rewritten.
        let a = vec!["use zeta::A;", "use beta::B;", "use gamma::C;"];
        let b = vec!["use beta::B;", "use gamma::C;", "use zeta::A;"];
        let result = compute_diff(&a, &b);

        assert_eq!(result.modified, 0);
        assert_eq!(result.removed, 1);
        assert_eq!(result.added, 1);
        for change in &result.changes {
            if let DiffChange::Modified { old, new, .. } = change {
                panic!("reported {:?} as modified into {:?}", old, new);
            }
        }
    }

    #[test]
    fn test_compute_diff_added_line_numbers_come_from_file2() {
        // The `Added` half of an unpaired replacement used file1's line
        // number, silently mixing two numbering conventions in one output.
        let a = vec!["x", "AAAA"];
        let b = vec!["NEW", "x", "BBBB"];
        let result = compute_diff(&a, &b);

        let added: Vec<usize> = result
            .changes
            .iter()
            .filter_map(|c| match c {
                DiffChange::Added { line2, text, .. } if text == "BBBB" => Some(*line2),
                DiffChange::Replaced { line2, new, .. } if new == "BBBB" => Some(*line2),
                _ => None,
            })
            .collect();
        assert_eq!(added, vec![3], "BBBB is line 3 of file2");
    }

    #[test]
    fn test_compute_diff_equal_lengths_have_no_cliff() {
        // In-place rewrites of a 10,000-line file: past the trace budget
        // the positional fallback still names every changed line, so one more
        // rewrite cannot take the output from complete to nothing.
        let render = |rewrites: usize| {
            let a_lines: Vec<String> = (0..10_000).map(|i| format!("line {}", i)).collect();
            let b_lines: Vec<String> = (0..10_000)
                .map(|i| {
                    if i < rewrites {
                        format!("line {} REWRITTEN", i)
                    } else {
                        format!("line {}", i)
                    }
                })
                .collect();
            let a: Vec<&str> = a_lines.iter().map(|s| s.as_str()).collect();
            let b: Vec<&str> = b_lines.iter().map(|s| s.as_str()).collect();
            compute_diff(&a, &b)
        };

        // A rewritten pair renders as one `Modified` when the two texts are
        // similar enough and as `Removed` + `Added` otherwise, so the count
        // that matters is how many changed lines are named at all.
        let named = |d: &DiffResult| d.modified + d.removed;

        // Sweep across the budget rather than pinning where it sits: the trace
        // budget tracks the shape of the change, so a run of contiguous
        // rewrites gets a narrower diagonal window than scattered ones and
        // aligns further. What must hold at every density is that a changed
        // line is named — by the aligner or by the positional fallback.
        let mut aligned = 0;
        let mut positional = 0;
        for rewrites in [349, 500, 501, 700, 1_000, 4_999] {
            let result = render(rewrites);
            assert!(
                result.unaligned.is_none(),
                "{} rewrites must produce a change list",
                rewrites
            );
            assert_eq!(
                named(&result),
                rewrites,
                "{} rewrites must all be named",
                rewrites
            );
            if result.positional {
                positional += 1;
            } else {
                aligned += 1;
            }
        }
        assert!(aligned > 0 && positional > 0, "both branches must be covered");
    }

    #[test]
    fn test_compute_diff_positional_fallback_is_bounded() {
        // Two wholly different equal-length files: listing every line would
        // build a change list the size of both files. The count is exact
        // because equal lengths make it a single pass.
        let a_lines: Vec<String> = (0..20_000).map(|i| format!("alpha {}", i)).collect();
        let b_lines: Vec<String> = (0..20_000).map(|i| format!("bravo {}", i)).collect();
        let a: Vec<&str> = a_lines.iter().map(|s| s.as_str()).collect();
        let b: Vec<&str> = b_lines.iter().map(|s| s.as_str()).collect();

        let result = compute_diff(&a, &b);
        assert_eq!(result.unaligned, Some(Unaligned::DifferingLines(20_000)));
        assert!(result.changes.is_empty());
        assert!(!result.positional);

        let (out, code) = render_file_diff(
            Path::new("a.txt"),
            Path::new("b.txt"),
            &a_lines.join("\n"),
            &b_lines.join("\n"),
        );
        assert_eq!(code, 1);
        assert!(out.contains("20000 lines differ"), "got:\n{}", out);
        assert!(
            !out.contains("differences fall between"),
            "count is exact, so no region bounds, got:\n{}",
            out
        );
    }

    #[test]
    fn test_render_positional_fallback_says_so() {
        // Covers `render_file_diff`, not what `run` prints: at near-total
        // rewrite the render exceeds raw and `never_worse` substitutes the two
        // files, taking the label with it. That threshold is far above the
        // densities this branch exists for (10,000 lines / 4,999 rewritten
        // still renders), and there the raw concatenation is the better answer.
        let content1: String = (0..2400).map(|i| format!("line {}\n", i)).collect();
        let content2: String = (0..2400)
            .map(|i| format!("line {} REWRITTEN\n", i))
            .collect();
        let (out, code) =
            render_file_diff(Path::new("a.txt"), Path::new("b.txt"), &content1, &content2);

        assert_eq!(code, 1);
        assert!(out.contains("paired by line position"), "got:\n{}", out);
        for line in [1usize, 1200, 2400] {
            assert!(
                out.contains(&format!("line {} REWRITTEN", line - 1)),
                "line {} must be named, got:\n{}",
                line,
                out
            );
        }
    }

    #[test]
    fn test_render_over_cap_message_counts_lines_not_operations() {
        // Unequal lengths, so no positional fallback. The first clause must not
        // say "over 1412 lines" when 706 is the floor the cap actually implies:
        // the aligner gave up at round 1412, and an in-place rewrite is two
        // rounds, so half of it is all that is proven.
        let content1: String = (0..2000).map(|i| format!("a{}\n", i)).collect();
        let content2: String = (0..2001).map(|i| format!("b{}\n", i)).collect();
        let (out, _) =
            render_file_diff(Path::new("a.txt"), Path::new("b.txt"), &content1, &content2);

        assert!(out.contains("at least 706 lines differ"), "got:\n{}", out);
        // The region is stated as line bounds in each file. A figure shaped
        // like a count would read as the amount of change, which it is not:
        // scattered edits in a large file span nearly the whole file.
        assert!(
            out.contains("differences fall between lines 1-2000 of a.txt and 1-2001 of b.txt"),
            "region must be stated as line bounds, got:\n{}",
            out
        );
        assert!(!out.contains("spans"), "no span-shaped figure, got:\n{}", out);
    }

    #[test]
    fn test_render_region_bounds_are_lines_not_a_change_count() {
        // 1,101 changed lines in a 10,000-line file, first at 5 and last at
        // 9,500, so the changed region is ~9,495 lines. Stating that as a figure
        // next to "at least 706 lines differ" would read as 9x the real change.
        let a_lines: Vec<String> = (0..10000).map(|i| format!("line {}", i)).collect();
        let mut b_lines = a_lines.clone();
        for i in 0..1100 {
            b_lines[4 + i * 8] = format!("line {} EDITED", 4 + i * 8);
        }
        b_lines.insert(9500, "INSERTED".to_string());
        let content1: String = a_lines
            .iter()
            .map(|l| format!("{}\n", l))
            .collect::<String>();
        let content2: String = b_lines
            .iter()
            .map(|l| format!("{}\n", l))
            .collect::<String>();

        let (out, code) =
            render_file_diff(Path::new("a.txt"), Path::new("b.txt"), &content1, &content2);
        assert_eq!(code, 1);
        assert!(out.contains("at least 706 lines differ"), "got:\n{}", out);
        assert!(
            out.contains("differences fall between lines 5-"),
            "bounds start at the first difference, got:\n{}",
            out
        );
    }

    #[test]
    fn test_compute_diff_pure_insertion_past_cap_still_lists_every_line() {
        // 701 appended lines, which under an edit-distance cap was one past it
        // and exhausted the aligner's rounds. One side of the trimmed middle is empty, so the
        // script is a single insertion run and needs no search at all.
        let a_lines: Vec<String> = (0..50).map(|i| format!("keep {}", i)).collect();
        let mut b_lines = a_lines.clone();
        for i in 0..701 {
            b_lines.push(format!("appended {}", i));
        }
        let a: Vec<&str> = a_lines.iter().map(|s| s.as_str()).collect();
        let b: Vec<&str> = b_lines.iter().map(|s| s.as_str()).collect();

        let result = compute_diff(&a, &b);
        assert!(
            result.unaligned.is_none(),
            "a pure insertion is never too different to align, got: {:?}",
            result.unaligned
        );
        assert_eq!(result.added, 701);
        assert_eq!(result.removed, 0);
        assert_eq!(result.modified, 0);
        assert_eq!(result.changes.len(), 701);
        match &result.changes[0] {
            DiffChange::Added { line2, text, .. } => {
                assert_eq!((*line2, text.as_str()), (51, "appended 0"));
            }
            other => panic!("expected an addition at line 51, got {:?}", other),
        }
    }

    #[test]
    fn test_compute_diff_empty_against_populated_lists_every_line() {
        let b_lines: Vec<String> = (0..1050).map(|i| format!("line {}", i)).collect();
        let b: Vec<&str> = b_lines.iter().map(|s| s.as_str()).collect();

        let result = compute_diff(&[], &b);
        assert!(result.unaligned.is_none());
        assert_eq!(result.added, 1050);
        assert_eq!(result.changes.len(), 1050);

        let reverse = compute_diff(&b, &[]);
        assert!(reverse.unaligned.is_none());
        assert_eq!(reverse.removed, 1050);
        assert_eq!(reverse.changes.len(), 1050);
    }

    /// Deterministic pseudo-random source, so a failure reproduces exactly.
    fn lcg(state: &mut u64) -> u64 {
        *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *state >> 33
    }

    #[test]
    fn test_myers_ops_script_reconstructs_the_second_file() {
        // The aligner explores a banded diagonal window, so the trace no longer
        // covers `[-d, d]` and the backtrack reads it through a per-round base.
        // An off-by-one there produces a plausible-looking script that does not
        // reconstruct file2, which no single hand-written case would catch.
        let mut seed = 0x5eed_1234_u64;
        let mut aligned = 0usize;

        for _ in 0..4_000 {
            let n = (lcg(&mut seed) % 40) as usize;
            let m = (lcg(&mut seed) % 40) as usize;
            let alphabet = 1 + (lcg(&mut seed) % 4);
            let a_lines: Vec<String> = (0..n)
                .map(|_| format!("L{}", lcg(&mut seed) % alphabet))
                .collect();
            let b_lines: Vec<String> = (0..m)
                .map(|_| format!("L{}", lcg(&mut seed) % alphabet))
                .collect();
            let a: Vec<&str> = a_lines.iter().map(|s| s.as_str()).collect();
            let b: Vec<&str> = b_lines.iter().map(|s| s.as_str()).collect();

            let ops = match myers_ops(&a, &b, 0) {
                Ok(Aligned::Script(ops)) => ops,
                Ok(Aligned::TooManySteps { .. }) | Err(_) => continue,
            };
            aligned += 1;

            // Replay the script: `Keep` consumes one line from each side, `Del`
            // one from file1, `Ins` one from file2.
            let (mut i, mut j) = (0usize, 0usize);
            let mut rebuilt: Vec<&str> = Vec::new();
            for op in &ops {
                match op {
                    Op::Keep => {
                        assert_eq!(a.get(i), b.get(j), "Keep must pair equal lines");
                        rebuilt.push(a[i]);
                        i += 1;
                        j += 1;
                    }
                    Op::Del {
                        line1,
                        after2,
                        text,
                    } => {
                        assert_eq!(*line1, i + 1, "Del numbers file1");
                        assert_eq!(*after2, j, "Del records file2's cursor");
                        assert_eq!(a[i], text.as_str(), "Del names file1's text");
                        i += 1;
                    }
                    Op::Ins {
                        after1,
                        line2,
                        text,
                    } => {
                        assert_eq!(*line2, j + 1, "Ins numbers file2");
                        assert_eq!(*after1, i, "Ins records file1's cursor");
                        assert_eq!(b[j], text.as_str(), "Ins names file2's text");
                        rebuilt.push(b[j]);
                        j += 1;
                    }
                }
            }
            assert_eq!((i, j), (n, m), "the script must consume both files");
            assert_eq!(rebuilt, b, "the script must reconstruct file2");
        }

        assert!(aligned > 3_000, "most pairs must align, got {}", aligned);
    }

    #[test]
    fn test_myers_ops_script_is_minimal() {
        // Minimality against a brute-force LCS: the banded window must not drop
        // a diagonal an optimal path needs.
        fn lcs_len(a: &[&str], b: &[&str]) -> usize {
            let mut prev = vec![0usize; b.len() + 1];
            for x in a {
                let mut cur = vec![0usize; b.len() + 1];
                for (j, y) in b.iter().enumerate() {
                    cur[j + 1] = if x == y {
                        prev[j] + 1
                    } else {
                        cur[j].max(prev[j + 1])
                    };
                }
                prev = cur;
            }
            prev[b.len()]
        }

        let mut seed = 0xfeed_4321_u64;
        for _ in 0..2_000 {
            let n = (lcg(&mut seed) % 25) as usize;
            let m = (lcg(&mut seed) % 25) as usize;
            let alphabet = 1 + (lcg(&mut seed) % 3);
            let a_lines: Vec<String> = (0..n)
                .map(|_| format!("L{}", lcg(&mut seed) % alphabet))
                .collect();
            let b_lines: Vec<String> = (0..m)
                .map(|_| format!("L{}", lcg(&mut seed) % alphabet))
                .collect();
            let a: Vec<&str> = a_lines.iter().map(|s| s.as_str()).collect();
            let b: Vec<&str> = b_lines.iter().map(|s| s.as_str()).collect();

            let Ok(Aligned::Script(ops)) = myers_ops(&a, &b, 0) else {
                continue;
            };
            let edits = ops.iter().filter(|op| !matches!(op, Op::Keep)).count();
            assert_eq!(
                edits,
                n + m - 2 * lcs_len(&a, &b),
                "script must be minimal for {:?} vs {:?}",
                a,
                b
            );
        }
    }

    /// Apply a classic diff to `a` and return what it produces, checking that
    /// every hunk's `<` bodies name file1 at the lines the header claims and
    /// every `>` body names file2 at its own.
    fn replay_classic(script: &str, a: &[&str], b: &[&str]) -> Vec<String> {
        fn range(spec: &str) -> (usize, usize) {
            match spec.split_once(',') {
                Some((s, e)) => (s.parse().unwrap(), e.parse().unwrap()),
                None => {
                    let n = spec.parse().unwrap();
                    (n, n)
                }
            }
        }

        let mut out: Vec<String> = Vec::new();
        let mut cursor = 0usize; // file1 lines already emitted or dropped
        let mut lines = script.lines().peekable();

        while let Some(header) = lines.next() {
            let op = header
                .chars()
                .find(|c| matches!(c, 'a' | 'c' | 'd'))
                .expect("hunk header carries an operation");
            let (left, right) = header.split_once(op).unwrap();

            let mut old_body = Vec::new();
            let mut new_body = Vec::new();
            while let Some(line) = lines.peek() {
                if let Some(rest) = line.strip_prefix("< ") {
                    old_body.push(rest.to_string());
                } else if let Some(rest) = line.strip_prefix("> ") {
                    new_body.push(rest.to_string());
                } else if *line != "---" {
                    break;
                }
                lines.next();
            }

            let (start1, end1, start2, end2) = match op {
                'a' => {
                    let anchor: usize = left.parse().unwrap();
                    let (s2, e2) = range(right);
                    (anchor + 1, anchor, s2, e2)
                }
                'd' => {
                    let (s1, e1) = range(left);
                    (s1, e1, 0, 0)
                }
                _ => {
                    let (s1, e1) = range(left);
                    let (s2, e2) = range(right);
                    (s1, e1, s2, e2)
                }
            };

            // Copy the untouched file1 lines that precede this hunk.
            assert!(start1 > cursor, "hunks must not overlap: {header}");
            for line in a.iter().take(start1 - 1).skip(cursor) {
                out.push((*line).to_string());
            }
            cursor = start1 - 1;

            if op != 'a' {
                assert_eq!(
                    old_body,
                    a[start1 - 1..end1]
                        .iter()
                        .map(|l| (*l).to_string())
                        .collect::<Vec<_>>(),
                    "`<` body must name file1 at {header}"
                );
                cursor = end1;
            }
            if op != 'd' {
                assert_eq!(
                    new_body,
                    b[start2 - 1..end2]
                        .iter()
                        .map(|l| (*l).to_string())
                        .collect::<Vec<_>>(),
                    "`>` body must name file2 at {header}"
                );
                out.extend(new_body);
            }
        }

        out.extend(a.iter().skip(cursor).map(|l| (*l).to_string()));
        out
    }

    #[test]
    fn test_classic_diff_replays_into_the_second_file() {
        // The classic renderer numbers each hunk in both files, and after an
        // insertion the two frames disagree. Deriving one from the other — or
        // pairing a replacement by equal line numbers — produces a script that
        // still parses and still looks like a diff, so only replaying it
        // catches the mislabelling.
        let mut seed = 0x0c1a_551c_u64;
        let mut replayed = 0usize;

        for _ in 0..3_000 {
            let n = (lcg(&mut seed) % 25) as usize;
            let m = (lcg(&mut seed) % 25) as usize;
            let alphabet = 1 + (lcg(&mut seed) % 5);
            let a_lines: Vec<String> = (0..n)
                .map(|_| format!("L{}", lcg(&mut seed) % alphabet))
                .collect();
            let b_lines: Vec<String> = (0..m)
                .map(|_| format!("L{}", lcg(&mut seed) % alphabet))
                .collect();
            let a: Vec<&str> = a_lines.iter().map(|s| s.as_str()).collect();
            let b: Vec<&str> = b_lines.iter().map(|s| s.as_str()).collect();

            let diff = compute_diff(&a, &b);
            if diff.unaligned.is_some() {
                continue;
            }
            replayed += 1;

            let script = format_classic_diff(&diff);
            assert_eq!(
                replay_classic(&script, &a, &b),
                b_lines,
                "script must rebuild file2 for {:?} vs {:?}:\n{}",
                a,
                b,
                script
            );
        }

        assert!(replayed > 2_500, "most pairs must render, got {}", replayed);
    }

    #[test]
    fn test_compute_diff_lopsided_pair_aligns_past_the_edit_distance() {
        // One line against five thousand, sharing a line in the middle so the
        // prefix/suffix trim cannot empty either side. The minimal script is
        // 4,999 insertions, so an edit-distance cap refused it — but only one
        // deletion is possible, so the diagonal window stays three wide and the
        // whole trace is a few thousand cells.
        let b_lines: Vec<String> = (0..5_000)
            .map(|i| {
                if i == 2_500 {
                    "KEEP".to_string()
                } else {
                    format!("ins {}", i)
                }
            })
            .collect();
        let b: Vec<&str> = b_lines.iter().map(|s| s.as_str()).collect();

        let result = compute_diff(&["KEEP"], &b);
        assert!(
            result.unaligned.is_none(),
            "a lopsided pair must still align, got: {:?}",
            result.unaligned
        );
        assert!(!result.positional);
        assert_eq!(result.added, 4_999);
        assert_eq!(result.removed, 0);
        assert_eq!(result.modified, 0);
    }

    #[test]
    fn test_compute_diff_listing_is_bounded_by_bytes_not_only_lines() {
        // 3,000 changes is inside `POSITIONAL_CHANGE_CAP`, but each holds two
        // 1,000-byte strings, so the list would clone six megabytes before
        // `never_worse` could discard it. The count stays exact.
        let long_a = "a".repeat(1_000);
        let long_b = "b".repeat(1_000);
        let a_lines: Vec<String> = (0..3_000).map(|_| long_a.clone()).collect();
        let b_lines: Vec<String> = (0..3_000).map(|_| long_b.clone()).collect();
        let a: Vec<&str> = a_lines.iter().map(|s| s.as_str()).collect();
        let b: Vec<&str> = b_lines.iter().map(|s| s.as_str()).collect();

        let result = compute_diff(&a, &b);
        assert_eq!(result.unaligned, Some(Unaligned::DifferingLines(3_000)));
        assert!(result.changes.is_empty());

        // Short lines at the same count still list.
        let short_a: Vec<String> = (0..3_000).map(|i| format!("a{}", i)).collect();
        let short_b: Vec<String> = (0..3_000).map(|i| format!("b{}", i)).collect();
        let sa: Vec<&str> = short_a.iter().map(|s| s.as_str()).collect();
        let sb: Vec<&str> = short_b.iter().map(|s| s.as_str()).collect();
        let short = compute_diff(&sa, &sb);
        assert!(short.unaligned.is_none(), "3,000 short changes still list");
    }

    #[test]
    fn test_compute_diff_one_sided_run_is_bounded() {
        // The listing is exact but not unbounded: past `POSITIONAL_CHANGE_CAP`
        // the count says more than ten thousand lines of change list would.
        let b_lines: Vec<String> = (0..POSITIONAL_CHANGE_CAP + 1)
            .map(|i| format!("line {}", i))
            .collect();
        let b: Vec<&str> = b_lines.iter().map(|s| s.as_str()).collect();

        let result = compute_diff(&[], &b);
        assert_eq!(
            result.unaligned,
            Some(Unaligned::DifferingLines(POSITIONAL_CHANGE_CAP + 1))
        );
        assert!(result.changes.is_empty());
    }

    #[test]
    fn test_render_reference_frames_are_labelled() {
        // `-` and `+` numbers come from different files; the legend appears
        // only when the output actually mixes them.
        let (mixed, _) = render_file_diff(
            Path::new("a.txt"),
            Path::new("b.txt"),
            "x\nAAAAAAAA\n",
            "NEW\nx\nZZZZZZZZ\n",
        );
        assert!(
            mixed.contains("(- numbered in a.txt, + in b.txt)"),
            "got:\n{}",
            mixed
        );

        let (modified_only, _) =
            render_file_diff(Path::new("a.txt"), Path::new("b.txt"), "a: 1\n", "a: 2\n");
        assert!(
            !modified_only.contains("numbered in"),
            "no legend when nothing mixes frames, got:\n{}",
            modified_only
        );
    }

    #[test]
    fn test_render_frame_legend_covers_added_beside_modified() {
        // `+` is numbered in file2 and `~` in file1, so the two mix frames just
        // as `-` and `+` do. Gating the legend on a `-` being present left this
        // shape bare: `value = alpha2` is at line 5 of b.txt, not the 4 the `~`
        // shows, and nothing said which file the number belonged to.
        let (out, _) = render_file_diff(
            Path::new("a.txt"),
            Path::new("b.txt"),
            "a\nb\nc\nvalue = alpha\n",
            "a\nEXTRA\nb\nc\nvalue = alpha2\n",
        );

        assert!(out.contains("+1 added, -0 removed, ~1 modified"), "got:\n{}", out);
        assert!(
            out.contains("(~ numbered in a.txt, + in b.txt)"),
            "an insertion above a modification mixes frames, got:\n{}",
            out
        );
    }

    #[test]
    fn test_render_frame_legend_names_only_the_markers_present() {
        // The legend exists to stop a line-number misread, so one that
        // describes different output than the reader is looking at is worse
        // than none: with no `-` on screen it announced a `-` frame with no
        // lines in it and said nothing about the `~` lines that were there.
        let (added_and_modified, _) = render_file_diff(
            Path::new("a.txt"),
            Path::new("b.txt"),
            "a\nb\nc\nvalue = alpha\n",
            "a\nEXTRA\nb\nc\nvalue = alpha2\n",
        );
        assert!(
            !added_and_modified.lines().any(|l| l.starts_with('-')),
            "no `-` lines in this shape, got:\n{}",
            added_and_modified
        );
        assert!(
            !added_and_modified.contains("(- "),
            "the legend must not name an absent frame, got:\n{}",
            added_and_modified
        );

        // A `-` with no `~`: the legend names the `-` alone.
        let (added_and_removed, _) = render_file_diff(
            Path::new("a.txt"),
            Path::new("b.txt"),
            "a\nb\nzzzz\n",
            "a\nEXTRA\nb\nqqqq\n",
        );
        assert!(
            added_and_removed.contains("(- numbered in a.txt, + in b.txt)"),
            "got:\n{}",
            added_and_removed
        );

        // Both frames on screen: the legend names both.
        let (all_three, _) = render_file_diff(
            Path::new("a.txt"),
            Path::new("b.txt"),
            "a\nb\nzzzz\nvalue = alpha\n",
            "a\nEXTRA\nb\nqqqq\nvalue = alpha2\n",
        );
        assert!(
            all_three.contains("(- and ~ numbered in a.txt, + in b.txt)"),
            "got:\n{}",
            all_three
        );
    }

    #[test]
    fn test_render_added_only_needs_no_frame_legend() {
        // Every listed line comes from file2. One frame, no note.
        let (out, _) =
            render_file_diff(Path::new("a.txt"), Path::new("b.txt"), "a\nb\n", "a\nNEW\nb\n");

        assert!(out.contains("+1 added, -0 removed, ~0 modified"), "got:\n{}", out);
        assert!(
            !out.contains("numbered in"),
            "one frame needs no legend, got:\n{}",
            out
        );
    }

    #[test]
    fn test_compute_diff_aligned_script_past_the_count_cap_refuses() {
        // The listing path the band made reachable. One line against 60,000
        // sharing a line in the middle aligns cheaply — the window stays three
        // diagonals wide — and would then build 59,999 changes, 11x
        // `POSITIONAL_CHANGE_CAP`, from an input the empty-file case refuses at
        // the same size for free.
        let a = vec!["SHARED"];
        let b_lines: Vec<String> = (0..60000)
            .map(|i| {
                if i == 30000 {
                    "SHARED".to_string()
                } else {
                    format!("x{}", i)
                }
            })
            .collect();
        let b: Vec<&str> = b_lines.iter().map(|s| s.as_str()).collect();

        let result = compute_diff(&a, &b);
        assert_eq!(
            result.unaligned,
            Some(Unaligned::EditScript {
                removed: 0,
                added: 59999
            })
        );
        assert!(result.changes.is_empty());
    }

    #[test]
    fn test_aligned_script_over_the_count_cap_is_never_materialised() {
        // The count budget is spent before the script exists. `myers_ops` knows
        // the edit distance the moment it reaches the end, so the cheap-to-
        // answer shape — one line against 60,000, which the band aligns in a
        // three-diagonal window — refuses there rather than cloning 59,999
        // lines into an `Op` vector it is about to throw away.
        let a = vec!["SHARED"];
        let b_lines: Vec<String> = (0..60000)
            .map(|i| {
                if i == 30000 {
                    "SHARED".to_string()
                } else {
                    format!("x{}", i)
                }
            })
            .collect();
        let b: Vec<&str> = b_lines.iter().map(|s| s.as_str()).collect();

        match myers_ops(&a, &b, 0) {
            Ok(Aligned::TooManySteps { removed, added }) => {
                assert_eq!((removed, added), (0, 59999));
            }
            Ok(Aligned::Script(ops)) => panic!("built a {}-step script it cannot list", ops.len()),
            Err(d) => panic!("the band must reach this pair, gave up at d = {}", d),
        }
    }

    #[test]
    fn test_listing_cap_counts_positions_not_rendered_lines() {
        // `POSITIONAL_CHANGE_CAP` bounds differing positions, and a position
        // whose two texts share no characters renders as a `-` and a `+`. The
        // constant's own comment has to say that: at exactly the cap the output
        // is twice its value, which reads as a broken bound if the cap is
        // documented as a line count.
        // Disjoint character sets, so `similarity` can never rate a pair as
        // `Modified` and fold two rendered lines back into one.
        let encode = |i: usize, alphabet: &[u8]| -> String {
            i.to_string()
                .bytes()
                .map(|d| alphabet[(d - b'0') as usize] as char)
                .collect()
        };
        let content1: String = (0..POSITIONAL_CHANGE_CAP)
            .map(|i| format!("{}\n", encode(i, b"abcdefghij")))
            .collect();
        let content2: String = (0..POSITIONAL_CHANGE_CAP)
            .map(|i| format!("{}\n", encode(i, b"PQRSTUVWXY")))
            .collect();

        let diff = changes_of(&content1, &content2);
        assert!(diff.unaligned.is_none(), "exactly at the cap is admitted");
        assert_eq!(
            (diff.added, diff.removed, diff.modified),
            (POSITIONAL_CHANGE_CAP, POSITIONAL_CHANGE_CAP, 0)
        );
        assert_eq!(
            format_diff_changes(&diff).lines().count(),
            2 * POSITIONAL_CHANGE_CAP,
            "each differing position renders two lines"
        );
    }

    #[test]
    fn test_compute_diff_aligned_script_past_the_byte_cap_refuses() {
        // Two changes, past the byte budget. A count cap alone would pass this
        // and clone 2.2MB twice on the way to the render.
        let big1 = "x".repeat(1_100_000);
        let big2 = "y".repeat(1_100_000);
        let a = vec!["head", big1.as_str(), "tail"];
        let b = vec!["head", big2.as_str(), "tail"];

        let result = compute_diff(&a, &b);
        assert_eq!(
            result.unaligned,
            Some(Unaligned::EditScript {
                removed: 1,
                added: 1
            })
        );
        assert!(result.changes.is_empty());
    }

    #[test]
    fn test_render_edit_script_over_cap_states_both_sides() {
        // Both counts, because the script knows both exactly and one figure
        // would not: the runs have not been paired, and a pairing turns two
        // steps into either one `~` or one `-` plus one `+`.
        let content1 = "SHARED\n".to_string();
        let content2: String = (0..60000)
            .map(|i| {
                if i == 30000 {
                    "SHARED\n".to_string()
                } else {
                    format!("x{}\n", i)
                }
            })
            .collect();
        let (out, code) =
            render_file_diff(Path::new("a.txt"), Path::new("b.txt"), &content1, &content2);

        assert_eq!(code, 1);
        assert!(
            out.contains("-0 lines only in a.txt, +59999 only in b.txt"),
            "got:\n{}",
            out
        );
        assert!(out.contains("rtk proxy diff"), "got:\n{}", out);
        assert!(
            out.lines().count() <= 2,
            "the refusal must not grow with the input, got:\n{}",
            out
        );
    }

    #[test]
    fn test_compute_diff_moderately_changed_file_still_aligns() {
        // An 18%-changed 2,000-line file. One inserted line makes the lengths
        // unequal, which removes the positional fallback, so an aligner that
        // gives up here lists nothing at all. The trace budget has to reach
        // further than the listing budget refuses at, or the refusal is a cliff.
        let a_lines: Vec<String> = (0..2000).map(|i| format!("line {}", i)).collect();
        let mut b_lines = a_lines.clone();
        for line in b_lines.iter_mut().take(360) {
            *line = format!("{} EDITED", line);
        }
        b_lines.insert(1500, "INSERTED".to_string());
        let a: Vec<&str> = a_lines.iter().map(|s| s.as_str()).collect();
        let b: Vec<&str> = b_lines.iter().map(|s| s.as_str()).collect();

        let result = compute_diff(&a, &b);
        assert!(result.unaligned.is_none(), "must still align");
        assert_eq!(result.modified, 360);
        assert_eq!(result.added, 1);
        assert_eq!(result.removed, 0);
    }

    #[test]
    fn test_compute_diff_scattered_rewrites_align_to_about_seven_hundred() {
        // What `MAX_TRACE_CELLS` is really choosing, pinned so a change to the
        // constant has to restate it. Scattered in-place rewrites in a file far
        // longer than the change get the full diagonal window, which is the
        // worst case for the trace.
        let base: Vec<String> = (0..10000).map(|i| format!("line {}", i)).collect();
        let rewrite = |count: usize| {
            let mut b = base.clone();
            for i in 0..count {
                b[i * 9] = format!("line {} EDITED", i * 9);
            }
            b
        };

        let a: Vec<&str> = base.iter().map(|s| s.as_str()).collect();

        let under = rewrite(700);
        let under_ref: Vec<&str> = under.iter().map(|s| s.as_str()).collect();
        let aligned = compute_diff(&a, &under_ref);
        assert!(!aligned.positional, "700 rewrites must still align");
        assert_eq!(aligned.modified, 700);

        let over = rewrite(720);
        let over_ref: Vec<&str> = over.iter().map(|s| s.as_str()).collect();
        let fell_back = compute_diff(&a, &over_ref);
        assert!(
            fell_back.positional,
            "past the budget equal lengths pair by position rather than listing nothing"
        );
    }

    #[test]
    fn test_render_positional_fallback_has_no_frame_legend() {
        // `positional_changes` numbers both halves of a dissimilar pair from
        // the same position, so there is one frame. The legend would tell the
        // reader to read two identical numbers as belonging to different files.
        let content1: String = (0..2400).map(|i| format!("aaa{}\n", i)).collect();
        let content2: String = (0..2400).map(|i| format!("zzz{}\n", i)).collect();
        let (out, _) =
            render_file_diff(Path::new("a.txt"), Path::new("b.txt"), &content1, &content2);

        assert!(out.contains("paired by line position"), "got:\n{}", out);
        assert!(out.contains("-   1 aaa0"), "got:\n{}", out);
        assert!(out.contains("+   1 zzz0"), "got:\n{}", out);
        assert!(
            !out.contains("numbered in"),
            "one frame needs no legend, got:\n{}",
            out
        );
    }

    #[test]
    fn test_compute_diff_two_far_apart_edits_still_align() {
        // The cap is on the amount of change, not on the span between the
        // first and last edit. Two 2100-line files differing at lines 5 and
        // 2095 have a 2091-line changed region and an edit distance of 4.
        let a_lines: Vec<String> = (0..2100).map(|i| format!("line {}", i)).collect();
        let mut b_lines = a_lines.clone();
        b_lines[4] = "line 4 EDITED".to_string();
        b_lines[2094] = "line 2094 EDITED".to_string();
        let a: Vec<&str> = a_lines.iter().map(|s| s.as_str()).collect();
        let b: Vec<&str> = b_lines.iter().map(|s| s.as_str()).collect();

        let result = compute_diff(&a, &b);
        assert!(
            result.unaligned.is_none(),
            "two edits must not exhaust the aligner"
        );
        assert_eq!(result.modified, 2, "both edits are in-place rewrites");
        assert_eq!(result.added, 0);
        assert_eq!(result.removed, 0);

        let lines: Vec<usize> = result
            .changes
            .iter()
            .filter_map(|c| match c {
                DiffChange::Modified { line1, .. } => Some(*line1),
                _ => None,
            })
            .collect();
        assert_eq!(lines, vec![5, 2095], "line numbers, not region offsets");
    }

    #[test]
    fn test_compute_diff_past_edit_distance_cap_reports_a_region_not_counts() {
        // Wholly different middles of unequal length, so neither the aligner
        // nor the positional fallback applies. The counts stay zero rather
        // than restating the region span as if it had been measured.
        let a_lines: Vec<String> = (0..2001).map(|i| format!("a{}", i)).collect();
        let b_lines: Vec<String> = (0..2002).map(|i| format!("b{}", i)).collect();
        let a: Vec<&str> = a_lines.iter().map(|s| s.as_str()).collect();
        let b: Vec<&str> = b_lines.iter().map(|s| s.as_str()).collect();

        let result = compute_diff(&a, &b);
        assert_eq!(
            result.unaligned,
            Some(Unaligned::RegionBounds {
                differing_floor: 706,
                first: 1,
                last1: 2001,
                last2: 2002
            })
        );
        assert_eq!(result.added, 0);
        assert_eq!(result.removed, 0);
        assert_eq!(result.modified, 0);
        assert!(result.changes.is_empty());
    }

    #[test]
    fn test_render_far_apart_edits_reports_two_changes_not_a_region() {
        // The shape the positional compare on develop got right, and that a
        // size-keyed cap would have turned into a confident "+2091 / -2091".
        let content1: String = (0..2100).map(|i| format!("line {}\n", i)).collect();
        let content2: String = (0..2100)
            .map(|i| match i {
                4 => "line 4 EDITED\n".to_string(),
                2094 => "line 2094 EDITED\n".to_string(),
                _ => format!("line {}\n", i),
            })
            .collect();
        let (out, code) =
            render_file_diff(Path::new("a.txt"), Path::new("b.txt"), &content1, &content2);

        assert_eq!(code, 1);
        assert!(out.contains("+0 added, -0 removed, ~2 modified"), "got:\n{}", out);
        assert!(!out.contains("2091"), "region size must not appear, got:\n{}", out);
    }

    #[test]
    fn test_render_past_edit_distance_cap_names_the_region_as_a_region() {
        let content1: String = (0..2001).map(|i| format!("a{}\n", i)).collect();
        let content2: String = (0..2002).map(|i| format!("b{}\n", i)).collect();
        let (out, code) =
            render_file_diff(Path::new("a.txt"), Path::new("b.txt"), &content1, &content2);

        assert_eq!(code, 1);
        assert!(
            out.contains("differences fall between lines 1-2001 of a.txt and 1-2002 of b.txt"),
            "got:\n{}",
            out
        );
        assert!(out.contains("too different to align"), "got:\n{}", out);
        assert!(!out.contains("added"), "no count is claimed, got:\n{}", out);
        assert!(!out.contains("text matches"), "got:\n{}", out);
    }

    #[test]
    fn test_crlf_line_numbers_ignores_an_unterminated_tail() {
        // A file ending in a bare `\r` has no newline there, so that `\r` is
        // content rather than half a CRLF terminator.
        assert_eq!(crlf_line_numbers("x\r\ny\r"), vec![1]);
        assert_eq!(crlf_line_numbers("x\ny\r"), Vec::<usize>::new());
        assert_eq!(crlf_line_numbers("a\r\nb\r\n"), vec![1, 2]);

        let (out, _) =
            render_file_diff(Path::new("a.txt"), Path::new("b.txt"), "x\r\ny\r", "x\ny\r");
        assert!(out.contains("1 CRLF vs 0 CRLF"), "got:\n{}", out);
    }

    #[test]
    fn test_invisible_message_affordability_ignores_path_length() {
        // A three-line CSV saved once on Windows and once on Unix. The message
        // is the only thing that distinguishes the two blobs, so path length
        // must not be what disqualifies it.
        let content1 = "a,b\nc,d\ne,f\n";
        let content2 = "a,b\r\nc,d\r\ne,f\r\n";
        let raw = format!("{}\n---\n{}", content1, content2);

        for (f1, f2) in [
            ("e.txt", "f.txt"),
            (
                "/home/user/projects/rtk/tests/fixtures/expected_output.csv",
                "/home/user/projects/rtk/tests/fixtures/actual_output.csv",
            ),
        ] {
            let (p1, p2) = (Path::new(f1), Path::new(f2));
            let (rtk, _) = render_file_diff(p1, p2, content1, content2);
            assert!(
                invisible_message_affordable(&rtk, p1, p2, &raw),
                "{} vs {} must not price the message out",
                f1,
                f2
            );
        }
    }

    #[test]
    fn test_invisible_message_affordability_still_has_a_ceiling() {
        // The exception stays bounded by what the message says. Excluding the
        // header raised the effective ceiling — `"a\n"` vs `"a"` now shows the
        // note where it used to show raw — but the longest note form against a
        // 16-byte pair is still priced out.
        let (p1, p2) = (Path::new("a"), Path::new("b"));
        let (content1, content2) = ("a\r\nb\n", "a\nb\r\n");
        let raw = format!("{}\n---\n{}", content1, content2);
        let (rtk, _) = render_file_diff(p1, p2, content1, content2);

        assert!(rtk.contains("first differing at line 1"), "got:\n{}", rtk);
        assert!(
            !invisible_message_affordable(&rtk, p1, p2, &raw),
            "got:\n{}",
            rtk
        );
    }

    #[test]
    fn test_describe_invisible_difference_never_prints_equal_byte_counts() {
        // Same CRLF count on both sides, different placement. The old fallback
        // printed "5 vs 5 bytes", which reads as "no difference at all".
        let (out, code) =
            render_file_diff(Path::new("a.txt"), Path::new("b.txt"), "a\r\nb\n", "a\nb\r\n");

        assert_eq!(code, 1);
        assert!(!out.contains("5 vs 5 bytes"), "got:\n{}", out);
        assert!(
            out.contains("1 CRLF on each side, first differing at line 1"),
            "got:\n{}",
            out
        );
    }

    // --- classic diff fallback, baseline and guard routing ---

    #[test]
    fn test_never_worse_fallback_is_a_classic_diff() {
        let comparison = compare_files("alpha beta\n", "alpha zzzz\n");
        let fallback = classic_fallback(&comparison);
        let (rendered, code) = render_diff(Path::new("before"), Path::new("after"), &comparison);
        let shown = select_file_diff_output(&comparison, &fallback, "", &rendered, false);

        assert_eq!(code, 1);
        assert!(shown.contains("1c1"), "got:\n{}", shown);
        assert!(shown.contains("< alpha beta"));
        assert!(shown.contains("\n---\n"));
        assert!(shown.contains("> alpha zzzz"));
    }

    #[test]
    fn test_tracking_baseline_never_books_a_loss() {
        // Two unrelated files: the classic diff carries both of them plus the
        // "< " / "> " markers, so it is bigger than a plain dump. Measuring
        // against the dump used to record negative savings.
        let content1: String = (0..40).map(|i| format!("old line {i}\n")).collect();
        let content2: String = (0..40).map(|i| format!("brand new content {i}\n")).collect();
        let both_files = format!("{}\n---\n{}", content1, content2);

        let comparison = compare_files(&content1, &content2);
        let fallback = classic_fallback(&comparison);
        let (rendered, _) = render_diff(Path::new("a"), Path::new("b"), &comparison);
        let shown = select_file_diff_output(&comparison, &fallback, &both_files, &rendered, false);
        let baseline = tracking_baseline(&fallback, &both_files, shown);

        assert!(
            tracking::estimate_tokens(baseline) >= tracking::estimate_tokens(shown),
            "baseline {} < shown {} would record negative savings",
            tracking::estimate_tokens(baseline),
            tracking::estimate_tokens(shown)
        );
    }

    #[test]
    fn test_tracking_baseline_identical_files_use_both_files() {
        let both_files = "a: 1\nb: 2\n\n---\na: 1\nb: 2\n";
        let shown = "[ok] Files are identical\n";

        assert_eq!(
            tracking_baseline("", both_files, shown),
            both_files,
            "identical files should still measure against the dump"
        );
    }

    #[test]
    fn test_tracking_baseline_empty_files_do_not_book_a_loss() {
        // Both files empty: the dump is shorter than the verdict line.
        let shown = "[ok] Files are identical\n";

        assert_eq!(tracking_baseline("", "\n---\n", shown), shown);
    }

    #[test]
    fn test_identical_files_keep_the_success_message() {
        let comparison = compare_files("same\n", "same\n");
        let rendered = "[ok] Files are identical\n";

        assert_eq!(
            select_file_diff_output(&comparison, "", "", rendered, false),
            rendered
        );
    }

    #[test]
    fn test_classic_diff_covers_modified_line_boundary_cases() {
        for (old, new) in [
            ("alpha beta gamma delta", "alpha beta XXXXX delta"),
            ("alpha beta gamma", "alpha beta"),
            ("alpha beta gamma delta", "XXXXX beta gamma delta"),
        ] {
            let diff = changes_of(&format!("{old}\n"), &format!("{new}\n"));
            let fallback = format_classic_diff(&diff);

            assert!(fallback.contains(&format!("< {old}")), "got:\n{fallback}");
            assert!(fallback.contains(&format!("> {new}")), "got:\n{fallback}");
        }
    }

    #[test]
    fn test_classic_diff_groups_a_replacement_after_a_shift() {
        // The `-`/`+` halves of a replacement carry different line numbers once
        // an insertion has shifted the two files apart. Grouping them by equal
        // line numbers degrades every replacement past the shift into a
        // separate `NdM` plus `NaM`: still well-formed, still wrong about what
        // changed together.
        let content1 = "keep\nalpha beta\n";
        let content2 = "INSERTED\nkeep\nzzzz yyyy\n";
        let diff = changes_of(content1, content2);
        let fallback = format_classic_diff(&diff);

        assert!(
            fallback.contains("2c3"),
            "replacement must group as one change hunk, got:\n{}",
            fallback
        );
        assert!(fallback.contains("< alpha beta"), "got:\n{}", fallback);
        assert!(fallback.contains("> zzzz yyyy"), "got:\n{}", fallback);
        // The insertion is anchored in file1 and ranged in file2.
        assert!(fallback.contains("0a1"), "got:\n{}", fallback);
    }

    #[test]
    fn test_classic_diff_anchors_a_deletion_in_file2() {
        // `NdM` names the file2 line the deleted text would have followed. With
        // one frame for both files that anchor drifts by every earlier
        // insertion.
        let content1 = "a\nGONE\nb\n";
        let content2 = "NEW\na\nb\n";
        let diff = changes_of(content1, content2);
        let fallback = format_classic_diff(&diff);

        assert!(fallback.contains("0a1"), "got:\n{}", fallback);
        assert!(fallback.contains("2d2"), "got:\n{}", fallback);
        assert!(fallback.contains("< GONE"), "got:\n{}", fallback);
    }

    #[test]
    fn test_over_cap_comparison_is_not_reported_as_identical() {
        // An empty change list is not a synonym for "identical". Every refusal
        // to build a listing produces one, and routing those through the
        // identical branch reports two wholly different files as the same, with
        // exit 0 — the bug this module exists to close.
        let content1: String = (0..60_000).map(|i| format!("a{}\n", i)).collect();
        let content2: String = (0..60_000).map(|i| format!("b{}\n", i)).collect();
        let both_files = format!("{}\n---\n{}", content1, content2);

        let comparison = compare_files(&content1, &content2);
        let fallback = classic_fallback(&comparison);
        let (rendered, code) = render_diff(Path::new("a.txt"), Path::new("b.txt"), &comparison);
        let shown = select_file_diff_output(&comparison, &fallback, &both_files, &rendered, false);

        assert_eq!(code, 1, "files that differ must exit 1");
        assert!(
            !shown.contains("identical"),
            "over-cap comparison reported as identical:\n{}",
            shown
        );
        assert!(shown.contains("lines differ"), "got:\n{}", shown);
        assert!(
            shown.len() < both_files.len(),
            "the refusal must not fall back to the dump"
        );
    }

    #[test]
    fn test_invisible_difference_is_not_reported_as_identical() {
        let comparison = compare_files("x\r\ny\r\n", "x\ny\n");
        let fallback = classic_fallback(&comparison);
        let (rendered, code) = render_diff(Path::new("a.txt"), Path::new("b.txt"), &comparison);

        assert_eq!(code, 1);
        assert!(fallback.is_empty(), "no classic diff exists here");
        assert_eq!(
            select_file_diff_output(&comparison, &fallback, "x\r\ny\r\n\n---\nx\ny\n", &rendered, true),
            rendered,
            "an affordable invisible-difference message survives the guard"
        );
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
            DiffChange::Removed { text, .. } | DiffChange::Added { text, .. } => {
                assert_eq!(text.len(), 500, "Line was truncated!");
            }
            DiffChange::Modified { old, .. } | DiffChange::Replaced { old, .. } => {
                assert_eq!(old.len(), 500, "Line was truncated!");
            }
        }
    }
}
