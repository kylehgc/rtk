# Conflicted Sync — 2026-09-07, second (branch `sync/upstream-2026-09-07-b`)

Sync of `upstream/develop` (head `f749546`, tag `dev-0.49.0-rc.416`; 16 commits, 10 non-merge,
since the first 2026-09-07 sync at `ab0cf40`) into fork `develop`. One conflict:
`src/cmds/git/diff_cmd.rs`. Resolved on a topic branch via fork PR, per the Conflicted Sync
policy (CONTEXT.md): upstream wins on everything it covers, only proven-additive fork code
survives, and surviving code takes upstream's current shape.

Incoming upstream work:

- **#3268 merged** (`9512e1a`, Ilia Alshanetsky, via Nicolas Le Cam's `up/diff-correctness`
  merge `db98a06`) — `d4239ec` and `8073112`, which the fork already carried as the
  cherry-picks `8ca2e0e` and `091e4a5`, plus **`501dd01`**, the legend fix the previous
  writeup listed as the known residual ("the adoption refreshes when Ilia pushes the fix").
  It pushed, upstream merged it, and this sync is that refresh.
- **#3083** (Loi Nguyen, `bf2748b` + `afa9cae`) — `rtk ruff` preserves non-check subcommands;
  the subcommand list is pinned against ruff's CLI.
- **#2515** (Allan Batista, `b02b567` + `98e66e0` + `80e4568`) — `rtk test` treats `!` and
  `(` as native test expressions only when what follows is one; `Commands::Test` routing
  covered end to end in `tests/native_test_expression_test.rs`.
- **#560** (Florian BRUNIAUX, `cd9f1a9` + `c81c097`) — prompt-caching FAQ in the README and
  the published troubleshooting guide.

The rest auto-merged. No change to `Cargo.toml`, `build.rs`, or `Cargo.lock` in the range
(`git diff --stat develop upstream/develop -- Cargo.toml Cargo.lock build.rs` is blank).
`hooks/pi/rtk.ts` did not move in the range (`git log develop..upstream/develop --
hooks/pi/rtk.ts` is empty), so the Pi hash allowlist needed nothing;
`test_all_git_pi_plugin_revisions_are_allowlisted` passes.

## `src/cmds/git/diff_cmd.rs`

**Upstream's file verbatim** in the merge commit (`git checkout --theirs`; `git diff --cached
upstream/develop -- src/cmds/git/diff_cmd.rs` is blank). Eleven conflict regions, every one
the fork's cherry-picked #3268 head colliding with upstream's merge of the same head plus one
more round. Proof that nothing fork-side was additive:

- `git diff develop upstream/develop -- src/cmds/git/diff_cmd.rs` is content-identical to
  `git show 501dd01 -- src/cmds/git/diff_cmd.rs` once `@@` hunk headers are ignored — the
  entire fork-vs-upstream delta on the file is that one commit.
- `091e4a5` has the same stable patch-id as upstream's `8073112`
  (`d96fdce2ded93133fb858440054fc6da97c28ca1`). `8ca2e0e` differs from `d4239ec` only by
  the two adjacency resolutions recorded in the previous writeup (import block, `Hunk`
  placement), so its patch-id differs while its content does not.
- `src/core/guard.rs` and `tests/diff_byte_accuracy_test.rs`, the two files the #3268
  adoption touched outside `diff_cmd.rs`, auto-merged to byte-identical with upstream.

Both fork cherry-picks are therefore superseded. `091e4a5` drops out of the fork delta by
patch-id; `8ca2e0e` gets a `HEALED_SHAS` entry in `scripts/fork-delta.sh`.

The only file still divergent from upstream after the merge, beyond `develop`'s existing
divergence, is this writeup. `src/main.rs` differs from upstream as it did before the sync:
the fork's non-UTF-8 argv handling in `run_fallback` (`args_os`, `tests/non_utf8_argv_test.rs`)
and the `--` flag-injection test for `rtk rewrite` (#1350). Neither moved on either side.

## Repro, real binaries

Before = `~/bin/rtk.exe` built 2026-09-07 21:02 from `develop` at `a73d599` (carries
`091e4a5`); after = debug build of the merge commit `90d3121`. Fixtures in a scratch
directory, `RTK_DB_PATH` scratch. Exit code is 1 on every pair, both sides.

The crossed-rewrite fixtures from `501dd01`'s own unit tests (`a\nkey_1..key_3\ntimeout = 30\n
retries = 3\nz` against `a\nretries = 5\ntimeout = 60\nz`, and the insertion variant) print
the same classic listing on both binaries: `2,6c2,3` / `2,3c2,5` with `<`/`>` lines. The
classic fallback wins the never-worse check on a seven-line file either way, so the legend
never reaches the terminal there; the round's behaviour is asserted at `render_file_diff`
level by its two new tests (`test_render_frame_legend_names_the_crossed_shape`,
`test_render_frame_legend_separates_the_two_modified_shapes`), both green.

A fixture where the aligned render does reach the terminal — 40 `key_N = alpha` lines, every
third one changed to `beta`, with a swapped-and-changed `timeout`/`retries` pair between two
separator lines — shows the round's effect through the CLI:

```
$ rtk diff j.txt k.txt
before (12 lines, 464 bytes):            after (46 lines, 492 bytes):
~   3 key_3 = alpha → key_3 = beta       3c3
~   6 key_6 = alpha → key_6 = beta       < key_3 = alpha
…                                        ---
~  30 key_30 = alpha → key_30 = beta     > key_3 = beta
~  32→33 timeout = 30 → timeout = 60     …
~  33→32 retries = 3 → retries = 5       32,33c32,33
                                         < timeout = 30
                                         < retries = 3
                                         ---
                                         > retries = 5
                                         > timeout = 60
```

The before listing is the one KuSh flagged on #3268: crossed `~ N→M` rows with no legend
saying what the two numbers are. The after listing is `diff`'s own output, 492 bytes, because
`select_file_diff_output` runs `never_worse(fallback, rendered)` on a bytes/4 token estimate
and the legend line the round adds (`   (~ = file 1; ~ N→M spans both files)`, ~40 bytes)
lifts the aligned render from 464 bytes past the 492-byte classic listing. The output is
correct on both sides; the after side is longer on this shape. With the adjacent variant (no
separator line between `key_30` and the pair) the before build also printed a spurious
`~  30→30 key_30 = alpha → key_30 = beta` — a plain modification numbered as a crossing — and
the after build prints the classic listing for it too.

## Quality gate

x64 host toolchain (`scripts/win-dev-env.ps1`): `cargo fmt --all -- --check` clean ·
`cargo clippy --all-targets` 0 warnings · `cargo test --all` exit 0 — 3440 unit tests passed,
0 failed, 8 ignored; every integration suite green, including upstream's new
`native_test_expression_test` and the ruff subcommand pin. No test was changed or removed.

## Fork Delta

`scripts/fork-delta.sh` regenerated: 49 fixes (the page said 50). `091e4a5` dropped by
patch-id; `8ca2e0e` added to `HEALED_SHAS` with the evidence above. 11 healed by patch-id,
25 audited.

## Deliberately not done

- **The legend-versus-`never_worse` interaction** above is upstream's merged, reviewed
  behaviour and the fork ships it verbatim. The aligned render loses to classic `diff` by a
  margin of one legend line on shapes where it used to win by ~30 bytes. Not patched here —
  a fork-authored change to the guard or the legend would be a repair of upstream code nobody
  upstream has reviewed. If it is worth raising, it is an upstream issue (or a fork ticket
  that becomes an upstream PR), not a rider on a sync.
- **Upstream #3414 and #3567** remain open; the `rules.rs` and `lint_cmd.rs` adoptions
  recorded in the earlier writeups are unchanged and were not in this range.
