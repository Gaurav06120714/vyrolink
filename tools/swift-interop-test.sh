#!/usr/bin/env bash
# Builds the Rust core for the macOS host, generates Swift bindings with UniFFI,
# compiles core/bindings-tests/swift/main.swift against them and runs it.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/core"
cargo build -p vl-ffi
OUT="$ROOT/core/target/swift-bindings"
rm -rf "$OUT" && mkdir -p "$OUT"
cargo run -q -p uniffi-bindgen -- generate --library target/debug/libvl_ffi.dylib --language swift --out-dir "$OUT"
swiftc -O \
  -I "$OUT" -Xcc -fmodule-map-file="$OUT/vl_ffiFFI.modulemap" \
  "$OUT/vl_ffi.swift" "$ROOT/core/bindings-tests/swift/main.swift" \
  -L target/debug -lvl_ffi -framework Security \
  -o "$OUT/interop-test"
"$OUT/interop-test"
