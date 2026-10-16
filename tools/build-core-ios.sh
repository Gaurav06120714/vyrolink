#!/usr/bin/env bash
# Builds the Rust core for iOS devices and the Apple-silicon simulator, packs it
# as VlFFI.xcframework and generates the UniFFI Swift bindings, both inside the
# vl_host plugin (app/packages/vl_host/ios/vl_host). Run before `flutter build ios`
# or `flutter run` whenever core/ changes. Usage: tools/build-core-ios.sh [debug|release]
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE="${1:-release}"
FLAG=""; [ "$PROFILE" = "release" ] && FLAG="--release"
PKG="$ROOT/app/packages/vl_host/ios/vl_host"
export IPHONEOS_DEPLOYMENT_TARGET=17.0

cd "$ROOT/core"
for target in aarch64-apple-ios aarch64-apple-ios-sim; do
  cargo build -p vl-ffi $FLAG --target "$target"
done
cargo build -p vl-ffi

GEN="$ROOT/core/target/swift-bindings-app"
rm -rf "$GEN" && mkdir -p "$GEN/headers"
cargo run -q -p uniffi-bindgen -- generate --library target/debug/libvl_ffi.dylib --language swift --out-dir "$GEN"
cp "$GEN/vl_ffiFFI.h" "$GEN/headers/"
cp "$GEN/vl_ffiFFI.modulemap" "$GEN/headers/module.modulemap"

rm -rf "$PKG/VlFFI.xcframework"
xcodebuild -create-xcframework \
  -library "target/aarch64-apple-ios/$PROFILE/libvl_ffi.a" -headers "$GEN/headers" \
  -library "target/aarch64-apple-ios-sim/$PROFILE/libvl_ffi.a" -headers "$GEN/headers" \
  -output "$PKG/VlFFI.xcframework" >/dev/null
mkdir -p "$PKG/Sources/VlCore"
cp "$GEN/vl_ffi.swift" "$PKG/Sources/VlCore/vl_ffi.swift"
echo "VlFFI.xcframework ($PROFILE) and Swift bindings written to $PKG"
