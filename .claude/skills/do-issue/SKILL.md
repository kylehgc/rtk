---
name: do-issue
description: >
  Take one open fork issue from ticket to merged PR, following the fork-maintenance
  workflow in CONTEXT.md: Sync, classify Adoption vs Original fix, investigate deeply,
  cherry-pick preserving authorship, repro before/after, quality gate, fork PR, merge.
  Enforces the Upstream PR obligation for Original fixes.
  Args: an issue number (e.g. "45"), or no arg to pick from the open backlog.
allowed-tools:
  - Bash
  - Read
  - Write
  - Edit
  - Grep
  - Glob
  - Skill
  - AskUserQuestion
effort: high
tags: [workflow, fork, adoption, cherry-pick, upstream, pr, rtk]
---

# /do-issue

Drive one open issue on **this fork** (`kylehgc/rtk`) from ticket to merged PR.

The fork's workflow is not upstream's. Upstream develops; this fork *adopts* — cherry-picking
community PRs upstream is too slow to merge, plus the occasional original fix. Attribution and
the local quality gate are the two things that matter most, and they are exactly what the
inherited upstream skills get wrong (see **Traps** at the bottom).

[CONTEXT.md](../../../CONTEXT.md) is the authority. This skill is its executable form. Where
they disagree, CONTEXT.md wins — and fix this file.

---

## Phase 0 — Preconditions

```bash
pwd                                  # must be the rtk project root
git branch --show-current            # note it; you will branch off develop
git status --short                   # must be clean before starting
gh auth status                       # must be the kylehgc account
```

The active `gh` account may be `frogicporn`, which has **no push rights** here. If so:

```bash
gh auth switch --user kylehgc
```

Pass `--repo kylehgc/rtk` explicitly on `gh` calls. `gh repo set-default` is pinned to the fork,
but it has silently flipped to upstream before — which is how upstream's backlog once got
mistaken for the fork's.

### Sync first — always

Never start work on a stale `develop`. From CONTEXT.md: *"Always Sync before starting any new
work."*

```bash
git fetch upstream
git rev-list --count develop..upstream/develop     # 0 => already synced, skip ahead
git checkout develop
git merge upstream/develop --no-edit
```

- **Clean merge** → commits to `develop` directly. Run the quality gate on it (Phase 4) before
  branching; kick it off in the background while you read the ticket.
- **Conflicted merge** → this is a **Conflicted Sync**. Stop. It goes through a topic branch and
  its own fork PR, never a direct commit to `develop`. The PR must record which fork code was
  removed or superseded, why the upstream version won, and any fork tests updated. See
  `claudedocs/sync-conflict-2026-07-23.md` for the first case.

---

## Phase 1 — Read the ticket, classify it

```bash
gh issue view <N> --repo kylehgc/rtk
```

Two kinds of work, different endings:

| | **Adoption** | **Original fix** |
|---|---|---|
| Source | An upstream PR exists | Authored here |
| Core step | Cherry-pick, authorship preserved | Write it |
| Done when | Fork PR merges | **Upstream PR is open** (Phase 7) |

An Adoption is self-healing upstream — the contributor's PR is already open there. An Original
fix exists nowhere else and is stranded until submitted. Getting this wrong means shipping a fix
only this fork will ever have.

If the ticket says "Adopt upstream #NNNN", it is an Adoption *unless* Phase 2 shows the upstream
PR no longer applies. If Phase 2 finds an upstream PR for something you assumed was original,
reclassify to Adoption and say so.

---

## Phase 2 — Investigate before touching code

Do not skip to the diff. The two failure modes are patching a symptom, and duplicating work that
already exists somewhere.

### 2a. The codebase

- Read the functions the ticket names, then **trace every caller** — `Grep` the symbol, don't
  trust the ticket's file list.
- Fix the root cause, not the path the ticket happened to walk. One guard in a shared function
  beats a guard in each caller, and patching only the reported path leaves the siblings broken.
- Check sibling call sites deliberately: if a check is being tightened, decide for *each* caller
  whether it wants the strict or the loose version. Say which in the PR.
- Verify the bug is still live on current `develop` by inspection or repro. Per CONTEXT.md, an
  open upstream PR is **not** evidence its bug is unfixed — upstream may have merged a different
  fix (triage 2026-07-17: #2263, #937).

### 2b. Open issues and PRs — both repos

⚠️ **The `issue-triage` / `pr-triage` / `rtk-triage` skills sweep the *current* repo only.** They
run bare `gh issue list` / `gh pr list` with no repo override, and the default is pinned to the
fork. They are useful for the fork's own backlog and **cannot** sweep upstream. For upstream,
query explicitly:

```bash
gh search prs   --repo rtk-ai/rtk --state open "<keywords>" --limit 30
gh search issues --repo rtk-ai/rtk --state open "<keywords>" --limit 30
gh pr view <N> --repo rtk-ai/rtk
```

Upstream carries thousands of open PRs and keyword search misses anything phrased differently.
Try several phrasings, including the *symptom* rather than the cause — the bug may already be
reported under a description that never names the mechanism. Treat a negative result as
"probably none", never as proof, and say so in the PR.

Also look for a **cluster**: related upstream PRs that interact. Adopting one blind can conflict
with another, or ship a partial fix whose sibling PR is the half that matters. If one exists,
decide explicitly whether to adopt together or land one and document the residual gap.

---

## Phase 3 — Implement on a topic branch

All work happens on a topic branch. **Only Syncs touch `develop` directly.**

```bash
git checkout develop
git checkout -b <type>/<short-slug>
```

### Adoption path

```bash
git fetch upstream pull/<PR>/head:pr-<PR>
git log --oneline upstream/develop..pr-<PR>
git show <sha>                        # READ THE FULL DIFF FIRST
```

**Safety gate (CONTEXT.md, Fork PR intake §3):** building or testing an external branch executes
its code — `build.rs`, proc-macro deps, test bodies. Read the whole diff *before* running cargo
on it. Any change to `Cargo.toml`, `build.rs`, or dependencies is a hard stop for manual
scrutiny. Pure `src/**/*.rs` + `tests/**` diffs are low risk once read.

```bash
git cherry-pick <sha>                 # authorship preserved — verify it
git log --format='%h %an <%ae>' -2
```

Cherry-pick is the default. Re-implement only if it conflicts badly or the fix is right but the
code is poor — and say which in the PR.

**Amendments** — anything the contributor's commit is missing (tests, a gap it doesn't cover) is
a **separate fork-authored commit** on top. Never squash it into theirs. A PR needing more than
amendments is not adopted: re-implement, reject, or open contributor outreach.

### Original fix path

Write it. Same branch discipline, no cherry-pick. This is novel code nothing upstream vouches
for, so route it through the repo's own skills — see **Skills** below.

---

## Phase 4 — Verify

### Repro before/after with a real binary

Unit tests alone are not verification. The PR must show the bug happening and then not happening.
Capture both, and put the real output in the PR body.

For hook / `settings.json` work, sandbox with `CLAUDE_CONFIG_DIR` so you never touch the real
`~/.claude`:

```powershell
$sandbox = Join-Path $env:TEMP "rtk-repro"; New-Item -ItemType Directory -Force $sandbox | Out-Null
$env:CLAUDE_CONFIG_DIR = $sandbox
```

Two Windows traps that make a working fix look broken:

- **Writing fixtures**: PowerShell 5.1's `Out-File -Encoding utf8` and `>` emit a **BOM**, and
  serde_json rejects it — you get `expected value at line 1 column 1` and blame the code. Use
  `[System.IO.File]::WriteAllText($p, $json, (New-Object System.Text.UTF8Encoding($false)))`.
- **Piping JSON to a native exe** from PS 5.1 mangles the encoding. Pipe stdin from Bash instead.

### Quality gate — the merge gate

**Fork CI never runs.** GitHub Actions produces zero runs on this fork despite active workflows,
so a green-looking PR proves nothing. The local gate is the only gate. Don't wait on
`gh pr checks`.

```bash
cargo fmt --all && cargo clippy --all-targets && cargo test --all
```

Zero clippy warnings, zero failures. On this Windows ARM64 machine `cargo` is not on PATH and the
native target cannot link — build with the x86_64 **host** toolchain, from PowerShell (not Git
Bash, where coreutils `link` shadows MSVC's):

```powershell
$env:RUSTUP_TOOLCHAIN  = "stable-x86_64-pc-windows-msvc"
$env:CARGO_BUILD_TARGET = "x86_64-pc-windows-msvc"
$msvc = "C:\Program Files (x86)\Microsoft Visual Studio\18\BuildTools\VC\Tools\MSVC\14.50.35717"
$sdk  = "C:\Program Files (x86)\Windows Kits\10"; $sdkv = "10.0.26100.0"
# Git usr\bin appended LAST: core::stream tests spawn sh/cat/true, but it must not
# shadow System32 find/sort.
$env:PATH = "$env:USERPROFILE\.cargo\bin;$msvc\bin\Hostarm64\x64;$env:PATH;C:\Program Files\Git\usr\bin"
$env:LIB  = "$msvc\lib\x64;$sdk\Lib\$sdkv\um\x64;$sdk\Lib\$sdkv\ucrt\x64"
```

A failing test is a finding, not an obstacle. Read it before changing it: if a pre-existing
assertion now fails, decide whether the code is wrong or the assertion encoded an assumption the
fix legitimately changes — then update the assertion to encode the *intent*, not the new number.

---

## Phase 5 — Fork PR

Never merge a topic branch into `develop` locally. It goes through a PR.

```bash
git push -u origin <branch>
gh pr create --repo kylehgc/rtk --base develop --title "<conventional commit title>" --body-file <file>
```

The body carries the evidence. Include:

- Which upstream PR is adopted and its author, or that this is an Original fix.
- A commit table: which commits are cherry-picked (and from whom) vs fork-authored amendments.
- Whether the cherry-pick is byte-identical to upstream, or every deviation explained.
- **Repro before/after with real output.**
- Any fork test updated, and why the old assertion was wrong.
- Quality gate result.
- What you deliberately did *not* fix, and why.

---

## Phase 6 — Review, then merge

Run a review before merging. `/code-review` gives Standards + Spec axes in parallel; the repo's
`security-guardian` is worth adding for anything touching hooks, shell escaping, or untrusted
input parsing.

Fix what review finds, re-run the gate, and record the round as a PR comment so the reasoning
survives the merge.

```bash
gh pr merge <N> --repo kylehgc/rtk --merge --delete-branch
```

🚨 **`--merge`. Never `--squash`.** Squashing collapses the cherry-pick into one fork-authored
commit and **destroys the contributor's attribution** — the single thing the Adoption flow exists
to preserve. Published history is append-only here: no rebase, no force push, ever.

Verify after merging:

```bash
git checkout develop && git pull origin develop
git log --format='%h %an' -4          # contributor's name must still be there
```

---

## Phase 7 — Original fix: the part that is easy to forget

An Original fix **is not done when the fork PR merges.** From CONTEXT.md:

> an Adoption is self-healing upstream — the contributor's PR is already open there — but an
> Original fix exists nowhere else and is stranded until submitted.

This was learned the hard way: the `mvnd` fix merged into the fork 2026-07-24 and the issue
reporter noticed three hours later that upstream still had nothing.

```bash
git fetch upstream
git checkout -b upstream/<slug> upstream/develop
git cherry-pick <sha>...              # ONLY the commits for this one change
# quality gate on THIS branch too
git push -u origin upstream/<slug>
gh pr create --repo rtk-ai/rtk --base develop --title "..." --body-file <file>
```

- Never open an Upstream PR from `develop` — the fork's mainline carries dozens of unrelated
  commits.
- Target upstream's `develop`, never `master`.
- Upstream requires a CLA signature on first contribution.
- **Only then close the fork ticket.**

---

## Skills to use

For novel fork-authored code — Original fixes and Amendments — nothing upstream vouches for the
style, so lean on the repo's own skills:

| Skill | Use for |
|---|---|
| `rtk-tdd` | Red-Green-Refactor. Write the failing test first; if it passes immediately it tests nothing. |
| `security-guardian` | Anything touching hooks, shell escaping, command execution, or parsing untrusted output. |
| `code-simplifier` | Before opening the PR. Knows what *not* to touch (`lazy_static!`, `.context()`, fallback, exit codes). |
| `tdd-rust` | Filter work specifically — real fixtures, ≥60% savings assertions. |
| `design-patterns` | When a new helper's home or shape is non-obvious. |

Binding regardless: `.claude/rules/rust-patterns.md` and `.claude/rules/cli-testing.md`.

---

## Traps

Inherited skills written for upstream maintainers, and what they do here:

| Skill | Trap |
|---|---|
| `pr-review` | Merge step is `gh pr merge --merge --squash` — conflicting flags, and squash **destroys cherry-pick authorship**. It also hardcodes `gh api repos/rtk-ai/rtk/...`, so it reads upstream's reviews while `gh pr list` reads the fork. Do not use it for fork PRs. |
| `pr-triage` | Phase 2 spawns `subagent_type: code-reviewer` and references a `backend-architect` skill. **Neither exists in this repo.** That phase fails. |
| `issue-triage`, `pr-triage`, `rtk-triage` | Sweep the *current* repo only. Fine for the fork's backlog; they cannot crawl upstream, which is what the fork's Triage means. |
| `ship` | Pushes to `origin main`. This fork's mainline is `develop`; there is no `main`. Also assumes upstream's release-please and working CI. |
| `ship`, `performance` | Use `/usr/bin/time -l` (macOS-only) and assume a plain `cargo` on PATH. Neither holds on this machine. |

And the standing ones: never `--squash`, never rebase published history, never force push, never
commit directly to `develop` except a clean Sync, never trust fork CI.
