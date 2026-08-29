---
name: preclear
description: >
  Pre-clear an upstream-bound branch by simulating rtk-ai/rtk's actual review:
  an uncontaminated run of upstream's code-reviewer prompt plus the empirical
  verification techniques the human maintainer (KuSh) uses on top of it —
  fixture A/B against the merge base, prior-probe re-runs, bisect attribution.
  Loop fix rounds until a cold pass approves. Use before pushing any branch
  that upstream will review, and after every CHANGES_REQUESTED round.
  Args: "<worktree-path> <merge-base-sha>" — the branch checkout to review and
  the upstream base commit to diff/A/B against. Optional "rounds:<N>" to cap
  the loop (default: until approve).
---

# /preclear — pass upstream's review before it happens

Upstream reviews every PR with an LLM-assisted flow driven by prompts that
ship **in this repo**. Running the same review locally, cold, catches the
findings before they cost a round-trip. Validated on rtk-ai/rtk#3199: the
loop found 4 real silent-loss blockers the upstream round had missed, and
the reviewer's next pass verified all prior findings fixed.

## Know the reviewer

- **The accounts (`arkgum`, `KuSh`) run upstream's own prompts.** `KuSh` is
  Nicolas Le Cam, a human maintainer (author of upstream's
  `.claude/rules/cli-testing.md` rewrite) — expect empirical verification,
  honest self-corrections, and zero tolerance for unverified claims.
- **The engine**: `.claude/agents/code-reviewer.md` — mandatory Call-Site
  Analysis ("list every distinct input shape, verify a test exists for EACH;
  missing = Critical"), the adversarial-questions list, savings floors.
- **The grading**: `.claude/skills/pr-triage/templates/review-comment.md` —
  🔴 includes "test missing for new feature"; 🟡 explicitly includes
  "performance regression, **scope creep**, missing token savings assertion".
  A new dependency riding along in a feature PR is graded off that list.

## The pass

Spawn a `general-purpose` agent (code-reviewer is NOT a registered
subagent_type — read the file, run its body as the prompt). Give it ONLY:

1. The full body of `.claude/agents/code-reviewer.md` to adopt as persona,
   process, severity grading, and output format.
2. The review target: `git diff <merge-base>` against the working tree in
   the worktree.
3. Repo access, read-only. Probe code goes in the scratchpad, never the
   repo; the repo must be byte-identical after the pass.

**Uncontaminated means uncontaminated**: no prior-round history, no
"deferred per author's note" framing, no list of what was already fixed.
A primed reviewer ratifies the deferral it was handed; upstream reading
cold will not.

## Techniques the prompt doesn't say (the maintainer uses them — so do we)

- **Fixture A/B against the merge base.** For every filter entry point the
  diff touches, run ALL existing repo fixtures through base and head and
  diff bytes + savings. Shape enumeration alone missed a plain-`mvn`
  regression on #3199 that base-vs-head fixture bytes caught immediately.
- **Re-run every prior-round probe** against the new head before trusting
  a fix. The reviewer does exactly this on their next pass.
- **Bisect regressions.** When A/B finds one, `git bisect` to the commit
  that introduced it — usually an earlier fix round that over-corrected.
  Fix at that root, not at the symptom.
- **Probe with realistic input, and audit test inputs for realism.** A
  test whose input can't occur in reality validates the wrong code path:
  on #3199 the summary-cap tests omitted the `Running` lines real mvnd
  always emits, so both "modules" collapsed onto the root lane and the
  tests kept passing while the cap was inert on every real reactor. When
  a fix changes routing/classification, re-ask of every green test:
  "does this input still exercise the path it claims to?" Prefer probe
  inputs modeled on the real fixtures over minimal synthetic shapes.

## Fix rounds

One finding set per round, then gate (`cargo fmt --all && cargo clippy
--all-targets && cargo test --all`, zero warnings), then a fresh cold pass.

- **Prove every fix load-bearing**: revert it, watch its new test fail,
  restore. A fix whose test passes either way tests nothing.
- **State each fixed behavior as one literal invariant** in the doc comment
  — the reviewer's shape enumeration terminates on total contracts, never
  on examples. An "e.g." in an invariant doc is where the next blocker
  hides: enumerate, don't illustrate.
- **Never document a deferral.** "Known limit" comments get promoted to
  next-round blockers. Cheap to fix → fix it; genuinely out of scope →
  keep it out of the doc comments and defend it in the PR conversation.
- **No scope-creep riders**: no new dependencies (dev-deps included), no
  drive-by refactors. If it isn't the PR's stated purpose or a review
  response, it's a separate PR.
- **Treat any concretely traceable silent diagnostic/data loss as 🔴**,
  regardless of claimed rarity — that is the reviewer's standard.

## Convergence

Findings shrink round over round when the fixes close the state space;
they recur forever when fixes add routing state. Converge by making every
classifier's contract exhaustive (each input shape has a stated, tested
arm) plus order-preserving sweep tests over the interleavings. Stop when a
cold pass returns approve-level with nothing you'd be ashamed to ship;
then hold everything uncommitted for the human's go-ahead.
