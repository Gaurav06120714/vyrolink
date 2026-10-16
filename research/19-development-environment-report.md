# Document 19 — Development Environment Report

**Date:** 2026-10-14 · **Scope:** research, audit, selection, installation and verification of development dependencies **only**.

No application code, project scaffolding, UI, protocol, cryptography or prototype was created. Builds on Docs 00–18 (Doc 18 is the authoritative blueprint).

> Installation results and verification are in §8–§10. Everything said to work below was verified by running the tool on this Mac. Items marked *pending* or *manual* were **not** verified.

---

## 1. Inputs reviewed

- **Research:** `00-INDEX`, `01`, `14`, `15`, `16`, `17`, `18`, plus the dependency, ADR and risk sections of `05`, `06`, `07`, `10`, `12`, `13`.
- **Doc 19 did not exist before this task.** This file is new; no research file was modified except the index (§13).
- **Starting point:** Doc 18 §12 (dependency specification) and ADR-013…019. These were re-validated against current sources (§3–§5), not assumed.

## 2. Pre-installation audit (verified on 2026-10-14)

| Item | Finding |
|---|---|
| Machine | Apple **M2 Pro** (10 cores), **16 GB** RAM, arm64, Rosetta 2 **not installed** |
| macOS | **26.6.2** (25G83) |
| Disk | Data volume 460 GiB; **130 GiB free** (136,491,764 KB) before installs |
| Xcode | **27.0** (27A266a), iOS SDK 27.0, macOS SDK 27.0, **Swift 6.4**; first-launch tasks done (`-checkFirstLaunchStatus` = 0); licence accepted (`-license check` = 0) |
| Command Line Tools package | 26.5 receipt (Xcode's own toolchain is selected via `xcode-select`) |
| iOS Simulator runtimes | **None installed** |
| Code-signing identities | 1 valid identity present (details not printed) |
| Homebrew | 7.0.9, prefix `/opt/homebrew` |
| Git | 2.55.0 |
| CocoaPods | 1.17.0 (prints a UTF-8 locale warning; unchanged) |
| Python / Node / pnpm / Ruby | 3.12.13 (pyenv) / 24.18.0 (nvm) / 11.22.0 / 4.0.7 |
| clang / make / perl | Apple clang 21.0.0 / GNU Make 3.81 / system perl (needed by vendored OpenSSL builds later) |
| Rust, rustup, cargo | **Missing** |
| Flutter, Dart | **Missing** |
| Java / javac | **Missing** (macOS stub only) |
| Android Studio, SDK, adb, sdkmanager, Gradle, Kotlin | **Missing** (no `~/Library/Android`, no `~/.gradle`) |
| CMake, Ninja, LLVM (Homebrew), protoc, Go | Missing (not required now; §6) |
| Environment | No `JAVA_HOME`, `ANDROID_HOME`, `CARGO_HOME`; `~/.zshrc` contained only nvm/pyenv/Antigravity lines; `~/.zprofile` runs `brew shellenv` |
| Devices | **iPhone17,4, iOS 27.0, paired with Xcode but "unavailable" (offline)**. No Android device (no adb to enumerate; none on USB). No Nordic/BLE sniffer hardware |
| Homebrew cache | 331 MB |

## 3. Technology research and re-validation (sources at the end of this section)

### 3.1 Cross-platform framework: is ADR-013 still the best choice?

| Option | Current evidence (2026) | Fit for VYRO LINK | Verdict |
|---|---|---|---|
| **Flutter 3.47 (UI) + native hosts + Rust core** (ADR-013) | Flutter 3.47 is stable (12 Aug 2026), Dart 3.13; Homebrew ships 3.47.7. SwiftPM became the default iOS dependency manager in 3.44; CocoaPods is in maintenance mode and its registry becomes read-only on 2 Dec 2026 [F1–F3]. Flutter 3.47.7 source: Java ≥ 17, AGP ≤ 9.2, Gradle ≤ 9.3.1, default NDK 28.2.13676358 [F4] | UI only; background is native + Rust (unchanged rationale) | **Keep** |
| Kotlin Multiplatform + Compose Multiplatform | Compose for iOS has been stable since CMP 1.8.0 (May 2025) [K1]. Kable offers coroutine-based BLE on Android/iOS/macOS [K2]. Swift↔Kotlin interop friction is reported for native protocol code [K3] | Viable for shared Kotlin logic + UI, but **no audited Olm/Signal-class library in Kotlin/Native**; crypto would need native wrappers on each side | Not better for this project (crypto ecosystem) |
| React Native (New Architecture) | Active (MIT). BLE via `react-native-ble-plx` (last push 2026-03) | JS runtime in background wakes is a worse fit than the native + Rust design; same native modules still needed | Reject |
| Fully native UIs (SwiftUI + Compose) + Rust core | Mature | Remains the **fallback** (ADR-013) | Fallback only |

### 3.2 Rust ↔ Swift/Kotlin bindings

| Option | Evidence | Verdict |
|---|---|---|
| **UniFFI 0.32.2** (MPL-2.0; 2026-09-23 on crates.io) | Swift + Kotlin; foreign-implemented traits supported; **async trait methods (Rust- and foreign-implemented) since 0.27.0**; 0.32.0 adds zero-copy `&[u8]`; Mozilla production use [U1–U3] | **Keep.** This resolves the earlier open question (R-30): async foreign traits are supported per the changelog, still to be confirmed in a Phase 1 spike |
| BoltFFI 0.31.1 (MIT) | Self-reported benchmarks claim large per-call speedups over UniFFI; young project (≈ 86k downloads) [B1] | **Watch only.** BLE frame rates are low; FFI overhead is not the bottleneck. Benchmarks unverified |
| swift-bridge (Apache-2.0) | Swift only (active) | Reject (Kotlin side needs another tool) |
| Diplomat | C/C++-oriented | Reject |
| Hand-written C ABI (cbindgen) | Maximum control, maximum boilerplate | Reject (more unsafe surface) |

### 3.3 Rust toolchain

**Rust 1.99.0** is current stable (released 2026-10-01; the toolchain reports a build date of 2026-09-28) [R1]. This satisfies vodozemac's MSRV of 1.96 (Doc 12 S85).

**Targets needed:**
- iOS: `aarch64-apple-ios`, `aarch64-apple-ios-sim` (Apple silicon simulator).
- Android: `aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android` (emulator).
- `x86_64-apple-ios` is unnecessary on Apple silicon.

### 3.4 Android toolchain

- **AGP 9.1.1+** is required for compileSdk **37** (minimum Android Studio Panda 3 / 2025.3.3 P1); **JDK 17+** [A1].
- The Homebrew cask provides Android Studio **2026.2.1.8**, newer than the "Quail 2" announced in July 2026 [A2]. It bundles a JetBrains Runtime (JBR), which serves as the JDK, so **no separate JDK is installed** (avoids duplication and a `sudo` pkg installer).
- **NDK:** latest LTS is r27d, latest stable r29, r30 in beta [A3]. Flutter 3.47.7 pins **28.2.13676358** [F4]. Installing exactly that version avoids a second NDK download when Flutter builds; cargo-ndk works with any modern NDK.
- **Flutter template defaults:** compileSdk/targetSdk 36, minSdk 24 [F4]. **VYRO LINK will set compileSdk/targetSdk 37 and minSdk 31** (ADR-016) when the project is created (not now).

### 3.5 Storage

| Option | Evidence | Verdict |
|---|---|---|
| **rusqlite + SQLCipher** (`bundled-sqlcipher[-vendored-openssl]`) | Features verified in rusqlite's Cargo.toml (Doc 12 S82); SQLCipher BSD-3 community edition; no published independent audit; DoS CVEs fixed (S83) | **Keep** (ADR-018) |
| SQLite3MultipleCiphers (MIT) | ChaCha20-Poly1305 default cipher; reads SQLCipher formats [D1] | **Alternative to evaluate in Phase 4.** No built-in rusqlite feature, so custom linking is needed |
| redb (Apache-2.0, pure Rust KV) | Active; no built-in encryption | Reject (encryption would need custom design) |

### 3.6 Serialization

| Option | Notes | Verdict |
|---|---|---|
| **Fixed binary header + TLV** (ADR-017) | Smallest overhead, trivially bounds-checked | **Keep** |
| postcard (Apache-2.0) | Compact serde format | Possible for *inner* payloads only if needed |
| FlatBuffers / Cap'n Proto | Zero-copy, larger generated surface | Reject for the MVP |
| Protobuf / CBOR | General-purpose; parser surface | Reject (ADR-017) |
| Compression (any) | Decompression-bomb history in mesh apps [S28, S29] | **Reject** |

### 3.7 Algorithms (selection unchanged; evidence summarised)

| Problem | Candidates | Recommendation | Evidence class | Validation still needed |
|---|---|---|---|---|
| Routing | Direct; epidemic [S41]; spray-and-wait [S40]; PRoPHET [S39]; utility-based (MaxProp, RAPID: literature, not re-read) | **Direct + binary spray-and-wait + expiry + vaccines** (ADR-003): bounded copies, no destination knowledge needed (privacy), and bitchat converged on the same [S27] | Published literature; no local measurements | E9 simulation, Exp B |
| Discovery | Continuous scan; duty-cycled filtered scan; iOS pending connects; rotating app identifiers | **Filtered (service-UUID) scans with duty cycling; iOS wake-on-proximity pending connects; no app identifiers in adverts** | Apple/Android docs (A); bitchat code (B) | Exp A, C |
| Delivery | ARQ per hop; custody + E2E receipts; dedup sets; vaccines | **Per-envelope custody + E2E authenticated receipts + SeenSet + vaccines** (Doc 18 §7) | Design (D) | Exp B |
| Crypto | Olm (vodozemac), libsignal (AGPL), Noise one-way seals, MLS | **Olm in a ChaCha20-Poly1305 wrap; Noise NN links** (ADR-007/008/014) | Audits (B) | P9 review |
| Storage | SQLCipher, SQLite3MC, KV stores | SQLCipher (ADR-018) | Vendor docs | Phase 4 build tests |
| Performance techniques | Batching, bounded queues, connection reuse, index-driven GC | Per Doc 18 §9.3 / §15 | Theoretical | Phase 4 benchmarks |

**No benchmark numbers were produced or invented in this task.**

### 3.8 Similar projects (status re-checked)

Status is unchanged from Doc 03 / Doc 12, re-checked on 2026-10-14:
- bitchat: iOS Unlicense, Android GPL-3.0, active.
- Briar: GPL-3.0, active.
- Meshtastic: GPL-3.0, active.
- Bridgefy: proprietary.

Apple Find My remains **research inspiration only** (Doc 02 §6). It is not a transport.

### 3.9 Sources (accessed 2026-10-14)

- **[F1]** Flutter "What's new" / 3.47 release notes: https://docs.flutter.dev/release/release-notes/release-notes-3.47.0 ; https://flutter.dev/blog/whats-new-in-flutter-3-47
- **[F2]** "Saying goodbye to CocoaPods": https://flutter.dev/blog/saying-goodbye-to-cocoapods-swift-package-manager-is-soon-the-default-in-flutter
- **[F3]** SwiftPM for app developers: https://docs.flutter.dev/packages-and-plugins/swift-package-manager/for-app-developers
- **[F4]** Flutter 3.47.7 source: `packages/flutter_tools/lib/src/android/gradle_utils.dart` and `packages/flutter_tools/gradle/src/main/kotlin/FlutterExtension.kt` (read via GitHub API at tag 3.47.7)
- **[F5]** Supported platforms: https://docs.flutter.dev/release/release-notes/supported-platforms (Android API 24–37, iOS 15–27 supported in 3.47)
- **[K1]** Compose Multiplatform 1.8.0 (iOS stable): https://blog.jetbrains.com/kotlin/2025/05/compose-multiplatform-1-8-0-released-compose-multiplatform-for-ios-is-stable-and-production-ready/
- **[K2]** Kable: https://github.com/JuulLabs/kable (Apache-2.0, active)
- **[K3]** KMP BLE/Swift interop talk abstract: https://sessionize.com/s/soumen-pal/cross-platform-hardware-in-kmp-ble-workers-and-nat/182349
- **[U1]** UniFFI CHANGELOG (0.27.0 async traits; 0.32.x): https://github.com/mozilla/uniffi-rs/blob/main/CHANGELOG.md
- **[U2]** UniFFI interfaces/foreign traits: https://mozilla.github.io/uniffi-rs/latest/types/interfaces.html
- **[U3]** crates.io `uniffi` 0.32.2: https://crates.io/crates/uniffi
- **[B1]** BoltFFI: https://github.com/boltffi/boltffi ; https://docs.rs/boltffi_cli
- **[R1]** Rust release index: https://blog.rust-lang.org/releases/latest (→ 1.99.0)
- **[A1]** AGP 9.1 release notes and compatibility: https://developer.android.com/build/releases/agp-9-1-0-release-notes ; https://developer.android.com/build/jdks
- **[A2]** Android Studio Quail 2 stable (2026-07-14): https://androidstudio.googleblog.com/2026/07/android-studio-quail-2-now-available.html
- **[A3]** NDK downloads / revision history: https://developer.android.com/ndk/downloads ; https://developer.android.com/ndk/downloads/revision_history
- **[D1]** SQLite3MultipleCiphers: https://github.com/utelle/SQLite3MultipleCiphers ; https://turso.tech/blog/sqlite-encryption-what-developers-actually-use
- **Crate/tool versions:** crates.io API (cargo-ndk 4.1.2, uniffi 0.32.2, cargo-fuzz 0.13.2, boltffi 0.31.1); Homebrew formula/cask metadata (rustup 1.29.1, cargo-nextest 0.9.148, cargo-deny 0.20.2, cargo-audit 0.22.2, osv-scanner 2.7.0, flutter 3.47.7, android-studio 2026.2.1.8, android-commandlinetools 15859902)
- **Earlier evidence reused:** Doc 12 S01–S85 (platform docs, audits, advisories)

## 4. Dependency matrix

| Dependency | Class | Why it is needed | Chosen version | Status |
|---|---|---|---|---|
| Xcode + iOS SDK | Required | iOS builds, Swift, simulator, signing | 27.0 / iOS SDK 27.0 | Pre-installed |
| rustup + Rust stable | Required | Shared core (ADR-013) | rustup 1.29.1, Rust 1.99.0 | **Installed** |
| Rust iOS targets | Required | `aarch64-apple-ios`, `aarch64-apple-ios-sim` | with 1.99.0 | **Installed** |
| Rust Android targets | Required | `aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android` | with 1.99.0 | **Installed** |
| Rust nightly (minimal) | Required for fuzzing only | cargo-fuzz needs `-Z` sanitizer flags | 1.101.0-nightly 2026-10-14 | **Installed** |
| cargo-fuzz | Required (security testing) | Fuzz every decoder (Doc 17 T-SEC) | 0.13.2 | **Installed** |
| cargo-deny | Required (supply chain) | Licence, advisory and source policy | 0.20.2 | **Installed** |
| cargo-audit | Required (supply chain) | RustSec advisories | 0.22.2 | **Installed** |
| osv-scanner | Recommended | Second advisory source (OSV) | 2.7.0 | **Installed** |
| cargo-nextest | Optional | Faster test runner | 0.9.148 | **Installed** (not required by any script) |
| cargo-ndk | Required (Android) | Builds Rust for the NDK ABIs | 4.1.2 | **Installed** |
| UniFFI | Required (crate, not a tool) | Swift/Kotlin bindings | 0.32.2 | Cargo dependency (pinned) |
| Flutter / Dart | Required later (Phase 8) | UI (ADR-013) | 3.47.7 / 3.13.5 | **Installed**; analytics disabled |
| Android Studio (bundled JBR) | Required (Android) | IDE, Gradle, emulator, JDK 25 | 2026.2.1.8 / OpenJDK 25.0.3 | **Installed** |
| Android cmdline-tools | Required (Android) | `sdkmanager` | 15859902 (Homebrew bootstrap) | **Installed** |
| Android SDK platform 37, build-tools, platform-tools | Required (Android) | compileSdk 37 (ADR-016), adb | latest | **Pending**: licence acceptance (§10) |
| Android NDK | Required (Android) | Native Rust builds; SQLCipher vendored OpenSSL | 28.2.13676358 (Flutter's pin) | **Pending**: licence acceptance |
| CocoaPods | Not required | Flutter 3.44+ defaults to SwiftPM | 1.17.0 | Pre-installed, unused |
| perl, make, clang | Required (build) | Vendored OpenSSL/SQLCipher builds | system | Pre-installed |

## 5. Version selection rules

- **Latest stable, pinned.** Tools are pinned to the exact version verified here. Crates use exact `=` pins in `Cargo.toml` plus `Cargo.lock`.
- **No floating versions in CI.** `.github/workflows/core.yml` pins Rust 1.99.0, cargo-audit 0.22.2 and cargo-fuzz 0.13.2.
- **MSRV correction.** §3.3 quoted vodozemac's MSRV as 1.96, which applies to its git head. The crates.io release actually used (0.11.1) declares `rust-version = "1.89"`, so the workspace MSRV is 1.89. The toolchain in use is 1.99.0 either way.

## 6. Considered and not installed

| Item | Reason |
|---|---|
| Separate JDK (Temurin/Zulu) | Android Studio's bundled JBR (OpenJDK 25.0.3) suffices; avoids a `sudo` pkg installer |
| CMake, Ninja, Homebrew LLVM | Not needed by any current build (rusqlite/SQLCipher builds with `cc`) |
| protoc, Go | No protobuf (ADR-017); no Go tooling |
| iOS Simulator runtime | Not needed until Phase 7. Large download (~8 GB); install from Xcode → Settings → Components when needed |
| Wireshark, nRF Sniffer | Only useful with sniffer hardware, which is not present (Doc 13) |
| BoltFFI, swift-bridge | Rejected in §3.2 |
| `x86_64-apple-ios` target | Not needed on Apple silicon |
| Rosetta 2 | Not needed by any tool here |

## 7. Installation method

All installs are user-level (Homebrew prefix `/opt/homebrew`, `~/.rustup`, `~/.cargo`, `/Applications`). **No `sudo` was used.**

```bash
brew install rustup cargo-nextest cargo-deny cargo-audit osv-scanner
rustup toolchain install stable && rustup default stable
rustup target add aarch64-apple-ios aarch64-apple-ios-sim aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
rustup toolchain install nightly --profile minimal
cargo install cargo-ndk --version 4.1.2 --locked
cargo install cargo-fuzz --version 0.13.2 --locked
brew install --cask flutter android-studio android-commandlinetools
flutter --disable-analytics && dart --disable-analytics
```

## 8. Installation results

| Item | Result |
|---|---|
| Rust 1.99.0 stable + rustfmt + clippy | OK: `rustc --version` → 1.99.0 |
| Nightly 1.101.0 | OK; used only via `cargo +nightly fuzz` |
| 6 Rust targets | OK: `rustup target list --installed` lists aarch64-apple-darwin + the 5 cross targets |
| cargo tools | OK. Versions as in §4, each verified with `--version` |
| Flutter 3.47.7 / Dart 3.13.5 | OK. `flutter doctor` (re-run 2026-10-15) reports 2 expected issues: no Android SDK, and no iOS Simulator runtimes (§6). It also says the iPhone must be unlocked and in Developer Mode |
| Android Studio 2026.2 | OK; JBR `java -version` → OpenJDK 25.0.3 |
| `sdkmanager` | Runs. The licence prompt (8 of 8 SDK package licences not accepted) **was left for you to answer**; nothing was accepted on your behalf |
| `~/Library/Android/sdk` | Exists and is **empty** (no packages yet) |

## 9. Verification (after installation, on real code)

The tools were verified by using them on the Phase 1–6 core (Doc 20 §3), not just by printing versions:

| Tool | Verified by |
|---|---|
| rustc/cargo | 72 tests pass |
| clippy / rustfmt | 0 warnings; formatting clean |
| cargo-deny / cargo-audit / osv-scanner | All pass on `core/Cargo.lock` (171 packages) |
| cargo-fuzz + nightly | 3 targets, 120 s each, ~59 M total runs, 0 crashes |
| Xcode / swiftc | Swift interop test 11/11; iOS device and simulator static libraries build |
| UniFFI | Swift and Kotlin bindings generated |
| cargo-ndk | **Not exercised**: needs the NDK (§10) |
| Flutter | Version and doctor only; no Flutter project exists yet (Phase 8) |

## 10. Manual actions remaining

1. **Android SDK licences.** Your decision, so they were not accepted for you. Then install the packages. Exact commands: `SETUP.md` §7.
2. After that, configure Flutter:

   ```bash
   flutter config --android-sdk ~/Library/Android/sdk
   flutter doctor -v
   ```

3. Remove the Homebrew bootstrap cmdline-tools once `cmdline-tools;latest` is installed into the SDK (`brew uninstall --cask android-commandlinetools`).
4. iPhone: connect, unlock, enable Developer Mode (Settings → Privacy & Security), and trust this Mac.
5. Android hardware: none present (Doc 20 §8).

## 11. Shell and configuration changes

- `~/.zshrc` was backed up to `~/.zshrc.vyrolink-backup-20261014`. One marked block was appended (exact content in `SETUP.md` §3): rustup and cargo on PATH, `ANDROID_HOME`, SDK tools on PATH, `JAVA_HOME` set to the Android Studio JBR. To revert, delete the block between the `>>> VYRO LINK` and `<<< VYRO LINK` markers.
- Flutter and Dart analytics are disabled (in their own config files under `~`).
- No other system settings, launch agents or global git configuration were changed.

## 12. Disk impact

| Item | Size |
|---|---|
| Free before | 130 GiB (136,491,764 KB) |
| Free on 2026-10-15 | ~106 GiB (111,051,836 KB), including the core's build outputs |
| Flutter | 4.1 GB |
| Android Studio | 3.3 GB |
| `~/.rustup` | 2.5 GB |
| `~/.cargo` | 0.5 GB |
| cmdline-tools | 0.2 GB |
| `core/target` + `core/fuzz/target` | 4.1 GB (rebuildable; `cargo clean`) |
| Homebrew cache | 4.0 GB, reclaimable with `brew cleanup` (Flutter zip 2.26 GB, Android Studio dmg 1.51 GB) |

Expected additional size once the Android SDK and NDK are installed: about 3–4 GB (an estimate, not measured).

## 13. Index and follow-up

- `00-INDEX.md` now lists Docs 19–22.
- **This document's scope was environment only.** Application-core development started afterwards under the master development prompt. It is tracked in Doc 20, and `SETUP.md` is now the single source of truth for setup.
