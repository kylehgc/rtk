#!/usr/bin/env bash
#
# Stamp Cargo.toml's package version from the release tag being built.
#
# Cargo.toml on develop carries whatever version release-please last wrote on
# master, so every RC built from develop reports that stale number: an RC tagged
# fork-dev-0.1.0-rc.9 shipped `rtk_0.42.4-1_amd64.deb` and `rtk --version` said
# 0.42.4. Upstream has the same behaviour, but for a fork it is worse — that
# filename is byte-identical to upstream's own RC artifact, so two different
# binaries share one package identity.
#
# CARGO_PKG_VERSION feeds `rtk --version`, telemetry, cargo-deb, and
# cargo-generate-rpm, so stamping it once here fixes all four.
#
# Usage: scripts/version-from-tag.sh <tag> [cargo-toml-path]
#        scripts/version-from-tag.sh --test
set -euo pipefail

# fork-dev-0.1.0-rc.9 -> 0.1.0+rc.9    fork-v0.1.0 -> 0.1.0
# dev-0.44.2-rc.344   -> 0.44.2+rc.344 v0.44.1     -> 0.44.1
#
# The pre-release marker becomes `+rc.N` (semver build metadata) rather than
# `-rc.N` (semver pre-release), because RPM forbids `-` in a version:
#
#   invalid version "0.1.0-rc.10": contains invalid character
#   (allowed: alphanumeric, '.', '_', '+', '%', '{', '}', '~', '^')
#
# `+` is legal in all three: semver, Debian, and RPM. `~` would suit RPM and
# Debian better (it sorts before the release) but cargo rejects it outright.
version_from_tag() {
  local v="$1"
  v="${v#fork-}"
  v="${v#dev-}"
  v="${v#v}"
  printf '%s' "$v" | sed -E 's/^([0-9]+\.[0-9]+\.[0-9]+)-(.+)$/\1+\2/'
}

if [ "${1:-}" = "--test" ]; then
  fail=0
  check() {
    local got
    got="$(version_from_tag "$1")"
    if [ "$got" = "$2" ]; then
      echo "  ok   $1 -> $got"
    else
      echo "  FAIL $1 -> $got (expected $2)"
      fail=1
    fi
  }
  check "fork-dev-0.1.0-rc.9" "0.1.0+rc.9"
  check "fork-v0.1.0"         "0.1.0"
  check "fork-v1.2.3"         "1.2.3"
  check "dev-0.44.2-rc.344"   "0.44.2+rc.344"
  check "v0.44.1"             "0.44.1"
  exit $fail
fi

TAG="${1:?usage: version-from-tag.sh <tag> [cargo-toml-path]}"
MANIFEST="${2:-Cargo.toml}"

VERSION="$(version_from_tag "$TAG")"

# Refuse anything cargo would reject, rather than corrupting the manifest and
# failing later with a confusing parse error.
if ! printf '%s' "$VERSION" | grep -qE '^[0-9]+\.[0-9]+\.[0-9]+(\+[0-9A-Za-z.]+)?$'; then
  echo "::error::Tag '$TAG' does not yield a semver version (got '$VERSION')" >&2
  exit 1
fi

[ -f "$MANIFEST" ] || { echo "::error::$MANIFEST not found" >&2; exit 1; }

# Only the first `version =` — that is the [package] one. Dependency versions
# appear later in the file and must not be touched.
#
# awk rather than sed, for two separate portability reasons on macOS runners:
# BSD sed reads the argument after -i as a backup suffix (the GNU form fails with
# `sed: 1: "Cargo.toml`), and the `0,/re/` address range used to match only the
# first occurrence is a GNU extension that BSD sed rejects outright.
awk -v v="$VERSION" '
  !done && /^version = "/ {
    print "version = \"" v "\""
    done = 1
    next
  }
  { print }
' "$MANIFEST" > "${MANIFEST}.tmp"
mv "${MANIFEST}.tmp" "$MANIFEST"

STAMPED="$(grep -m1 '^version = ' "$MANIFEST" | sed 's/version = "\(.*\)"/\1/')"
if [ "$STAMPED" != "$VERSION" ]; then
  echo "::error::Failed to stamp $MANIFEST (it reads '$STAMPED', wanted '$VERSION')" >&2
  exit 1
fi

echo "Stamped $MANIFEST version = $VERSION (from tag $TAG)"
