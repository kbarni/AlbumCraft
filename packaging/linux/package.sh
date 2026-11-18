#!/usr/bin/env bash
# Build and package AlbumCraft for Linux x86_64 (or aarch64 when built there):
#
#   $DIST/albumcraft-<version>-linux-<arch>.tar.gz          unpack and run ./albumcraft
#   $DIST/albumcraft-<version>-linux-<arch>.deb             Debian, Ubuntu, Mint, Pop!_OS, ...
#   $DIST/albumcraft-<version>-linux-<arch>.AppImage        any distribution with glibc >= $GLIBC
#   $DIST/albumcraft-<version>-linux-<arch>.AppImage.zsync  delta updates (needs zsyncmake)
#
# The Flatpak bundle is made from the .deb afterwards by flatpak-bundle.sh.
#
# Usage: packaging/linux/package.sh [--skip-build] [--formats "tar deb appimage"]
#
# Needs cargo with cargo-zigbuild and zig (`cargo install --locked cargo-zigbuild`, zig from
# https://ziglang.org/download/): the program is linked against glibc $GLIBC whatever the build
# machine has, so the packages run on Ubuntu 22.04, Debian 12 and newer. ONNX Runtime (the AI
# features) is not linked in (Cargo feature `onnx-dynamic`): Microsoft's build of
# libonnxruntime.so, which needs only glibc 2.28, is downloaded (checksum-pinned, kept in
# $CARGO_TARGET_DIR) and packaged beside the program. Also needs nfpm for the .deb
# (https://nfpm.goreleaser.com); appimagetool for the AppImage (downloaded into
# $CARGO_TARGET_DIR, checksum-pinned, when it isn't on the PATH), and for the AppImage dpkg-deb
# (unpacks Ubuntu 22.04's libxkbcommon-x11, which it carries) and patchelf (likewise downloaded
# when missing). Optional:
# desktop-file-validate and appstreamcli check the metadata, zsyncmake writes the .zsync.
#
# Videos: the FFmpeg libraries of packaging/ffmpeg/build.sh, built on a system with glibc
# $GLIBC or older (Ubuntu 22.04: release.yml's ffmpeg-linux job), in .tools/ffmpeg/linux-<arch>
# or the folder named by $ALBUMCRAFT_FFMPEG. They go where the program looks: beside it in the
# .tar.gz, /usr/lib/albumcraft in the others, with their licence texts.
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
    -h | --help) sed -n '2,23p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
has() { case " $FORMATS " in *" $1 "*) return 0 ;; *) return 1 ;; esac; }

ARCH="$(uname -m)"
# The oldest glibc the program runs with (Ubuntu 22.04; Debian 12 has 2.36). Keep nfpm.yaml's
# libc6 dependency the same.
GLIBC=2.35
# Microsoft's ONNX Runtime release packaged with the program: the version the `ort` crate's own
# prebuilt library has (ort-sys's dist.tsv), so every platform runs the same one.
ONNXRUNTIME_VERSION=1.31.0
case "$ARCH" in
  x86_64)
    DEB_ARCH=amd64
    ONNXRUNTIME_ARCH=x64
    ONNXRUNTIME_SHA256=cc5c72baf5ae5c8238a6841f0897227be2d02826b9cf98eaf02fdefaeeb03a57 ;;
  aarch64 | arm64)
    ARCH=aarch64
    DEB_ARCH=arm64
    ONNXRUNTIME_ARCH=aarch64
    ONNXRUNTIME_SHA256=c5b8b3cca31f3d643a3b313b8f42e2d0f16a4fcefd02cda343d97bd5306afaab ;;
  *) echo "unsupported architecture $ARCH" >&2; exit 2 ;;
esac
TARGET="$ARCH-unknown-linux-gnu"
BASENAME="albumcraft-$VERSION-linux-$ARCH"
echo "==> AlbumCraft $VERSION for Linux $ARCH ($FORMATS)"

FFMPEG="${ALBUMCRAFT_FFMPEG:-$ROOT/.tools/ffmpeg/linux-$ARCH}"
[[ -f "$FFMPEG/lib/pkgconfig/libavcodec.pc" && -f "$FFMPEG/share/licenses/FFmpeg-COPYING.LGPLv2.1" ]] || {
  echo "error: no FFmpeg build in $FFMPEG: run packaging/ffmpeg/build.sh (on glibc $GLIBC or older)" >&2; exit 1
}
FFMPEG="$(cd "$FFMPEG" && pwd)"
# cargo finds them with pkg-config (and build.rs gives the program an rpath to them, besides the
# package folders)
export "PKG_CONFIG_PATH_$(echo "$TARGET" | tr '-' '_')=$FFMPEG/lib/pkgconfig"

if [[ "$SKIP_BUILD" = 0 ]]; then
  command -v cargo-zigbuild >/dev/null || { echo "error: cargo-zigbuild not found (cargo install --locked cargo-zigbuild)" >&2; exit 1; }
  command -v zig >/dev/null || { echo "error: zig not found (https://ziglang.org/download/)" >&2; exit 1; }
  # The build machine's own libraries (ALSA) may use newer glibc symbols than $GLIBC has; the
  # program is linked against their names only and runs with the ones of the user's system.
  rustflags_var="CARGO_TARGET_$(echo "$TARGET" | tr 'a-z-' 'A-Z_')_RUSTFLAGS"
  (cd "$ROOT" && env "$rustflags_var=-C link-arg=-Wl,--allow-shlib-undefined" \
    cargo zigbuild --release --locked -p albumcraft --features onnx-dynamic --target "$TARGET.$GLIBC")
fi
BIN="$CARGO_TARGET_DIR/$TARGET/release/albumcraft"
WORK="$CARGO_TARGET_DIR/linux-package"
STAGE="$WORK/root"
rm -rf "$WORK"

# The newest glibc version a file asks for must not be past $GLIBC.
check_glibc() {
  local newest
  newest="$(objdump -T "$1" | grep -o 'GLIBC_[0-9.]*' | sort -uV | tail -1)"
  if [[ "$(printf '%s\n' "GLIBC_$GLIBC" "$newest" | sort -V | tail -1)" != "GLIBC_$GLIBC" ]]; then
    echo "error: $1 needs $newest, newer than glibc $GLIBC" >&2
    exit 1
  fi
  echo "$(basename "$1") needs ${newest:-no glibc}"
}
check_glibc "$BIN"

# ---- ONNX Runtime: Microsoft's release, loaded by the program when an AI feature first runs ----
ONNXRUNTIME="onnxruntime-linux-$ONNXRUNTIME_ARCH-$ONNXRUNTIME_VERSION"
if [[ ! -f "$CARGO_TARGET_DIR/$ONNXRUNTIME.tgz" ]]; then
  curl --proto '=https' -fsSL -o "$CARGO_TARGET_DIR/$ONNXRUNTIME.tgz.download" \
    "https://github.com/microsoft/onnxruntime/releases/download/v$ONNXRUNTIME_VERSION/$ONNXRUNTIME.tgz"
  verify_sha256 "$ONNXRUNTIME_SHA256" "$CARGO_TARGET_DIR/$ONNXRUNTIME.tgz.download"
  mv "$CARGO_TARGET_DIR/$ONNXRUNTIME.tgz.download" "$CARGO_TARGET_DIR/$ONNXRUNTIME.tgz"
fi
verify_sha256 "$ONNXRUNTIME_SHA256" "$CARGO_TARGET_DIR/$ONNXRUNTIME.tgz"
mkdir -p "$WORK/onnxruntime"
tar -C "$WORK/onnxruntime" -xzf "$CARGO_TARGET_DIR/$ONNXRUNTIME.tgz" \
  "$ONNXRUNTIME/lib/libonnxruntime.so.$ONNXRUNTIME_VERSION" "$ONNXRUNTIME/LICENSE" "$ONNXRUNTIME/ThirdPartyNotices.txt"
ONNXRUNTIME_DIR="$WORK/onnxruntime/$ONNXRUNTIME"
check_glibc "$ONNXRUNTIME_DIR/lib/libonnxruntime.so.$ONNXRUNTIME_VERSION"
# The library and its licence texts, into a package directory.
copy_onnxruntime() {
  local lib="$1" docs="$2"
  install -Dm644 "$ONNXRUNTIME_DIR/lib/libonnxruntime.so.$ONNXRUNTIME_VERSION" "$lib/libonnxruntime.so"
  install -Dm644 "$ONNXRUNTIME_DIR/LICENSE" "$docs/LICENSE-ONNXRuntime"
  install -Dm644 "$ONNXRUNTIME_DIR/ThirdPartyNotices.txt" "$docs/ThirdPartyNotices-ONNXRuntime.txt"
}

# ---- FFmpeg: the libraries videos are played with ----------------------------------------------
# Each under the name the program and the others ask for (its SONAME, libavcodec.so.62), not the
# libavcodec.so symlinks; they find each other through their own $ORIGIN (build.sh checks).
FFMPEG_LIBS=()
for f in "$FFMPEG"/lib/*.so.*; do
  [[ -L "$f" ]] && continue
  soname="$(objdump -p "$f" | awk '$1 == "SONAME" {print $2}')"
  [[ -n "$soname" ]] || { echo "error: $f has no SONAME" >&2; exit 1; }
  check_glibc "$f"
  FFMPEG_LIBS+=("$f:$soname")
done
[[ ${#FFMPEG_LIBS[@]} -ge 6 ]] || { echo "error: expected 6 FFmpeg and dav1d libraries in $FFMPEG/lib, found ${#FFMPEG_LIBS[@]}" >&2; exit 1; }
# every FFmpeg library the program asks for is one of them
for needed in $(objdump -p "$BIN" | awk '$1 == "NEEDED" {print $2}' | grep -E '^lib(av|sw)'); do
  [[ " ${FFMPEG_LIBS[*]} " == *":$needed "* ]] || { echo "error: the program needs $needed, which $FFMPEG/lib doesn't have" >&2; exit 1; }
done
copy_ffmpeg() {
  local lib="$1" docs="$2" entry
  for entry in "${FFMPEG_LIBS[@]}"; do
    install -Dm644 "${entry%%:*}" "$lib/${entry#*:}"
  done
  install -Dm644 "$FFMPEG/share/licenses/FFmpeg-COPYING.LGPLv2.1" "$docs/FFmpeg-COPYING.LGPLv2.1"
  install -Dm644 "$FFMPEG/share/licenses/dav1d-COPYING" "$docs/dav1d-COPYING"
}

# ---- an FHS tree shared by the .deb, the AppImage and the Flatpak ------------------------------
install -Dm755 "$BIN" "$STAGE/usr/bin/albumcraft"
strip "$STAGE/usr/bin/albumcraft" 2>/dev/null || true
# where the program looks for it: ../lib/albumcraft from its own folder
copy_onnxruntime "$STAGE/usr/lib/albumcraft" "$STAGE/usr/share/doc/albumcraft"
copy_ffmpeg "$STAGE/usr/lib/albumcraft" "$STAGE/usr/share/doc/albumcraft"
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
  # beside the program, the other place it looks
  copy_onnxruntime "$dir" "$dir"
  copy_ffmpeg "$dir" "$dir"
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

  # winit's X11 backend dlopens libxkbcommon-x11.so.0 (with libxcb-xkb.so.1), which only Qt pulls
  # in: GTK desktops (Cinnamon, Xfce, MATE) often lack it, and the window then fails to open. The
  # AppImage carries Ubuntu 22.04's copies (glibc 2.35, checksums from its Packages index), with
  # libxkbcommon.so.0 of the same version: libxkbcommon-x11 fills libxkbcommon's private structs,
  # and 1.4.0's with the system's 1.6.0 crashes. The program finds them through a RUNPATH set on
  # the AppImage's copy only, and libxkbcommon-x11 finds the others beside itself through its own;
  # libxcb.so.1 stays the system's (every X11 desktop has it).
  PATCHELF_VERSION=0.18.0
  case "$ARCH" in
    x86_64)
      UBUNTU_MIRROR=https://archive.ubuntu.com/ubuntu
      PATCHELF_SHA256=ce84f2447fb7a8679e58bc54a20dc2b01b37b5802e12c57eece772a6f14bf3f0
      XKB_SHA256=a0470edb237d52b71ad41f755c043164abed01423585ed5ea19525b3baaf84f7
      XKB_X11_SHA256=e29c40fab6e0f055218aee4c7e60cd61ea82283229d7ac7ac02e226e92a5b0d5
      XCB_XKB_SHA256=4cf69af3ff6a60b63f9f6ea6e7d27471114c6bcbcf3f981f7b88f361029e5eb8 ;;
    aarch64)
      UBUNTU_MIRROR=https://ports.ubuntu.com/ubuntu-ports
      PATCHELF_SHA256=ae13e2effe077e829be759182396b931d8f85cfb9cfe9d49385516ea367ef7b2
      XKB_SHA256=451be798e301077cde9d34afe00886e6c77699b1734985c2191a7a4ba783911d
      XKB_X11_SHA256=a11f15cd8ecaf166f545acc2dccb0026378e467f841f1dd19248ead79f237162
      XCB_XKB_SHA256=2ab72c0e6ca4ace102f330a83055a7655d2ef9b00ff8abc2c217c0aed2980628 ;;
  esac
  # Download $1 to $CARGO_TARGET_DIR/$3 once, checking its SHA-256 ($2) each time.
  fetch_pinned() {
    local url="$1" sha="$2" file="$CARGO_TARGET_DIR/$3"
    if [[ ! -f "$file" ]]; then
      curl --proto '=https' -fsSL -o "$file.download" "$url"
      verify_sha256 "$sha" "$file.download"
      mv "$file.download" "$file"
    fi
    verify_sha256 "$sha" "$file"
  }
  command -v dpkg-deb >/dev/null || { echo "error: dpkg-deb not found (it unpacks Ubuntu's libxkbcommon-x11 for the AppImage)" >&2; exit 1; }
  PATCHELF="${PATCHELF:-$(command -v patchelf || true)}"
  if [[ -z "$PATCHELF" ]]; then
    fetch_pinned "https://github.com/NixOS/patchelf/releases/download/$PATCHELF_VERSION/patchelf-$PATCHELF_VERSION-$ARCH.tar.gz" \
      "$PATCHELF_SHA256" "patchelf-$PATCHELF_VERSION-$ARCH.tar.gz"
    mkdir -p "$WORK/patchelf"
    tar -C "$WORK/patchelf" -xzf "$CARGO_TARGET_DIR/patchelf-$PATCHELF_VERSION-$ARCH.tar.gz" ./bin/patchelf
    PATCHELF="$WORK/patchelf/bin/patchelf"
  fi
  # package, version, pool directory, checksum, library file
  while read -r pkg ver pool sha lib; do
    deb="${pkg}_${ver}_$DEB_ARCH.deb"
    fetch_pinned "$UBUNTU_MIRROR/pool/main/$pool/$deb" "$sha" "ubuntu-$deb"
    mkdir -p "$WORK/ubuntu/$pkg"
    dpkg-deb -x "$CARGO_TARGET_DIR/ubuntu-$deb" "$WORK/ubuntu/$pkg"
    install -Dm644 "$(readlink -f "$WORK/ubuntu/$pkg/usr/lib/$ARCH-linux-gnu/$lib")" "$APPDIR/usr/lib/albumcraft/$lib"
    install -Dm644 "$WORK/ubuntu/$pkg/usr/share/doc/$pkg/copyright" "$APPDIR/usr/share/doc/albumcraft/copyright-$pkg"
    check_glibc "$APPDIR/usr/lib/albumcraft/$lib"
  done <<EOF
libxkbcommon0 1.4.0-1 libx/libxkbcommon $XKB_SHA256 libxkbcommon.so.0
libxkbcommon-x11-0 1.4.0-1 libx/libxkbcommon $XKB_X11_SHA256 libxkbcommon-x11.so.0
libxcb-xkb1 1.14-3ubuntu3 libx/libxcb $XCB_XKB_SHA256 libxcb-xkb.so.1
EOF
  "$PATCHELF" --set-rpath '$ORIGIN' "$APPDIR/usr/lib/albumcraft/libxkbcommon-x11.so.0"
  "$PATCHELF" --set-rpath '$ORIGIN/../lib/albumcraft' "$APPDIR/usr/bin/albumcraft"

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
