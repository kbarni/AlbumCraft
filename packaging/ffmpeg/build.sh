#!/usr/bin/env bash
# Build the FFmpeg libraries AlbumCraft plays videos with: a playback-only build (decoders and
# file readers for camera and phone videos, nothing else), plus dav1d for AV1.
#
#   .tools/ffmpeg/<os>-<arch>/          include/, lib/ (libraries or, on Windows, import libraries
#                                       for GNU ld and MSVC, and pkg-config files), bin/ (Windows: the DLLs),
#                                       share/licenses/
#
# Usage: packaging/ffmpeg/build.sh [--prefix DIR] [--jobs N] [--archive]
#   --archive   also pack the result as dist/ffmpeg/albumcraft-ffmpeg-<os>-<arch>.tar.gz (+ .sha256),
#               to hand to someone building the app on another machine (packaging/ffmpeg/README.md)
#
# Linux, macOS (Xcode command-line tools + Homebrew) and Windows (an MSYS2 UCRT64 shell).
#
# Licence: LGPL 2.1 or later (FFmpeg without its GPL and non-free parts) and BSD (dav1d), both
# as shared libraries beside the program, so the MIT/Apache app may load them. Keep it that way:
# never add --enable-gpl, --enable-nonfree or --enable-version3 here.
#
# Needs a C compiler, make, pkg-config, nasm (x86_64 only: FFmpeg's and dav1d's x86 assembly;
# without it decoding is several times slower), meson and ninja (dav1d), curl. Downloads are
# checksum-pinned and kept in .tools/ffmpeg/src/, so building again needs no network.
#
# After building, cargo finds the libraries through PKG_CONFIG_PATH_<target> (see the line this
# script prints, for .cargo/config.toml; the .pc files name their own folder, so the result can
# be moved) and the program finds them at run time through its rpath (build.rs; macOS: the
# libraries are named @rpath/…), or on Windows beside the program / on PATH.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

FFMPEG_VERSION=9.0.2
# checked against FFmpeg's release signature (ffmpeg-$FFMPEG_VERSION.tar.xz.asc, key FCF986EA15E6E293A5644F10B4322F04D67658D8)
FFMPEG_SHA256=8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e
DAV1D_VERSION=1.5.5
# = the published dav1d-$DAV1D_VERSION.tar.xz.sha256
DAV1D_SHA256=ca96b1d0ba3f01f55de7231d775892b64994ba061aaeeaaa4f662a390b3abf9e
# the oldest macOS the app runs on (packaging/macos/Info.plist, release.yml)
MACOS_MIN=13.4

case "$(uname -s)" in
  Linux) OS=linux ;;
  Darwin) OS=macos ;;
  MINGW* | MSYS* | CYGWIN*) OS=windows ;;
  *) echo "unsupported system $(uname -s)" >&2; exit 2 ;;
esac
if [[ "$OS" = windows && "${MSYSTEM:-}" != UCRT64 ]]; then
  echo "error: run this from an MSYS2 UCRT64 shell (MSYSTEM is '${MSYSTEM:-}')" >&2; exit 2
fi
ARCH="$(uname -m)"
[[ "$ARCH" = arm64 ]] && ARCH=aarch64
PREFIX="$ROOT/.tools/ffmpeg/$OS-$ARCH"
JOBS="$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4)"
ARCHIVE=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    --prefix) PREFIX="$2"; shift 2 ;;
    --jobs) JOBS="$2"; shift 2 ;;
    --archive) ARCHIVE=1; shift ;;
    -h | --help) sed -n '2,26p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
SRC="$ROOT/.tools/ffmpeg/src"
WORK="$ROOT/.tools/ffmpeg/build-$OS-$ARCH"

tools=(cc make pkg-config meson ninja curl)
[[ "$ARCH" != x86_64 ]] || tools+=(nasm)
for tool in "${tools[@]}"; do
  command -v "$tool" >/dev/null || { echo "error: $tool not found" >&2; exit 1; }
done

sha256() { if command -v sha256sum >/dev/null; then sha256sum "$1"; else shasum -a 256 "$1"; fi | cut -d' ' -f1; }
fetch() { # url file sha256
  mkdir -p "$SRC"
  if [[ ! -f "$SRC/$2" ]]; then
    echo "==> downloading $2"
    curl -sSfL "$1" -o "$SRC/$2.part"
    mv "$SRC/$2.part" "$SRC/$2"
  fi
  local got
  got="$(sha256 "$SRC/$2")"
  [[ "$got" = "$3" ]] || { echo "error: $2 has checksum $got, expected $3" >&2; rm -f "$SRC/$2"; exit 1; }
}
fetch "https://downloads.videolan.org/pub/videolan/dav1d/$DAV1D_VERSION/dav1d-$DAV1D_VERSION.tar.xz" "dav1d-$DAV1D_VERSION.tar.xz" "$DAV1D_SHA256"
fetch "https://ffmpeg.org/releases/ffmpeg-$FFMPEG_VERSION.tar.xz" "ffmpeg-$FFMPEG_VERSION.tar.xz" "$FFMPEG_SHA256"

rm -rf "$WORK" "$PREFIX"
# FFmpeg's configure can't take paths with spaces (it mangles -I/-L flags): then build and install
# into a temporary folder without any, and copy the result to PREFIX at the end.
STAGE="$PREFIX"
if [[ "$WORK$PREFIX" =~ [[:space:]] ]]; then
  WORK="$(mktemp -d)"
  trap 'rm -rf "$WORK"' EXIT
  [[ ! "$WORK" =~ [[:space:]] ]] || { echo "error: the temporary folder $WORK has a space in it" >&2; exit 1; }
  STAGE="$WORK/prefix"
fi
mkdir -p "$WORK" "$STAGE"
tar -xf "$SRC/dav1d-$DAV1D_VERSION.tar.xz" -C "$WORK"
tar -xf "$SRC/ffmpeg-$FFMPEG_VERSION.tar.xz" -C "$WORK"

# The libraries find each other in their own folder (beside the program once packaged).
EXTRA_LIBS=''
CONFIGURE_EXTRA=()
case "$OS" in
  # configure drops a "$ORIGIN" it is given; added to its makefile settings below instead
  linux) RPATH_FLAGS='' ;;
  # named @rpath/<file> (not their build folder) and looking in their own folder for each other
  macos)
    RPATH_FLAGS='-Wl,-rpath,@loader_path'
    CONFIGURE_EXTRA=(--install-name-dir=@rpath)
    export MACOSX_DEPLOYMENT_TARGET="$MACOS_MIN"
    ;;
  # no rpath (Windows looks beside the program); gcc's own runtime (libgcc_s_seh-1.dll,
  # libwinpthread-1.dll) linked into the DLLs, so they need nothing but Windows' own
  windows)
    RPATH_FLAGS='-static-libgcc'
    EXTRA_LIBS='-l:libwinpthread.a'
    ;;
esac

echo "==> dav1d $DAV1D_VERSION"
(
  cd "$WORK/dav1d-$DAV1D_VERSION"
  meson setup build --prefix="$STAGE" --libdir=lib --buildtype=release --default-library=shared \
    -Denable_tools=false -Denable_tests=false -Denable_examples=false -Denable_docs=false >/dev/null
  ninja -C build -j "$JOBS" install >/dev/null
)
if [[ "$OS" = macos ]]; then
  # before FFmpeg links against it, so FFmpeg records the @rpath name
  for f in "$STAGE"/lib/libdav1d.*.dylib; do
    [[ -L "$f" ]] || install_name_tool -id "@rpath/$(basename "$(otool -D "$f" | tail -1)")" "$f"
  done
fi

# What camera and phone videos are made of. Decoders: H.264, HEVC, AV1 (dav1d), VP8/VP9,
# MJPEG (compact cameras), MPEG-4 Part 2 / H.263 (older phones, 3GP), MPEG-1/2 (DVD-style camcorders),
# DV, ProRes; sound: AAC, AC-3/E-AC-3 (camcorders), MP3/MP2, Opus, Vorbis, FLAC, ALAC, AMR (3GP) and PCM.
DECODERS=h264,hevc,libdav1d,vp8,vp9,mjpeg,mpeg4,h263,mpeg1video,mpeg2video,dvvideo,prores
DECODERS+=,aac,aac_latm,ac3,eac3,mp3,mp3float,mp2,mp2float,opus,vorbis,flac,alac,amrnb,amrwb
DECODERS+=,pcm_s16le,pcm_s16be,pcm_s24le,pcm_s24be,pcm_s32le,pcm_f32le,pcm_u8,pcm_alaw,pcm_mulaw
DECODERS+=,adpcm_ima_qt,adpcm_ima_wav,pcm_dvd,pcm_bluray
DEMUXERS=mov,matroska,avi,mpegts,mpegps,dv,flv,ogg
PARSERS=h264,hevc,av1,vp8,vp9,mjpeg,mpeg4video,mpegvideo,h263,dvbsub,aac,aac_latm,ac3,mpegaudio,opus,vorbis,flac
# copies cut between the in and out points, without decoding (MP4/MOV, Matroska)
MUXERS=mov,mp4,matroska
BSFS=h264_mp4toannexb,hevc_mp4toannexb,aac_adtstoasc,extract_extradata

echo "==> FFmpeg $FFMPEG_VERSION"
(
  cd "$WORK/ffmpeg-$FFMPEG_VERSION"
  PKG_CONFIG_PATH="$STAGE/lib/pkgconfig" ./configure \
    --prefix="$STAGE" --libdir="$STAGE/lib" \
    --enable-shared --disable-static --enable-pic \
    --disable-gpl --disable-nonfree \
    --disable-programs --disable-doc --disable-debug \
    --disable-avdevice --disable-avfilter --disable-network --disable-autodetect \
    --disable-everything \
    --enable-protocol=file \
    --enable-libdav1d \
    --enable-decoder="$DECODERS" --enable-demuxer="$DEMUXERS" --enable-parser="$PARSERS" \
    --enable-muxer="$MUXERS" --enable-bsf="$BSFS" \
    --enable-swscale --enable-swresample \
    ${CONFIGURE_EXTRA[@]+"${CONFIGURE_EXTRA[@]}"} \
    --extra-ldflags="$RPATH_FLAGS" --extra-libs="$EXTRA_LIBS" >"$WORK/configure.log" \
    || { tail -20 ffbuild/config.log >&2; exit 1; }
  # make expands it twice ($$$$ -> $), the single quotes keep the shell from expanding it
  [[ "$OS" != linux ]] || sed -i "s|^LDFLAGS=|LDFLAGS= -Wl,-rpath,'\$\$\$\$ORIGIN'|" ffbuild/config.mak
  grep -q '^#define HAVE_X86ASM 1' config.h || [[ "$ARCH" != x86_64 ]] || { echo "error: FFmpeg was configured without assembly" >&2; exit 1; }
  make -j "$JOBS" >/dev/null
  make install >/dev/null
)
# No hardware decoding in this build, so the headers of the hardware frame types are of no use.
# Left in, ffmpeg-sys-next's bindgen reads each one whose system SDK header it finds (on Windows
# d3d11.h, hence all of windows.h, whose compiler intrinsics break the bindings).
rm -f "$STAGE"/include/libavutil/hwcontext_?*.h
if [[ "$STAGE" != "$PREFIX" ]]; then
  mkdir -p "$PREFIX"
  cp -R "$STAGE"/. "$PREFIX"/
fi
# FFmpeg puts the import libraries for MSVC's linker (avcodec.lib, …) beside the DLLs; with
# FFMPEG_DIR (packaging/windows/package.ps1), ffmpeg-sys-next looks for them in lib/
[[ "$OS" != windows ]] || mv "$PREFIX"/bin/*.lib "$PREFIX"/lib/
# the pkg-config files name the folder they are in, not where they were built (on Windows that
# is also written as C:/msys64/…)
STAGE_WIN="$(cygpath -m "$STAGE" 2>/dev/null || echo "$STAGE")"
for pc in "$PREFIX"/lib/pkgconfig/*.pc; do
  sed -e "s|^prefix=.*|prefix=\${pcfiledir}/../..|" -e "s|$STAGE_WIN|\${prefix}|g" -e "s|$STAGE|\${prefix}|g" "$pc" >"$pc.new"
  mv "$pc.new" "$pc"
done

# Check what the libraries ask for when they are loaded.
case "$OS" in
  linux)
    readelf -d "$PREFIX/lib/libavformat.so" | grep -q 'R.*PATH.*\[\$ORIGIN\]' \
      || { echo "error: the libraries don't look for each other in their own folder (rpath)" >&2; exit 1; }
    ;;
  macos)
    for f in "$PREFIX"/lib/*.dylib; do
      [[ -L "$f" ]] && continue
      otool -D "$f" | tail -1 | grep -q '^@rpath/' || { echo "error: $f is not named @rpath/…" >&2; exit 1; }
      # every library it needs: the system's or one of these, by @rpath
      if otool -L "$f" | tail -n +2 | awk '{print $1}' | grep -vE '^(@rpath/|/usr/lib/|/System/Library/)'; then
        echo "error: $f needs the libraries above, which are not on every Mac" >&2; exit 1
      fi
      otool -l "$f" | grep -A2 LC_RPATH | grep -q '@loader_path' || [[ "$f" == *dav1d* ]] \
        || { echo "error: $f doesn't look for the others in its own folder" >&2; exit 1; }
      otool -l "$f" | grep -A4 LC_BUILD_VERSION | grep -q "minos $MACOS_MIN" \
        || { echo "error: $f is not built for macOS $MACOS_MIN" >&2; exit 1; }
    done
    ;;
  windows)
    for f in "$PREFIX"/bin/*.dll; do
      # every DLL it needs: Windows' own (the C runtime is part of Windows 10 and later) or one of these
      if objdump -p "$f" | sed -n 's/^\s*DLL Name: //p' \
        | grep -viE '^(kernel32|user32|ole32|bcrypt|advapi32|shell32|api-ms-win-.*|av[a-z]+-[0-9]+|sw[a-z]+-[0-9]+|libdav1d)\.dll$'; then
        echo "error: $f needs the DLLs above, which are not part of Windows" >&2; exit 1
      fi
    done
    ;;
esac
rm -rf "$WORK"
# the licence texts go with the libraries
mkdir -p "$PREFIX/share/licenses"
tar -xOf "$SRC/ffmpeg-$FFMPEG_VERSION.tar.xz" "ffmpeg-$FFMPEG_VERSION/COPYING.LGPLv2.1" >"$PREFIX/share/licenses/FFmpeg-COPYING.LGPLv2.1"
tar -xOf "$SRC/dav1d-$DAV1D_VERSION.tar.xz" "dav1d-$DAV1D_VERSION/COPYING" >"$PREFIX/share/licenses/dav1d-COPYING"

LIBS="$PREFIX/lib"
[[ "$OS" != windows ]] || LIBS="$PREFIX/bin"
echo "==> done: $PREFIX ($(du -sh "$LIBS" | cut -f1) of libraries)"

if [[ "$ARCHIVE" = 1 ]]; then
  NAME="albumcraft-ffmpeg-$OS-$ARCH"
  OUT="$ROOT/dist/ffmpeg"
  mkdir -p "$OUT"
  # the folder as <os>-<arch>/, to unpack into .tools/ffmpeg/
  tar -czf "$OUT/$NAME.tar.gz" -C "$(dirname "$PREFIX")" "$(basename "$PREFIX")"
  echo "$(sha256 "$OUT/$NAME.tar.gz")  $NAME.tar.gz" >"$OUT/$NAME.tar.gz.sha256"
  echo "==> packed: $OUT/$NAME.tar.gz"
  cat "$OUT/$NAME.tar.gz.sha256"
fi

TARGET="$(rustc -vV 2>/dev/null | sed -n 's/^host: //p' || true)"
if [[ -n "$TARGET" ]]; then
  echo "For cargo, in .cargo/config.toml:"
  echo "  [env]"
  echo "  PKG_CONFIG_PATH_${TARGET//-/_} = { value = \"${PREFIX#"$ROOT/"}/lib/pkgconfig\", relative = true }"
fi
