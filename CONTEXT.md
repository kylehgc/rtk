# rtk Fork Maintenance

This fork (kylehgc/rtk) tracks upstream rtk-ai/rtk and adopts solid community fixes that upstream is too slow to merge, plus original fixes. This glossary covers the fork-maintenance process.

The `/do-issue` skill (`.claude/skills/do-issue/`) is this document's executable form: it drives one open fork issue from ticket to merged PR along the flow defined below. This file stays the authority — where the two disagree, fix the skill.

## Language

**Upstream**:
The original rtk-ai/rtk repository. The fork merges its `develop` in periodically; it never receives pushes from us except via PRs.

**Origin**:
This fork, `kylehgc/rtk`. Unqualified "open issues" / "open PRs" — "look at open issues", "what's still open" — always mean origin's: adoption tickets and fork PRs. Upstream's are named explicitly ("upstream issues", "upstream PRs", "rtk-ai/rtk"). Triage is the standing exception: it crawls upstream's PRs by definition.

`gh repo set-default` is pinned to `kylehgc/rtk` so bare `gh issue list` / `gh pr list` resolve here (`remote.origin.gh-resolved=base`). It previously pointed at `upstream`, which is how "look at open issues" surfaced upstream's backlog and #3184 became work by accident. If a clone or a re-run of `gh repo set-default` ever flips it back, `gh repo set-default --view` should read `kylehgc/rtk`.

An upstream issue is never work on sight. Reading one is not claiming it: it becomes work only once it has an origin ticket, and that decision is the maintainer's, not a byproduct of a listing.

**Sync**:
A `git merge upstream/develop` into the fork's mainline. Never a rebase — published history is append-only. Always Sync before starting any new work: fetch upstream, and if `upstream/develop` has commits the fork lacks, merge them into `develop` (quality gate included) first, so every Adoption or Original fix branches from a synced `develop`. A clean merge commits to `develop` directly. A **Conflicted Sync** — any merge requiring manual conflict resolution — goes through a topic branch and fork PR instead (see Conflicted Sync).
_Avoid_: rebase, update

**Conflicted Sync**:
A Sync whose merge hits conflicts, meaning fork work and upstream work collided — usually because upstream shipped its own solution to a problem the fork had already patched. Resolution happens on a topic branch and lands via a fork PR, never as a direct commit to `develop`. The PR description must record: which fork code was removed or superseded, why the upstream version won (or why the fork's was kept), and any fork tests updated to match new upstream behavior. This keeps the removal of contributed work reviewable and attributable — contributor-friendly for the fork, and an honest record of where upstream superseded us. First instance: merge `e32bb38` (2026-07-23, resolved pre-policy, direct to `develop`) — see `claudedocs/sync-conflict-2026-07-23.md`.

**Candidate PR**:
An open upstream PR that triage has flagged as potentially worth adopting, but not yet vetted.

**Adopted PR**:
A community PR whose commits have been cherry-picked into the fork, preserving original authorship, and verified against the quality gate.
_Avoid_: ported, re-implemented, merged (reserve "merged" for upstream's own actions)

**Adoption**:
The process of taking a Candidate PR into the fork: cherry-pick its commits, verify the bug is actually fixed (repro-before/fixed-after), run the quality gate. Cherry-pick is the default; re-implementation is a fallback only when cherry-pick conflicts or the fix is right but the code is poor. All adoption work happens on a topic branch and lands in `develop` via a fork PR — never as direct commits to `develop` (only Syncs touch `develop` directly). All fork PRs land as merge commits (`--merge`) — never squash or rebase merges, which rewrite cherry-picked commits as fork-authored and destroy the attribution Adoption exists to preserve.

**Amendment**:
A fork-authored commit added on top of a cherry-picked contribution to complete it — missing tests, small fixes. Always a separate commit; never squashed into the contributor's commit. A PR needing more than amendments is not adopted — it's re-implemented, rejected, or queued for Contributor outreach.

**Triage**:
Crawling upstream's open PRs to classify each as a Candidate or a pass, bug fixes ranked ahead of features. Before a Candidate is filed as an adoption ticket, verify the bug is real and still present on current develop (code inspection or repro) — an open upstream PR is not evidence its bug is unfixed; upstream may have merged a different fix (see triage 2026-07-17 verification sweep: #2263, #937).

**Original fix**:
A bug fix or change authored in the fork itself, not adopted from an upstream PR. Follows the same ticket → PR → verification workflow as an Adoption, minus the cherry-pick. It is **not done when the fork PR merges**: an Adoption is self-healing upstream — the contributor's PR is already open there — but an Original fix exists nowhere else and is stranded until submitted. So if it closes an upstream issue, cherry-pick the commits onto a branch off `upstream/develop`, run the quality gate on *that* branch, and open the Upstream PR before closing the ticket. First instance: `mvnd`/upstream #3184, merged into the fork 2026-07-24 and caught by the issue reporter three hours later — see upstream PR #3199.

**Upstream PR**:
A PR from the fork to `rtk-ai/rtk`, the only way fork work reaches upstream users. Never open one from `develop` — the fork's mainline carries dozens of unrelated commits. Always a topic branch off `upstream/develop` carrying only the commits for that one change, targeting upstream's `develop` (never `master`). Upstream requires a CLA signature on first contribution.

**Contributor outreach**:
Engaging a community PR author directly — commenting upstream or inviting a PR against the fork — instead of silently adopting or fixing their work. When a contributor opens a PR against the fork, it goes through Fork PR intake.

## Fork PR intake

How to handle a PR opened against the fork by someone other than the maintainer (first case: PR #43 by kingpy-bot, author of upstream #2951). Four gates, in order:

1. **Scope**: The PR must be an Adoption of an upstream PR, ideally tied to an open fork adoption issue. Original work from non-contributors is redirected upstream with a comment and closed — the fork tracks upstream; new features belong there first.
2. **Verification**: Same bar as a self-made Adoption, zero trust in the author's claims. The diff must match the upstream PR (byte-identical, or every deviation explained), authorship preserved as a cherry-pick, and the maintainer runs the repro check and quality gate locally. Fork CI does not run, so the local gate is the merge gate — a green-looking PR proves nothing.
3. **Safety**: Building or testing an external branch executes its code (build.rs, proc-macro deps, test bodies). Read the full diff before running cargo on the branch. Any change to `Cargo.toml`, `build.rs`, or dependencies is a hard stop for manual scrutiny; pure `src/**/*.rs` + `tests/**` diffs are low risk once read.
4. **Fixes**: Follow the Amendment rule — small gaps get fork-authored commits on top, never revision ping-pong with a drive-by contributor. Needs more than amendments → close with credit and adopt via the normal cherry-pick route (authorship survives either way).
