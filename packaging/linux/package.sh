#!/usr/bin/env bash
# Build and package AlbumCraft for Linux x86_64 (or aarch64 when built there):
#
#   $DIST/albumcraft-<version>-linux-<arch>.tar.gz          unpack and run ./albumcraft
#   $DIST/albumcraft-<version>-linux-<arch>.deb             Debian, Ubuntu, Mint, Pop!_OS, ...
#   $DIST/albumcraft-<version>-linux-<arch>.AppImage        any distribution with glibc >= the build host's
#   $DIST/albumcraft-<version>-linux-<arch>.AppImage.zsync  delta updates (needs zsyncmake)
#
# The Flatpak bundle is made from the .deb afterwards by flatpak-bundle.sh.
#
# Usage: packaging/linux/package.sh [--skip-build] [--formats "tar deb appimage"]
#
# Needs cargo; nfpm for the .deb (https://nfpm.goreleaser.com); appimagetool for the AppImage
# (downloaded into $CARGO_TARGET_DIR, checksum-pinned, when it isn't on the PATH). Build on an
# old distribution (CI: Ubuntu 24.04, glibc 2.39, the oldest the ONNX Runtime library allows) so the binary runs on newer ones. Optional:
# desktop-file-validate and appstreamcli check the metadata, zsyncmake writes the .zsync.
set -euo pipefail
# shellcheck source=../common.sh
. "$(dirname "${BASH_SOURCE[0]}")/../common.sh"
HERE="$ROOT/packaging/linux"

SKIP_BUILD=0
FORMATS="tar deb appimage"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-build) SKIP_BUILD=1; shift ;;
    --formats) FORMATS="$2"; shift 2 ;;
    -h | --help) sed -n '2,17p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
has() { case " $FORMATS " in *" $1 "*) return 0 ;; *) return 1 ;; esac; }

ARCH="$(uname -m)"
case "$ARCH" in
  x86_64) DEB_ARCH=amd64 ;;
  aarch64 | arm64) ARCH=aarch64; DEB_ARCH=arm64 ;;
  *) echo "unsupported architecture $ARCH" >&2; exit 2 ;;
esac
BASENAME="albumcraft-$VERSION-linux-$ARCH"
echo "==> AlbumCraft $VERSION for Linux $ARCH ($FORMATS)"

if [[ "$SKIP_BUILD" = 0 ]]; then
  (cd "$ROOT" && cargo build --release --locked -p albumcraft)
fi
BIN="$CARGO_TARGET_DIR/release/albumcraft"
WORK="$CARGO_TARGET_DIR/linux-package"
STAGE="$WORK/root"
rm -rf "$WORK"

# ---- an FHS tree shared by the .deb, the AppImage and the Flatpak ------------------------------
install -Dm755 "$BIN" "$STAGE/usr/bin/albumcraft"
strip "$STAGE/usr/bin/albumcraft" 2>/dev/null || true
install -Dm644 "$HERE/$APP_ID.desktop" "$STAGE/usr/share/applications/$APP_ID.desktop"
mkdir -p "$STAGE/usr/share/metainfo"
sed -e "s/@VERSION@/$VERSION/g" -e "s/@DATE@/$BUILD_DATE/g" \
  "$HERE/$APP_ID.metainfo.xml.in" >"$STAGE/usr/share/metainfo/$APP_ID.metainfo.xml"
# Icons are named after the app id (Flatpak exports only icons with that name).
for dir in "$ROOT"/assets/app-icon/hicolor/*/apps; do
  size="$(basename "$(dirname "$dir")")"
  for icon in "$dir"/albumcraft.*; do
    install -Dm644 "$icon" "$STAGE/usr/share/icons/hicolor/$size/apps/$APP_ID.${icon##*.}"
  done
done
mkdir -p "$STAGE/usr/share/doc/albumcraft"
copy_docs "$STAGE/usr/share/doc/albumcraft"

if command -v desktop-file-validate >/dev/null; then
  desktop-file-validate "$STAGE/usr/share/applications/$APP_ID.desktop"
fi
if command -v appstreamcli >/dev/null; then
  appstreamcli validate --no-net --explain "$STAGE/usr/share/metainfo/$APP_ID.metainfo.xml"
fi
"$STAGE/usr/bin/albumcraft" --version

# ---- .tar.gz: one folder to unpack and run from ------------------------------------------------
if has tar; then
  dir="$WORK/tar/$BASENAME"
  mkdir -p "$dir"
  cp "$STAGE/usr/bin/albumcraft" "$HERE/$APP_ID.desktop" "$dir/"
  cp "$STAGE/usr/share/icons/hicolor/256x256/apps/$APP_ID.png" "$dir/"
  copy_docs "$dir"
  tar -C "$WORK/tar" -czf "$DIST/$BASENAME.tar.gz" "$BASENAME"
  echo "wrote $DIST/$BASENAME.tar.gz"
fi

# ---- .deb ---------------------------------------------------------------------------------------
if has deb; then
  command -v nfpm >/dev/null || { echo "error: nfpm not found (https://nfpm.goreleaser.com/install/)" >&2; exit 1; }
  export NFPM_ARCH="$DEB_ARCH"
  # nfpm expands environment variables in fields like `version`, but not in `contents[].src`.
  sed "s|\${STAGE}|$STAGE|g" "$HERE/nfpm.yaml" >"$WORK/nfpm.yaml"
  (cd "$ROOT" && nfpm package -f "$WORK/nfpm.yaml" -p deb -t "$DIST/$BASENAME.deb")
fi

# ---- AppImage -----------------------------------------------------------------------------------
if has appimage; then
  APPIMAGETOOL_VERSION=1.9.1
  # appimagetool otherwise downloads the unpinned `continuous` runtime and embeds it.
  APPIMAGE_RUNTIME_VERSION=20251108
  case "$ARCH" in
    x86_64)
      APPIMAGETOOL_SHA256=ed4ce84f0d9caff66f50bcca6ff6f35aae54ce8135408b3fa33abfc3cb384eb0
      APPIMAGE_RUNTIME_SHA256=2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d ;;
    aarch64)
      APPIMAGETOOL_SHA256=f0837e7448a0c1e4e650a93bb3e85802546e60654ef287576f46c71c126a9158
      APPIMAGE_RUNTIME_SHA256=00cbdfcf917cc6c0ff6d3347d59e0ca1f7f45a6df1a428a0d6d8a78664d87444 ;;
  esac
  APPDIR="$WORK/AlbumCraft.AppDir"
  cp -R "$STAGE" "$APPDIR"
  ln -s usr/bin/albumcraft "$APPDIR/AppRun"
  cp "$HERE/$APP_ID.desktop" "$APPDIR/$APP_ID.desktop"
  cp "$STAGE/usr/share/icons/hicolor/256x256/apps/$APP_ID.png" "$APPDIR/$APP_ID.png"
  ln -s "$APP_ID.png" "$APPDIR/.DirIcon"

  TOOL="${APPIMAGETOOL:-$(command -v appimagetool || true)}"
  if [[ -z "$TOOL" ]]; then
    TOOL="$CARGO_TARGET_DIR/appimagetool-$APPIMAGETOOL_VERSION-$ARCH.AppImage"
    if [[ ! -x "$TOOL" ]]; then
      curl --proto '=https' -fsSL -o "$TOOL.download" \
        "https://github.com/AppImage/appimagetool/releases/download/$APPIMAGETOOL_VERSION/appimagetool-$ARCH.AppImage"
      verify_sha256 "$APPIMAGETOOL_SHA256" "$TOOL.download"
      chmod 755 "$TOOL.download"
      mv "$TOOL.download" "$TOOL"
    fi
    verify_sha256 "$APPIMAGETOOL_SHA256" "$TOOL"
  fi
  RUNTIME="$CARGO_TARGET_DIR/appimage-runtime-$APPIMAGE_RUNTIME_VERSION-$ARCH"
  if [[ ! -f "$RUNTIME" ]]; then
    curl --proto '=https' -fsSL -o "$RUNTIME.download" \
      "https://github.com/AppImage/type2-runtime/releases/download/$APPIMAGE_RUNTIME_VERSION/runtime-$ARCH"
    verify_sha256 "$APPIMAGE_RUNTIME_SHA256" "$RUNTIME.download"
    mv "$RUNTIME.download" "$RUNTIME"
  fi
  verify_sha256 "$APPIMAGE_RUNTIME_SHA256" "$RUNTIME"
  TOOL="$(cd "$(dirname "$TOOL")" && pwd)/$(basename "$TOOL")"
  RUNTIME="$(cd "$(dirname "$RUNTIME")" && pwd)/$(basename "$RUNTIME")"
  OUT="$DIST/$BASENAME.AppImage"
  rm -f "$OUT" "$OUT.zsync"
  # Update information: AppImageUpdate and similar tools fetch only the changed blocks of the
  # newest release (not a pre-release) through the .zsync published next to the AppImage.
  REPO="${GITHUB_REPOSITORY:-kbarni/AlbumCraft}"
  UPDATE_INFO="gh-releases-zsync|${REPO%%/*}|${REPO#*/}|latest|albumcraft-*-linux-$ARCH.AppImage.zsync"
  # Extract-and-run works without FUSE (containers, CI). The AppImage embeds the static runtime,
  # so users don't need libfuse2 either. appimagetool writes the .zsync into its working
  # directory, hence the cd.
  (cd "$DIST" && ARCH="$ARCH" APPIMAGE_EXTRACT_AND_RUN=1 "$TOOL" --no-appstream \
    --runtime-file "$RUNTIME" -u "$UPDATE_INFO" "$APPDIR" "$OUT")
  echo "wrote $OUT"
  if [[ ! -s "$OUT.zsync" ]]; then
    warn "zsyncmake not found, so $OUT.zsync was not written; AppImage delta updates need it"
  fi
  APPIMAGE_EXTRACT_AND_RUN=1 "$OUT" --version
fi

echo "==> done"
ls -lh "$DIST"
