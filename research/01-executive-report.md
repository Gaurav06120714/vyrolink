# Document 1 — Executive Research Report: VYRO LINK

> **Final decisions now live in [Doc 18](18-development-blueprint.md)** (architecture freeze, 2026-10-14). Where this report differs (wrap AEAD, delivery states, core language), Doc 18 wins.

**Date:** 2026-10-14
**Scope:** research only. No application code was written, no project was initialised, and nothing was installed. All claims carry evidence levels **A** (official), **B** (reproducible / source-inspected), **C** (credible, unverified) or **D** (hypothesis). Sources are listed in Doc 12.

## Recommendation: **CONDITIONAL GO**

The architecture is technically plausible, and a production existence proof of most of its mechanics already exists (bitchat, [S27]). **Three physical-device experiments must succeed before any product development** (§6). If the first one fails, the project should pivot to dedicated relay hardware (Architecture D) or stop.

---

## 1. Product vision and problem

VYRO LINK aims to deliver **private 1:1 text messages between iPhones and Android phones with no internet, cellular service or servers**. Messages are carried *store-and-forward* by whichever VYRO LINK devices happen to come near each other, and an authenticated acknowledgement returns to the sender.

## 2. Existing solutions (Doc 3)

- **bitchat:** a cross-platform BLE mesh with couriers. Its iOS code is public domain and its Android code is GPL. It is the closest prior art, and its own documentation lists weak metadata privacy and no forward secrecy for carried mail.
- **Briar:** Android-only and contacts-only. It is the strongest audited design.
- **Bridgefy:** proprietary and broken twice in academic analyses.
- **Meshtastic:** requires LoRa hardware.
- **Serval:** inactive.
- **Bluetooth Mesh:** phones are proxy clients, not relays.
- **Apple Find My:** not a transport at all; it is an internet-backed reporting system (Doc 2).

## 3. Key findings

| # | Finding | Evidence |
|---|---|---|
| K1 | Find My is one-hop to Apple's servers over the internet; there is no third-party payload API. Its reusable lessons are rotating unlinkable identifiers and encrypt-to-recipient through untrusted relays. | A [S01]; B [S02, S03] |
| K2 | **BLE is the only transport usable between iOS and Android, by third-party apps, without per-peer pairing UI, in the background.** | A [S10, S12–S14, S17–S19] |
| K3 | A **backgrounded iPhone's advertisement cannot be decoded by Android** (the overflow area is Apple-only). Discovery is asymmetric: background iPhones must initiate connections to Android devices. | A [S10, S15] |
| K4 | **A force-quit iPhone is never relaunched for Bluetooth events**, and after a reboot only after first unlock. iOS background work comes in ~10 s wakes and may be throttled. **An iPhone cannot be a dependable unattended relay.** | A [S09, S11] |
| K5 | Android can relay continuously **only with a foreground service** (`connectedDevice`), which is subject to Google Play review and OEM battery killers. | A [S18, S19, S24]; C |
| K6 | **Wi-Fi Aware is now available on iOS 26+**, but iOS requires a system pairing UI per device, and iPhone↔Android interop failures are reported. It is a future contact-to-contact accelerator, not a relay transport. | A [S12, S13]; C [S16] |
| K7 | Spray-and-wait (bounded copies) plus expiry is the right MVP routing. Epidemic and flood routing are too costly and abusable; encounter-history routing conflicts with rotating-tag privacy. | B [S39–S41]; bitchat independently chose spray-and-wait [S27] |
| K8 | Content can have forward secrecy despite days-long storage on strangers' phones: **Olm (Double Ratchet) via vodozemac (Apache-2.0, audited 2022)**, with one-time prekeys exchanged at in-person QR pairing. libsignal is AGPL; Noise one-way seals lack forward secrecy and are replayable. | A [S42, S43]; B [S45] |
| K9 | No numeric throughput, latency, range or battery figures for modern iOS 26/27 background BLE exist in the sources found. The best published GATT figures are ≈311–365 kbps on iPhones and ≈1.2 Mbps on Android (vendor test, older phones). | C [S49]; gap |
| K10 | Licensing traps: flutter_blue_plus requires a commercial licence for for-profit use; libsignal, OpenHaystack and Briar/Meshtastic/bitchat-android are AGPL/GPL. | B [S46, S47] |

## 4. Technical feasibility

| Question | Verdict |
|---|---|
| Offline direct messaging iOS↔Android | **Feasible** (documented primitives; bitchat claims cross-platform operation, C) |
| Multi-hop store-and-forward | **Feasible in principle**, best-effort; delivery depends on density, movement and expiry (D → E7, E9) |
| Always-on unattended relaying on iPhones | **Not feasible as a guarantee** (A) |
| Always-on relaying on Android | **Feasible with an opt-in foreground service**, subject to policy and OEMs (A + Q) |
| E2E security through untrusted relays | **Feasible** with standard primitives and an audited library |
| Strong metadata privacy | **Partially**: presence of a device is observable; relays see tag grouping per epoch |

## 5. Main constraints

1. iOS background limits and the force-quit rule (K4).
2. Asymmetric cross-platform discovery (K3).
3. Delivery depends on human mobility and install density (R-04).
4. Low BLE throughput, so text only in the MVP.
5. Play policy for relay foreground services (R-07).

## 6. Critical unknowns and the three first experiments

1. **E1 + E2:** does a **backgrounded, locked iPhone** complete link sessions with an **Android relay (FGS)** and with other iPhones, repeatedly over hours, within the ~10 s wake budget?
2. **E7:** does **three/four-device store-carry-forward** across separated encounters deliver, with an authenticated receipt back to the sender, and survive app restarts?
3. **E5:** what does Android Relay mode and iOS background participation cost in **battery**?

## 7. Recommended direction

- **Architecture C-with-A:** one portable core (protocol, crypto, routing, storage) with thin native BLE adapters and a Flutter UI. The core language (provisionally Rust) is decided after the PoC.
- **BLE** dual-role on both platforms.
- **Routing:** direct delivery + binary spray-and-wait + expiry + delivery vaccines.
- **Pairing:** contacts-only, in-person QR.
- **Encryption:** Olm inside a pairwise wrap, with **pairwise rotating recipient tags** and **anonymous Noise NN links**.
- **Platforms:** Android opt-in Relay mode; iPhone best-effort with user education.

Also consider **contributing to bitchat** instead (Doc 9 §6.2).

---

## 8. Final architectural decision: answers to the 20 questions

| # | Question | Answer | Basis |
|---|---|---|---|
| 1 | Feasible without internet? | **Yes, for best-effort delivery.** Every hop is device-to-device BLE; no server or infrastructure is required. | K2, Doc 5 |
| 2 | When can multi-hop work? | When a time-ordered chain of encounters A→…→C happens **before expiry**, the participating apps are not force-quit or stopped, Bluetooth is on, and relays have quota left. Dense, mobile groups help; sparse ones may never deliver. | Doc 4 §2, Doc 5 §5 |
| 3 | Can iPhones and Android phones form one network? | **Yes over BLE**, with asymmetry: background iPhones must initiate to Android; two background Androids without a foreground service rarely meet. | K3, Doc 4 §2 |
| 4 | Primary transport? | **BLE** (GATT; L2CAP CoC as a measured upgrade). | ADR-001 |
| 5 | Meaningful secondary transports? | Later: **Wi-Fi Aware between paired contacts** (bulk/media), Apple P2P (iOS↔iOS), possibly **Nearby Connections**. None needed for the MVP. | K6, Doc 4 §6 |
| 6 | BLE for discovery, data, or both? | **Both.** Adverts carry only a fixed service UUID; data goes over Noise-encrypted link sessions. | ADR-001/008 |
| 7 | MVP routing algorithm? | **Direct + binary spray-and-wait** (initial L = 4, cap 8), expiry (default 72 h), hop limit 8, delivery vaccines, one bounded sender retry. Parameters are set by simulation (E9). | ADR-003 |
| 8 | Surviving disconnected encounters? | Envelopes persist in an encrypted **Class R** store, readable after first unlock. iOS state restoration and Android FGS restart resume operation; expiry bounds all storage. | ADR-010, Doc 7 §6 |
| 9 | Identifying the recipient without a server? | **Pairwise, epoch-rotating HMAC tag** derived from a secret established at in-person QR pairing. Only the sender and recipient can compute or recognise it. | ADR-009, Doc 6 §4 |
| 10 | Relays forwarding without reading? | **Two-layer E2E:** Olm (Double Ratchet) inside a pairwise ChaCha20-Poly1305 wrap (RFC 8439; ADR-014 in Doc 18, originally XChaCha20). Relays see only a fixed header. | ADR-007, Doc 7 §3 |
| 11 | Cryptographic architecture? | In-person mutual QR pairing → X25519/HKDF pairwise keys; **vodozemac Olm** with one-time keys exchanged at pairing; **Noise NN** anonymous links; SQLCipher stores keyed via Keychain/Keystore. Independent review required. | Doc 6 |
| 12 | Metadata relays must see? | Random envelope ID, recipient tag, coarse expiry, size bucket, hop count, copy budget, ack commitment. **Not:** sender, recipient identity, content, precise timestamps. | Doc 7 §3.1 |
| 13 | Backgrounded / terminated app? | **iOS:** service-filtered scanning, overflow-only advertising, ~10 s wakes, relaunch for pending BT events; **force-quit = nothing**. **Android:** full operation with FGS, short bursts via `PendingIntent` without it; force-stop = nothing. | Doc 4 §3–4 |
| 14 | iPhone as reliable unattended relay? | **No.** Best-effort only; no OS guarantee; force-quit ends participation. | K4 |
| 15 | Most significant limitation? | **Delivery is opportunistic and unbounded in time**, and it is undermined further by iOS background limits. Users must understand that "sent" is not "delivered". | R-01, R-04 |
| 16 | Best reusable foundation? | **bitchat's iOS BLE layer** (public domain) as a pattern and possibly a code source; **vodozemac** for E2E; Briar's sync model as design reference. No complete project is adopted. | Doc 3 §7 |
| 17 | Build independently? | Envelope format, routing engine, pairwise tag/wrap scheme, link protocol, storage classes, delivery-state model, and the Android adapter (bitchat-android is GPL). | Docs 5–7 |
| 18 | Smallest viable PoC? | **3–4 physical phones (≥1 iPhone, ≥1 Android) running minimal native test apps** that exchange fixed-size dummy envelopes over BLE, store-carry them across separated encounters, and return a receipt. No Flutter, no real crypto yet. | Doc 11 P1 |
| 19 | First three experiments? | **E1/E2** (background-iPhone ↔ FGS-Android link viability over hours), **E7** (separated-encounter multi-hop + receipt + restart), **E5** (battery). | Doc 8 |
| 20 | What justifies abandoning or changing course? | (a) E1/E2 show background iPhones rarely or never complete sessions with Android relays → pivot to Architecture D or Android-only; (b) one session cannot fit an iOS wake budget → iPhones become clients only; (c) E5 relay cost unacceptable to pilot users; (d) Play rejects Relay mode with no viable fallback; (e) E9/E12 delivery ratios unusable at realistic densities; (f) bitchat closes the privacy/forward-secrecy gaps first → contribute rather than compete. | Doc 10 gates |

## 9. Go/no-go justification

- **GO elements:** documented platform primitives exist on both OSes (A). A comparable system ships today (bitchat, B/C). Standard, audited cryptography fits the DTN setting (A/B).
- **Conditions:**
  - G1 (E1/E2) and G3 (E5) must pass before architecture approval.
  - G2 (E7/E9) must pass before MVP build-out.
  - G4 (independent security review) must pass before release.
- **Why not NO-GO:** no critical requirement is shown impossible. The hardest one (iPhone relaying) is degraded, not eliminated, and the design does not depend on iPhones being relays.
