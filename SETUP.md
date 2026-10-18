# VYRO LINK — Setup (single source of truth)

**Last verified:** 2026-10-15 on an Apple M2 Pro MacBook Pro (16 GB RAM, macOS 26.6.2).

> **Status:**
> - **Done:** the shared Rust core (protocol, crypto, storage, routing, simulator, Swift/Kotlin bindings) builds and is tested.
> - **iOS app:** builds and runs on the iOS 27 simulator with the real Rust core. An end-to-end test drives onboarding, messaging with a real receipt, reactions and panic wipe (§4a).
> - **Android app:** builds and runs on an Android 37 arm64 emulator. The same end-to-end test passes there (last run 2026-10-16).
> - **Bluetooth between physical phones:** not validated (no devices). See `research/20-development-progress.md`.
> - **Not production-ready.**

## 1. What this repository contains

```
VyroLink/
├── SETUP.md                 ← this file
├── README.md
├── research/                ← research, specifications, decisions, progress (start at 00-INDEX.md)
├── app/                     ← Flutter app (UI) + packages/vl_host (native iOS/Android host plugin)
├── core/                    ← Rust workspace (shared core)
│   ├── crates/vl-proto      wire formats, framing, validation (no dependencies)
│   ├── crates/vl-crypto     pairing, wrap/tags, Olm sessions (vodozemac), Noise NN links (snow)
│   ├── crates/vl-routing    store-and-forward engine (spray-and-wait), MemStore
│   ├── crates/vl-store      SQLCipher databases (Class R relay, Class C conversation)
│   ├── crates/vl-core       Node: the API native hosts call
│   ├── crates/vl-ffi        UniFFI bindings (Swift/Kotlin)
│   ├── crates/vl-sim        routing simulator (drives the real engine)
│   ├── crates/uniffi-bindgen  host-only binding generator
│   ├── fuzz/                cargo-fuzz targets (nightly)
│   ├── vectors/             golden protocol test vectors (shared with Swift/Kotlin)
│   ├── bindings-tests/swift Swift ⇄ Rust interop check
│   └── deny.toml            dependency policy
├── tools/                   scripts (swift-interop-test.sh)
├── website/                 one-page site + privacy policy (Netlify publishes it; see netlify.toml)
├── .github/workflows/       CI for core/ (runs on GitHub on push; repo: github.com/Gaurav06120714/vyrolink)
└── references/              third-party clones for study only (git-ignored, GPL/AGPL)
```

Architecture: `research/18-development-blueprint.md`. Wire protocol: `research/21-wire-protocol-v1.md`.

## 2. Verified toolchain

| Tool | Version (verified) | Install method | Needed for |
|---|---|---|---|
| macOS | 26.6.2 (25G83), arm64 | — | — |
| Xcode | 27.0 (27A266a), iOS SDK 27.0, Swift 6.4 | App Store / developer.apple.com (pre-installed) | iOS builds, Swift interop |
| Homebrew | 7.0.9 | brew.sh (pre-installed) | package manager |
| Git | 2.55.0 | Homebrew (pre-installed) | VCS |
| rustup | 1.29.1 (keg-only) | `brew install rustup` | Rust toolchains |
| Rust stable | **1.99.0** (default) + rustfmt 1.10.0, clippy 0.1.99 | `rustup toolchain install stable` | core |
| Rust nightly | 1.101.0-nightly (2026-10-14), minimal profile | `rustup toolchain install nightly --profile minimal` | **fuzzing only** |
| Rust targets | aarch64-apple-ios, aarch64-apple-ios-sim, aarch64-linux-android, armv7-linux-androideabi, x86_64-linux-android | `rustup target add …` | cross-compilation |
| cargo-nextest | 0.9.148 | `brew install cargo-nextest` | test runner (optional) |
| cargo-deny | 0.20.2 | `brew install cargo-deny` | licence/advisory/source policy |
| cargo-audit | 0.22.2 | `brew install cargo-audit` | RustSec advisories |
| osv-scanner | 2.7.0 | `brew install osv-scanner` | OSV advisories |
| cargo-ndk | 4.1.2 | `cargo install cargo-ndk --version 4.1.2 --locked` | Android cross-builds |
| cargo-fuzz | 0.13.2 | `cargo install cargo-fuzz --version 0.13.2 --locked` | fuzzing |
| Flutter / Dart | 3.47.7 / 3.13.5 (analytics disabled; SwiftPM integration on) | `brew install --cask flutter` | the app (`app/`) |
| iOS Simulator runtime | iOS 27.0 (24A434), 7.5 GB | `xcodebuild -downloadPlatform iOS` (installed 2026-10-15 with your approval) | running the app without an iPhone |
| Android Studio | 2026.2 (cask 2026.2.1.8) with bundled JBR **OpenJDK 25.0.3** (no separate JDK) | `brew install --cask android-studio` | Android builds |
| Android cmdline-tools | 15859902 (bootstrap copy) | `brew install --cask android-commandlinetools` | SDK installation |
| Android SDK packages | platform-tools 37.0.1, platforms 36 + 37.0, build-tools 36.0.0, NDK 28.2.13676358, emulator, system image `android-37.0;google_apis;arm64-v8a` (licences accepted by you on 2026-10-15) | `tools/android-sdk-setup.sh` | Android builds |
| CocoaPods | 1.17.0 (pre-installed; Flutter now defaults to SwiftPM) | — | not required |
| Node.js | 24.18.0 (pre-installed) | — | REA MCP server only |

Pinned Rust crate versions are in `core/Cargo.lock` (tracked in git; no commits have been made yet). Direct dependencies use exact `=` pins (see each crate's `Cargo.toml`).

## 3. Environment configuration

`~/.zshrc` contains one marked block, added 2026-10-14. The original file is backed up at `~/.zshrc.vyrolink-backup-20261014`.

```sh
# >>> VYRO LINK dev environment >>>
export PATH="/opt/homebrew/opt/rustup/bin:$HOME/.cargo/bin:$PATH"
export ANDROID_HOME="$HOME/Library/Android/sdk"
export PATH="$PATH:$ANDROID_HOME/platform-tools:$ANDROID_HOME/cmdline-tools/latest/bin"
export JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
# <<< VYRO LINK dev environment <<<
```

- Flutter and Dart telemetry are disabled (`flutter --disable-analytics`, `dart --disable-analytics`).
- Nothing else in the system configuration was changed.

## 4. Build and test (core)

```bash
cd core
cargo fmt --all --check
cargo clippy --all-targets            # must report zero warnings
cargo test                            # 137 tests: unit, property, vectors, storage, features, groups, routing, end-to-end
cargo deny check && cargo audit       # dependency policy + advisories
osv-scanner scan source --lockfile Cargo.lock
```

**Golden vectors.** If you change the wire format on purpose, regenerate the vectors and review the diff:

```bash
VL_REGEN_VECTORS=1 cargo test -p vl-proto --test vectors
```

**Fuzzing** (nightly). Each target runs for a time limit you choose:

```bash
cd core/fuzz
cargo +nightly fuzz run decode_all   -- -max_total_time=600
cargo +nightly fuzz run deframer     -- -max_total_time=600
cargo +nightly fuzz run link_session -- -max_total_time=600
```

**Routing simulation.** This regenerates `research/22-routing-simulation-results.md` and is deterministic per seed:

```bash
cd core && cargo run -q -p vl-sim --release > ../research/22-routing-simulation-results.md
```

**Algorithm benchmarks and routing experiments** (`research/algorithms/`). Run before and after any change to routing or storage hot paths, then compare:

```bash
cd core && cargo run -q --release -p vl-store --example bench_relay
cargo run -q --release -p vl-store --example bench_conversation
cargo run -q -p vl-sim --release --bin experiments > ../research/algorithms/experiments-output.md
```

**Swift interop check.** It builds the core, generates Swift bindings and runs `core/bindings-tests/swift/main.swift`:

```bash
tools/swift-interop-test.sh
```

**iOS cross-builds.** The minimum iOS version is 17 (ADR-016). Always set the deployment target, or SQLCipher fails to link:

```bash
cd core && IPHONEOS_DEPLOYMENT_TARGET=17.0 cargo build -p vl-ffi --release --target aarch64-apple-ios
IPHONEOS_DEPLOYMENT_TARGET=17.0 cargo build -p vl-ffi --release --target aarch64-apple-ios-sim
```

**Independent artifact checks with REA** (the `reverse-engineer-anything` skill, now in `~/.claude/skills/`; the CLI is pinned to the same version as the project's `.mcp.json`). Run these after dependency or FFI changes:

```bash
npx -y rea-agents@6.5.0 inspect-macho core/target/aarch64-apple-ios/release/libvl_ffi.dylib --format json
```

Expected: dependencies are only Security, CoreFoundation, libiconv and libSystem, with **no networking imports**. The `os_log` symbol is present because it is statically linked from SQLCipher; its runtime level is NONE (test `sqlcipher_device_logging_is_off`).

```bash
npx -y rea-agents@6.5.0 inspect-sqlite-database <any VYRO LINK .db file> --format json
```

Expected: `"file is not a database"`, meaning the file is encrypted. Use REA only on VYRO LINK's own artifacts or on artifacts whose licence permits analysis. Never use it on proprietary apps.

**Kotlin bindings.** Generation works now; compiling them needs the Android SDK:

```bash
cd core && cargo build -p vl-ffi && cargo run -q -p uniffi-bindgen -- generate --library target/debug/libvl_ffi.dylib --language kotlin --out-dir target/kotlin-bindings
```

**Android cross-builds.** These are **blocked** until the SDK and NDK are installed (§7). The command will then be:

```bash
cd core && cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 build -p vl-ffi --release
```

On Android, SQLCipher bundles OpenSSL (`bundled-sqlcipher-vendored-openssl`, which needs perl and make). Apple targets use CommonCrypto.

## 4a. The mobile app

**Layout.**
- `app/lib/`: the Flutter UI. It is split into:
  - `app/` (startup, services, routes, errors);
  - `design/` (tokens, theme, widgets);
  - `features/`: one folder per screen group.
- `app/packages/vl_host/`: a Flutter plugin that is the native host, with the Pigeon API in `pigeons/host_api.dart`:
  - **iOS (Swift):** `ios/vl_host/Sources/vl_host`. Contains NodeHost, KeyStore (Keychain), BleRelay (Core Bluetooth), QrScanner (AVFoundation) and LoopbackPeer (debug only).
  - **Android (Kotlin):** `android/src/main/kotlin/com/vyrolink/vl_host`. Contains NodeHost, KeyStore (Keystore-wrapped keys), BleRelay, QrScanner (CameraX + ZXing), Permissions/Notifier and LoopbackPeer.

**Dependencies.**
- **Dart:** Flutter SDK only, plus `pigeon 29.0.4` as a dev dependency. There are no runtime pub packages.
- **Android:** JNA 5.19.1, kotlinx-coroutines 1.11.0, lifecycle-process 2.10.0, CameraX 1.6.1 and ZXing core 3.5.4. All are Apache-2.0.
- **iOS:** system frameworks only.

**Build the Rust core for iOS** (needed after any change in `core/`):

```bash
tools/build-core-ios.sh debug
```

This writes `VlFFI.xcframework` and `Sources/VlCore/vl_ffi.swift` into the plugin. Use `release` for release builds.

**Regenerate the Flutter ↔ native API** after editing `pigeons/host_api.dart`:

```bash
cd app/packages/vl_host && dart run pigeon --input pigeons/host_api.dart
```

**Run on the iOS simulator:**

```bash
cd app && flutter run -d "iPhone 18 Pro"
```

Debug builds have **Settings → Developer tools**. It creates a second real node inside the app (a "loopback peer") that exchanges over memory instead of Bluetooth, so sending, receipts and incoming messages can be exercised on a simulator. Release builds refuse these calls.

**Tests:**

```bash
cd app && flutter analyze && flutter test
```

`flutter test` runs Dart unit and widget tests with a fake host, including accessibility guideline checks.

```bash
cd app && flutter drive --driver test_driver/integration_test.dart --target integration_test/app_flow_test.dart -d "iPhone 18 Pro"
```

This runs the end-to-end test on the simulator against the real core. Screenshots go to `build/screenshots/`.

If the driver hangs after "Xcode build done" (it sometimes cannot attach on iOS 27 simulators), use the driverless runner instead:

```bash
tools/run-ios-e2e.sh <simulator-udid>
```

It builds the test app, launches it, takes a screenshot every 2 s into `app/build/e2e/` and reads the result from the device log. Its exit code is 0 only when the device log says "All tests passed".

**App icons** are regenerated from one drawing:

```bash
python3 tools/generate_icons.py
```

**Build for a device** (needs signing, §7):

```bash
cd app && flutter build ios --release
```

**Android** (after §7). Build the core for the emulator only, with low memory use:

```bash
ABIS=arm64-v8a CARGO_BUILD_JOBS=2 tools/build-core-android.sh debug
```

Leave out `ABIS` for all phone ABIs.

```bash
cd app && flutter build apk --debug --target-platform android-arm64
```

`app/android/gradle.properties` caps Gradle at a 2 GB heap with 2 workers and in-process Kotlin, so a 16 GB Mac stays responsive. Stop the daemon afterwards with `cd app/android && ./gradlew --stop`.

**Emulator:** the AVD `VyroLink_API37` (Android 37, arm64, 2 GB RAM) lives in `~/.android/avd`.

```bash
~/Library/Android/sdk/emulator/emulator -avd VyroLink_API37 -memory 2048 -no-audio -no-boot-anim
```

Then install and launch:

```bash
~/Library/Android/sdk/platform-tools/adb install -r app/build/app/outputs/flutter-apk/app-debug.apk
```

```bash
~/Library/Android/sdk/platform-tools/adb shell am start -n com.vyrolink.app/.MainActivity
```

End-to-end test on the emulator (the driver attaches reliably on Android):

```bash
cd app && flutter drive --driver test_driver/integration_test.dart --target integration_test/app_flow_test.dart -d emulator-5554
```

## 4b. Google sign-in (optional, ADR-030)

The app builds and runs without this; the sign-in button then says it isn't set up. To turn it on, use Google Cloud Console (no Firebase):

1. Open https://console.cloud.google.com, create a project (e.g. "VYRO LINK"), then **APIs & Services → OAuth consent screen**:
   - choose **External**, app name, your support email;
   - privacy policy URL `https://vyrolink.netlify.app/#privacy`;
   - scopes `openid`, `email`, `profile`;
   - while testing, add your own Google account under **Test users**.
2. **Credentials → Create credentials → OAuth client ID**, three times:
   - **Web application** (no redirect URIs needed). Copy its client ID into `googleWebClientId` in `app/lib/app/google_account.dart`. Android uses it as `serverClientId`.
   - **Android**: package `com.vyrolink.app`, plus the **SHA-1** of each signing key. For the debug key:
     ```bash
     "/Applications/Android Studio.app/Contents/jbr/Contents/Home/bin/keytool" -list -v -keystore ~/.android/debug.keystore -alias androiddebugkey -storepass android -keypass android | grep SHA1
     ```
     (This Mac has no system Java; Android Studio's bundled `keytool` works.)
     Add the Play App Signing key's SHA-1 later (Play Console → App integrity).
   - **iOS**: bundle ID `com.vyrolink.app`. Copy its client ID into `googleIosClientId`. Add its **reversed client ID** (`com.googleusercontent.apps.…`) as a URL scheme in `app/ios/Runner/Info.plist` under `CFBundleURLTypes`.
3. Rebuild both apps. Client IDs are public identifiers, so committing them is fine; never commit a *client secret* (installed apps don't use one).

Before the App Store, Apple's guideline 4.8 will likely require **Sign in with Apple** next to Google sign-in. That needs a paid Apple Developer account (capability "Sign in with Apple").

## 5. Security requirements for contributors

- Never log plaintext, keys, tags, ids or message bodies. Diagnostics carry counters only.
- No network permission or internet code in any build (MVP is offline-only).
- No new cryptographic constructions. Use only the primitives listed in `research/21-wire-protocol-v1.md` §5.
- Every new decoder must cap counts and lengths before allocating, and gets a fuzz target.
- Delivery may be shown as "Delivered" only from a verified receipt or a matching vaccine secret (ADR-015, ADR-020). Never from relay custody.
- New conversation features are CONTENT events (ADR-023, Doc 21 §4.1):
  - specify them in Doc 21 first;
  - add golden vectors;
  - keep merge rules order-independent;
  - test both arrival orders.
- Do not weaken tests or silence lints to make a build pass.
- **Panic wipe integration (native hosts):**
  1. Delete both database keys from the Keychain/Keystore first.
  2. Then call `VlNode.panic_wipe()`.
  3. Then drop the object.

  The key deletion is what makes the data unrecoverable on flash storage (cryptographic erasure).

## 6. Performance and battery

There are no measured battery or BLE figures yet; Experiment C is blocked. The routing simulator reports *simulated* delivery and transfer counts only, and they are not Bluetooth measurements. Do not quote battery-life or delivery-time claims.

## 7. Manual actions still required (you)

1. **Accept the Android SDK licences.** This is your decision, so I did not do it. Run:

   ```bash
   JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home" sdkmanager --sdk_root="$HOME/Library/Android/sdk" --licenses
   ```

   Then install the packages:

   ```bash
   JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home" sdkmanager --sdk_root="$HOME/Library/Android/sdk" "platform-tools" "platforms;android-36" "platforms;android-37.0" "build-tools;36.0.0" "ndk;28.2.13676358" "cmdline-tools;latest" "emulator" "system-images;android-37.0;google_apis;arm64-v8a"
   ```

   Afterwards, run `flutter config --android-sdk "$HOME/Library/Android/sdk"` and `flutter doctor -v`.

   `tools/android-sdk-setup.sh` does all of this in one go:
   - accepting the licences stays interactive and is your decision;
   - the package IDs are the ones that exist today (`platforms;android-37.0`, not `android-37`), plus the emulator and an `android-37.0` arm64 system image.
2. **Choose the product licence** (decision U-1). No LICENSE file exists yet.
3. **Obtain an Android 16/17 phone** (blocking for Experiments A–C) and, ideally, a second iPhone.
4. **Connect and unlock the iPhone** (iPhone17,4, iOS 27.0, currently offline) with Developer Mode enabled before Phase 5.

## 8. Troubleshooting

| Symptom | Cause | Fix |
|---|---|---|
| `rustc: command not found` in a new shell | PATH block not loaded | `source ~/.zshrc`. rustup is keg-only, so its directory must be on PATH |
| iOS link error "built for newer iOS version" | Missing deployment target | Set `IPHONEOS_DEPLOYMENT_TARGET=17.0` |
| `cargo fuzz` fails with "nightly option" | Stable toolchain selected | Use `cargo +nightly fuzz …` |
| `cargo deny` reports wildcard path dependencies | Internal path crates | Already allowed (`allow-wildcard-paths`) |
| `DbError::WrongKeyOrCorrupt` | Wrong 32-byte key or damaged file | Keys come from the platform key store and are never hard-coded |
| `sdkmanager` "deprecated" warning | Google is moving to the `android sdk` CLI | Still works. Migrate later |
| `codesign` fails: "resource fork, Finder information, or similar detritus not allowed" | `~/Desktop` is synced by iCloud Drive, which tags build files | `app/build` is a symlink to `~/Library/Caches/VyroLink-build/app` (outside iCloud). If you clone afresh, recreate it: `mkdir -p ~/Library/Caches/VyroLink-build/app && ln -s ~/Library/Caches/VyroLink-build/app app/build` |
| Linker errors for `x86_64` simulator | The Rust core is built for Apple-silicon simulators only | `EXCLUDED_ARCHS[sdk=iphonesimulator*] = x86_64` is set in `app/ios/Flutter/*.xcconfig`. Intel Macs would need `rustup target add x86_64-apple-ios` and a fat simulator library |
| `flutter drive` hangs after "Xcode build done" | The driver sometimes fails to attach on iOS 27 simulators | Stop it, `xcrun simctl shutdown <id> && xcrun simctl boot <id>`, run again. Never run two `flutter drive` commands at once |

## 9. Verification checklist (run before calling any change done)

**One command runs all of it** (about 35 min, one step at a time): `tools/run-all-checks.sh [fuzz-seconds]`.
- Steps: Rust fmt/clippy/tests, 3 fuzz targets, Swift interop, Flutter analyze/tests, the iOS simulator and Android emulator end-to-end tests, and a **release-build smoke test** on the emulator. That last one catches R8/JNA breakage, which debug builds hide.
- Logs: `app/build/all-checks/`.
- It does **not** cover real phones or real Bluetooth.


- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --all-targets` reports zero warnings
- [ ] `cargo test` passes (all 111 or more tests)
- [ ] `cargo deny check` and `cargo audit` pass
- [ ] Fuzz targets run for at least 10 minutes each without crashes after protocol changes
- [ ] `tools/swift-interop-test.sh` passes after any FFI change
- [ ] `cd app && flutter analyze && flutter test` pass
- [ ] After app or host changes: `tools/build-core-ios.sh debug`, then `tools/run-ios-e2e.sh <udid>` passes
- [ ] `research/20-development-progress.md` is updated

## 10. Disk usage (measured 2026-10-15)

| Item | Size |
|---|---|
| Flutter SDK | 4.1 GB |
| Android Studio | 3.3 GB |
| Rust toolchains (`~/.rustup`) | 2.5 GB |
| `~/.cargo` | 0.5 GB |
| `core/target` | 3.8 GB (rebuildable) |
| Fuzz build | 0.3 GB (rebuildable) |
| Homebrew download cache | 4.0 GB (removable with `brew cleanup`; contains the Flutter and Android Studio installers) |

Free space went from 130 GiB to about 106 GiB.

## 11. Internet mailbox and web chat (ADR-039, ADR-040)

**Mailbox (Render):**
- Render → New → Blueprint → this repository. `render.yaml` creates the mailbox (`vyrolink-mailbox`) and its Postgres database. The web chat is served by Netlify at `/chat/`.
- Local test: `brew services start postgresql`, `createdb vl_mailbox_test`, then:
  ```bash
  MAILBOX_TEST_DB="host=/tmp dbname=vl_mailbox_test" cargo test -p vl-mailbox
  ```

**Web chat:**
- One-time: `rustup target add wasm32-unknown-unknown`, `rustup component add llvm-tools`, and `cargo install wasm-bindgen-cli --version <the wasm-bindgen version in core/Cargo.lock>`.
- Build: `VL_SCRIPTS=https://vyrolink.netlify.app/chat/ tools/build-web.sh https://vyrolink-mailbox.onrender.com --base-href /chat/`, then copy `app/build/web` to `website/chat/`. Remove `*.symbols` files and commit; Netlify deploys it.
- If Render gives the mailbox a different address, rebuild with it.
- Then set the mailbox's `WEB_ORIGIN` to the web chat's address.

**Phones:** build with `--dart-define=VL_MAILBOX=<mailbox URL>` if it is not the default.
