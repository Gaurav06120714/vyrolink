#!/usr/bin/env bash
# Runs the end-to-end integration test on an iOS simulator without the
# host-side driver (which sometimes fails to attach on iOS 27 simulators):
# builds the test app, launches it, captures periodic screenshots and reads
# the test result from the device log. Usage: tools/run-ios-e2e.sh <simulator-udid> [out-dir]
set -euo pipefail
SIM="$1"
OUT="${2:-$(cd "$(dirname "$0")/.." && pwd)/app/build/e2e}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/app"
flutter build ios --simulator --debug -t integration_test/app_flow_test.dart >/dev/null
xcrun simctl terminate "$SIM" com.vyrolink.app 2>/dev/null || true
xcrun simctl install "$SIM" build/ios/iphonesimulator/Runner.app
rm -rf "$OUT" && mkdir -p "$OUT"
START="$(date '+%Y-%m-%d %H:%M:%S')"
xcrun simctl launch "$SIM" com.vyrolink.app >/dev/null
log_lines() {
  xcrun simctl spawn "$SIM" log show --start "$START" --style compact \
    --predicate 'process == "Runner" AND eventMessage CONTAINS "flutter:"' 2>/dev/null || true
}
for i in $(seq -w 1 300); do
  xcrun simctl io "$SIM" screenshot "$OUT/frame-$i.png" >/dev/null 2>&1 || true
  if log_lines | grep -qE "All tests passed|Some tests failed"; then break; fi
  sleep 2
done
log_lines | sed 's/.*flutter: //' > "$OUT/test.log"
grep -E "All tests passed|Some tests failed|Timed out|Expected|Which" "$OUT/test.log" || echo "no result within the time limit"
grep -q "All tests passed" "$OUT/test.log"
