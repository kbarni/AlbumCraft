#!/usr/bin/env bash
# CI's setup step (`.github/workflows/ci.yml`, the gate's `setup-command`): on Linux, build the
# FFmpeg libraries (packaging/ffmpeg/build.sh) and point cargo at them for the next steps.
# Windows and macOS have no FFmpeg build yet: there the gate builds without the `video`
# feature (packaging/ci-gate.sh).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
[[ "$(uname -s)" = Linux ]] || exit 0
"$ROOT/packaging/ffmpeg/build.sh"
target="$(rustc -vV | sed -n 's/^host: //p')"
echo "PKG_CONFIG_PATH_${target//-/_}=$ROOT/.tools/ffmpeg/linux-$(uname -m)/lib/pkgconfig" >>"${GITHUB_ENV:-/dev/null}"
