# Document 3 — Existing Systems and Open-Source Analysis

**Checked:** 2026-10-14 · Repository metadata via GitHub API on that date · Evidence levels A/B/C/D (Doc 12)

> **Bottom line.**
> - **bitchat** is the closest prior art. It is a cross-platform iOS/Android BLE mesh with spray-and-wait "couriers" and daily-rotating recipient tags. Its iOS code is public domain (Unlicense); its Android code is GPL-3.0. Its own whitepaper documents the gaps VYRO LINK could address: static on-air identity, no forward secrecy for carried mail, and payload length visible on most packet types [S27].
> - **Briar** is the best-audited design, but it is Android-only, uses Bluetooth Classic, and syncs only among contacts.
> - **Bridgefy** is proprietary and has a history of broken security [S28, S29].
> - **Meshtastic** needs LoRa hardware.
> - **Serval** is inactive.
> - **Bluetooth Mesh** does not let phones act as relays.
>
> No complete library should be adopted as VYRO LINK's core. Study bitchat (and possibly reuse its iOS code), Briar's sync model, and the DTN literature.

---

## 1. System inventory and comparison

| | **bitchat** | **Briar** | **Bridgefy** | **Meshtastic** | **Serval Mesh** | **Bluetooth Mesh** | **Apple Multipeer / P2P Wi-Fi** | **Google Nearby Connections** |
|---|---|---|---|---|---|---|---|---|
| Kind | Messaging app + protocol | Messaging app + framework (Bramble) | Proprietary SDK + app | LoRa radio mesh firmware + phone clients | Mesh telephony/messaging suite | SIG standard for device networks | OS framework | Cross-device transport SDK |
| Platforms | iOS, macOS, Android (separate codebases) [S27, S27a] | Android, desktop (JVM) [S31] | iOS, Android [S30] | Dedicated radio; iOS/Android clients via BLE [S33, S64] | Android (historic) [S35] | Embedded nodes; phones via GATT proxy [S36] | Apple only [S14] | Android, iOS (+Windows/ChromeOS code) [S26, S26r] |
| Transports | BLE GATT (dual role); Nostr over internet | BT Classic RFCOMM, LAN TCP, Tor, removable media [S31] | BLE (details closed) | LoRa between nodes; BLE/USB/Wi-Fi to phone | Wi-Fi (ad-hoc/infrastructure) | BLE advertising bearer + GATT proxy | AWDL/infra Wi-Fi, BT | BLE, BT, Wi-Fi LAN/hotspot/Direct/Aware (per medium) |
| Discovery | Fixed service UUID adverts; signed announces (multi-hop) | BT discovery/known addresses, LAN, Tor rendezvous | Closed | LoRa broadcast; phone-node BLE pairing | Wi-Fi | Unprovisioned beacons, provisioning | Bonjour | Advertise/discover by service ID |
| Routing | Controlled flood (TTL 7, clamps, LRU dedup, jitter, log₂ fanout) + source routes from neighbour lists [S27 §4] | **No relaying through strangers.** Sync per contact/group; group messages spread via sharing members [S31 `Group.Visibility`] | Flood (per 2020 analysis) [S28] | Managed flooding + next-hop router; hop limit 3 default, 7 max [S33] | Rhizome sync epidemic store-and-forward [S35] | Managed flooding with TTL + cache [S36] | None (single hop) | None (single hop, topology strategies) |
| Store-and-forward | Sender outbox (100/peer, 24 h); courier spray-and-wait (budget 4, cap 8, 16 KiB, 24 h); gossip public history 6 h [S27 §6] | Yes, between contacts; Briar Mailbox (own always-on device) | Unclear | Store-and-forward module on nodes (not inspected) | Rhizome | No (real-time) | No | No |
| E2E crypto | Noise XX live; Noise X seals for couriers (**no FS**); Ed25519 signatures [S27 §5] | Curve25519, Ed25519, XSalsa20-Poly1305, BLAKE2b, scrypt (from code grep) | libsignal adopted after 2020, still broken in 2022 [S29] | AES-CTR channel PSK; x25519 DMs [S33]; key-duplication CVE [S34] | Custom (historic) | AES-CCM network/app keys | TLS/DTLS optional | "Fully encrypted" link (vendor claim) |
| Identity / pairing | Static Noise key; 8-byte peer ID = fingerprint prefix (stable); optional QR verify | QR key agreement in person; introductions | Account-based (historic) | Node keys; channel PSK sharing via QR/URL | SID keys | Provisioning (OOB optional) | Apple IDs / none | Endpoint IDs + auth token |
| Metadata exposure (self-documented or found) | Stable sender ID in every packet; static keys + nickname in clear announcements; neighbour lists; payload length visible [S27 §8] | Tor hides IPs; BT reveals proximity of contacts | Social graph exposure, tracking [S28] | Node IDs, positions often public by design | High (historic) | Network-level | Device names | Endpoint names |
| Internet | Optional (Nostr) | Optional (Tor) | Optional | None | None | None | None | None |
| Cross-platform | Yes (claimed binary compatibility) [S27a] | No iOS | Yes | Via hardware | No | N/A | No | Claimed |
| Background | iOS: `bluetooth-central` + `-peripheral` modes, wake-on-proximity pending connects; Android: foreground service [S27, S27a] | Android foreground service | Unknown | Radio does the work; phone optional | Android | N/A | Foreground-oriented | Undocumented on iOS page |
| License | **iOS: Unlicense; Android: GPL-3.0** | GPL-3.0 | Proprietary, subscription [S30] | GPL-3.0 | GPL/various (NOASSERTION) | Spec (SIG) | Proprietary OS | Apache-2.0 |
| Activity (last push) | 2026-10-11 (36.6k★) | 2026-10-13 | SDK updates ongoing per pub.dev | 2026-10-14 | 2022-06-02 | — | — | 2026-10-14 |
| Independent security review | None found (self-reported fixes) | Cure53 2017 [S32] | Two academic breaks [S28, S29] | CVE-2025-52464 [S34] | None found | Spec-level literature | Apple | None found |

## 2. Project deep dives

### 2.1 bitchat — closest prior art (source inspected, Level B)

**Architecture** (WHITEPAPER v2.0, 2026-07-06):
- Two transports sit behind one `Transport` interface: BLE mesh and Nostr.
- A `MessageRouter` prefers a live mesh link, falls back to Nostr, then to couriers.

**Facts verified in code:**
- `Info.plist` declares `UIBackgroundModes = bluetooth-central, bluetooth-peripheral`.
- `BLEService.swift` defines a single fixed service UUID (mainnet `F47B5E2D-…4B5C`) and one characteristic.
- `TransportConfig.swift`: `bleMaxCentralLinks = 6`, `bleBackgroundPendingConnectSlotReserve = 2`, timestamp skew limit 120 s.
- `BLERadioController.swift` implements **"wake-on-proximity"**. On backgrounding, it issues indefinite `connect()` calls to recently seen peripherals. The code comment says "iOS never expires it — the controller completes it whenever the peer comes back into range, waking (or relaunching) the app." This uses the documented fact that pending connections survive app suspension/termination (S09, S11). The technique is **worth testing for VYRO LINK**. Its limit: it only covers peers already seen, and with Android's rotating addresses it is unknown whether reconnecting to an old peripheral identifier works (**D**, experiment E2 in Doc 8).
- In background, scanning passes the service UUID filter (`scanForPeripherals(withServices: [serviceUUID])`) and `allowDuplicates` only when active. This is consistent with S10b.

**Store-and-forward design:**
- Couriers are chosen among *connected* peers.
- The opaque 16-byte recipient tag = HMAC(recipient static key, UTC day).
- Trust tiers: favourites get 5 deposits, verified peers 2, out of a 40-slot pool.
- Spray-and-wait with copy budget 4 (cap 8), 16 KiB, 24 h.
- Handover on a verified direct announce from the recipient.

**Self-declared weaknesses (S27 §8):**
1. A stable 8-byte sender ID derived from a never-rotating key.
2. Clear-text static keys and nickname in announcements.
3. Neighbour lists in announcements (they expose the adjacency graph).
4. Hop distance reveals the originator.
5. No forward secrecy for courier mail or Nostr envelopes.
6. Padding only for Noise frames.

**Assessment for VYRO LINK:**
- *Study:* router layering, courier quotas, wake-on-proximity, fragmentation (~469-byte fragments), duplicate suppression with jitter.
- *Redesign:* identity exposure; tag derivation (HMAC over the recipient's *static public key* means anyone who knows that key, i.e. any of the recipient's contacts, can compute the tag); forward secrecy.
- *Licensing:* the iOS code is public domain, so it is reusable without licence friction. Android is GPL-3.0, so reuse would force GPL on the Android app.
- *Strategic option:* **contributing to bitchat** instead of building VYRO LINK is a legitimate alternative (Doc 9 §6).

### 2.2 Briar (source inspected, Level B)

- `AndroidBluetoothPlugin.java` uses **Bluetooth Classic RFCOMM** (`listenUsingInsecureRfcommWithServiceRecord`, `createInsecureRfcommSocketToServiceRecord`). iOS does not expose RFCOMM to apps outside MFi. This is the main reason Briar's transport cannot be ported to iOS (inference, **D**+A: iOS public BT APIs are Core Bluetooth/BLE).
- Sync protocol records (`RecordTypes.java`): `ACK, MESSAGE, OFFER, REQUEST, VERSIONS, PRIORITY`. Max message body 32 KiB (`SyncConstants.java`). The OFFER/REQUEST/ACK pattern is a good model for VYRO LINK's link-session "summary → request → transfer" exchange.
- Group visibility per contact (`INVISIBLE / VISIBLE / SHARED`) means messages only move between contacts who share the group. **Briar does not relay private messages through strangers.** Its "mesh" is a social graph, not an opportunistic stranger network (inference from code, B−).
- Audit: Cure53 2017, 12 issues, 1 high (DNS leak via RSS), fixed [S32]. That audit is now 9 years old.
- **Reusable:** sync record design; transport-plugin abstraction (`plugin/{bluetooth,tcp,tor,file}`); QR key agreement concept. **Not reusable:** the RFCOMM transport (Android-only); the Java codebase under GPL-3.0 if VYRO LINK is not GPL.

### 2.3 Bridgefy (closed source; published analyses, Level B)

- In 2020: tracking, impersonation, no effective confidentiality, and network-wide DoS from a single crafted (compressed) message [S28].
- After adopting libsignal: MITM, broadcast impersonation, and a residual decompression-bomb DoS — "adopting libsignal is not enough" [S29].
- **Lessons:** (1) a strong message cipher does not fix identity binding or broadcast authenticity; (2) relays must never decompress or deeply parse untrusted payloads; (3) a mesh message format must be designed for adversarial input.
- **Reuse: reject** (proprietary licence [S30], security history).

### 2.4 Meshtastic (source inspected, Level B)

- `MeshTypes.h`: `HOP_RELIABLE 3`, `HOP_MAX 7`. `mesh.proto`: `hop_start − hop_limit` = hops travelled, trustworthy only from firmware 2.3.0.
- `CryptoEngine.h`: AES-CTR with a 128/256-bit channel PSK; `x25519` for DMs.
- CVE-2025-52464: duplicated vendor-flashed keys and insufficient RNG initialisation broke DM confidentiality (fixed 2.6.11/2.6.12) [S34]. **Lesson:** key generation must happen on the device at first run, with health-checked entropy.
- The phone ↔ node BLE GATT API (`BluetoothCommon.h`) is a clean example of a phone as *client* of a radio node.
- **Relevance:** an *alternative architecture* (Architecture D in Doc 5) where cheap LoRa nodes provide always-on relaying. Not part of the MVP because the brief requires commodity phones.

### 2.5 Serval Mesh (metadata only)

- Rhizome = epidemic store-and-forward sync of bundles between any two nodes that meet [S35].
- Last Serval Mesh app release listed is 0.93 (2016); `serval-dna` last push 2022-06-02. **Inactive.**
- Useful as a historical validation that epidemic bundle sync works between phones. Reject for reuse.

### 2.6 Bluetooth Mesh (spec level, A)

- Managed flooding over the advertising bearer, with TTL and a message cache. Phones participate through a **GATT proxy node** and are not relays [S36].
- Designed for provisioned device networks (lighting, sensors). It needs a shared network key, which conflicts with E2E per-user privacy and open participation.
- **Reject** as the VYRO LINK network layer. Ideas worth borrowing: TTL plus a message cache for dedup, and heartbeats.

### 2.7 Apple Multipeer Connectivity / Apple peer-to-peer Wi-Fi (A)

- "Apple peer-to-peer Wi-Fi is not documented for third-party use, so this mechanism only works between Apple devices" [S14].
- **Possible** later iPhone↔iPhone bulk-transfer upgrade. No Android interop. Background behaviour is undocumented in the sources reviewed (**D**).

### 2.8 Google Nearby Connections (docs A; repo B)

- Google states Nearby Connections is "fully-offline peer-to-peer" and that the API is available for Android and iOS with cross-platform communication [S26].
- Repo inspection: `internal/platform/implementation/apple/` contains `ble_medium`, `ble_gatt_server/client`, `ble_l2cap_socket`, `awdl`, `wifi_aware`, `wifi_hotspot`, `wifi_lan`. Recent commits (2026-09-28 "NC changes for AW") add Wi-Fi Aware on Apple. **But** the only iOS release tag is `v0.0.1-ios` (2021-11-16); builds use Bazel.
- **Constraints:** a single-hop session framework, so VYRO LINK would still need its own routing. Background behaviour on iOS is undocumented. On Android the API ships in Google Play services (dependence on GMS, which excludes de-Googled devices; inference **D**).
- **Recommendation: investigate further** as a candidate *secondary transport adapter* for high-throughput contact-to-contact links. Not the core.

### 2.9 OpenHaystack (source inspected, Level B)

Covered in Doc 2. AGPL-3.0. Research artefact only; **reject** for reuse.

### 2.10 Delay-Tolerant Networking standards (A)

- RFC 4838 (architecture), RFC 9171 (BPv7, Jan 2022), RFC 9172 (BPSec) define bundles, custody, endpoint IDs and block-level security. RFC 6693 PRoPHET (Experimental) defines encounter-history routing.
- **Assessment:** BPv7 is heavyweight for 0.25–2 KiB BLE messages. Its endpoint IDs are designed to be routable names, which conflicts with metadata minimisation.
- **Adopt the concepts, not the wire format:** bundle lifetime, hop count, deduplication by ID, custody vs delivery separation, and integrity of the primary block where possible. Decision recorded in ADR-006 (Doc 5).

## 3. Repository inventory (promising repositories)

| Repository | Owner | License | Last push | Platforms | Language | Entry points / relevant modules | Routing impl. | Crypto impl. | Build | Reuse strategy | Reasons to reject |
|---|---|---|---|---|---|---|---|---|---|---|---|
| github.com/permissionlesstech/bitchat | permissionlesstech | Unlicense | 2026-10-11 | iOS, macOS | Swift | `bitchat/Services/BLE/*`, `Protocols/*`, `Noise/*`, `WHITEPAPER.md` | Flood + source routes + couriers | Noise (own Swift impl over CryptoKit, not audited) | Xcode/SPM | **Study; selectively reuse iOS BLE adapter patterns** | No audit; identity design conflicts with VYRO LINK goals |
| github.com/permissionlesstech/bitchat-android | permissionlesstech | GPL-3.0 | 2026-10-11 | Android | Kotlin | (not inspected beyond README) | Same protocol | Noise | Gradle | Study only (GPL) | GPL |
| github.com/briar/briar (mirror) | Briar Project | GPL-3.0 | 2026-10-13 | Android, JVM | Java | `bramble-core/.../sync`, `plugin/*`, `keyagreement`, `rendezvous` | Contact sync | Bramble crypto | Gradle | Study sync and key agreement | GPL; Android-only transport |
| github.com/meshtastic/firmware | Meshtastic | GPL-3.0 | 2026-10-14 | MCU | C++ | `src/mesh/*Router*`, `CryptoEngine`, `PhoneAPI` | Managed flood, next-hop | AES-CTR, x25519 | PlatformIO | Study; optional Architecture D | Requires LoRa hardware |
| github.com/google/nearby | Google | Apache-2.0 | 2026-10-14 | Android, iOS, Windows… | C++/ObjC/Swift | `connections/`, `internal/platform/implementation/apple/*` | None (single hop) | Ukey2 (per Nearby docs; not verified) | Bazel | Investigate as secondary transport | iOS packaging immature; GMS dependency on Android |
| github.com/seemoo-lab/openhaystack | SEEMOO | AGPL-3.0 | 2026-08-17 | macOS, ESP32, micro:bit | Swift/C | `FindMy/*`, `Firmware/*` | N/A | P-224 ECIES | Xcode | Research reference only | AGPL; depends on private Apple services |
| github.com/servalproject/serval-dna | Serval | NOASSERTION | 2022-06-02 | Android/Linux | C | Rhizome | Epidemic sync | Custom | autotools | Historical study | Inactive |
| github.com/matrix-org/vodozemac | Matrix.org | Apache-2.0 | 2026-10-08 | Rust (any) | Rust | `olm`, `megolm` | — | Olm (Double Ratchet) + Megolm; audited 2022 [S45] | Cargo | **Candidate E2E library** (ADR-007) | Needs FFI bindings; famedly wrapper is AGPL |
| github.com/signalapp/libsignal | Signal | AGPL-3.0 | 2026-10-07 | Rust + Java/Swift/TS bindings | Rust | protocol crate | — | X3DH/PQXDH + Double Ratchet | Cargo | Reference implementation | AGPL forces the whole app to be AGPL |
| github.com/mcginty/snow | mcginty | Apache-2.0 | 2026-07-06 | Rust | Rust | Noise handshakes | — | Noise | Cargo | Candidate for link-layer Noise NN | Unaudited (none found) |

## 4. Protocol comparison (message path)

| Step | bitchat | Briar | VYRO LINK proposal (Doc 5–7) |
|---|---|---|---|
| Discovery advert content | Fixed service UUID; announces carry static keys + nickname | BT/LAN/Tor addressing between contacts | Fixed service UUID only; **no identity on air** |
| Link security | Noise XX between peers (static keys) | BTP with contact keys | **Noise NN (ephemeral) per link**: anonymous, encrypted against passive sniffers |
| Recipient addressing | 8-byte peer ID; courier tag = HMAC(static key, day) | Contact-scoped groups | **Pairwise-secret rotating tag** (only sender + recipient can compute) |
| Sender identity on wire | Yes (header) | Contacts only | **No** (inside E2E layer) |
| Stored-mail FS | No (Noise X) | Contacts' sessions | **Yes, via Olm prekeys exchanged at pairing** (ADR-007) |
| Relay of strangers' mail | Yes (quota-limited couriers) | No | Yes (quota-limited, spray-and-wait) |
| Delivery receipt | Inside Noise session | ACK records | Authenticated receipt envelope + optional delivery "vaccine" (Doc 7) |

## 5. Licensing review

| If VYRO LINK wants to… | Compatible sources | Incompatible unless VYRO LINK adopts that licence |
|---|---|---|
| Ship closed-source or permissive | bitchat iOS (Unlicense), vodozemac/snow/nearby (Apache-2.0), drift/bluetooth_low_energy (MIT), protobuf.dart/flutter_secure_storage/mobile_scanner (BSD) | Briar, Meshtastic, bitchat-android (GPL-3.0); libsignal, OpenHaystack, dart-vodozemac (AGPL-3.0) |
| Use flutter_blue_plus commercially | — | **Commercial licence required for for-profit use** (FBP License v1.6, S47) |

**Action:** decide the product licence before PoC (Doc 10, R-14). All conclusions here are engineering analysis, **not legal advice**.

## 6. Maintenance and dependency health

- Healthy (pushes within 30 days): bitchat (both), Briar, Meshtastic, google/nearby, vodozemac, drift, flutter_secure_storage, mobile_scanner, libsodium bindings.
- Slower: `dint-dev/cryptography` (last push 2025-11-21), `bluetooth_low_energy` (2026-01-17). Single-maintainer risk.
- Inactive: Serval; `signalapp/libsignal-protocol-java` (archived).

## 7. Reuse recommendations (summary)

| Component | Recommendation |
|---|---|
| bitchat iOS BLE patterns (dual role, wake-on-proximity, fragmentation) | **Study + prototype** (public domain) |
| bitchat protocol as-is | **Reject as the wire protocol**: identity exposure conflicts with goals. Optional interop is out of scope. |
| Briar sync record model | **Adopt the concept** (OFFER/REQUEST/ACK) |
| vodozemac (Olm) | **Prototype** (ADR-007) |
| libsignal | **Reject** unless the product is AGPL |
| Noise (snow) for link layer | **Prototype** |
| Nearby Connections | **Investigate further** (secondary transport) |
| Meshtastic | **Defer** (Architecture D) |
| Bridgefy, Serval, OpenHaystack | **Reject** |
