# Where upstream is worse than the fork — candidates for upstream PRs (2026-10-09)

Built during the 2026-10-09 sync (`upstream/develop` = `e4f0509`), then corrected the same day
after an A/B pass against an upstream build and a read of each maintainer decision. Every entry
compares upstream's code (`git show e4f05094:<path>`) with what the fork ships. **Evidence**
says how solid each claim is:

- **verified**: seen in upstream's own code or tests, or reproduced on a real binary.
- **inspection**: follows from reading upstream's code, not reproduced.
- **unverified**: needs an A/B run against an upstream build before filing.

**Bottom line: no new "fork improvements" PR is warranted.** Every verified gap either already
has an open upstream PR (most of them the fork's own or ones it adopted), or is a behaviour
upstream chose on purpose. The fork dropped the latter in the alignment follow-up (see
`sync-conflict-2026-10-09.md`).

## Open gaps — each already has an upstream PR

| # | Upstream behaviour | Fork behaviour | Evidence | Upstream PR |
|---|---|---|---|---|
| 1 | `yadm status/add/commit/...` rewrite to `rtk git ...` (`^(?:git\|yadm)`), which runs **git on the current directory's repo**, not yadm's dotfiles repo. | Routes to the `yadm.toml` filter, which runs yadm. | **verified**: upstream's own `test_rewrite_yadm_status` asserts `Some("rtk git status")`. | rtk-ai/rtk#3414 |
| 2 | `pnpm lint` / `npm run lint` rewrite to `rtk lint`, replacing the package's own `lint` script with eslint/biome. | Routes to `rtk pnpm lint`, which runs the script. | **inspection** | rtk-ai/rtk#3289 |
| 3 | Jest `--reporters default` forwards `default` as a test-path pattern. | Drops the flag and its value. | **inspection**; fork tests | rtk-ai/rtk#3777 (fork's) |
| 4 | `run_fallback` panics on a non-UTF-8 argument. | `args_os`; the child gets the original bytes. | **verified** (fork-proof claim) | rtk-ai/rtk#3224 |
| 5 | A passing `cargo test` drops every compiler warning. | Keeps them, counts them. | **verified** (fork-proof claim) | rtk-ai/rtk#2877 |
| 6 | An already-prefixed `rtk git push --force` escapes `Bash(git push:*)` deny rules. | Matches raw and rtk-stripped forms; the hook asserts the deny. | code difference; fork tests | rtk-ai/rtk#3195 |
| 7 | `git status --porcelain` (v1 and v2) loses its final newline, so `wc -l` undercounts and `while read` drops the last entry. `git log --format=%H` silently stops at 50 commits; `git log -z --format=%H -5` comes back 121 bytes of 205. | Machine output passes through byte-identical. | **verified** on an e4f0509 build | porcelain: rtk-ai/rtk#3682, but see below. log: KuSh's rtk-ai/rtk#4152 keeps user formats capped and announces the cap. When it merges, the fork's log passthrough becomes a design difference and goes. |
| 8 | `rtk format prettier --check` on a failing check prints "All files formatted correctly", exit 1. | Adopted rtk-ai/rtk#3571. | **verified** on an e4f0509 build | rtk-ai/rtk#3571 (KuSh's pick) |
| 9 | `rtk <tool> ... --help` shows rtk's help, not the tool's. | Forwards it. | **verified** in fork tests | rtk-ai/rtk#4348 |
| 10 | `rtk git diff` with `diff.external` may drop the driver's output. | Driver output reaches stdout. | **unverified** on upstream | rtk-ai/rtk#3607 |

**rtk-ai/rtk#3682 has a bug of its own.** Its `with_trailing_newline` also appends `\n` after
`git status -z`'s final NUL, which `xargs -0` reads as an extra record. Reproduced on a build of
the PR head (353fdb71); `e4f0509` prints git's bytes there, and the call site is unchanged on
current `develop`. Commented on the PR with the repro and a one-line guard
(`|| output.ends_with('\0')`). The fork doesn't need the guard: its passthrough sends
`status -z` to the caller raw.

## Unverified, fork-only, low stakes

- `tsc --version` / `--help` / `--showConfig` run through the diagnostics filter upstream.
- Hook-warning marker written with `b""`; on Windows an empty rewrite may not bump mtime.

A/B both against an upstream build before considering them.

## Upstream's decision — the fork follows it (don't file)

- **Permission `"ask"`**: upstream omits `permissionDecision` on a default verdict on purpose so
  Claude Code prompts and remembers (KuSh, rtk-ai/rtk#3018; rtk-ai/rtk#3031 closed). Dropped.
- **`input`-keyed hook payloads**: no Claude Code hook event sends them (rtk-ai/rtk#2535). Dropped.
- **PowerShell matcher**: running PowerShell through the bash rewriter mangles `\` paths, takes
  over aliases, bypasses PowerShell deny rules (rtk-ai/rtk#2075). Dropped.
- **`--` in the legacy shell-script hooks**: closed as legacy-only, no further contributions
  wanted (rtk-ai/rtk#2475). Upstream never carried `--` there; the earlier claim that `c6f484f`
  dropped it was wrong. Dropped.
- **Prettier report on stderr**: upstream forwards it on its own stream (rtk-ai/rtk#3772). The
  fork's combined-stream summary is gone; rtk-ai/rtk#3571 covers the `rtk format` case.
- Also superseded: `read --max-lines` (upstream `--head-lines`), rg `-r` (rtk-ai/rtk#3681
  tokenizer), opencode `--` (JSON protocol), stdout-only stderr forwarding (rtk-ai/rtk#3772).

## Considered and not filed

- **Hermes/pi plugins and `-h`/`--help`**: both call `rtk rewrite <cmd>` without `--`, so a
  command that is exactly `-h` or `--help` is replaced by rtk's help text (reproduced with
  upstream's hermes plugin and an e4f0509 build). Not filed. The trigger is a command no agent
  issues. Upstream's hermes rewrite doesn't apply inside Hermes at all yet (rtk-ai/rtk#3797,
  fix pending in rtk-ai/rtk#3833). KuSh closed the same class of fix for the shell scripts. The
  fork's plugins keep their `--`.

## Fork-side weaknesses

Fixed in the sync: grammar residue in `is_rtk_prefixed`. Fixed in the alignment follow-up: the
stale `GitCommand::user_args` doc and test, `detect_linter`'s checkout-touching tests, the hermes
Windows fake's missing `--`, and the adopted prettier test's unisolated spawn.

Still open: the surviving argument scans (`requests_help`, `strip_jest_conflicting_args`)
predate the now-mandatory `core::arg_tokenizer`. Upstream's reviewer would flag them on
rtk-ai/rtk#4348 and rtk-ai/rtk#3777.
