# Conflicted Sync — 2026-09-03 (branch `sync/upstream-2026-09-03`)

Sync of `upstream/develop` (head `9cf048a`, tag `dev-0.48.0-rc.400`; 63 commits since the
2026-09-01 sync) into fork `develop`. Two conflicts: `src/cmds/git/diff_cmd.rs` and
`src/hooks/hook_cmd.rs`. Resolved on a topic branch via fork PR, per the Conflicted Sync
policy (CONTEXT.md): upstream wins on everything it covers, only proven-additive fork code
survives, and surviving code takes upstream's current shape.

Incoming upstream work that collided with fork code:

- **#3469 fix** — `0cb34ac` (Ben Younes, co-authored Luca Vitale) "don't report byte-different
  files as identical" plus `428af61` (KuSh) "pass the byte-equality verdict, not the file
  contents". `render_diff(file1, file2, &diff, bytes_equal)`, a `WHITESPACE_ONLY_DIFF_DETAIL`
  message for CRLF/trailing-newline-only differences, a new integration test
  `tests/diff_byte_accuracy_test.rs` that runs the real binary.
- **Hook-decision logging** (PR #3206 lineage: `0c952eb`, `56a2640`, `59e0a2d`, `37b8106`,
  `64c918d`, `c4b25d2`, `cf0bc20`; KuSh) — `PayloadAction::Skip { decision: HookOutcome }`
  replaces `Skip { reason: &str }`, and `process_claude_payload` is split into a thin
  extractor plus a pure `process_claude_payload_from_decision(v, cmd, decision)` core.

The other 61 commits auto-merged (Bun/Deno runtime support, `find` disclosure fixes, discover
coverage accounting, telemetry status labels, TOML `match_command` anchoring, tracking
migrations). No change to `Cargo.toml`, `build.rs`, or `Cargo.lock` in the range.

## `src/cmds/git/diff_cmd.rs`

The fork carries the adoption of **still-open** upstream #3268 (Ilia Alshanetsky, `acbe88c`,
refreshed by fork PR #153 to the PR's head `ecef036`). That PR does two things: (a) decide
identity on the bytes and name a difference `str::lines()` cannot see, and (b) align by
Myers instead of comparing line N to line N, with listing budgets and a `Replaced` change
kind. Upstream has now solved (a) its own way. Under the Conflicted Sync rule, (a) is a
duplicate and goes; (b) is genuinely additive and stays, reshaped onto upstream's code.

Proof that (b) is additive, from `git show upstream/develop:src/cmds/git/diff_cmd.rs`:
`grep -n 'unaligned\|LCS\|Myers\|Replaced\|positional'` is blank, and upstream's
`compute_diff` (L282) is still the positional comparison.

### Taken from upstream verbatim

- Imports (`use crate::core::guard::never_worse;`), `IDENTICAL_FILES_MESSAGE`,
  `WHITESPACE_ONLY_DIFF_DETAIL`.
- `run()` body: `compute_diff` → `format_classic_diff` → `render_diff(.., content1 == content2)`
  → `select_file_diff_output(&diff, &fallback, &rtk)` → `tracking_baseline(&diff, ..)`.
- `render_file_header`, `render_diff`'s identity / whitespace branch, `tracking_baseline`,
  `select_file_diff_output` (the fork's `FileComparison`-shaped versions of the last two are
  gone).
- The whole `#3469` test block: `test_render_crlf_vs_lf_not_identical`,
  `test_render_trailing_newline_not_identical`, `test_render_byte_identical_exit_zero_with_crlf`,
  `test_never_worse_fallback_is_a_classic_diff`, `test_tracking_baseline_never_books_a_loss`,
  `test_tracking_baseline_identical_files_use_both_files`,
  `test_tracking_baseline_empty_files_do_not_book_a_loss`,
  `test_identical_files_keep_the_success_message`,
  `test_classic_diff_covers_modified_line_boundary_cases`; the `render_test_diff` test helper;
  the four `render_diff (issue #2364 regression)` tests in upstream's call shape.
- `tests/diff_byte_accuracy_test.rs` (new, auto-merged; `tempfile` was already a dev-dependency).
- `src/core/guard.rs` is byte-identical to upstream again (`INVISIBLE_DIFF_TOKEN_ALLOWANCE`
  and its module doc removed with their only caller).

### Fork work removed (superseded by upstream)

From the #3268 adoption, the identity half: `FileComparison`, `compare_files`,
`classic_fallback`, `invisible_message_affordable`, `file_pair_header`,
`crlf_line_numbers`, `describe_invisible_difference`, and the fork-shaped
`tracking_baseline` / `select_file_diff_output`. Tests dropped as duplicates of upstream's
block: `test_render_crlf_difference_is_not_identical`,
`test_render_trailing_newline_difference_is_not_identical`,
`test_render_byte_identical_is_identical`, `test_render_partial_crlf_matches_reported_repro`,
`test_invisible_difference_is_not_reported_as_identical`, and the fork copies of the five
baseline/guard tests that carry the same names as upstream's. Tests dropped because their
subject no longer exists: `test_crlf_line_numbers_ignores_an_unterminated_tail`,
`test_invisible_message_affordability_ignores_path_length`,
`test_invisible_message_affordability_still_has_a_ceiling`,
`test_describe_invisible_difference_never_prints_equal_byte_counts`.

User-visible consequence: a CRLF-only or trailing-newline-only difference now prints
upstream's `files differ only in whitespace or line endings (no line-content change)` instead
of the fork's `differs, text matches (line endings: 0 CRLF vs 2 CRLF)`. Exit code is 1 on
both sides. The fork's affordability guard on that message is gone with it — upstream shows
the message whenever the change list is empty and the bytes differ, and its integration test
pins that.

### Fork work surviving (additive), and how it was re-seated

- `compute_diff` (Myers, `MAX_TRACE_CELLS`, positional fallback, `POSITIONAL_*_CAP`),
  `DiffResult { unaligned, positional }`, `Unaligned`, `DiffChange::Replaced`, `frame_legend`,
  and #3268's `format_classic_diff` (both-frame hunk headers) — all auto-merged, untouched.
- The three `Unaligned` refusal arms of #3268's `render_diff` now sit inside upstream's
  `if diff.changes.is_empty()` branch, ahead of the `bytes_equal` check. A refusal produces an
  empty change list with differing bytes, so without this ordering upstream's whitespace
  message would describe a 60,000-line difference. Header calls renamed to upstream's
  `render_file_header`. The tail of `render_diff` also carries #3268's `positional` note and
  `frame_legend` push (auto-merged; both absent upstream). Those are the only places
  upstream's function is not verbatim.
- Region parser for `condense_unified_diff` (fork PR #128; upstream #3788, open) auto-merged.
  `use anyhow::{Context, Result}` kept for `condense_stdin`, as in PR #153.
- Test helpers: `changes_of` now computes the diff directly (it used to unwrap
  `FileComparison::Lines`); the fork's strict/passthrough `condense_unified_diff` test wrapper
  kept. Fifteen #3268 alignment tests that called `render_file_diff(Path::new(a), Path::new(b), ..)`
  now call upstream's `render_test_diff(a, b, ..)`; assertions unchanged.
  `test_over_cap_comparison_is_not_reported_as_identical` drives upstream's
  `render_diff` / `select_file_diff_output` directly; assertions unchanged.

## `src/hooks/hook_cmd.rs`

Upstream's split is taken verbatim: `process_claude_payload` extracts the command and calls
`process_claude_payload_from_decision(v, cmd, decision)`; the fork's
`process_claude_payload_impl(v, decide: impl Fn)` injected-closure shape is gone, and the four
fork tests that drove it (`test_claude_already_rtk_passthrough`,
`test_claude_payload_asserts_allow_for_already_rtk_command`,
`test_claude_payload_asserts_deny_for_already_rtk_command`,
`test_claude_payload_original_form_deny_stays_skip`) now pass the decision directly, with
`Skip { decision: HookOutcome::Defer / Deny }` in place of the removed `reason` strings.

Fork code kept, each proved absent upstream with `git show upstream/develop:src/hooks/hook_cmd.rs | grep`:

- `claude_payload_input` — accepts the current `input` key as well as `tool_input`
  (`c8d6e28` 李冠辰, fork proof `claude_hook_rewrites_current_input_key_shape`). Upstream reads
  only `/tool_input/command`. `process_claude_payload` extracts through it, and
  `process_claude_payload_from_decision` builds `updatedInput` through it instead of
  upstream's `v.get("tool_input")`, so the sibling-field preservation the proof test checks
  still holds for the `input` shape.
- The already-rtk `PayloadAction::Deny` assertion and `contains_already_rtk_segment`
  (#3195 Chris Brown `30557a6` + fork amendments `8adbeba`..`31fceff`; upstream #3195 open).
  `run_claude`'s `Deny` arm now writes stdout first and then calls upstream's new
  `log_hook_decision(.., HookOutcome::Deny, None)`, matching the `Rewrite` / `Skip` arms —
  the fork's deny path would otherwise have been the one decision missing from
  `hook_decisions` (review finding).
- The `permission_mode`-aware `"ask"` decision (`e93cde8` hth; proof
  `claude_hook_emits_ask_decision_for_default_verdict`). Upstream has no `permission_mode`.
- Test helper `claude_current_input_with_fields`. Upstream's new helpers
  (`claude_payload_with_ids`, `claude_input_value`) and its `hook_log_fields` /
  decision-matrix tests sit beside it; both sides of that hunk were additions at the same spot.

## Repro, real binaries

Before = installed `~/bin/rtk.exe` (fork `develop` at `f69375a`); after = this branch's
release build. `CLAUDE_CONFIG_DIR` sandboxed to a scratch dir with
`{"permissions":{"deny":["Bash(rm:*)"],"allow":["Bash(grep:*)"]}}`; `RTK_DB_PATH` scratch.

```
$ rtk diff lf.txt crlf.txt            # "alpha\nbeta\n" vs "alpha\r\nbeta\r\n"
before: lf.txt → crlf.txt
        differs, text matches (line endings: 0 CRLF vs 2 CRLF)          exit 1
after:  lf.txt → crlf.txt
        files differ only in whitespace or line endings (no line-content change)   exit 1

$ rtk diff plain200.txt plain200_ins.txt   # 200 lines, one line inserted after line 100
before: 100a101 / > INSERTED                                             exit 1
after:  100a101 / > INSERTED                                             exit 1
        (Myers alignment kept: upstream's positional compute_diff would list ~200 changes)

$ rtk diff big_a.txt big_b.txt         # 60,000 lines each, nothing in common
before: 60000 lines differ, too many to list; use `rtk proxy diff` for the full text   exit 1
after:  identical (the refusal arm answers before upstream's whitespace branch can)

$ rtk hook claude < {"tool_name":"Bash","tool_input":{"command":"rtk rm -rf /tmp/x"}}
before: {"hookSpecificOutput":{..,"permissionDecision":"deny","permissionDecisionReason":"RTK: matches a configured deny rule"}}
after:  identical

$ rtk hook claude < {"tool":"Bash","input":{"command":"git status","timeout":30000}}
before: {"hookSpecificOutput":{..,"updatedInput":{"command":"rtk git status","timeout":30000},"permissionDecision":"ask"}}
after:  identical
```

## Review round

Independent read-only reviewer pass over the resolution against this writeup, function by
function versus `upstream/develop` and `develop`. All nine verification items passed (no
markers or mis-encoded text; upstream regions byte-identical apart from the documented
`render_diff` additions; every empty-change-list path exits and reports correctly; region
parser byte-identical to `develop`; every additive claim confirmed by a blank grep on
upstream's copy; dropped/kept test lists exact). Two findings acted on:

1. **`run_claude`'s `Deny` arm skipped `log_hook_decision`** — fixed as described above.
2. **Non-rustfmt-shaped call sites** among the converted tests. `cargo fmt` never visits
   `src/cmds/**` (`automod::dir!`), so the file is not rustfmt-clean on `develop` (22 sites)
   or upstream either. rustfmt's reflow was applied only to lines this sync introduced,
   with upstream's verbatim test blocks and helper excluded; `rustfmt --check` now reports
   20 sites, all pre-existing.

Nits taken: the `render_diff` comment no longer says "invisible difference" (removed fork
vocabulary); the test section header that used to cover the baseline/guard tests now names
what is left under it. Declined: removing the `;` after the deny arm's `return
PayloadAction::Skip {..}` — rustfmt requires it there because an `if` block precedes the
return, unlike upstream's arm. Noted, not changed: `claude_payload_input` runs twice per
rewrite (extractor and `updatedInput`); one JSON lookup, harmless.

## Quality gate

x64 host toolchain (`scripts/win-dev-env.ps1`): `cargo fmt --all` clean · `cargo clippy
--all-targets` 0 warnings · `cargo test --all` 3233 unit tests passed, 0 failed, 8 ignored; all
integration suites green, including upstream's new `diff_byte_accuracy_test`. `git diff --check`
clean. No duplicate test names in either file.

## Fork Delta

`scripts/fork-delta.sh --check` is current at 58 fixes; no `HEALED_SHAS` entry added.
`acbe88c` still carries the alignment half of #3268, so it stays a real divergence.

## Deliberately not done

- **Upstream #3788 is now `CONFLICTING`** (`0cb34ac` / `428af61` touched `diff_cmd.rs` under
  it). Rebasing the `cla-fix` checkout onto `upstream/develop` for round 6 is #3788 work, not
  sync work.
- **Upstream #3268 is `CHANGES_REQUESTED` and will conflict on Ilia's next rebase** for the same
  reason. When the PR's head changes shape, the adoption refresh repeats (PR #153's own note).
  The identity half is expected to fold into upstream's `bytes_equal` there too.
- The `rules.rs` overrides recorded in the 2026-09-01 writeup are unchanged.
