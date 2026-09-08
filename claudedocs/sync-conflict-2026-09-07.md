# Conflicted Sync — 2026-09-07 (branch `sync/upstream-2026-09-07`)

Sync of `upstream/develop` (head `ab0cf40`, tag `dev-0.49.0-rc.410`; 42 commits, 34 non-merge,
since the 2026-09-05 sync) into fork `develop`. Three conflicts: `src/cmds/git/diff_cmd.rs`,
`src/cmds/js/lint_cmd.rs`, `src/discover/rules.rs`. Resolved on a topic branch via fork PR, per
the Conflicted Sync policy (CONTEXT.md): upstream wins on everything it covers, only
proven-additive fork code survives, and surviving code takes upstream's current shape.

Incoming upstream work that collided with, or now constrains, fork code:

- **#3788 merged** (`ab0cf40`) — the fork's own `condense_unified_diff` region-parser rework,
  rounds 1 through 8, under the fork author's name. The fork's `develop` carried rounds 1-4 as
  its own commits plus the #3268 adoption interleaved with them; upstream now has rounds 1-8.
- **Pipeline safety** (#3343, KuSh: `4f54b82`, `2a30415`, `dd914c2`, `148e0d4`, `ceb1d17`) —
  `RtkRule` gains `pipeline_safety: PipelineSafety`, and a pipeline *producer* is rewritten
  when every consumer is display-only (`head`, `tail`, `cat`, minus the following forms of
  `tail`). The hint text and the awareness file were slimmed alongside.
- **sqlfluff** (#253, TheGlitching: `b605716`, `f03d643`, `0db62bd`, `ebf4cbf`) — `rtk sqlfluff
  lint` and a `sqlfluff` arm in `lint_cmd::run` that plans its own argv.
- **Awareness levels** (`aefea91`, `bc75192`, `cddcecd`, `3e5eaac`, `4bbf778`, `c6f83ae`,
  `d667f21`, `6c441a7`) — `awareness.level` config, one neutral awareness file per level under
  `hooks/`, per-agent awareness files removed.
- **Upstream's git rule still lists `yadm`** (`(?:git|yadm)`, `rewrite_prefixes: ["git",
  "yadm"]`) — upstream #3414 / #3408 remain open.

The rest auto-merged. No change to `Cargo.toml`, `build.rs`, or `Cargo.lock` in the range
(`git diff --stat develop upstream/develop -- Cargo.toml Cargo.lock build.rs` is blank).
`hooks/pi/rtk.ts` did not move on either side, so the Pi hash allowlist needed nothing;
`test_all_git_pi_plugin_revisions_are_allowlisted` passes.

## `src/cmds/git/diff_cmd.rs`

**Upstream's file verbatim** in the merge commit (`git checkout --theirs`). Eight conflict
regions, every one of them the fork's rounds 1-4 (now upstream's rounds 1-8) interleaved with
the alignment half of the #3268 adoption. Nothing fork-side in the file was additive to
upstream *and* separable from the rework, so the merge takes upstream's shape whole and the
still-open adoption is re-applied on top as its own commits:

| commit | author | content |
|---|---|---|
| `8ca2e0e` | Ilia Alshanetsky | cherry-pick of `d4239ec` (upstream #3268) — align by LCS and name the cause of an invisible difference |
| `091e4a5` | Ilia Alshanetsky | cherry-pick of `8073112` (upstream #3268 head) — number both files on a crossed rewrite pairing, and name the cap a refusal hit |

`091e4a5` applied clean. `8ca2e0e` had two adjacency conflicts, both resolved as the union:
the import block (`Context`, `Regex` from upstream's rounds; `HashSet` from the PR) and the
`Hunk` struct, which lands directly after upstream's `condense_stdin`. No line of the PR's own
was changed.

### Why the whole head, not the alignment half alone

The 2026-09-03 sync kept only the alignment half of #3268 because its identity half competed
with upstream's #3469 byte-equality verdict. That split no longer exists. Ilia's 2026-09-04
rebase narrowed the PR onto #3469 ("`develop` gained the byte-equality verdict independently,
so this branch no longer argues for it"): `compare_files` decides identity on `content1 ==
content2` first, exactly upstream's check; `IDENTICAL_FILES_MESSAGE` and `render_file_header`
are adopted from upstream; the invisible-difference message opens with upstream's "files differ
only in whitespace or line endings" wording and appends the cause. `FileComparison` is the
third state the aligner needs — a comparison that ran and then refused to build a listing —
which a `bytes_equal` bool cannot carry; routing a refusal through the identical branch would
exit 0 on files that differ. Cutting the enum out would be a fork-authored re-implementation
of the integration nobody upstream has reviewed, which is the "repair" the do-issue skill
forbids. Proof the adoption is additive, from `git show upstream/develop:src/cmds/git/diff_cmd.rs`:
`grep -n 'FileComparison\|Unaligned\|myers_ops\|frame_legend\|LCS'` is blank, and upstream's
`compute_diff` (L352) is still the positional comparison.

Files the PR touches outside `diff_cmd.rs`: `src/core/guard.rs` gains an 11-line module doc
naming the invisible-difference exception — **doc only, no code**;
`tests/diff_byte_accuracy_test.rs` keeps every upstream assertion (exit 1, whitespace message,
never "identical"), folds them into a helper, adds a one-line-pair case and an assertion that
the two-blob dump never replaces the explanation.

### Known residual (upstream #3268 is `CHANGES_REQUESTED`, KuSh, 2026-09-07 13:50Z)

The head adopted here carries one blocking finding: the frame legend describes `~` as numbered
in file 1, but a crossed rewrite pairing renders `~ N→M` (file1→file2); and the legend is
omitted when the crossing is the only pairing (`diff.added == 0`). Two non-blocking notes: the
"only in X: 1 line" subject clause under the byte cap is false for two near-identical single
lines, and the crossed-render tests assert below `render_diff` so no test sees the legend. All
three are wording/legend defects in a corner the positional fallback used to reach by a
different route; none loses content. The adoption refreshes when Ilia pushes the fix, the
same way PR #153 refreshed `ce07aff`/`54cb3a1` to `ecef036`.

## `src/cmds/js/lint_cmd.rs`

Upstream's sqlfluff restructure taken verbatim: `is_python_linter` includes `sqlfluff`, the
`sqlfluff_cmd::plan` call, the argv loop wrapped in `if let Some(plan)`, and the dispatch
`match` nested under `if let Some(plan) = &sqlfluff { plan.render(..) } else { .. }`. The
conflict was the fork's three `match` arms left dangling under upstream's new nesting; they
go, and the one surviving difference is the generic arm calling
`filter_generic_lint(&raw, result.success())`.

Two fork changes survive because upstream #3567 (WoldenMn) is still open and upstream has
nothing for either symptom (`git show upstream/develop:src/cmds/js/lint_cmd.rs | grep -n
'exists()\|succeeded'` is blank):

- `617ef8e` (adoption of #3567, fork PR #122): `detect_linter` treats a first argument that
  exists on disk as a path unless it names a linter the dispatch table knows. `known_linter`
  reads `is_python_linter`, so upstream's new `sqlfluff` keeps its linter meaning for free.
- `3240075` (fork amendment): `filter_generic_lint` hands back the bounded raw text on a
  non-zero exit instead of summarising a linter that never ran as "No issues found".

## `src/discover/rules.rs`

Upstream's new `pipeline_safety: PipelineSafety::ProducerOnly` line taken on the git rule. The
fork's `rewrite_prefixes: &["git"]` survives (`1d67272`, adoption of upstream #3414 by RawNuke,
fork PR #116): `yadm` stays out of the git rule and routes through `src/filters/yadm.toml` to
the yadm binary. Upstream still carries the bug (#3408 open; its rule rewrites `yadm commit`
to `rtk git commit` in whatever directory the shell stands in). The pattern line
(`^(?:git)\s+…`) auto-merged as the fork's, since upstream did not touch it.

The pre-existing fork overrides on this file (lint delegation to package managers, `cae0f21`,
recorded in the 2026-09-01 writeup) are unchanged.

## `src/discover/registry.rs` — one fork test removed

`test_rewrite_ls_pipe_skipped` asserted `ls -t | head -1` is never rewritten, with the comment
"rtk ls decorates names with size columns". It came from `e95207d` (XU Pengfei), which
`HEALED_SHAS` already records as superseded by upstream's `analyze_pipeline` redesign; the test
outlived its commit. Upstream's per-rule safety now marks `ls` a safe producer on purpose, so
the assertion encoded a design upstream replaced. Measured with the merged debug binary in
`src/cmds/`: `rtk ls -t | head -3` prints `git/`, `system/`, `python/` where `ls -t | head -3`
prints `git`, `system`, `python` — a trailing `/` on directories, no size columns
(`-l` adds a mode column, and `-l` is where a reader expects one). Upstream's call; the test
goes rather than being flipped into a duplicate of upstream's own producer tests.

The other fork-only tests in the file — yadm classification and rewrite (`#3414`), grep as a
pipeline producer staying raw, the `LC_ALL=`/`sudo`/`env`/absolute-path forms, lint delegation
(`cae0f21`) — all pass unchanged against upstream's new pipeline gate.

## Repro, real binaries

Before = `~/bin/rtk.exe` built 2026-09-07 16:10 from `develop` at `5809fb7`; after = debug
build of `091e4a5`. Fixtures in a scratch directory, `RTK_DB_PATH` scratch.

```
$ rtk rewrite -- "yadm status"
before: rtk yadm status            exit 3     after: rtk yadm status            exit 3
$ rtk rewrite -- "git log --oneline | head -3"
before: (no rewrite)               exit 1     after: rtk git log --oneline | head -3   exit 3
$ rtk rewrite -- "ls -t | head -1"
before: (no rewrite)               exit 1     after: rtk ls -t | head -1        exit 3
$ rtk rewrite -- "grep -n x f | wc -l"
before: (no rewrite)               exit 1     after: (no rewrite)               exit 1
$ rtk rewrite -- "npx eslint src"
before: rtk lint src               exit 3     after: rtk lint src               exit 3

$ rtk diff a.txt b.txt              # 12 lines, one line inserted after line 4
before: 4a5                                   after: +   5 inserted
        > inserted                  exit 1                                       exit 1
$ rtk diff lf.txt crlf.txt          # alpha/beta, LF vs CRLF
before: lf.txt → crlf.txt
           files differ only in whitespace or line endings (no line-content change)   exit 1
after:  files differ only in whitespace or line endings (line endings: 0 CRLF vs 2 CRLF)  exit 1
$ rtk diff lf.txt lf.txt
before: [ok] Files are identical   exit 0     after: [ok] Files are identical   exit 0
```

The two producer rewrites are upstream's #3343 behaviour arriving; grep stays raw because its
rule is `PipelineSafety::None`. The `diff` "before" is the classic fallback winning the
never-worse check against the old header-plus-counts render; the "after" is #3268's
headerless render, which is cheaper than `diff`'s own output for the first time.

## Quality gate

x64 host toolchain (`scripts/win-dev-env.ps1`): `cargo fmt --all -- --check` clean ·
`cargo clippy --all-targets` 0 warnings · `cargo test --all` exit 0 — 3430 unit tests passed,
0 failed, 8 ignored; every integration suite green, including upstream's new pipeline-safety
and sqlfluff tests and the PR's `one_line_crlf_vs_lf_diff_prints_whitespace_message`.

The first gate run, on the merge commit before the test removal above, failed exactly one test:
`test_rewrite_ls_pipe_skipped` (`assertion left == right failed`, got
`Some("rtk ls -t | head -1")`). That run is the unit-level "before" for the registry change.

## Fork Delta

`scripts/fork-delta.sh` regenerated: 51 fixes (the page said 61). Five fork commits healed
mechanically by patch-id — the preclear rounds 1-4 and review round 1 (`8243d1a`, `2219c6d`,
`25a17c0`, `fb5a3dc`, `5939842`), byte-identical to upstream's `f42eaf9`, `970b3b2`, `7689020`,
`eb088f2`, `1b11b52`. Seven new `HEALED_SHAS` entries: `5a61595` (merged as `ee614e4`,
rebased so the patch-id differs), the five pre-region-parser hunk fixes it subsumed
(`1f64783`, `8e65037`, `89cfe86`, `5bcee1c`, `6375941`), and `acbe88c` (the `ecef036` snapshot
of #3268, now counted once through `8ca2e0e`/`091e4a5`).

## Review round

Read-only verification pass over the branch against this writeup, eleven items, all passed:
no conflict markers or mojibake beyond upstream's own; `git diff 8a6658a upstream/develop` on
`diff_cmd.rs`, `guard.rs` and the byte-accuracy test is blank; every sqlfluff symbol upstream
added to `lint_cmd.rs` is present; `rules.rs` carries the same 55 `pipeline_safety:` lines as
upstream; the `registry.rs` diff is test-only; the re-applied #3268 adds and removes exactly
the PR's own lines (the only set differences are lines present in both the added and removed
sets on one side — context that moved with upstream's rounds 6-8, not content); `guard.rs`
differs by `//!` lines only and the byte-accuracy test still asserts exit 1, the whitespace
message and never "identical"; the range is 42 commits, 34 non-merge, with no `Cargo.*` or
`build.rs` change; both cherry-picks carry `Ilia Alshanetsky <ilia@ilia.ws>`; the only files
newly divergent from upstream beyond `develop`'s existing divergence are this writeup,
`guard.rs` and the byte-accuracy test. A background reviewer subagent launched for the same
checklist stalled without reporting; the pass above was run directly.

## Deliberately not done

- **`rtk ls` as a pipeline producer** appends `/` to directory names, so `ls | head` now
  shows `git/` where `ls` showed `git`. Upstream chose `ProducerOnly` for the rule knowingly
  (the consumers it allows are display-only). Not filed upstream; noted so the next person
  who sees a trailing slash in piped `ls` output knows it is by design, not a fork defect.
- **Upstream #3268's open findings** are Ilia's to fix. The fork ships the head as reviewed
  on 2026-09-07 and refreshes on the next push; no fork-authored patch on top.
- **Upstream #3414 and #3567** remain open. Nothing to do here beyond keeping the adoptions.
- The `rules.rs` overrides recorded in the 2026-09-01 writeup are unchanged.
