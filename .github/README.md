<p align="center">
  <strong>rtk — a CLI proxy that cuts up to 90% of the bash output your coding agent reads</strong>
</p>

<p align="center">
  <a href="https://github.com/kylehgc/rtk/actions/workflows/ci.yml"><img src="https://github.com/kylehgc/rtk/actions/workflows/ci.yml/badge.svg?branch=develop" alt="CI"></a>
  <a href="https://github.com/kylehgc/rtk/releases"><img src="https://img.shields.io/github/v/release/kylehgc/rtk?include_prereleases&label=fork%20release" alt="Release"></a>
  <a href="https://opensource.org/licenses/Apache-2.0"><img src="https://img.shields.io/badge/License-Apache_2.0-blue.svg" alt="License: Apache 2.0"></a>
</p>

# rtk — kylehgc fork

## What rtk is

When a coding agent runs `cargo test`, `git log`, `npm install`, or `pytest`, the raw
output goes straight into its context window — thousands of lines of progress bars,
timestamps, and repetition, most of which the model cannot use. rtk sits in front of
those commands, runs them for real, and passes back a filtered version that keeps the
errors, the failures, and the diagnostics while dropping the noise. Across rtk's own
benchmark suite — 73 cases, run with [`scripts/benchmark.sh`](../scripts/benchmark.sh) —
aggregate output came to **541,111 → 123,205 tokens, a 77% reduction**. Individual
commands vary enormously: some are cut by 90%, and some are passed through untouched
because filtering them would lose information or gain nothing.

That is a measure of **bash output**, not of your bill, and rtk ships no tokenizer —
it estimates tokens as bytes/4, so the ratios are sound and the absolute counts are
approximate.

It is a transparent proxy: `rtk cargo test` runs `cargo test`, exits with the same
status code, and works for commands it has no specific filter for. A hook can rewrite
your agent's commands automatically, so nothing in your workflow changes.

> **📖 [Full command reference, installation guides, and architecture docs →](../README.md)**
> That is upstream's documentation and it applies to this fork unchanged. Everything
> below is only about what this fork adds.

## Why this fork

[rtk-ai/rtk](https://github.com/rtk-ai/rtk) is active but slow to merge: good community
bug fixes sit open for months. This fork tracks upstream `develop`, adopts those fixes
with their original authorship intact, and adds its own where a bug has no fix pending.
Fixes authored here are submitted back upstream — the goal is for this list to shrink.

Most of what this fork fixes is **fidelity**, not compression. Upstream sometimes filters
away information the agent actually needed: compiler warnings, failing-test stderr,
`--porcelain` output that was never meant to be human-readable. Those fixes make rtk's
output *larger* and more correct. This fork does not claim to save more tokens than
upstream — that is upstream's pitch. It claims to not lose your errors.

<!-- FORK_DELTA_START -->
**59 fixes in this fork that upstream does not have.** Each links to the commit,
where the original author is recorded. Adopted fixes come from community PRs that upstream
has not merged — see the [adoption issues](https://github.com/kylehgc/rtk/issues?q=is%3Aissue+Adopt+upstream)
for provenance.

This count is an upper bound: fixes upstream has since merged verbatim are dropped
automatically by patch-id, and audited equivalents are excluded by hand
(`scripts/fork-delta.sh`) — but pending the next audit, an entry may already exist
upstream in another form.

| Fix | Commit |
|---|---|
| fix(benchmark): count find names only, not the disclosure note and tee pointer | [`e2e11b7`](https://github.com/kylehgc/rtk/commit/e2e11b7) |
| fix(git): fall back to raw text when compact_diff gets non-unified input | [`c8e7a8b`](https://github.com/kylehgc/rtk/commit/c8e7a8b) |
| fix(diff): stop reporting differing files as identical, and align by LCS | [`acbe88c`](https://github.com/kylehgc/rtk/commit/acbe88c) |
| fix(jest): equals-form --reporters is greedy too; respect -- terminator | [`f0f34af`](https://github.com/kylehgc/rtk/commit/f0f34af) |
| fix(jest): consume the values of a space-separated --reporters flag | [`b4aceb6`](https://github.com/kylehgc/rtk/commit/b4aceb6) |
| fix(playwright): consume the value of a space-separated --reporter flag | [`692fb72`](https://github.com/kylehgc/rtk/commit/692fb72) |
| fix(hooks): broaden ask assert to any-segment; silence no-op renders | [`31fceff`](https://github.com/kylehgc/rtk/commit/31fceff) |
| fix(hooks): add rtk run to wrappers; never assert for wrapped invocations | [`d9e7458`](https://github.com/kylehgc/rtk/commit/d9e7458) |
| fix(hooks): see through rtk command wrappers; assert ask for already-rtk | [`505e355`](https://github.com/kylehgc/rtk/commit/505e355) |
| fix(hooks): require every segment rtk-prefixed before asserting Allow | [`371e058`](https://github.com/kylehgc/rtk/commit/371e058) |
| fix(hooks): harden already-rtk permission matching (amendments to upstream #3195) | [`8adbeba`](https://github.com/kylehgc/rtk/commit/8adbeba) |
| fix(hooks): honor permission rules for already-rtk-prefixed commands | [`30557a6`](https://github.com/kylehgc/rtk/commit/30557a6) |
| fix(diff): preclear round 4 -- gate new sections on a following hunk | [`5939842`](https://github.com/kylehgc/rtk/commit/5939842) |
| fix(diff): preclear round 3 -- keep the no-newline marker, strict UTF-8 gate | [`fb5a3dc`](https://github.com/kylehgc/rtk/commit/fb5a3dc) |
| fix(diff): preclear round 2 -- file-level facts, sha256 mbox, contexts | [`25a17c0`](https://github.com/kylehgc/rtk/commit/25a17c0) |
| fix(diff): preclear round 1 -- report hunkless empty-file sections, tighten guards | [`2219c6d`](https://github.com/kylehgc/rtk/commit/2219c6d) |
| fix(diff): review round 1 -- byte-raw fallback, strict prefix width, fixture hygiene | [`8243d1a`](https://github.com/kylehgc/rtk/commit/8243d1a) |
| fix(diff): region parser for condense_unified_diff -- budget-owned hunks, raw fallback | [`5a61595`](https://github.com/kylehgc/rtk/commit/5a61595) |
| fix(diff): fall back to raw when a hunk budget disagrees with its body | [`1f64783`](https://github.com/kylehgc/rtk/commit/1f64783) |
| fix(diff): close hunks on the @@ line budget; drop the phantom overflow trailer | [`8e65037`](https://github.com/kylehgc/rtk/commit/8e65037) |
| fix(diff): end a hunk on any non-body line, not only `diff --git` | [`89cfe86`](https://github.com/kylehgc/rtk/commit/89cfe86) |
| fix(diff): classify ---/+++ by position, not prefix alone | [`5bcee1c`](https://github.com/kylehgc/rtk/commit/5bcee1c) |
| fix(diff): cover both flush sites and stop dropping ---/+++ content | [`6375941`](https://github.com/kylehgc/rtk/commit/6375941) |
| fix(lint): guard known linter names from the on-disk path check, bound failure passthrough | [`3240075`](https://github.com/kylehgc/rtk/commit/3240075) |
| fix(hook): delegate lint scripts to package managers | [`cae0f21`](https://github.com/kylehgc/rtk/commit/cae0f21) |
| fix(lint): stop reading a bare path as a linter name | [`617ef8e`](https://github.com/kylehgc/rtk/commit/617ef8e) |
| fix(prettier): enforce the failure invariant at every call site | [`59f2564`](https://github.com/kylehgc/rtk/commit/59f2564) |
| fix(prettier): strip ANSI before parsing [warn] markers | [`6f4d9f7`](https://github.com/kylehgc/rtk/commit/6f4d9f7) |
| fix(prettier): read check results from stderr, never report a failing check as success | [`34946b7`](https://github.com/kylehgc/rtk/commit/34946b7) |
| fix(rewrite): never rewrite yadm to rtk git; keep bare git add a no-op | [`1d67272`](https://github.com/kylehgc/rtk/commit/1d67272) |
| fix(hook): make gemini runner fail open on empty stdin | [`cf976df`](https://github.com/kylehgc/rtk/commit/cf976df) |
| fix(hook): fail open when stdin payload stalls | [`04f881a`](https://github.com/kylehgc/rtk/commit/04f881a) |
| fix(read): honor --max-lines N as exact head count | [`653fb56`](https://github.com/kylehgc/rtk/commit/653fb56) |
| fix(grep): stop -l/-m/-t shadowing native grep flags | [`0704f58`](https://github.com/kylehgc/rtk/commit/0704f58) |
| fix(js): preserve failed command output in parser fallbacks | [`723bea4`](https://github.com/kylehgc/rtk/commit/723bea4) |
| fix(proof): enforce tool-required guards inside the script itself | [`6fb87aa`](https://github.com/kylehgc/rtk/commit/6fb87aa) |
| fix(release): make the version stamp work on macOS and RPM | [`79f2dd9`](https://github.com/kylehgc/rtk/commit/79f2dd9) |
| fix(release): stamp Cargo.toml's version from the release tag | [`0a876c8`](https://github.com/kylehgc/rtk/commit/0a876c8) |
| fix(cd): compute the first fork RC without a tag that does not exist | [`1696cda`](https://github.com/kylehgc/rtk/commit/1696cda) |
| fix(bench): serve curl/wget fixtures over local HTTP, not file:// | [`1d1d86b`](https://github.com/kylehgc/rtk/commit/1d1d86b) |
| fix(bench): pin the curl fixtures instead of fetching a random payload | [`8a0d305`](https://github.com/kylehgc/rtk/commit/8a0d305) |
| fix(cargo): stop the raw-tail fallback restating captured warnings | [`f954b61`](https://github.com/kylehgc/rtk/commit/f954b61) |
| fix(cargo): keep compile errors visible when warnings are captured | [`6235d4b`](https://github.com/kylehgc/rtk/commit/6235d4b) |
| fix(cargo): preserve compiler warnings in cargo test output on passing runs | [`ff13986`](https://github.com/kylehgc/rtk/commit/ff13986) |
| fix(git): correct the machine-output flag set and stop diluting gain stats | [`da407dc`](https://github.com/kylehgc/rtk/commit/da407dc) |
| fix(git): keep machine output raw | [`46d16ea`](https://github.com/kylehgc/rtk/commit/46d16ea) |
| fix(search): spare value tokens from the rg -r/-R letter strip | [`d43cef5`](https://github.com/kylehgc/rtk/commit/d43cef5) |
| fix(search): strip ripgrep -r/-R so rg --replace no longer corrupts output | [`00d8ee4`](https://github.com/kylehgc/rtk/commit/00d8ee4) |
| fix(hook): accept current Claude tool input keys | [`c8d6e28`](https://github.com/kylehgc/rtk/commit/c8d6e28) |
| feat(init): add PowerShell hook for Claude Code on Windows | [`cc5b6d3`](https://github.com/kylehgc/rtk/commit/cc5b6d3) |
| feat(mvn): add rtk mvnd support for Maven Daemon | [`fa55089`](https://github.com/kylehgc/rtk/commit/fa55089) |
| fix(pnpm): preserve install failure output | [`2bbe81f`](https://github.com/kylehgc/rtk/commit/2bbe81f) |
| fix(hook): hook warning repeats on every command on Windows | [`8945b96`](https://github.com/kylehgc/rtk/commit/8945b96) |
| fix(hooks): add -- terminator to hermes, opencode, and pi rewrite callers | [`8253401`](https://github.com/kylehgc/rtk/commit/8253401) |
| fix(cli): handle non-UTF-8 argv in raw-execution fallback | [`22990e9`](https://github.com/kylehgc/rtk/commit/22990e9) |
| fix(runner): surface a failing tool's stderr under stdout-only filtering | [`c27bbd0`](https://github.com/kylehgc/rtk/commit/c27bbd0) |
| fix(core): map code page 54936 to GB18030 instead of GBK | [`7ead416`](https://github.com/kylehgc/rtk/commit/7ead416) |
| fix(core): decode process output using Windows console code page | [`13cf995`](https://github.com/kylehgc/rtk/commit/13cf995) |
| fix(hook): emit ask decision for Claude rewrites | [`e93cde8`](https://github.com/kylehgc/rtk/commit/e93cde8) |

<!-- FORK_DELTA_END -->

### Proof

Every claim above is backed by a test you can run. These are the fork's own integration
tests, executed against a checkout of `upstream/develop` — they pass here and fail there.
This covers a subset of the fixes listed above; fixes tested only by internal unit tests
cannot be lifted into upstream's tree, so they are claimed but not proven.

<!-- FORK_PROOF_START -->
**12 claims proven** by tests that pass on this fork and fail against
`upstream/develop` — 11 fidelity, 1 reduction. Run them yourself with
`scripts/fork-proof.sh`.

| Claim | Test | Upstream | This fork |
|---|---|---|---|
| Upstream filters `git --porcelain` output that was never meant to be read by a human; the fork passes machine output through byte-for-byte. | `machine_readable_git_output_is_byte_identical_to_native` | ❌ fails | ✅ passes |
| Upstream aborts (SIGABRT, exit 134) on any argument containing non-UTF-8 bytes, before the wrapped command runs; the fork executes it. | `non_utf8_pattern_does_not_abort` | ❌ fails | ✅ passes |
| The wrapped tool receives the exact bytes the user typed, not a lossy copy. | `non_utf8_pattern_forwards_original_bytes` | ❌ fails | ✅ passes |
| A non-UTF-8 argument to a command that does not exist exits cleanly instead of aborting. | `non_utf8_arg_to_missing_command_exits_cleanly` | ❌ fails | ✅ passes |
| Upstream discards the error output of a failed `pnpm install`; the fork preserves it. | `pnpm_install_failure_preserves_stdout_error_output` | ❌ fails | ✅ passes |
| Upstream swallows a failing tool's stderr under stdout-only filtering; the fork surfaces it. | `failing_tool_stderr_reaches_the_user` | ❌ fails | ✅ passes |
| Upstream's Claude hook only reads the legacy `tool_input` key and silently ignores the current `input`-shaped PreToolUse payload; the fork rewrites it and preserves sibling fields. | `claude_hook_rewrites_current_input_key_shape` | ❌ fails | ✅ passes |
| Upstream's Claude hook omits `permissionDecision` for an unconfigured (Default-verdict) rewrite, relying on an absent key; the fork explicitly emits `"ask"`. | `claude_hook_emits_ask_decision_for_default_verdict` | ❌ fails | ✅ passes |
| Upstream forwards `-r`/`-R` to ripgrep unchanged, so grep muscle memory (`rg -rn`) is silently read as `--replace` and every match is rewritten to garbage; the fork strips it before rg runs. | `rg_short_r_cluster_does_not_trigger_ripgrep_replace` | ❌ fails | ✅ passes |
| Upstream's `-r`/`-R` ambiguity corrupts matches the same way even when a value token (e.g. a `--glob` value) happens to start with `-r`; the fork strips only the real flag and leaves the value intact. | `rg_dash_prefixed_flag_value_survives_r_strip` | ❌ fails | ✅ passes |
| Upstream drops every compiler warning on a passing `cargo test` run; the fork preserves the full warning detail and annotates the summary with a compiler-warning count. | `cargo_test_preserves_compiler_warnings_on_passing_run` | ❌ fails | ✅ passes |
| Measures: on `cargo test` output with a warning but no test-result line, the fork's raw-tail fallback excludes lines its captured-warnings section already printed — the warning's detail line appears exactly once (not restated) and the filtered output is smaller than raw cargo output; upstream has no captured-warnings section to protect. | `piped_cargo_test_filter_does_not_restate_captured_warnings` | ❌ fails | ✅ passes |

<!-- FORK_PROOF_END -->

## Install

Download a binary from [releases](https://github.com/kylehgc/rtk/releases/latest).
Builds are published for Linux (x86_64 musl, aarch64), macOS (Intel, Apple Silicon),
and Windows (x86_64), plus `.deb` and `.rpm` packages.

```bash
# Linux x86_64
curl -sSfL https://github.com/kylehgc/rtk/releases/latest/download/rtk-x86_64-unknown-linux-musl.tar.gz | tar xz
sudo mv rtk /usr/local/bin/
rtk --version
```

Or build from source:

```bash
cargo install --git https://github.com/kylehgc/rtk --branch develop
```

Two release tracks:

| Tag | What it is |
|---|---|
| `fork-vX.Y.Z` | Stable. Cut deliberately, CI green. Use this one. |
| `fork-dev-X.Y.Z-rc.N` | Built on every merge to `develop`. Current, less settled. |

Fork versions start at `0.1.0` and are **not** comparable to upstream's — they are a
separate line. Each release states which upstream commit it is based on.

Once installed, setup is identical to upstream:

```bash
rtk init -g     # register the agent hook globally
rtk gain        # see what it saved
```

## Relationship to upstream

This is a merge-based tracking fork, not a hard fork. It pulls `upstream/develop` in
periodically and never rebases published history. Upstream's version always wins in a
conflict — the fork exists to *add* what upstream lacks, never to hold a different
opinion about what upstream already has.

If you want rtk itself, use [upstream](https://github.com/rtk-ai/rtk). Use this fork if
one of the fixes above is blocking you.

Maintenance process, glossary, and decision records: [CONTEXT.md](../CONTEXT.md) and
[docs/adr/](../docs/adr/).
