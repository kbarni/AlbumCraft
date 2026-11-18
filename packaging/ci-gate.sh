#!/usr/bin/env bash
# The CI gate (`.github/workflows/ci.yml`): build everything and run the tests. Linux builds
# with videos (FFmpeg libraries from packaging/ffmpeg/ci-setup.sh); Windows and macOS without
# the `video` feature until their FFmpeg builds exist.
set -euo pipefail
if [[ "$(uname -s)" = Linux ]]; then
  cargo build --locked --all-targets
  cargo test --locked
else
  cargo build --locked --all-targets --no-default-features --features onnx
  cargo test --locked --no-default-features --features onnx
fi
