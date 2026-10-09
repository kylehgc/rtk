# Upstream sweep — 2026-10-09

Incremental triage of rtk-ai/rtk open PRs above the previous watermark.

- **Coverage**: open upstream PRs **#3796–#4491** (watermark from `upstream-sweep-2026-08-31.md` was #3790). 253 open PRs swept: 42 auto-passed (drafts, own, maintainer-authored deferred to sync), 211 classified by 9 parallel agents.
- **Verdicts**: roughly 75 agent-flagged CANDIDATEs, consolidated into 30 tickets drafted; 29 filed (rank 1–2), 1 rank-3 held back. The rest PASS (off-profile harnesses, Python/Go/JVM/.NET tools, features, cosmetic, superseded) or UNSURE.
- **Verified against**: fork develop `9af2b0f3` by code inspection. `upstream/develop` was already ahead (`be8d546d`) and was **not merged first**, so a later sync may supersede some tickets.
- **Tickets filed**: fork **#169–#197** (29 tickets) + 5 cross-reference comments (#144, #146, #143, #114, #104).

## Tickets filed

| Fork issue | Upstream PRs | Rank | Area |
|---|---|---|---|
| #169 | 4174 + 4225 | 1 | git log: a commit body containing `---END---` forges a record boundary |
| #170 | 3895 | 1 | npm: `filter_npm_output` drops Jest code frames, blank lines and `...` markers |
| #171 | 4184 | 1 | vitest: a suite-load failure reports `PASS (0) FAIL (0)` |
| #172 | 4342 | 1 | find: `-delete` runs against the wrong target when the first arg is not an existing directory |
| #173 | 3996 + 4327 | 1 | find: grouped render mangles filenames containing spaces and search roots |
| #174 | 4305 | 1 | ls: entries ending in `.` are silently dropped |
| #175 | 4406 | 1 | gh: `issue view -c/-q/-w` and `pr view` short flags take the filtered path and drop comments |
| #176 | 4185 | 1 | eslint: errors without a `ruleId` never reach the summary and warning-only files can crowd out error files |
| #177 | 4087 + 4347 | 1 | json: value loss in `rtk json` (scalar arrays truncated, `--keys-only` can echo raw values) |
| #178 | 3830 | 1 | git branch: `--format` output is trimmed and blank lines dropped |
| #179 | 4258 + 4294 | 1 | read: `Language::Unknown` inherits C comment syntax; aggressive level leaks function locals |
| #180 | 4276 + 4066 | 1 | read: UTF-8 BOM breaks line-1 classification; non-UTF-8 files exit 1 with no output |
| #181 | 4386 | 1 | git commit: a successful commit hides pre-commit hook warnings |
| #182 | 4450 | 2 | hooks: an apostrophe in an unquoted `#` comment hides the rest of the command from the permission gate |
| #183 | 4287 | 2 | hooks: an unreadable or unparseable permissions settings file fails open |
| #184 | 3879 + 4490 | 2 | hooks: git is unusable in Claude Code worktree sessions because the guard rejects rewritten `rtk git` |
| #185 | 4469 (ref 4382) | 2 | rewrite: a pipeline producer followed by `tail`/`head` hides the failure summary |
| #186 | 4488 (+ 4487) | 2 | rewrite: `grep -r` is rewritten to `rtk grep -r`, dropping .gitignore protection |
| #187 | 4264 | 2 | vitest 5: the JSON reporter writes to `outputFile`, so every parser tier fails |
| #188 | 4442 | 2 | jest: `-t run` has every `run` token stripped |
| #189 | 3991 | 2 | parser: `extract_json_object` miscounts CRLF offsets and can slice wrongly or panic on a multibyte prefix |
| #190 | 4484 | 2 | stream: `StdinMode::Null` pipes stdin, which fails with os error 231 under an MSYS parent on Windows |
| #191 | 4027 + 4344 | 2 | gh run view: `--exit-status` skips the filter and `--job` goes through the run-summary filter |
| #192 | 4191 + 4165 | 2 | git log: injected `-N` truncates before `--reverse`, and `--stdin` reads nothing |
| #193 | 3865 | 2 | err: bracketed `[ERROR]` / `[WARNING]` lines are dropped |
| #194 | 4440 | 2 | bun test: `--bail` reads as clean and `--coverage` is dropped |
| #195 | 4454 | 2 | cargo test: `-- --list` is run through the test-result filter |
| #196 | 4141 | 2 | init: exits 0 without installing the hook when stdin is not a TTY |
| #197 | 4375 | 2 | init: `tracking.enabled = false` is never read and a minimal `[tracking]` table fails to parse |

Comments: #144 ← 4265 · #146 ← 3897 · #143 ← 4348 · #114 ← 4253 · #104 ← 3849.

## Held back (not filed)

- **Rank 3, verified, not filed by choice**: #4405 (uninstall deletes any line starting with `@RTK.md`).
- **Rank 3–4 candidates**: #4304/#4261/#4254 (CLAUDE_CONFIG_DIR handling), #3973/#4409 (cc-economics numbers), #4104/#4109 (rewrite tab and subcommand boundary), #4082 (git add --dry-run), #4308 (telemetry ping on hook path), #4250, #4464, #4468, #4417, #4253 (commented on #114), #3915, #4032, #4166, #4081, #4234, #4376, #3799, #3938, #4070.
- **Off-profile real bugs**: pytest (#4466, #4192, #4374), ruff (#3942, #3965), Go (#3992, #4430, #4438, #4455), JVM (#4416, #4208), cargo nextest (#4467), go fuzz (#4437), kubectl (#4334).
- **UNSURE (stay out)**: #4069, #4090 (Windows regex quoting for grep/rg; diff listing showed tests only, re-check by hand), #4188, #4223, #4446, #4019, #4077, #4260, #3880, #4382 (folded into the #4469 ticket as a reference), #4031.
- **Superseded / site gone**: #4145, #4060, #4163, #4233, #4291, #4415, #4047, #4068.
- **Auto-passed (42)**: drafts, kylehgc, KuSh-authored (maintainer work arrives via sync).

