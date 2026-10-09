# Conflicted Sync — 2026-10-09 (branch `sync/upstream-2026-10-09`)

Sync of `upstream/develop` (head `e4f0509`, tag `dev-0.51.1-rc.519`; 87 merges, 289 non-merge
commits since the 2026-09-11 sync) into fork `develop`. 27 conflicted paths. Resolved on a topic
branch via fork PR, per the Conflicted Sync policy (CONTEXT.md): upstream wins on everything it
covers, only proven-additive fork code survives, and surviving code takes upstream's current shape.

Incoming upstream work that collided with, or now constrains, fork code:

- **Edition 2024 + rustfmt `style_edition = "2024"`** (`Cargo.toml`, new `rustfmt.toml`), let-chains
  everywhere. `cargo fmt` re-shaped 11 fork-touched files; fork code nested `if let` collapsed into
  let-chains where it was edited.
- **`build.rs`** (new): concatenates `src/filters/*.toml` (as before), adds a Windows
  `/STACK:8388608` link arg for the main thread, and fails the build on any `src/cmds/*/*.rs` not
  declared in its `mod.rs` (replaces the dropped `automod` crate). Read in full before building:
  local files only, no network, no process spawns. Dependencies: `dirs` 5→7, `automod` removed,
  dev-dep `temp-env` 0.3 added.
- **Shared arg tokenizer** (#3681 and the `*-arg-tokenizer` follow-ups #3999–#4008): git, grep/rg,
  ls, mvn, gh, glab, show migrate onto `core::arg_tokenizer`. `.claude/rules/rust-patterns.md` now
  makes it mandatory for new arg handling.
- **Tracked-command quoting** (#4403): labels built with `display_args`; `label_scan` test refuses
  `args.join(" ")` labels.
- **Test isolation** (#3758, #4460): `core::test_isolation` refuses any test file that spawns
  `CARGO_BIN_EXE_rtk` directly; `tests/common::rtk_command()` redirects the child's data.
- **`src/hooks/init.rs` split** into `src/hooks/init/*.rs` (`5ec8b8c`), and Claude hook presence
  rebuilt on matcher-aware `hook_present`/`group_covers_tool`.
- **`--head-lines`** for `rtk read`, and `head -N` → `rtk read <f> --head-lines N` in the registry.
- **Capped stderr forwarding** for stdout-only filters (#3772 + `d0535e2`).
- `hook_cmd::pre_tool_use_rewrite_output` extracted; new Codex PreToolUse hook shares `PayloadAction`.
- `opencode` plugin talks JSON to a new `rtk hook opencode` (#4323, #4349).

## Resolutions, file by file

**Upstream verbatim** (fork side was a rewording or a superseded fix):

| Path | Why |
|---|---|
| `.claude/rules/{cli-testing,rust-patterns,search-strategy}.md` | fork side only condensed upstream's text |
| `hooks/opencode/rtk.ts` | upstream's JSON protocol supersedes the fork's `--` (help text now fails `JSON.parse` and passes through) |
| `src/cmds/system/read.rs` | fork `653fb56`/`34bc096` (`head_truncate` for `--max-lines`) superseded by upstream's `--head-lines`; `--max-lines` is `smart_truncate` again |
| `src/core/filter.rs` | fork side only deleted `smart_truncate`, which `read.rs` uses again |
| `src/cmds/system/search.rs` | fork `00d8ee4`/`d43cef5` (`strip_rg_replace`) superseded by #3681: rg's `-r` is parsed as `--replace` and forwarded faithfully. Upstream closed the source PR #3162 in favour of #3681 |

**Merged — upstream's shape, additive fork behaviour kept** (each proved absent upstream with
`git show upstream/develop:<file> | grep <symbol>` → nothing):

| Path | Fork code kept | Upstream status |
|---|---|---|
| `.github/workflows/{cd,release}.yml` | `continue-on-error` App-token fallback + its comment, on upstream's SHA-pinned action | fork infrastructure |
| `hooks/{claude,cursor}/rtk-rewrite.sh` | `--` before `"$CMD"`, on upstream's `env -u RTK_REWRITE_HOST` line | upstream dropped `--` in `c6f484f`; `rtk rewrite --help` prints clap help with exit 0 again (verified below), so the fork's guard is additive again and `7db14a3` leaves `HEALED_SHAS` |
| `src/cmds/git/git_cmd.rs` | `requests_help` passthrough, now placed after upstream's double-dash/stash restore | upstream #3944 closed for #4348, **still open** |
| `src/cmds/js/{playwright,pnpm,vitest}_cmd.rs` | `passthrough_warning_reason` import | fork-only |
| `src/cmds/js/prettier_cmd.rs` | combined-stream, exit-aware filter (`run_filtered_with_exit`), upstream's `display_args` | fork-only (#2878) |
| `src/cmds/js/tsc_cmd.rs` | informational passthrough, now via `display_args` | fork-only |
| `src/cmds/js/vitest_cmd.rs` | `strip_jest_conflicting_args` inside upstream's `jest_invocation` (replaces `should_skip_jest_arg`, which still leaks `--reporters default`'s value as a test filter); exit-code-aware passthrough warning on `run_framework_test` | fork-only |
| `src/cmds/rust/cargo_cmd.rs` | compiler-warning count on passing runs, in upstream's let-chain | fork-only |
| `src/core/runner.rs` | `requests_help` + its `run_inner` guard, beside upstream's `forwarded_stderr` | #4348 open |
| `src/discover/rules.rs` | git rule without `yadm`, with upstream's new terminator group | upstream #3414 open |
| `src/hooks/hook_check.rs` | `touch_warn_marker` (non-empty payload) + its two tests | upstream still writes `b""` |
| `src/hooks/hook_cmd.rs` | `claude_payload_input` (either key), `contains_already_rtk_segment`, permission-mode-aware `"ask"`; upstream's `pre_tool_use_rewrite_output` now reads input through `claude_payload_input` (identical for every caller that has `tool_input.command`) | fork-only |
| `src/hooks/permissions.rs` | `deny_or_ask_matches`: the rtk-prefix see-through (`segment_matches`) composed with upstream's `strip_grammar_residue`, deny/ask only | fork-only |
| `src/hooks/init.rs` → `init/{claude,mod,pi}.rs` | PowerShell matcher ported: `CLAUDE_HOOK_MATCHERS`, `insert_claude_hook_entries` (Claude only — upstream's `insert_hook_entry` also serves Codex and stays Bash-only), `missing_hook_matchers`, three-state `--show`; matcher checks reuse upstream's `claude_group_covers` logic, so `Bash|PowerShell` counts for both. Three fork Pi-plugin hashes re-added to `KNOWN_PI_PLUGIN_HASHES` | upstream #2075 closed unmerged; no equivalent |
| `src/main.rs` | `run_fallback` keeps `args_os` (non-UTF-8 safe) and feeds it to upstream's `child_args`; `forward_help_to_wrapped_tools` beside upstream's `split_leading_negations` | fork-only / #4348 open |
| `tests/guard_integration_test.rs` | `git_diff_external_driver_output_is_not_dropped` beside upstream's new routing tests | upstream #3607 open |
| `tests/stderr_only_failure_test.rs` (add/add) | upstream's file, with the fork's prettier variant of the no-invented-success test and the fork's help-forwarding tests appended | — |

## Consequential changes outside the conflict hunks

- Removed with the superseded code: `diff_args_request_machine_output` and its 3 tests (upstream
  routes `--name-only/--name-status/--numstat/--raw` raw via `log_wants_raw_shape`, word-diff
  porcelain via `emits_word_diff`, and prints the raw path verbatim); `tests/rg_replace_fidelity_test.rs`.
- `PayloadAction::Deny` arm added to upstream's Codex hook match (never constructed there today;
  mirrors the Claude arm). Fork test pattern on `PayloadAction::Skip` gains `..` for upstream's
  new `reason` field.
- `permissions::is_rtk_prefixed` now strips grammar residue, like `deny_or_ask_matches`. Found in
  review: after composing the fork's rtk see-through with upstream's `strip_grammar_residue`,
  `true && { rtk rm -rf /x; }` computed a Deny, but the gate that asserts it
  (`contains_already_rtk_segment`) didn't see through `{`. The hook skipped, and Claude's native
  check doesn't know `rtk rm` is `rm`. Deny-only, so it can only make the hook stricter. The new
  payload test failed before the fix.
- Fork passthrough labels in `run_log`/`run_status` moved to `with_args` + `display_args` +
  `tracking::passthrough_label` (upstream's `label_scan` guard).
- 11 fork-only integration tests moved from `Command::new(env!("CARGO_BIN_EXE_rtk"))` to
  `common::rtk_command()`; `git_machine_output_test` also isolates its native git with
  `common::isolate_git` so both sides read the same config.
- Upstream tests updated for fork routing: `pnpm -r lint` is a package script under the fork's
  lint routing (upstream #3289, open), so it rewrites to `rtk pnpm -r lint` like `pnpm -r install`.
  `test_rewrite_pnpm_recursive_lint_safe_noop` → `…_routes_to_rtk_pnpm`; the case moves from the
  Unsupported list to the Supported one.
- `.cargo/config.toml` (new, fork-only): `RUST_MIN_STACK = 8 MiB` for test threads. Windows
  debug test threads overflowed their 2 MiB stack in clap parse tests once fork and upstream
  subcommands combined; upstream's own `build.rs` reserves 8 MiB for the binary's main thread.

## Repro (real binaries)

`before` = installed `~/bin/rtk.exe` (pre-sync fork develop); `after` = release build of this branch.

```
===== before: rtk 0.48.0
  rewrite "head -20 f.txt"   -> exit 3  rtk read f.txt --max-lines 20
  rewrite "yadm status"      -> exit 3  rtk yadm status
  rewrite "pnpm -r lint"     -> exit 1
  rewrite "git log | wc -l"  -> exit 1
  rewrite "git status"       -> exit 3  rtk git status
  rewrite --help           -> exit 0
  rewrite -- --help        -> exit 1
  hook claude (default mode): "permissionDecision":"ask"
  init -g --auto-patch matchers: "matcher": "Bash" "matcher": "PowerShell"
===== after: rtk 0.49.0
  rewrite "head -20 f.txt"   -> exit 3  rtk read f.txt --head-lines 20
  rewrite "yadm status"      -> exit 3  rtk yadm status
  rewrite "pnpm -r lint"     -> exit 3  rtk pnpm -r lint
  rewrite "git log | wc -l"  -> exit 1
  rewrite "git status"       -> exit 3  rtk git status
  rewrite --help           -> exit 0
  rewrite -- --help        -> exit 1
  hook claude (default mode): "permissionDecision":"ask"
  init -g --auto-patch matchers: "matcher": "Bash" "matcher": "PowerShell"
```

- `head -N` takes upstream's `--head-lines` (the fork's `--max-lines` fix is healed).
- `rewrite --help` exits 0 with clap help on both, so the hook scripts still need their `--`.
- `pnpm -r lint` is new: upstream's pnpm global-option strip (#3275) meets the fork's
  package-script routing. Checked with a fake `pnpm.cmd` that echoes its argv: `rtk pnpm -r lint`
  forwards exactly `-r lint` and the script output reaches the user, exit 0, on both binaries.
- `yadm` routes to the `yadm.toml` filter (the yadm binary), never git, on both.
- Pipeline producers stay raw (`git log | wc -l` exit 1); the permission-mode `"ask"` and both
  Claude matchers survive the `init/` split.

Quality gate: `cargo fmt --all` clean, `cargo clippy --all-targets` 0 warnings,
`cargo test --all` 0 failures (Windows x64 host toolchain); `cargo build --release` ok.

## Not re-audited

Fork code that auto-merged outside any conflict hunk was not swept for new upstream duplicates,
same scope as earlier syncs. One known candidate: `status_args_request_machine_output` /
`log_args_request_machine_output` (fork `46d16ea`, `da407dc`). Their diff sibling turned out to
be superseded here; the status and log halves may be too.

## Fork delta and claims

- `HEALED_SHAS`: `653fb56`, `00d8ee4`, `d43cef5` added; `7db14a3` removed (see hook scripts above).
- `.github/fork-claims.tsv`: both `rg_*` claims withdrawn with their code;
  `failing_tool_stderr_reaches_the_user` moved to healed (upstream merged #3772).
- The proof table in `.github/README.md` is generated by `fork-proof.yml` (Linux only) — run it
  after merge.
