#!/usr/bin/env bash
# Repackage the AlbumCraft .deb as a single-file Flatpak bundle:
#
#   $DIST/albumcraft-<version>-linux-<arch>.flatpak
#
# Usage: packaging/linux/flatpak-bundle.sh [--no-test] [DEB]
#
# DEB defaults to $DIST/albumcraft-<version>-linux-<arch>.deb from package.sh, for the host's
# architecture. Needs flatpak, flatpak-builder, dpkg-deb and the SVG pixbuf loader
# (librsvg2-common) for `appstreamcli compose`; the freedesktop runtime and SDK named in the
# manifest are installed per user from Flathub. Unless --no-test, the bundle is then installed per
# user and `albumcraft --version` runs inside the sandbox.
set -euo pipefail
# shellcheck source=../common.sh
. "$(dirname "${BASH_SOURCE[0]}")/../common.sh"
HERE="$ROOT/packaging/linux"
FLATHUB=https://dl.flathub.org/repo/flathub.flatpakrepo

TEST=1
DEB=""
while [[ $# -gt 0 ]]; do
  case "$1" in
    --no-test) TEST=0; shift ;;
    -h | --help) sed -n '2,13p' "$0"; exit 0 ;;
    -*) echo "unknown argument: $1" >&2; exit 2 ;;
    *) DEB="$1"; shift ;;
  esac
done

ARCH="$(uname -m)"
case "$ARCH" in
  x86_64) ;;
  aarch64 | arm64) ARCH=aarch64 ;;
  *) echo "unsupported architecture $ARCH" >&2; exit 2 ;;
esac
BASENAME="albumcraft-$VERSION-linux-$ARCH"
DEB="${DEB:-$DIST/$BASENAME.deb}"
[[ -f "$DEB" ]] || { echo "error: $DEB not found (run packaging/linux/package.sh first)" >&2; exit 1; }
for tool in flatpak flatpak-builder dpkg-deb; do
  command -v "$tool" >/dev/null || { echo "error: $tool not found (apt install flatpak flatpak-builder librsvg2-common)" >&2; exit 1; }
done
echo "==> AlbumCraft $VERSION Flatpak bundle for $ARCH from $(basename "$DEB")"

WORK="$CARGO_TARGET_DIR/flatpak-bundle"
rm -rf "$WORK"
mkdir -p "$WORK/stage"
dpkg-deb -x "$DEB" "$WORK/stage"
[[ -x "$WORK/stage/usr/bin/albumcraft" ]] || { echo "error: $DEB has no usr/bin/albumcraft" >&2; exit 1; }
cp "$HERE/flatpak/$APP_ID.yml" "$WORK/$APP_ID.yml"

flatpak remote-add --user --if-not-exists flathub "$FLATHUB"
# --disable-rofiles-fuse: no FUSE in containers and CI runners.
flatpak-builder --user --install-deps-from=flathub --disable-rofiles-fuse --force-clean \
  --default-branch=stable --repo="$WORK/repo" --state-dir="$WORK/state" "$WORK/build" "$WORK/$APP_ID.yml"

OUT="$DIST/$BASENAME.flatpak"
# --runtime-repo lets `flatpak install` fetch the freedesktop runtime from Flathub when missing.
flatpak build-bundle --runtime-repo="$FLATHUB" "$WORK/repo" "$OUT" "$APP_ID" stable
echo "wrote $OUT"

if [[ "$TEST" = 1 ]]; then
  flatpak install --user -y --noninteractive --reinstall "$OUT"
  flatpak info --user "$APP_ID"
  flatpak run "$APP_ID" --version
  flatpak run --command=sh "$APP_ID" -c 'ls /app/share/applications /app/share/metainfo /app/share/icons/hicolor/256x256/apps'
  # Printing runs lp/lpstat and movie export runs ffmpeg: say whether the sandbox has them.
  flatpak run --command=sh "$APP_ID" -c 'for t in lp lpstat ffmpeg; do printf "%s: %s\n" "$t" "$(command -v "$t" || echo missing)"; done'
fi
echo "==> done"
ls -lh "$OUT"
