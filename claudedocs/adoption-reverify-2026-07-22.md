# Upstream PR Re-verification Sweep — 2026-07-22

Full re-audit of the [2026-07-17 triage](adoption-triage-2026-07-17.md) and [batch-2 queue](adoption-batch2-2026-07-19.md) against current upstream state: 877 open PRs (22 agent chunks of ~40), every documented CANDIDATE re-checked for state/supersession, all ~130 PRs opened after the sweep (#3032–#3168) triaged fresh.

**Headline numbers**: ~35 documented CANDIDATEs corrected (superseded upstream, covered by fork adoptions, or dup-cluster consolidation) · 7 missed CANDIDATEs recovered from the metadata auto-pass bucket · 16 new CANDIDATEs among post-sweep PRs · top 10 filed as fork issues (see below).

**Biggest single source of staleness**: upstream's grep/search rework (`eafadce`, 2026-06-26 — "run the invoked engine instead of substituting rg for grep", `grep_cmd.rs` → `search.rs`, + `0adfae6` flag shapes) silently superseded **eight** documented candidates: #2183 #2168 #2126 #2149 #2434 #2460 #2254 #2224 (+ #2607, #2348, and demoted #1678 #1541 #2910 #2438). Second source: `d0a77cd` (ccusage `period` alias) superseded #1968 #2341 #2227.

## Corrections to the 2026-07-17 triage

### CANDIDATEs now superseded/stale — drop (close if a ticket exists)

| PR | Was | Why dropped |
|---|---|---|
| #673 pytest xfailed | CANDIDATE | Upstream pytest_cmd.rs tracks xfailed/xpassed; -q summary fixed in ecc34d7 (merged #588) |
| #679 clippy `--` separator | CANDIDATE | Replaced by `restore_double_dash_with_raw` (args_utils.rs) |
| #419 gh run view passthrough | CANDIDATE | `should_passthrough_run_view` + `--job`/`--attempt` upstream (e1bb17f); only `-j`/`-a` short forms remain |
| #792 diff identical-files guard | CANDIDATE | c126d45 rewrote guard + exit convention (fixes #2364) |
| #1075 cursor hook exit codes | CANDIDATE | Cursor moved to native Rust binary hook; also off-profile |
| #1185 intent-to-add status | CANDIDATE | `format_status_inner` rewritten — verbatim porcelain |
| #1204 npm rewrite subcommands | CANDIDATE | Superseded by fork-adopted #1951 |
| #1286 looks_like_path branch refs | CANDIDATE | Function gone; 13188a8 fixed differently |
| #1294 pytest xfailed | CANDIDATE | Same as #673 |
| #1358 ls C locale | CANDIDATE | b51a815/0d70760 LC_ALL=C |
| #1386 git show blob corruption | CANDIDATE | `is_blob_show_arg()` passthrough upstream |
| #1423 hook_check other integrations | CANDIDATE | `other_integration_installed()` at hook_check.rs:158 |
| #1508 ls .env in NOISE_DIRS | CANDIDATE | constants.rs:17 — `.env` intentionally removed |
| #1513 copilot instructions clobber | CANDIDATE | Marker-block upsert (init.rs) |
| #1638 go build false Success | CANDIDATE | f69ad6e exit-status driven |
| #1678 grep --pcre2 | CANDIDATE | grep rework removed rg-substitution path |
| #1689 ls LC_TIME | CANDIDATE | ls.rs:52 LC_ALL=C |
| #1725 grep --with-filename | CANDIDATE | search.rs:287 already passes it |
| #1843 ls .env NOISE_DIRS | CANDIDATE | Same as #1508 |
| #1857 detached HEAD SHA | CANDIDATE | `extract_detached_head()` (62fc0e0) |
| #1968 ccusage period field | CANDIDATE | d0a77cd serde alias — already in fork develop |
| #2049 stream UTF-8 drop | CANDIDATE | Fixed **in fork** via adopted #2997 (`read_lines_lossy`, c35968f) |
| #2126/#2168/#2183/#2149 grep cluster | CANDIDATE | All superseded by eafadce/0adfae6 |
| #2224/#2254/#2348 grep fallback strip | CANDIDATE | Same — rg-substitution path deleted |
| #2227 ccusage degrade | CANDIDATE | d0a77cd fixed root cause |
| #2263 (already struck 07-17) / #2341 ccusage | CANDIDATE | #2341 = d0a77cd identical fix |
| #2434/#2460 grep tool identity | CANDIDATE | eafadce implements the same design (incl. `rtk rg`) — do NOT adopt #2460 (the 07-17 mega-cluster note is obsolete) |
| #2440 backslash-newline strip | CANDIDATE | `collapse_line_continuations()` (registry.rs:531-570) |
| #2450 diff POSIX exit codes | CANDIDATE | 0/1 merged via #2394; adopt #2473 for the exit-2 gap instead |
| #2488 gain CJK truncation panic | CANDIDATE | 47b22e0/c9468ee char-safe truncation |
| #2533 parser UTF-8 boundaries | CANDIDATE | 27f9739 char_indices (fixes #2509), in fork develop |
| #2571 hook ask on compounds | CANDIDATE | Fork's #3031 port (e93cde8) strictly better (bypassPermissions carve-out) |
| #2602 copilot kubectl get | CANDIDATE | Upstream merged #2472 (f9d8c77) |
| #2607 grep --include/--exclude | CANDIDATE | grep rework — runs native grep |
| #2910 grep strip -E | CANDIDATE | rtk grep now execs real grep where -E is valid; adopting the strip would *break* ERE |
| #2302 (UNSURE) | UNSURE | Moot — `rtk rg` exists upstream |
| #2226 TOML filters in hook (2nd-wave) | SECOND-WAVE | Merged upstream as #2748 (31f9d43) |

### CANDIDATEs downgraded or narrowed

| PR | Now | Note |
|---|---|---|
| #737 npx -y | CANDIDATE (narrowed) | ccusage path fixed upstream; only tsc/next/prisma bare-`npx` call sites remain (few lines) |
| #837 orphaned hook cleanup | SECOND-WAVE | Reinstall overwrite + uninstall removal landed; only `init --show` warning novel |
| #1133 git op state + push blocking | PASS | `GitStatusState` covers the state half; rest is opinionated policy |
| #1444 go bench/fuzz | narrowed | b058c96 fixed bench; slivers remain, go peripheral |
| #1541 grep -c | re-verify | grep rework changed root cause |
| #1588 push rejection | SECOND-WAVE | `run_push_filter` failure path covers most; only exit-0/GH013 edge remains (see #2560) |
| #1856 --no-merges root fix | SECOND-WAVE | Upstream skips injection for --merges/min-parents/explicit counts; residual = count-less log (see #2328/#2016/#2264 cluster) |
| #1926 discover passthrough classes | CANDIDATE (rebase) | bb01d6c added checkout; general fallback still valid |
| #2016 merge-commit wrong SHA | CANDIDATE (narrowed) | Limit-flag case fixed upstream; positional-ref/--graph/--format shapes still wrong |
| #2048 hook eprintln gating | re-implement | hook_cmd.rs now clean; residual stderr risk in trust.rs/toml_filter.rs/permissions.rs |
| #2155 UTF-8 trio | parts 1+3 only | gain part upstream; core.quotepath + stream trailing-multibyte remain |
| #2221 git fetch + go build | git half only | go half fixed by f69ad6e; `run_fetch` still null-stdin |
| #2296 git log -p | check vs fork | Likely covered by fork-adopted #2951 (issue #33) — verify residuals before any adoption |
| #2438 grep count caps | re-verify | search.rs rework likely fixed |
| #2479 hook explicit allow | re-implement | Bug persists in native `run_claude()` (Skip/Ignore emit nothing, hook_cmd.rs:423-427) but PR patches the legacy shell script |
| #2591 binary match lines | SECOND-WAVE | `unparsed_signal()` passthrough mitigates data loss; savings-only now |
| #2612/#2396 exclude head/tail | drop | Cluster covered by filed #2887 (issue #24) — adopt exactly one |
| #2368 drive colon | drop | Dup of adopted #2952 |
| #2920 Windows discover case | CANDIDATE (half) | Colon half in fork via #2952; case-insensitive half still needed (provider.rs:84) |
| #2664/#2677 npm subcommands | drop | Superseded by adopted #1951 (batch2 already said so) |
| #2670/#2679 ruff dup pair | keep #2679 | Same fix for #2669 |

### Missed CANDIDATEs recovered from the auto-pass bucket

| PR | Why it matters |
|---|---|
| #2075 init: PowerShell hook matcher for Claude Code | `rtk init` registers only a `Bash` matcher — on Windows, Claude Code's PowerShell tool calls never hit the hook. Fork's exact daily platform |
| #1504 Windows hook detection (.cmd wrapper + REM version) | False "No hook installed" nag on Windows; adopt hook-detection half only |
| #1815 git grep filter | Core git area, mergeable, rebased |
| #1873 pnpm script rewrite + vitest streaming | Core pnpm/vitest workflow; prevents agent timeouts |
| #1987 tracking.enabled + arg redaction | Flag still dead code; secrets stored verbatim in history.db (conflicted, adopt selectively) |
| #2776 pnpm lint script rewrite drops flags | False exit 0 on lint failures; diff-verified, tested (fixes upstream #2094) |
| #2875 / #2547 native Windows ls/grep/wc/tree | Same-scope pair; SECOND-WAVE due to size/conflicts — mitigated by Git usr\bin on PATH, but the biggest Windows payoff available |

### Adoption-time state changes (verdicts stand)

Now conflicted since 07-17: #843 #891 #925 #1302 #1251 #2090 #2014 #2078 #2544 #2579 #2590 #2615 #2970 #3052. #2871 flag: patched helper `exec_capture_stdin` is shared with piped-grep stdin (37ee6cf) — audit callers before adopting. #2652 flag: upstream test now asserts the overflow indicator; adopting is a deliberate divergence.

## Batch-2 queue corrections

- **#2274 framing was wrong** ("overlaps our adopted #2965 — re-check what remains"): real gaps confirmed in fork source. The #2965 blacklist (registry.rs:605) covers only 8 shape-changing filters — `curl … | jq` still gets rewritten (truncated JSON), and `git diff > file.patch` still rewrites the command ahead of a redirect (corrupted patch; only cat/head/tail have the redirect exclusion). Adopt the *idea* of #2274 (last-segment-only + stdout-redirect skip) as a re-implementation.
- **#2571**: batch2's suspicion confirmed — fully covered by fork's e93cde8; mark PASS.
- Queue items re-verified still valid: #2535 #2475 #2565 #2483 #1985 #2887 #2830 #2321 #1047 #742-adjacent (all filed issues remain sound). #1951 supersession list confirmed.
- New dup-flood author since batch2: `albatrossflyon-coder` (9 PRs 07-21/22; #3135≈#3101, #3154≈#3121, #3155≈#3120 — higher quality than lntutor but prefer originals).

## Fresh triage: post-sweep PRs (#3032–#3168)

**New CANDIDATEs**: #3041 (exclude_commands bypassed for resolved tool — `python -m pytest`), #3042 (read stdin raw fallback; prefer over dup #3039), #3044 (telemetry forget honors get_db_path — same bug as conflicted #2970, prefer #3044), #3045 (find files-only default empties dir searches), #3048 (diff dumps both files — never_worse baseline bug), #3049 (preserve foreign PreToolUse hooks on uninstall; prefer over dup #3081), #3052 (bare `rg` routed to `rtk grep` instead of `rtk rg`; conflicted), #3060 (playwright: preserve explicit reporter), #3067 (js parser fallbacks lose stderr-only failures), #3101 (git commit multibyte byte-slice panic), #3109 (tail line-range exclusions — same cluster as filed #2887), #3132 (read --tail-lines unbounded memory / 49GB OOM), #3134 (search bounded capture), #3162 (rg `-r`/`-R` read as `--replace` — silent match rewriting), #3164 (discover counts hook rewrites as misses, ~19.5pt undercount), #3166 (head -N returned non-contiguous lines; overlaps #2964 — adopt together).

**SECOND-WAVE**: #2999 #3047 #3057 (breaking, wait upstream) #3061 #3064 #3075 #3088 #3089 #3090 #3114 #3115 #3120 #3121 #3128 #3131 #3136 #3147 #3150 #3165 #3167 #3168 (buries a real fix: `[tracking] enabled=false` ignored).

**PASS**: remainder — dups of triaged originals (#3083–#3087, #3135, #3154, #3155, #3081, #3043), niche agents, peripheral ecosystems, docs.

## Top 10 — filed as fork issues 2026-07-22

Ranked for the fork profile (Windows 11 + Claude Code + pnpm/vitest/Next/tsc/Playwright + rtk's own Rust dev). All verified against current upstream develop this sweep.

| # | Upstream PR | Fork issue | What |
|---|---|---|---|
| 1 | [#2075](https://github.com/rtk-ai/rtk/pull/2075) | [#45](https://github.com/kylehgc/rtk/issues/45) | init: register PowerShell matcher — hook currently silent for all Claude Code PowerShell traffic on Windows |
| 2 | [#3162](https://github.com/rtk-ai/rtk/pull/3162) | [#46](https://github.com/kylehgc/rtk/issues/46) | search: strip rg `-r`/`-R` — silent match-content corruption in the hottest search path |
| 3 | [#3041](https://github.com/rtk-ai/rtk/pull/3041) | [#47](https://github.com/kylehgc/rtk/issues/47) | hooks: apply exclude_commands to the resolved tool (`python -m pytest` bypass) |
| 4 | [#2573](https://github.com/rtk-ai/rtk/pull/2573) | [#48](https://github.com/kylehgc/rtk/issues/48) | git: keep machine output raw (`--porcelain`/`--format`/`-z` byte-exact) |
| 5 | [#3067](https://github.com/rtk-ai/rtk/pull/3067) | [#49](https://github.com/kylehgc/rtk/issues/49) | js: preserve failed vitest/playwright/pnpm output in parser fallbacks (stderr loss) |
| 6 | [#2964](https://github.com/rtk-ai/rtk/pull/2964) + [#3166](https://github.com/rtk-ai/rtk/pull/3166) | [#50](https://github.com/kylehgc/rtk/issues/50) | read: head-rewrite fidelity — exact --max-lines budget + literal contiguous prefix |
| 7 | [#2575](https://github.com/rtk-ai/rtk/pull/2575) | [#51](https://github.com/kylehgc/rtk/issues/51) | rewrite: preserve stdin-driven commands (`git apply`, `docker build -`, `kubectl -f -`) |
| 8 | [#2628](https://github.com/rtk-ai/rtk/pull/2628) | [#52](https://github.com/kylehgc/rtk/issues/52) | grep: stop `-l`/`-m`/`-t` shadowing native flags (grep -l exits 2 today) |
| 9 | [#2877](https://github.com/rtk-ai/rtk/pull/2877) | [#53](https://github.com/kylehgc/rtk/issues/53) | cargo: preserve compiler warnings in passing `cargo test` runs |
| 10 | [#2401](https://github.com/rtk-ai/rtk/pull/2401) | [#54](https://github.com/kylehgc/rtk/issues/54) | build: keep release `panic=unwind` so filter `catch_unwind` fail-open actually works |

**Next-wave shortlist** (queue behind the 10): #3002 (err/test quoting + exit codes, Windows-verified; supersedes-ish #2393/#1194 cluster), #2016 (git log wrong-SHA residual shapes; --no-merges cluster pick), #1122 + #2796 (gh pr checks: don't drop output on failure + status-column classification — batch), #3049 (foreign hooks on uninstall), #2776 (pnpm lint script rewrite), #2184 (tree native Windows), #3101 (git commit panic), #891/#1298/#1299 (weighted gain trio), #2397 + #2715 + #1337 (MinimalFilter code-loss cluster), #2643 (CRLF diff), #3164 (discover coverage accuracy), #2917 (quote-aware git -C stripping), #2757 (next build), #2659 (pnpm outdated exit code), #3132 (tail OOM).
