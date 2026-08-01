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

if ! git rev-parse --verify --quiet "$UPSTREAM_REF" >/dev/null; then
  echo "error: $UPSTREAM_REF not found. Add the upstream remote and fetch:" >&2
  echo "  git remote add upstream https://github.com/rtk-ai/rtk.git && git fetch upstream" >&2
  exit 1
fi

[ -f "$PAGE" ] || { echo "error: $PAGE not found" >&2; exit 1; }

rows=""
count=0
while IFS=$'\t' read -r sha subject; do
  [ -n "$sha" ] || continue
  rows+="| ${subject} | [\`${sha}\`](${REPO_URL}/commit/${sha}) |"$'\n'
  count=$((count + 1))
done < <(
  git log "$UPSTREAM_REF..HEAD" --no-merges --format='%h%x09%s' \
    | grep -E "	(feat|fix)" \
    | grep -vE "	(feat|fix)\((${EXCLUDE_SCOPES})\)"
)

if [ "$count" -eq 0 ]; then
  block="_Upstream has merged everything this fork carries. Nothing to see here — use upstream._"
else
  block="**${count} fixes in this fork that upstream does not have.** Each links to the commit,
where the original author is recorded. Adopted fixes come from community PRs that upstream
has not merged — see the [adoption issues](${REPO_URL}/issues?q=is%3Aissue+Adopt+upstream)
for provenance.

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
