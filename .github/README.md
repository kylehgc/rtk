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
away information the agent actually needed: compiler warnings on a passing `cargo test`,
the error output of a failed `pnpm install`, the final newline of `git status --porcelain` that `wc -l` and `while read` count on.
Those fixes make rtk's
output *larger* and more correct. This fork does not claim to save more tokens than
upstream — that is upstream's pitch. It claims to not lose your errors.

<!-- FORK_DELTA_START -->
**37 fixes in this fork that upstream does not have.** Each links to the commit,
where the original author is recorded. Adopted fixes come from community PRs that upstream
has not merged — see the [adoption issues](https://github.com/kylehgc/rtk/issues?q=is%3Aissue+Adopt+upstream)
for provenance.

This count is an upper bound: fixes upstream has since merged verbatim are dropped
automatically by patch-id, and audited equivalents are excluded by hand
(`scripts/fork-delta.sh`) — but pending the next audit, an entry may already exist
upstream in another form.

| Fix | Commit |
|---|---|
| fix(prettier): never report a failed run as formatted | [`3d27402f`](https://github.com/kylehgc/rtk/commit/3d27402f) |
| fix(cli): cover the help guards end to end, skip rtk's package-runner prefix, disable clap's help subcommand on wrapped parents | [`66f1cb71`](https://github.com/kylehgc/rtk/commit/66f1cb71) |
| fix(cli): exempt rtk format from help forwarding, opt prisma parents out, tree -h is a size flag | [`2a4dc090`](https://github.com/kylehgc/rtk/commit/2a4dc090) |
| fix(cli): keep clap's help on rtk's shell runners, treat -h as help where the tool does, route find --help verbatim | [`372cf482`](https://github.com/kylehgc/rtk/commit/372cf482) |
| fix(cli): route a forwarded --help to the tool's passthrough, not its filter | [`690c8a1b`](https://github.com/kylehgc/rtk/commit/690c8a1b) |
| fix(cli): keep rtk's help on meta commands, forward it past external-arm parents, surface git's usage | [`2993807d`](https://github.com/kylehgc/rtk/commit/2993807d) |
| fix(cli): hand --help/-h to the wrapped tool on every forwarding subcommand | [`beb45c20`](https://github.com/kylehgc/rtk/commit/beb45c20) |
| fix(tsc): preserve informational output | [`bb0a9c8c`](https://github.com/kylehgc/rtk/commit/bb0a9c8c) |
| fix(git): keep the trailing newline on filtered 'git status' output | [`d536891a`](https://github.com/kylehgc/rtk/commit/d536891a) |
| fix(git): fall back to raw text when compact_diff gets non-unified input | [`c8e7a8bf`](https://github.com/kylehgc/rtk/commit/c8e7a8bf) |
| fix(jest): equals-form --reporters is greedy too; respect -- terminator | [`f0f34aff`](https://github.com/kylehgc/rtk/commit/f0f34aff) |
| fix(jest): consume the values of a space-separated --reporters flag | [`b4aceb61`](https://github.com/kylehgc/rtk/commit/b4aceb61) |
| fix(playwright): consume the value of a space-separated --reporter flag | [`692fb72a`](https://github.com/kylehgc/rtk/commit/692fb72a) |
| fix(hooks): add rtk run to wrappers; never assert for wrapped invocations | [`d9e74588`](https://github.com/kylehgc/rtk/commit/d9e74588) |
| fix(hooks): see through rtk command wrappers; assert ask for already-rtk | [`505e3550`](https://github.com/kylehgc/rtk/commit/505e3550) |
| fix(hooks): harden already-rtk permission matching (amendments to upstream #3195) | [`8adbeba0`](https://github.com/kylehgc/rtk/commit/8adbeba0) |
| fix(hooks): honor permission rules for already-rtk-prefixed commands | [`30557a6f`](https://github.com/kylehgc/rtk/commit/30557a6f) |
| fix(lint): guard known linter names from the on-disk path check, bound failure passthrough | [`32400756`](https://github.com/kylehgc/rtk/commit/32400756) |
| fix(hook): delegate lint scripts to package managers | [`cae0f218`](https://github.com/kylehgc/rtk/commit/cae0f218) |
| fix(lint): stop reading a bare path as a linter name | [`617ef8e1`](https://github.com/kylehgc/rtk/commit/617ef8e1) |
| fix(rewrite): never rewrite yadm to rtk git; keep bare git add a no-op | [`1d672725`](https://github.com/kylehgc/rtk/commit/1d672725) |
| fix(hook): make gemini runner fail open on empty stdin | [`cf976dfb`](https://github.com/kylehgc/rtk/commit/cf976dfb) |
| fix(hook): fail open when stdin payload stalls | [`04f881a1`](https://github.com/kylehgc/rtk/commit/04f881a1) |
| fix(js): preserve failed command output in parser fallbacks | [`723bea40`](https://github.com/kylehgc/rtk/commit/723bea40) |
| fix(proof): enforce tool-required guards inside the script itself | [`6fb87aa9`](https://github.com/kylehgc/rtk/commit/6fb87aa9) |
| fix(release): make the version stamp work on macOS and RPM | [`79f2dd91`](https://github.com/kylehgc/rtk/commit/79f2dd91) |
| fix(release): stamp Cargo.toml's version from the release tag | [`0a876c85`](https://github.com/kylehgc/rtk/commit/0a876c85) |
| fix(cd): compute the first fork RC without a tag that does not exist | [`1696cda1`](https://github.com/kylehgc/rtk/commit/1696cda1) |
| fix(cargo): stop the raw-tail fallback restating captured warnings | [`f954b615`](https://github.com/kylehgc/rtk/commit/f954b615) |
| fix(cargo): keep compile errors visible when warnings are captured | [`6235d4b9`](https://github.com/kylehgc/rtk/commit/6235d4b9) |
| fix(cargo): preserve compiler warnings in cargo test output on passing runs | [`ff139867`](https://github.com/kylehgc/rtk/commit/ff139867) |
| fix(git): correct the machine-output flag set and stop diluting gain stats | [`da407dc3`](https://github.com/kylehgc/rtk/commit/da407dc3) |
| fix(git): keep machine output raw | [`46d16ead`](https://github.com/kylehgc/rtk/commit/46d16ead) |
| fix(pnpm): preserve install failure output | [`2bbe81fd`](https://github.com/kylehgc/rtk/commit/2bbe81fd) |
| fix(hook): hook warning repeats on every command on Windows | [`8945b96b`](https://github.com/kylehgc/rtk/commit/8945b96b) |
| fix(hooks): add -- terminator to hermes, opencode, and pi rewrite callers | [`82534016`](https://github.com/kylehgc/rtk/commit/82534016) |
| fix(cli): handle non-UTF-8 argv in raw-execution fallback | [`22990e91`](https://github.com/kylehgc/rtk/commit/22990e91) |

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
