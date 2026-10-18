#!/usr/bin/env bash
# Builds the Rust core for the web chat (ADR-040): WebAssembly plus its
# JavaScript glue, into app/web/vlcore/ (loaded by app/web/index.html).
# Needs: rustup target wasm32-unknown-unknown, rustup component llvm-tools,
# and wasm-bindgen-cli matching the wasm-bindgen version in core/Cargo.lock.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH" CARGO_BUILD_JOBS="${CARGO_BUILD_JOBS:-2}"
# Apple's ar cannot index WebAssembly objects (SQLite is C); LLVM's can.
export AR_wasm32_unknown_unknown="$(rustc --print sysroot)/lib/rustlib/$(rustc -vV | sed -n 's/host: //p')/bin/llvm-ar"
cd "$ROOT/core"
cargo build -p vl-wasm --target wasm32-unknown-unknown --release --locked
rm -rf "$ROOT/app/web/vlcore"
wasm-bindgen --target web --no-typescript --out-dir "$ROOT/app/web/vlcore" target/wasm32-unknown-unknown/release/vl_wasm.wasm
ls -la "$ROOT/app/web/vlcore"
