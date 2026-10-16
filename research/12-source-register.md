# Document 12 — Source and Evidence Register

**Project:** VYRO LINK · **Package version:** 1.0 · **All sources accessed:** 2026-10-14 unless stated · **Researcher:** Claude (AI research assistant), working on the user's MacBook; no software installed, no experiments run.

## How to read evidence levels

| Level | Meaning | Typical source |
|---|---|---|
| **A** | Officially documented by the platform/standard owner | Apple/Android developer docs, Apple Platform Security, RFCs, Bluetooth SIG |
| **B** | Demonstrated by a published, reproducible analysis **or** by source code inspected during this research (file paths cited) | Peer-reviewed papers, source files read at a pinned commit |
| **C** | Credible independent source, not independently verified | Vendor blogs, developer-forum reports, secondary press, project self-descriptions |
| **D** | Hypothesis / inference requiring experiment | Our own reasoning |

"Inspected" means the file was actually read during this research. Where only a summary or abstract was accessible, that is stated in *Limitations*.

---

## A. Apple — official documentation

| ID | Title | Publisher | URL | Published / revised | Category | Claims supported | Quality | Limitations |
|---|---|---|---|---|---|---|---|---|
| S01 | Find My security (Apple Platform Security) | Apple | https://support.apple.com/guide/security/sec6cbc80fd0/web | 2021-02-18 (page date) | Official security doc | P-224 key pair generated on device; key/secret never sent to Apple; derived public key rotates ~every 15 min; P-224 chosen to fit one BLE payload; finder traffic carries no auth info; Apple cannot link finder and owner; E2E encrypted | A | Does not name the report-encryption scheme, retention, or accessory program details |
| S09 | Core Bluetooth Programming Guide — Performing Long-Term Actions in the Background | Apple | https://developer.apple.com/library/archive/documentation/NetworkingInternetWeb/Conceptual/CoreBluetooth_concepts/CoreBluetoothBackgroundProcessingForIOSApps/PerformingTasksWhileYourAppIsInTheBackground.html | 2013-09-18 (archived) | Official guide (archived) | `bluetooth-central`/`bluetooth-peripheral` modes; background scan coalesces duplicates; longer scan interval; local name not advertised in background; overflow area; ~10 s per wake; state preservation & restoration | A (old) | Archived 2013 text; behaviour must be re-validated on iOS 26/27 |
| S10 | `CBPeripheralManager.startAdvertising(_:)` | Apple | https://developer.apple.com/documentation/corebluetooth/cbperipheralmanager/startadvertising(_:) | current | API reference | Only two keys supported (local name, service UUIDs); 28 bytes foreground + 10 bytes scan response for name; overflow area; in background name not advertised and all UUIDs in overflow | A | — |
| S10b | `CBCentralManager.scanForPeripherals(withServices:options:)` | Apple | https://developer.apple.com/documentation/corebluetooth/cbcentralmanager/scanforperipherals(withservices:options:) | current | API reference | Background scanning requires explicit service UUIDs; scan options have no effect in background | A | — |
| S11 | TN3115: Bluetooth State Restoration app relaunch rules | Apple | https://developer.apple.com/documentation/technotes/tn3115-bluetooth-state-restoration-app-relaunch-rules | rev. 2025-09-15 (iOS 26) | Technote | Relaunch table: removed from memory/crash/airplane/restart → yes; **force quit → no**; Bluetooth power toggled → no; restart → only after first unlock; iOS 26: only AccessorySetupKit apps relaunched in the noted cases; relaunch only if a pending BT request's event occurs | A | Not yet revised for iOS 27 (checked 2026-10-14) |
| S12 | Wi-Fi Aware framework; "Building peer-to-peer apps"; "Connecting devices for peer-to-peer Wi-Fi" | Apple | https://developer.apple.com/documentation/wifiaware ; …/building-peer-to-peer-apps ; …/connecting-paired-devices | iOS 26.0+ | Framework docs | Wi-Fi Aware on iPhone 12+; pairing via DeviceDiscoveryUI or AccessorySetupKit is required; connections to paired devices in foreground and background; entitlement `com.apple.developer.wifi-aware`; not in Simulator; `realtime` mode drains battery | A | No statement on Android interoperability guarantees |
| S13 | DeviceDiscoveryUI | Apple | https://developer.apple.com/documentation/devicediscoveryui | iOS 26.0+ | Framework docs | System pairing UI; "supports pairing with Apple and third-party devices"; system-managed connection avoids full local-network permission | A | — |
| S14 | TN3111: iOS Wi-Fi API overview | Apple | https://developer.apple.com/documentation/technotes/tn3111-ios-wifi-api-overview | rev. 2025-08-29 | Technote | Wi-Fi Aware is the standards-based path to non-Apple devices; Apple peer-to-peer Wi-Fi is undocumented for third parties and **only works between Apple devices**; no general Wi-Fi scanning API | A | — |
| S15 | Developer Forums 837516 — "Can a non-iOS central discover an iOS peripheral advertising in the background?" | Apple (staff engineer answer) | https://developer.apple.com/forums/thread/837516 | Jul 2026 | Official forum answer | Non-Apple central sees the advertisement but **cannot access overflow-area information**, which may be essential to recognise the device | A− (staff answer, not reference doc) | Forum answer, not normative |
| S16 | Developer Forums 801280, 790195, 791628, 797186 — Wi-Fi Aware iOS 26 ↔ Android | Apple forums (developers + DTS) | https://developer.apple.com/forums/thread/801280 ; /790195 ; /791628 | 2025–2026 | Forum reports | Pairing required (DTS); Android-publisher → iPhone-subscriber discovery failures, PIN display problems; DTS points to Accessory Design Guidelines | C | Anecdotal, device-specific |
| S08 | Find My network accessory program | Apple | https://developer.apple.com/find-my/ | current | Program page | Accessory spec available only via MFi enrolment | A | Spec itself is under NDA; not accessed |
| S06 | "Apple and Google deliver support for unwanted tracking alerts in iOS and Android" | Apple Newsroom / Google Security Blog | https://www.apple.com/newsroom/2024/05/apple-and-google-deliver-support-for-unwanted-tracking-alerts-in-ios-and-android/ | 2024-05-13 | Press release | Cross-platform DULT alerts shipped in iOS 17.5 and Android 6.0+ | A | Marketing-level detail |
| S63 | Data Protection classes (Apple Platform Security) | Apple | https://support.apple.com/guide/security/secb010e978a/web | current | Official security doc | Complete Protection class key discarded ~10 s after lock; Class C (until first user authentication) remains available after first unlock | A | Class C full wording only partially retrieved |
| S53a | iOS 27 / 27.0.1 availability | Apple PR via iPhone in Canada | https://www.iphoneincanada.ca/2026/09/28/you-can-now-download-ios-27-0-1-for-iphone-and-more/ | 2026-09-28 | Secondary press | iOS 27 released 2026-09-14; 27.0.1 current | C | Secondary; Apple docs reviewed here are mostly labelled iOS 26 — no iOS 27-specific Bluetooth changes were found |

## B. Android / Google — official documentation

| ID | Title | URL | Published / revised | Claims supported | Quality | Limitations |
|---|---|---|---|---|---|---|
| S17 | Bluetooth permissions | https://developer.android.com/develop/connectivity/bluetooth/bt-permissions | current | `BLUETOOTH_SCAN/ADVERTISE/CONNECT` runtime permissions on Android 12+; `neverForLocation`; location permission needed on ≤11 | A | — |
| S18 | Communicate in the background (BLE) | https://developer.android.com/develop/connectivity/bluetooth/ble/background | current | Scanning/connecting need a live process; `PendingIntent` scan wakes app for short work; avoid periodic scans; GATT connection closes if process killed; FGS `connectedDevice` or CompanionDeviceService for long connections; Android ≥10 batches connection requests | A | — |
| S19 | Foreground service types | https://developer.android.com/develop/background-work/services/fgs/service-types | current | `connectedDevice` = interactions with external devices over Bluetooth etc.; prerequisites incl. Bluetooth runtime permission | A | "External device" wording — applicability to phone-to-phone relaying is a policy interpretation |
| S20 | Android 15 behavior changes | https://developer.android.com/about/versions/15/behavior-changes-15 | current | `dataSync` FGS limited to 6 h / 24 h; BOOT_COMPLETED cannot start dataSync/camera/mediaPlayback/… FGS | A | `connectedDevice` not on the BOOT_COMPLETED list (inference: allowed — verify) |
| S21 | Wi-Fi Aware overview; `AwarePairingConfig` (API 34) | https://developer.android.com/develop/connectivity/wifi/wifi-aware ; https://developer.android.com/reference/kotlin/android/net/wifi/aware/AwarePairingConfig | current | API 26+; hardware feature check; discovery messages ≈255 bytes, unreliable; data path via network specifier; pairing setup/verification APIs from API 34 when supported | A | No iPhone-interop statement |
| S22 | Wi-Fi Direct (P2P) | https://developer.android.com/develop/connectivity/wifi/wifip2p | current | `NEARBY_WIFI_DEVICES` (API 33+); discovery/connect APIs | A | Does not cover invitation UX or background |
| S23 | Local network permission (Android 17) | https://developer.android.com/privacy-and-security/local-network-permission | current | `ACCESS_LOCAL_NETWORK` enforced for apps targeting SDK 37 | A | Retrieved via search summary |
| S24 | Understanding foreground service requirements (Play Console) | https://support.google.com/googleplay/android-developer/answer/13392821 | current | FGS types must be declared and justified; all types subject to review | A | Policy enforcement is discretionary |
| S53b | Android 17 = API 37, stable June 2026 | https://source.android.com/setup/build-numbers | current | API level for Android 17 | A (table) / C (date via press) | Release date via secondary press |
| S26 | Nearby Connections overview; Swift get-started | https://developers.google.com/nearby/connections/overview ; https://developers.google.com/nearby/connections/swift/get-started | current | "Fully-offline peer-to-peer"; Android & iOS; iOS mediums: BLE, Wi-Fi LAN, Wi-Fi hotspot | A | iOS page silent on background and Android interop details |
| S26r | google/nearby repository | https://github.com/google/nearby | last push 2026-10-14 | Apache-2.0; Apple platform implementation contains BLE (GATT + L2CAP), AWDL, Wi-Fi Aware, hotspot, LAN mediums; commit 2026-09-28 "NC changes for AW"; only iOS release tag `v0.0.1-ios` (2021-11-16) | B (directory listing + commit log inspected) | Code quality/stability not evaluated |

## C. Standards and specifications

| ID | Title | Publisher | URL | Date | Claims | Quality |
|---|---|---|---|---|---|---|
| S36 | Bluetooth Mesh glossary / managed flooding | Bluetooth SIG | https://www.bluetooth.com/learn-about-bluetooth/feature-enhancements/mesh/mesh-glossary/ | current | Managed flooding, relay & proxy features; phones join via GATT proxy | A |
| S37 | RFC 4838 Delay-Tolerant Networking Architecture | IETF/IRTF | https://www.rfc-editor.org/rfc/rfc4838 | 2007 | Store-carry-forward architecture | A (not re-read in this session) |
| S38 | RFC 9171 Bundle Protocol v7; RFC 9172 BPSec | IETF | https://www.rfc-editor.org/rfc/rfc9171 ; https://www.rfc-editor.org/rfc/rfc9172 | Jan 2022 | BPv7 standard; BPSec integrity/confidentiality between security source and acceptor | A |
| S39 | RFC 6693 PRoPHET | IRTF (Experimental) | https://www.rfc-editor.org/rfc/rfc6693 | Aug 2012 | Encounter-history probabilistic routing | A |
| S42 | Signal Double Ratchet (rev. 4) ; X3DH ; PQXDH | Signal | https://signal.org/docs/specifications/doubleratchet/ | rev 4, 2025-11-04 | Skipped-message keys & MAX_SKIP; forward secrecy; break-in recovery; suggested per-session skipped-key limits | A (spec) |
| S43 | The Noise Protocol Framework rev. 34 | Trevor Perrin | https://noiseprotocol.org/noise.html | 2018-07-11 | One-way patterns N/K/X: "forward secrecy for sender compromise only, vulnerable to replay"; XX properties | A (spec) |
| S44 | RFC 9420 Messaging Layer Security | IETF | https://www.rfc-editor.org/rfc/rfc9420 | Jul 2023 | Group key agreement assuming a delivery service | A (not re-read in this session) |
| S07 | DULT threat model draft-ietf-dult-threat-model-05 | IETF DULT WG | https://datatracker.ietf.org/doc/draft-ietf-dult-threat-model/ | 2026-08-06 | Unwanted-tracking threat taxonomy | A (draft, not final) |

## D. Research papers

| ID | Title | Authors / venue | URL | Date | Claims | Quality | Limitations |
|---|---|---|---|---|---|---|---|
| S02 | Who Can Find My Devices? Security and Privacy of Apple's Crowd-Sourced Bluetooth Location Tracking System | Heinrich, Stute, Kornhuber, Hollick — PoPETs 2021(3) | https://doi.org/10.2478/popets-2021-0045 | 2021 | Reverse-engineered offline-finding protocol; two vulnerabilities (incl. CVE-2020-9986) | B | Bibliographic record confirmed; details taken from OpenHaystack code/README rather than full paper text in this session |
| S05 | nRootTag (Find My abuse without root) | Chen, Ma, Luo, Zeng — USENIX Security 2025 | https://people.cs.gmu.edu/~zeng/papers/2025-security-nrootgag.pdf | 2025 | Any BT device can be made trackable via Find My by searching keys matching its address; artifact appendix lists Apple patches (iOS 18.2 etc.) | B | Read via summaries + appendix listing |
| S28 | Mesh Messaging in Large-scale Protests: Breaking Bridgefy | Albrecht, Blasco, Jensen, Mareková — CT-RSA 2021 | https://eprint.iacr.org/2021/214 | 2021 | Tracking, impersonation, no effective confidentiality, network-wide DoS via crafted message | B | Abstract + summaries |
| S29 | Breaking Bridgefy, again: Adopting libsignal is not enough | Albrecht, Eikenberg, Paterson — USENIX Security 2022 | https://www.usenix.org/conference/usenixsecurity22/presentation/albrecht | 2022 | MITM, broadcast impersonation, residual decompression-bomb DoS after libsignal adoption | B | Abstract + summaries |
| S40 | Spray and Wait | Spyropoulos, Psounis, Raghavendra — ACM WDTN 2005 | (ACM DL) | 2005 | L-copy spray + wait; binary variant | B | Citation via surveys |
| S41 | Epidemic Routing for Partially-Connected Ad Hoc Networks | Vahdat & Becker — Duke TR CS-2000-06 | — | 2000 | Summary-vector anti-entropy flooding | B | Not re-read this session |
| S50 | Connection-less BLE Performance Evaluation on Smartphones | Siva, Yang, Poellabauer — 2019 | https://par.nsf.gov/servlets/purl/10127090 | 2019 | Smartphone discovery latency ≈ models but higher variance; energy deviates from models; Wi-Fi interference matters | B | Android API 24 / BT 4.2 — old |
| S51 | Energy modeling of neighbor discovery in BLE networks | Sensors 19(22):4997 | https://www.mdpi.com/1424-8220/19/22/4997 | 2019 | Advertising interval/scan window drive latency-energy trade-off | B | Dedicated-chip testbed |

## E. Source code inspected (pinned)

| ID | Repository | Owner / license | Commit / date | Files actually read | Claims |
|---|---|---|---|---|---|
| S03 | seemoo-lab/openhaystack | TU Darmstadt SEEMOO / **AGPL-3.0** | `ea05dad` (2026-08-17) | `README.md`; `OpenHaystack/OpenHaystack/FindMy/DecryptReports.swift`; `Firmware/ESP32/main/openhaystack_main.c` | KDF = SHA-256(shared ‖ be32(1) ‖ ephemeral key); AES-GCM; lat/lon int32/1e7; 28-byte key split: 6 bytes in BLE address (top bits forced `11`) + 22 bytes in manufacturer data `4C 00 12 19`; report download needs authenticated Apple account (Mail plugin) |
| S27 | permissionlesstech/bitchat (iOS/macOS) | **Unlicense (public domain)** | `main` @ 2026-10-11 | `WHITEPAPER.md` (v2.0, 2026-07-06); `SECURITY.md`; `bitchat/Info.plist`; `Services/BLE/BLERadioController.swift`; `Services/BLE/BLEService.swift` (UUIDs); `Services/TransportConfig.swift` | Dual-role GATT mesh; TTL 7; LRU dedup; source routing; Noise XX live sessions; Noise X courier seals (no FS); spray-and-wait couriers (budget 4, cap 8; 16 KiB, 24 h); daily HMAC recipient tags; background wake via indefinite pending connects; `bluetooth-central` + `bluetooth-peripheral` modes; max 6 central links; self-declared metadata weaknesses |
| S27a | permissionlesstech/bitchat-android | **GPL-3.0** | 2026-10-11 | `README.md` | Claims binary-protocol compatibility with iOS; foreground service keeps mesh alive |
| S31 | briar/briar (GitHub mirror of code.briarproject.org) | Briar Project / GPL-3.0 (`LICENSE.txt`) | `d03d6fc` (2026-10-09) | `bramble-android/.../AndroidBluetoothPlugin.java`; `bramble-api/.../sync/SyncConstants.java`, `RecordTypes.java`, `Group.java` | Bluetooth Classic RFCOMM (`listenUsingInsecureRfcommWithServiceRecord`); sync records ACK/MESSAGE/OFFER/REQUEST/VERSIONS/PRIORITY; 32 KiB max body; group visibility INVISIBLE/VISIBLE/SHARED per contact |
| S33 | meshtastic/firmware + meshtastic/protobufs | Meshtastic / GPL-3.0 | `f7f66b4` (2026-10-14) / `28f4f5f` | `src/mesh/MeshTypes.h`, `src/mesh/CryptoEngine.h`, `src/BluetoothCommon.h`, `meshtastic/mesh.proto` | HOP_RELIABLE 3, HOP_MAX 7; AES-CTR channel crypto; x25519 for DMs; BLE GATT phone API UUIDs; hop_start semantics |
| S46 | signalapp/libsignal | Signal / **AGPL-3.0** | push 2026-10-07 | metadata only | License constraint |
| S45r | matrix-org/vodozemac | Matrix.org / **Apache-2.0** | push 2026-10-08 | metadata only | License + activity |

## F. Security advisories and audits

| ID | Title | URL | Date | Claims | Quality |
|---|---|---|---|---|---|
| S34 | CVE-2025-52464 Meshtastic repeated key pairs | https://osv.dev/vulnerability/CVE-2025-52464 | 2025-06-19 | Vendor flashing cloned key pairs + weak RNG → DM confidentiality loss; fixed 2.6.11/2.6.12 | A (CVE record) |
| S32 | Cure53 pentest of Briar | https://cure53.de/pentest-report_briar.pdf | 2017-03 | 12 issues, 1 high (DNS leak via RSS import); fixed in 2017 beta | B (via index/summaries) |
| S45 | Least Authority audit of vodozemac | https://leastauthority.com/static/publications/LeastAuthority-Matrix_vodozemac_Final_Audit_Report.pdf | 2022-03-30 | 10 issues + 8 suggestions (zeroisation, validation, overflow, MAC truncation…) | B (TOC via summary) |

## G. Libraries and tools (metadata checked via GitHub API, 2026-10-14)

| ID | Item | License | Last push | Note |
|---|---|---|---|---|
| S47 | chipweinberger/flutter_blue_plus | **"FlutterBluePlus License v1.6" — commercial licence required for for-profit use** (LICENSE read) | 2026-10-03 | Central role only; points to other packages for peripheral |
| S48a | yanshouwang/bluetooth_low_energy | MIT | 2026-01-17 | Central + peripheral (per FBP README pointer; not code-reviewed) |
| S48b | simolus3/drift | MIT | 2026-10-14 | SQLite ORM |
| S48c | davidmartos96/sqflite_sqlcipher | BSD-2 | 2026-08-04 | SQLCipher for Flutter |
| S48d | juliansteenbakker/flutter_secure_storage | BSD-3 | 2026-10-07 | Keychain/Keystore |
| S48e | juliansteenbakker/mobile_scanner | BSD-3 | 2026-10-07 | QR scanning |
| S48f | google/protobuf.dart | BSD-3 | 2026-09-11 | Serialization |
| S48g | dint-dev/cryptography | Apache-2.0 | 2025-11-21 | Dart crypto (slower activity) |
| S48h | Skycoder42/libsodium_dart_bindings | BSD-3 | 2026-10-14 | libsodium FFI |
| S48i | famedly/dart-vodozemac | **AGPL-3.0** | 2026-09-23 | Wrapper licence differs from vodozemac's Apache-2.0 |
| S54 | wireshark/wireshark | GPL-2.0 | 2026-10-14 | Local clone has `packet-btle.c`, `packet-btatt.c`, `packet-nordic_ble.c`, `packet-btmesh*.c`, `packet-protobuf.c` |
| S55 | skylot/jadx | Apache-2.0 | 2026-10-13 | APK decompiler |
| S56 | NationalSecurityAgency/ghidra | Apache-2.0 | 2026-10-13 | Binary analysis (Homebrew formula 12.1.4 observed) |
| S57 | frida/frida | custom (wxWindows-style per repo) | 2026-10-14 | Dynamic instrumentation |
| S60 | seemoo-lab/AirGuard | Apache-2.0 | 2026-09-30 | Android tracker detection (Find My research companion) |
| S64 | meshtastic/Meshtastic-Apple / -Android | GPL-3.0 | 2026-10-14 | Phone clients |

## H. Credible secondary / vendor sources (level C)

| ID | Title | URL | Claims | Limitations |
|---|---|---|---|---|
| S04 | "Send My" — Find My abused to upload arbitrary data (Positive Security; coverage by The Register) | https://www.theregister.com/2021/05/12/apples_find_network/ ; https://github.com/positive-security/send-my | Few bytes/s via Find My; no rate limit observed (2021) | Patch status unverified |
| S25 | Punch Through — Troubleshooting Android BLE scan errors | https://punchthrough.com/android-ble-scan-errors/ | ≈5 `startScan` per 30 s per app limit since Android 7; silent failure | Undocumented by Google beyond error constant |
| S30 | Bridgefy SDK docs / licence (pub.dev mirror) | https://docs.bridgefy.me ; https://pub.dev/packages/bridgefy | Proprietary licence, key-based, subscription | Terms may have changed |
| S35 | Serval Project | https://en.wikipedia.org/wiki/Serval_Project ; https://github.com/servalproject/serval-dna | Rhizome store-and-forward; last Serval Mesh release 0.93 (2016); serval-dna last push 2022-06-02 | Status inferred from inactivity |
| S49 | Infineon BLE throughput benchmark | https://community.infineon.com/t5/ModusToolbox-Blog/BLE-Throughput-Rate-Benchmark-Test-for-Cypress-BT-SoCs/bc-p/267238 | 2M PHY, MTU 515 GATT WwR: iPhones ≈311–365 kbps, Pixel 3/S10+ ≈1.2 Mbps | Vendor peripheral, old phones |
| S52 | Silicon Labs BGX throughput | https://docs.silabs.com/gecko-os/1/bgx/1.x/throughput | iOS apps cannot set PHY; min CI 15 ms iOS vs 11.25 ms Android | Vendor doc |
| S58 | Nordic DevZone threads on nRF Sniffer | https://devzone.nordicsemi.com/ (threads 34455, 39820) | Sniffer may lose encrypted connections; single-channel limits | Old threads |
| S59 | Apple Profiles & Logs (Bluetooth for iOS) | https://developer.apple.com/bug-reporting/profiles-and-logs/?name=bluetooth | Bluetooth logging profile + instructions PDF exist | Instructions PDF not read |
| S62 | heise: EU DMA requires Wi-Fi Aware in iOS 26 | https://heise.de/-10446649 | Regulatory origin of Wi-Fi Aware support | Press |
| S66 | Apple Developer Forums 840468 / 844477 / 818370 — force-quit relaunch and testing tips (DTS replies) | https://developer.apple.com/forums/thread/840468 ; /844477 ; /818370 | DTS: force-quit → no Bluetooth relaunch except AccessorySetupKit on iOS 26+; "Fast App Termination" developer setting suggested for testing | Forum replies; consistent with S11 |
| S65 | bitchat third-party commentary (identity flaw at launch, Noise adoption) | https://vibegraveyard.ai/story/bitchat-vibe-coded-messaging-security-flaws/ ; DEV Community posts | Launch-time MITM/impersonation, later Noise | Low-quality secondary; superseded by S27 whitepaper |

## J. Added in the feasibility-validation phase (accessed 2026-10-14; see Doc 14)

| ID | Title | Publisher | URL | Claims | Quality | Limitations |
|---|---|---|---|---|---|---|
| S67 | `BluetoothLeScanner` API reference | Google | https://developer.android.com/reference/android/bluetooth/le/BluetoothLeScanner | "For unfiltered scans, scanning is stopped on screen off to save power"; `startScan(filters, settings, PendingIntent)` | A | — |
| S68 | `BluetoothDevice` API reference | Google | https://developer.android.com/reference/android/bluetooth/BluetoothDevice | `createL2capChannel` / `createInsecureL2capChannel` added API 29, LE only; legacy `connectGatt` overloads deprecated in API 37 → `connectGatt(BluetoothGattConnectionSettings, Executor, BluetoothGattCallback)` | A | — |
| S69 | `CBPeripheralManager.publishL2CAPChannel(withEncryption:)` | Apple | https://developer.apple.com/documentation/corebluetooth/cbperipheralmanager/publishl2capchannel(withencryption:) | iOS 11+; PSM discovery is the app's responsibility | A | — |
| S70 | Restrictions on starting a foreground service from the background | Google | https://developer.android.com/develop/background-work/services/fgs/restrictions-bg-start | API 31+ rule and exemptions (user-visible transition, notification action, BOOT_COMPLETED, battery optimisations off, CDM permissions, exact alarm…) | A | No Bluetooth-specific exemption |
| S71 | Android 16 behavior changes (all apps) | Google | https://developer.android.com/about/versions/16/behavior-changes-all | Job quotas apply even alongside an FGS; bond-loss handling | A | — |
| S72 | Android 17 behavior changes (all apps) | Google | https://developer.android.com/about/versions/17/behavior-changes-all | Autonomous re-pairing; no BLE scan/advertise changes listed | A | Targeting-17 changes covered by S23 |
| S73 | Bluetooth privacy (Apple Platform Security) | Apple | https://support.apple.com/guide/security/sec82597d97e/web | Address randomised "on a frequent basis"; no interval published | A | Page dated 2021-02-18 |
| S74 | Developer Forums 839288 — iOS 27 public beta CoreBluetooth disconnect | Apple forums | https://developer.apple.com/forums/thread/839288 | Single developer report; DTS asked for re-test | C | Beta, unresolved |
| S75 | AOSP CDD change: 15-minute BLE RPA rotation | AOSP (googlesource) | https://android.googlesource.com/platform/docs/source.android.com/+/a13d876%5E%21/ | Android devices must rotate RPA at least every 15 min | C | Seen at search-result level only |

## K. Added in the architecture-freeze phase (accessed 2026-10-14; see Docs 15–18)

| ID | Title | Publisher | URL | Claims | Quality | Limitations |
|---|---|---|---|---|---|---|
| S76 | Public Report — RustCrypto AES-GCM and ChaCha20+Poly1305 Implementation Review | NCC Group (for MobileCoin) | https://www.nccgroup.com/research/public-report-rustcrypto-aesgcm-and-chacha20pluspoly1305-implementation-review/ | Review of RustCrypto `aes-gcm` and `chacha20poly1305` (Feb 2020; 5 person-days) | B | Old crate versions; findings read via summary only |
| S77 | GitHub security advisories: vodozemac (GHSA-j8cm-g7r6-hfpq, GHSA-c3hm-hxwf-g5c6, both low, 2024); snow (GHSA-7g9j-g5jg-3vv3, medium, 2024); RustCrypto AEADs (aes-gcm/ascon in-place decrypt exposure) | GitHub | https://github.com/matrix-org/vodozemac/security/advisories ; https://github.com/mcginty/snow/security/advisories ; https://github.com/RustCrypto/AEADs/security/advisories | Known issues and fixes (retrieved via GitHub API) | A (vendor advisories) | — |
| S78 | Security Audit of dalek libraries | Quarkslab (for Tari Labs) | https://blog.quarkslab.com/security-audit-of-dalek-libraries.html | 2019 audit of curve25519-dalek, subtle, bulletproofs; minor issues | B | Predates current major versions |
| S79 | RUSTSEC-2024-0344 curve25519-dalek timing variability | RustSec / OSV | https://rustsec.org/advisories/RUSTSEC-2024-0344 | Fixed in ≥ 4.1.3 | A | — |
| S80 | UniFFI user guide: interfaces / foreign traits; Firefox UniFFI docs | Mozilla | https://mozilla.github.io/uniffi-rs/latest/types/interfaces.html ; https://firefox-source-docs.mozilla.org/rust-components/developing-rust-components/uniffi.html | Foreign-implemented traits in Kotlin/Swift; production use in application-services | A | Async foreign-trait status not confirmed |
| S81 | `chacha20poly1305` crate documentation (`src/lib.rs`) | RustCrypto | https://github.com/RustCrypto/AEADs/tree/master/chacha20poly1305 | Crate v0.11.0, Apache-2.0 OR MIT; provides XChaCha20Poly1305 but notes "No authoritative specification exists for XChaCha20Poly1305" | B (source read) | — |
| S82 | rusqlite `Cargo.toml` features | rusqlite | https://github.com/rusqlite/rusqlite/blob/master/Cargo.toml | `bundled-sqlcipher`, `bundled-sqlcipher-vendored-openssl` features exist (v0.40.2, MIT) | B (source read) | — |
| S83 | SQLCipher evaluation checklist; CVE-2020-27207, CVE-2021-3119 | Zetetic; MITRE/NVD | https://www.zetetic.net/sqlcipher/comparison/ ; https://nvd.nist.gov/vuln/detail/CVE-2021-3119 | No published independent audit found; two DoS CVEs fixed (4.4.1/4.4.3) | A (CVE) / C (vendor claims) | — |
| S84 | "Who Can Find My Devices?" PoPETs 2021 abstract page | PoPETs | https://www.petsymposium.org/popets/2021/popets-2021-0045.php | Location-correlation attack; up to 7 days of location history; Apple partially addressed | A (venue page) | Abstract only |
| S85 | Repository metadata (licence, last push, latest release) for uniffi-rs (0.32.2, MPL-2.0), vodozemac (0.11.1, MSRV 1.96), snow (0.10.0), rusqlite (0.40.2), sqlcipher (4.19.0), x25519-dalek 3.0.0 / curve25519-dalek 5.0.0 (BSD-3), flutter/packages (BSD-3), qr.flutter (last push 2024-06-28), qrcode-rust (Apache-2.0), riverpod (MIT), flutter_rust_bridge (MIT) | GitHub API | https://github.com/ | Maintenance and licence facts | B | Snapshot on 2026-10-14 |

**URL check (2026-10-14):** all 68 URLs previously in this register were requested. 65 returned HTTP 200. Three returned 403 to automated requests (`community.infineon.com` S49, `devzone.nordicsemi.com` S58, `mdpi.com` S51); these sites block bots, and their content had been retrieved earlier via search.

## I. Sources that could not be accessed

| Item | Reason | Substitute used |
|---|---|---|
| Apple Find My Network Accessory Specification | MFi NDA | S01, S02, S03 |
| Full text of S02, S28, S29 | Only abstracts/summaries retrieved | OpenHaystack source (S03); summaries clearly labelled |
| Apple Bluetooth logging instructions PDF | Not retrieved | Listed as to-read in Doc 13 |
| Current Bridgefy protocol | Closed source | Published attacks S28/S29 |
