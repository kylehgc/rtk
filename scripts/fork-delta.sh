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
# Matched against compound scopes too, but only when EVERY component is a
# process scope: `fix(docs,test)` is excluded, `fix(git,docs)` still counts
# because it also changes behavior. Without the compound form a commit like
# `fix(docs,test): ...` slipped onto the landing page as a fix upstream lacks.
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
  e1e37fd # 2026-08-05 superseded by upstream's analyze_pipeline redesign (#88 audit)
  e95207d # 2026-08-05 superseded by upstream's analyze_pipeline redesign (#88 audit)
  bfce39e # 2026-08-12 subsumed by upstream PR #2997 review commit (claudedocs/sync-conflict-2026-08-12.md)
  75ec765 # 2026-09-01 superseded by upstream PR #3269 column-0 compact_diff (claudedocs/sync-conflict-2026-09-01.md)
  b008c53 # 2026-09-01 upstream condense_unified_diff emits column 0 itself (claudedocs/sync-conflict-2026-09-01.md)
  6871925 # 2026-09-01 superseded by upstream tool_form() exclude matching, PR #3749 (claudedocs/sync-conflict-2026-09-01.md)
  8a27c09 # 2026-09-01 superseded by upstream 2e0dd29 head/tail exclude check, PR #3324 (claudedocs/sync-conflict-2026-09-01.md)
  4e1ae5c # 2026-09-01 merged upstream as 55cbce2 (reworked in review, patch-id differs); proof test passes upstream (run 33357472129)
  a7998ad # 2026-09-01 merged upstream as 9d1c60a (patch-id differs, tsc_cmd.rs now byte-identical); proof test passes upstream
  791359c # 2026-09-01 superseded by upstream ca89767 requests_raw_log_output (--stat raw); proof test passes upstream
  9a8079a # 2026-09-01 the revert that removed 791359c/d88c1f4; restores upstream's code, nothing to advertise
  ce07aff # 2026-09-01 early snapshot of upstream PR #3268, superseded by the refreshed adoption acbe88c (fork PR #153); count it once
  54cb3a1 # 2026-09-01 early snapshot of upstream PR #3268, superseded by the refreshed adoption acbe88c (fork PR #153); count it once
  7753ec7 # 2026-09-01 test-only clippy amendment to acbe88c; nothing to advertise
  e2e11b7 # 2026-09-05 superseded by upstream d952a6b (Adrien Eppling, PR #3863) find disclosure-line skips; benchmark.sh is upstream's again (claudedocs/sync-conflict-2026-09-05.md)
  5a61595 # 2026-09-07 merged upstream as ee614e4 under PR #3788 (rebased, patch-id differs); rounds after it healed by patch-id (claudedocs/sync-conflict-2026-09-07.md)
  1f64783 # 2026-09-07 pre-region-parser hunk fix, subsumed by the region parser upstream merged as ee614e4 (PR #3788)
  8e65037 # 2026-09-07 pre-region-parser hunk fix, subsumed by the region parser upstream merged as ee614e4 (PR #3788)
  89cfe86 # 2026-09-07 pre-region-parser hunk fix, subsumed by the region parser upstream merged as ee614e4 (PR #3788)
  5bcee1c # 2026-09-07 pre-region-parser hunk fix, subsumed by the region parser upstream merged as ee614e4 (PR #3788)
  6375941 # 2026-09-07 pre-region-parser hunk fix, subsumed by the region parser upstream merged as ee614e4 (PR #3788)
  acbe88c # 2026-09-07 snapshot ecef036 of upstream PR #3268, superseded by the re-adoption of its head (8ca2e0e, 091e4a5); count it once
  8ca2e0e # 2026-09-07 upstream merged PR #3268 (9512e1a); this is its d4239ec with an adjacency resolution, so the patch-id differs. 091e4a5 heals by patch-id (claudedocs/sync-conflict-2026-09-07-b.md)
  371e058 # 2026-09-11 the Allow-assert arm it gated is gone: upstream d6ce8f7 decision::suppress_identity defers identity rewrites; its ASCII-IFS whitespace rule survives in strip_token, 505e355 (claudedocs/sync-conflict-2026-09-11.md)
  31fceff # 2026-09-11 Ask-assert arm and no-op renderer guards superseded by upstream d6ce8f7 decision::suppress_identity (claudedocs/sync-conflict-2026-09-11.md)
  653fb56 # 2026-10-09 superseded by upstream's --head-lines window: `head -N` rewrites there, --max-lines stays smart_truncate (claudedocs/sync-conflict-2026-10-09.md)
  00d8ee4 # 2026-10-09 superseded by upstream PR #3681 arg_tokenizer, which parses rg's -r as --replace; upstream closed #3162 for it (claudedocs/sync-conflict-2026-10-09.md)
  d43cef5 # 2026-10-09 amendment to 00d8ee4, removed with it (claudedocs/sync-conflict-2026-10-09.md)
  7db14a3 # 2026-10-09 declined upstream: the `--` in the legacy shell hooks; aeppling and KuSh closed upstream issue 1350 as legacy-only (native `rtk hook` is current). Upstream never carried it, so the 2026-08-05 note here ("merged ... byte-identical") was wrong; scripts restored to upstream's
  c8d6e28 # 2026-10-09 withdrawn: Claude `input`-key fallback; KuSh on upstream PR 2535: no Claude Code hook event sends it, and it widens the trigger surface
  cc5b6d3 # 2026-10-09 withdrawn: PowerShell Claude matcher; KuSh on upstream PR 2075: it runs PowerShell through the bash rewriter (mangles `\` paths, takes over ls/ps/curl aliases, ignores PowerShell-scoped deny rules). init/ restored to upstream's
  46d16ea # 2026-10-09 withdrawn: status/log machine-output passthrough; KuSh closed upstream PR 2573: upstream prints raw for -z/--name-only/--numstat/--raw/--format=%H, and runs other user log formats through its filter by design. The porcelain final newline stays fixed by d536891
  da407dc # 2026-10-09 amendment to 46d16ea, removed with it
  34946b7 # 2026-10-09 superseded by the re-adoption of the same author's current upstream PR 3571 (aa5f31e), which keeps stderr on its own stream after upstream PR 3772
  6f4d9f7 # 2026-10-09 amendment to 34946b7, removed with it (upstream PR 3571 strips the [warn] prefix itself)
  59f2564 # 2026-10-09 amendment to 34946b7, removed with it (upstream PR 3571 covers the rtk format call site itself)
  12db983 # 2026-10-09 upstream merged PR 3772; runner.rs matches upstream apart from help forwarding (verified against e4f0509)
  c27bbd0 # 2026-10-09 failure-only stderr forward, removed by 9ed2e49 in favour of upstream PR 3772; tests/stderr_passthrough_test.rs stays as a regression guard
  9ed2e49 # 2026-10-09 the removal of c27bbd0's forward; nothing of it remains to differ
  0704f58 # 2026-10-09 upstream merged PR 2628; only clap help tests remain in main.rs (verified against e4f0509)
  fa55089 # 2026-10-09 upstream merged PR 3199; mvn_cmd.rs and the jvm README are identical to upstream's
  13cf995 # 2026-10-09 upstream merged PR 2717; utils.rs, Cargo.toml and Cargo.lock identical, stream.rs differs only by help forwarding
  7ead416 # 2026-10-09 upstream's PR 2717 maps 54936 to GB18030 already; utils.rs identical
  1d1d86b # 2026-10-09 upstream fixed the benchmark fixtures (ba7a9ce); scripts/benchmark.sh identical
  8a0d305 # 2026-10-09 same as 1d1d86b; its fixtures are gone
  e93cde8 # 2026-10-09 withdrawn: omitting permissionDecision on Default/Ask rewrites is upstream's design (KuSh, upstream issue 3018); upstream closed its source PR 3031 as no longer applicable
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
# Newline-delimited full SHAs (not an associative array: macOS system bash is 3.2)
healed_auto=$'\n'"$(
  awk 'NR==FNR {seen[$1]=1; next} $1 in seen {print $2}' \
    <(git log --no-merges -p --since="$UPSTREAM_SINCE" "$UPSTREAM_REF" | git patch-id --stable) \
    <(git log --no-merges -p "$UPSTREAM_REF..HEAD" | git patch-id --stable)
)"$'\n'

rows=""
count=0
dropped_auto=0
dropped_audit=0
while IFS=$'\t' read -r full short subject; do
  [ -n "$short" ] || continue
  if [[ "$healed_auto" == *$'\n'"$full"$'\n'* ]]; then
    dropped_auto=$((dropped_auto + 1))
    continue
  fi
  # ${arr[@]+...} guard: expanding an empty array trips set -u on bash <4.4
  for h in ${HEALED_SHAS[@]+"${HEALED_SHAS[@]}"}; do
    if [[ "$full" == "$h"* ]]; then
      dropped_audit=$((dropped_audit + 1))
      continue 2
    fi
  done
  rows+="| ${subject} | [\`${short}\`](${REPO_URL}/commit/${short}) |"$'\n'
  count=$((count + 1))
done < <(
  git log "$UPSTREAM_REF..HEAD" --no-merges --format='%H%x09%h%x09%s' \
    | grep -E "	(feat|fix)" \
    | grep -vE "	(feat|fix)\((${EXCLUDE_SCOPES})(,(${EXCLUDE_SCOPES}))*\)"
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
