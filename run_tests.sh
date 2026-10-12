#!/bin/sh
# Gate G1: issue -> verify -> tamper tests against the real SBF artifact.
# Notes for this mini (macOS, cargo target-dir on external /Volumes/DATA):
#  - SBF_OUT_DIR points solana-program-test at tests/fixtures/trust_layer.so
#  - OPENSSL_NO_VENDOR avoids the vendored openssl build, which fails on the
#    external volume (AppleDouble "Directory not empty" errors).
set -e
cd "$(dirname "$0")"
export SBF_OUT_DIR="$PWD/tests/fixtures"
if [ "$(uname)" = "Darwin" ] && [ -d /opt/homebrew/opt/openssl@3 ]; then
  export OPENSSL_NO_VENDOR=1
  export OPENSSL_DIR=/opt/homebrew/opt/openssl@3
fi
exec cargo test --manifest-path tests/Cargo.toml "$@"
