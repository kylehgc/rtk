# Upstream PR Adoption Triage — 2026-07-17

Full sweep of **832 open PRs** on rtk-ai/rtk (see [ADR 0001](../docs/adr/0001-merge-based-tracking-fork.md) and [CONTEXT.md](../CONTEXT.md) for the adoption model).

**Coverage**: 485 deep-reviewed (all fix/unprefixed/test/perf PRs) + 278 feature PRs shortlisted by a dedicated pass + 69 auto-passed on metadata (drafts, dependabot, docs/chore/ci-only). Every PR number links to upstream.

**Verdict totals**: 263 CANDIDATE · 112 SECOND-WAVE (+16 shortlisted features) · 3 UNSURE · 107 PASS · 69 auto-pass.

263 candidates is far more than upstream's backlog "should" contain — the community is fixing real bugs and nothing is landing. Adoption order matters more than adoption bar.

## Recommended first batch (8 tickets)

Ranked by: core-area impact for a Windows + Claude Code + js-stack user, quality of evidence, freshness (low conflict risk). Related PRs batched per our one-PR-per-adoption-with-batching rule.

| Rank | Adopt | Why |
|---|---|---|
| 1 | [#3031](https://github.com/rtk-ai/rtk/pull/3031) hook: emit ask decision for Claude rewrites | Core hook correctness/security; root-caused, regression tests; supersedes older dups #2571/#3012 |
| 2 | [#2263](https://github.com/rtk-ai/rtk/pull/2263) permissions: newline-separated commands bypass allow-rules | Security bypass in the hook permission path; small lexer fix, excellent analysis |
| 3 | [#2997](https://github.com/rtk-ai/rtk/pull/2997) stream: lossy-decode instead of dropping output after non-UTF-8 byte | Silent output loss affecting 6 call sites; newer equivalent of #2049 |
| 4 | [#2717](https://github.com/rtk-ai/rtk/pull/2717) core: decode process output using Windows console code page | Mojibake on Windows — you will hit this; centralized decoder, 11/11 checks |
| 5 | [#3029](https://github.com/rtk-ai/rtk/pull/3029) runner: surface failing tool's stderr under stdout-only filters | False-quiet failures in rtk err/test; failing-before tests |
| 6 | [#3028](https://github.com/rtk-ai/rtk/pull/3028) git: keep every commit in `git log --stat` | Fresh, verified 30/30; git filter is your daily driver (related: #2161) |
| 7 | vitest cluster: [#2982](https://github.com/rtk-ai/rtk/pull/2982) exit codes + [#1497](https://github.com/rtk-ai/rtk/pull/1497) shim recursion + [#1922](https://github.com/rtk-ai/rtk/pull/1922) metadata commands | Your problem area — batch as one adoption PR; #1497 also touches vitest passthrough routing |
| 8 | [#937](https://github.com/rtk-ai/rtk/pull/937) npx: passthrough unknown packages instead of routing to npm_cmd | npx routing bug adjacent to your `npx vitest` bugbear; root-caused with regression tests |

**Runners-up** (next batch material): [#3027](https://github.com/rtk-ai/rtk/pull/3027) non-UTF-8 argv panic · [#2965](https://github.com/rtk-ai/rtk/pull/2965) pipe-segment rewrites · [#2475](https://github.com/rtk-ai/rtk/pull/2475) hook flag injection · [#2565](https://github.com/rtk-ai/rtk/pull/2565) hook stdin stall fail-open · [#2830](https://github.com/rtk-ai/rtk/pull/2830) proxy pipe hang (Windows-verified) · [#2952](https://github.com/rtk-ai/rtk/pull/2952) Windows discover drive-colon · [#2483](https://github.com/rtk-ai/rtk/pull/2483) hook tracking stats · [#1978](https://github.com/rtk-ai/rtk/pull/1978) gain absurd-figures cap · [#1951](https://github.com/rtk-ai/rtk/pull/1951) npm/pnpm rewrite coverage · [#2951](https://github.com/rtk-ai/rtk/pull/2951) git log -p passthrough.

## Duplicate clusters (adopt ONE per cluster; ticket should reference the rest)

- **git --no-merges data loss**: #1856 (smallest root fix) / #2328 / #2264 / #2016 / #1302 (--reverse variant)
- **exclude_commands ignored for head/tail**: #2396 (best TDD) / #2887 / #2612
- **pytest "No tests collected" family**: #2963 (ANSI) / #2850 / #2520 / #2006 / #925 / #2141 — overlapping but not identical; adopt #2963 + #2520 first, re-test the rest
- **pytest xfailed**: #673 / #1294
- **config serde(default)**: #1337-adjacent trio #1544 / #1483 / #843 — different sections, check overlap
- **Python docstring MinimalFilter**: #1337 (supersedes) / #2713 / #1793 / #2397
- **stream UTF-8 drop**: #2997 (newer) / #2049
- **learn cancellations**: #3024 (newer) / #1662
- **npm subcommand injection**: #2677 / #2664 (same fix) / #1204 / #1951 (broader)
- **Windows discover drive-colon**: #2952 (preferred) / #3007 / #2368
- **ccusage period field**: #2341 / #1968
- **git log -n<N>**: #2740 (smaller) / #2666
- **ruff subcommand routing**: #2679 / #2670
- **cursor init standalone**: #3000 (best of 3) / #1840 / #3014
- **ls locale dates**: #1358 / #1689
- **grep tool-identity mega-cluster**: #2183 (design) vs #2460 (minimal) vs #2434 / #2254 / #2126 — needs a deliberate pick, likely #2460 first
- **find unsupported predicates**: #2160 / #676 / #2014 / #2824 / #2453 / #2090 — philosophies differ (fallback vs fail-closed); decide policy before adopting

## All candidates (263, newest first)

| PR | Title | Verdict | Tests | Area | Reason |
|---|---|---|---|---|---|
| [#3031](https://github.com/rtk-ai/rtk/pull/3031) | fix(hook): emit ask decision for Claude rewrites | CANDIDATE | tests:y | hook | Root-caused: default-to-ask path lost rewrites; regression tests both modes; fixes #3018 |
| [#3029](https://github.com/rtk-ai/rtk/pull/3029) | fix(runner): surface a failing tool's stderr under stdout-only filte | CANDIDATE | tests:y | runner | Excellent root cause (stdout_only drops stderr on failure), failing-before tests, real uv repro |
| [#3028](https://github.com/rtk-ai/rtk/pull/3028) | fix(git): keep every commit in git log --stat output | CANDIDATE | tests:y | git | Parser marker fix (--END-- vs diffstat ordering), hash-set verified 30/30, honest savings note |
| [#3027](https://github.com/rtk-ai/rtk/pull/3027) | fix(cli): handle non-UTF-8 argv in raw-execution fallback | CANDIDATE | tests:y | cli/UTF-8 | Fallback path panicked via env::args(); OsString fix, 3 failing-before tests, byte-identical regression check |
| [#3024](https://github.com/rtk-ai/rtk/pull/3024) | fix(learn): ignore parallel tool call cancellations | CANDIDATE | tests:y | learn | Cancellation payload matched 'errored' and generated false Use-X-not-Y rules; regression test from real payload |
| [#3023](https://github.com/rtk-ai/rtk/pull/3023) | fix(go): report failure when build produces unrecognized error outpu | CANDIDATE | tests:y | go | Silent success on unmatched go build errors; small, tested, fixes #1599 |
| [#3019](https://github.com/rtk-ai/rtk/pull/3019) | fix(find): preserve native type defaults | CANDIDATE | tests:? | system/find | Files-only default silently emptied directory searches; small, matches native find semantics |
| [#3017](https://github.com/rtk-ai/rtk/pull/3017) | fix(hook): preserve explicit linter rewrites | CANDIDATE | tests:? | hook | Small targeted fix keeping explicit biome/eslint in rewrites; thin body but issue-linked and tiny |
| [#3002](https://github.com/rtk-ai/rtk/pull/3002) | fix(err,test): preserve argument quoting and exit codes in rtk err/t | CANDIDATE | tests:y | runner/Windows | argv.join flattened quoting; shell re-quote + Windows raw_arg; live-verified exit codes |
| [#3001](https://github.com/rtk-ai/rtk/pull/3001) | fix(grep): tee log for +N more overflow now holds full untruncated l | CANDIDATE | tests:y | grep/tee | Recovery log was as truncated as display; keeps raw text for tee; repro from issue fixture |
| [#3000](https://github.com/rtk-ai/rtk/pull/3000) | fix(init): --agent cursor installs Cursor only and creates ~/.cursor | CANDIDATE | tests:y | init | Best of the 3 cursor-init PRs: fixes both install_claude gating and missing ~/.cursor, e2e proven |
| [#2997](https://github.com/rtk-ai/rtk/pull/2997) | fix(stream): decode lossily instead of dropping lines on invalid UTF | CANDIDATE | tests:y | stream/UTF-8 | map_while(Result::ok) silently dropped all output after one non-UTF-8 line; fixes shared path for 6 callers |
| [#2982](https://github.com/rtk-ai/rtk/pull/2982) | fix: report nonzero vitest process exits | CANDIDATE | tests:? | js/vitest | False-green when vitest exits nonzero with clean JSON report; small, core js area |
| [#2965](https://github.com/rtk-ai/rtk/pull/2965) | fix(hooks): don't rewrite pipe-feeding segments whose filter changes | CANDIDATE | tests:y | hook/rewrite | Generalizes find/fd pipe-skip to all shape-changing filters; fixes wc -l miscounts; thorough tests |
| [#2964](https://github.com/rtk-ai/rtk/pull/2964) | fix(read): fill the --max-lines budget exactly instead of stopping a | CANDIDATE | tests:y | system/read | smart_truncate returned half the requested lines; tight fix with budget-invariant tests |
| [#2963](https://github.com/rtk-ai/rtk/pull/2963) | fix(pytest): stop false 'No tests collected' on ANSI and double-quie | CANDIDATE | tests:y | python/pytest | ANSI broke summary parsing (failing run shown as passed); +4 lines behavior, e2e verified |
| [#2952](https://github.com/rtk-ai/rtk/pull/2952) | fix(discover): sanitize drive-letter colon so Windows discover finds | CANDIDATE | tests:y | discover/Windows | One-char root cause (missing ':' in SANITIZED_CHARS), TDD, Windows-verified; prefer over dup #3007 |
| [#2951](https://github.com/rtk-ai/rtk/pull/2951) | fix(git): preserve patch output from log commands | CANDIDATE | tests:y | git | -p/--patch passthrough so patch bodies aren't filtered away; e2e byte-match test; complements #3028 |
| [#2970](https://github.com/rtk-ai/rtk/pull/2970) | fix(telemetry): honor configured database path on forget | CANDIDATE | tests:y | tracking | forget deleted platform-default DB ignoring RTK_DB_PATH/config; small with test |
| [#2917](https://github.com/rtk-ai/rtk/pull/2917) | fix(discover): make git global-opt stripping quote-aware | CANDIDATE | tests:y | hook/rewrite+discover | Root cause traced, quote-aware lexer reuse, CI 11/11, manual repro verified |
| [#2871](https://github.com/rtk-ai/rtk/pull/2871) | fix(search): close stdin on search subprocesses to prevent hang in ag | CANDIDATE | tests:n | search/agent | 6-line Stdio::null() fix mirroring merged #979; fixes real agent hangs |
| [#2899](https://github.com/rtk-ai/rtk/pull/2899) | fix: preserve grep patterns in hook rewrites | CANDIDATE | tests:y | hook/rewrite | Fixes #880 grep alternation split by rewrite; regression tests + full gate |
| [#2830](https://github.com/rtk-ai/rtk/pull/2830) | fix(proxy): stop blocking on orphaned stdio pipes after the child exi | CANDIDATE | tests:y | proxy | Real hang bug, failing-before test, Windows-verified, clear mechanism |
| [#2877](https://github.com/rtk-ai/rtk/pull/2877) | fix(cargo): preserve compiler warnings in cargo test output on passin | CANDIDATE | tests:y | cargo/runner | Filter silently hid warnings on exit-0 runs; mirrors build handler, tested |
| [#2850](https://github.com/rtk-ai/rtk/pull/2850) | fix(pytest): surface collection errors instead of 'No tests collected | CANDIDATE | tests:y | pytest filter | Collection errors/version output swallowed; real fixtures, thorough body |
| [#2920](https://github.com/rtk-ai/rtk/pull/2920) | fix(discover): match Windows project slugs case-insensitively | CANDIDATE | tests:y | discover/Windows | Small Windows drive-case fix with regression test, fixes #2919 |
| [#2889](https://github.com/rtk-ai/rtk/pull/2889) | fix(discover): sanitize absolute -p paths before matching project dir | CANDIDATE | tests:y | discover | Root-caused false all-clear on -p abs paths; 5 unit tests, better of dup pair |
| [#2887](https://github.com/rtk-ai/rtk/pull/2887) | fix(hooks): honor exclude_commands for head/tail line-range fast path | CANDIDATE | tests:y | hook/rewrite | Config bypass in fast path; minimal fix, 3 regression tests (vs larger #2825) |
| [#2910](https://github.com/rtk-ai/rtk/pull/2910) | fix(grep): strip bare -E before forwarding to ripgrep | CANDIDATE | tests:y | grep filter | rg treats -E as --encoding, silent failure; tiny fix with tests |
| [#2894](https://github.com/rtk-ai/rtk/pull/2894) | fix(find): support multiple path arguments | CANDIDATE | tests:y | find filter | Silently dropped extra paths (unsafe for agents); CI 11/11, dedup covered |
| [#2852](https://github.com/rtk-ai/rtk/pull/2852) | fix(find): round dir-display truncation up to a char boundary (fixes | CANDIDATE | tests:y | find/UTF-8 | Panic on multi-byte slice; uses ceil_char_boundary like sibling fixes |
| [#2868](https://github.com/rtk-ai/rtk/pull/2868) | fix(ls): preserve per-directory headers for multiple directory operan | CANDIDATE | tests:y | ls filter | Wrong-dir attribution bug; pins single-dir output, better of dup pair |
| [#2909](https://github.com/rtk-ai/rtk/pull/2909) | fix(next): drop redundant leading build arg | CANDIDATE | tests:? | js/next | Double-build made successful builds fail; trivial, tests easily added |
| [#2895](https://github.com/rtk-ai/rtk/pull/2895) | fix(build): emit GNU-compatible stack linker arg on windows-gnu | CANDIDATE | tests:n | build/Windows | One-line build.rs fix, MinGW link failure shown before/after; no test possible |
| [#2824](https://github.com/rtk-ai/rtk/pull/2824) | fix(find): fail closed on unknown flags instead of warn-and-broaden | CANDIDATE | tests:y | find/exit-codes | Silent superset results on dropped predicates; small, RED-before tests |
| [#2818](https://github.com/rtk-ai/rtk/pull/2818) | fix(gain): remove negative recent savings sign | CANDIDATE | tests:y | gain display | Best of 3 dups for #2815: same 1-char fix plus display tests |
| [#2810](https://github.com/rtk-ai/rtk/pull/2810) | fix(rewrite): stop rewriting sudo commands (pass them through) | CANDIDATE | tests:y | hook/rewrite | Root-caused sudo breakage under secure_path; verification table; suite green |
| [#2804](https://github.com/rtk-ai/rtk/pull/2804) | fix(init): upsert GEMINI.md instead of clobbering user content | CANDIDATE | tests:y | init | Data-loss fix reusing existing write_rtk_block; unit + e2e verified |
| [#2796](https://github.com/rtk-ai/rtk/pull/2796) | fix(gh): classify pr checks by status column, not name substring | CANDIDATE | tests:y | gh filter | Real miscount bug (names containing pass/fail; skips dropped); 3 unit tests |
| [#2790](https://github.com/rtk-ai/rtk/pull/2790) | Fix #2762: `rtk gain` reports unreproducible savings — read/grep/tail | CANDIDATE | tests:y | tracking/gain | Diff confirms baseline was full content vs user-requested window; integ tests |
| [#2770](https://github.com/rtk-ai/rtk/pull/2770) | fix(rewrite): strip GNU timeout prefix before rewriting inner comman | CANDIDATE | tests:y | hook/rewrite | Mirrors existing strip-prefix contract; 13 unit tests; byte-safe slicing |
| [#2757](https://github.com/rtk-ai/rtk/pull/2757) | fix(next): drop redundant leading `build` arg so `rtk next build` ex | CANDIDATE | tests:y | js/next | Root cause reproduced empirically (next build build); small pure fn + test |
| [#2740](https://github.com/rtk-ai/rtk/pull/2740) | fix(git): handle -n<N> combined form in git log | CANDIDATE | tests:y | git filter | Clear limit-flag parsing bug; 22-line fix with tests; smaller than dup #2695 |
| [#2736](https://github.com/rtk-ai/rtk/pull/2736) | fix: propagate golangci-lint exit code in filtered run | CANDIDATE | tests:n | exit codes | 2-line inverted-exit-code fix in high-rank area; trivially testable |
| [#2717](https://github.com/rtk-ai/rtk/pull/2717) | fix(core): decode process output using Windows console code page | CANDIDATE | tests:y | Windows/UTF-8 | Mojibake on non-UTF8 code pages; centralized decoder; 11/11 checks; fork is on Windows |
| [#2716](https://github.com/rtk-ai/rtk/pull/2716) | fix(search): surface error exit codes and guard against false zero m | CANDIDATE | tests:y | grep/exit codes | Small defensive fix vs silent false "0 matches" in hook contexts |
| [#2715](https://github.com/rtk-ai/rtk/pull/2715) | fix(filter): preserve code around inline block comments | CANDIDATE | tests:y | core filter | Code-loss bug in MinimalFilter; before/after table; 3 tests |
| [#2713](https://github.com/rtk-ai/rtk/pull/2713) | fix(filter): handle Python single-line docstrings in MinimalFilter | CANDIDATE | tests:y | core filter | Clear state-corruption bug; tiny fix with regression test |
| [#2711](https://github.com/rtk-ai/rtk/pull/2711) | fix(tracking): parse RFC3339 timestamps in first_seen_days() | CANDIDATE | tests:y | tracking/gain | Format mismatch made function always return 0; matches existing correct usage |
| [#2709](https://github.com/rtk-ai/rtk/pull/2709) | fix(gradle): correct match_command regex to match bare gradle/gradle | CANDIDATE | tests:? | filters/toml | Impossible-regex bug (gradlegradle); 2-line fix, clearly correct |
| [#2690](https://github.com/rtk-ai/rtk/pull/2690) | fix(dotnet): parse MTP multi-line test run summary | CANDIDATE | tests:y | dotnet | .NET 10 MTP counts unavailable; fallback regex; 2 unit tests |
| [#2683](https://github.com/rtk-ai/rtk/pull/2683) | fix(test): replace generic runner last-5-lines fallback with 3-tier | CANDIDATE | tests:y | runner | Hidden failure details on unrecognized runners; exit-code aware; 4 tests |
| [#2679](https://github.com/rtk-ai/rtk/pull/2679) | fix(ruff): prevent check injection for non-check subcommands | CANDIDATE | tests:y | python/ruff | Injection misrouting broke 7 subcommands; allowlist mirrors npm pattern |
| [#2677](https://github.com/rtk-ai/rtk/pull/2677) | fix(npm): add 15 missing npm subcommands to prevent incorrect run in | CANDIDATE | tests:y | js/npm | Core npm run-injection bug (#2663); diff bloated (661 lines) but concept simple |
| [#2666](https://github.com/rtk-ai/rtk/pull/2666) | fix(git): recognize -n<N> combined form in git log limit detection | CANDIDATE | tests:y | git | Core git-log limit bug (#2665), 15 lines, root cause + regression test |
| [#2664](https://github.com/rtk-ai/rtk/pull/2664) | fix(npm): add 15 missing subcommands to prevent incorrect `run` inject | CANDIDATE | tests:y | js/npm | Real npm run-injection bug, verified vs npm help, 11/11 checks green |
| [#2659](https://github.com/rtk-ai/rtk/pull/2659) | fix(pnpm): propagate exit code from pnpm outdated | CANDIDATE | tests:? | js/pnpm | Exit-code bug (#2658), 8 lines, consistent with sibling handlers; beats dup #2661 |
| [#2655](https://github.com/rtk-ai/rtk/pull/2655) | fix(npm): correct operator precedence in progress indicator filter | CANDIDATE | tests:y | js/npm | Clear && vs || precedence bug dropping lines, tiny + regression test |
| [#2657](https://github.com/rtk-ai/rtk/pull/2657) | fix(curl): use signal-aware exit code instead of unwrap_or(1) | CANDIDATE | tests:n | exit codes | 3-line fix reusing existing status_to_exit_code helper, trivial adopt |
| [#2612](https://github.com/rtk-ai/rtk/pull/2612) | fix(hook): apply exclude_commands to head/tail rewrites | CANDIDATE | tests:y | hook/rewrite | Hook exclusion silently ignored for head/tail (#2363), small, top-rank area |
| [#2628](https://github.com/rtk-ai/rtk/pull/2628) | fix(grep): stop -l/-m/-t shadowing native grep flags | CANDIDATE | tests:y | grep | Hard exit-2 failure on grep -l; 3-line prod change + characterization tests |
| [#2607](https://github.com/rtk-ai/rtk/pull/2607) | fix(grep): translate --include/--exclude to rg --glob | CANDIDATE | tests:y | grep | Common idiom forced slow grep fallback; verified e2e, 11/12 checks (conflict flag) |
| [#2635](https://github.com/rtk-ai/rtk/pull/2635) | fix(grep): honor GNU -h/--no-filename instead of intercepting as help | CANDIDATE | tests:y | grep | Silent false-negative (help banner instead of matches); tests red-before; conflict flag |
| [#2670](https://github.com/rtk-ai/rtk/pull/2670) | fix(ruff): preserve non-check subcommands | CANDIDATE | tests:y | python/ruff | Real routing bug breaking ruff rule/config/linter (#2669), small allowlist fix |
| [#2668](https://github.com/rtk-ai/rtk/pull/2668) | fix(diff): replace naive index-based comparison with LCS algorithm | CANDIDATE | tests:y | diff | Cascading false positives (#1869), clean repro, inline LCS, no new deps |
| [#2643](https://github.com/rtk-ai/rtk/pull/2643) | fix(diff): rtk diff reports CRLF/LF-only differences as identical | CANDIDATE | tests:y | diff/Windows | CRLF-vs-LF wrongly identical (#2627); excellent root cause; CI-only verification |
| [#2652](https://github.com/rtk-ai/rtk/pull/2652) | fix(diff): remove misleading overflow indicator from condense_unified_ | CANDIDATE | tests:y | diff | "+N more" shown when nothing hidden — wastes LLM tokens; small, tests updated |
| [#2644](https://github.com/rtk-ai/rtk/pull/2644) | fix(ls): strip unsupported --depth flag instead of erroring | CANDIDATE | tests:y | system/ls | Well-scoped fix for #2365 incl. value-token path corruption; CI-only verification |
| [#2598](https://github.com/rtk-ai/rtk/pull/2598) | fix: rtk find returns empty results after git init (#2589) | CANDIDATE | tests:y | system/find | Gitignore filtering made rtk find diverge from native find; small + regression test |
| [#2615](https://github.com/rtk-ai/rtk/pull/2615) | perf(tracking): gate cleanup_old behind 24h interval | CANDIDATE | tests:y | tracking | ~7ms SQLite tax on every command (#2208); tests; conflict flag = rebase risk |
| [#2622](https://github.com/rtk-ai/rtk/pull/2622) | fix(dotnet): pass MSBuild query invocations through verbatim | CANDIDATE | tests:y | dotnet | -getProperty output destroyed by compaction; thorough detection + tests |
| [#2599](https://github.com/rtk-ai/rtk/pull/2599) | fix(tee): harden recovery-file perms against world-read and symlink at | CANDIDATE | tests:y | core/tee security | 0600/O_EXCL hardening of secret-bearing recovery files; unix-gated, tested |
| [#2602](https://github.com/rtk-ai/rtk/pull/2602) | fix: preserve kubectl get in Copilot instructions | CANDIDATE | tests:y | hooks/init | One-line template bug (#2471), regression test verified failing-before |
| [#2595](https://github.com/rtk-ai/rtk/pull/2595) | fix(rewrite): stop rewritting gradle to gradlew | CANDIDATE | tests:y | rewrite | Tiny fix, gradle/gradlew are different binaries; clear cause, fixes #2374 |
| [#2593](https://github.com/rtk-ai/rtk/pull/2593) | fix(pnpm): preserve install failure output | CANDIDATE | tests:y | js/pnpm | Core pnpm bug, RED-before regression test, CI green |
| [#2592](https://github.com/rtk-ai/rtk/pull/2592) | fix(rewrite): preserve npm tsc scripts | CANDIDATE | tests:y | rewrite/js | npm run tsc wrongly collapsed to rtk tsc; RED evidence, small |
| [#2591](https://github.com/rtk-ai/rtk/pull/2591) | fix(grep): show binary-only match lines | CANDIDATE | tests:y | grep filter | Dropped binary match lines; RED evidence, all checks pass |
| [#2590](https://github.com/rtk-ai/rtk/pull/2590) | fix(read): ignore broken pipe for partial output | CANDIDATE | tests:y | read/tracking | Panic on broken pipe fixed; solid tests but has merge conflicts |
| [#2588](https://github.com/rtk-ai/rtk/pull/2588) | fix(init): replace stale legacy hook registrations | CANDIDATE | tests:y | hook/init | Stale rtk-rewrite.sh entries not replaced; conflicts, overlaps #2558 |
| [#2587](https://github.com/rtk-ai/rtk/pull/2587) | fix(hook): suppress missing-hook warning in hook entrypoints | CANDIDATE | tests:y | hook | Hook stderr pollution fix in core hook path, integration tests |
| [#2586](https://github.com/rtk-ai/rtk/pull/2586) | fix(git): preserve explicit branch listings | CANDIDATE | tests:y | git filter | git branch -a/-r output mangled; integration test with local remote |
| [#2581](https://github.com/rtk-ai/rtk/pull/2581) | fix(stream): collapse terminal redraw controls | CANDIDATE | tests:y | core/stream | CR/backspace spinner noise reaching filters; core capture path |
| [#2579](https://github.com/rtk-ai/rtk/pull/2579) | fix(git): wire up status_max_files and status_max_untracked confi | CANDIDATE | tests:? | git status | Dead config knobs never read; real bug but conflicts, manual unchecked |
| [#2578](https://github.com/rtk-ai/rtk/pull/2578) | fix(telemetry): avoid low savings arg leakage | CANDIDATE | tests:y | tracking/telemetry | Privacy leak of paths/tokens in telemetry; small, tested |
| [#2576](https://github.com/rtk-ai/rtk/pull/2576) | fix(rewrite): preserve npm workspace run semantics | CANDIDATE | tests:y | rewrite/js | Workspace selectors broken by rewrite collapse; core npm path |
| [#2575](https://github.com/rtk-ai/rtk/pull/2575) | fix(rewrite): preserve stdin-driven commands | CANDIDATE | tests:y | rewrite | kubectl -f -, docker build - etc. broken by rewrite; safety fix |
| [#2573](https://github.com/rtk-ai/rtk/pull/2573) | fix(git): keep machine output raw | CANDIDATE | tests:y | git/rewrite | --porcelain/--format output must stay byte-exact; correctness fix |
| [#2572](https://github.com/rtk-ai/rtk/pull/2572) | fix(gemini): prefix Windows hook with Git Bash | CANDIDATE | tests:y | hook/windows | Windows .sh hook unrunnable without bash prefix; Windows ranks high |
| [#2571](https://github.com/rtk-ai/rtk/pull/2571) | fix(hook): keep ask decision on claude rewrites | CANDIDATE | tests:y | hook/security | Mixed compounds auto-approved when they should ask; small security fix |
| [#2567](https://github.com/rtk-ai/rtk/pull/2567) | fix(cargo): preserve quiet check success output | CANDIDATE | tests:y | cargo | Synthetic "0 crates compiled" on quiet success; tiny, conflicts noted |
| [#2565](https://github.com/rtk-ai/rtk/pull/2565) | fix(hook): fail open when stdin payload stalls | CANDIDATE | tests:y | hook | 1s deadline so hook can't hang Claude forever; core robustness |
| [#2560](https://github.com/rtk-ai/rtk/pull/2560) | Fail git push when remote output reports rejection | CANDIDATE | tests:y | git/exit-codes | Push rejections exit 0 today; exit-code core area, marker-based fix |
| [#2559](https://github.com/rtk-ai/rtk/pull/2559) | Stream git commit hook output while the child runs | CANDIDATE | tests:y | git commit | Commit hooks look hung (buffered output); real UX bug, conflicts |
| [#2546](https://github.com/rtk-ai/rtk/pull/2546) | fix(filter): preserve inline comment markers in code | CANDIDATE | tests:y | core/filter | read --level minimal drops lines with inline /* in strings; small |
| [#2544](https://github.com/rtk-ai/rtk/pull/2544) | grep: detect format flags bundled inside short-flag clusters | CANDIDATE | tests:y | grep filter | -rln cluster misses -l format flag, fakes "0 matches"; small, root cause clear, tests added |
| [#2542](https://github.com/rtk-ai/rtk/pull/2542) | fix(git): keep status paths relative to cwd | CANDIDATE | tests:y | git status filter | Porcelain paths repo-root relative in monorepo subdirs; root cause + full test/validation list |
| [#2539](https://github.com/rtk-ai/rtk/pull/2539) | fix(build): target-aware stack flag so windows-gnu toolchain links | CANDIDATE | tests:n | Windows build | /STACK MSVC-only flag breaks windows-gnu link; 11-line fix, verified on Win11 MinGW |
| [#2536](https://github.com/rtk-ai/rtk/pull/2536) | fix(json): tolerate raw control chars in strings instead of dropping ou | CANDIDATE | tests:y | json cmd | Strict parse dropped whole payload on raw control chars; lenient retry, 9 new tests, zero change for valid JSON |
| [#2535](https://github.com/rtk-ai/rtk/pull/2535) | fix(hook): accept current Claude tool input keys | CANDIDATE | tests:y | hook system | Hook missed current Claude payload shape (input vs tool_input); regression test, 11/12 CI |
| [#2534](https://github.com/rtk-ai/rtk/pull/2534) | fix(rewrite): strip rtk prefix from shell builtins | CANDIDATE | tests:y | rewrite system | Strips bad rtk prefix from shell builtins during rewrite; small, tested, core rewrite path |
| [#2533](https://github.com/rtk-ai/rtk/pull/2533) | fix(parser): preserve UTF-8 boundaries extracting JSON | CANDIDATE | tests:y | parser/UTF-8 | Byte-offset slice panics on CJK JSON; 18-line fix with multibyte regression test |
| [#2532](https://github.com/rtk-ai/rtk/pull/2532) | fix(grep): disable help flag to allow -h passthrough | CANDIDATE | tests:y | grep/clap | One-line disable_help_flag matching merged psql precedent (#650); clap eats -h today |
| [#2521](https://github.com/rtk-ai/rtk/pull/2521) | fix(init): create Claude config dir for global init | CANDIDATE | tests:y | init/hook | atomic_write fails on fresh machine without ~/.claude; regression tests, full gate run |
| [#2520](https://github.com/rtk-ai/rtk/pull/2520) | fix(pytest): preserve collection and loader errors | CANDIDATE | tests:y | pytest filter | Collection/loader errors wrongly shown as "No tests collected" (#2317); deterministic pytest 8/9 fixtures; medium size |
| [#2515](https://github.com/rtk-ai/rtk/pull/2515) | [codex] fix native test expression passthrough | CANDIDATE | tests:y | runner/test cmd | rtk test -d X broke via sh -c "-d X"; routes native test exprs to system test; 11/11 CI |
| [#2488](https://github.com/rtk-ai/rtk/pull/2488) | fix(gain): char-safe truncation in `gain --history` (panic on CJK) | CANDIDATE | tests:y | gain/UTF-8 | Byte slice panic on multibyte in history column; char-safe helper + regression tests |
| [#2483](https://github.com/rtk-ai/rtk/pull/2483) | fix(tracking): record stats for hook-rewritten commands (#1082) | CANDIDATE | tests:y | tracking | busy_timeout=0 dropped contended SQLite writes from hook; RED->GREEN lock-contention test |
| [#2480](https://github.com/rtk-ai/rtk/pull/2480) | fix(output): preserve tail lines to prevent CI summary loss (#1035) | CANDIDATE | tests:y | output truncation | Head-only truncation dropped CI pass/fail summaries; head+tail rewrite, RED->GREEN evidence |
| [#2479](https://github.com/rtk-ai/rtk/pull/2479) | fix(hook): emit explicit allow for non-rewritten commands (#1033) | CANDIDATE | tests:y | hook system | Silent exit 0 caused unexpected permission prompts; emits explicit allow, shell-level tests |
| [#2475](https://github.com/rtk-ai/rtk/pull/2475) | fix(hook): prevent flag injection in rtk-rewrite hooks (#1350) | CANDIDATE | tests:y | hook security | Missing -- lets hyphen-leading commands trigger clap help fed back as "rewrite"; clap + shell tests |
| [#2473](https://github.com/rtk-ai/rtk/pull/2473) | fix(diff): exit 2 on unreadable operand per POSIX diff convention | CANDIDATE | tests:y | exit codes | Missing file returned exit 1 (=differs) instead of 2; 4 unit tests, parity-checked vs system diff |
| [#2460](https://github.com/rtk-ai/rtk/pull/2460) | fix(grep): keep rg on the ripgrep path - split the rg/grep rewrite rule | CANDIDATE | tests:y | grep/rewrite | Splits rg/grep rule so rg keeps ripgrep semantics; fixes 4 open issues; minimal vs conflicting #2183 |
| [#2453](https://github.com/rtk-ai/rtk/pull/2453) | fix(discover): don't rewrite `find` with compound predicates to `rtk fi | CANDIDATE | tests:y | rewrite system | Rewrite produced a command rtk refuses to run, causing agent retry loops; 6 unit tests |
| [#2450](https://github.com/rtk-ai/rtk/pull/2450) | fix(diff): preserve POSIX exit codes | CANDIDATE | tests:y | diff/exit codes | Small, closes #2446, regression tests for all 3 exit paths, CI green |
| [#2440](https://github.com/rtk-ai/rtk/pull/2440) | fix: strip leading backslash-newline before rewrite | CANDIDATE | tests:y | hook/rewrite | Root cause at decide_hook_action entry, benefits all hook formats, tested |
| [#2438](https://github.com/rtk-ai/rtk/pull/2438) | fix(grep): report displayed counts when max cap is applied | CANDIDATE | tests:y | grep filter | Correctness of reported counts under --max, small, tested, CI green |
| [#2434](https://github.com/rtk-ai/rtk/pull/2434) | fix(grep): prevent silent fallback to system grep when rg flags prece | CANDIDATE | tests:y | grep/rewrite | Fixes #2120/#2167 silent wrong-tool fallback, 16 new tests, medium size |
| [#2430](https://github.com/rtk-ai/rtk/pull/2430) | fix(read): preserve template literals in aggressive filter | CANDIDATE | tests:y | core filter | Clear root cause (backtick state untracked), 3 tests, small |
| [#2427](https://github.com/rtk-ai/rtk/pull/2427) | fix(test): surface error lines for generic runners | CANDIDATE | tests:y | runner | Core runner area, gated on exit code to avoid false positives, 3 tests |
| [#2410](https://github.com/rtk-ai/rtk/pull/2410) | fix: avoid false prettier success output | CANDIDATE | tests:y | js/prettier | Fixes false-success reporting, small, 11/12 checks pass |
| [#2401](https://github.com/rtk-ai/rtk/pull/2401) | fix: keep release panic=unwind so filter catch_unwind fails open | CANDIDATE | tests:y | core/fail-open | Real correctness bug (catch_unwind dead in release), regression guard; checks 0/2 needs a look |
| [#2399](https://github.com/rtk-ai/rtk/pull/2399) | fix(pytest): count errors in summary instead of silently dropping the | CANDIDATE | tests:y | pytest filter | Most thorough of 3 dup PRs; repro evidence, anchored parser, 7 failing-before tests |
| [#2397](https://github.com/rtk-ai/rtk/pull/2397) | fix(read): only treat /* at line start as block comment opener | CANDIDATE | tests:y | core filter | One-word root-cause fix for file-swallowing state leak, TDD, 11/11 checks |
| [#2396](https://github.com/rtk-ai/rtk/pull/2396) | fix(hook): apply exclude_commands to head/tail rewrites | CANDIDATE | tests:y | hook/rewrite | Exact diagnosed root cause, one-guard fix, TDD, manual repro shown |
| [#2393](https://github.com/rtk-ai/rtk/pull/2393) | fix(err): pass child argv verbatim instead of re-splitting on whitesp | CANDIDATE | tests:y | runner/exit codes | Exit codes masked as success; verbatim argv fix with #388 parity tests |
| [#2378](https://github.com/rtk-ai/rtk/pull/2378) | fix(rewrite): route direct biome invocations to rtk biome instead of | CANDIDATE | tests:y | hook/rewrite | Wrong-tool routing (ESLint adapter on Biome projects), tested, repro shown |
| [#2368](https://github.com/rtk-ai/rtk/pull/2368) | fix(discover,learn): encode Windows drive colon in project path slug | CANDIDATE | tests:y | discover/Windows | Windows scans 0 sessions; 1-char root-cause fix, before/after evidence |
| [#2352](https://github.com/rtk-ai/rtk/pull/2352) | fix(cli): use clap external_subcommand for passthrough instead of run | CANDIDATE | tests:y | cli/tracking | 91% of parse_failures table is noise; idiomatic clap fix, small; verify fallback still covers flag errors |
| [#2348](https://github.com/rtk-ai/rtk/pull/2348) | fix(grep): use PCRE-converted pattern with -E flag in grep fallback | CANDIDATE | tests:n | grep fallback | 2-line fix, wrong-regex-dialect silent zero matches; tests easily added |
| [#2341](https://github.com/rtk-ai/rtk/pull/2341) | fix(ccusage): accept period field in economics | CANDIDATE | tests:y | ccusage/gain | Fixes 2 issues (ccusage 20.x period field), serde alias, 11/12 checks |
| [#2334](https://github.com/rtk-ai/rtk/pull/2334) | fix(rewrite): passthrough native grep/ls when short flags collide wit | CANDIDATE | tests:y | hook/rewrite | grep -v silently inverts results; conservative skip-rewrite guard, tested |
| [#2328](https://github.com/rtk-ai/rtk/pull/2328) | fix(git): preserve merge commits in log output | CANDIDATE | tests:y | git filter | Injected --no-merges hid HEAD merge commits; regression tests + smoke |
| [#2327](https://github.com/rtk-ai/rtk/pull/2327) | fix(hook): rewrite Windows python module invocations | CANDIDATE | tests:y | hook/Windows | python.exe / venv paths bypassed pytest/mypy rewrites; well validated; 1/4 checks needs a look |
| [#2322](https://github.com/rtk-ai/rtk/pull/2322) | fix(core): stop capture hanging on orphaned pipes | CANDIDATE | tests:y | core/stream | Indefinite hang when grandchild holds pipes; regression test; larger single-file change |
| [#2321](https://github.com/rtk-ai/rtk/pull/2321) | fix(resolve): resolve project-local node_modules/.bin tools (fixes ex | CANDIDATE | tests:y | js/Windows | Exit 127 for every hook-rewritten npx local tool; mirrors npx resolution, tested |
| [#2309](https://github.com/rtk-ai/rtk/pull/2309) | fix(git): rewrite -C with quoted paths | CANDIDATE | tests:y | hook/rewrite | Root cause + repro on current develop, shell-lexer fix, full validation, small |
| [#2308](https://github.com/rtk-ai/rtk/pull/2308) | fix(discover): use recorded hook rewrites | CANDIDATE | tests:y | discover | Uses actual recorded PreToolUse rewrites; independently validated on 178 real sessions |
| [#2306](https://github.com/rtk-ai/rtk/pull/2306) | fix(lint): mark truncated parse fallbacks | CANDIDATE | tests:y | lint (ruff/golangci) | Tiny follow-up to merged #2204, visible passthrough markers, regression tests |
| [#2296](https://github.com/rtk-ai/rtk/pull/2296) | fix(git): preserve patch content when `git log -p` is used | CANDIDATE | tests:y | git filter | Core faithfulness bug, RED->GREEN evidence, 10/11 CI green; better than #2276 |
| [#2284](https://github.com/rtk-ai/rtk/pull/2284) | fix(go): preserve test and vet diagnostics | CANDIDATE | tests:y | go filter | Failures no longer summarized as empty; exit-aware filtering, all 11 checks pass |
| [#2274](https://github.com/rtk-ai/rtk/pull/2274) | fix(registry): only rewrite last pipe segment; prevent data corr | CANDIDATE | tests:y | hook/rewrite | Fixes real data corruption (jq parse fail, broken git patches); 14 tests, 11/12 CI |
| [#2265](https://github.com/rtk-ai/rtk/pull/2265) | fix(find): support -print flag as no-op | CANDIDATE | tests:y | find filter | Tiny, clear root cause, spurious warning removed, 2 tests |
| [#2264](https://github.com/rtk-ai/rtk/pull/2264) | fix(git): preserve merge commits with topology flags | CANDIDATE | tests:y | git filter | --graph broken by injected --no-merges; clear repro, 5 tests, 11/12 CI |
| [#2263](https://github.com/rtk-ai/rtk/pull/2263) | fix(permissions): split commands on newlines to close allow-rule | CANDIDATE | tests:y | hook/permissions | Security bypass (newline-separated cmds auto-allowed); small lexer fix, excellent analysis |
| [#2254](https://github.com/rtk-ai/rtk/pull/2254) | fix(grep): strip grep -r/-R/-E before ripgrep to prevent silent | CANDIDATE | tests:y | grep filter | rg -r=--replace silently rewrites matches; destructive corruption, small, tested |
| [#2247](https://github.com/rtk-ai/rtk/pull/2247) | fix(lint): preserve ESLint message details | CANDIDATE | tests:y | js/lint | Small focused fix making ESLint findings actionable; cleaner than overlapping #2223 |
| [#2232](https://github.com/rtk-ai/rtk/pull/2232) | fix(tsc): handle pretty diagnostics | CANDIDATE | tests:y | js/tsc | ANSI + --pretty parsing, no more false "No errors found" on non-zero exit; 11/12 CI |
| [#2227](https://github.com/rtk-ai/rtk/pull/2227) | fix(cc-economics): gracefully degrade when ccusage JSON fields m | CANDIDATE | tests:? | gain/ccusage | Hard serde failure on ccusage schema drift -> Option fields + filter_map, small |
| [#2224](https://github.com/rtk-ai/rtk/pull/2224) | fix(grep): strip ripgrep-only flags when falling back to system | CANDIDATE | tests:n | grep fallback | Real fallback breakage (grep: invalid option -- g); small, tests easily added |
| [#2221](https://github.com/rtk-ai/rtk/pull/2221) | fix: git fetch, go build error detection fixes | CANDIDATE | tests:? | git/go | git fetch null-stdin SSH failure + go build false Success; two clear root causes |
| [#2212](https://github.com/rtk-ai/rtk/pull/2212) | fix(ruby): route rspec/rubocop version queries to passthrough | CANDIDATE | tests:y | ruby filters | Spurious JSON-parse warnings; excellent root-cause writeup, supersedes #1982 |
| [#2197](https://github.com/rtk-ai/rtk/pull/2197) | fix(rewrite): handle newline-separated commands | CANDIDATE | tests:y | hook/rewrite | Core rewrite bug (newline/CRLF separators), tests, 11/12 checks green |
| [#2016](https://github.com/rtk-ai/rtk/pull/2016) | fix(git/log): stop dropping merge commits when user pins a selection | CANDIDATE | tests:y | git | Silent wrong-SHA bug from injected --no-merges; 11 tests, 11/11 checks |
| [#2161](https://github.com/rtk-ai/rtk/pull/2161) | fix(git): preserve log stat summary lines | CANDIDATE | tests:y | git | Correctness fix for log --stat truncation, thorough tests + manual repro |
| [#2164](https://github.com/rtk-ai/rtk/pull/2164) | fix(git): preserve branch all remote refs | CANDIDATE | tests:y | git | Small fix: git branch -a lost remote refs; regression tests present |
| [#2168](https://github.com/rtk-ai/rtk/pull/2168) | fix(grep): `rtk grep -v` inverts match instead of bumping verbose (clos | CANDIDATE | tests:y | grep | Clap short-flag collision root-caused; 8 tests, careful -- handling |
| [#2183](https://github.com/rtk-ai/rtk/pull/2183) | fix(grep): respect the invoked tool — grep runs grep, rg runs ripgrep | CANDIDATE | tests:y | grep | Real semantic bug (grep -r vs rg --replace); well-designed, medium size |
| [#2149](https://github.com/rtk-ai/rtk/pull/2149) | fix(grep): skip .claude/worktrees by default | CANDIDATE | tests:y | grep | Duplicate worktree matches fixed; 8 tests incl. e2e, override preserved |
| [#2126](https://github.com/rtk-ai/rtk/pull/2126) | fix: parse leading grep search flags | CANDIDATE | tests:y | grep | Leading flags treated as pattern; tests; partial overlap with #2183 |
| [#2160](https://github.com/rtk-ai/rtk/pull/2160) | fix(find): fall back to raw find for unsupported flags | CANDIDATE | tests:y | find | Implements RTK fallback principle for -exec/-not etc; tests, exit codes |
| [#2090](https://github.com/rtk-ai/rtk/pull/2090) | fix(rewrite): only rewrite find when invocation fits compact-find gram | CANDIDATE | tests:y | hook/rewrite | Default-deny whitelist guard, 28 regression tests, fixes loud+silent fails |
| [#2184](https://github.com/rtk-ai/rtk/pull/2184) | fix(tree): use native Windows flags | CANDIDATE | tests:y | system/Windows | Windows tree.com broken by Unix -I injection; platform adapter + tests |
| [#2155](https://github.com/rtk-ai/rtk/pull/2155) | fix: non-ASCII / UTF-8 robustness (git filenames, gain truncation, pro | CANDIDATE | tests:n | UTF-8/git/gain | Three small UTF-8 fixes incl. gain byte-slice panic; no unit tests yet |
| [#2049](https://github.com/rtk-ai/rtk/pull/2049) | fix(stream): replace map_while(Result::ok) with from_utf8_lossy to pre | CANDIDATE | tests:y | core/stream/UTF-8 | Non-UTF-8 byte silently drops rest of output; regression test included |
| [#2048](https://github.com/rtk-ai/rtk/pull/2048) | fix(hooks): gate eprintln! behind RTK_HOOK_MODE to prevent hook disabl | CANDIDATE | tests:? | hook | stderr in PreToolUse disables hook; 16 sites gated, tiny diff, no tests noted |
| [#2061](https://github.com/rtk-ai/rtk/pull/2061) | fix/ls 1 flag 2058 | CANDIDATE | tests:y | system/ls | BSD ls -1 returned (empty); strips display-format flags, 15 tests |
| [#2141](https://github.com/rtk-ai/rtk/pull/2141) | fix(pytest): pass --version through unfiltered | CANDIDATE | tests:y | python/pytest | Filter swallowed version banner; narrow scope, root cause documented |
| [#2142](https://github.com/rtk-ai/rtk/pull/2142) | fix(discover): rewrite dotnet test/restore/format, not just build | CANDIDATE | tests:y | hook/rewrite | One-regex routing fix for existing handlers; tests, tiny |
| [#2165](https://github.com/rtk-ai/rtk/pull/2165) | fix(proxy): reject shell snippets in single-arg form (#2163) | CANDIDATE | tests:y | proxy | Small guard against misparsed shell metachars in proxy; 4 tests |
| [#2140](https://github.com/rtk-ai/rtk/pull/2140) | fix(aws): honor explicit --output json/yaml losslessly | CANDIDATE | tests:y | cloud/aws | Filter corrupted user-requested JSON; correctness-over-savings, tested |
| [#2078](https://github.com/rtk-ai/rtk/pull/2078) | fix: remove yadm from git rewrite rules | CANDIDATE | tests:y | hook/rewrite | yadm is not a git alias, rewrite broke it; 20-line fix with tests |
| [#2004](https://github.com/rtk-ai/rtk/pull/2004) | fix(discover): register ssh in the rewrite rule table so hook routes | CANDIDATE | tests:y | hook/rewrite | Clear root cause (verify vs RULES split), tiny, 4 tests, CI green, fixes #1654 |
| [#1985](https://github.com/rtk-ai/rtk/pull/1985) | fix(hooks/exclude): match multi-token entries against command prefix | CANDIDATE | tests:y | hook/rewrite | Core hook bug (exclude_commands "git diff" ignored), 8 tests, manual repro, fixes #1919 |
| [#1951](https://github.com/rtk-ai/rtk/pull/1951) | fix(rewrite): broaden npm AND pnpm rule patterns to cover all subcomm | CANDIDATE | tests:y | hook/rewrite/js | Core js gap (npm test/pnpm test unrouted), real 30-day data, downstream handlers already exist |
| [#1922](https://github.com/rtk-ai/rtk/pull/1922) | fix(rewrite): preserve vitest metadata commands | CANDIDATE | tests:y | hook/rewrite/js | Invalid vitest/jest rewrites (--version, --run) fixed at registry layer; small, tests |
| [#1903](https://github.com/rtk-ai/rtk/pull/1903) | Route rg --files to find | CANDIDATE | tests:y | hook/rewrite | rtk grep --files rewrite is plain broken today; conservative skip cases + regression tests |
| [#1981](https://github.com/rtk-ai/rtk/pull/1981) | fix(cmds/git/diff): preserve POSIX/git contract for programmatic con | CANDIDATE | tests:y | git filter | git apply/name-only/exit-code broken by decorations; TTY detection + 7 tests, fixes #1918/#1869 |
| [#1855](https://github.com/rtk-ai/rtk/pull/1855) | fix(git): preserve context lines before first change in compact_diff | CANDIDATE | tests:y | git filter | Tiny guard removal, regression test, clear before/after, fixes #1852 |
| [#1856](https://github.com/rtk-ai/rtk/pull/1856) | fix(git): stop injecting --no-merges into git log commands | CANDIDATE | tests:n | git filter | 8-line fix, silent data loss (merge commits hidden, --graph corrupted), fixes #1853 |
| [#1857](https://github.com/rtk-ai/rtk/pull/1857) | fix(git): preserve detached HEAD commit SHA in status output | CANDIDATE | tests:y | git filter | Small, follows existing extract_state_header pattern, 2 tests, fixes #1854 |
| [#1978](https://github.com/rtk-ai/rtk/pull/1978) | fix(analytics/gain): cap per-call saved tokens at Claude tool-result | CANDIDATE | tests:y | tracking/gain | Fixes absurd 1.4B-token gain figures (#1973/#1935), 4 tests, idempotent DB migration |
| [#1968](https://github.com/rtk-ai/rtk/pull/1968) | fix(ccusage): accept period field emitted by ccusage >=19.0 for all | CANDIDATE | tests:y | tracking/gain | rtk gain hard-fails on modern ccusage; serde alias keeps back-compat; bundles minor extras |
| [#2006](https://github.com/rtk-ai/rtk/pull/2006) | fix(pytest): surface diagnostic context when no tests are collected | CANDIDATE | tests:y | python/pytest | Root-caused, bounded 15-line diagnostics, fallback fence, 4 tests, CI green, fixes #1417 |
| [#1984](https://github.com/rtk-ai/rtk/pull/1984) | fix(cmds/go): surface failure context inline instead of hiding in te | CANDIDATE | tests:y | go runner | Panic site lines dropped by 5-line cap; trace mode + fixtures + savings tests; fixes #1882 |
| [#1906](https://github.com/rtk-ai/rtk/pull/1906) | fix(go): preserve coverage output from go test -cover on passing run | CANDIDATE | tests:y | go runner | Small, clear root cause (coverage routed to fail-only field), test, closes #1765 |
| [#1969](https://github.com/rtk-ai/rtk/pull/1969) | fix(golangci-lint): accept null source lines | CANDIDATE | tests:y | go lint | 39-line serde null-handling fix with regression test, closes #1958 |
| [#1965](https://github.com/rtk-ai/rtk/pull/1965) | fix(grep): preserve filename for single-file matches with colons | CANDIDATE | tests:y | grep filter | Small -H fix for colon-in-content parse bug, fallback kept, tests, fixes #1613 |
| [#2014](https://github.com/rtk-ai/rtk/pull/2014) | fix(find): passthrough safe native predicates | CANDIDATE | tests:y | system/find | Kills fail-then-rerun double call for read-only native find; destructive actions still blocked |
| [#1926](https://github.com/rtk-ai/rtk/pull/1926) | fix(discover): classify universal-passthrough git subcommands as sup | CANDIDATE | tests:y | discover | Cleaner general passthrough-fallback fix for #1897; 5 regression tests; 0% savings kept honest |
| [#1986](https://github.com/rtk-ai/rtk/pull/1986) | fix(aws): redact secretsmanager get-secret-value payload | CANDIDATE | tests:y | cloud/aws security | Real secret-leak-to-LLM fix, redacted by default with opt-in reveal, 7 tests |
| [#1841](https://github.com/rtk-ai/rtk/pull/1841) | fix(toml-filter): bypass filtering when stdout is piped (#1060) | CANDIDATE | tests:y | toml-filter/dispatcher | Core dispatcher bug breaking pipelines; unit matrix + e2e test, 17/18 checks |
| [#1639](https://github.com/rtk-ai/rtk/pull/1639) | fix(rewrite): leave pipe groups raw end-to-end (#1560) | CANDIDATE | tests:y | hook/rewrite | Rewriting pipe LHS breaks downstream consumers; generalizes #439 carve-out, tests updated |
| [#1640](https://github.com/rtk-ai/rtk/pull/1640) | fix(rewrite): pass ls -O / -@ / -e through unchanged (#1627) | CANDIDATE | tests:y | hook/rewrite | Rewrite drops metadata columns user asked for; small, unit-tested helper |
| [#1648](https://github.com/rtk-ai/rtk/pull/1648) | fix(hooks/claude): omit permissionDecision under bypassPermissions | CANDIDATE | tests:y | hooks/claude | Rewrites silently dropped under bypass; bisected root cause, 4 tests; interacts with #1809 |
| [#1809](https://github.com/rtk-ai/rtk/pull/1809) | fix(hooks): honor permissions.defaultMode in claude hook (closes #17 | CANDIDATE | tests:y | hooks/permissions | Mode-aware auto-allow, 12 tests, verified on Mac app; check interaction with #1648 first |
| [#1804](https://github.com/rtk-ai/rtk/pull/1804) | fix(hooks): add timeout to Claude and Gemini hook entries | CANDIDATE | tests:y | hooks/init | Tiny fix preventing indefinite Claude stall; Copilot already had timeout |
| [#1840](https://github.com/rtk-ai/rtk/pull/1840) | fix(init): make --agent cursor truly standalone (#213) | CANDIDATE | tests:y | hooks/init | Clear routing bug (install_claude ignored --agent); regression tests + e2e, 17/18 checks |
| [#1661](https://github.com/rtk-ai/rtk/pull/1661) | fix(gh): passthrough --help/-h on pr subcommands (#1474) | CANDIDATE | tests:y | gh filter | 6 of 9 gh pr subcommands corrupt --help; small flag-aware passthrough, 5 tests |
| [#1588](https://github.com/rtk-ai/rtk/pull/1588) | fix(git): surface push rejection errors | CANDIDATE | tests:y | git filter | GH013/ruleset push rejections hidden as success; small, unit-tested |
| [#1834](https://github.com/rtk-ai/rtk/pull/1834) | fix(parser/formatter): show every failure name in compact mode (#181 | CANDIDATE | tests:y | parser/vitest/playwright | Truncation hides 44/49 failure names from agent; 4 regression tests, invariants preserved |
| [#1635](https://github.com/rtk-ai/rtk/pull/1635) | fix(windows): Git Bash support for install.sh and rtk run | CANDIDATE | tests:? | windows | Runtime MSYSTEM shell detection + installer support; directly relevant to Windows fork use |
| [#1793](https://github.com/rtk-ai/rtk/pull/1793) | fix: handle single-line Python docstrings in MinimalFilter (fixes #1 | CANDIDATE | tests:y | core/filter | Clear state-toggle bug corrupting all lines after single-line docstring; tiny + regression test |
| [#1696](https://github.com/rtk-ai/rtk/pull/1696) | fix(tee): keep head + tail when raw output exceeds max_file_size | CANDIDATE | tests:y | core/tee | Tee recovery log dropped tail where summaries live; UTF-8-safe slicing, tests |
| [#1544](https://github.com/rtk-ai/rtk/pull/1544) | fix(config): add #[serde(default)] to partial config sections | CANDIDATE | tests:y | core/config | 5-line fix; partial TOML sections silently discarded user overrides |
| [#1562](https://github.com/rtk-ai/rtk/pull/1562) | fix(pytest): avoid rendering each failure twice | CANDIDATE | tests:y | pytest filter | Every failure rendered twice; small fallback-only fix + regression test |
| [#1725](https://github.com/rtk-ai/rtk/pull/1725) | fix(grep): force --with-filename to fix single-file rg output parsin | CANDIDATE | tests:y | grep filter | rg omits filename on single file, parser misreads line number; small + tests |
| [#1541](https://github.com/rtk-ai/rtk/pull/1541) | fix(grep): return bare integer for -c/--count flag | CANDIDATE | tests:y | grep filter | -c output misparsed as matches; dedicated count path, tests; CI 0/2 needs recheck |
| [#1678](https://github.com/rtk-ai/rtk/pull/1678) | fix(grep): support --pcre2 flag | CANDIDATE | tests:y | grep filter | --pcre2 fell through to system grep; small, validated against rg |
| [#1662](https://github.com/rtk-ai/rtk/pull/1662) | fix(learn): drop parallel-tool-call cancellations from corrections | CANDIDATE | tests:y | learn | Cancellations dominated output (5060 lines); filtered both sides, 5 tests |
| [#1638](https://github.com/rtk-ai/rtk/pull/1638) | fix(go): pass through unrecognized go build output instead of "Succe | CANDIDATE | tests:y | go filter | False "Success" on unrecognized failures; more thorough of the two #1599 fixes |
| [#1689](https://github.com/rtk-ai/rtk/pull/1689) | fix(ls): force LC_TIME=C for locale-independent date parsing | CANDIDATE | tests:n | ls filter | 3-line fix; non-English locales made rtk ls always "(empty)"; CI failures need recheck |
| [#1843](https://github.com/rtk-ai/rtk/pull/1843) | fix(ls): remove .env from NOISE_DIRS to prevent silent credential ov | CANDIDATE | tests:n | ls filter | 2-line safety fix; hidden .env invites agent overwriting real credentials |
| [#1845](https://github.com/rtk-ai/rtk/pull/1845) | fix(container): surface docker health status in rtk docker ps outpu | CANDIDATE | tests:y | docker filter | Status field parsed then discarded; 7 unit tests, pure-function refactor |
| [#1844](https://github.com/rtk-ai/rtk/pull/1844) | fix(log): add CRITICAL, ALERT, EMERGENCY and DEBUG log level support | CANDIDATE | tests:y | log filter | CRITICAL lines silently discarded; tests + snapshot; bundles some refactor (3 files) |
| [#1518](https://github.com/rtk-ai/rtk/pull/1518) | fix(gh): rewrite gh search commands | CANDIDATE | tests:y | rewrite/gh | gh search left unrewritten; word-boundary guard, heavy regression tests, CI green |
| [#1508](https://github.com/rtk-ai/rtk/pull/1508) | fix(ls): preserve .env in default output | CANDIDATE | tests:y | ls filter | data-loss hazard (.env silently hidden), one-line fix + regression test, strong repro |
| [#1513](https://github.com/rtk-ai/rtk/pull/1513) | fix(hook): preserve existing Copilot instructions | CANDIDATE | tests:y | hooks/init | init clobbered user copilot-instructions.md; root cause named, marker-block idempotent fix |
| [#1497](https://github.com/rtk-ai/rtk/pull/1497) | Fix RTK shim recursion for Vitest passthrough | CANDIDATE | tests:y | js/vitest | shim recursion + help/version misparse in core vitest path, tests listed; 1 CI check failing |
| [#1491](https://github.com/rtk-ai/rtk/pull/1491) | fix(rewrite): route npm lint scripts through rtk npm (biome-safe) | CANDIDATE | tests:y | rewrite/npm | mirrors merged pnpm fix #678; biome projects silently exited 0 via eslint adapter |
| [#1483](https://github.com/rtk-ai/rtk/pull/1483) | fix(config): parse partial sub-config sections | CANDIDATE | tests:y | core/config | missing serde(default) made partial TOML sections silently fall back to defaults; 6 tests |
| [#1302](https://github.com/rtk-ai/rtk/pull/1302) | fix(git): fix --reverse showing newest commits instead of oldest | CANDIDATE | tests:y | git filter | -50 injection before --reverse inverted semantics; regression tests + fixture savings test |
| [#1298](https://github.com/rtk-ai/rtk/pull/1298) | fix(tracking): weighted savings rate in low_savings_commands and avg | CANDIDATE | tests:y | tracking/gain | unweighted AVG(savings_pct) misflags good filters; same root cause as merged #891 |
| [#1299](https://github.com/rtk-ai/rtk/pull/1299) | fix(cc): align monthly savings_pct denominator, use weighted totals | CANDIDATE | tests:y | analytics/cc | monthly used different denominator than daily/weekly; note: changes reported values |
| [#1294](https://github.com/rtk-ai/rtk/pull/1294) | fix(pytest): parse xfailed test output | CANDIDATE | tests:y | python/pytest | xfailed summary ignored -> "No tests collected"; small, clear repro; 2 CI checks failing |
| [#1358](https://github.com/rtk-ai/rtk/pull/1358) | fix(ls): force C locale for consistent date parsing across locales | CANDIDATE | tests:y | ls filter | non-English locales -> "(empty)" listing; LC_ALL=C at spawn, test added (dup #1390 exists) |
| [#1337](https://github.com/rtk-ai/rtk/pull/1337) | fix: Python single-line docstring and trailing brace on function sig | CANDIDATE | tests:y | core/filter | fixes #1322+#1323 in one PR with root-cause writeup; supersedes #1328/#1329 |
| [#1386](https://github.com/rtk-ai/rtk/pull/1386) | Fix #1071 | CANDIDATE | tests:n | git filter | git show rev:path corrupted binary blobs; tiny fix, md5-verified manually, partial scope |
| [#1422](https://github.com/rtk-ai/rtk/pull/1422) | fix: suppress hook warning during rtk init and verify | CANDIDATE | tests:n | hook_check | spurious warning during init -g; 7-line matches! guard, trivially verifiable |
| [#1423](https://github.com/rtk-ai/rtk/pull/1423) | fix(hook_check): treat non-Claude integrations as installed | CANDIDATE | tests:y | hook_check | false "No hook installed" when Codex/Cursor/etc configured; testable via status_for_home |
| [#1510](https://github.com/rtk-ai/rtk/pull/1510) | fix(rspec): skip JSON parsing for non-JSON output | CANDIDATE | tests:y | ruby/rspec | false JSON-parse warning on default text formatter; guard + test, clean quality |
| [#1444](https://github.com/rtk-ai/rtk/pull/1444) | fix(go): preserve benchmark and fuzz output in go test filter | CANDIDATE | tests:y | go filter | benchmark-only runs showed "No tests found"; good repro, though CI check failing |
| [#1380](https://github.com/rtk-ai/rtk/pull/1380) | fix(dotnet): rtk dotnet test breaks for global.json MTP mode | CANDIDATE | tests:y | dotnet | exceptional verification matrix across frameworks/SDKs, 11/11 checks pass; dotnet area only |
| [#1286](https://github.com/rtk-ai/rtk/pull/1286) | fix(git): exclude branch refs from looks_like_path heuristic | CANDIDATE | tests:y | git | Root-caused regression from #1217 (`git diff origin/main` empty), 6 new tests, CI 10/10 |
| [#1281](https://github.com/rtk-ai/rtk/pull/1281) | fix(ls): support ISO date format from GNU coreutils | CANDIDATE | tests:y | ls filter | rtk ls empty on GNU/Nix ISO dates; tiny regex fix with 3 tests |
| [#1251](https://github.com/rtk-ai/rtk/pull/1251) | fix(ls): preserve hierarchy for -R + passthrough incompatible flags (#7 | CANDIDATE | tests:y | ls filter | Fixes flat -R output + parser-breaking flags passthrough; ported to current arch, tested |
| [#1204](https://github.com/rtk-ai/rtk/pull/1204) | fix: expand npm rewrite rule to route install, ci, test, and other sub | CANDIDATE | tests:y | hook rewrite/npm | Best of 3 dupes for #1148: thoughtful subcommand list, 5 tests, regression + negative checks |
| [#1194](https://github.com/rtk-ai/rtk/pull/1194) | fix(security): replace sh -c with direct exec in err, test, summary | CANDIDATE | tests:n | runner/security | Real shell-injection fix (closes #640), matches pattern used everywhere else, small |
| [#1185](https://github.com/rtk-ai/rtk/pull/1185) | fix(git): detect intent-to-add files in status output | CANDIDATE | tests:n | git status | One-char match-arm fix with clear porcelain root cause and repro; test easily added |
| [#1122](https://github.com/rtk-ai/rtk/pull/1122) | fix(gh): don't early exit on failure for "pr checks" | CANDIDATE | tests:n | gh filter | 4-line fix: failing PR checks made rtk drop output entirely; core gh path |
| [#1083](https://github.com/rtk-ai/rtk/pull/1083) | fix(hook_check): skip hook warning on Windows when --claude-md mode is | CANDIDATE | tests:y | hook_check/Windows | Kills impossible-to-fix 24h nag on Windows; narrowly cfg-gated, unit tests included |
| [#1075](https://github.com/rtk-ai/rtk/pull/1075) | fix(hook): cursor hook fails to rewrite commands due to incorrect exit | CANDIDATE | tests:n | cursor hook | Exit-code-3 mishandling made rewrites never fire; thorough evidence, supersedes #1100 |
| [#1057](https://github.com/rtk-ai/rtk/pull/1057) | fix: preserve caller binary path for path-prefixed command rewrites | CANDIDATE | tests:y | hook rewrite | .venv/bin/pytest rewrites lost binary path (#1053); RTK_BIN carry, 13 tests, CI 11/11 |
| [#1047](https://github.com/rtk-ai/rtk/pull/1047) | fix(windows): wrap .cmd/.bat wrappers with cmd.exe /C for reliable exe | CANDIDATE | tests:y | Windows/exec | Centralized cmd.exe /C wrap fixes npm/pnpm shim exec on Windows (#950), 7 tests; CI red needs check |
| [#1023](https://github.com/rtk-ai/rtk/pull/1023) | fix(json): quote object keys in schema output | CANDIDATE | tests:? | json filter | Filter emitted invalid JSON (unquoted keys) breaking jq/json.load; tiny fix for #1015 |
| [#1019](https://github.com/rtk-ai/rtk/pull/1019) | fix(read): fallback to raw bytes for non-UTF-8 reads | CANDIDATE | tests:y | read/UTF-8 | Non-UTF-8 files hard-failed rtk read; narrow raw passthrough fallback with regression tests |
| [#1018](https://github.com/rtk-ai/rtk/pull/1018) | fix(read): preserve exact head semantics | CANDIDATE | tests:y | read/rewrite | head rewrites returned wrong line counts/summaries; exact --head-lines mode, good coverage |
| [#964](https://github.com/rtk-ai/rtk/pull/964) | fix: not merging git show with contradictory flags | CANDIDATE | tests:n | git show | Compact show merged incompatible flags causing git fatal; clean passthrough fix, tests unchecked |
| [#937](https://github.com/rtk-ai/rtk/pull/937) | fix(npx): passthrough unknown packages instead of routing to npm_cmd | CANDIDATE | tests:y | npx | npx unknown pkg became `npm run <pkg>` hard failure; root-caused, 2 regression tests |
| [#925](https://github.com/rtk-ai/rtk/pull/925) | fix(pytest): handle --collect-only output correctly | CANDIDATE | tests:y | pytest | --collect-only misread as "no tests"; dedicated filter path, tests, real agent-waste repro |
| [#891](https://github.com/rtk-ai/rtk/pull/891) | fix(gain): use weighted savings rate in per-command stats | CANDIDATE | tests:y | tracking/gain | Real root cause (unweighted AVG dilutes high-volume cmds), regression test, CI green |
| [#860](https://github.com/rtk-ai/rtk/pull/860) | fix(runner): propagate exit code from rtk err and rtk test | CANDIDATE | tests:n | runner/exit codes | 6-line fix for exit 0 always returned; matches RTK exit-propagation rule; CI red but trivial |
| [#843](https://github.com/rtk-ai/rtk/pull/843) | fix(config): add serde(default) to TeeConfig fields | CANDIDATE | tests:y | config | Partial [tee] section silently kills entire config; clear repro, tests, CI green |
| [#837](https://github.com/rtk-ai/rtk/pull/837) | fix(init): detect and clean up orphaned .github/hooks/rtk-rewrite.json | CANDIDATE | tests:n | hook/init | Orphaned "rtk hook" file breaks every Bash call silently; clear cause, small, single file |
| [#793](https://github.com/rtk-ai/rtk/pull/793) | fix(wget): detect -O - in trailing args absorbed by Clap | CANDIDATE | tests:y | wget/arg parsing | Clap trailing_var_arg swallows -O -; 14 unit tests, fixes #716, CI green |
| [#792](https://github.com/rtk-ai/rtk/pull/792) | fix(diff): check modified count in identical-files guard | CANDIDATE | tests:y | diff filter | Diff swallows modified-only changes; two root causes explained, 4 regression tests |
| [#742](https://github.com/rtk-ai/rtk/pull/742) | fix(hook): hook warning repeats on every command on Windows | CANDIDATE | tests:n | hook/Windows | 6-line fix: empty-write doesn't bump mtime on Windows; directly relevant to this Windows fork |
| [#737](https://github.com/rtk-ai/rtk/pull/737) | fix(npx): auto-approve installable fallbacks | CANDIDATE | tests:y | js/npx | First-run npx prompt hangs cc-economics/tsc/next/prisma; shared -y helper, 11/11 checks |
| [#679](https://github.com/rtk-ai/rtk/pull/679) | fix(cargo): preserve clippy -- separator with cargo flags | CANDIDATE | tests:y | cargo/arg parsing | -- restoration broken with cargo flags before it; regression tests, 10/10 checks |
| [#678](https://github.com/rtk-ai/rtk/pull/678) | fix(rewrite): route pnpm lint through rtk pnpm | CANDIDATE | tests:y | rewrite/pnpm | pnpm lint wrongly forced to rtk lint causing ESLint retry loops; tests, 10/10 checks |
| [#676](https://github.com/rtk-ai/rtk/pull/676) | fix(rewrite): skip unsupported find predicates | CANDIDATE | tests:y | rewrite/find | rtk find errors on -exec/-not etc; passthrough guard, regression tests, 10/10 checks |
| [#673](https://github.com/rtk-ai/rtk/pull/673) | fix(pytest): handle xfailed summary | CANDIDATE | tests:y | pytest filter | xfailed counted as failed / missed in -q mode; repro shown, regression tests, 10/10 checks |
| [#881](https://github.com/rtk-ai/rtk/pull/881) | fix(grep): read stdin when piped instead of searching filesystem | CANDIDATE | tests:n | grep filter | Real bug (fixes #838), tiny diff; but CI 0/3 and body mentions bundled extras — verify diff before adopting |
| [#419](https://github.com/rtk-ai/rtk/pull/419) | fix: preserve gh run view passthrough args | CANDIDATE | tests:y | gh filter | Flag-only gh run view forms lost passthrough; regression tests; older so check conflicts |

## Second wave (112 fixes deferred + 16 shortlisted features)

### Deferred fixes

| PR | Title | Verdict | Tests | Area | Reason |
|---|---|---|---|---|---|
| [#3022](https://github.com/rtk-ai/rtk/pull/3022) | fix(git): stop silently truncating large diffs | SECOND-WAVE | tests:y | git/diff | Solid work but removes all diff caps — policy change that cuts token savings; 5 files, needs deliberate adoption |
| [#3021](https://github.com/rtk-ai/rtk/pull/3021) | test(hook): cover option-bearing uv runs in Claude chains | SECOND-WAVE | tests:y | hook/tests | Test-only coverage for uv rewrite chains; useful but no behavior change |
| [#3005](https://github.com/rtk-ai/rtk/pull/3005) | perf(tracking): skip schema migration on warm Tracker::new | SECOND-WAVE | tests:? | tracking/perf | Real warm-path win but adds schema-marker scheme; perf not bug; depends on #3004 measurements |
| [#3004](https://github.com/rtk-ai/rtk/pull/3004) | perf(git): single porcelain status via git-dir state detection | SECOND-WAVE | tests:y | git/perf | ~30% faster git status but 235-line rework reimplementing state detection; regression risk |
| [#2993](https://github.com/rtk-ai/rtk/pull/2993) | fix(hook): propagate global flags through Claude rewrites | SECOND-WAVE | tests:y | hook | Real gap but feature-flavored flag plumbing, 169 lines, CI 0/1 passing |
| [#2981](https://github.com/rtk-ai/rtk/pull/2981) | fix(tracking): cap token baseline at agent harness truncation limit | SECOND-WAVE | tests:y | tracking/gain | Compelling honesty fix for inflated savings but changes metric semantics; 5 files, env knobs, no DB migration |
| [#2971](https://github.com/rtk-ai/rtk/pull/2971) | fix(docker): forward value-taking command flags | SECOND-WAVE | tests:y | docker | Real flag-forwarding fix but 186 lines in lower-priority docker area |
| [#2931](https://github.com/rtk-ai/rtk/pull/2931) | fix(discover): match Claude project dirs case-insensitively on Windo | SECOND-WAVE | tests:y | discover/Windows | Colon half superseded by #2952; case-insensitivity half worthwhile but author never ran the toolchain |
| [#2930](https://github.com/rtk-ai/rtk/pull/2930) | fix(discover): count hook-rewritten commands as RTK usage | SECOND-WAVE | tests:y | discover | Plausible metrics-accuracy fix but test plan unchecked and area is reporting-only |
| [#2903](https://github.com/rtk-ai/rtk/pull/2903) | fix: add is_unsupported_shape() guard before rewrite_compound() | SECOND-WAVE | tests:? | hook/rewrite | Fixes 3 issues but disables all find/fd rewrites — policy change, revisit |
| [#2825](https://github.com/rtk-ai/rtk/pull/2825) | fix(rewrite): respect exclude_commands in head/tail fast path; map ba | SECOND-WAVE | tests:y | hook/rewrite | Good, but bundles bare-head remap + test expectation change; #2887 is narrower |
| [#2860](https://github.com/rtk-ai/rtk/pull/2860) | fix(npm,lint): remove hardcoded allowlists, forward args untouched | SECOND-WAVE | tests:y | npm/lint | Maintainer-endorsed direction, CI green, but behavior-changing allowlist removal |
| [#2905](https://github.com/rtk-ai/rtk/pull/2905) | fix(php): inject -v so pint reports which rules fired | SECOND-WAVE | tests:y | php/pint | High-quality (Docker-verified fixtures) but PHP is off the core list |
| [#2886](https://github.com/rtk-ai/rtk/pull/2886) | fix(cargo): rewrite cargo nextest commands | SECOND-WAVE | tests:y | rewrite registry | Missed-savings coverage gap, not a breakage; small and clean |
| [#2843](https://github.com/rtk-ai/rtk/pull/2843) | fix(search): actionable error when engine binary is missing | SECOND-WAVE | tests:y | search DX | Error-string-only improvement; nice but not a correctness bug |
| [#2826](https://github.com/rtk-ai/rtk/pull/2826) | fix(search): group multi-file matches under per-file filename header | SECOND-WAVE | tests:n | search output | Token-savings feature with size guard; integration test still unchecked |
| [#2812](https://github.com/rtk-ai/rtk/pull/2812) | fix(grep): add regression test for --ultra-compact grep data loss | SECOND-WAVE | tests:y | grep | Test-only; bug already fixed on develop, adds regression coverage |
| [#2754](https://github.com/rtk-ai/rtk/pull/2754) | fix(dotnet): skip raw stdout prepend when build errors are fully par | SECOND-WAVE | tests:y | dotnet | Solid, mirrors #2501, but token-optimization not correctness; medium size |
| [#2728](https://github.com/rtk-ai/rtk/pull/2728) | fix(windows): add PowerShell compatibility fallbacks for ls, wc, pro | SECOND-WAVE | tests:? | Windows | Advisory messages/UX polish across 8 files, not a correctness fix |
| [#2718](https://github.com/rtk-ai/rtk/pull/2718) | fix(hook): add output transparency section to RTK instructions | SECOND-WAVE | tests:y | hook/init | 4-line prompt-text addition; useful vs tamper heuristics but not a code fix |
| [#2707](https://github.com/rtk-ai/rtk/pull/2707) | fix(make): preserve tail summary for tail-heavy output | SECOND-WAVE | tests:y | make filter | Reasonable head+tail heuristic change with test; output-shape tuning |
| [#2700](https://github.com/rtk-ai/rtk/pull/2700) | fix(init): include codex in global uninstall | SECOND-WAVE | tests:y | init | Real uninstall gap but codex integration cleanup; low urgency |
| [#2689](https://github.com/rtk-ai/rtk/pull/2689) | perf(deps): move regex compilation to lazy_static | SECOND-WAVE | tests:y | perf | Correct per repo convention; perf cleanup, not a bug |
| [#2684](https://github.com/rtk-ai/rtk/pull/2684) | fix(hook): detect JetBrains run_in_terminal in Copilot hook | SECOND-WAVE | tests:y | hooks/copilot | Well-tested but JetBrains Copilot niche; deny-with-suggestion semantics |
| [#2671](https://github.com/rtk-ai/rtk/pull/2671) | fix(discover): report Claude hook coverage uncertainty | SECOND-WAVE | tests:y | discover | Good post-review revision for #2648, but discover reporting is non-core |
| [#2634](https://github.com/rtk-ai/rtk/pull/2634) | fix(stream): name the program in spawn-failure errors | SECOND-WAVE | tests:y | core/stream | DX error-message improvement, not a behavior bug; nice-to-have |
| [#2616](https://github.com/rtk-ai/rtk/pull/2616) | fix(git): include branch name in commit output | SECOND-WAVE | tests:y | git | Output-enrichment feature (#2021), changes snapshot output format |
| [#2603](https://github.com/rtk-ai/rtk/pull/2603) | test(telemetry): add comprehensive unit tests for telemetry_cmd.rs | SECOND-WAVE | tests:y | telemetry | Solid coverage + testability refactor but large tests-only, adds mock dep |
| [#2601](https://github.com/rtk-ai/rtk/pull/2601) | Handle pytest error summaries | SECOND-WAVE | tests:y | python/pytest | Plausible parser fix but author never ran cargo, 0/2 checks passing |
| [#2585](https://github.com/rtk-ai/rtk/pull/2585) | fix(cargo): use valid SPDX identifier for license field | SECOND-WAVE | tests:n | metadata | Correct 2-line SPDX fix, zero risk, but metadata-only and 0 checks ran |
| [#2584](https://github.com/rtk-ai/rtk/pull/2584) | fix(filters): mark truncated fallback output | SECOND-WAVE | tests:y | filters | Useful truncation hints across 4 files but 1 CI check failing |
| [#2583](https://github.com/rtk-ai/rtk/pull/2583) | fix(hermes): use absolute path for rtk binary discovery | SECOND-WAVE | tests:n | hermes plugin | Windows PATH fix but fringe integration, manual verification only |
| [#2580](https://github.com/rtk-ai/rtk/pull/2580) | fix(cli): support pre-command ultra compact short flag | SECOND-WAVE | tests:y | cli | Minor ergonomics (-u flag), has conflicts; low value vs risk |
| [#2574](https://github.com/rtk-ai/rtk/pull/2574) | fix(hook): support copilot ide terminal tool | SECOND-WAVE | tests:y | hook/copilot | Feature-flavored JetBrains Copilot support, secondary integration |
| [#2570](https://github.com/rtk-ai/rtk/pull/2570) | fix(lint): preserve biome routing | SECOND-WAVE | tests:y | js/lint | Real biome-vs-eslint bug but failing check plus merge conflicts |
| [#2568](https://github.com/rtk-ai/rtk/pull/2568) | fix(rewrite): handle process wrapper prefixes | SECOND-WAVE | tests:y | rewrite | timeout/nice/nohup peeling; capability add more than bug fix, 222 lines |
| [#2566](https://github.com/rtk-ai/rtk/pull/2566) | fix(opencode): export plugin as default | SECOND-WAVE | tests:y | opencode | 10-line real fix but fringe OpenCode integration |
| [#2558](https://github.com/rtk-ai/rtk/pull/2558) | Remove stale duplicate RTK hook entries during init | SECOND-WAVE | tests:y | hook/init | Semver-aware dedup; overlaps #2588, conflicts, pick one of the two |
| [#2557](https://github.com/rtk-ai/rtk/pull/2557) | Add native Codex PreToolUse hook support | SECOND-WAVE | tests:y | hook/codex | Pure feature, 1223 lines; well-specified but large and conflicts |
| [#2556](https://github.com/rtk-ai/rtk/pull/2556) | Preserve existing Bash hooks when installing RTK | SECOND-WAVE | tests:y | hook | Valuable coexistence fix but 1891 lines, new manifest+guard, risky |
| [#2555](https://github.com/rtk-ai/rtk/pull/2555) | Improve command rewrite success rate for pipes and redirects (lex | SECOND-WAVE | tests:y | rewrite/lexer | High-value pipe/redirect correctness but 1062 lines, deep core change |
| [#2541](https://github.com/rtk-ai/rtk/pull/2541) | Add ng test/Karma TOML filter with yarn rewrite | SECOND-WAVE | tests:y | js/angular filter | New Angular Karma TOML filter; clean but feature, not a fix |
| [#2531](https://github.com/rtk-ai/rtk/pull/2531) | fix(discover): rewrite xcodebuild commands to rtk | SECOND-WAVE | tests:y | discover rules | Only wires existing xcodebuild filter into rewrite; useful but mac-niche feature wiring |
| [#2526](https://github.com/rtk-ai/rtk/pull/2526) | Add Bruno CLI (bru) TOML filter and hook rewrite support. | SECOND-WAVE | tests:y | TOML filters | New Bruno CLI filter; well-shaped but pure feature for a niche tool |
| [#2504](https://github.com/rtk-ai/rtk/pull/2504) | Add Nix flake for building from source | SECOND-WAVE | tests:n | packaging | Nix flake, zero-maintenance claim; packaging feature, no code impact |
| [#2482](https://github.com/rtk-ai/rtk/pull/2482) | fix(gradle): match normalized gradlew/gradle basename lookups (#1177, # | SECOND-WAVE | tests:y | gradle filter | Solid RED->GREEN fix but gradle is a peripheral ecosystem for this fork |
| [#2481](https://github.com/rtk-ai/rtk/pull/2481) | fix(filters): use tail_lines for just/mise/task to preserve failure sum | SECOND-WAVE | tests:y | TOML filters | Real summary-loss bug but niche delegator tools; overlaps #2480's core fix |
| [#2478](https://github.com/rtk-ai/rtk/pull/2478) | fix(kubectl): support global flags + exec/delete/rollout subcommands (# | SECOND-WAVE | tests:y | kubectl rules | Well-tested but cloud/kubectl peripheral; routes destructive subcommands through rewrite |
| [#2477](https://github.com/rtk-ai/rtk/pull/2477) | fix(init): honor RTK_TELEMETRY_DISABLED in consent prompt (#1307) | SECOND-WAVE | tests:y | init/telemetry | Real CI-hang fix but telemetry consent is a minor surface; hang itself untestable in harness |
| [#2476](https://github.com/rtk-ai/rtk/pull/2476) | fix(json): only flag strict ISO dates, not 10-char dashed strings (#141 | SECOND-WAVE | tests:y | json schema | Correct tiny fix (date? mislabel) but low-impact schema-inference cosmetics |
| [#2472](https://github.com/rtk-ai/rtk/pull/2472) | fix(copilot): add missing 'get' to kubectl example in init template | SECOND-WAVE | tests:y | init template | Real template typo with regression test; low impact, only matters for copilot users |
| [#2459](https://github.com/rtk-ai/rtk/pull/2459) | fix(git): wire up status_max_files and status_max_untracked config limi | SECOND-WAVE | tests:? | git status filter | Orphaned config knobs are a real silent no-op, but no clear new tests and CI/CLA red |
| [#2311](https://github.com/rtk-ai/rtk/pull/2311) | fix(filter-quality): preserve error signal in go test/vet/golangci fi | SECOND-WAVE | tests:y | go filters | High quality, but go area and medium-large 2-file change; revisit |
| [#2426](https://github.com/rtk-ai/rtk/pull/2426) | fix: align OpenCode and OpenClaw TypeScript plugins with Pi contract | SECOND-WAVE | tests:n | agent plugins | Reasonable hardening but tests deliberately omitted, niche plugin area |
| [#2417](https://github.com/rtk-ai/rtk/pull/2417) | fix(aws): keep eks describe-cluster detail instead of one summary lin | SECOND-WAVE | tests:y | aws filter | Well-built but feature-flavored output expansion in non-core aws area |
| [#2412](https://github.com/rtk-ai/rtk/pull/2412) | Add rewrite for "rg --files" -> rtk find | SECOND-WAVE | tests:y | rewrite | New rewrite path (feature), narrow scope, verified; not a bug fix |
| [#2351](https://github.com/rtk-ai/rtk/pull/2351) | fix(init): write cline rules to .clinerules/rtk.md when dir exists | SECOND-WAVE | tests:n | init/cline | Real crash fix but test plan entirely unchecked, niche agent |
| [#2331](https://github.com/rtk-ai/rtk/pull/2331) | [codex] stabilize Windows Claude and Codex hooks | SECOND-WAVE | tests:y | hooks/Windows | Valuable Windows hook work but 6382 lines / 40 files mega-PR; too risky to adopt wholesale |
| [#2325](https://github.com/rtk-ai/rtk/pull/2325) | fix(tee): quote recovery hint paths with spaces | SECOND-WAVE | tests:y | tee hints | Real but cosmetic hint-formatting polish, macOS-specific path issue |
| [#2298](https://github.com/rtk-ai/rtk/pull/2298) | test(telemetry): isolate device salt tests | SECOND-WAVE | tests:y | telemetry tests | Test hygiene only (stops tests touching prod .device_salt); low impact |
| [#2287](https://github.com/rtk-ai/rtk/pull/2287) | fix(find): reject unsafe bare native actions | SECOND-WAVE | tests:y | find filter | Real -delete footgun but 300 lines / 7 files and CI failing |
| [#2282](https://github.com/rtk-ai/rtk/pull/2282) | Add whitelist mode for command filtering (#2231) | SECOND-WAVE | tests:y | hook/config | Feature (include_commands whitelist), well built but 351 lines and CI red |
| [#2272](https://github.com/rtk-ai/rtk/pull/2272) | fix: keep filter output faithful for find, ls -R, env, and cargo | SECOND-WAVE | tests:y | find/ls/env/cargo | Four unrelated fixes bundled (305 lines); good ideas but grab-bag, CI red |
| [#2273](https://github.com/rtk-ai/rtk/pull/2273) | fix(gh): passthrough run view job output | SECOND-WAVE | tests:y | gh filter | Small sensible passthrough fix but 0/3 checks passing |
| [#2228](https://github.com/rtk-ai/rtk/pull/2228) | fix(discover,learn): default to scanning all projects | SECOND-WAVE | tests:? | discover/learn | Plausible but changes default semantics; may interact with #2308; CI 1/3 |
| [#2226](https://github.com/rtk-ai/rtk/pull/2226) | fix(rewrite): apply custom TOML filters during command rewrite | SECOND-WAVE | tests:? | hook/rewrite | Real gap (TOML filters dead in hook path) but CI failing (4/11 + 1 fail) |
| [#2225](https://github.com/rtk-ai/rtk/pull/2225) | fix(proxy): reject compound shell snippets with clear error mess | SECOND-WAVE | tests:? | proxy | Reasonable UX hardening but behavior change; rejection may annoy users |
| [#2222](https://github.com/rtk-ai/rtk/pull/2222) | fix(grep): skip .claude/worktrees directory | SECOND-WAVE | tests:n | grep filter | 8-line exclusion, but silently hiding dirs from grep is opinionated |
| [#2220](https://github.com/rtk-ai/rtk/pull/2220) | fix(openclaw): handle rtk rewrite exit code 3 as valid suggestio | SECOND-WAVE | tests:? | openclaw plugin | Real dead-code bug (execSync throws on exit 3) but niche integration |
| [#2205](https://github.com/rtk-ai/rtk/pull/2205) | fix(git): add visible truncation markers to filtered output | SECOND-WAVE | tests:? | git filter | Good idea (visible truncation) but one-line body, 0/1 checks passing |
| [#2199](https://github.com/rtk-ai/rtk/pull/2199) | fix(grep): group compact output by file | SECOND-WAVE | tests:y | grep | Output-format improvement (grouping), not a bug; modest savings |
| [#2125](https://github.com/rtk-ai/rtk/pull/2125) | fix: rewrite rg --files to rtk find | SECOND-WAVE | tests:y | hook/rewrite | Small and tested, but #2183 also addresses #2060 — possible supersession |
| [#2153](https://github.com/rtk-ai/rtk/pull/2153) | fix(hook): support OpenCode Desktop (Node.js) in plugin | SECOND-WAVE | tests:y | hook/opencode | Real root cause + tests, but niche agent plugin (JS side) |
| [#2054](https://github.com/rtk-ai/rtk/pull/2054) | Improve Codex support on Windows | SECOND-WAVE | tests:y | hook/codex/Windows | Feature-flavored 8-file change; Windows test portability bits interesting |
| [#2051](https://github.com/rtk-ai/rtk/pull/2051) | fix(hooks): eliminate redundant disk reads on every hook invocation | SECOND-WAVE | tests:n | hook/perf | Perf caching (OnceLock) in hot hook path; no tests, checks failing |
| [#2050](https://github.com/rtk-ai/rtk/pull/2050) | fix(trust): eliminate TOCTOU race in filter loading; harden CI trust b | SECOND-WAVE | tests:? | trust/security | Real security issues but 3 bundled behavior changes, 333 lines, needs review |
| [#1924](https://github.com/rtk-ai/rtk/pull/1924) | fix(go): surface runtime fatal output and panic-during-test context | SECOND-WAVE | tests:y | go runner | Solid alternate fix for #1882 but duplicates #1984; revisit if #1984 not adopted |
| [#1982](https://github.com/rtk-ai/rtk/pull/1982) | fix(cmds/ruby): accept version-only JSON from rubocop/rspec --versio | SECOND-WAVE | tests:y | ruby | Clean small fix with 8 tests, but ruby is a low-priority area for the fork |
| [#1939](https://github.com/rtk-ai/rtk/pull/1939) | fix(gradlew): anchor 'e:' in ERROR_LINE to avoid mid-line false posi | SECOND-WAVE | tests:y | gradle | Genuine anchoring bug with regression test, but gradle is a marginal area |
| [#1977](https://github.com/rtk-ai/rtk/pull/1977) | fix(hooks/codex): write RTK reference to AGENTS.override.md, not AGE | SECOND-WAVE | tests:y | hooks/init codex | Well-built migration but behavior-change in codex layering; niche integration |
| [#1963](https://github.com/rtk-ai/rtk/pull/1963) | fix(init): honor GEMINI_CLI_HOME for Gemini directory resolution | SECOND-WAVE | tests:y | hooks/init gemini | Small env-var honor, thin body, minor feature-flavored fix |
| [#1899](https://github.com/rtk-ai/rtk/pull/1899) | fix(grep): normalize rg-compatible rewrite flags | SECOND-WAVE | tests:y | hook/rewrite/grep | Valuable core fix (#1824) but 17K-line / 136-file diff = high conflict risk |
| [#1931](https://github.com/rtk-ai/rtk/pull/1931) | fix(kubectl): stop dropping resource data in get filters (TOON outpu | SECOND-WAVE | tests:y | cloud/kubectl | Real data-loss fix with measured gains, but 10K-line rewrite + new TOON format |
| [#1894](https://github.com/rtk-ai/rtk/pull/1894) | Add Lua language and tools support | SECOND-WAVE | tests:y | new ecosystem | Feature with real savings evidence, 1270 lines / 10 files; not a bug fix |
| [#1800](https://github.com/rtk-ai/rtk/pull/1800) | fix(hooks): recognize run_in_terminal for VS Code Copilot Chat | SECOND-WAVE | tests:y | hooks/copilot | Well-verified but Copilot-specific and larger (573 lines); revisit if Copilot matters |
| [#1775](https://github.com/rtk-ai/rtk/pull/1775) | fix(hooks): emit passthrough JSON when no RTK rewrite available | SECOND-WAVE | tests:y | hooks/claude | Diff checked: changes hook output for every non-rewritten command; body uninformative, behavioral risk |
| [#1542](https://github.com/rtk-ai/rtk/pull/1542) | fix(grep): support piped stdin input | SECOND-WAVE | tests:y | grep filter | Real gap but reimplements matching in-process (new stdin path); CI 0/2 |
| [#1660](https://github.com/rtk-ai/rtk/pull/1660) | fix: report ok for silent npm scripts | SECOND-WAVE | tests:y | npm filter | Core npm area but author never ran cargo locally; needs verification |
| [#1732](https://github.com/rtk-ai/rtk/pull/1732) | fix(terraform): preserve long plan and HCL data output | SECOND-WAVE | tests:? | terraform filter | Reasonable limit-tuning, peripheral area, replaces CLA-failed #1722 |
| [#1685](https://github.com/rtk-ai/rtk/pull/1685) | Improve discover command normalisation | SECOND-WAVE | tests:? | discover | Feature-flavored improvement, 7 files, checks failing; useful redaction idea |
| [#1366](https://github.com/rtk-ai/rtk/pull/1366) | Fix/windows ls native | SECOND-WAVE | tests:? | ls/windows | valuable for Windows fork (native ls fallback) but 1322 lines, refactor+feature, risky |
| [#1378](https://github.com/rtk-ai/rtk/pull/1378) | fix(discover): normalize venv/absolute paths with binary preservation | SECOND-WAVE | tests:y | rewrite/discover | introduces new RTK_BIN_PATH env mechanism across 7 files; cross-cutting, revisit later |
| [#1395](https://github.com/rtk-ai/rtk/pull/1395) | fix(install): verify sha256 checksum of downloaded release tarball | SECOND-WAVE | tests:n | install.sh | solid security hardening, well argued, but installer script not a runtime bug fix |
| [#1343](https://github.com/rtk-ai/rtk/pull/1343) | fix(cursor): handle rtk rewrite exit code 3 | SECOND-WAVE | tests:n | hooks/cursor | real total-breakage fix with clear contract analysis, but Cursor-only shell hook |
| [#1330](https://github.com/rtk-ai/rtk/pull/1330) | fix (copilot): hook syntax for copilot integration | SECOND-WAVE | tests:n | hooks/copilot | 7-line schema fix, manually verified, but niche integration and 0/4 checks passing |
| [#1488](https://github.com/rtk-ai/rtk/pull/1488) | Fix/issue 1465 | SECOND-WAVE | tests:y | init | init target-selection refactor mixed with unrelated tracking-test changes, 8 files |
| [#1280](https://github.com/rtk-ai/rtk/pull/1280) | Add first-class uv command dispatch and hook rewrite | SECOND-WAVE | tests:y | uv/hooks | Solid but pure feature (372 lines); clean rebase of #176, revisit if uv matters |
| [#1174](https://github.com/rtk-ai/rtk/pull/1174) | fix(grep): increase default limits and add --full-lines flag | SECOND-WAVE | tests:y | grep filter | Default-limit tuning plus new flag; feature-flavored, savings tradeoff to weigh |
| [#1133](https://github.com/rtk-ai/rtk/pull/1133) | fix(git): preserve in-progress operation state and block unsafe push/c | SECOND-WAVE | tests:? | git | Real masking bug (merge/rebase state hidden) but bundles opinionated push-blocking policy, 227 lines |
| [#1097](https://github.com/rtk-ai/rtk/pull/1097) | fix(init): allow --hook-only for local project hook installation | SECOND-WAVE | tests:n | init/hooks | Useful capability gap fill but feature-ish; manual verification unchecked |
| [#1095](https://github.com/rtk-ai/rtk/pull/1095) | Add glab command support (#1085) | SECOND-WAVE | tests:y | glab | New tool support, small, but thin body and a failing check |
| [#1092](https://github.com/rtk-ai/rtk/pull/1092) | fix(cli): reserve -u for git push | SECOND-WAVE | tests:n | cli/git push | Real `git push -u` collision but breaking flag change (-u→-U), test plan unchecked |
| [#1068](https://github.com/rtk-ai/rtk/pull/1068) | fix(security): add integrity check for global TOML filters | SECOND-WAVE | tests:y | security/filters | Good supply-chain hardening but 275-line new trust mechanism, feature-scale |
| [#1020](https://github.com/rtk-ai/rtk/pull/1020) | fix(init): clean up data and config directories during uninstall | SECOND-WAVE | tests:y | init | Thorough uninstall cleanup with prompts, but data-deletion behavior change, 246 lines |
| [#1013](https://github.com/rtk-ai/rtk/pull/1013) | fix(init): skip redundant hook warning during rtk init | SECOND-WAVE | tests:n | init | Tiny valid UX fix (misleading warning), low priority; matches existing Gain skip pattern |
| [#901](https://github.com/rtk-ai/rtk/pull/901) | test: data-driven filter test engine with RTK_UPDATE_TEST_DATA | SECOND-WAVE | tests:y | test infra | High-quality test tooling + documents filter quirks, but 480-line infra, not a bug fix |
| [#656](https://github.com/rtk-ai/rtk/pull/656) | fix(security): restrict tee file/directory permissions to 0600/0700 | SECOND-WAVE | tests:y | tee/security | Legit hardening but unix-only (no-op on this Windows fork), larger than body implies |
| [#655](https://github.com/rtk-ai/rtk/pull/655) | fix(security): telemetry opt-in consent model | SECOND-WAVE | tests:y | telemetry | Sound privacy change but 18-file behavioral/policy shift, not a bug fix |
| [#691](https://github.com/rtk-ai/rtk/pull/691) | fix(ccusage): prefer bunx before npx | SECOND-WAVE | tests:y | ccusage | Runner-priority improvement, feature-flavored; useful only if Bun in play |
| [#760](https://github.com/rtk-ai/rtk/pull/760) | fix: track ls savings against baseline output | SECOND-WAVE | tests:n | tracking/ls | Metric-accuracy fix but runs extra baseline ls per call, no tests, CI failing |
| [#579](https://github.com/rtk-ai/rtk/pull/579) | fix: rewrite docker-compose (legacy hyphenated CLI) to rtk docker | SECOND-WAVE | tests:y | rewrite/docker | Solid small routing addition with 6 tests, but legacy-CLI coverage is feature-flavored |
| [#420](https://github.com/rtk-ai/rtk/pull/420) | security: comprehensive OWASP security review and fixes | SECOND-WAVE | tests:n | tee/summary | Mostly a report doc, but the UTF-8-safe truncation fix in tee.rs is worth cherry-picking |

### Shortlisted features

| PR | Title | Verdict | Tests | Area | Reason |
|---|---|---|---|---|---|
| [#3030](https://github.com/rtk-ai/rtk/pull/3030) | feat(gain): add scope, version and per-command breakdown to JSON expo | SECOND-WAVE | tests:y | gain/analytics | Fills gaps in existing JSON export; tests confirmed failing pre-change, parser-verified |
| [#2827](https://github.com/rtk-ai/rtk/pull/2827) | feat(secrets): hard-exclude credential files in grep/rg/find/read | SECOND-WAVE | tests:y | security/system | Deny-list across 4 commands, symlink-safe, 10 tests + hyperfine + e2e on real repro |
| [#3003](https://github.com/rtk-ai/rtk/pull/3003) | feat(python): recognize poetry run, filtering output like uv run | SECOND-WAVE | tests:y | python filters | Top unhandled command in discover; mirrors uv_cmd, live-verified full path + suite green |
| [#2357](https://github.com/rtk-ai/rtk/pull/2357) | feat(discover): count hook-rewritten commands as captured, not missed | SECOND-WAVE | tests:y | discover accuracy | Fixes wildly misleading adoption stats; real before/after numbers, loader + report tests |
| [#2991](https://github.com/rtk-ai/rtk/pull/2991) | feat(docker): add compact filter for docker compose up | SECOND-WAVE | tests:y | docker filter | Real 4-service fixture, 61% savings asserted, detached/foreground split, e2e verified |
| [#2932](https://github.com/rtk-ai/rtk/pull/2932) | feat(hooks): optimize commands inside wsl wrappers (#2008) | SECOND-WAVE | tests:y | hook robustness | 1-file fix unwrapping quoted wsl -c inner commands; 8 unit tests, directly useful on Windows |
| [#2255](https://github.com/rtk-ai/rtk/pull/2255) | feat(hooks): add native Windows PowerShell hook (rtk-rewrite.ps1) | SECOND-WAVE | tests:y | hooks/windows | Solves PS 5.1 quote-stripping with argv-correct spawn; e2e table + included smoke test |
| [#2474](https://github.com/rtk-ai/rtk/pull/2474) | feat(find): fall back to native find for unsupported predicates | SECOND-WAVE | tests:y | find/never-block | Refusal -> passthrough with exit-code propagation; 4 tests, CI 11/11, manual vs native find |
| [#2630](https://github.com/rtk-ai/rtk/pull/2630) | feat(grep): fold shared path prefix in file-list output | SECOND-WAVE | tests:y | grep filter | Lossless prefix folding turns 0% into 30-55% savings; 9 tests, never-worse guarded |
| [#2685](https://github.com/rtk-ai/rtk/pull/2685) | feat(runner): skip filtering for short output (auto-passthrough) | SECOND-WAVE | tests:y | runner/core | Dual-threshold passthrough, config-gated, backward compat + boundary tests |
| [#2676](https://github.com/rtk-ai/rtk/pull/2676) | feat(discover): skip tiny real outputs in missed-savings estimate | SECOND-WAVE | tests:y | discover accuracy | 1-file fix for inflated estimates; threshold boundary tests + behavioral check |
| [#2506](https://github.com/rtk-ai/rtk/pull/2506) | feat(read): support for offset and slice read | SECOND-WAVE | tests:y | read command | Adds Claude-compatible positional line ranges + --offset/--limit; unit + manual coverage |
| [#2537](https://github.com/rtk-ai/rtk/pull/2537) | feat(tracking+gain): per-session token savings tracking | SECOND-WAVE | tests:? | gain/analytics | Migration-safe session_id column + gain --session views; body strong, no explicit test plan |
| [#2926](https://github.com/rtk-ai/rtk/pull/2926) | feat(hook): rewrite npm and pnpm test commands | SECOND-WAVE | tests:y | hook coverage | Tiny (31 lines): routes npm test/npm t/pnpm test; regression coverage, full suite run |
| [#2377](https://github.com/rtk-ai/rtk/pull/2377) | feat(read): add Auto filter level that scales by file size | SECOND-WAVE | tests:y | read command | Opt-in size-scaled filter level, no behavior change by default; parsing + scaling tests |
| [#1176](https://github.com/rtk-ai/rtk/pull/1176) | feat(tracking): add --no-track flag and reduce default retention to 3 | SECOND-WAVE | tests:y | tracking/privacy | Clean opt-out flag + env var, 10/10 checks passed, tests for both tracking paths |

## Passed on (107 reviewed + 3 unsure)

| PR | Title | Verdict | Tests | Area | Reason |
|---|---|---|---|---|---|
| [#3012](https://github.com/rtk-ai/rtk/pull/3012) | fix(hooks): emit allow with Claude updatedInput | PASS | tests:y | hook | Superseded by better-reasoned #3031; blanket allow escalates permissions; branched off master with CI issues |
| [#3014](https://github.com/rtk-ai/rtk/pull/3014) | fix: rtk init -g --agent cursor fails when ~/.claude directory is mi | PASS | tests:? | init | Narrower fix (atomic_write parent only); #3000 fixes both defects incl. wrong Claude install |
| [#3007](https://github.com/rtk-ai/rtk/pull/3007) | fix(discover): encode Windows drive-colon so project auto-detect mat | PASS | tests:y | discover/Windows | Duplicate of #2952 (same one-char fix); 2952 has stronger verification and predates it |
| [#2958](https://github.com/rtk-ai/rtk/pull/2958) | fix(init): make --agent cursor install Cursor hooks only | PASS | tests:y | init | Duplicate of #3000 with less complete root-cause writeup |
| [#2986](https://github.com/rtk-ai/rtk/pull/2986) | fix(benchmark): replace live HTTP requests with local fixtures | PASS | tests:n | bench script | Dev-infra benchmark script only; no user-facing value for fork adoption |
| [#2976](https://github.com/rtk-ai/rtk/pull/2976) | fix(discover): separate Bash session count | PASS | tests:? | discover | Cosmetic reporting tweak; test plan checkboxes left unchecked |
| [#2938](https://github.com/rtk-ai/rtk/pull/2938) | perf(hook): avoid Pi package barrel import | PASS | tests:? | pi hook | Niche Pi extension TS startup perf; low relevance to fork's core |
| [#2912](https://github.com/rtk-ai/rtk/pull/2912) | fix(docs): separate Claude Code and Copilot init commands in README q | PASS | tests:n | docs | Claims 2-line README fix but diff is 7308 lines / 56 files — broken branch |
| [#2916](https://github.com/rtk-ai/rtk/pull/2916) | fix(init): add always_on trigger frontmatter to antigravity rules | PASS | tests:n | init/antigravity | Niche agent template tweak, no tests, unverifiable claim |
| [#2907](https://github.com/rtk-ai/rtk/pull/2907) | fix(gain): remove hardcoded negative sign on savings percentage | PASS | tests:n | gain display | Duplicate of #2815 fix; #2818 same change with tests, CI 0/1 here |
| [#2814](https://github.com/rtk-ai/rtk/pull/2814) | fix(gain): remove hardcoded negative sign in Recent Commands display | PASS | tests:n | gain display | Duplicate of #2815 fix; good writeup but no tests, superseded by #2818 |
| [#2865](https://github.com/rtk-ai/rtk/pull/2865) | fix(ls): preserve multiple directory headers | PASS | tests:y | ls filter | Duplicate of #2857 fix; #2868 covers same plus mixed operands and pinning |
| [#2888](https://github.com/rtk-ai/rtk/pull/2888) | fix(find): stop panicking on multi-byte UTF-8 directory names | PASS | tests:y | find/UTF-8 | Duplicate of #2851 fix; #2852 uses established ceil_char_boundary pattern |
| [#2835](https://github.com/rtk-ai/rtk/pull/2835) | fix(discover): encode -p project path for sanitized transcript dir na | PASS | tests:n | discover | Duplicate of #2834 fix; author could not compile locally, #2889 verified |
| [#2880](https://github.com/rtk-ai/rtk/pull/2880) | test(telemetry): add unit tests for telemetry_cmd.rs (fixes #1254) | PASS | tests:y | telemetry tests | Test-only, 661 lines, garbled body (shell spam), touches source for test env vars |
| [#2893](https://github.com/rtk-ai/rtk/pull/2893) | fix(benchmark): resolve file URLs with spaces on Windows | PASS | tests:n | bench scripts | Dev-infra TS scripts only, no rtk runtime impact for the fork |
| [#2845](https://github.com/rtk-ai/rtk/pull/2845) | fix(benchmark): replace live HTTP calls with local fixtures for curl/ | PASS | tests:n | bench scripts | CI-flake infra fix, test plan entirely unchecked, no fork value |
| [#2774](https://github.com/rtk-ai/rtk/pull/2774) | Next Release | PASS | tests:? | release | Bot-generated release aggregation PR, not adoptable |
| [#2783](https://github.com/rtk-ai/rtk/pull/2783) | fix(openclaw): plugin manifest and configurable exit-3 handling | PASS | tests:n | openclaw plugin | Niche OpenClaw integration, irrelevant to this fork |
| [#2750](https://github.com/rtk-ai/rtk/pull/2750) | fix(cargo): pass through --message-format=json output without text f | PASS | tests:y | cargo | Superseded: #2422 fixing same issue #2419 already in Next Release; conflict flag |
| [#2749](https://github.com/rtk-ai/rtk/pull/2749) | fix(parser): use char_indices to prevent panic on CJK/emoji output | PASS | tests:y | parser | Superseded: #2751 fixes same #2509 with byte offsets, already in Next Release |
| [#2743](https://github.com/rtk-ai/rtk/pull/2743) | fix(init): correct RTK.md summary output | PASS | tests:y | init | Cosmetic summary-text/line-count polish; 0/3 checks passing |
| [#2705](https://github.com/rtk-ai/rtk/pull/2705) | fix: make OpenCode plugin work under OpenCode Desktop (Node runtime) | PASS | tests:n | opencode plugin | Niche TS plugin fix, conflict marker, irrelevant to fork |
| [#2695](https://github.com/rtk-ai/rtk/pull/2695) | fix(git): handle -n<N> combined form in git log (fixes #2665) | PASS | tests:y | git filter | Duplicate of smaller #2740 (same fix, same issue); body placeholders stripped |
| [#2691](https://github.com/rtk-ai/rtk/pull/2691) | Add a textutil proxy for document-to-text conversion | PASS | tests:y | new feature | macOS-only textutil feature, untested on real Mac; not a bug fix |
| [#2686](https://github.com/rtk-ai/rtk/pull/2686) | fix(grep): accept common grep flags (-r, -E, -A, -i, -o, -q, -B, -C) | PASS | tests:n | grep | Body claims all flags already handled and no tests needed; unclear what 39-line diff does |
| [#2675](https://github.com/rtk-ai/rtk/pull/2675) | test(telemetry): add comprehensive unit tests for telemetry_cmd.rs | PASS | tests:y | telemetry | Duplicate of #2603 (same author/title/issue), mangled body |
| [#2672](https://github.com/rtk-ai/rtk/pull/2672) | fix(git): respect combined -n log limits | PASS | tests:y | git | Duplicate of #2666 (both fix #2665); #2666 is smaller |
| [#2661](https://github.com/rtk-ai/rtk/pull/2661) | fix(pnpm): propagate exit code from pnpm outdated | PASS | tests:n | js/pnpm | Duplicate of #2659 (both close #2658), 0/2 checks passing |
| [#2651](https://github.com/rtk-ai/rtk/pull/2651) | fix(discover): detect Claude Code hook to flag overstated missed savin | PASS | tests:y | discover | Superseded by revised approach in #2671 for same issue #2648 |
| [#2642](https://github.com/rtk-ai/rtk/pull/2642) | fix: COPILOT_INSTRUCTIONS example drops kubectl get subcommand | PASS | tests:y | hooks/init | Duplicate of #2602 (#2471); #2602 verified failing-before and ran tests |
| [#2619](https://github.com/rtk-ai/rtk/pull/2619) | fix(grep): skip redundant header on single-match results | PASS | tests:n | grep | Minor cosmetic header/grammar tweak, conflict flag |
| [#2624](https://github.com/rtk-ai/rtk/pull/2624) | fix: misleading copilot init in quick start | PASS | tests:n | docs | README wording only |
| [#2623](https://github.com/rtk-ai/rtk/pull/2623) | fix: wrong troubleshooting link in Korean README | PASS | tests:n | docs | Docs link; superseded by #2611 which fixes all translated READMEs |
| [#2611](https://github.com/rtk-ai/rtk/pull/2611) | fix(docs): update broken troubleshooting links in translated READMEs | PASS | tests:n | docs | Docs-only link fixes, no code impact |
| [#2621](https://github.com/rtk-ai/rtk/pull/2621) | Texan | PASS | tests:n | analytics/gain | 649-line web dashboard feature, unvalidated (no local tests), 0/2 checks |
| [#2596](https://github.com/rtk-ai/rtk/pull/2596) | fix(ci): make curl benchmark deterministic | PASS | tests:n | ci | Upstream CI benchmark flakiness only, irrelevant to fork; conflict flag |
| [#2577](https://github.com/rtk-ai/rtk/pull/2577) | fix(hook): use absolute claude hook command | PASS | tests:y | hook/init | Fork went the opposite way (merged #2885 removes absolute hook paths) |
| [#2564](https://github.com/rtk-ai/rtk/pull/2564) | fix(build): restore stable Rust test gate | PASS | tests:y | build | Repo-state-specific CI fix; fork develop already builds, superseded |
| [#2538](https://github.com/rtk-ai/rtk/pull/2538) | fix(gemini): correct path for Antigravity settings and hooks | PASS | tests:n | gemini hooks | Repoints GEMINI_DIR to antigravity-cli subdir; no tests, could break plain gemini users, CI red |
| [#2524](https://github.com/rtk-ai/rtk/pull/2524) | fix(openclaw): add build tooling and compiled output for plugin install | PASS | tests:n | openclaw plugin | Niche plugin packaging; commits compiled dist/ artifacts to the repo |
| [#2507](https://github.com/rtk-ai/rtk/pull/2507) | fix(init): preserve kubectl get in copilot template | PASS | tests:y | init template | Duplicate of #2471 fix; #2472 is the better-tested version of the same change |
| [#2505](https://github.com/rtk-ai/rtk/pull/2505) | fix(discover): route `rg` to `rtk rg`, not `rtk grep` | PASS | tests:y | grep/rewrite | Subset of #2460 which does the same split plus savings/docs handling; adopt #2460 instead |
| [#2451](https://github.com/rtk-ai/rtk/pull/2451) | Add The Conductor twist for Hermes warnings | PASS | tests:y | hermes plugin | Opt-in warning "branding" env switch; cosmetic, unclear value, smells like noise |
| [#2435](https://github.com/rtk-ai/rtk/pull/2435) | Add support for MiMoCode (Xiaomi opencode fork) | PASS | tests:? | agent integration | Niche new-agent feature, 1/3 checks failing, low value for fork |
| [#2370](https://github.com/rtk-ai/rtk/pull/2370) | fix(discover+ccusage): Windows drive colon not sanitized in slug + ac | PASS | tests:y | discover/ccusage | Two bugs bundled; superseded by cleaner split PRs #2368 + #2341 |
| [#2398](https://github.com/rtk-ai/rtk/pull/2398) | fix(filter): preserve inline comment markers | PASS | tests:y | core filter | Same issue #2385 as #2397 which is more thorough (state-leak fix, 11/11 CI); checks 0/2 here |
| [#2346](https://github.com/rtk-ai/rtk/pull/2346) | fix(pytest): add error count parsing for collection errors and mixed | PASS | tests:y | pytest filter | Duplicate of #2399 which is far more thorough (anchored parser, repro evidence) |
| [#2329](https://github.com/rtk-ai/rtk/pull/2329) | fix(pytest): surface collection errors in summaries | PASS | tests:y | pytest filter | Third duplicate of the pytest error-count issue; #2399 supersedes |
| [#2335](https://github.com/rtk-ai/rtk/pull/2335) | Add Windows installation instructions to README | PASS | tests:n | docs | Docs-only, empty body, 0/3 checks passing |
| [#2285](https://github.com/rtk-ai/rtk/pull/2285) | fix: add delegation confirmation to prevent LLM confusion loops | PASS | tests:n | runner output | Appends "rtk: X done" to every command — token cost + debatable design |
| [#2276](https://github.com/rtk-ai/rtk/pull/2276) | fix(git): preserve patch output for log -p | PASS | tests:y | git filter | Same issue (#2275) as #2296 which is better documented with green CI |
| [#2261](https://github.com/rtk-ai/rtk/pull/2261) | hooks/hermes: add transform_tool_result output filter for high-t | PASS | tests:y | hermes plugin | 713-line Python plugin for external Hermes system; out of scope for fork |
| [#2229](https://github.com/rtk-ai/rtk/pull/2229) | fix: bug(rewrite): hook rewrites left-hand side of piped com... | PASS | tests:n | hook/rewrite | Crude skip-on-pipe guard; superseded by #2274's last-segment approach; unchecked test plan |
| [#2223](https://github.com/rtk-ai/rtk/pull/2223) | fix(lint): detect oxlint, include message body in ESLint summary | PASS | tests:? | js/lint | Empty PR body, no explanation; overlaps with focused #2247 |
| [#2210](https://github.com/rtk-ai/rtk/pull/2210) | test(proxy): regression test for #2148 (proxy reads live FS stat | PASS | tests:y | proxy tests | Test-only pin of already-correct behavior; author never ran it locally; unix-only |
| [#2209](https://github.com/rtk-ai/rtk/pull/2209) | fix: truncate kubectl exec output before LLM context | PASS | tests:? | kubectl rewrite | Silently mutates user's inner exec commands (appends tail -100) — risky semantics |
| [#2182](https://github.com/rtk-ai/rtk/pull/2182) | Add compact dotnet ef filtering | PASS | tests:? | dotnet | 966-line new feature with empty PR body; non-core area |
| [#2143](https://github.com/rtk-ai/rtk/pull/2143) | fix(cicd): scope semgrep filesystem-deletion rule to exclude src/hooks | PASS | tests:n | ci | Upstream CI semgrep config only; irrelevant to fork |
| [#2113](https://github.com/rtk-ai/rtk/pull/2113) | fix(openclaw): add TypeScript build for OpenClaw 2026.5.4+ compatibili | PASS | tests:n | openclaw | Niche agent plugin packaging (tsconfig/npm), no Rust |
| [#2109](https://github.com/rtk-ai/rtk/pull/2109) | Compact up-to-date git push output | PASS | tests:n | git | Cosmetic tweak; author could not even build/test locally |
| [#2105](https://github.com/rtk-ai/rtk/pull/2105) | fix(telemetry): distinguish "consent not given" from missing salt in s | PASS | tests:y | telemetry | High-quality but cosmetic status-label change in low-value area |
| [#2089](https://github.com/rtk-ai/rtk/pull/2089) | fix(ci): publish linux aarch64 musl binary | PASS | tests:n | ci/release | Release-pipeline change for upstream distribution; not fork-relevant |
| [#2062](https://github.com/rtk-ai/rtk/pull/2062) | Create rtk-setup-guide.md | PASS | tests:n | docs | Doc dump with empty summary, wrong-target template untouched |
| [#2056](https://github.com/rtk-ai/rtk/pull/2056) | Add Trae.ai agent support | PASS | tests:? | hook/agents | Niche agent feature; author could not run tests locally |
| [#2052](https://github.com/rtk-ai/rtk/pull/2052) | fix(deps): pin all floating dependencies to exact versions from Cargo. | PASS | tests:n | deps | Cargo.lock already pins builds; exact = pins are questionable practice |
| [#2000](https://github.com/rtk-ai/rtk/pull/2000) | Fix find native expression passthrough | PASS | tests:y | system/find | Superseded by better-documented #2014; also mixes multi-agent hook changes, failing check |
| [#1979](https://github.com/rtk-ai/rtk/pull/1979) | fix(discover): reclassify universal-passthrough git subcommands and | PASS | tests:y | discover | Duplicate of #1897 fix; #1926 is the cleaner general solution; extra note scope, failing check |
| [#1983](https://github.com/rtk-ai/rtk/pull/1983) | fix(hooks/rewrite): restore php passthrough rewrite | PASS | tests:y | hook/rewrite | Passthrough with 0% savings and no PHP filter — tracking-only value, marginal |
| [#1912](https://github.com/rtk-ai/rtk/pull/1912) | fix(discover): guard dbg/gdbg debugger commands from rewrite hook | PASS | tests:y | hook/rewrite | Speculative niche guard for non-standard dbg/gdbg aliases only hit via user misconfig |
| [#1940](https://github.com/rtk-ai/rtk/pull/1940) | fix(gradlew): collapse consecutive blank lines in build filter outpu | PASS | tests:y | gradle | Author admits ~0 token savings; cosmetic readability in a marginal area |
| [#1880](https://github.com/rtk-ai/rtk/pull/1880) | feat(filters): add swiftlint support | PASS | tests:y | filters/swift | Niche feature, 0/2 checks passing, no savings evidence |
| [#1953](https://github.com/rtk-ai/rtk/pull/1953) | Fixes #823 : standalone rtk init for Copilot | PASS | tests:n | docs | README-only, empty summary, unchecked test plan |
| [#1847](https://github.com/rtk-ai/rtk/pull/1847) | fix(docs): repair broken links across README, docs, and templates | PASS | tests:n | docs | Thorough but docs-only link fixes; upstream-specific URLs, nothing to adopt in fork |
| [#1761](https://github.com/rtk-ai/rtk/pull/1761) | Fix link to user guide in README | PASS | tests:n | docs | Cosmetic README link fix; fork does not need upstream docs links |
| [#1600](https://github.com/rtk-ai/rtk/pull/1600) | fix(go): report failure when build produces unrecognized error outpu | PASS | tests:y | go filter | Valid but duplicate of #1638 which is more thorough for same issue #1599 |
| [#1730](https://github.com/rtk-ai/rtk/pull/1730) | Mvilrokx patch 1 | PASS | tests:? | hooks/copilot | Superseded by #1800, 0/2 checks, clippy not run, vague title |
| [#1731](https://github.com/rtk-ai/rtk/pull/1731) | [codex] Add personal progress workspace | PASS | tests:y | off-topic | 12k-line Supabase web app unrelated to rtk; spam-grade scope |
| [#1390](https://github.com/rtk-ai/rtk/pull/1390) | fix(ls): force C locale so non-English month names don't break parsing | PASS | tests:n | ls filter | excellent writeup but duplicate of #1358 which includes a test |
| [#1329](https://github.com/rtk-ai/rtk/pull/1329) | fix: count braces on signature line in AggressiveFilter | PASS | tests:y | core/filter | superseded by #1337 which fixes same bug (#1323) with fuller analysis |
| [#1328](https://github.com/rtk-ai/rtk/pull/1328) | fix: handle single-line Python docstrings in MinimalFilter | PASS | tests:y | core/filter | superseded by #1337 which fixes same bug (#1322) with fuller analysis |
| [#1332](https://github.com/rtk-ai/rtk/pull/1332) | fix(rewrite): auto-allow read-only command rewrites | PASS | tests:y | rewrite/permissions | security-sensitive permission-model change with 3 failing CI checks; too risky |
| [#1446](https://github.com/rtk-ai/rtk/pull/1446) | fix: align rewrite regression tests and hook audit behavior | PASS | tests:? | hooks/tests | vague mix of test-expectation churn and behavior tweaks, CI failing |
| [#1403](https://github.com/rtk-ai/rtk/pull/1403) | test(init): cover CODEX_HOME in Codex global init | PASS | tests:y | init/codex | test-only, author could not reproduce the underlying issue; no behavior change |
| [#1455](https://github.com/rtk-ai/rtk/pull/1455) | fix(openclaw): add installer metadata for rtk plugin package | PASS | tests:n | openclaw | niche OpenClaw plugin packaging, not relevant to fork |
| [#1389](https://github.com/rtk-ai/rtk/pull/1389) | fix(openclaw): preserve rewrite stdout on exit 3 | PASS | tests:n | openclaw | OpenClaw-only TS plugin fix, 5/6 checks failing, niche |
| [#1312](https://github.com/rtk-ai/rtk/pull/1312) | fix(ci): add write permissions to pr-target-check workflow | PASS | tests:n | ci | upstream GitHub Actions infra, irrelevant to fork |
| [#1306](https://github.com/rtk-ai/rtk/pull/1306) | fix(docs): fix broken links in README files | PASS | tests:n | docs | translated-README link fixes only, no code |
| [#1485](https://github.com/rtk-ai/rtk/pull/1485) | Update README_ko.md to match version 0.28.2 | PASS | tests:n | docs | Korean docs sync, no code changes |
| [#1287](https://github.com/rtk-ai/rtk/pull/1287) | fix(tracking): rename history.db to tracking.db to match documentation | PASS | tests:y | tracking | Naming churn with migration risk to fix a doc mismatch; fixing docs is cheaper; CI 0/2 |
| [#1271](https://github.com/rtk-ai/rtk/pull/1271) | [Documentation] Add DeepWiki badge to README | PASS | tests:n | docs | Cosmetic badge, no code value |
| [#1270](https://github.com/rtk-ai/rtk/pull/1270) | test(telemetry_cmd): add unit tests for telemetry subcommands | PASS | tests:y | telemetry | Test-only addition for telemetry path; no behavior fix, low fork value |
| [#1242](https://github.com/rtk-ai/rtk/pull/1242) | fix: disambiguate rtk (Rust Token Killer) from Rust Type Kit | PASS | tests:n | branding | Cosmetic version-string/branding tweaks; fork already documents the collision |
| [#1236](https://github.com/rtk-ai/rtk/pull/1236) | fix(openclaw): fix plugin install — add openclaw.extensions, document  | PASS | tests:n | openclaw | Niche openclaw plugin packaging/docs, irrelevant to fork's core use |
| [#1186](https://github.com/rtk-ai/rtk/pull/1186) | fix(discover): expand npm rewrite rule to match install and other subc | PASS | tests:n | hook rewrite/npm | Duplicate of #1148 fix; #1204 does it better with tests and curated list |
| [#1150](https://github.com/rtk-ai/rtk/pull/1150) | fix(rules): expand npm rewrite pattern to include install, ci, list, o | PASS | tests:y | hook rewrite/npm | Duplicate of #1148 fix; superseded by #1204's more complete version |
| [#1100](https://github.com/rtk-ai/rtk/pull/1100) | fix(cursor): handle exit code 3 in rtk-rewrite.sh | PASS | tests:n | cursor hook | Same bug as #1075, which absorbed this author's review feedback and is more hardened |
| [#953](https://github.com/rtk-ai/rtk/pull/953) | Hardening Script for corporative/enterprise users | PASS | tests:n | scripts | 702-line vague enterprise hardening script, no cargo checks run, risky and out of scope |
| [#864](https://github.com/rtk-ai/rtk/pull/864) | fix(init): warn when jq is missing during hook installation | PASS | tests:y | init | Self-described stopgap superseded by native rtk hook claude work (#785) |
| [#715](https://github.com/rtk-ai/rtk/pull/715) | fix(init): honor OPENCODE_CONFIG_DIR for OpenCode plugin path | PASS | tests:y | init/opencode | Niche OpenCode integration, irrelevant to this fork's usage |
| [#628](https://github.com/rtk-ai/rtk/pull/628) | fix(contributing): fix contributing guidelines | PASS | tests:n | docs | Docs-only; fork already cleaned CONTRIBUTING recently, likely superseded |
| [#609](https://github.com/rtk-ai/rtk/pull/609) | Config to install on NixOs as a flake | PASS | tests:n | packaging | Nix packaging feature, CI failing, no adoption value for a Windows fork |
| [#583](https://github.com/rtk-ai/rtk/pull/583) | fix: resolve clippy warnings in container, git, and init modules | PASS | tests:n | cosmetic | Pure clippy cleanup, no behavior change, high staleness/conflict risk |
| [#456](https://github.com/rtk-ai/rtk/pull/456) | Added German Translation | PASS | tests:n | docs | Marketing/tool-generated translation PR, no value |
| [#447](https://github.com/rtk-ai/rtk/pull/447) | Add documentation for RTK CLI proxy | PASS | tests:n | docs | Docs-only addition, no code |
| [#341](https://github.com/rtk-ai/rtk/pull/341) | Add az CLI provider with compact Azure DevOps/Monitor/WebApp outp | PASS | tests:y | new provider | 1816-line new feature, huge/risky, stale (Mar 05) |
| [#306](https://github.com/rtk-ai/rtk/pull/306) | Replace lazy_static with std::sync::LazyLock | PASS | tests:n | refactor | 14-file refactor contradicting repo's documented lazy_static pattern; old, high conflict risk |
| [#2890](https://github.com/rtk-ai/rtk/pull/2890) | fix(install): warn when multiple rtk own the name on PATH | UNSURE | tests:? | install | No body at all; topic is relevant (name collision) but nothing to judge |
| [#2302](https://github.com/rtk-ai/rtk/pull/2302) | fix(hook): rewrite rg through rtk rg | UNSURE | tests:y | hook/rewrite | Depends on whether `rtk rg` backend exists/is correct; CI failing, legacy suite unrun |
| [#1595](https://github.com/rtk-ai/rtk/pull/1595) | fix, add correct copilot init cmd | UNSURE | tests:n | init | 3-line change; body describes missing ~/.claude failure but title says copilot; unclear what it does |

## Auto-passed on metadata (69)

| PR | Title | Reason |
|---|---|---|
| [#3015](https://github.com/rtk-ai/rtk/pull/3015) | docs: add Takayuki Maeda as core contributor | non-code or maintenance-only |
| [#3013](https://github.com/rtk-ai/rtk/pull/3013) | docs: clarify Copilot quick start command | non-code or maintenance-only |
| [#2996](https://github.com/rtk-ai/rtk/pull/2996) | fix(find): prevent silent false negatives | draft |
| [#2987](https://github.com/rtk-ai/rtk/pull/2987) | docs(readme): separate Copilot setup command | draft |
| [#2979](https://github.com/rtk-ai/rtk/pull/2979) | feat(git): add GitButler command filtering | draft |
| [#2978](https://github.com/rtk-ai/rtk/pull/2978) | docs: clarify Copilot initialization in Quick Start | non-code or maintenance-only |
| [#2957](https://github.com/rtk-ai/rtk/pull/2957) | docs: update Windows hook documentation | non-code or maintenance-only |
| [#2872](https://github.com/rtk-ai/rtk/pull/2872) | docs: add README_zh-TW.md translation | non-code or maintenance-only |
| [#2777](https://github.com/rtk-ai/rtk/pull/2777) | docs: add italian translation | non-code or maintenance-only |
| [#2765](https://github.com/rtk-ai/rtk/pull/2765) | docs: Add accents in README_fr.md | non-code or maintenance-only |
| [#2704](https://github.com/rtk-ai/rtk/pull/2704) | docs: document native-Windows manual settings.json hook (no WSL) | non-code or maintenance-only |
| [#2699](https://github.com/rtk-ai/rtk/pull/2699) | fix(read): window tracking baseline | draft |
| [#2698](https://github.com/rtk-ai/rtk/pull/2698) | feat(hooks): allow configured ask rewrites | draft |
| [#2697](https://github.com/rtk-ai/rtk/pull/2697) | ci(pr): enforce conventional titles | draft |
| [#2696](https://github.com/rtk-ai/rtk/pull/2696) | feat(rewrite): support bare turbo commands | draft |
| [#2639](https://github.com/rtk-ai/rtk/pull/2639) | feat(config): add capture = "pty" for tools that hang on a pipe (Part 3/3) | draft |
| [#2638](https://github.com/rtk-ai/rtk/pull/2638) | feat(config): inject [[tools]] env vars before spawn (Part 2/3) | draft |
| [#2636](https://github.com/rtk-ai/rtk/pull/2636) | refactor(hooks): inject the permission verdict so rewrite_cmd tests don't read ~ | non-code or maintenance-only |
| [#2633](https://github.com/rtk-ai/rtk/pull/2633) | [codex] fix env-stable rewrite tests and grep header | draft |
| [#2604](https://github.com/rtk-ai/rtk/pull/2604) | chore(supply-chain): add cargo-vet config with trusted audit imports | non-code or maintenance-only |
| [#2600](https://github.com/rtk-ai/rtk/pull/2600) | chore: fix SPDX license id, refresh version refs, drop stale formula | non-code or maintenance-only |
| [#2562](https://github.com/rtk-ai/rtk/pull/2562) | docs(readme): fix troubleshooting link in README_ko.md | non-code or maintenance-only |
| [#2525](https://github.com/rtk-ai/rtk/pull/2525) | docs(readme): replace hardcoded version pins with version-agnostic check | non-code or maintenance-only |
| [#2418](https://github.com/rtk-ai/rtk/pull/2418) | refactor(js): decouple vitest/jest runner from CLI Commands enum | non-code or maintenance-only |
| [#2402](https://github.com/rtk-ai/rtk/pull/2402) | feat: add Codex PreToolUse hook integration | draft |
| [#2356](https://github.com/rtk-ai/rtk/pull/2356) | refactor(hook): slim agent awareness for hooked agents | non-code or maintenance-only |
| [#2278](https://github.com/rtk-ai/rtk/pull/2278) | docs: clarify PowerShell cmdlet usage for Codex | non-code or maintenance-only |
| [#2269](https://github.com/rtk-ai/rtk/pull/2269) | feat(windows): native Copilot Chat hook support without WSL | draft |
| [#2266](https://github.com/rtk-ai/rtk/pull/2266) | docs(readme): add dedicated Claude Code setup section | non-code or maintenance-only |
| [#2257](https://github.com/rtk-ai/rtk/pull/2257) | docs(readme): clarify Copilot init command | non-code or maintenance-only |
| [#2196](https://github.com/rtk-ai/rtk/pull/2196) | ci: add aarch64-unknown-linux-musl release target (closes #1331) | non-code or maintenance-only |
| [#2195](https://github.com/rtk-ai/rtk/pull/2195) | docs: fix contributing guide typos | non-code or maintenance-only |
| [#2156](https://github.com/rtk-ai/rtk/pull/2156) | docs(hooks): add "reading the full output" to agent instructions | non-code or maintenance-only |
| [#2152](https://github.com/rtk-ai/rtk/pull/2152) | docs: Change 'cursor' to 'agent cursor' in init command | non-code or maintenance-only |
| [#2151](https://github.com/rtk-ai/rtk/pull/2151) | docs: add Turkish translation (README_tr.md) | non-code or maintenance-only |
| [#2116](https://github.com/rtk-ai/rtk/pull/2116) | docs: update native Windows hook guidance | non-code or maintenance-only |
| [#2079](https://github.com/rtk-ai/rtk/pull/2079) | docs: add Persian translation for README | non-code or maintenance-only |
| [#1927](https://github.com/rtk-ai/rtk/pull/1927) | chore: git missing subcommands | non-code or maintenance-only |
| [#1902](https://github.com/rtk-ai/rtk/pull/1902) | docs(readme): avoid stale version check | non-code or maintenance-only |
| [#1884](https://github.com/rtk-ai/rtk/pull/1884) | Feat/hermes agent | draft |
| [#1758](https://github.com/rtk-ai/rtk/pull/1758) | docs: align telemetry disclaimer with opt-in policy | non-code or maintenance-only |
| [#1746](https://github.com/rtk-ai/rtk/pull/1746) | docs(zh): sync README_zh.md with current README.md | non-code or maintenance-only |
| [#1742](https://github.com/rtk-ai/rtk/pull/1742) | chore(deps): bump quick-xml from 0.37.5 to 0.40.1 | dependabot (upstream will handle deps) |
| [#1702](https://github.com/rtk-ai/rtk/pull/1702) | docs: add Turkish README translation | non-code or maintenance-only |
| [#1693](https://github.com/rtk-ai/rtk/pull/1693) | refacto(crate): split into lib + bin for downstream bundling | non-code or maintenance-only |
| [#1673](https://github.com/rtk-ai/rtk/pull/1673) | refactor(json): Single-line output with truncation hints | non-code or maintenance-only |
| [#1625](https://github.com/rtk-ai/rtk/pull/1625) | build(deps): bump tempfile from 3.26.0 to 3.27.0 | dependabot (upstream will handle deps) |
| [#1623](https://github.com/rtk-ai/rtk/pull/1623) | build(deps): bump colored from 2.2.0 to 3.1.1 | dependabot (upstream will handle deps) |
| [#1622](https://github.com/rtk-ai/rtk/pull/1622) | build(deps): bump which from 8.0.1 to 8.0.2 | dependabot (upstream will handle deps) |
| [#1621](https://github.com/rtk-ai/rtk/pull/1621) | build(deps): bump ureq from 2.12.1 to 3.3.0 | dependabot (upstream will handle deps) |
| [#1620](https://github.com/rtk-ai/rtk/pull/1620) | build(deps): bump actions/download-artifact from 4 to 8 | dependabot (upstream will handle deps) |
| [#1619](https://github.com/rtk-ai/rtk/pull/1619) | build(deps): bump googleapis/release-please-action from 4 to 5 | dependabot (upstream will handle deps) |
| [#1618](https://github.com/rtk-ai/rtk/pull/1618) | chore(deps): bump actions/setup-go from 5 to 6 | dependabot (upstream will handle deps) |
| [#1617](https://github.com/rtk-ai/rtk/pull/1617) | chore(deps): bump actions/github-script from 7.1.0 to 9.0.0 | dependabot (upstream will handle deps) |
| [#1616](https://github.com/rtk-ai/rtk/pull/1616) | build(deps): bump actions/checkout from 4 to 6 | dependabot (upstream will handle deps) |
| [#1578](https://github.com/rtk-ai/rtk/pull/1578) | feat(git): support git sparse-checkout subcommand | draft |
| [#1438](https://github.com/rtk-ai/rtk/pull/1438) | docs(i18n): add Brazilian Portuguese README | non-code or maintenance-only |
| [#1411](https://github.com/rtk-ai/rtk/pull/1411) | docs: clarify Codex rewrite behavior | non-code or maintenance-only |
| [#1365](https://github.com/rtk-ai/rtk/pull/1365) | feat(omp): add extension-based rewrite integration for Oh My Pi | draft |
| [#1333](https://github.com/rtk-ai/rtk/pull/1333) | docs(license): make repo license references consistent | non-code or maintenance-only |
| [#1202](https://github.com/rtk-ai/rtk/pull/1202) | docs: add Portuguese (Brazil) README | non-code or maintenance-only |
| [#1170](https://github.com/rtk-ai/rtk/pull/1170) | docs: add comprehensive filtering behavior reference | non-code or maintenance-only |
| [#1051](https://github.com/rtk-ai/rtk/pull/1051) | docs: clarify token consumption type in main title description | non-code or maintenance-only |
| [#943](https://github.com/rtk-ai/rtk/pull/943) | ci: enforce test presence on new/modified filter modules | non-code or maintenance-only |
| [#871](https://github.com/rtk-ai/rtk/pull/871) | docs: update grep descriptions for passthrough behavior | non-code or maintenance-only |
| [#853](https://github.com/rtk-ai/rtk/pull/853) | Docs/add german readme de only | non-code or maintenance-only |
| [#636](https://github.com/rtk-ai/rtk/pull/636) | docs: add sandbox troubleshooting for Claude Code tracking | non-code or maintenance-only |
| [#560](https://github.com/rtk-ai/rtk/pull/560) | docs: add prompt caching FAQ (RTK does not break cache) | non-code or maintenance-only |
| [#421](https://github.com/rtk-ai/rtk/pull/421) | docs: reposition RTK as agent-agnostic | non-code or maintenance-only |
