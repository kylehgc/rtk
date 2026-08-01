#!/usr/bin/env bash
#
# Run the fork's portable integration tests against a checkout of upstream/develop
# and render the result into the fork landing page.
#
# Green here and red there is the proof. A test that starts passing upstream means
# upstream merged the fix and the fork can drop that divergence — this script says
# so loudly rather than quietly continuing to claim credit for it.
#
# Enforces "no claim without a test": every row in .github/fork-claims.tsv must have
# a test that passes here and fails upstream, or this script exits non-zero.
#
# Linux only — some of these tests use std::os::unix.
#
# Usage: scripts/fork-proof.sh [--no-page]
set -euo pipefail

UPSTREAM_REF="${UPSTREAM_REF:-upstream/develop}"
PAGE="${PAGE:-.github/README.md}"
CLAIMS="${CLAIMS:-.github/fork-claims.tsv}"
WORKTREE="${WORKTREE:-$(mktemp -d -t rtk-upstream-XXXXXX)}"

# Tool-availability guards must fail, never skip, inside a proof run: a test
# that skips still prints "ok", and this script would record its claim as
# proven when nothing ran. Enforced here rather than only in fork-proof.yml so
# a local run of this script gets the same rigor.
export RTK_PROOF_REQUIRE_RG=1
START="<!-- FORK_PROOF_START -->"
END="<!-- FORK_PROOF_END -->"

cleanup() {
  git worktree remove --force "$WORKTREE" 2>/dev/null || rm -rf "$WORKTREE"
}
trap cleanup EXIT

git rev-parse --verify --quiet "$UPSTREAM_REF" >/dev/null \
  || { echo "error: $UPSTREAM_REF not found — fetch the upstream remote" >&2; exit 1; }
[ -f "$CLAIMS" ] || { echo "error: $CLAIMS not found" >&2; exit 1; }

# Test files the fork has diverged on. These are the only proof candidates: a test
# upstream already has proves nothing about the difference between us.
mapfile -t DIVERGED < <(git diff --name-only "$UPSTREAM_REF" HEAD -- tests/ | grep -E '^tests/.*\.rs$' || true)
[ ${#DIVERGED[@]} -gt 0 ] || { echo "error: no diverged test files found" >&2; exit 1; }

echo "Fork-diverged test files:"
printf '  %s\n' "${DIVERGED[@]}"

# `test <name> ... ok|FAILED` -> "<name> ok|FAILED", one per line.
run_targets() {
  local dir="$1" target
  for path in "${DIVERGED[@]}"; do
    target="$(basename "$path" .rs)"
    ( cd "$dir" && cargo test --test "$target" -- --test-threads=1 2>/dev/null || true ) \
      | sed -n 's/^test \([a-z_0-9]*\) \.\.\. \(ok\|FAILED\)$/\1 \2/p'
  done
}

echo "==> running proof tests on the fork"
FORK_RESULTS="$(run_targets .)"

echo "==> preparing upstream worktree at $WORKTREE"
rm -rf "$WORKTREE"
git worktree add --detach "$WORKTREE" "$UPSTREAM_REF" >/dev/null
# Lift the fork's test files (and any fixtures they need) into upstream's tree.
for path in $(git diff --name-only "$UPSTREAM_REF" HEAD -- tests/); do
  [ -f "$path" ] || continue   # deleted on our side; nothing to lift
  mkdir -p "$WORKTREE/$(dirname "$path")"
  cp "$path" "$WORKTREE/$path"
done

echo "==> running the same tests against $UPSTREAM_REF"
UPSTREAM_RESULTS="$(run_targets "$WORKTREE")"

# First match wins: a test name appears once per target, but never assume it.
result_of() {
  local r
  r="$(printf '%s\n' "$2" | awk -v t="$1" '$1 == t { print $2; exit }')"
  printf '%s\n' "${r:-MISSING}"
}

rows=""
proven=0
problems=()
while IFS=$'\t' read -r test class claim; do
  case "$test" in ''|\#*) continue ;; esac
  fork="$(result_of "$test" "$FORK_RESULTS")"
  up="$(result_of "$test" "$UPSTREAM_RESULTS")"

  if [ "$fork" != "ok" ]; then
    problems+=("$test: claimed, but does not pass on the fork (got: $fork)")
    continue
  fi
  if [ "$up" = "ok" ]; then
    problems+=("$test: PASSES ON UPSTREAM — upstream has fixed this. Drop the claim and the fork divergence.")
    continue
  fi
  if [ "$up" = "MISSING" ]; then
    problems+=("$test: did not run against upstream (did it fail to compile there?)")
    continue
  fi

  rows+="| ${claim} | \`${test}\` | ❌ fails | ✅ passes |"$'\n'
  proven=$((proven + 1))
done < "$CLAIMS"

if [ ${#problems[@]} -gt 0 ]; then
  echo
  echo "✗ Claim/proof mismatch:" >&2
  printf '  - %s\n' "${problems[@]}" >&2
  exit 1
fi

fidelity=$(grep -cE $'\tfidelity\t' "$CLAIMS" || true)
reduction=$(grep -cE $'\treduction\t' "$CLAIMS" || true)

block="**${proven} claims proven** by tests that pass on this fork and fail against
\`${UPSTREAM_REF}\` — ${fidelity} fidelity, ${reduction} reduction. Run them yourself with
\`scripts/fork-proof.sh\`.

| Claim | Test | Upstream | This fork |
|---|---|---|---|
${rows}"

echo
echo "✓ ${proven} claims proven (${fidelity} fidelity, ${reduction} reduction)"

if [ "${1:-}" = "--no-page" ]; then
  printf '%s\n' "$block"
  exit 0
fi

awk -v start="$START" -v end="$END" -v block="$block" '
  $0 == start { print; print block; skip = 1; next }
  $0 == end   { skip = 0 }
  !skip       { print }
' "$PAGE" > "$PAGE.tmp" && mv "$PAGE.tmp" "$PAGE"
echo "✓ Proof block updated in $PAGE"
