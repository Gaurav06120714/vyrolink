#!/usr/bin/env bash
# Runs every automated check, bottom to top, one at a time (keeps a 16 GB Mac
# responsive): Rust format/lint/tests, fuzzing, Swift interop, Flutter
# analyze/tests, the end-to-end test on the iOS simulator and the Android
# emulator, and a release-build smoke test on the emulator (catches R8/JNA
# breakage that debug builds hide). Usage: tools/run-all-checks.sh [fuzz-seconds]
# Physical phones and real Bluetooth are NOT covered here.
set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FUZZ_SECS="${1:-60}"
SIM="${SIM:-C687D5D0-C5E3-4585-9FF5-E4F2D3BDD315}"
AVD="${AVD:-VyroLink_API37}"
ADB="$HOME/Library/Android/sdk/platform-tools/adb"
export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH" CARGO_BUILD_JOBS=2
LOG="$ROOT/app/build/all-checks"
mkdir -p "$LOG"
results=()
step() { # name, command...
  local name="$1"; shift
  echo "== $name"
  if "$@" >"$LOG/$name.log" 2>&1; then results+=("PASS  $name"); else results+=("FAIL  $name  (see app/build/all-checks/$name.log)"); fi
}

cd "$ROOT/core"
step rust-fmt cargo fmt --all --check
step rust-clippy cargo clippy --all-targets --locked -- -D warnings
# The mailbox test needs a local Postgres (Homebrew: brew services start postgresql).
createdb vl_mailbox_test 2>/dev/null
step rust-tests env MAILBOX_TEST_DB="${MAILBOX_TEST_DB:-host=/tmp dbname=vl_mailbox_test}" cargo test --workspace --locked
for t in decode_all deframer link_session; do
  step "fuzz-$t" bash -c "cd fuzz && cargo +nightly fuzz run $t -- -max_total_time=$FUZZ_SECS"
done
step swift-interop "$ROOT/tools/swift-interop-test.sh"

cd "$ROOT/app"
step flutter-analyze flutter analyze
step flutter-tests flutter test

xcrun simctl boot "$SIM" >/dev/null 2>&1
step e2e-ios "$ROOT/tools/run-ios-e2e.sh" "$SIM"
xcrun simctl shutdown "$SIM" >/dev/null 2>&1

"$HOME/Library/Android/sdk/emulator/emulator" -avd "$AVD" -memory 2048 -no-audio -no-boot-anim -gpu auto >"$LOG/emulator.log" 2>&1 &
"$ADB" wait-for-device
until [ "$("$ADB" -s emulator-5554 shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" = "1" ]; do sleep 3; done
step e2e-android flutter drive --driver test_driver/integration_test.dart \
  --target integration_test/app_flow_test.dart -d emulator-5554

release_smoke() {
  flutter build apk --release --target-platform android-arm64 || return 1
  "$ADB" -s emulator-5554 install -r build/app/outputs/flutter-apk/app-release.apk || return 1
  "$ADB" -s emulator-5554 shell pm clear com.vyrolink.app
  "$ADB" -s emulator-5554 logcat -c
  "$ADB" -s emulator-5554 shell monkey -p com.vyrolink.app -c android.intent.category.LAUNCHER 1 >/dev/null
  sleep 12
  "$ADB" -s emulator-5554 shell pidof com.vyrolink.app || { echo "app is not running"; return 1; }
  if "$ADB" -s emulator-5554 logcat -d | grep -E "UnsatisfiedLinkError|FATAL EXCEPTION|Rejecting re-init" | grep -i vyrolink; then
    echo "release build failed to reach the Rust core"; return 1
  fi
  echo "release build started and reached the Rust core"
}
step android-release-smoke release_smoke
"$ADB" -s emulator-5554 emu kill >/dev/null 2>&1
(cd android && ./gradlew --stop >/dev/null 2>&1)

echo; echo "== Summary"; printf '%s\n' "${results[@]}"
! printf '%s\n' "${results[@]}" | grep -q '^FAIL'
