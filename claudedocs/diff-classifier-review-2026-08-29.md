# condense_unified_diff classifier — six-round review record (2026-08-29)

Design brief for the follow-up ticket. PR #121 shipped only the column-0 fix;
the classifier rewrite explored on its branch was reverted after six
adversarial review rounds showed the approach was not converging. The full
exploration survives in the branch history: `b008c53..1f64783` on
`claude/heuristic-booth-4f0500` (reverted, never force-pushed).

## Why the revert

Each fix round added classification state and traded one failure mode for
another; the critical-finding count rose (2, 2, 2, 3, 5) instead of falling.

| Round | Approach | What broke |
|---|---|---|
| 1 | prefix guards (`"+++ "` vs `"+++"`) | dead code — outer `if` consumed the lines first; SQL `-- comment` still dropped |
| 2 | `in_hunk` state, closed on `diff --git` | `git show --cc`, `diff -ru`, svn `Index:` collapsed to one file |
| 3 | close on marker set (` +-\`) | plain POSIX `diff -u` collapsed (no separator line exists); mbox `---` counted as a removal |
| 4 | `@@ -a,b +c,d @@` line budget, trusted | stale counts silently dropped trailing changes / swallowed the next file |
| 5 | budget + mismatch-detect → raw fallback | 38% of real `git format-patch` output false-fell-back (unindented commit-message bullets are `-` lines outside hunks); 4 silent-loss paths remained where exclusions preceded the detector |

## What is genuinely established

- The line budget is the right close mechanism for plain unified diffs; a
  separator list cannot work (round 4's `diff -u` case proves it).
- `@@@` combined headers need a distinct rule (second parent column).
- Mismatch → raw passthrough is the right *shape* of safety net, but detector
  order matters: value-exclusions (`---`, `-- `) and the header branches must
  not shadow it, and "before the first file header" is prose, not patch.
- Base itself has bugs the rewrite fixed: base drops `--- note`/`+++ note`
  content lines outright, renames files from `+++ text` content, and counts
  the `format-patch` `-- ` signature as a removal.

## Reproducers (all verified against real binaries)

1. `-- comment` removals (`--- note` on the wire): base drops them,
   counter under-reports. SQL/Lua/Haskell/Ada, mail sigs, Markdown `---`.
2. `+++ text` added line: base parses it as a file header — `[file]` renamed
   to user content.
3. Plain `diff -u` multi-file: any in_hunk close keyed on separators folds
   files together.
4. `git format-patch`: unindented body bullets are `-`/`+` lines outside all
   hunks — any "marked line outside a hunk = mismatch" rule must first skip
   the mbox prose region (before the first `+++ ` header).
5. Stale budgets both directions (hand-maintained patch queues, truncated
   streams): under-declared drops the tail, over-declared eats the next file.
6. `@@ -0,0 +0,0 @@`: close must be evaluated before classifying the next line.
7. `+++ /dev/null` (deletions): file renders as `/dev/null`.
8. Binary / rename-only / mode-only files: no output at all.
9. `--color` input: every line starts with ESC → empty output, exit 0
   (`strip_ansi` never called in `run_stdin`).
10. Non-UTF-8 stdin: hard error, no raw fallback (violates rust-patterns §4).
11. CRLF-only changes render as two identical lines (`lines()` strips `\r`).
12. `trim_start_matches("b/")` strips repeatedly (`b/b/x.rs` → `x.rs`);
    `diff -u` timestamps / svn `(working copy)` pollute the filename (split at
    first tab).
13. Savings: this filter measures ~8.6% by design (never truncates content);
    the 60% floor in cli-testing.md is written for truncating filters and
    needs a named exemption or a contract decision.

## Recommended next approach

Parse structure first, classify second: split the stream into
(mbox-prologue)(file-header)(hunks)* regions using the budget, with `@@@`
handled by its own region rule, and only then classify lines within regions.
Detector precedence written down as a total order before coding. Every fixture
must be captured from a real producer — four synthetic fixtures with
impossible hunk counts masked bugs for five rounds.
