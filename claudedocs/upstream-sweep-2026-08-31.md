# Upstream sweep — 2026-08-31

Incremental triage of rtk-ai/rtk open PRs above the previous watermark.

- **Coverage**: open upstream PRs **#3600–#3790** (watermark from `upstream-sweep-2026-08-17.md` was #3596). 95 PRs swept.
- **Verdicts**: 24 CANDIDATE (verified live) · 4 UNSURE · 67 PASS (11 auto-pass: docs/drafts/release-bot/own; ~10 maintainer-authored deferred to sync; 4 verified superseded; rest off-profile/features).
- **Tickets filed**: fork **#135–#148** (14 tickets covering 20 upstream PRs) + 5 cross-reference comments on existing #8, #34, #101, #110, #114.
- Ran immediately after the 2026-08-31 Conflicted Sync (fork PR #134, upstream through `9b16854`); all verification against post-sync develop. Classification ran as 8 parallel agents.

## Tickets filed

| Fork issue | Upstream PRs | Area |
|---|---|---|
| #135 | 3772 | stdout-only filters swallow stderr on success exits |
| #136 | 3754 | tsc informational output replaced by canned line |
| #137 | 3753 | passing test named FAIL booked as failure |
| #138 | 3643 + 3670 + 3651 (ref 3616) | ls drops -d operands / -R headers / symlink targets |
| #139 | 3682 | git status -s trailing newline undercount |
| #140 | 3631 + 3601 | commit failure reads as success; `--` pathspec lost |
| #141 | 3607 | external diff drivers → empty git diff |
| #142 | 3614 | setup nag on the hook protocol channel |
| #143 | 3666 + 3664 (residual) | grep -f pattern source; -h claimed by clap |
| #144 | 3636 + 3648 (bug only) | hook-detection false negatives incl. Windows paths |
| #145 | 3674 | allow rules prefix-match fail-open |
| #146 | 3606 | npm buffers dev-server output until exit |
| #147 | 3617 | Windows absolute paths never classify |
| #148 | 3618 (seq. before #111) | hardcoded cmd /C shell dispatch |

Comments: #8 ← 3680 (vitest `run` injection; #2982 slice partially covered on develop — re-verify at adoption) · #34 ← 3635 (capture-time cap alternative) · #101 ← 3662 (alt implementation; sites moved to git.rs:582-596) · #110 ← 3728 (quoting companion to noglob) · #114 ← 3658 (broader, `--ignore-vcs`).

## Full verdict table

| PR | Verdict | Reason |
|----|---------|--------|
| 3790 | PASS | auto: docs (Spanish README) |
| 3789 | PASS | SignPath signing tied to upstream's cert/account |
| 3788 | PASS | auto: own (fork's diff region parser sent upstream) |
| 3782 | PASS | maintainer (KuSh); edition-2024 chore, arrives via sync |
| 3781 | PASS | auto: draft |
| 3779 | PASS | off-profile 13.5k-line Windows-CMD/MCP feature |
| 3778 | PASS | windows-gnu target the fork never builds |
| 3777 | PASS | auto: own (jest --reporters, fork PR #133) |
| 3775 | PASS | maintainer (KuSh); CI action pinning |
| 3774 | PASS | maintainer (KuSh); benchmark measurement |
| 3773 | PASS | maintainer (KuSh); golangci benchmark, Go-stack |
| 3772 | **CANDIDATE** | → #135 (rank-1 despite KuSh authorship) |
| 3763 | PASS | maintainer (KuSh); CI path filtering |
| 3758 | PASS | maintainer (KuSh); test DB hygiene |
| 3755 | PASS | python launcher routing, off-profile |
| 3754 | **CANDIDATE** | → #136 |
| 3753 | **CANDIDATE** | → #137 |
| 3752 | PASS | maintainer (arkgum); worktree repair/list — real, re-check at next sync (git.rs:2269-2306 still allowlists without repair, drops --porcelain/-z) |
| 3749 | PASS | maintainer (KuSh); exclude_commands peeling |
| 3747 | PASS | auto: docs |
| 3728 | **CANDIDATE** | → comment on #110 (MSYS quoting companion) |
| 3726 | PASS | DUP #30 (npm lifecycle; same ground as 3672) |
| 3725 | PASS | off-profile k8s feature |
| 3724 | PASS | feature; broadens rewrite blast radius on #108's surface |
| 3721 | PASS | auto: release bot |
| 3716 | PASS | superseded: strip_rg_replace (search.rs:258-290, :594-597) |
| 3715 | PASS | gain observability feature |
| 3714 | PASS | off-profile TOML filters (journalctl etc.) |
| 3713 | PASS | off-profile ZCode agent |
| 3712 | PASS | cosmetic missing-table hint, not worth divergence |
| 3711 | PASS | off-profile Antigravity CLI |
| 3709 | PASS | real (stream.rs:271-282) but unix-only — inert on fork profile |
| 3707 | PASS | off-profile Oh My Pi |
| 3706 | PASS→note | real 7-line init-instruction fix (`init.rs:126` unqualified "always prefix rtk" → `rtk env X=Y cmd` loses filter); small enough to fold into any hooks adoption — recorded here, no ticket |
| 3705 | PASS | dep bumps; quick-xml advisory only reaches dotnet TRX |
| 3704 | PASS | maintainer (KuSh); lexer refactor, arrives via sync |
| 3701 | PASS | off-profile session-summary feature |
| 3700 | UNSURE | install.sh rejects MINGW — real but installer-only |
| 3699 | PASS | auto: docs |
| 3698 | PASS | superseded: find emits via emit_guarded/println (dup 3660) |
| 3692 | PASS | off-profile mysql filter |
| 3689 | UNSURE | mostly superseded by fork PR #92; live slice: `run_stdin` lacks the empty-filter raw fallback `run` has (read.rs:42-50 vs 112-129) — 6-line fork guard, no ticket |
| 3688 | PASS | analytics-only miscount; risky shared-lexer rename |
| 3682 | **CANDIDATE** | → #139 |
| 3681 | PASS | maintainer (KuSh); arg_tokenizer module, arrives via sync |
| 3680 | **CANDIDATE** | → comment on #8 |
| 3678 | PASS | superseded: fork PR #92 head_truncate (read.rs:180-201) |
| 3676 | PASS | DUP #30; boundary fix already local (rules.rs:82) |
| 3674 | **CANDIDATE** | → #145 |
| 3672 | PASS | DUP #30 |
| 3670 | **CANDIDATE** | → #138 |
| 3668 | PASS | feature: head space-form rewrite arms, coverage gap only |
| 3666 | **CANDIDATE** | → #143 |
| 3664 | **CANDIDATE** (residual) | → #143; main fix superseded by 9b16854 |
| 3662 | PASS | DUP #101 → comment |
| 3660 | PASS | superseded: emit_guarded newline-terminates |
| 3658 | PASS | DUP #114 → comment |
| 3655 | PASS | off-profile dotnet routing |
| 3653 | PASS | off-profile docker-compose v1 routing |
| 3651 | **CANDIDATE** | → #138 |
| 3648 | **CANDIDATE** (bug only) | → #144; proposed patch is itself wrong |
| 3647 | PASS | auto: docs (INSTALL verification step) |
| 3645 | PASS | test hygiene refactor |
| 3643 | **CANDIDATE** | → #138 |
| 3642 | PASS | Copilot-only cosmetic warning, off-profile |
| 3641 | PASS | auto: docs |
| 3639 | PASS | docs/help-string only |
| 3638 | PASS | test hygiene refactor (conflicts with 3645) |
| 3637 | PASS | policy feature; kills rewriting on everyday js compounds |
| 3636 | **CANDIDATE** | → #144 |
| 3635 | PASS | DUP #34 → comment (broader alternative) |
| 3633 | PASS | off-profile Nix ecosystem |
| 3631 | **CANDIDATE** | → #140 |
| 3630 | PASS | Copilot repo-hook only |
| 3629 | PASS | off-profile sqlite3 filter |
| 3627 | PASS | auto: draft docs |
| 3623 | PASS | packaging bound to upstream identity (winget/scoop/choco) |
| 3622 | UNSURE | Windows test coverage (5 suites still `#![cfg(unix)]`) — real gap, no user-visible bug; revisit if Windows regressions bite |
| 3621 | UNSURE | PATH-fallback stderr noise real (utils.rs:405-412) but PR is an opt-in feature |
| 3620 | PASS | PowerShell cmdlet rules never fire via git-bash hook path |
| 3619 | **CANDIDATE** (slices) | → folded into #144's area notes: BOM slice already fixed locally; live: audit reader/writer dir divergence (hook_audit_cmd.rs:12-15 vs hook_cmd.rs:638) |
| 3618 | **CANDIDATE** | → #148 |
| 3617 | **CANDIDATE** | → #147 |
| 3616 | **CANDIDATE** (deferred) | native Windows ls/tree/wc, +1503 lines — referenced from #138, adopt only after its upstream review settles |
| 3615 | PASS | U+FFFD assertion — not reproduced on the fork machine (full suite green 2026-08-31) |
| 3614 | **CANDIDATE** | → #142 |
| 3613 | PASS | gnu-toolchain stack flag; fork builds MSVC only |
| 3607 | **CANDIDATE** | → #141 |
| 3606 | **CANDIDATE** | → #146 |
| 3602 | PASS | auto: docs (star chart) |
| 3601 | **CANDIDATE** | → #140 |
| 3600 | PASS | lone-CR edge; local divergence errs safe, off-profile input |

## Notes for the next sweep

- Maintainer-authored keepers to re-check landed via sync: 3752 (worktree repair), 3749, 3758, 3772 (if #135 not yet done), 3704, 3681.
- 3706's init-instruction line and 3689's `run_stdin` guard are sub-ticket-size fork amendments — fold into neighboring adoptions.
