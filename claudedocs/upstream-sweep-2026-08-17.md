# Upstream sweep — 2026-08-17

Incremental triage of rtk-ai/rtk open PRs above the previous watermark.

- **Coverage**: open upstream PRs **#3169–#3596** (watermark from `adoption-reverify-2026-07-22.md` was #3168). ~175 PRs swept; the newest-200 page reached #3105, so coverage is complete.
- **Verdicts**: 67 CANDIDATE · 2 UNSURE · rest PASS (25 auto-pass: 10 drafts, 11 docs, 1 release-bot, 1 chore, 2 own; ~13 verified superseded on develop).
- **Tickets filed**: fork #99–#114 (16 tickets covering 24 upstream PRs), plus a note on existing #8.
- Classification ran as 10 parallel agents, each verifying candidates against current develop (synced, 0 behind upstream at sweep time).

## Tickets filed

| Fork issue | Upstream PRs | Area |
|---|---|---|
| #99 | 3414 | git add injects `.` — stages whole tree |
| #100 | 3268 (refs 3471, 3519) | diff false-identical, byte-accurate compare |
| #101 | 3270 + 3379 (ref 3375) | git log silent caps + phantom blank line |
| #102 | 3269 | git diff hunk indent breaks `^-` anchors |
| #103 | 3208 | grep --max-len truncation discarded |
| #104 | 3333 + 3579 | unbounded capture buffers (OOM) |
| #105 | 3571 | prettier false success |
| #106 | 3567 + 3289 | lint fabricated verdicts / script hijack |
| #107 | 3572 | playwright space-form --reporter |
| #108 | 3202 + 3192 + 3462 (ref 3554) | rewrite corrupts case/continuation/assignment |
| #109 | 3195 | permission rules bypassed by `rtk ` prefix |
| #110 | 3238 | MSYS children eat backslashes |
| #111 | 3505 (refs 3596, 3272, 3547/3239, 3333) | exec second-shell re-parse |
| #112 | 3566 + 3231 (refs 3502, 3451) | prisma counts + subcommand gate |
| #113 | 3573 | curl -s swallows error cause |
| #114 | 3352 (ref 3373) | find bracket globs / hidden / ignored files |
| #8 (comment) | 3271, 3325 | vitest cluster additions |

## Bench — CANDIDATE, verified live, not filed

Real bugs confirmed on develop but below the filing cut. Re-rank in a future sweep or pull directly.

| PR | Rank | Evidence on develop |
|---|---|---|
| 3451 | b | Six byte-indexed slice sites panic on multibyte: `core/utils.rs:181`, `analytics/gain.rs:464`, `display_helpers.rs:205,210`, `prisma_cmd.rs:319`, `find_cmd.rs:326`. Subsumes 3435; only git.rs site already fixed |
| 3582 | b,c | `main.rs:1623-1626` hook nag can hit protocol stderr; stream.rs tests non-portable to Windows (spawn sh/echo/cat directly) |
| 3547 / 3239 | c | `build.rs:6-13` MSVC `/STACK:` under plain `#[cfg(windows)]` breaks *-pc-windows-gnu; fork builds MSVC so low urgency |
| 3424 | a/c | Dead filter patterns never fire: `filters/gradle.toml:3`, `gcc.toml:3` unmatchable; harness only tests transforms |
| 3420 | b/c | `main.rs:1595-1605` uninstall for unknown agents wipes Claude Code integration; fork's extra Vibe arm means re-port, not cherry-pick |
| 3390 | c | `rules.rs:72` no `+toolchain` handling — `cargo +nightly test` unrouted |
| 3385 | c | `cargo_cmd.rs:939-943` redundant summary on 0-crate builds |
| 3542 | a | `gh_cmd.rs:471-502` --watch snapshot repeats inflate pass/fail counts |
| 3538 | a | `filters/du.toml` max_lines drops `du -s`/`-d N` totals silently |
| 3540 | a (low) | `ls.rs:265-266` NOISE_DIRS dropped without count → "(empty)" lie |
| 3517 | c | `discover/report.rs:52-56` no Claude PreToolUse detection → false "missed savings" |
| 3559 | a,c | `filters/turbo.toml:9-10` strips Tasks:/Duration: tally (turbo hunk only; rest niche) |
| 3569 | b | `registry.rs:4719` sudo re-attached — rtk runs as root |
| 3554 | c (low) | `lexer.rs:429-431` commands after a pipeline vanish from discover (diagnostics only) |
| 3550 | c (low) | `config.rs:93-108` ignore_dirs/ignore_files config is dead — no reader in cmds/system |
| 3504 | c (low) | `hooks/init.rs:1701+` bare `contains("rtk")` skips rules-file install |
| 3483 | c | `hooks/constants.rs:12` hook registered as shell string → shell wrapper per call; PR untested by author, verify migration |
| 3592 + 3590 | c | path-prefixed commands (`./node_modules/.bin/vitest`) classify but don't rewrite (`registry.rs:1476-1485`); 3590 is prereq plumbing |
| 3596 / 3272 | b,c | `rtk test` argv join→shell (covered by #111's cluster refs) |
| 3273 | b | `utils.rs:394` missing binary → EACCES noise + exit 1 instead of 127; wide diff (12 files) |
| 3275 | c | `registry.rs:135` no pnpm global-flag strip — flag-first pnpm never rewrites |
| 3246 | c | `registry.rs:1177` `command` bypass prefix swallowed — escape hatch rewritten |
| 3266 | a (minor) | `tee.rs:19` 40-char slug truncation collides recovery files |
| 3344 | c (partial) | tee `~` hint broken under Git Bash (`tee.rs:189-196`); Tier-3 parse failures untracked; hunk 3 superseded |
| 3325 / 3271 | a,c | vitest — noted on ticket #8 instead |
| 3355 / 3509 / 3523 | a (off-profile) | pytest truth cluster: summary-line hijack (`pytest_cmd.rs:108-116`), zero-counts → "No tests collected" (`:195-197`), missing footer (`:182`); python-side, below the JS profile |
| 3353 | a/b | never_worse can discard the FAIL banner (`guard.rs:6-12` + `rust/runner.rs:88-99`); large diff overlapping #2427/#2683/#2996 — adopt with care |
| 3336 | a (low) | `core/filter.rs:186-192` python docstring toggle latches — read -l minimal drops code |
| 3462-adjacent | — | (3462 filed in #108) |
| 3438 | c (half) | `discover/provider.rs:200` matches only "Bash" tool — PowerShell sessions under-report; colon half superseded |
| 3439 | c (feature) | no PowerShell cmdlet translation in registry — on-profile feature, not a bug |
| 3440 | c (low) | `tracking.rs:455` history_days dead knob |
| 3317 | low | `config.rs:57-62` tracking.enabled has zero read sites |
| 3461 | c (low) | tests read developer's real config.toml (`hook_cmd.rs:251-256`) — fork-maintenance friction |
| 3374 | b (weak) | `report.rs:149` zero-session scan prints "looks good!" |
| 3373 | a | folded into #114 as reference |
| 3375 | a | folded into #101 as reference |
| 3231/3566 | — | filed in #112 |

## UNSURE

| PR | Why |
|---|---|
| 3513 | Right class (pytest/truncation truth) but 7 files, new `RTK_OUTPUT_META_V1` protocol, author states no independent review; overlaps 3523 |
| 3470 | ls silent-drop premise real (`ls.rs:265-266`) but bundles dotfile-semantics change + tee-hint feature; needs diff-level review |

## PASS (superseded on develop — verified)

| PR | Superseded by |
|---|---|
| 3548 | `discover/provider.rs:132` colon already in SANITIZED_CHARS (#2952/#2919 adoption) |
| 3563 | read --max-lines exact slice (`read.rs:180-181`, fork PR #92) + exclude head/tail (`registry.rs:1364-1370`, fork PRs #93/#94) |
| 3591 | `registry.rs:1382-1394` normalizes before exclusion matching (#3035) |
| 3577 | Profile filters already exit-code gated: pytest, mypy, tsc, cargo, vitest |
| 3378 / 3324 | fork PRs #93/#94 (`registry.rs:4885` test block) |
| 3377 | fork PR #92 (`read.rs:180-181`) |
| 3259 / 3258 | fork PR #91 freed -l/-m/-t (`main.rs:317-335`, tests :3772/:3783/:3793) |
| 3224 | fork PR #18 args_os handling (`main.rs:1307`); residual args_utils.rs path unreachable/unix-only |
| 3221 / 3295 | `tsc_cmd.rs:107-121` exit-code guard with tests |
| 3274 / 3188 | multiline block rewrite already in `registry.rs:600/:828-857` with #3319 hardening |
| 3183 | fork PR #68 machine-output raw (`git.rs:79-82`, :928-940); only `--null` spelling missing |
| 3435 | git.rs half fixed (`git.rs:1142-1156`); rest subsumed by 3451 |

## PASS (other)

Off-profile ecosystems, features, CI-only, analytics, docs, drafts:

- **Niche ecosystems**: 3533 (glab), 3521/3520 (go), 3568 (rake), 3560 (spring/ssh — ssh.toml `ssh\b` false-positive is a one-line cherry-pick if wanted), 3555/3501 (aws), 3551 (gradle), 3552 (codex), 3497 (psql), 3496 (ruff), 3503 (gemini), 3480 (lua), 3455 (cpp), 3453/3347 (Pi/OMP), 3464/3330 (opencode), 3382 (xcodebuild), 3363/3279 (dotnet), 3337 (salesforce), 3351 (minimax), 3302/3369 (kiro), 3235 (grok), 3213 (cursor), 3290 (cursor init — partly landed), 3570 (cursor migration), 3243 (copilot), 3316 (mypy polish), 3335 (python aggressive mode), 3362 (docker — parse-error fallback, no corruption)
- **Features**: 3580 (gh jsonpack), 3581/3343 (pipeline rewrite — 3343 conflicts with the fork's producer-raw design), 3515 (make), 3514 (llvm-lit), 3495 (sort), 3478 (gci), 3474 (pipe --toml), 3456 (grep modes), 3280 (stash), 3265 (git show blob filter), 3209 (read ceilings — lossy by default), 3198 (graphify), 3561 (quiet_hook_warnings), 3511 (gain stats), 3227 (parse-failure project_path), 3206 (hook_decisions telemetry), 3218 (audit labels), 3510 (avg weighting), 3360 (token estimate — errs toward raw), 3479 (db vacuum), 3228 (test hygiene)
- **CI/build-only**: 3595, 3585 (benchmark harness), 3318 (aarch64 musl), 3447, 3446, 3251 (fork-relevant CD skip — cherry-pick as housekeeping if CD noise returns), 3245 (yaml agent defs)
- **Missed-savings only**: 3583 (shell wrappers, untested)
- **Docs (auto)**: 3586, 3576, 3528, 3475, 3454, 3437, 3381, 3364, 3349, 3223, 3200
- **Drafts (auto)**: 3587, 3539, 3460, 3372, 3332, 3301, 3284, 3283, 3278, 3201
- **Bot/chore/own**: 3544 (release bot), 3240 (chore), 3203/3199 (own upstream PRs)

## Next sweep

Watermark: **#3596**.
