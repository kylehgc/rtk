# Upstream PR Adoption — Batch 2 shortlist (2026-07-19)

Next 20 PRs to consider adopting, drawn from the [2026-07-17 triage](adoption-triage-2026-07-17.md) (rankings re-checked against upstream state today). Ranked for this fork's profile: Windows + Claude Code + js stack.

**Filed as fork issues (2026-07-19)**: items 1–4 were already covered by open issues [#7](https://github.com/kylehgc/rtk/issues/7) (git log --stat) and [#8](https://github.com/kylehgc/rtk/issues/8) (vitest cluster); items 5–20 are now issues [#19](https://github.com/kylehgc/rtk/issues/19)–[#34](https://github.com/kylehgc/rtk/issues/34), filed in the order listed below.

**Upstream drift check (2026-07-19)**: since the sweep, upstream merged only [#2472](https://github.com/rtk-ai/rtk/pull/2472) (supersedes candidate #2602 — kubectl Copilot template) and [#1478](https://github.com/rtk-ai/rtk/pull/1478) (Kimi agent), and closed stale #860/#881. None of the 20 below were touched. Per [ADR 0001](../docs/adr/0001-merge-based-tracking-fork.md): **repro-before-adopt is still mandatory at adoption time** — this list is a queue, not a verification.

## The 20

### Carryover from first batch (already verified 07-17)

| # | PR | Why |
|---|---|---|
| 1 | [#3028](https://github.com/rtk-ai/rtk/pull/3028) git: keep every commit in `git log --stat` | Live-verified on develop: 8/30 commits dropped. Daily-driver filter, low conflict risk |
| 2–4 | vitest cluster: [#1922](https://github.com/rtk-ai/rtk/pull/1922) metadata cmds + [#2982](https://github.com/rtk-ai/rtk/pull/2982) exit codes + [#1497](https://github.com/rtk-ai/rtk/pull/1497) shim recursion | One adoption PR, order 1922 → 2982 → 1497. #2982/#1497 need hand-port (predate develop's vitest refactor); consider dropping #1497's node shim |

### Hook / Claude Code correctness & security

| # | PR | Why |
|---|---|---|
| 5 | [#2535](https://github.com/rtk-ai/rtk/pull/2535) hook: accept current Claude tool input keys | Hook misses current Claude payload shape (`input` vs `tool_input`) — directly affects daily use; regression test |
| 6 | [#2475](https://github.com/rtk-ai/rtk/pull/2475) hook: prevent flag injection in rtk-rewrite | Missing `--` lets hyphen-leading commands feed clap help back as a "rewrite"; security-flavored, tested |
| 7 | [#2565](https://github.com/rtk-ai/rtk/pull/2565) hook: fail open when stdin payload stalls | 1s deadline so the hook can never hang Claude; core robustness |
| 8 | [#2483](https://github.com/rtk-ai/rtk/pull/2483) tracking: record stats for hook-rewritten commands | `busy_timeout=0` drops contended SQLite writes — hook usage invisible in `rtk gain`; RED→GREEN test |
| 9 | [#1985](https://github.com/rtk-ai/rtk/pull/1985) hooks: match multi-token exclude_commands prefixes | `exclude_commands = ["git diff"]` silently ignored; 8 tests + manual repro |
| 10 | [#2887](https://github.com/rtk-ai/rtk/pull/2887) hooks: honor exclude_commands in head/tail fast path | Config bypass; minimal fix, 3 regression tests. Cluster: supersedes #2396/#2612 — pick one |

### Windows

| # | PR | Why |
|---|---|---|
| 11 | [#2952](https://github.com/rtk-ai/rtk/pull/2952) discover: sanitize drive-letter colon | Windows discover finds 0 sessions; 1-char root cause, TDD, Windows-verified. Dups: #3007/#2368/#3043 |
| 12 | [#2830](https://github.com/rtk-ai/rtk/pull/2830) proxy: stop blocking on orphaned stdio pipes | Real hang, failing-before test, Windows-verified |
| 13 | [#2321](https://github.com/rtk-ai/rtk/pull/2321) resolve project-local node_modules/.bin tools | Exit 127 for every hook-rewritten npx local tool — js + Windows double hit |
| 14 | [#1047](https://github.com/rtk-ai/rtk/pull/1047) windows: wrap .cmd/.bat with `cmd.exe /C` | npm/pnpm shim execution on Windows (#950); 7 tests. CI was red at triage — verify first |
| 15 | [#742](https://github.com/rtk-ai/rtk/pull/742) hook: warning repeats every command on Windows | Empty-write doesn't bump mtime on Windows; 6-line fix, no tests yet |

### js stack

| # | PR | Why |
|---|---|---|
| 16 | [#1951](https://github.com/rtk-ai/rtk/pull/1951) rewrite: broaden npm AND pnpm rule patterns | `npm test`/`pnpm test` unrouted today; handlers already exist downstream. Cluster: supersedes #2677/#2664/#1204/#3087 |
| 17 | [#2593](https://github.com/rtk-ai/rtk/pull/2593) pnpm: preserve install failure output | Core pnpm bug, RED-before regression test |
| 18 | [#2232](https://github.com/rtk-ai/rtk/pull/2232) tsc: handle pretty diagnostics | ANSI/`--pretty` parsing — no more false "No errors found" on non-zero exit |

### git / gain

| # | PR | Why |
|---|---|---|
| 19 | [#2951](https://github.com/rtk-ai/rtk/pull/2951) git: preserve patch output from log commands | `-p`/`--patch` passthrough; e2e byte-match test; complements #3028 |
| 20 | [#1978](https://github.com/rtk-ai/rtk/pull/1978) gain: cap per-call saved tokens at tool-result max | Fixes absurd 1.4B-token gain figures; 4 tests, idempotent DB migration |

## Adoption-time flags

- **#2571** (keep ask on mixed compound rewrites) deliberately left out: likely overlaps our adopted #3031 port (`e93cde8`). Re-check against our hook code before considering.
- **#2274** (only rewrite last pipe segment) left out: overlaps our adopted #2965 pipe-skip generalization (`e95207d`/`e1e37fd`). Re-check what remains unfixed.
- **#2887** vs already-adopted work: none — but it's a duplicate cluster (#2396 best-TDD, #2612); adopt exactly one.
- **New PR flood 07-19/07-20**: `lntutor` opened ~24 PRs in one evening, many duplicating existing candidates (#3083≈#2670, #3084≈#2473, #3085≈#2635, #3086≈#2715, #3087≈#2677). Treat as low-trust dups; prefer the originals already triaged.
- **Worth a look next sweep**: [#3067](https://github.com/rtk-ai/rtk/pull/3067) (js parser fallbacks preserve failed output), [#3057](https://github.com/rtk-ai/rtk/pull/3057) (argv boundary preservation — breaking-change flagged), [#3041](https://github.com/rtk-ai/rtk/pull/3041) (exclude_commands on resolved tool — interacts with #9/#10 above).
