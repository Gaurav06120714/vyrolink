# VYRO LINK: research notes

**Date:** 2026-10-14
**Scope:** I read every repository installed or cloned into this folder. Facts marked *(code)* come straight from the source at the commit listed. Facts marked *(background)* are general platform knowledge, not something read here; check those before relying on them.
**Status:** this is setup and research only. No design decisions have been made, because the project brief hasn't arrived yet.

---

## 1. Summary

- **82 Claude Code skills** from 9 repos are installed in this project (`~/.claude/skills/`). The project also has the **REA MCP server** (`.mcp.json`) and an **`/extract-ipa`** command. **Update 2026-10-15:** at your request the skills and `/extract-ipa` were moved to `~/.claude/skills/` and `~/.claude/commands/` (user scope); `.mcp.json` stays in the project.
- **5 reference codebases** are cloned in `references/` (shallow clones, ~530 MB total, most of it Wireshark).
- The reference set all points at one area: **off-grid, device-to-device communication and location**. That means BLE, Bluetooth/Wi-Fi sync, LoRa mesh, and Find My-style crowd finding, built as a **Flutter** app with **iOS as a first-class target**.
- **Two blockers before building:**
  1. Most of the toolchain is missing. There is no Flutter/Dart SDK, no JDK, no RE engines and no Wireshark (see §5).
  2. **Every reference codebase is GPL or AGPL.** You can study them, but don't copy their code unless VYRO LINK will itself ship under a compatible open-source licence.

---

## 2. Installed skills (`~/.claude/skills/`)

| Area | Source repo | Commit | Licence | Skills |
|---|---|---|---|---|
| Flutter (official) | [flutter/skills](https://github.com/flutter/skills) | `0ef3972` | BSD-3 | 10 `flutter-*` |
| Dart (official) | [dart-lang/skills](https://github.com/dart-lang/skills) | `b046fa1` | BSD-3 | 15 `dart-*` |
| Flutter architecture & engineering | [thiennc-tesoglobal/flutter-skills](https://github.com/thiennc-tesoglobal/flutter-skills) | `5b89c05` | BSD | 37 |
| Flutter Clean Architecture | [gabuldev/flutter-skills](https://github.com/gabuldev/flutter-skills) | `9957062` | MIT | 15 |
| Flutter testing automation | [z-chu/flutter-autonomous-skill](https://github.com/z-chu/flutter-autonomous-skill) | `73c7679` | MIT | 1 (`flutter-autonomous`) |
| Mobile UI/UX | [noumanzindani/mobile-app-uiux-pro-max-skill](https://github.com/noumanzindani/mobile-app-uiux-pro-max-skill) | `9501388` | Apache-2.0 | 1 (`mobile-uiux-pro-max`) |
| Flutter + Apple HIG | [karanb05/mobile-flutter-design-skill](https://github.com/karanb05/mobile-flutter-design-skill) | `9407123` | MIT | 1 (`mobile-flutter-design`) |
| iOS reverse engineering | [incogbyte/iOS-reverse-engineering-claude-skill](https://github.com/incogbyte/iOS-reverse-engineering-claude-skill) | `ced3523` | Unlicense | 1 (`ios-reverse-engineering`) + `/extract-ipa` |
| Reverse engineer anything | [morluto/rea](https://github.com/morluto/rea) | `1438422` (re-checked 2026-10-15 against `a1ca8c7`: skill identical, npm still 6.5.0) | MIT | 1 (`reverse-engineer-anything`) + MCP server |

### 2.1 Flutter & Dart engineering

- **flutter/skills (official):** `flutter-add-integration-test`, `-add-widget-preview`, `-add-widget-test`, `-apply-architecture-best-practices`, `-build-responsive-layout`, `-fix-layout-issues`, `-implement-json-serialization`, `-setup-declarative-routing`, `-setup-localization`, `-use-http-package`.
  - The repo also ships the 15 `dart-*` skills. They match `dart-lang/skills` exactly except `dart-use-pattern-matching`, where flutter's copy is older (23 Sep vs 13 Oct). **I installed the `dart-lang` copies.**
  - Some skills (`dart-fix-runtime-errors`, `flutter-fix-layout-issues`) expect the Dart/Flutter MCP tools. These come with the SDK (`dart mcp-server`).
- **dart-lang/skills (official):** unit tests, mockito mocks, coverage, static analysis, FFI/ffigen and native assets, pattern matching, primary constructors, docs, CLI apps, package conflicts.
  - `dart-setup-ffi-assets` and `dart-use-ffigen` matter if VYRO LINK ends up wrapping C crypto or radio libraries.
- **thiennc-tesoglobal/flutter-skills:** 37 skills. The entry point is `flutter-app-workflow`. The most relevant ones for an offline/BLE app:
  - `flutter-background-execution`
  - `flutter-platform-integration` (method channels into Swift/Kotlin)
  - `flutter-security`
  - `flutter-persistence`
  - `flutter-notifications`
  - `flutter-device-testing`
  - `flutter-runtime-debugging`
  - `flutter-performance`
- **gabuldev/flutter-skills:** 15 opinionated skills: Clean Architecture, Cubit (bloc), `flutter_injections` DI, Melos monorepo, Maestro E2E, offline-first caching.
  - Two names clashed with thiennc's, so I renamed them: **`flutter-testing-gabul`** and **`flutter-accessibility-gabul`** (both the folder and the frontmatter `name`).
- **⚠ The architecture skills overlap and disagree:**
  - Official `flutter-apply-architecture-best-practices` uses layered UI/Logic/Data.
  - thiennc's `flutter-architecture` and `flutter-state-management` say to follow whatever the project already uses.
  - gabuldev's `flutter-clean-arch` and `flutter-state` use Clean Architecture + Cubit.

  **Pick one architecture early and write it into a project `CLAUDE.md`** so the right skill wins.
- **z-chu/flutter-autonomous-skill:** runs the app on a real device or simulator, reads the screen through `mobilecli` + `Semantics` labels, asserts with Patrol by `Key`, and allows at most 5 self-repair rounds.
  - Needs `mobilecli`, `patrol_cli`, and adb/Xcode. `scripts/bootstrap.sh` installs them.
  - **Not run yet:** `setup-project.sh`. It adds a permission allowlist, format/analyze hooks and the `/spec /ship /verify /debug /nightly` commands to `.claude/`, and it needs a Flutter project to exist first.

### 2.2 Design

- **`mobile-uiux-pro-max`:** Material 3 Expressive, Apple HIG (iOS 26 Liquid Glass) and WCAG 2.2.
  - Includes about 670 atomic rules, DTCG design tokens, and framework packs for Flutter, React Native, SwiftUI and Compose.
  - It requires all **7 UI states**, including **offline** and **permission-denied**, which an app that depends on Bluetooth and location permissions will hit constantly.
  - Ships 6 deterministic validators (Python stdlib only): contrast, target size, state coverage, token lint, Dynamic Type, RTL.
    `python3 ~/.claude/skills/mobile-uiux-pro-max/quality-checks/validators/run_all.py lib/screens/x.dart`
- **`mobile-flutter-design`:** Apple HIG expressed in Flutter. Covers Cupertino vs Material, navigation stacks and tabs, sheets with detents (via `DraggableScrollableSheet`), Dynamic Type, haptics and dark mode. Derived from wshobson/agents.

### 2.3 Reverse engineering

- **`reverse-engineer-anything` (REA):** the skill holds the instructions; the tools come from the MCP server.
  - **`.mcp.json` registers it at project scope**, pinned to `rea-agents@6.5.0`. I checked that `npx -y rea-agents@6.5.0 --version` prints `6.5.0`. Claude Code asks you to approve the project MCP server the first time you open this folder.
  - Coverage: native binaries (via Hopper, Ghidra or IDA), JS/Electron, .NET, Android APKs (JADX + JDK), Flutter APK provenance (Dart snapshot hash, engine build), pcaps (tshark), websites, SQLite, firmware (Linux only).
  - **No native engine is installed**, so binary decompilation won't work until one is.
  - I skipped the repo's 3 contributor-only skills (`rea-ci-tests-audit`, `rea-changelog-update`, `rea-tool-design`). They exist for developing REA itself.
- **`ios-reverse-engineering`:** works on IPA, .app, Mach-O, .dylib and .framework files.
  - Steps: class-dump via `ipsw`, API endpoint discovery (URLSession, Alamofire, GraphQL, WebSocket), an ATS/pinning/keychain audit, a cloud-secret scan, SDK fingerprinting, protection detection, and Ghidra headless Java scripts.
  - `/extract-ipa <file>` starts the workflow. **Fix applied:** the command referenced `${CLAUDE_PLUGIN_ROOT}/skills/…`, which is only set for plugins. I rewrote it to `.claude/skills/…`, and after the move to `~/.claude/skills/…`.
  - *(background)* App Store IPAs are FairPlay-encrypted, so static analysis only works on decrypted builds (your own, TestFlight dev builds, etc.).

---

## 3. Reference codebases (`references/`)

### 3.1 OpenHaystack: `references/seemoo-lab-openhaystack` (`ea05dad`, AGPL-3.0, 21 MB)

This framework lets you build your own accessories that are tracked through **Apple's Find My network**. It comes from TU Darmstadt's SEEMOO lab; their security analysis led to CVE-2020-9986.

**Components** *(code)*:
- A macOS SwiftUI app, plus an Apple Mail plugin. The plugin borrows Mail's entitlements to call the private location-report API.
- `AnisetteDataManager.swift` generates the Apple auth headers.
- Firmware in `Firmware/`: `ESP32`, `Microbit_v1`, `Linux_HCI`.
- **`openhaystack-mobile/` is a Flutter app** (accessory, map, history, location modules). It pins Dart SDK `>=2.12 <3.0`, so it needs a Dart 3 migration before it will build on a current SDK.

**Protocol as implemented** (`FindMy/DecryptReports.swift`) *(code)*:
1. The accessory broadcasts an **EC P-224 public key** in its BLE advertisements.
2. Any passing Apple device encrypts its own GPS fix to that key and uploads it.
3. The owner fetches reports by key hash. The ephemeral key and the private key give an ECDH shared secret.
4. KDF: `SHA-256(shared ‖ uint32_be(1) ‖ ephemeral_pubkey)`. This yields the AES key + IV.
5. **AES-GCM** decrypt. Latitude and longitude are `int32 / 1e7`.

**Caveats:**
- *(code/README)* Their firmware broadcasts a **fixed key** (no rotation), so accessories are trackable by anyone nearby.
- Fetching reports depends on an Apple private API and anisette data. That makes it fragile and against Apple's ToS for a shipping product.
- *(background)* The sanctioned route is Apple's **MFi Find My Network Accessory** program.
- *(background)* Apple and Google both alert users to unknown trackers following them (the IETF **DULT** work). A design that evades those alerts is a hard ethical and legal red line.

### 3.2 Briar: `references/briar-briar` (`d03d6fc`, GPL-3.0, 25 MB)

A decentralised messenger with no central server. This is the GitHub mirror; the canonical repo is code.briarproject.org.

- **Transports** *(code)*: `bramble-core/…/plugin/{bluetooth, tcp, tor, file}`. That covers Bluetooth, LAN/Wi-Fi, Tor, and removable-media ("sneakernet") sync.
- **Layering** *(code)*:
  - **Bramble** is the base layer: transport, sync, crypto, key agreement, rendezvous, mailbox, contact and identity.
  - **Briar** sits on top and provides the messaging clients (private chat, forums, blogs, groups).
  - `briar-mailbox` is an always-on relay for delivering to offline contacts.
  - `briar-headless` exposes the core as a REST API.
- **Crypto primitives in bramble-core** *(code)*: Curve25519 (key agreement), Ed25519 (signatures), XSalsa20-Poly1305 (AEAD), BLAKE2b (hash), scrypt (password KDF).
- **In-person contact exchange** is a QR-code key agreement (`keyagreement/`, `qrcode/`). It's a good pattern for building trust offline.
- **Platform:** Android/JVM only. *(background)* There's no iOS client. iOS doesn't give third-party apps Bluetooth Classic RFCOMM (MFi accessories aside), and it heavily restricts background Bluetooth.

### 3.3 Meshtastic: `references/meshtastic-firmware` (`f7f66b4`, 34 MB) + `references/meshtastic-protobufs` (`28f4f5f`, 1.6 MB), both GPL-3.0

A LoRa radio mesh for text, position and telemetry with no internet or cell coverage. Phones talk to a radio node over BLE, and the nodes form the mesh.

- **Routing** *(code)*:
  - Managed flooding (`FloodingRouter`), plus `NextHopRouter` for direct messages and `PacketHistory` for de-duplication.
  - Default `hop_limit` is 3 (`HOP_RELIABLE`); the maximum is 7 (`HOP_MAX`).
  - `hop_start - hop_limit` gives the number of hops a packet travelled, but it's only reliable from firmware 2.3.0 onward.
- **Crypto** *(code, `CryptoEngine.h`)*: channel traffic uses **AES-CTR** with a 128- or 256-bit pre-shared key. Direct messages use **x25519** public-key crypto.
- **Phone ↔ radio BLE GATT API** *(code, `src/BluetoothCommon.h`)*:

  | Role | UUID |
  |---|---|
  | Service | `6ba1b218-15a8-461f-9fa8-5dcae273eafd` |
  | ToRadio (write) | `f75c76d2-129e-4dad-a1dd-7866124401e7` |
  | FromRadio (read) | `2c55e69e-4993-11ed-b878-0242ac120002` |
  | FromNum (notify) | `ed9da18c-a800-4f66-a670-aa7547e34453` |

  Payloads are the protobuf `ToRadio`/`FromRadio` messages from `meshtastic-protobufs/meshtastic/mesh.proto`. So a Flutter app can be a Meshtastic client: a BLE plugin plus `protoc --dart_out` bindings.
- **Licence:** the protobufs are GPL-3.0 too. Get licence advice before shipping generated bindings in a closed-source app.

### 3.4 Wireshark: `references/wireshark-wireshark` (`c481759`, GPL-2.0, 450 MB source)

Use Wireshark as a **tool**, not a library. The source is only useful if you write a dissector for VYRO LINK's own protocol (a Lua plugin is the light way to do that).

- **Relevant dissectors** *(code, `epan/dissectors/`)*:
  - `packet-btle.c`: BLE link layer and advertising
  - `packet-btatt.c`: ATT/GATT
  - `packet-bthci_*.c`: HCI
  - `packet-nordic_ble.c`: nRF Sniffer
  - `packet-btmesh*.c`: Bluetooth Mesh
  - `packet-loratap.c`, `packet-lorawan.c`
  - `packet-protobuf.c`: generic protobuf decoding from loaded `.proto` files, which can decode Meshtastic payloads
- **Capture paths** *(background)*:
  - **iOS:** install Apple's Bluetooth logging profile, then use **PacketLogger** (Additional Tools for Xcode). Its `.pklg` files open in Wireshark.
  - **Android:** turn on Developer options → *Enable Bluetooth HCI snoop log*. That produces `btsnoop_hci.log`.
  - **Over the air:** an nRF52840 dongle with the *nRF Sniffer for Bluetooth LE* extcap.

---

## 4. Cross-cutting findings

**Transport matrix for a Flutter iOS + Android app** *(background; check current plugin status on pub.dev before choosing)*:

| Transport | Android | iOS | Seen in |
|---|---|---|---|
| BLE central (scan/connect) | ✅ | ✅ | Meshtastic phone app, Find My finders |
| BLE peripheral (advertise/serve) | ✅ | ✅ in foreground; **limited in background**: no local name, and service UUIDs move to an "overflow area" that only iOS scanners looking for that UUID can see | Find My accessories |
| Bluetooth Classic RFCOMM | ✅ | ❌ (MFi only) | Briar |
| Wi-Fi Direct | ✅ | ❌ | (Briar uses LAN) |
| Apple Multipeer Connectivity (AWDL) | ❌ | ✅ | none |
| LoRa via external radio over BLE | ✅ | ✅ | Meshtastic |
| Internet relay / Tor | ✅ | ✅ | Briar (Tor, Mailbox) |

**Implications:**
- **Cross-platform phone-to-phone has to be BLE.** Backgrounded iPhones make poor mesh relays. Dedicated hardware relays avoid that (LoRa nodes like Meshtastic, or ESP32 beacons like OpenHaystack's firmware).
- **Crypto families seen:**
  - Briar: X25519/Ed25519/XSalsa20-Poly1305/BLAKE2b
  - Meshtastic: AES-CTR PSK + x25519
  - Find My: P-224 ECIES + AES-GCM

  For a new design, the modern default is X25519 + Ed25519 + (X)ChaCha20-Poly1305 with a Noise-style handshake. Use a vetted library (libsodium via FFI, or Dart `cryptography`) and never hand-rolled crypto.
- **Flutter packages to evaluate** (none installed yet): `flutter_blue_plus` (BLE central), a peripheral-mode plugin (e.g. `bluetooth_low_energy`), `protobuf` + `protoc_plugin`, `cryptography` / a libsodium binding.
- **Not cloned, but worth adding if the brief is phone-only mesh chat:** *bitchat* (`permissionlesstech/bitchat`). It's an open-source BLE mesh messenger for iOS and Android using the Noise protocol, and the closest public example of mesh messaging with no extra hardware.

---

## 5. Machine readiness (checked 2026-10-14)

| Tool | Status | Needed by | Install |
|---|---|---|---|
| Xcode | ✅ 27.0 | iOS builds, flutter-autonomous | none |
| Node / npm | ✅ 24.18 / 11.16 | REA MCP | none |
| python3, jq | ✅ | UI/UX validators, bootstrap | none |
| **Flutter / Dart SDK** | ❌ | everything Flutter | `brew install --cask flutter` (3.47.7) |
| **JDK** | ❌ (`java` is the macOS stub) | Ghidra, JADX, Android | `brew install --cask temurin` |
| Android Studio / adb | ❌ | Android builds, flutter-autonomous | `brew install --cask android-studio` |
| ipsw | ❌ | ios-reverse-engineering (required) | `brew install ipsw` |
| radare2 | ❌ | ios-reverse-engineering (recommended) | `brew install radare2` |
| Ghidra | ❌ (no brew cask) | REA, iOS RE deep mode | download from ghidra-sre.org (needs JDK) |
| Wireshark / tshark | ❌ | capture analysis, REA pcaps | `brew install --cask wireshark-app` (GUI) / `brew install wireshark` (tshark) |
| mobilecli, patrol_cli | ❌ | flutter-autonomous | `bash ~/.claude/skills/flutter-autonomous/scripts/bootstrap.sh` |

None of these were installed. They're big downloads and the user should choose.

---

## 6. Install log and deviations

- **Folder** is `MyProjects/VyroLink`, following the CamelCase style of the other projects.
- **Project-scoped only:** `~/.claude/skills/` (82), `~/.claude/commands/extract-ipa.md`, `.mcp.json`.
- **Safety scan:** I grepped every skill for `curl|sh`, `sudo`, `~/.ssh`, prompt-injection phrases, and the like before installing. I only found normal installer docs. No repo installer scripts were run.
- **De-dup and renames:** dart-* skills come from `dart-lang`. The gabuldev collisions got a `-gabul` suffix. `/extract-ipa` paths were fixed.
- **References are `--depth 1` clones.** To update one: `git -C references/<repo> pull`. For full history: `git -C references/<repo> fetch --unshallow`.
- **Updating skills:** re-clone the source repo and `rsync -a --exclude=.git` the skill folder over `~/.claude/skills/<name>/`.

---

## 7. Open questions for the project brief

1. Is it **phone-to-phone only**, or phones plus custom hardware (LoRa/ESP32 nodes, tags)?
2. Is it **messaging**, **location/finding**, or both?
3. Is it **iOS-first**, or iOS/Android parity?
4. **Licence:** open source and GPL-compatible, or closed? This decides whether any reference code can be reused.
5. Does it use **Apple's Find My network** (MFi program) or its own network?
6. Which architecture: official layered, Clean Arch + Cubit, or something else? (See the conflict in §2.1.)
