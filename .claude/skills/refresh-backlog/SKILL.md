---
name: refresh-backlog
description: >
  Full backlog refresh: run refresh-issues (re-verify open fork tickets, close stale ones),
  then sweep-upstream (crawl new upstream PRs, file candidate tickets). One command to bring
  the fork's issue list fully up to date. Args passed through: "dry" for sweep-upstream,
  issue numbers for refresh-issues.
allowed-tools:
  - Bash
  - Read
  - Grep
  - Glob
  - Agent
  - Write
  - Skill
effort: medium
tags: [issues, backlog, triage, upstream, compound]
---

# Refresh Backlog

Compound skill — no logic of its own:

1. **`refresh-issues`** first: re-verify open fork tickets, close stale ones. Running this
   first means the sweep dedups against a *clean* backlog, not one full of tickets about to
   close. Its precondition sync also covers step 2 — don't sync twice.
2. **`sweep-upstream`** second: crawl upstream PRs since the last sweep watermark, file
   tickets for verified new candidates.

Invoke each via the Skill tool, in order, passing through any matching args (issue numbers →
refresh-issues, `dry`/`since:` → sweep-upstream). Each skill's own validation gates
(AskUserQuestion before closing or filing anything) still apply — do not batch the two
skills' confirmations into one.

Finish with a combined summary:

```
Backlog refresh: <K> tickets kept · <C> closed · <N> upstream PRs swept · <F> tickets filed
Open backlog is now: #<n1> #<n2> ... (ranked)
```

The final ranked list is what `do-issue` (no-arg) picks from — order it by do-issue's
Phase 1 ranking.
