#!/usr/bin/env bash
#
# Regenerate the Fork Delta block in the fork landing page.
#
# The Delta is what this fork has that upstream does not, computed from
# upstream/develop..HEAD — never maintained by hand. Entries disappear when
# upstream merges them; that is the fork working as intended, not value lost.
#
# The Delta credits the commit, never the person: adopted authors wrote their
# PRs against upstream and never contributed here. See CONTEXT.md, "Attribution".
#
# Usage: scripts/fork-delta.sh [--check]
#   --check  exit 1 if the page is stale instead of rewriting it
set -euo pipefail

UPSTREAM_REF="${UPSTREAM_REF:-upstream/develop}"
PAGE="${PAGE:-.github/README.md}"
REPO_URL="${REPO_URL:-https://github.com/kylehgc/rtk}"
START="<!-- FORK_DELTA_START -->"
END="<!-- FORK_DELTA_END -->"

# Fork-process scopes. Real commits, but they change how the fork is maintained,
# not what rtk does — a visitor evaluating the binary does not care about them.
# `fork` is here because the landing page must not advertise itself as a fix.
EXCLUDE_SCOPES="skills|review|sync|context|docs|ci|cicd|test|fork"

# Healed divergence (#88): fork commits upstream has since merged or superseded
# under a different SHA. `upstream/develop..HEAD` compares SHAs, so these stay
# in the range forever; listing them as "fixes upstream does not have" is false.
# Two layers: a mechanical patch-id match below catches verbatim round-trips,
# and this audited list catches equivalent-but-different fixes patch-ids can't
# see. Add entries with date + evidence when an audit or a green upstream proof
# test confirms healing.
HEALED_SHAS=(
  673ad19 # 2026-08-05 upstream hook_cmd.rs matches run_in_terminal itself (#88 audit)
  7db14a3 # 2026-08-05 upstream merged its PR #1350, byte-identical both sides (#88 audit)
  e1e37fd # 2026-08-05 superseded by upstream's analyze_pipeline redesign (#88 audit)
  e95207d # 2026-08-05 superseded by upstream's analyze_pipeline redesign (#88 audit)
  bfce39e # 2026-08-12 subsumed by upstream PR #2997 review commit (claudedocs/sync-conflict-2026-08-12.md)
)

# ponytail: patch-id scan bounded to upstream commits since UPSTREAM_SINCE.
# A round-trip whose upstream twin keeps an older committer date (long-lived
# PR branches) is missed here and needs a HEALED_SHAS entry instead.
UPSTREAM_SINCE="${UPSTREAM_SINCE:-2026-04-01}"

if ! git rev-parse --verify --quiet "$UPSTREAM_REF" >/dev/null; then
  echo "error: $UPSTREAM_REF not found. Add the upstream remote and fetch:" >&2
  echo "  git remote add upstream https://github.com/rtk-ai/rtk.git && git fetch upstream" >&2
  exit 1
fi

[ -f "$PAGE" ] || { echo "error: $PAGE not found" >&2; exit 1; }

# Layer 1 — mechanical: fork commits whose exact patch upstream has merged
# under a different SHA. `git cherry` cannot do this here (after every sync
# upstream/develop is an ancestor of HEAD, so its side of the symmetric
# difference is empty); match stable patch-ids against upstream's own history.
declare -A healed_auto
while read -r _pid sha; do
  [ -n "$sha" ] && healed_auto[$sha]=1
done < <(
  awk 'NR==FNR {seen[$1]=1; next} $1 in seen {print}' \
    <(git log --no-merges -p --since="$UPSTREAM_SINCE" "$UPSTREAM_REF" | git patch-id --stable) \
    <(git log --no-merges -p "$UPSTREAM_REF..HEAD" | git patch-id --stable)
)

rows=""
count=0
dropped_auto=0
dropped_audit=0
while IFS=$'\t' read -r full sha subject; do
  [ -n "$sha" ] || continue
  if [ -n "${healed_auto[$full]:-}" ]; then
    dropped_auto=$((dropped_auto + 1))
    continue
  fi
  for h in "${HEALED_SHAS[@]}"; do
    if [[ "$full" == "$h"* ]]; then
      dropped_audit=$((dropped_audit + 1))
      continue 2
    fi
  done
  rows+="| ${subject} | [\`${sha}\`](${REPO_URL}/commit/${sha}) |"$'\n'
  count=$((count + 1))
done < <(
  git log "$UPSTREAM_REF..HEAD" --no-merges --format='%H%x09%h%x09%s' \
    | grep -E "	(feat|fix)" \
    | grep -vE "	(feat|fix)\((${EXCLUDE_SCOPES})\)"
)
echo "  healed divergence excluded: ${dropped_auto} by patch-id, ${dropped_audit} audited" >&2

if [ "$count" -eq 0 ]; then
  block="_Upstream has merged everything this fork carries. Nothing to see here — use upstream._"
else
  block="**${count} fixes in this fork that upstream does not have.** Each links to the commit,
where the original author is recorded. Adopted fixes come from community PRs that upstream
has not merged — see the [adoption issues](${REPO_URL}/issues?q=is%3Aissue+Adopt+upstream)
for provenance.

This count is an upper bound: fixes upstream has since merged verbatim are dropped
automatically by patch-id, and audited equivalents are excluded by hand
(\`scripts/fork-delta.sh\`) — but pending the next audit, an entry may already exist
upstream in another form.

| Fix | Commit |
|---|---|
${rows}"
fi

updated=$(awk -v start="$START" -v end="$END" -v block="$block" '
  $0 == start { print; print block; skip = 1; next }
  $0 == end   { skip = 0 }
  !skip       { print }
' "$PAGE")

if [ "${1:-}" = "--check" ]; then
  if [ "$updated" = "$(cat "$PAGE")" ]; then
    echo "✓ Delta block is current (${count} fixes)"
    exit 0
  fi
  echo "✗ Delta block is stale — run scripts/fork-delta.sh" >&2
  exit 1
fi

printf '%s\n' "$updated" > "$PAGE"
echo "✓ Delta block updated (${count} fixes)"
