---
name: refresh-issues
description: >
  Refresh the fork's open issue backlog: re-verify each open adoption ticket against current
  develop, detect tickets superseded by upstream syncs or already-merged fork PRs, and close
  stale ones with evidence. Lighter than a full triage — it re-checks existing tickets, it does
  not crawl upstream for new candidates. Args: issue numbers to focus (e.g. "42 57"), or no arg
  = refresh all open issues.
allowed-tools:
  - Bash
  - Read
  - Grep
  - Glob
  - Agent
effort: medium
tags: [issues, backlog, staleness, adoption, fork]
---

# Refresh Issues

Open fork tickets rot silently: every upstream sync can land a different fix for the same bug,
a fork PR can merge without auto-closing its ticket, or an upstream candidate PR can be closed.
The 2026-07-22 reverify found ~35 stale candidates in one sweep — mostly killed by two upstream
commits nobody re-checked against. This skill is the cheap, repeatable version of that sweep,
scoped to **open fork issues only**.

**Not this skill**: crawling upstream for *new* candidates (that's the full Triage), generic
metadata audit / labeling (that's `issue-triage`), doing the work (that's `do-issue`).

## Preconditions

```bash
gh auth status                      # must be kylehgc; if frogicporn: gh auth switch --user kylehgc
git status --short                  # warn if dirty, but read-only phases can proceed
```

Always pass `--repo kylehgc/rtk` explicitly — the default has silently flipped to upstream
before. **Sync develop first** (fetch + merge upstream/develop per CONTEXT.md): staleness
verdicts are only meaningful against *current* develop. If the merge conflicts, stop —
that's a Conflicted Sync and outranks this refresh.

## Phase 1 — Gather

```bash
gh issue list --repo kylehgc/rtk --state open --limit 50 \
  --json number,title,body,createdAt,updatedAt
gh pr list --repo kylehgc/rtk --state all --limit 100 --json number,title,body,state
```

If issue-number args were given, restrict to those. From each issue body, extract:

- **Upstream PR ref(s)** — `rtk-ai/rtk` PR numbers the adoption ticket is based on.
- **Patch site** — file paths / symbols the ticket names (e.g. `git.rs`, `filter_git_log`).

## Phase 2 — Verify each ticket

Run the checks cheapest-first; the first conclusive one decides the verdict.

1. **Fork PR already handled it?** A merged fork PR referencing the issue → `CLOSE (completed)`.
   An *open* fork PR referencing it → `KEEP (in flight)`, skip remaining checks.
2. **Code already on develop?** This is the decisive check and the biggest staleness source.
   Inspect the patch site on current develop: does the fix (or an equivalent one) already
   exist? Grep for the symbols the upstream PR touches; check file history if ambiguous:
   ```bash
   git log --oneline -5 -- <patch-site-file>
   ```
   Fix present → `CLOSE (superseded upstream — commit <sha>)`.
3. **Patch site gone?** The file/function the ticket targets no longer exists (upstream
   rework) → the candidate can't apply as written. Usually `CLOSE (superseded by rework)`,
   but confirm the *bug* is gone too, not just the file — if the bug survived the rework,
   `KEEP` and note the ticket needs a re-port.
4. **Upstream PR state changed?** `gh pr view <N> --repo rtk-ai/rtk --json state,mergedAt`.
   Merged → its code arrives via sync; recheck step 2 confirmed it. Closed-unmerged → the
   candidate is dead but the bug may live; `KEEP` only if step 2 showed the bug still present.

🚨 **Never use upstream PR state as the verdict by itself.** An open upstream PR is not
evidence its bug is unfixed (see CONTEXT.md Triage: #2263, #937). Step 2's code inspection
on develop is what decides.

For >10 tickets, fan the step-2 inspections out to parallel Explore agents (one per ticket,
return verdict + evidence sha/line); otherwise check inline.

## Phase 3 — Report

One table, every open ticket:

```
| # | Title | Verdict | Evidence |
|---|-------|---------|----------|
| 12 | ... | KEEP | bug still repros: git.rs:455 marker ordering unchanged |
| 3  | ... | CLOSE (superseded) | 952245d split_for_permissions handles \n |
| 7  | ... | KEEP (in flight) | fork PR #68 open |
```

Plus a one-line summary: `N open · K keep · C close candidates · U unsure`.

`UNSURE` verdicts stay open — say what would resolve them (usually a live repro), don't guess.

## Phase 4 — Close stale tickets (validation required)

Show every proposed close with its draft comment, then confirm via `AskUserQuestion`
(multiSelect: all / per-issue / none) before touching anything. Comments in English.

Comment template — record the evidence, mirror the reverify doc style:

```
Superseded on develop: <commit sha / fork PR link> — <one line: what fixed it>.
Verified <how: code inspection at <file:line> / live repro>. Closing.
```

```bash
gh issue comment <N> --repo kylehgc/rtk --body-file - <<'EOF'
<comment>
EOF
gh issue close <N> --repo kylehgc/rtk --reason "not planned"   # superseded/stale
gh issue close <N> --repo kylehgc/rtk --reason "completed"     # fork PR fixed it
```

Never auto-close an `UNSURE`. Never close without the evidence comment.

## Edge cases

| Situation | Behavior |
|-----------|----------|
| Ticket has no upstream PR ref (Original fix ticket) | Steps 2–3 only: is the bug still present on develop? |
| Duplicate tickets found in passing | Note in report; close the newer in favor of the older (with validation) |
| Sync merge conflicts in Preconditions | Stop; Conflicted Sync workflow first |
| 0 open issues | Say so and stop |
