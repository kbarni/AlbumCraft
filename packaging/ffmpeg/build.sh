#!/usr/bin/env bash
# Build the FFmpeg libraries AlbumCraft plays videos with: a playback-only build (decoders and
# file readers for camera and phone videos, nothing else), plus dav1d for AV1.
#
#   .tools/ffmpeg/<os>-<arch>/          include/, lib/ (shared libraries and pkg-config files)
#
# Usage: packaging/ffmpeg/build.sh [--prefix DIR] [--jobs N]
#
# Licence: LGPL 2.1 or later (FFmpeg without its GPL and non-free parts) and BSD (dav1d), both
# as shared libraries beside the program, so the MIT/Apache app may load them. Keep it that way:
# never add --enable-gpl, --enable-nonfree or --enable-version3 here.
#
# Needs a C compiler, make, pkg-config, nasm (FFmpeg's and dav1d's x86 assembly; without it
# decoding is several times slower), meson and ninja (dav1d). Downloads are checksum-pinned and
# kept in .tools/ffmpeg/src/, so building again needs no network.
#
# After building, cargo finds the libraries through PKG_CONFIG_PATH_<target> (see the line this
# script prints, for .cargo/config.toml) and the program finds them at run time through its
# rpath (build.rs).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

FFMPEG_VERSION=9.0.2
# checked against FFmpeg's release signature (ffmpeg-$FFMPEG_VERSION.tar.xz.asc, key FCF986EA15E6E293A5644F10B4322F04D67658D8)
FFMPEG_SHA256=8c3850283eb25fa026482078a04051e0be17347b09ef81a0849bec15a96e002e
DAV1D_VERSION=1.5.5
# = the published dav1d-$DAV1D_VERSION.tar.xz.sha256
DAV1D_SHA256=ca96b1d0ba3f01f55de7231d775892b64994ba061aaeeaaa4f662a390b3abf9e

case "$(uname -s)" in
  Linux) OS=linux ;;
  Darwin) OS=macos ;;
  MINGW* | MSYS* | CYGWIN*) OS=windows ;;
  *) echo "unsupported system $(uname -s)" >&2; exit 2 ;;
esac
ARCH="$(uname -m)"
[[ "$ARCH" = arm64 ]] && ARCH=aarch64
PREFIX="$ROOT/.tools/ffmpeg/$OS-$ARCH"
JOBS="$(getconf _NPROCESSORS_ONLN 2>/dev/null || echo 4)"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --prefix) PREFIX="$2"; shift 2 ;;
    --jobs) JOBS="$2"; shift 2 ;;
    -h | --help) sed -n '2,22p' "$0"; exit 0 ;;
    *) echo "unknown argument: $1" >&2; exit 2 ;;
  esac
done
SRC="$ROOT/.tools/ffmpeg/src"
WORK="$ROOT/.tools/ffmpeg/build-$OS-$ARCH"

for tool in cc make pkg-config nasm meson ninja curl; do
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
mkdir -p "$WORK" "$PREFIX"
tar -xf "$SRC/dav1d-$DAV1D_VERSION.tar.xz" -C "$WORK"
tar -xf "$SRC/ffmpeg-$FFMPEG_VERSION.tar.xz" -C "$WORK"

# The libraries find each other in their own folder (beside the program once packaged).
case "$OS" in
  # configure drops a "$ORIGIN" it is given; added to its makefile settings below instead
  linux) RPATH_FLAGS='' ;;
  macos) RPATH_FLAGS='-Wl,-rpath,@loader_path' ;;
  windows) RPATH_FLAGS='' ;;
esac

echo "==> dav1d $DAV1D_VERSION"
(
  cd "$WORK/dav1d-$DAV1D_VERSION"
  meson setup build --prefix="$PREFIX" --libdir=lib --buildtype=release --default-library=shared \
    -Denable_tools=false -Denable_tests=false -Denable_examples=false -Denable_docs=false >/dev/null
  ninja -C build -j "$JOBS" install >/dev/null
)

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
  PKG_CONFIG_PATH="$PREFIX/lib/pkgconfig" ./configure \
    --prefix="$PREFIX" --libdir="$PREFIX/lib" \
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
    --extra-ldflags="$RPATH_FLAGS" >"$WORK/configure.log"
  # make expands it twice ($$$$ -> $), the single quotes keep the shell from expanding it
  [[ "$OS" != linux ]] || sed -i "s|^LDFLAGS=|LDFLAGS= -Wl,-rpath,'\$\$\$\$ORIGIN'|" ffbuild/config.mak
  grep -q '^#define HAVE_X86ASM 1' config.h || [[ "$ARCH" != x86_64 ]] || { echo "error: FFmpeg was configured without assembly" >&2; exit 1; }
  make -j "$JOBS" >/dev/null
  make install >/dev/null
)
if [[ "$OS" = linux ]] && ! readelf -d "$PREFIX/lib/libavformat.so" | grep -q 'R.*PATH.*\[\$ORIGIN\]'; then
  echo "error: the libraries don't look for each other in their own folder (rpath)" >&2; exit 1
fi
rm -rf "$WORK"
# the licence texts go with the libraries
mkdir -p "$PREFIX/share/licenses"
tar -xOf "$SRC/ffmpeg-$FFMPEG_VERSION.tar.xz" "ffmpeg-$FFMPEG_VERSION/COPYING.LGPLv2.1" >"$PREFIX/share/licenses/FFmpeg-COPYING.LGPLv2.1"
tar -xOf "$SRC/dav1d-$DAV1D_VERSION.tar.xz" "dav1d-$DAV1D_VERSION/COPYING" >"$PREFIX/share/licenses/dav1d-COPYING"

echo "==> done: $PREFIX ($(du -sh "$PREFIX/lib" | cut -f1) of libraries)"
TARGET="$(rustc -vV 2>/dev/null | sed -n 's/^host: //p')"
if [[ -n "$TARGET" ]]; then
  echo "For cargo, in .cargo/config.toml:"
  echo "  [env]"
  echo "  PKG_CONFIG_PATH_${TARGET//-/_} = { value = \"${PREFIX#"$ROOT/"}/lib/pkgconfig\", relative = true }"
fi
