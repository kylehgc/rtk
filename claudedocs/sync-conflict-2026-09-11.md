# Conflicted Sync — 2026-09-11 (branch `sync/upstream-2026-09-11`)

Sync of `upstream/develop` (head `fc19d7a`, tag `dev-0.50.0-rc.427`; 95 commits since the
2026-09-07 sync) into fork `develop`. One conflict: `src/hooks/hook_cmd.rs`. Resolved on a
topic branch via fork PR, per the Conflicted Sync policy (CONTEXT.md): upstream wins on
everything it covers, only proven-additive fork code survives, and surviving code takes
upstream's current shape.

Incoming upstream work that collided with, or now constrains, fork code:

- **Hook decision consolidation** (#3955, KuSh: `d6ce8f7` extract, `6eb915b` route `rtk hook
  check` through it) — the per-host `decide_from_verdict` logic moves to a new
  `src/hooks/decision.rs`: `decide` / `decide_with_params` / `decide_for_agent`, plus
  `suppress_identity`, which turns a rewrite that changed nothing (an already-`rtk` command)
  into `Defer` for every hook. `HookDecision` moves there too. `hook_cmd::decide_from_verdict`
  becomes a four-line shim: deny short-circuit, `track_tee_read`, `decide_for_agent`.
- **Retriever recall** (#3278), **process-wrapper prefixes** (#3937), **git show large-blob
  preview** (#3265), **grep -l max-len collision** (#3258), **prisma migrate status panic**
  (#3836), **winget automation** (#1318), diff mutation-gap tests (#3923). All auto-merged.
- `Cargo.toml`: version `0.42.4` → `0.48.0` and a direct `toml_edit = "0.22"` dependency
  (already in `Cargo.lock` transitively via `toml`; the lock diff is the version line only).
  No `build.rs` change.

## `src/hooks/hook_cmd.rs`

Two conflict hunks, both inside `decide_from_verdict`. Both resolved to **upstream's side
verbatim**:

1. The fork's local `get_rewritten` helper and `enum HookDecision` — gone; upstream's
   `decision.rs` owns both.
2. The fork's already-rtk assertion arms inside `decide_from_verdict` — gone:
   - `contains_unattestable_construct` + already-rtk → `AskRewrite(cmd)` (fork `d9e7458`,
     broadened by `31fceff`);
   - `None if Allow && all_segments_already_rtk(cmd)` → `AllowRewrite(cmd)` (fork `30557a6`,
     `371e058`, wrapper exclusion `d9e7458`);
   - `None if Ask && contains_already_rtk_segment(cmd)` → `AskRewrite(cmd)` (fork `505e355`,
     `31fceff`).

   Upstream now has an explicit policy for exactly this case: `suppress_identity` documents
   the identity rewrite as a no-op the hook must not report, accepts the Gemini
   `ask_user` trade-off in its doc comment, and pins the contract in
   `tests/hook_decision_protocol_test.rs::decision_consistency`. The fork's arms are a
   different opinion on the same case, not an addition — they go.

Consequential removals outside the conflict hunks (dead or unreachable once the arms are gone):

- `all_segments_already_rtk` (hook_cmd.rs) and `permissions::is_wrapped_invocation`
  (`d9e7458`) — only callers were the removed Allow arm. `-D dead-code` flagged both.
- The `rewritten == cmd` early-return guards in the Copilot IDE, Vibe and Droid renderers
  (fork `31fceff`). They existed to silence the already-rtk assert shapes; `suppress_identity`
  now guarantees no renderer ever sees an identity rewrite. Duplicate of upstream, gone.
- Doc comments on `COMMAND_WRAPPERS`, `is_rtk_prefixed` and `contains_already_rtk_segment`
  trimmed of their references to the removed Allow gate and its quantifier note.

**Additive fork code kept**, each proved absent upstream by
`git show upstream/develop:src/hooks/permissions.rs | grep -c 'normalize_for_matching\|is_rtk_prefixed\|COMMAND_WRAPPERS'` → `0`:

- The rtk-aware permission matcher (`30557a6`, upstream #3195 by albatrossflyon-coder, still
  open; `8adbeba`, `505e355` amendments): `normalize_for_matching`, `is_rtk_prefixed`,
  `COMMAND_WRAPPERS` stripping, raw-or-stripped pattern matching. This is what makes the
  verdict for `rtk rm -rf /` or `rtk proxy rm -rf /` come back `Deny` against a `Bash(rm:*)`
  rule. It composes with upstream's `decide`: Deny is checked first, before
  `suppress_identity` can turn anything into `Defer`.
- The already-rtk `Deny` assertion in `process_claude_payload_from_decision` and
  `PayloadAction::Deny` (`30557a6`): the host's native matcher cannot see through the
  `rtk` prefix, so the deny is asserted rather than left to it.
- `claude_payload_input` either-key extraction (`c8d6e28`), the `permission_mode`-aware
  `"ask"` (`e93cde8`), the stdin read timeout thread, the empty-input fail-open guard in
  `run_gemini`. Untouched by the conflict; listed for completeness of the diff against
  upstream.

## `src/hooks/permissions.rs` (no conflict, follow-up edit)

`is_wrapped_invocation` and `test_is_wrapped_invocation_contract` removed with their only
caller. Two doc sentences reworded. Everything else is the fork's additive matcher, unchanged.

## Fork tests removed

All in `hook_cmd.rs`, all pinning the removed assert arms or the removed renderer guards:

`test_decide_allow_for_already_rtk_command`, `test_decide_defer_for_unrewritable_non_rtk_command`,
`test_decide_defer_for_mixed_rtk_and_raw_compound`, `test_decide_allow_for_all_rtk_compound`,
`test_decide_ask_rule_for_already_rtk_command_asserts_ask`,
`test_wrapped_invocations_never_allow_asserted`, `test_unattestable_already_rtk_asserts_ask`,
`test_probe_shapes_fail_safe` (its two `contains_already_rtk_segment` asserts moved into
`test_contains_already_rtk_segment_ignores_original_form`),
`test_copilot_ide_already_rtk_allow_stays_silent`,
`test_vscode_already_rtk_allow_asserted_with_command_unchanged`,
`test_copilot_cli_already_rtk_allow_asserted_with_command_unchanged`,
`test_cursor_already_rtk_allow_asserted_with_command_unchanged`,
`test_gemini_already_rtk_allow_asserted_with_command_unchanged`,
`test_droid_renderer_with_command_unchanged_stays_silent`,
`test_vibe_renderer_with_command_unchanged_stays_silent`,
`test_decide_ask_asserts_for_mixed_and_wrapped_compounds`,
`test_claude_payload_asserts_allow_for_already_rtk_command`, and
`test_decide_defer_for_already_rtk_command_with_no_matching_rule` (review finding: under
`suppress_identity` every non-Deny verdict on an already-rtk command is `Defer`, so the
"no matching rule" distinction it pinned can no longer fail; upstream's
`decision_consistency` pins the suppression itself).

Kept, still green under upstream's `decide`: `test_decide_deny_for_already_rtk_command`,
`test_decide_deny_sees_through_command_wrappers`,
`test_claude_payload_asserts_deny_for_already_rtk_command`,
`test_claude_payload_original_form_deny_stays_skip`, both `contains_already_rtk_segment`
tests, and every `permissions.rs` matcher test.

## Fork delta

`371e058` and `31fceff` added to `HEALED_SHAS` in `scripts/fork-delta.sh`: their headline
behavior (the Allow-assert quantifier, the Ask-assert broadening, the no-op renderer
silence) is superseded by upstream `d6ce8f7`. `371e058`'s surviving ASCII-IFS whitespace
rule lives in `strip_token`, which `505e355` introduced. `30557a6`, `8adbeba`, `505e355` and
`d9e7458` stay in the delta: the deny path, the wrapper see-through and `rtk run` in
`COMMAND_WRAPPERS` are live. Caveat for the next reader: the generated `.github/README.md`
rows for `505e355` ("…assert ask for already-rtk") and `d9e7458` ("…never assert for wrapped
invocations") keep their commit subjects and so still name the removed assert behavior; the
commits stay because their matcher halves are live.

## User-visible change

With `{"permissions":{"allow":["Bash(grep:*)"]}}` and no rewrite to make, `rtk grep foo`
used to get an explicit `permissionDecision: "allow"` from the Claude hook; it now gets no
hook output, and Claude Code's own matcher decides. The same holds for an already-rtk command
under an `ask` rule or with a redirect. Deny through the `rtk` prefix is unchanged.

## Repro, real binaries

Sandboxed `CLAUDE_CONFIG_DIR` with
`{"permissions":{"deny":["Bash(rm:*)"],"allow":["Bash(grep:*)"],"ask":["Bash(git push:*)"]}}`,
scratch `RTK_DB_PATH` and `HOME`. before = installed `~/bin/rtk.exe` (develop build of
2026-09-08), after = `target/x86_64-pc-windows-msvc/release/rtk.exe` from this branch.

```
$ printf '{"tool_name":"Bash","tool_input":{"command":"<cmd>"}}' | rtk hook claude

rtk grep foo
before: {..,"updatedInput":{"command":"rtk grep foo"},"permissionDecision":"allow"}
after:  <silent>

rtk git push origin main
before: {..,"updatedInput":{"command":"rtk git push origin main"},"permissionDecision":"ask"}
after:  <silent>

rtk git log > /tmp/out.txt
before: {..,"updatedInput":{"command":"rtk git log > /tmp/out.txt"},"permissionDecision":"ask"}
after:  <silent>

rtk rm -rf /tmp/x
before/after: {..,"permissionDecision":"deny","permissionDecisionReason":"RTK: matches a configured deny rule"}

rtk proxy rm -rf /tmp/x
before/after: {..,"permissionDecision":"deny","permissionDecisionReason":"RTK: matches a configured deny rule"}

git status
before/after: {..,"updatedInput":{"command":"rtk git status"},"permissionDecision":"ask"}
```

## Quality gate

x64 host toolchain (`scripts/win-dev-env.ps1`): `cargo fmt --all` clean · `cargo clippy
--all-targets` 0 warnings · `cargo test --all` 3540 unit tests passed, 0 failed, 8 ignored;
all integration suites green, including upstream's new `hook_decision_protocol_test`,
`git_show_blob_differential_test` and `signal_flush_test`. `git diff --check` clean.
