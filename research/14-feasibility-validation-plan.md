# Document 14 — Technical Feasibility Validation Plan

**Project:** VYRO LINK · **Phase:** P1 preparation (feasibility validation), research only · **Date:** 2026-10-14
**Builds on:** Docs 00–13 in this directory · **Evidence levels:** A official · B reproducible/source-inspected · C credible, unverified · D hypothesis (Doc 12)

> **Status of every experiment in this document: NOT TESTED.**
> No physical-device measurement has been made. No test application exists. Nothing was installed or configured while preparing this plan. Application development is **not authorised** by this document.

---

## 1. Executive summary

VYRO LINK's research verdict remains **CONDITIONAL GO** (Doc 01). This document turns the conditions into **three executable, pre-registered experiments**. Their outcomes decide whether development may start.

| Experiment | Question it answers | Maps to (Doc 08) | Can start with existing tools? |
|---|---|---|---|
| **A: iPhone ↔ Android BLE across iPhone app states** | Can an iPhone that is foregrounded, backgrounded, locked, suspended or force-quit exchange data with an Android device, and who must initiate? | E1, E2, E3 (partial), E14 | **Partially.** Foreground interop baseline only (A0). Background, locked, suspended and force-quit behaviour of *our* design needs a temporary probe app. |
| **B: Multi-hop store-and-forward without internet** | Does A → B → C deliver an encrypted, recipient-authenticated message with a returned receipt, including time-separated encounters, duplicates, restarts and expiry? | E7, E8, E13, E15 + security checks | **No.** Requires the probe app. |
| **C: Battery cost** | What do discovery, advertising, idle connections and forwarding cost on each platform? | E5 | **Partially.** Generic scan cost via tools; cost of *our* duty cycle needs the probe app. |

**Re-verification of platform facts (2026-10-14, §3).** The architectural premises in Docs 04–05 hold. Two evidence levels were upgraded:
- Android stops unfiltered scans when the screen is off: C → **A** [S67].
- L2CAP CoC API levels: → **A** [S68, S69].

Three new facts matter:
- Android 17 (API 37) **deprecates the classic `connectGatt` overloads** in favour of `connectGatt(BluetoothGattConnectionSettings, Executor, BluetoothGattCallback)` [S68].
- Android 16 job quotas apply **even while a foreground service runs** [S71].
- Apple publishes **no** Bluetooth address-rotation interval [S73].

**Largest open question (unchanged):** whether a *backgrounded, screen-locked* iPhone completes link sessions with an Android relay often enough, and fast enough to fit a ~10 s wake. Android-initiated *new* connections to a backgrounded iPhone are **not supported by documented means** (§3.3), so the design relies on iPhone-initiated links.

**Environment (§4):** an Apple M2 Pro MacBook Pro (16 GB, macOS 26.6.2) with Xcode 27.0 and the iOS 27.0 SDK, and **one paired iPhone (iOS 27.0, currently offline)**. There is **no Android device, no Android tooling, no JDK, and no Flutter/Dart**. Experiments A–C cannot start until Android hardware and tooling are obtained, with the user's explicit authorisation.

---

## 2. Audit of the existing research

### 2.1 Files verified (paths checked on disk, 2026-10-14)

`research/00-INDEX.md` … `research/13-re-toolkit-and-lab.md` all exist (14 files). This document adds `14-feasibility-validation-plan.md`.

The audit method was a targeted review of the documents named in the brief (00, 01, 04–08, 10, 12):
- consistency of key parameters, checked by grep across all files;
- cross-document identifiers (S/E/ADR/R/T IDs all resolve);
- re-verification of disputed platform claims against primary sources (§3).

**Not done:** a line-by-line re-verification of every citation in Docs 02, 03, 09, 11 and 13. Those documents were **not** re-validated beyond the identifier and parameter checks.

### 2.2 Established decisions (unchanged)

| Area | Decision | Source |
|---|---|---|
| Architecture | C-with-A: portable core + native BLE adapters + Flutter UI | Doc 05 §1, ADR-001/002 |
| Transport | BLE, dual role on every device; adverts carry only a fixed service UUID | ADR-001, Doc 07 §1 |
| Routing | Direct delivery + binary spray-and-wait; L = 4 (cap 8); hop limit 8; default expiry 72 h; delivery vaccines; one bounded sender retry | ADR-003, Doc 05 §5, Doc 06 §7 |
| Addressing | Pairwise epoch-rotating HMAC tags, E = 6 h | ADR-009, Doc 06 §4 |
| E2E | Olm (vodozemac) inside pairwise ChaCha20-Poly1305 wrap (ADR-014, Doc 18) | ADR-007 |
| Links | Anonymous Noise NN per BLE session | ADR-008 |
| Storage | Class R (after first unlock) / Class C (unlocked only) | ADR-010 |
| Platform roles | Android opt-in Relay mode (FGS `connectedDevice`); iPhone best-effort | ADR-011 |
| Delivery states | Final model in Doc 18 §7.4 (QUEUED / SENT_TO_NEARBY / ACCEPTED_BY_RELAY / ACKNOWLEDGED / READ / EXPIRED / DELIVERY_UNKNOWN / FAILED) | Doc 18 (supersedes Doc 07 §7) |

The parameter grep found these values used consistently in Docs 01, 05, 06 and 07.

### 2.3 Unresolved assumptions this phase must test

| ID | Assumption | Where assumed | Tested by |
|---|---|---|---|
| U1 | A backgrounded/locked iPhone discovers an Android relay (filtered background scan) and completes a session | Doc 04 §2, ADR-011 | A2–A3 |
| U2 | One full session (connect → Noise NN → HELLO → SUMMARY → ≥ 1 envelope) fits the ~10 s wake [S09] | Doc 08 H3 | A2–A4 |
| U3 | Pending connects / state restoration bring a system-terminated iPhone app back into the network | Doc 04 §3, bitchat pattern [S27] | A4 |
| U4 | Class R data survives restarts and is readable while locked after first unlock | ADR-010 | B2, B5 |
| U5 | Receipts can find their way back through the same opportunistic process | Doc 07 §7 | B1–B2 |
| U6 | Relay energy cost is acceptable to volunteers | ADR-011, R-06 | C |
| U7 | Android FGS relaying survives screen-off on the tested OEMs | Doc 04 §4 | A, C |

### 2.4 Contradictions found and resolved

| # | Contradiction | Resolution (targeted edit) |
|---|---|---|
| X1 | Doc 11 P2 said ADR-004 (core language) is updated at P2, but ADR-004, R-16 and R-22 say it is decided only after P3 measurements | Doc 11 P2 now reads: ADR-004 stays provisional; final decision at the end of P3 |
| X2 | Doc 08 E1 set its success threshold "X % within T" from the product requirement in Doc 09 §5, but Doc 09 §5 sets that requirement *after* the E1 baselines (circular) | Doc 08 E1 now points to the decision rules in §11 of this document. Product targets remain post-baseline (Doc 09 §5) |
| X3 | Doc 04 labelled two now-verified facts as C / "not re-read" (screen-off unfiltered scans; L2CAP API levels) | Doc 04 updated to A with sources S67–S69 |

No contradiction was found between the routing design (Doc 05 §5), the threat model (Doc 06 §8), the protocol (Doc 07) and the risk register (Doc 10).

---

## 3. Platform feasibility re-verification (primary sources, checked 2026-10-14)

### 3.1 iOS (Core Bluetooth). Current release iOS 27.0.x; Apple docs reviewed are labelled iOS 26

| Claim | Status | Evidence |
|---|---|---|
| Background scanning requires explicit service UUIDs; scan options have no effect in the background | **Guaranteed (documented)** | [S10b] |
| Peripheral adverts accept only local name + service UUIDs; 28 bytes in the foreground; in the background the name is dropped and UUIDs go to the overflow area, discoverable only by iOS devices scanning for them | **Guaranteed (documented)** | [S10] |
| A non-Apple central sees a background iPhone's advertisement but cannot read the overflow-area information needed to recognise it | **Documented by Apple staff (forum)** | [S15] (A−) |
| With `bluetooth-peripheral`, the system wakes the app for read/write/subscription events from connected centrals | **Guaranteed (documented, archived 2013 guide)** | [S09], to re-validate on iOS 27 |
| ~10 s per wake; excessive background time → throttled or killed | **Documented (old)**; actual budget on iOS 27 unknown | [S09] → measure (A) |
| Relaunch rules: removed from memory / crash / reboot (after first unlock) → yes; **force quit → no**; Bluetooth power toggled → no; iOS 26+: relaunch in noted cases only for AccessorySetupKit apps | **Guaranteed (documented)** | [S11] (rev. 2025-09-15; no iOS 27 revision found) |
| Relaunch happens **only if** a pending BT request's event occurs (scan, connect, subscribed notification, actively advertising) | **Guaranteed (documented)** | [S11] |
| L2CAP CoC listener `publishL2CAPChannel` iOS 11+; PSM discovery is the app's responsibility | **Guaranteed (documented)** | [S69] |
| BLE address randomised "on a frequent basis"; no interval published | **Documented (no number)** | [S73]; a 15-minute figure appears only in forum posts (C) |
| iOS 27 Core Bluetooth changes | **No primary-source change notes found**; one iOS 27 *beta* disconnect report | [S74] (C) → re-test on iOS 27.x |

### 3.2 Android (Android 12–17; 17 = API 37)

| Claim | Status | Evidence |
|---|---|---|
| Runtime permissions `BLUETOOTH_SCAN` (`neverForLocation`), `BLUETOOTH_ADVERTISE`, `BLUETOOTH_CONNECT` (API 31+) | **Guaranteed** | [S17] |
| **Unfiltered scans stop when the screen turns off**; use `ScanFilter` to avoid this | **Guaranteed (documented)**, upgraded from C | [S67] |
| `startScan(filters, settings, PendingIntent)` delivers results when the process is not running | **Documented** | [S67], [S18] |
| ~5 scan starts / 30 s throttle | **Best-effort knowledge (C)**; only the error constant is documented | [S25] |
| FGS can't be started from the background (API 31+) except via listed exemptions: user-visible transition, user action on a notification, BOOT_COMPLETED, **battery optimisations turned off by the user**, Companion Device Manager permissions, exact alarm, etc. | **Guaranteed** | [S70] |
| `connectedDevice` FGS type and prerequisites; Play declaration and review | **Guaranteed (policy discretionary)** | [S19], [S24] |
| Android 15: `dataSync` limited to 6 h/24 h; `connectedDevice` not on the BOOT_COMPLETED prohibited list | **Documented**; allowed-from-boot is an inference (D) | [S20] |
| Android 16: job runtime quotas apply even when running alongside an FGS | **Documented** | [S71] |
| Android 17: no BLE scan/advertise changes listed; autonomous re-pairing for bond loss; legacy `connectGatt` overloads **deprecated** in API 37 → `connectGatt(BluetoothGattConnectionSettings, Executor, BluetoothGattCallback)` | **Documented** | [S72], [S68] |
| L2CAP CoC `createL2capChannel` / `createInsecureL2capChannel` API 29, LE only | **Documented** | [S68] |
| GATT connections close when the process is killed | **Documented** | [S18] |
| OEM battery managers kill FGS | **Best-effort / variable (C)** | Doc 10 R-08 |

### 3.3 Can a backgrounded iPhone reliably receive connections *initiated by* an Android device?

**Answer: not for new peers by any documented means. Possibly for re-connections within a short window, which must be tested.**

1. **Accepting** a connection and serving GATT while backgrounded is supported. The OS handles the link layer, and the app is woken for reads/writes/subscriptions ([S09], A).
2. **Finding** the iPhone is the problem. Its background advert carries the VYRO LINK service UUID only in the overflow area, which Android cannot interpret [S10, S15]. An Android central therefore cannot tell a VYRO LINK iPhone apart from any other Apple device.
3. Two non-documented workarounds exist. Both are **diagnostic-only** in this plan, not design options:
   - (a) *Reconnect to the last-known address* of an iPhone met while it was foregrounded or connected. This works only until the iPhone's address rotates, and Apple publishes no interval [S73] (D → A2-R).
   - (b) *Probe every Apple-manufacturer advert* by connecting and checking GATT. It relies on an undocumented format, costs battery, and connects to strangers' Apple devices. **Not acceptable for production** (privacy and abuse).
4. **Consequence (already in Doc 04 §2): the design must rely on iPhone-initiated connections to Android relays.** Experiment A measures both directions to confirm this.

### 3.4 Can an iPhone serve as a dependable unattended relay?

**No (A).**
- **Hard exclusions:** force-quit and Bluetooth power-off [S11]. Before first unlock after a reboot it does not run at all [S11].
- **What remains is best-effort:** event-driven wakes of roughly 10 s, throttled at the OS's discretion [S09].

Experiment A measures *how much* best-effort relaying actually happens. It cannot turn best-effort into a guarantee.

### 3.5 Guaranteed vs best-effort vs needs real devices (summary)

| Behaviour | Guaranteed | Best-effort | Needs device testing |
|---|---|---|---|
| iPhone FG ↔ Android: discovery + GATT both directions | ✔ (documented primitives) | | Interop quirks (MTU, timing) |
| BG iPhone → Android relay (iPhone initiates) | Mechanism documented | Discovery latency, wake budget | **Yes (A2, A3)** |
| Android → BG iPhone (new peer) | ✘ not documented | | Confirm negative (A2) |
| BG iPhone ↔ BG iPhone | Overflow discovery documented | Slow, throttled | Yes (A-ext) |
| System-terminated iPhone app back in network | Relaunch rules documented | Depends on pending requests | **Yes (A4)** |
| Force-quit iPhone | ✘ excluded (documented) | | Confirm (A5) |
| Android FGS relaying with screen off | FGS allowed, filtered scans continue | OEM kills, Doze | **Yes (A, C)** |
| Persistence across restarts | Platform storage classes documented | | **Yes (B2, B5)** |
| Energy | — | — | **Yes (C)** |

---

## 4. Verified development environment (read-only checks, 2026-10-14)

| Item | Finding | Command used (read-only) |
|---|---|---|
| macOS | **26.6.2** (build 25G83) | `sw_vers` |
| Architecture / CPU / RAM | arm64, **Apple M2 Pro**, **16 GB** (17,179,869,184 bytes) | `uname -m`, `sysctl` |
| Disk (data volume) | 460 GiB total, **131 GiB free** (70 % used) | `df -h ~` |
| Xcode | **27.0** (27A266a), iOS SDK **27.0**, at `/Applications/Xcode.app` | `xcodebuild -version`, `xcrun --show-sdk-version` |
| Command Line Tools package | 26.5.0 receipt present | `pkgutil --pkg-info` |
| Flutter / Dart | **Not found** on PATH | `command -v` |
| Java | **No Java runtime** (macOS stub only) | `/usr/libexec/java_home` |
| Android tooling | **adb, sdkmanager, emulator not found**; `ANDROID_HOME` unset; no Android Studio; no `~/Library/Android/sdk` | `command -v`, `ls` |
| Git | 2.55.0 | `git --version` |
| CocoaPods | 1.17.0 (prints a UTF-8 locale warning; nothing changed) | `pod --version` |
| Node | v24.18.0 | `node -v` |
| Physical Apple devices | **1 iPhone paired with Xcode**: model identifier `iPhone17,4`, **iOS 27.0**, state **unavailable/offline** at check time. Marketing name not verified. UDID intentionally omitted. | `xcrun devicectl list devices`, `xcrun xctrace list devices` |
| Android devices | **Unknown**: no adb to enumerate; no Android USB device seen | `system_profiler SPUSBDataType` |
| Mac Bluetooth controller | Broadcom **BCM_4388**, firmware 23.5.574.4413, state On; supports GATT and LE Audio | `system_profiler SPBluetoothDataType` |
| BLE sniffer hardware | **None detected** (no Nordic USB device) | `system_profiler SPUSBDataType` |
| Not checked | Apple Developer Program membership type; PacketLogger presence; nRF Connect/LightBlue on the iPhone | — (would need account access or an unlocked device) |

**Implications:**
- iOS-side probe development is technically possible on this Mac once authorised.
- **Android-side work is blocked** until Android hardware and tooling (Android Studio/adb, which bundles a JDK) are obtained **with explicit user authorisation**. This is out of scope for this phase.

---

## 5. Instrumentation: what existing tools can test vs what needs a temporary probe app

### 5.1 Testable with existing diagnostic tools only (no code)

| Test | Tools | What it proves | What it cannot prove |
|---|---|---|---|
| **A0** Foreground cross-platform GATT baseline: Android advertises a custom service UUID + GATT server; the iPhone discovers, connects, reads/writes, negotiates MTU; and the reverse direction | **nRF Connect for Mobile** (Android: advertiser + GATT server; iOS: scanner/client). For an iOS-side GATT server, a third-party virtual-peripheral app such as **LightBlue** (feature availability **to verify**, C) | Radio + OS-level interop on this exact hardware; MTU values; connection-setup times as seen by tools | Our background modes, restoration, session protocol |
| **A0-obs** What Android sees when an iOS app advertises in the background | nRF Connect (Android) scanner + **Apple Bluetooth logging profile + PacketLogger** on the iPhone | Visibility of Apple manufacturer/overflow frames | Requires *some* iOS app advertising in the background; whether any tool does is **to verify**. Otherwise this moves to the probe |
| HCI-level traces of any test | iOS: Bluetooth logging profile + PacketLogger [S59]; Android: Developer options → HCI snoop log + bug report; Wireshark (`packet-btle`, `packet-btatt`, `packet-bthci_*`) [S54] | Exact timing of connection events, ATT traffic | Over-the-air view of third parties (needs a sniffer) |
| **C0** Generic scan/advertise cost on Android | nRF Connect continuous scan/advertise + `dumpsys batterystats` / Battery Historian | Order of magnitude of raw radio cost on that phone | Cost of VYRO LINK duty cycles; iOS background cost |

### 5.2 Requires a temporary test application ("VL Probe"), specified here, **not built**

The probe is a **throwaway native harness** (one Swift app, one Kotlin app; no Flutter, no production code reuse). It is built only in roadmap phase P1 after explicit authorisation. Minimum requirements:

| # | Requirement | Needed for |
|---|---|---|
| P-1 | Dual-role BLE with one fixed test service UUID; iOS `UIBackgroundModes` = `bluetooth-central`, `bluetooth-peripheral`; restore identifiers on both managers; filtered scans | A |
| P-2 | iOS wake-on-proximity pending connects (pattern from [S27]); Android FGS `connectedDevice` toggle; Android `PendingIntent` scan mode | A, C |
| P-3 | Link session implementing the Doc 07 L1 sequence: HELLO, SUMMARY, REQUEST, ENVELOPE, CUSTODY, VACCINE, BYE (Noise NN if a vetted library is approved for the probe; otherwise a clearly labelled plaintext-link variant for transport-only runs) | A, B |
| P-4 | Doc 07 L2 envelope with the 62-byte header; AEAD wrap with **pre-provisioned test pairwise keys** (no QR pairing in P1); HMAC tags. AEAD: **ChaCha20-Poly1305 (RFC 8439), random 96-bit nonce**, per ADR-014. Platform-native (CryptoKit `ChaChaPoly`) or the Rust core once Phases 3–4 exist | B |
| P-5 | Class R store with the platform protection classes from ADR-010; survives kill/restart | B |
| P-6 | Routing: direct + binary spray-and-wait, expiry, hop limit, dedup, vaccines, receipt envelopes, one sender retry; all parameters loaded from a config file | B |
| P-7 | **Structured event log** (JSON lines, monotonic + wall clock): app state transitions, wakes, restoration callbacks, scan results (filtered), connect/disconnect with error codes, session phase timestamps, envelope custody/delivery events with IDs, battery level samples | A, B, C |
| P-8 | **Ground-truth oracle export** over USB after runs; no network use at all | A, B, C |
| P-9 | Fault-injection build flags: drop-all, tamper-byte, replay, forged-receipt | B (security) |
| P-10 | No analytics, no internet permission/entitlement (Android: no `INTERNET` permission) | All |

**Out of probe scope:** Olm/forward secrecy (P4), QR pairing (E16), Flutter, UI beyond a debug screen.

---

## 6. Experiment A — iPhone ↔ Android BLE across iPhone app states

| Field | Specification |
|---|---|
| **Objective** | Characterise discovery, connection and full-session completion between an iPhone and an Android device for each iPhone app state, in both initiator directions, and confirm the expected negatives |
| **Hypotheses** | **HA1 (=H1):** a backgrounded and a background+locked iPhone (not force-quit) completes iPhone-initiated sessions with an Android device running the FGS. **HA2 (=H3):** a full session (P-3 sequence + one 2 KiB envelope) completes within one background wake (~10 s documented [S09]). **HA3 (expected negative):** an Android device cannot initiate a *new* connection to a backgrounded iPhone it has not met before, using documented means. **HA4:** after the system terminates the app (not force-quit), state restoration returns it to service when a pending event occurs. **HA5 (expected negative):** after force-quit, the iPhone takes part in no session until the user reopens the app |
| **Hardware** | Minimum: 1 iPhone (the present iPhone17,4, iOS 27.0) + 1 Android phone on Android 16 or 17 with FGS. Recommended: + a second iPhone (different generation, iOS 26.x if available) + a second Android OEM with an aggressive battery manager. Optional: nRF52840 + nRF Sniffer |
| **OS versions** | Record exact build numbers per run; target the current iOS 27.0.x and Android 17 (API 37) + 15/16 |
| **Prerequisites** | §4 gaps closed with user authorisation (Android tooling); probe built (§5.2); A0 tool baseline passed; devices charged > 80 %; Wi-Fi/cellular off (airplane mode with Bluetooth re-enabled); clocks synced before going offline (offset recorded) |
| **Independent variables** | iPhone state ∈ {FG, BG-unlocked, BG-locked, system-terminated (restored), force-quit, rebooted-before-first-unlock, rebooted-after-first-unlock}; Android state ∈ {FG, FGS screen-on, FGS screen-off, no-FGS + PendingIntent scan}; initiator ∈ {iPhone, Android}; distance ∈ {1 m, 5 m} line of sight; iPhone Low Power Mode on/off (secondary) |
| **Controlled variables** | Same room and positions, logged 2.4 GHz conditions (Wi-Fi off on the test devices; note nearby APs), identical probe build and config, battery > 50 % |

### 6.1 Procedure

1. **A0 (tools only).** Foreground GATT interop both directions; record MTU, connect time and failures. 20 trials per direction.
2. **Arrival protocol (all A1–A6 cells).**
   - Keep one device in an RF-shielded pouch or box ("out of range").
   - At t₀ remove it; place it at the test distance.
   - Observe for a fixed **observation window W_obs = 10 min** per trial (measurement window, not a target), then return it to the shield.
   - **Inter-trial gap ≥ 2 min**, so connection state and caches settle.
3. **A1:** iPhone FG ↔ Android FGS: both initiators.
4. **A2:** iPhone BG-unlocked (another app in front, probe backgrounded via the home gesture):
   - iPhone-initiated (filtered background scan finds the Android advert).
   - Android-initiated variants, diagnostic only:
     - **A2-R:** reconnect to the iPhone address recorded during a previous FG session, at delays {1, 5, 15, 30 min};
     - **A2-P:** probe of Apple adverts.
5. **A3:** iPhone BG-locked (locked ≥ 1 min before t₀) × Android FGS screen-off.
6. **A4:**
   - System-terminated iPhone app: induce termination by memory pressure (open large apps), or by the developer "Fast App Termination" setting if it exists on the device [S66, C].
   - Confirm termination from the log gap and a restoration callback.
   - Run the arrival protocol; record the restoration callback, time to first session, and whether the iOS pending connect (P-2) completed after the Android address changed.
7. **A5:** Force-quit (app switcher swipe) → arrival protocol → expect no session; then open the app and record time-to-recovery.
8. **A6:** Reboot without unlocking → arrival protocol (expect no participation); unlock → arrival protocol.
9. **A-L (long run, from E2):**
   - iPhone BG-locked for 8 h.
   - The Android FGS peer is brought in and out on a schedule (every 15/30/60 min, randomised order).
   - Count wakes, completed sessions and envelopes moved.
   - Run 3 times.
10. **A-ext (if two iPhones):** BG-locked iPhone ↔ BG-locked iPhone (overflow discovery).

| **Measurements** | Per trial: t_discovery, t_connect, t_noise, t_hello, t_summary, t_envelope_done (s from t₀); session_complete (bool); session_duration (connect→done); wake count and the duration of each background wake (iOS); restoration callbacks; error codes; MTU; RSSI; Android FGS alive (bool) |
|---|---|
| **Repetitions** | 20 per cell per device pair (gives 95 % Wilson CIs of about ±0.2 at mid proportions, §10.3); A-L ×3; A0 20 per direction |
| **Success criteria** | Decision rules in §11 (GA). Per hypothesis: HA1 supported if the success proportion's 95 % CI lower bound ≥ 0.5 in A2 and A3 for every tested pair (a provisional, pre-registered decision rule, **not a product target**). HA2 supported if the 90th-percentile session_duration in BG cells ≤ the measured median wake duration on the same device. Measure the wake duration rather than assume 10 s. HA3/HA5 confirmed if 0 sessions occur in the respective cells. HA4 supported if a restoration callback is followed by a completed session in ≥ 1 trial and the proportion is reported |
| **Failure criteria** | HA1: CI upper bound < 0.5 in A3 on all pairs. HA2: sessions systematically exceed wakes. Unexpected positive HA3 (Android-initiated new connections succeed) is *not* a failure: re-examine the mechanism and document it. A0 failing means a hardware/OS problem: stop and fix the setup before any A-cell |
| **Limitations / confounders** | iOS scheduling heuristics are opaque and version-specific; results on iOS 27.0 do not generalise to 27.x; shielded-pouch quality; human handling; thermal state; the archived 2013 wake-budget documentation; only one iPhone is available today |

---

## 7. Experiment B — Multi-hop store-and-forward without internet

| Field | Specification |
|---|---|
| **Objective** | Demonstrate end-to-end delivery A → B → C (and A → B / A → D → C) with no internet. Distinguish relay custody from delivery. Verify duplicate suppression, encrypted payloads, recipient authentication, persistence across restarts, expiry, and relay-failure recovery |
| **Hypotheses** | **HB1 (=H4):** messages reach C through relays and an authenticated receipt reaches A, in live chains and with time-separated encounters. **HB2:** C displays each message exactly once regardless of the number of copies/paths. **HB3:** carried envelopes and the outbox survive app kills and reboots (Class R). **HB4:** relays never hold or transmit plaintext; tampered, replayed, mis-addressed and forged items are rejected. **HB5:** expired envelopes are purged everywhere and the sender ends in EXPIRED or UNKNOWN, never DELIVERED |
| **Hardware** | Minimum 3 devices including ≥ 1 iPhone and ≥ 1 Android; recommended 4 (2 + 2) so each platform takes each role. Roles rotate across runs: {A, B, C} ∈ permutations covering iPhone-as-relay and Android-as-relay |
| **Prerequisites** | Experiment A cells A1–A3 run first (B depends on the link mechanics); probe with P-3–P-9; test pairwise keys provisioned A↔C (and A↔D for the wrong-recipient test) |
| **No-internet assurance** | Airplane mode on all devices with Bluetooth re-enabled; Wi-Fi off; no SIM data; the probe has no network permission (P-10); record OS reachability status at start and end |
| **Range separation** | Before each run, verify **A and C cannot discover each other**: place them at their run positions, run mutual scans for 10 min (measurement window), require 0 discoveries. Otherwise increase separation or use separate rooms/shielding. Record the positions |

### 7.1 Scenarios

| ID | Scenario | Procedure | Key variable levels |
|---|---|---|---|
| **B1** | Live chain A–B–C | B sits between A and C (in range of both). A sends message m (a 2 KiB bucket with a unique plaintext canary string). Observe until C displays m and A shows DELIVERED | B platform ∈ {iOS, Android}; B state ∈ {FG, BG-locked/FGS} |
| **B2** | Time-separated encounters | (1) A and B together, A sends, until A shows ACCEPTED_BY_RELAY. (2) Shield A (out of range). (3) Carry B for Δ. (4) B meets C; C receives. (5) For the receipt path: B returns to A (or a fresh relay D carries it). Record each transition | Δ ∈ {5 min, 1 h, 4 h}; B state during the carry ∈ {FG, BG-locked}; B app killed by the system during the carry ∈ {no, yes}; B rebooted during the carry ∈ {no, yes} (unlock before the encounter, and once *without* unlocking to confirm non-participation) |
| **B3** | Multiple copies / duplicate paths | Budget L = 4: A meets B (B gets 2), then A meets D (D gets 1, A keeps 1). B and D each later meet C. Then B meets D | Order of arrival at C |
| **B4** | Expiry | Test config with a short expiry (e.g. 10 min; a **test value**, not the product default of 72 h) | Δ before and after expiry |
| **B5** | Security checks | (i) After B1/B2, export B's Class R store and HCI/air captures; search for the canary → expect absent. (ii) Tamper-byte build on B → C rejects, no receipt. (iii) Replay: B re-injects a delivered envelope → C shows no duplicate, logs DUPLICATE. (iv) Wrong recipient: D holds A↔D keys only; an envelope for C passes through D → D cannot unwrap, sends no receipt, keeps it as an opaque carried envelope. (v) Forged receipt: B emits a receipt wrapped with a wrong key → A stays HANDED_OFF | Each fault, 10 repetitions |
| **B6** | Relay failure and recovery | Budget L = 2: A hands B a copy and keeps 1. B runs the drop-all build (black hole). A's retry rule (Doc 05 §5 step 5) fires; A meets D, D carries to C | Retry timing |

### 7.2 Distinguishing relay transfer from delivery (operational definitions)

| Event logged | Logged by | Counts as |
|---|---|---|
| `CUSTODY=ACCEPTED` for envelope_id | Sender/relay receiving the custody frame | **Transfer to a relay** (ACCEPTED_BY_RELAY). Not delivery |
| Tag match + successful unwrap + message_id first seen | Recipient C | **Delivered at C** (ground truth) |
| `CUSTODY=DELIVERED_LOCAL` | The relay that handed the envelope to C | Relay's view of delivery; **unverified by A** |
| Verified `RECEIPT_DELIVERED` (wrap-authenticated with K_wrap C→A) | Sender A | **DELIVERED in A's UI** |
| Expiry without a receipt after ACCEPTED_BY_RELAY | A | **DELIVERY_UNKNOWN** (it may have been delivered) |

**Oracle rule:** an experiment counts a delivery only when C's log shows the unwrap and display of m **and** A's log shows the verified receipt for the same message_id. Any UI state of DELIVERED without both entries is a **critical failure** (misleading indicator, R-19).

| **Measurements** | Per message: timestamps of QUEUED, each CUSTODY, delivered-at-C, receipt-at-A, EXPIRED/UNKNOWN; copies per device over time; dedup counts; vaccine propagation; store sizes; restart/restoration events; canary search results; rejection reason codes |
|---|---|
| **Repetitions** | B1: 10 per role permutation × B state. B2: 5 per (Δ × carry state × kill/reboot) cell, prioritising Δ = 5 min and 1 h. B3/B4/B6: 10 each. B5: 10 per fault |
| **Success criteria** | The logic is deterministic once encounters happen, so **every repetition whose encounter sequence physically occurred must succeed** (HB1–HB5), including zero false DELIVERED states. Encounters that did not occur (Experiment A territory) are classified "no encounter", not a routing failure |
| **Failure criteria** | Any lost carried envelope after a kill/reboot (after first unlock); any duplicate display; any plaintext canary found outside A/C; any accepted tampered/forged/replayed item; any DELIVERED without a verified receipt |
| **Limitations** | Pre-provisioned keys bypass pairing; no Olm, so forward secrecy is not tested (P4); small N relative to real crowds (routing parameters need simulation, E9); scripted encounters do not represent human mobility |

---

## 8. Experiment C — Battery consumption

| Field | Specification |
|---|---|
| **Objective** | Measure energy cost for discovery (scan), advertising, connection maintenance, and message forwarding on each platform and app state, producing a cost model for gate GC |
| **Hypothesis** | **HC1 (=H5):** the cost of Android Relay mode and of iOS background participation, at the default duty cycle, is acceptable to the volunteer population. "Acceptable" is defined by the elicitation in §8.2, not by an invented number |
| **Hardware** | Same models as A/B; **paired-control design**: for each test phone, a same-model control phone (probe absent) runs the identical preparation concurrently where available. Otherwise sequential control runs on the same phone |
| **Conditions** | **C0** idle baseline (probe installed, not running). **C1** filtered scanning only. **C2** advertising only. **C3** scan + advertise, no peers. **C4** maintained connections to {1, 3} idle peers with periodic PING. **C5** forwarding at {10, 100, 1000} envelopes/hour of 2 KiB (independent-variable levels, **not targets**). iOS: BG-locked (wake-driven, peers scheduled as in A-L) and FG screen-on (reference only). Android: FGS screen-off; Battery Saver on/off |
| **Method** | Restart the device; standard preparation (airplane mode + Bluetooth on, fixed brightness, same installed apps, notifications silenced); charge to 100 %, unplug, settle 10 min; run 4 h screen-off (8 h for C3/C5 at the lowest level); record battery % (1 % resolution) at start/end and every 15 min from the probe log. **Android:** `dumpsys batterystats` + Battery Historian per run; on-device power-rail data if the model exposes it (e.g. via Perfetto on supported Pixels; **availability to verify**). **iOS:** probe battery samples (`UIDevice` battery level), Settings → Battery per-app attribution (coarse), and MetricKit daily payloads where available (field coverage **to verify**); no per-app Bluetooth power API is known (D) |
| **Measurements** | Drain rate (%/h) per condition; Δ vs paired control (%/h); slope of drain vs forwarding rate (≈ energy per relayed envelope, in % battery); wake counts (iOS); FGS survival (Android); device temperature at start/end |
| **Repetitions** | ≥ 3 per condition per device (Doc 08), preferably 5; randomise condition order across days |
| **Success criteria** | Cost model complete: every condition has a mean Δ-drain with a 95 % CI on every device. Gate GC per §11 |
| **Failure criteria** | Uninterpretable data: CI of Δ spans more than the measured effect in C3/C5 after 5 runs → increase duration or use power-rail instrumentation |
| **Limitations** | Percent-resolution gauges; battery ageing (record maximum capacity from settings); temperature; OS background activity; iOS gives no direct per-app radio energy; lab duty cycles ≠ real life |

### 8.1 What is testable now with tools only

**C0-tool** (Android, nRF Connect continuous scan vs idle) gives a raw scanner-cost order of magnitude on one device. It **does not** represent VYRO LINK's duty cycle and must not be used for the gate.

### 8.2 Establishing the acceptability threshold (no invented targets)

1. Run C first and produce the cost table.
2. Survey pilot volunteers (≥ 20; a convenience sample, a limitation). Show the measured costs expressed as **extra % battery per day** under typical usage scenarios. Ask the maximum they would accept for (a) always-on relay and (b) relay-while-charging.
3. The threshold is the **median accepted value**, reported with its distribution.
4. Gate GC compares the measured Δ-drain CI against that threshold (§11).

---

## 9. Hardware and software prerequisites

| Item | Present? (§4) | Needed for | Action (requires explicit user authorisation; **not done in this phase**) |
|---|---|---|---|
| Mac (Apple silicon, Xcode 27) | ✔ | iOS probe, PacketLogger, analysis | — |
| iPhone #1 (iPhone17,4, iOS 27.0) | ✔ paired, offline | A, B, C | Connect, unlock, enable Developer Mode if not already (not checked) |
| iPhone #2 | ✘ | A-ext, B role rotation | Obtain |
| Android phone #1 (Android 16/17) | ✘ | A, B, C | **Obtain (blocking)** |
| Android phone #2 (other OEM) | ✘ | A (OEM variance), B (4-device) | Obtain (recommended) |
| Android Studio + platform-tools (adb) | ✘ | Android probe, logs, batterystats | Install later (bundled JBR covers Java) |
| Additional Tools for Xcode (PacketLogger) + iOS Bluetooth logging profile | Unknown | HCI traces | Download later |
| Wireshark | ✘ | Trace analysis | Install later |
| nRF Connect for Mobile (both phones) | Unknown | A0, C0-tool | Install later on phones |
| nRF52840 dongle + nRF Sniffer | ✘ | Air captures (B5-i, privacy) | Optional |
| RF-shielded pouch/box (×2) | Unknown | Arrival protocol | Obtain |
| Apple developer account able to run background-mode apps on device | Not checked | iOS probe | Verify membership type (free provisioning limits **to verify**) |

---

## 10. Measurement tables and results (all **NOT TESTED**)

### 10.1 Experiment A results

| Cell | iPhone state | Android state | Initiator | n | Successes | 95 % CI | Median t_session (s) | P90 session duration (s) | Median wake duration (s) | Status |
|---|---|---|---|---|---|---|---|---|---|---|
| A0 | FG (tool) | FG (tool) | iPhone | 20 | — | — | — | — | — | **NOT TESTED** |
| A0 | FG (tool) | FG (tool) | Android | 20 | — | — | — | — | — | **NOT TESTED** |
| A1 | FG | FGS on | iPhone | 20 | — | — | — | — | n/a | **NOT TESTED** |
| A1 | FG | FGS on | Android | 20 | — | — | — | — | n/a | **NOT TESTED** |
| A2 | BG-unlocked | FGS on | iPhone | 20 | — | — | — | — | — | **NOT TESTED** |
| A2-R | BG-unlocked | FGS on | Android (known address) | 20 | — | — | — | — | — | **NOT TESTED** |
| A2-P | BG-unlocked | FGS on | Android (probe Apple adverts, diagnostic) | 20 | — | — | — | — | — | **NOT TESTED** |
| A3 | BG-locked | FGS screen-off | iPhone | 20 | — | — | — | — | — | **NOT TESTED** |
| A3 | BG-locked | no FGS, PendingIntent | iPhone | 20 | — | — | — | — | — | **NOT TESTED** |
| A4 | System-terminated → restored | FGS screen-off | iPhone | 20 | — | — | — | — | — | **NOT TESTED** |
| A5 | Force-quit | FGS on | either | 20 | — (expected 0) | — | — | — | — | **NOT TESTED** |
| A6 | Rebooted, before first unlock | FGS on | either | 10 | — (expected 0) | — | — | — | — | **NOT TESTED** |
| A6 | Rebooted, after first unlock | FGS on | iPhone | 10 | — | — | — | — | — | **NOT TESTED** |
| A-L | BG-locked 8 h | FGS scheduled | iPhone | 3 runs | sessions/encounters — | — | — | — | — | **NOT TESTED** |
| A-ext | BG-locked | (iPhone BG-locked) | either | 20 | — | — | — | — | — | **NOT TESTED** |

### 10.2 Experiment B results

| Scenario | Variant | n | Encounters occurred | Delivered at C (oracle) | Verified receipt at A | False DELIVERED | Duplicates shown | Median delivery latency | Status |
|---|---|---|---|---|---|---|---|---|---|
| B1 | B = iOS FG | 10 | — | — | — | — | — | — | **NOT TESTED** |
| B1 | B = iOS BG-locked | 10 | — | — | — | — | — | — | **NOT TESTED** |
| B1 | B = Android FGS | 10 | — | — | — | — | — | — | **NOT TESTED** |
| B2 | Δ = 5 min, carry BG-locked | 5 | — | — | — | — | — | — | **NOT TESTED** |
| B2 | Δ = 1 h, carry BG-locked, system kill | 5 | — | — | — | — | — | — | **NOT TESTED** |
| B2 | Δ = 1 h, reboot + unlock | 5 | — | — | — | — | — | — | **NOT TESTED** |
| B2 | Δ = 4 h | 5 | — | — | — | — | — | — | **NOT TESTED** |
| B3 | Two paths | 10 | — | — | — | — | — | — | **NOT TESTED** |
| B4 | Expiry | 10 | — | — | — | — | — | — | **NOT TESTED** |
| B5-i | Canary search | 10 | — | n/a | n/a | — | — | — | **NOT TESTED** |
| B5-ii | Tamper | 10 | — | rejected? — | — | — | — | — | **NOT TESTED** |
| B5-iii | Replay | 10 | — | — | — | — | — | — | **NOT TESTED** |
| B5-iv | Wrong recipient | 10 | — | — | — | — | — | — | **NOT TESTED** |
| B5-v | Forged receipt | 10 | — | — | — | — | — | — | **NOT TESTED** |
| B6 | Black-hole relay + retry | 10 | — | — | — | — | — | — | **NOT TESTED** |

### 10.3 Experiment C results

| Platform / device | Condition | Runs | Drain %/h (mean, 95 % CI) | Δ vs control %/h (95 % CI) | Notes | Status |
|---|---|---|---|---|---|---|
| iPhone | C0–C5 (each) | ≥ 3 | — | — | — | **NOT TESTED** |
| Android #1 | C0–C5 (each), Battery Saver off/on | ≥ 3 | — | — | — | **NOT TESTED** |
| Android #2 | C0–C5 (each) | ≥ 3 | — | — | — | **NOT TESTED** |
| Acceptability survey | Median accepted extra %/day | — | — | — | — | **NOT TESTED** |

**Reference: 95 % Wilson intervals** (arithmetic, for interpreting proportions):

| Result | n = 10 | n = 20 | n = 30 |
|---|---|---|---|
| all succeed | [0.72, 1.00] | [0.84, 1.00] | [0.89, 1.00] |
| 90 % / 18 of 20 | [0.60, 0.98] (9/10) | [0.70, 0.97] | [0.79, 0.98] (28/30) |
| 80 % | [0.49, 0.94] | [0.58, 0.92] | [0.63, 0.90] |
| 50 % | [0.24, 0.76] | [0.30, 0.70] | [0.33, 0.67] |
| none | [0.00, 0.28] | [0.00, 0.16] | [0.00, 0.11] |

With n = 20, the rule "CI lower bound ≥ 0.5" is met at **≥ 15/20 successes**.

---

## 11. Decision gates and GO / CONDITIONAL GO / NO-GO criteria

**Priority order** (as instructed):
1. Reliable iOS ↔ Android communication (GA).
2. Actual multi-hop delivery (GB).
3. End-to-end security (GS).
4. Acceptable battery (GC).

**Pre-registration:** the product owner confirms or adjusts these rules **before** any data is collected. They are decision rules on measured distributions, not product performance targets.

| Gate | **Authorise next phase** | **Requires architecture change** | **Justifies abandoning the current approach** |
|---|---|---|---|
| **GA: iOS ↔ Android** (A) | A0 passes; A1 succeeds (CI lower bound ≥ 0.5) in both directions; **A2 and A3 iPhone-initiated** meet CI lower bound ≥ 0.5 on every tested pair; HA2 holds (P90 session ≤ median wake on the same device); A5/A6-before-unlock show 0 sessions as documented | (a) A3 fails but A2 passes → iPhones act as relays only while unlocked: change ADR-011 and the UX. (b) HA2 fails → resumable multi-wake sessions, or **iPhone as endpoint only** (§12). (c) Only A1 passes → **iPhone endpoint + Android backbone**, or Architecture D. (d) A4 never restores → drop reliance on restoration; foreground re-entry only | **No BG-iPhone ↔ Android session in any state on any pair** (A2 and A3 at 0/20 everywhere) **and** Architecture D rejected by the owner → **NO-GO** for the commodity-phone mixed network. If **A1** also fails while A0 passes → treat as an implementation defect and re-test before any conclusion |
| **GB: multi-hop** (B) | Every repetition with physically occurred encounters succeeds in B1–B4 and B6; 0 false DELIVERED; 0 duplicates; persistence holds after kill and after reboot+unlock | Loss on restart → storage-class/persistence redesign (ADR-010). Receipts not returning → receipt routing changes (e.g. receipts sprayed with a larger budget or carried by the same relay). Dedup failures → fix the ID scheme | Store-and-forward cannot survive the iOS process lifecycle even with a correct storage class → exclude iPhones as carriers (architecture change). **Abandon only in combination with a GA abandon outcome** |
| **GS: E2E security in this phase** (B5) | Canary never found outside A/C; tamper, replay, wrong-recipient and forged-receipt rejected in 100 % of runs | Any failure is a design or implementation defect: fix and re-run before P2. Full assurance remains deferred to P4 (Olm/FS) and P9 (independent review) | A failure traced to the *design* (not the code) that the Doc 06 primitives cannot fix → redesign the security architecture before continuing |
| **GC: battery** (C) | Δ-drain CI upper bound for Android Relay mode (C3/C5 at the default duty cycle) ≤ the median accepted threshold (§8.2); iOS background participation cost likewise | Relay mode exceeds the threshold → relay only while charging or in scheduled windows, a lower duty cycle, then re-run C. iOS cost exceeds it → reduce wake work or make iPhones endpoint-only | Even the minimum discovery duty cycle exceeds the threshold on both platforms **and** charging-only relaying is shown by simulation (E9) to give insufficient density → **NO-GO** or Architecture D |

### Overall verdict rules (authorising P2 → P3, i.e. development)

| Verdict | Condition |
|---|---|
| **GO** | GA, GB, GS and GC all "Authorise" |
| **CONDITIONAL GO** | GB and GS "Authorise", **and** GA "Authorise" or GA "Architecture change" (b)/(c)/(d) with the redesign accepted by the product owner, **and** GC "Authorise" or "Architecture change" with an accepted mitigation |
| **NO-GO (current architecture)** | Any "Abandon" outcome, or GB/GS failures that remain after root-cause analysis and implicate OS lifecycle or design rather than code |

**Current verdict:** research **CONDITIONAL GO** is unchanged. **Development is NOT authorised.** No gate has data.

---

## 12. Architecture changes if iOS background restrictions prevent reliable forwarding

These are **pre-analysed options**, not decisions. Each is triggered by a GA/GB outcome.

| Trigger | Change | Effect on existing docs | Cost / risk |
|---|---|---|---|
| HA2 fails (sessions do not fit wakes) | **Resumable sessions:** per-envelope custody is already atomic (Doc 07 CUSTODY). Add explicit session resumption so a transfer can span wakes; send DELIVER traffic first (already specified) | Doc 07 L1: add a resume token in HELLO (minor) | Low; protocol change |
| A3 fails, A2 passes | **iPhone relays only while unlocked/foreground**; "Relay while using" mode on iOS; Android remains the backbone | ADR-011, Doc 09 UX copy | Lower density from iPhones |
| A2 and A3 fail, A1 passes | **iPhone = endpoint only.** iPhones send and receive via Android relays when foregrounded; routing prefers relays that declare an always-on class | Doc 07 HELLO: add `relay_class` (always-on / best-effort / endpoint); Doc 05 §5: spray prefers always-on relays; Doc 09: messaging between two iPhone-only users becomes slow | Large UX impact; Android density becomes critical |
| A4 never restores | No reliance on state restoration; prompt users to reopen the app (local notification) | Doc 04 §3, ADR-011 | Medium |
| GA abandon / GC abandon | **Architecture D:** always-on BLE (or LoRa) relay nodes; phones only need short sessions | Doc 05 §1 (D); new hardware workstream | Hardware cost; changes the product brief (commodity phones only) |
| Contacts frequently co-located | Wi-Fi Aware between **paired contacts** (iOS allows paired-device connections in the background [S12]) as an auxiliary channel | Doc 04 §6 (deferred) | Pairing UI; iPhone↔Android interop unproven [S16] |

---

## 13. Reproducibility requirements

1. **Pre-registration:** commit this plan, the decision rules and probe config files before running; changes after data collection must be logged with a reason.
2. **Device inventory sheet per run:** model identifier, OS build, battery maximum capacity, Low Power/Battery Saver state, installed app list hash, Developer Mode, distance and positions (sketch/photo).
3. **Build provenance:** probe source commit hash, build configuration, config-file hash; identical builds across devices of the same platform.
4. **Clock handling:** NTP sync before going offline; record offsets; analysis uses per-device monotonic time plus offset correction.
5. **Raw data retention:** probe logs (JSON lines), HCI captures (`.pklg`, `btsnoop`), sniffer `.pcapng`, batterystats dumps. Archive with SHA-256 checksums; never edit raw files.
6. **Analysis scripts versioned**, deterministic, run on raw files only; results tables generated, not hand-typed.
7. **Environment log:** time, location, visible Wi-Fi APs (from the Mac, passively), other BLE devices count (nRF Connect scan snapshot), temperature.
8. **Randomisation:** condition order randomised per day; device role permutation in B.
9. **Negative controls:** A5/A6 (expected zero); C0 / paired control phone; B5 fault builds.
10. **Version re-runs:** repeat A1–A3 and C3 on each new major iOS/Android release (TN3115 changed in iOS 26 [S11]).

---

## 14. Failure analysis guide

| Observation | Likely causes | Distinguish by | Next action |
|---|---|---|---|
| A0 fails | Hardware/OS issue, tool misconfiguration | Retry with a second device/tool | Fix the setup; do not proceed |
| A1 fails, A0 passes | Probe bug (GATT server, UUID, MTU handling) | HCI traces vs A0 traces | Fix the probe |
| A2 passes, A3 fails | iOS lock-state throttling; Class R not accessible; Android screen-off scan suppression (unfiltered scan [S67]) | Probe logs: wake events present? scan results present? Android scan filters active? | If iOS wakes are absent: platform limit → §12 row 2. If data-protection errors: storage-class fix |
| Sessions start but don't finish in BG | Wake budget exceeded; slow MTU/connection interval | Session phase timestamps vs wake duration | Session ordering / resumable sessions (§12 row 1) |
| A4 no restoration | No pending request at termination; restore identifier missing | Restoration logs; P-2 pending connects armed? | Fix the probe; re-test; else §12 row 4 |
| Pending connects never complete after the Android address changes | Address rotation; non-resolvable addresses (no bonding) | Compare the Android address in logs before/after | Rely on scan-driven discovery, not pending connects |
| B delivers but no receipt | Receipt routing (no carrier back to A) | Per-envelope custody logs for the receipt envelope | Adjust the receipt spray/carry policy |
| B loses envelopes on restart | Storage class/flush timing | Store dumps before/after | Persistence fix (ADR-010) |
| Duplicate display | message_id dedup missing on one path | Recipient logs | Fix dedup |
| Battery Δ indistinguishable from noise | Run too short; gauge resolution | CI width | Longer runs or power-rail data |
| Android FGS dies screen-off | OEM battery manager | Logs + OEM settings | Document per-OEM onboarding; R-08 |

---

## 15. Risk assessment (this phase)

Existing risks R-01…R-24 in Doc 10 remain valid. This phase adds:

| ID | Risk | Type | Prob. (conf.) | Impact | Evidence | Mitigation |
|---|---|---|---|---|---|---|
| R-25 | **No Android hardware or tooling available**: validation cannot start | E (operational) | H (high) | 4 | §4 | Obtain devices; authorise tooling install |
| R-26 | Android 17 deprecates legacy `connectGatt` overloads; probe/production code targeting API 37 must use `connectGatt(BluetoothGattConnectionSettings, …)` | E | M (high) | 2 | [S68] | Use the new API on API 37+, legacy below |
| R-27 | iOS 27 Core Bluetooth regressions or undocumented behaviour changes (one beta disconnect report) | Q | L (low) | 3 | [S74] | Re-run A1–A3 on each 27.x update; file Feedback reports |
| R-28 | Single available iPhone limits generalisation (one model, one OS build) | E | H (high) | 3 | §4 | Second iPhone (different generation) before the GA decision |
| R-29 | Probe deviations from spec (e.g. temporary AEAD choice, no Olm) bias the security conclusions | E | M (med) | 2 | §5.2 | Record deviations; security gate limited to the wrap/tag layer; full crypto at P4 |

**Largest remaining technical risks (ranked):**
1. R-01/R-02/HA1: backgrounded iPhone participation.
2. HA2: session fits the wake budget.
3. R-04: density (cannot be resolved by A–C; needs E9/E12).
4. R-07: Play policy for Relay mode.
5. R-06/HC1: battery acceptability.

---

## 16. What must happen before development begins

1. **Product owner review** of this plan and **pre-registration** of the §11 decision rules (adjust them now, not after data).
2. **Obtain hardware:** ≥ 1 Android phone (Android 16/17) — **blocking**. Recommended: a second Android OEM, a second iPhone, two RF-shielded enclosures.
3. **Explicit user authorisation** to install research tooling (Android Studio/adb, Wireshark, Additional Tools for Xcode/PacketLogger, the iOS Bluetooth logging profile, nRF Connect on phones). **Not done in this phase.**
4. **Explicit user authorisation** to build the throwaway **VL Probe** harness (§5.2) in roadmap phase P1. It is not the application and must not be reused as production code.
5. Run **A0 → A → B → C** in that order; fill §10 tables from raw data; evaluate gates GA → GB → GS → GC.
6. Update Doc 01 (verdict), Doc 10 (risk status) and the ADRs (P2) from the measured outcomes.
7. Only a **GO** or an accepted **CONDITIONAL GO** under §11 authorises P3 (transport proof of concept). The application itself (P6) still needs P3–P5 and gate G2.

---

## 17. References

All source IDs resolve in Doc 12. **New in this phase** (added to Doc 12 §J):
- **S67** Android `BluetoothLeScanner` reference: unfiltered scans stop on screen off; `PendingIntent` scans.
- **S68** Android `BluetoothDevice` reference: `createL2capChannel` API 29 (LE only); legacy `connectGatt` overloads deprecated in API 37.
- **S69** Apple `CBPeripheralManager.publishL2CAPChannel(withEncryption:)`, iOS 11+.
- **S70** Android "Restrictions on starting a foreground service from the background".
- **S71** Android 16 behaviour changes (all apps).
- **S72** Android 17 behaviour changes (all apps).
- **S73** Apple Platform Security, Bluetooth privacy.
- **S74** Apple Developer Forums 839288 (iOS 27 beta disconnect report).
- **S75** AOSP CDD change requiring 15-min RPA rotation on Android (search-result level).

**Key reused sources:** S09, S10, S10b, S11, S12, S15, S16, S17, S18, S19, S20, S24, S25, S27, S54, S59, S66.
