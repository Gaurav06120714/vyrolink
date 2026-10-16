#!/usr/bin/env bash
# Builds the Rust core for Android (arm64-v8a, armeabi-v7a, x86_64 for the
# emulator) into the vl_host plugin's jniLibs and generates the UniFFI Kotlin
# bindings. Needs the Android NDK (SETUP.md §7). Usage: tools/build-core-android.sh [debug|release]
# ABIS="arm64-v8a" limits the targets (the Apple-silicon emulator needs only
# arm64-v8a); CARGO_BUILD_JOBS limits parallel compilers to save memory.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE="${1:-release}"
FLAG=""; [ "$PROFILE" = "release" ] && FLAG="--release"
PLUGIN="$ROOT/app/packages/vl_host/android"
export ANDROID_NDK_HOME="${ANDROID_NDK_HOME:-$HOME/Library/Android/sdk/ndk/28.2.13676358}"

cd "$ROOT/core"
TARGETS=()
for abi in ${ABIS:-arm64-v8a armeabi-v7a x86_64}; do TARGETS+=(-t "$abi"); done
cargo ndk "${TARGETS[@]}" --platform 31 -o "$PLUGIN/src/main/jniLibs" build -p vl-ffi $FLAG
cargo build -p vl-ffi
cargo run -q -p uniffi-bindgen -- generate --library target/debug/libvl_ffi.dylib --language kotlin \
  --out-dir "$PLUGIN/src/main/kotlin"
echo "jniLibs ($PROFILE) and Kotlin bindings written to $PLUGIN"
