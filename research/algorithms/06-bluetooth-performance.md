# 06 — Bluetooth Performance Optimization Report (design for Phase 7)

**Status:**
- **Nothing here is measured on devices.** There is no Android phone, and the iPhone is offline.
- Platform facts are labelled VERIFIED (official documentation) or REPORTED (developer forums or third-party write-ups).
- This document prepares the native hosts. It does not claim performance.

## 1. Platform constraints that shape every algorithm

| Constraint | Label | Source (accessed 2026-10-15) |
|---|---|---|
| iOS background peripheral: local name dropped; service UUIDs moved to an "overflow area" visible only to iOS devices scanning for that UUID | VERIFIED (Apple *Core Bluetooth Programming Guide*, background execution; API docs) | developer.apple.com Core Bluetooth guide; forums thread 126461 |
| iOS background central: must scan with service UUIDs; `AllowDuplicates` ignored; discoveries coalesced | VERIFIED (API docs) / REPORTED (forums) | CBCentralManager docs; forums thread 93758 |
| iOS force-quit app is not relaunched for Bluetooth events | VERIFIED (TN3115; Doc 04) | Doc 12 |
| Android: unfiltered scans stop when the screen is off; use `ScanFilter`s | VERIFIED | developer.android.com `BluetoothLeScanner` reference |
| Android: about 5 `startScan` calls per 30 s; extra starts silently return nothing | REPORTED (not in official docs) | punchthrough.com *Android BLE scan errors*; proandroiddev |
| Android: scans longer than ~30 min become opportunistic | REPORTED | punchthrough.com |
| Android 14+: the first `requestMtu` sets 517; later requests on that ACL link are ignored | VERIFIED | developer.android.com Android 14 behaviour changes |
| Android 14+: a foreground service needs a declared type (`connectedDevice`) | VERIFIED | developer.android.com fg-service-types |
| L2CAP CoC: iOS 11+ `openL2CAPChannel`; Android 10+ `createL2capChannel` | VERIFIED (API docs; Doc 04) | — |
| Reported throughput: iOS GATT notify ≈ 300–400 kbps; L2CAP device-to-device 37–138 kbps; Android-to-peripheral notify > 1,000 kbps | REPORTED (forums; varies by device) | developer.apple.com forums 770717, 89644; Nordic DevZone |

## 2. Discovery scheduling

**Goal:** find peers with the fewest radio-on milliseconds.

- **Android:**
  - Keep **one long-running filtered scan** (service-UUID `ScanFilter`) instead of toggling, which avoids the 5-per-30 s throttle.
  - Re-arm it every < 30 min to avoid demotion to opportunistic.
  - In the relay (foreground service, `connectedDevice`) mode, use `SCAN_MODE_LOW_POWER` and raise to balanced only while the app is open.
- **iOS:** background scanning must name the service UUID; foreground uses the same UUID.
- **iPhone ↔ Android in the background:** a backgrounded iPhone's advert is invisible to Android. So:
  - **the iPhone initiates** (ADR-013 / Doc 14);
  - Android advertises the UUID continuously in relay mode.
- **Privacy:** the advert carries only the fixed service UUID. No rotating identifiers are needed because none are broadcast.

## 3. Connection, MTU and payload batching

- **Negotiate the MTU once.**
  - Android requests 517; under Android 14 rules the first request wins.
  - iOS reports `maximumWriteValueLength(for:)`.
- **Write size:** usable payload per write = min(MTU − 3 for notifications, `maximumWriteValueLength`) for writes. The ATT header subtraction follows the Android 14 guidance.
- **Feed `link_receive` with whatever chunk size arrives.** The Rust `Deframer` reassembles frames up to `MAX_FRAME_LEN` (6,917 B) and is fuzzed. End-to-end tests use 20 B and 185 B MTUs.
- **Batching:** a link message is already one frame. The host should concatenate the bytes returned by `link_receive` and slice them into MTU-sized writes, so many small frames share one ATT packet. This is pure host logic: no protocol change, and no duplicate business logic, because the Rust core decides *what* to send and the host only decides *how to chunk it*.
- **Flow control:**
  - iOS: write-without-response must respect `canSendWriteWithoutResponse` / `peripheralIsReady(toSendWriteWithoutResponse:)`.
  - Android: wait for `onCharacteristicWrite` (or the Android 13+ write-status return) before the next write.
  - This is the platform's backpressure. No additional sliding window is needed: the GATT/ATT and L2CAP links are reliable and ordered per connection, and custody acknowledgements already give per-envelope selective acknowledgement (01 E2).
- **Prefer L2CAP CoC when both sides offer it.** `HELLO` has `l2cap_psm`, and Experiment A measures it.

## 4. Reconnection and encounter scheduling

- **Backoff:** exponential backoff with **full jitter**, `sleep = random(0, min(cap, base·2^attempt))` (Brooker, AWS 2015), applied per peer advert (keyed by the platform's peripheral identifier, which already rotates).
  - Suggested starting values for Experiment C: base 2 s, cap 5 min.
  - Reset after a successful session.
- **Cooldown:** after a complete session with nothing to exchange (no REQUEST either way), skip that peer for a cooldown period, so two stationary phones don't reconnect continuously. To be tuned in Experiment C.
- **Where it lives:** a pure function is suitable for the Rust core (`vl-core`) so iOS and Android share it, e.g. `next_attempt(peer_hash, outcome, now)`. **Not implemented yet:** it only has value once native hosts exist (YAGNI). It is listed in the roadmap with its tests.

## 5. What Experiment A/C must measure (cannot be simulated)

- Encounter durations and setup times per OS state.
- Achieved bytes per second for GATT and L2CAP, per direction.
- Battery per hour in relay mode.

These numbers replace the simulator's assumptions (20 kB/s goodput, exponential encounter durations) and decide whether compact SUMMARY entries (03 §6) are urgent.
