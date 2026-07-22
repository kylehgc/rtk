# rtk Fork Maintenance

This fork (kylehgc/rtk) tracks upstream rtk-ai/rtk and adopts solid community fixes that upstream is too slow to merge, plus original fixes. This glossary covers the fork-maintenance process.

## Language

**Upstream**:
The original rtk-ai/rtk repository. The fork merges its `develop` in periodically; it never receives pushes from us except via PRs.

**Sync**:
A `git merge upstream/develop` into the fork's mainline. Never a rebase — published history is append-only. Always Sync before starting any new work: fetch upstream, and if `upstream/develop` has commits the fork lacks, merge them into `develop` (quality gate included) first, so every Adoption or Original fix branches from a synced `develop`.
_Avoid_: rebase, update

**Candidate PR**:
An open upstream PR that triage has flagged as potentially worth adopting, but not yet vetted.

**Adopted PR**:
A community PR whose commits have been cherry-picked into the fork, preserving original authorship, and verified against the quality gate.
_Avoid_: ported, re-implemented, merged (reserve "merged" for upstream's own actions)

**Adoption**:
The process of taking a Candidate PR into the fork: cherry-pick its commits, verify the bug is actually fixed (repro-before/fixed-after), run the quality gate. Cherry-pick is the default; re-implementation is a fallback only when cherry-pick conflicts or the fix is right but the code is poor. All adoption work happens on a topic branch and lands in `develop` via a fork PR — never as direct commits to `develop` (only Syncs touch `develop` directly).

**Amendment**:
A fork-authored commit added on top of a cherry-picked contribution to complete it — missing tests, small fixes. Always a separate commit; never squashed into the contributor's commit. A PR needing more than amendments is not adopted — it's re-implemented, rejected, or queued for Contributor outreach.

**Triage**:
Crawling upstream's open PRs to classify each as a Candidate or a pass, bug fixes ranked ahead of features. Before a Candidate is filed as an adoption ticket, verify the bug is real and still present on current develop (code inspection or repro) — an open upstream PR is not evidence its bug is unfixed; upstream may have merged a different fix (see triage 2026-07-17 verification sweep: #2263, #937).

**Original fix**:
A bug fix or change authored in the fork itself, not adopted from an upstream PR. Follows the same ticket → PR → verification workflow as an Adoption, minus the cherry-pick.

**Contributor outreach**:
Engaging a community PR author directly — commenting upstream or inviting a PR against the fork — instead of silently adopting or fixing their work. When a contributor opens a PR against the fork, it goes through Fork PR intake.

## Fork PR intake

How to handle a PR opened against the fork by someone other than the maintainer (first case: PR #43 by kingpy-bot, author of upstream #2951). Four gates, in order:

1. **Scope**: The PR must be an Adoption of an upstream PR, ideally tied to an open fork adoption issue. Original work from non-contributors is redirected upstream with a comment and closed — the fork tracks upstream; new features belong there first.
2. **Verification**: Same bar as a self-made Adoption, zero trust in the author's claims. The diff must match the upstream PR (byte-identical, or every deviation explained), authorship preserved as a cherry-pick, and the maintainer runs the repro check and quality gate locally. Fork CI does not run, so the local gate is the merge gate — a green-looking PR proves nothing.
3. **Safety**: Building or testing an external branch executes its code (build.rs, proc-macro deps, test bodies). Read the full diff before running cargo on the branch. Any change to `Cargo.toml`, `build.rs`, or dependencies is a hard stop for manual scrutiny; pure `src/**/*.rs` + `tests/**` diffs are low risk once read.
4. **Fixes**: Follow the Amendment rule — small gaps get fork-authored commits on top, never revision ping-pong with a drive-by contributor. Needs more than amendments → close with credit and adopt via the normal cherry-pick route (authorship survives either way).
