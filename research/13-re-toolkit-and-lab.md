# VYRO LINK — Reverse Engineering Toolkit and Experimental Research Plan

**Research only.** Nothing listed here was installed or run. The machine was left unchanged: an earlier, separately requested install attempt was stopped and rolled back before this research began. Tool status on the MacBook (checked 2026-10-14): Xcode 27.0, Node 24.18, Python 3, jq and CocoaPods are present; Flutter, a JDK, Android Studio/adb, ipsw, radare2, Ghidra and Wireshark are **absent**.

---

## 1. Black-box vs source vs protocol vs binary analysis

| Method | What it is | When VYRO LINK needs it | Authorisation needs |
|---|---|---|---|
| **Source-code analysis** | Read public repositories at pinned commits | **Primary method.** bitchat, Briar, Meshtastic, OpenHaystack and Nearby are all open source (Docs 2–3) | Respect licences |
| **Protocol analysis** | Read specs/whitepapers; compare with captures | Primary (Noise, Signal, DTN, BLE specs; bitchat whitepaper) | None |
| **Black-box behavioural analysis** | Run apps on owned devices; observe adverts/GATT with nRF Connect; log behaviour | Validate source-derived claims, e.g. bitchat iOS↔Android interop, background behaviour | Own devices; app terms of use |
| **Authorised packet capture** | HCI logs (PacketLogger, Android btsnoop) or over-the-air sniffing (nRF Sniffer) | Measure our own and open-source apps; privacy test E10 | Own devices/traffic only; local radio-monitoring law |
| **Binary analysis** (JADX, Ghidra, Frida) | Decompile/instrument closed apps | **Generally unnecessary.** Justified only for closed apps (e.g. Bridgefy) when published research is insufficient *and* the activity is lawful in the jurisdiction (interoperability/security-research exceptions vary). Prefer published analyses [S28, S29] | Legal advice; no circumvention of DRM/FairPlay; no extraction of secrets |

**Boundaries (non-negotiable):**
- no access to Apple's private services or others' location reports;
- no breaking encryption or extracting keys from apps you do not own;
- no capture of third parties' traffic;
- no publishing of others' secrets.

## 2. Tool inventory

| Tool | Official site / repo | Purpose for VYRO LINK | OS | Prereqs | License | Maintenance (2026-10-14) | Reveals | Cannot reveal | Needs device / HW | Priority |
|---|---|---|---|---|---|---|---|---|---|---|
| **Xcode** (+ Instruments, Console) | developer.apple.com/xcode | Build test apps; device logs; energy/battery logs | macOS | Apple ID; device for BLE (Simulator has no BLE) | Apple EULA | 27.0 installed | App-level behaviour, logs, energy | Over-the-air packets | iPhone | **Required** |
| **Additional Tools for Xcode → PacketLogger** | developer.apple.com/download/all (Additional Tools) | HCI-level Bluetooth traces from Mac/iOS | macOS | Apple developer account; iOS **Bluetooth logging profile** [S59] | Apple EULA | Ships per Xcode release (not verified for 27) | HCI commands/events, ATT/GATT on the traced device | Other devices' traffic; controller-internal scheduling | iPhone + profile | **Required** |
| **Apple Bluetooth logging profile** | developer.apple.com/bug-reporting/profiles-and-logs/?name=bluetooth | Enables BT diagnostics on iOS | iOS | Install profile; follow instructions PDF (not read in this session) | Apple | Current | sysdiagnose BT logs | — | iPhone | **Required** |
| **Core Bluetooth** | developer.apple.com/documentation/corebluetooth | The platform API under test | iOS | Xcode | Apple | Current | — | — | iPhone | **Required** (API) |
| **Bluetooth Explorer** | Additional Tools for Xcode (historically) | Mac-side BLE diagnostics | macOS | — | Apple | **Availability in current Additional Tools not verified** | Mac controller info | iOS internals | Mac | Optional |
| **nRF Connect for Mobile** | nordicsemi.com (app stores) | Inspect adverts, GATT tables, act as a test central/peripheral | iOS/Android | — | Proprietary freeware | Android repo last push 2025-08 (GitHub API) | What a phone sees on air at app level | Raw link-layer packets | Phones | **Required** |
| **Android Studio + adb** | developer.android.com/studio | Build test apps; `logcat`; `dumpsys bluetooth_manager`; `batterystats`; **HCI snoop log** via Developer options → bug report | macOS/Win/Linux | JDK (bundled JBR) | Apache-2.0 (platform) / Google terms | 2026.2.1 (Homebrew metadata) | HCI on Android; battery stats | Other devices' traffic | Android phone | **Required** |
| **Wireshark** | wireshark.org; github.com/wireshark/wireshark | Open PacketLogger `.pklg`, Android `btsnoop_hci.log`, nRF Sniffer pcap; dissectors `btle`, `btatt`, `bthci_*`, `nordic_ble`, `btmesh*`, `protobuf` [S54] | macOS etc. | Homebrew cask `wireshark-app` (needs admin for capture helper) | GPL-2.0 | Active | Protocol fields | Encrypted payloads (by design) | — | **Required** |
| **nRF52840 Dongle + nRF Sniffer for Bluetooth LE** | nordicsemi.com (nRF Sniffer) | Over-the-air advert and connection capture (E10) | Wireshark extcap; Python | Dongle (~low-cost dev hardware) | Nordic licence (closed firmware) | Active | Adverts, connection setup, unencrypted link-layer traffic | Encrypted LL traffic; may lose connections; single-channel [S58] | Dongle | Recommended (E10) |
| **Git / GitHub CLI** | git-scm.com; cli.github.com | Source/commit/issue inspection | all | — | GPL-2.0 / MIT | Installed | History, releases | Unpublished code | — | **Required** (installed) |
| **Visual Studio Code** | code.visualstudio.com | Code reading, docs | all | — | MIT (OSS) / MS licence (binary) | Active | — | — | — | Optional |
| **JADX** | github.com/skylot/jadx | Decompile APKs (only for authorised analysis) | all | JDK | Apache-2.0 | Active | Java/Kotlin structure | Native code; server logic | APK | Optional (unlikely needed) |
| **Ghidra** | github.com/NationalSecurityAgency/ghidra | Native binary analysis (authorised) | all | JDK 21+ (verify) | Apache-2.0 | Active | Native code | Encrypted/obfuscated intent | Binary | Optional (unlikely needed) |
| **Frida** | github.com/frida/frida | Dynamic instrumentation of **own** test builds (e.g. fault injection) | all | Python | Custom (wxWindows-style) | Active | Runtime calls | — | Rooted/jailbroken or debuggable builds | Optional |
| **REA MCP / skills** (in this folder) | github.com/morluto/rea | Agent-driven artefact analysis (APK/JS/binaries) | macOS | Node 22.19+/24.11+ (present); engines missing | MIT | Active | Structured RE evidence | Same limits as engines | — | Optional |
| **AirGuard** | github.com/seemoo-lab/AirGuard | Tracker-detection reference (anti-tracking research) | Android | — | Apache-2.0 | Active | How tracking detection works | — | Android | Optional (privacy study) |
| **Discrete-event simulator** (own, using the portable core) | — | E9 routing | macOS | Rust toolchain if ADR-004(b) | — | — | Delivery vs parameters | Radio realism | — | **Required** (P5) |

**Application-level debugging vs over-the-air capture:**
- PacketLogger and btsnoop show traffic **as seen by one device's host stack**.
- Only a sniffer sees what *other* observers see on air.
- macOS does **not** expose arbitrary controller operations or raw air capture from the built-in radio (inference, D). Over-the-air work needs the dongle.

## 3. Mac setup for a future research phase (not executed)

| Step | Command / action | Needed for |
|---|---|---|
| 1 | Xcode 27 (present); install Additional Tools for Xcode (PacketLogger) from the Apple developer downloads | E1–E3, E10 |
| 2 | `brew install --cask android-studio` (then the SDK via its wizard; accept licences yourself) | Android test app |
| 3 | `brew install --cask wireshark-app` (needs admin password for ChmodBPF) | Capture analysis |
| 4 | nRF Sniffer: flash dongle firmware, install the Wireshark extcap per Nordic docs | E10 |
| 5 | (If ADR-004 = Rust) `rustup` stable + iOS/Android targets | P3+ |
| 6 | (P6) `brew install --cask flutter` (Flutter 3.47.7 seen in Homebrew metadata on 2026-10-14; **verify against flutter.dev release notes before pinning**) | MVP UI |
| 7 | (Optional RE) `brew install --cask temurin` or `openjdk@21`; `brew install ghidra radare2 ipsw` | Only if authorised binary analysis is approved |

## 4. Future development environment specification (requirements, not installed)

> Updated by [Doc 18 §12](18-development-blueprint.md): **Rust (≥ 1.96) is now required** (ADR-013); Flutter is UI-only; Android `minSdk 31` / `targetSdk 37` and iOS 17+ are proposed (ADR-016).

| Tool | Requirement level | Version guidance (verify at P2) | Notes |
|---|---|---|---|
| Xcode / Swift | Required | Xcode 27.x (installed) | iOS 26 SDK minimum for Wi-Fi Aware later |
| iOS deployment target | Required | Proposal: iOS 17+ (covers the DULT era); BLE APIs are old | Decide from device stats at P2 |
| Android Studio / SDK / Kotlin | Required | Android Studio 2026.2.x; compileSdk 37; minSdk proposal 29 (L2CAP CoC API) or 31 (new BT permissions) | Decide at P2 |
| JDK | Required (Android build) | Bundled JBR; JDK 21 LTS for tools | — |
| Flutter / Dart | Required (UI) | Stable channel at P6 time | UI only (ADR-004) |
| CocoaPods | Conditional | 1.17 present | Flutter's Apple dependency manager transition: **verify at P6** |
| Rust | Conditional (ADR-004 b) | stable | Core + simulator |
| Signing | Required | Apple Developer Program; Android upload key + Play App Signing | Wi-Fi Aware entitlement is managed (request) later |
| Physical devices | Required | Doc 8 §3 | Simulators lack BLE |
| Testing | Required | XCTest, Android instrumented tests, core unit/conformance tests; flutter-autonomous skill (installed in this folder) for UI flows | Multi-device orchestration is manual for now |
| Security tooling | Required before P9 | Static analysis (Semgrep/CodeQL), dependency audit (cargo-audit, OSV-scanner), MobSF optional | — |
| Docs | Required | Markdown + Mermaid (this package) | — |
| VCS | Required | Git; signed commits recommended | No repository was initialised during research |

**Developer-machine vs runtime dependencies:** runtime = vodozemac (Apache-2.0), SQLCipher, a Noise library, QR scanner, platform BLE APIs. Everything else in the table is build or test tooling.

## 5. Physical-device research laboratory

| Tier | Contents | Enables |
|---|---|---|
| **Minimum** | Apple Silicon MacBook Pro (16 GB, present) · 2 iPhones (≥ 1 iPhone 12+), iOS 27 · 2 Android phones, different OEMs · USB cables · a Faraday pouch/shielded box | E1, E2, E3, E5 baseline, E7 (3 devices), E13–E16 |
| **Recommended** | 4 iPhones (3 generations, iOS 26 & 27) · 4–6 Android (Pixel, Samsung, aggressive-battery OEM, budget) · nRF52840 dongle + sniffer · second dongle as a programmable test peripheral · USB power meter (for controlled battery tests) | All of E1–E17 |
| **Optional (Architecture D research)** | 2–3 ESP32/nRF52 boards as BLE relay prototypes; Meshtastic-compatible LoRa nodes | Dedicated-relay alternative |

## 6. Bluetooth protocol-analysis workflow (authorised)

1. Collect docs and the protocol spec/whitepaper (e.g. bitchat WHITEPAPER).
2. Read the source at a pinned commit and identify the service UUIDs, characteristics, framing and crypto (done for bitchat in Doc 3).
3. Run the app on owned devices in a controlled RF environment.
4. Observe adverts and GATT with nRF Connect (record the UUIDs, MTU and properties).
5. Capture HCI on each phone (PacketLogger / btsnoop) and optionally on air (nRF Sniffer).
6. Correlate captures with the source-derived message flow and label each field: *documented / source-derived / observed only*.
7. Identify encryption boundaries: confirm ciphertext where expected; **do not attempt to break or extract keys**.
8. Compare with standards (Noise framing, Signal ratchet) and with other implementations.
9. Write up the results, uncertainties and a reproducible procedure (device models, OS builds, commit hashes).

## 7. Research targets: study first

| Target | Why | Decision it informs |
|---|---|---|
| bitchat `WHITEPAPER.md` + `Services/BLE/*` (iOS) | The only cross-platform BLE mesh with couriers in production | ADR-001/003/011; adapter design |
| Apple TN3115, Core Bluetooth background guide, `startAdvertising` / `scanForPeripherals` docs | Hard platform rules | ADR-011; UX copy |
| Android BLE background guide, FGS types, Play FGS policy | Android backbone feasibility | R-07 |
| Signal Double Ratchet rev 4; vodozemac docs + Least Authority audit | E2E design | ADR-007 |
| Noise rev 34 (NN, X, XX properties) | Link layer; why not one-way seals | ADR-008 |
| Spray-and-Wait (2005); Epidemic (2000); PRoPHET RFC 6693 | Routing | ADR-003 |
| Briar `bramble-core/sync` | Offer/request sync semantics | L1 protocol |
| Bridgefy break papers (2021, 2022) | Failure modes to avoid | Abuse controls |
| OpenHaystack + PoPETs 2021 paper | Privacy-preserving rotating identifiers | Tag design |
| Apple Wi-Fi Aware docs + forum interop threads | Future secondary transport | Post-MVP |

## 8. Platform diagnostics (official mechanisms)

| Platform | Mechanism | Measures |
|---|---|---|
| iOS | Bluetooth logging profile + sysdiagnose; PacketLogger; Console (unified logging); Xcode Organizer energy/battery reports; MetricKit (in-app, opt-in) | HCI traces, wake events, energy |
| Android | `adb logcat`; Developer options → *Enable Bluetooth HCI snoop log* + bug report; `dumpsys bluetooth_manager`; `dumpsys batterystats` / Battery Historian; FGS timeout testing flags [S20] | HCI traces, scan throttling, battery |

**What the OS controls (cannot be measured away):** scan scheduling in the background, advert frequency in the background [S09], wake budgets, process kills, and address rotation.

## 9. Licensing and legal considerations

- **GPL/AGPL code** (Briar, Meshtastic, bitchat-android, OpenHaystack, libsignal): study freely; copying code imposes the licence.
- **flutter_blue_plus:** commercial licence required for for-profit use [S47].
- **App analysis** of closed apps: jurisdiction-specific. Use only owned devices and legally obtained apps; no DRM circumvention; get legal advice before any binary analysis.
- **Radio:** passive sniffing of *your own* devices is generally fine; capturing others' traffic may be unlawful.

**This is not legal advice.**

## 10. Known tooling limitations

- Simulators/emulators: no realistic BLE (the iOS Simulator has none; Android emulator BLE is limited).
- nRF Sniffer: single channel; may lose encrypted connections [S58].
- PacketLogger/btsnoop: host-side view only.
- Battery measurement on phones is coarse (percent deltas); controlled tests need many repetitions.

## 11. Research milestones and effort

| Stage | Deliverable | Effort (estimate) |
|---|---|---|
| Lab setup | Devices procured, tools installed, test apps skeleton | 1–2 pw |
| bitchat black-box validation | Confirm the iOS↔Android interop claim and background behaviour on our lab devices | 1 pw |
| E1/E2 | Transport feasibility report | 2–3 pw |
| E5 baseline | Battery model v0 | 1 pw |
| E10 | Privacy capture report | 1 pw |

**Feasibility criteria:** gates G1 and G3 (Doc 10).

## 12. Final research decision lists

### Install or obtain later (only what the first feasibility experiments need)

1. **2 iPhones + 2 Android phones** (different OEMs). These are the only way to resolve R-01/R-02; simulators cannot.
2. **Additional Tools for Xcode (PacketLogger) + the iOS Bluetooth logging profile.** These show what the iPhone stack actually did during background wakes (E1/E2).
3. **Android Studio + adb** (with HCI snoop log). Needed to build the Android test app and see scan throttling and FGS behaviour (E1, E2-Android, E5).
4. **Wireshark.** Needed to read every capture format above.
5. **nRF Connect for Mobile.** An independent reference central/peripheral to tell OS limits apart from our bugs.

Later and optional: nRF52840 dongle (E10), JDK/Ghidra/JADX (only if authorised closed-app analysis is approved), Flutter (P6).

### Study first

1. Apple TN3115 + Core Bluetooth background docs → decides whether iPhone relaying can be relied on.
2. bitchat whitepaper + iOS BLE source → the closest working design; decides build-vs-contribute.
3. Android BLE background + FGS + Play policy → decides the Android backbone viability.
4. Signal Double Ratchet + vodozemac audit → decides ADR-007.
5. Spray-and-Wait + PRoPHET → decides the routing parameters/approach.

### Investigate before implementation (most likely to invalidate VYRO LINK)

1. **Backgrounded/locked iPhone ↔ FGS Android session viability** (E1, E2). Resolves G1.
2. **One full link session within an iOS wake budget** (H3). Resolves whether iPhones relay at all.
3. **Multi-hop store-carry-forward with authenticated receipts across separated encounters** (E7). Resolves G2.
4. **E2E with forward secrecy under DTN conditions** (out-of-order, OTK exhaustion; P4). Resolves ADR-007.
5. **Relay privacy:** residual linkability under sniffing (E10) and malicious relays (E11). Resolves whether the differentiation (Doc 9 §6.2) is real.
