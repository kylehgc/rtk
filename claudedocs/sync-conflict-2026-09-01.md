# Conflicted Sync — 2026-09-01 (branch `sync/upstream-2026-09-01`)

Sync of `upstream/develop` (head `9a695d1`, release 0.47.0; 39 commits since the
2026-08-31 sync) into fork `develop`. Four conflicts: `src/cmds/git/git.rs`,
`src/cmds/git/diff_cmd.rs`, `src/discover/registry.rs`, `docs/usage/FEATURES.md`.
Resolved on a topic branch via fork PR, per the Conflicted Sync policy (CONTEXT.md).

Incoming upstream work that collided with fork code:

- **#3269** (iliaal, merged) — `git diff`: hunk lines at column 0, `emits_word_diff`
  passthrough, byte-sliced markers, combined-diff parents, singular truncation note.
- **#3048** (mvanhorn, merged) — `rtk diff`: classic-diff fallback instead of dumping
  both files (`format_classic_diff`, `tracking_baseline`, `select_file_diff_output`).
- **#3324** + **#3749** + **#3803** (katspa-sc / KuSh, merged) — hooks: `exclude_commands`
  honoured under routable wrappers and matched against the peeled/tool form (`tool_form`).

## Fork work removed (superseded by upstream)

- **`75ec765` Arif Waram — "keep +/-/@@ diff markers at column 0 in compact_diff"**, and
  the fork's `(+N -M)` per-file tally spelling. Upstream #3269 solves the same column-0
  bug structurally (hunk-budgeted parser, `  +N -M` tally kept indented so `^[+-]` never
  matches it). Both `compact_diff` conflict hunks resolved to upstream's side verbatim;
  the fork's `test_compact_diff_keeps_diff_markers_at_column_zero` was dropped as a
  duplicate of upstream's `tally must stay indented` / column-0 tests. Note: the fork's
  own upstream PR #3788 had already dropped its `git.rs` companion commit in favour of
  #3269 (round 2, 2026-08-31), so this removal matches what was already sent upstream.
- **`6871925` Anay Garodia — "apply exclude_commands to the resolved tool"** (#3035)
  in `registry.rs`. Upstream now covers the `python -m pytest` form via `tool_form()`;
  both conflict hunks resolved to upstream's side. Of the fork's three `#3035` tests,
  `test_exclude_covers_python_m_form` is a literal subset of upstream's
  `test_exclude_covers_interpreter_and_path_forms` and was dropped; the word-boundary
  and other-tool tests have no upstream counterpart and were kept.
- **`22ee0d4` (fork docs)** — the `rtk git diff` example in `FEATURES.md`. Replaced by
  upstream's rewritten section, which documents the same column-0 guarantee plus the
  three anchored-audit caveats. `FEATURES.md` is now byte-identical to upstream.

Healed without a conflict (auto-merged, but upstream now has its own fix, so the
Fork Delta must stop listing them — recorded in `scripts/fork-delta.sh` `HEALED_SHAS`):

- **`8a27c09` Husam — "apply exclude_commands to head/tail rewrites"**: upstream `2e0dd29`
  (Katspa, #3324) adds the same `is_excluded(cmd_part, excluded)` check on the head/tail
  fast path. Different patch-id, same fix; the fork's test `c8d0115` still passes.
- **`b008c53` (fork) — "keep +/- markers at column 0 in condense_unified_diff"**: upstream
  now has `test_condense_unified_diff_markers_at_column_0` and emits change lines at
  column 0 itself. The fork's region parser (#3788) supersedes it on our side as well.

## Fork work surviving (verified additive)

Each verified with `git show upstream/develop:<file> | grep -n <symbol>` → no match.

- **`ce07aff` + `54cb3a1` Ilia Alshanetsky — byte-equality identity check and LCS
  alignment in `rtk diff`** (adoption of upstream **#3268, still open**). Upstream's
  `compute_diff` is still positional. Kept, and re-seated under #3048's new `run()`
  shape: `run()` computes the diff once (upstream's flow, needed for the classic-diff
  fallback and tracking baseline), then a thin `render_file_diff(file1, file2,
  content1, content2, &diff)` wrapper applies the byte check and the
  `describe_invisible_difference` branch before handing off to upstream's
  `render_diff`, which is verbatim. `format_classic_diff` / `tracking_baseline` /
  `select_file_diff_output` are upstream's, untouched.
- **Region parser for `condense_unified_diff`** (fork PR #128; upstream **#3788**, open).
  Auto-merged with no conflict; `run_stdin` / `condense_stdin` unchanged.
- **`46d16ea` 李冠辰 + `da407dc` fork amendment — machine-output passthrough** for
  `git diff/log/status` (upstream **#2573**, still open). `wants_machine_output` folded
  into upstream's new `wants_compact && !emits_word_diff(args)` condition.
- **`1d67272` RawNuke — bare `git add` no-op and `yadm` never rewritten to `rtk git`**
  (#3408; `append_add_pathspecs`, `rules.rs` drops `yadm` from the git rule) and
  **`cae0f21` Ousama Ben Younes — `npm/pnpm run lint` delegated to the package-manager
  filter** (`rules.rs`). Both auto-merged; upstream still routes `yadm status → rtk git`
  and `npm run lint → rtk lint`, so the fork flips upstream's own registry tests for
  those two cases. Pre-existing adoptions, listed so the override is on record.
- **`791359c` georgyia / `d88c1f4` kingpy-bot — `COMMIT_MARKER` leading marker** for
  `git log --stat` (#2882 lineage). Auto-merged, **but see Follow-ups**: upstream has since
  fixed #2882 its own way (`ca89767`, `requests_raw_log_output` routes `--stat`/`-p` and
  friends to raw passthrough), so this is now healed drift, not additive.

## Fork tests updated

- `diff_cmd.rs`: the four identity-check tests that called the old 4-arg
  `render_file_diff(path, path, content, content)` now go through a test-local
  `render_contents` helper that computes the diff first — same assertions, new plumbing.
  The section header comment reverted to upstream's wording where the two only differed
  by function name.
- `diff_cmd.rs` `test_condense_unified_diff_markers_at_column_0`: the anti-indent
  assertion restored to upstream's `trim_start` form (`fc2cc72`, KuSh). The fork's
  `starts_with(" +")` version from `6375941` had won the 2026-08-31 sync by accident and
  cannot fire on the two-space indent it guards against.
- `registry.rs`: three tests from healed `8a27c09` (`test_exclude_head_line_range_rewrite`,
  `test_exclude_tail_line_range_rewrite`, `test_head_still_rewrites_when_not_excluded`)
  dropped — upstream's `test_head_tail_honour_exclude_commands` /
  `test_head_tail_rewrite_when_not_excluded` cover the same cases. The fork-authored
  `c8d0115` tests (redirect suffix, raw-regex pattern on the head/tail path) stay.

## Follow-ups (not done here)

- **Fork-proof CI has been red since 2026-08-31** (before this sync): four claims now
  pass on upstream — `init_dry_run_tolerates_bom_prefixed_settings_json`,
  `tsc_pipe_filter_compresses_real_pretty_diagnostics`, `git_log_patch_preserves_diff_hunks`,
  `git_log_stat_keeps_every_commit`. Upstream healed the last two via `ca89767`
  (`requests_raw_log_output`), which also makes the fork's `COMMIT_MARKER` change dead in
  production. Dropping those four divergences (code, claims, `HEALED_SHAS`) is its own PR;
  mixing it into a conflict resolution would hide it.

- Upstream #3268 has been reworked substantially since the fork's snapshot (Myers
  alignment, `FileComparison` enum, listing budgets, `MAX_TRACE_CELLS`) and is still
  under `CHANGES_REQUESTED`. The fork carries the older shape. Refreshing that adoption
  is a separate ticket; this sync only kept it compiling against #3048.

Quality gate (x64 host toolchain, `scripts/win-dev-env.ps1`): `cargo fmt --all` clean,
`cargo clippy --all-targets` 0 warnings, `cargo test --all` 3085 unit tests passed / 0
failed / 8 ignored (3089 before the four duplicate tests were dropped), all integration
suites green.
