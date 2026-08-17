---
name: sweep-upstream
description: >
  Incremental upstream triage: crawl rtk-ai/rtk PRs opened since the last sweep, classify each
  as CANDIDATE or PASS, verify candidates against current develop, and file fork issues for the
  keepers. The incremental counterpart to the full-backlog triage sweeps in claudedocs/.
  Args: "since:<PR#>" to override the watermark, "dry" to classify without filing issues,
  no arg = auto-detect watermark from the latest sweep doc.
allowed-tools:
  - Bash
  - Read
  - Grep
  - Glob
  - Agent
  - Write
effort: medium
tags: [triage, upstream, adoption, candidates, fork]
---

# Sweep Upstream

Find what's *new*: upstream PRs opened since the last sweep, classified and verified, top
candidates filed as fork adoption tickets. This is the standing exception to "issues mean
origin" — the sweep crawls **upstream (rtk-ai/rtk) by definition**, but files tickets on
**kylehgc/rtk**.

**Not this skill**: re-verifying existing fork tickets (that's `refresh-issues`), doing the
adoption (that's `do-issue`).

## Preconditions

```bash
gh auth status            # must be kylehgc; if frogicporn: gh auth switch --user kylehgc
```

Sync develop first (fetch + merge upstream/develop per CONTEXT.md) — candidate verification
only means anything against current develop. Conflicted merge → stop, Conflicted Sync first.

## Phase 1 — Watermark

The last sweep's coverage is recorded in its own doc. Find it:

```bash
ls claudedocs/ | grep -iE 'sweep|triage|reverify'
```

Read the newest one and take the highest upstream PR number it covered (e.g. the 2026-07-22
reverify covered through `#3168`). That's the watermark; `since:<PR#>` overrides it. No sweep
doc found → ask for a watermark rather than re-crawling 800+ PRs.

## Phase 2 — Crawl

```bash
gh pr list --repo rtk-ai/rtk --state open --limit 200 \
  --json number,title,author,createdAt,isDraft,labels,body \
  --search "sort:created-desc"
```

Keep PRs with `number > watermark`. Paginate if the newest 200 don't reach the watermark.

**Auto-pass on metadata** (record, don't review): drafts, dependabot, pure docs/chore/ci
titles. ⚠️ The 07-22 sweep found 7 missed CANDIDATEs in this bucket — title-only "docs"/"chore"
judgments are unreliable; when the title smells like it touches behavior, review it anyway.

## Phase 3 — Classify

For each remaining PR, verdict `CANDIDATE` / `PASS` / `UNSURE`. Rank per CONTEXT.md Triage and
do-issue's Phase 1 ranking: bug fixes ahead of features, and among fixes:

1. Silent output corruption (a filter dropping/mangling output — invisible at the call site)
2. Crashes / broken commands
3. Windows + Claude Code + js-stack relevance (the fork's user profile)
4. Everything else; off-profile features are a PASS by default

For >15 PRs, fan out to parallel Explore agents (batch ~10 PRs each, return
verdict + one-line reason + files touched); otherwise classify inline from title/body/diff.

Check each CANDIDATE against existing fork tickets (open *and* closed) — already filed or
already declined → note and skip.

## Phase 4 — Verify candidates

🚨 An open upstream PR is not evidence its bug is unfixed (#2263, #937). Before filing, verify
each CANDIDATE against current develop: inspect the exact code the PR patches; grep for an
equivalent fix; live repro only if inspection is inconclusive and the repro is cheap. Bug
already fixed → demote to `PASS (superseded)` with the commit sha.

Duplicate candidates for the same bug → one ticket for the best fix, referencing the rest
(one-PR-per-adoption-with-batching rule).

## Phase 5 — Report and file

Report table, all swept PRs:

```
| PR | Title | Verdict | Reason |
|----|-------|---------|--------|
```

Summary: `N swept (#A–#B) · C candidates · P pass · U unsure · D dup-of-existing`.

Then, unless `dry`: draft a fork issue per surviving CANDIDATE —

```
Title: adopt <area>: <one-line bug> (upstream #<N>)
Body:  upstream PR link · bug summary · verification evidence (what was checked on develop,
       repro if run) · related/duplicate upstream PRs · suggested rank
```

Show all drafts, confirm via `AskUserQuestion` (all / per-ticket / none), then:

```bash
gh issue create --repo kylehgc/rtk --title "<title>" --body-file - <<'EOF'
<body>
EOF
```

## Phase 6 — Record the sweep

Write `claudedocs/upstream-sweep-YYYY-MM-DD.md`: coverage range (`#A–#B`), verdict totals,
the report table, tickets filed. This doc **is the next sweep's watermark** — without it the
next run re-crawls everything.

## Edge cases

| Situation | Behavior |
|-----------|----------|
| Watermark PR now closed/merged | Fine — watermark is a number, not a state |
| 0 new PRs since watermark | Say so and stop — no sweep doc (watermark unchanged) |
| CANDIDATE duplicates an existing open ticket | Skip filing; note in report |
| CANDIDATE matches a *closed* fork ticket | Skip — it was declined or superseded; re-open only with new evidence |
| gh rate limit | Reduce --limit, paginate slower, tell the user |
