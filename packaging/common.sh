# shellcheck shell=bash
# Shared setup for the packaging scripts. Source it: `. "$(dirname "$0")/../common.sh"`.
#
# Exports:
#   ROOT              workspace root
#   VERSION           [workspace.package] version from Cargo.toml (override: ALBUMCRAFT_VERSION)
#   BUILD_DATE        UTC build date, YYYY-MM-DD (AppStream release date)
#   DIST              output directory for the packages (default: $ROOT/dist/release)
#   CARGO_TARGET_DIR  cargo's target directory (default: $ROOT/target)
#   APP_ID            reverse-DNS id of the desktop file, AppStream metadata and Flatpak

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export ROOT

# The version lives in one place: `[workspace.package] version` in the root Cargo.toml.
workspace_version() {
  awk '
    /^\[/ { in_pkg = ($0 == "[workspace.package]"); next }
    in_pkg && $1 == "version" { gsub(/[" ]/, "", $3); print $3; exit }
  ' "$ROOT/Cargo.toml"
}

VERSION="${ALBUMCRAFT_VERSION:-$(workspace_version)}"
if [[ -z "$VERSION" ]]; then
  echo "error: could not read [workspace.package] version from $ROOT/Cargo.toml" >&2
  exit 1
fi
export VERSION
export BUILD_DATE="${BUILD_DATE:-$(date -u +%Y-%m-%d)}"

DIST="${DIST:-$ROOT/dist/release}"
mkdir -p "$DIST"
DIST="$(cd "$DIST" && pwd)"
export DIST
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"
export APP_ID=io.github.kbarni.albumcraft

# A GitHub Actions warning (plain stderr elsewhere).
warn() {
  if [[ -n "${GITHUB_ACTIONS:-}" ]]; then echo "::warning::$*"; else echo "warning: $*" >&2; fi
}

# Copy the readme and licence files into a package directory.
copy_docs() {
  local dest="$1" f
  for f in README.md LICENSE-MIT LICENSE-APACHE NOTICE; do
    cp "$ROOT/$f" "$dest/"
  done
}

verify_sha256() {
  local expected="$1" file="$2"
  printf '%s  %s\n' "$expected" "$file" | sha256sum -c -
}
