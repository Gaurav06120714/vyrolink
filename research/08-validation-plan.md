# Document 8 — Feasibility and Experimental Validation Plan

> E1, E2, E5, E7, E8, E13, E14 and E15 are operationalised in more detail by **[Doc 14](14-feasibility-validation-plan.md)** (Experiments A/B/C, gates GA/GB/GS/GC), which is authoritative for them. The other E-series experiments remain defined here.

**None of these experiments has been performed.** They are future procedures. Where the evidence does not yet justify a numeric target, the plan specifies **how to derive one** rather than inventing it.

## 1. Research hypotheses

| ID | Hypothesis | Invalidates project if false? | Experiments |
|---|---|---|---|
| H1 | A backgrounded (not force-quit), screen-locked iPhone discovers and completes a BLE link session with an Android device in Relay mode (FGS) within minutes of coming into range | **Yes**, for mixed networks (→ Architecture D) | E1, E2 |
| H2 | Two backgrounded iPhones discover each other via the overflow area often enough to exchange envelopes during realistic encounters | Partially (degrades iOS-only crowds) | E1, E2 |
| H3 | One link session (connect + Noise NN + HELLO + SUMMARY + ≥1 envelope) completes within an iOS background wake budget (~10 s [S09]) | **Yes** for iPhone relaying | E2, E3 |
| H4 | Store-carry-forward across ≥3 devices with separated encounters delivers and returns an authenticated receipt | **Yes** | E7 |
| H5 | Android Relay mode costs battery acceptable to volunteers (threshold set from E5 baseline + user study) | Partially (lower density) | E5 |
| H6 | Spray-and-wait with bounded L and expiry achieves useful delivery ratios in realistic human contact patterns | Partially (parameter retuning / D) | E9, E12 |
| H7 | A passive observer cannot link a VYRO LINK user across locations beyond what OS address randomisation allows | Privacy goal | E10 |
| H8 | Quotas bound a malicious relay's damage to storage/battery within declared limits | Security goal | E11 |

## 2. Test harness (to be built in roadmap P1)

- **Minimal native test apps** (Swift and Kotlin, no Flutter) implementing L0–L2 with dummy payloads. They log every event with monotonic timestamps to local files (no network).
- **Clock alignment:** sync device clocks over NTP before going offline; record the offset; use a shared start trigger (QR shown by the Mac).
- **Mac:** collects logs over USB after runs; runs analysis scripts. Optional: an nRF52840 dongle with nRF Sniffer + Wireshark for over-the-air capture (Doc 13).
- **Repeatability:** every run records device model, OS build, battery level, Low Power/Battery Saver state, Bluetooth/Wi-Fi state, RF environment notes, and the app build hash.

## 3. Device matrix

| Tier | iOS | Android |
|---|---|---|
| Minimum (first feasibility) | 2 iPhones (one iPhone 12 or later, so Wi-Fi Aware can be tested later), iOS 27.x; ideally one on iOS 26.x | 2 Android phones from different OEMs (e.g. Pixel on Android 17 + Samsung on Android 15/16) |
| Recommended | 4 iPhones across ≥3 model generations | 4–6 Android: Pixel, Samsung, one Xiaomi/OnePlus-class (aggressive battery manager), one budget device |

## 4. Experiments

Common fields: **Repetitions** default ≥ 20 per condition for latency/success-rate metrics. Report the median, IQR and success proportion with a 95 % CI. **Failure criteria** default: inability to complete the procedure at all. Other failure criteria are stated per experiment.

### E1 — Two-device discovery and link establishment (cross-platform, per app state)

| Item | Detail |
|---|---|
| Objective | Measure P(discovery) and time-to-link for every pair type × app-state combination (Doc 4 §2 matrix) |
| Hardware | Minimum tier |
| Setup | Devices 1 m apart, then 5 m, line of sight; one starts out of range (Faraday bag or shielded box) and is introduced at t₀ |
| Independent variables | Pair (iOS-iOS, iOS-Android, Android-Android); state of each side (FG, BG, BG+locked, Android FGS on/off); which side initiates |
| Measurements | t(discovery callback), t(connected), t(Noise done), t(HELLO done); failures and error codes |
| Success criteria | Baselines established for all cells; H1 evaluated with the pre-registered decision rules in Doc 14 §11 (gate GA). Product performance targets (Doc 9 §5) are set *after* these baselines, not before |
| Failure criteria | BG-iPhone ↔ FGS-Android cell fails systematically → gate G1 (Doc 10) |
| Confounders | 2.4 GHz Wi-Fi load; other BLE devices; OS scan scheduling; device temperature |

### E2 — iOS background longevity, wake-on-proximity, restoration

| Item | Detail |
|---|---|
| Objective | Determine whether a locked iPhone keeps relaying over hours, and test bitchat-style pending connects (S27) |
| Setup | iPhone app backgrounded, screen locked; Android FGS peer brought in and out of range on a schedule (every 15/30/60 min) for 8 h; repeat with the system terminating the app (memory pressure via other apps; or the Developer "Fast App Termination" setting suggested in Apple forums [S66, C]) |
| Variables | Schedule; Low Power Mode on/off; pending-connect vs scan-only; Android address rotation observed or not |
| Measurements | Wake count, sessions completed, envelopes moved per wake, time per session vs ~10 s budget, restoration callbacks, whether pending connects complete after the peer's address changes |
| Success | Sessions keep completing across the 8 h window (rate reported, no fixed target until the baseline exists) |
| Failure | Relaying stops within the first hour in the locked state → H1/H3 false |
| Confounders | iOS version-specific heuristics; usage patterns; thermal state |

### E3 — Direct encrypted transfer (GATT vs L2CAP CoC)

| Item | Detail |
|---|---|
| Objective | Throughput and latency for 256 B–2 KiB envelopes, single and batch (1, 10, 100) |
| Variables | Transport (GATT WwR vs L2CAP CoC), negotiated MTU, pair type, distance |
| Measurements | Session time breakdown; goodput; retransmissions; MTU; failures |
| Success | Establish which transport to use per pair; decide the L2CAP upgrade (ADR-001 follow-up) |
| Confounders | Connection interval chosen by the OS (iOS cannot pick PHY [S52]) |

### E4 — Range and environment

Indoor office, crowded corridor, outdoor open space, phone in pocket/bag. Measure discovery success and session success vs distance. **Report distributions; never a single "range" number.**

### E5 — Battery

| Item | Detail |
|---|---|
| Objective | Energy per hour for: idle app (baseline), scan-only, advertise-only, Relay mode (FGS) idle, Relay mode with N sessions/hour; iOS background mode with M wakes/hour |
| Method | Full charge → fixed screen-off protocol 4 h → battery % delta; repeat ≥ 3; Android: `dumpsys batterystats`; iOS: Xcode energy/battery logs where available. Control device without the app |
| Variables | Duty-cycle parameters; Low Power Mode / Battery Saver |
| Success | Produces the cost model the product decision needs. The acceptance threshold is decided with users (Doc 9) |
| Confounders | Battery age; background apps; temperature |

### E6 — Congested radio / many peers

5–10 devices in one room (or devices + BLE beacon noise). Measure discovery time, session success, link drops, and the max stable concurrent links per OS (iOS cap undocumented; bitchat uses 6 [S27]).

### E7 — Three-device (and four-device) store-and-forward with separated encounters + acknowledgement

| Item | Detail |
|---|---|
| Objective | Validate H4 end to end |
| Setup | A and C never meet. A meets B (A→B handoff). B is carried away; after a delay Δ (1 min, 30 min, 2 h), B meets C. Then the receipt returns via B (or D) to A. A four-device variant: A→B→D→C |
| Variables | Δ; app states during the carry (BG/locked); platform mix; app restart on B between encounters |
| Measurements | State transitions (QUEUED → ACCEPTED_BY_RELAY → ACKNOWLEDGED; Doc 18 §7.4) with timestamps; receipt latency; copies at each node |
| Success | Delivery + authenticated receipt in all repetitions where the encounter sequence physically occurred; UI states never claim "Delivered" without a receipt |
| Failure | Envelope lost on restart; receipt not authenticated; wrong state shown |

### E8 — Duplicate suppression and multi-path

C receives the same envelope via two paths and the same message via a re-encrypted retry. **Success:** exactly one message is displayed; the vaccine purges remaining copies.

### E9 — Routing simulation (desktop, using the portable core)

| Item | Detail |
|---|---|
| Objective | Choose L, expiry, retry and quotas; estimate delivery ratio vs density |
| Method | Run the real core in a discrete-event simulator driven by (a) synthetic mobility (random waypoint, community models) and (b) public human contact traces if licensable (candidate: Haggle/Cambridge iMote-style datasets; availability unverified **D**) |
| Measurements | Delivery ratio, latency distribution, copies/message, storage per node, receipts returned |
| Success | Parameter set where delivery ratio and resource use are Pareto-reasonable. Feeds back into Doc 6 §7 |

### E10 — Passive privacy (sniffing)

nRF Sniffer + Wireshark (`packet-btle.c`, `packet-nordic_ble.c` dissectors present in source [S54]). Record adverts across sessions and locations; attempt to link devices using address, timing, and payload sizes. **Success:** no application-layer identifier persists; residual linkability is documented.

### E11 — Malicious relay and spam

A modified test build that (a) drops everything, (b) inflates copy budgets, (c) floods random tags at max rate, (d) replays old envelopes, (e) sends malformed headers. **Success:** honest nodes stay within storage/battery caps; no crash; the recipient rejects replays; delivery degrades gracefully, as measured.

### E12 — Field pilot (after MVP)

A campus or event with 20–50 volunteers for 1–3 days. Measure delivery ratio, latency, battery, and the user understanding of delivery states (survey).

### E13 — Offline persistence and restart recovery

Kill the app (system-style), reboot the device mid-carry, and verify the carried store, outbox, dedup and vaccine sets are intact and restored after first unlock.

### E14 — Termination behaviour

iOS force-quit, Android force-stop: confirm no relaying (expected per S11 / platform behaviour) and that the app explains this on next open.

### E15 — Expiry

Envelopes near expiry: relays refuse, purge on time, the sender shows EXPIRED/UNKNOWN correctly; clock-skew cases (±2 h).

### E16 — Cross-platform QR pairing

Pairing iOS↔Android in varied lighting; measure time-to-pair and errors; verify identical safety numbers; a usability probe of a malicious-QR substitution scenario.

### E17 — Unsupported-transport fallback (post-MVP)

When Wi-Fi Aware/L2CAP is unavailable or fails, sessions fall back to GATT without user-visible failure.

## 5. Acceptance criteria and gates (summary)

| Gate | Requires | If it fails |
|---|---|---|
| **G1 Transport** | E1 + E2: H1 holds (mixed BG-iPhone ↔ FGS-Android links are viable); H3 holds | Re-scope to Architecture D (dedicated relays) or Android-backbone-only; or NO-GO |
| **G2 Delivery** | E7 + E8 + E13 pass; E9 shows a usable parameter region | Retune routing; reconsider hybrid routing |
| **G3 Energy** | E5 cost acceptable to pilot users | Relay mode limited to charging/scheduled windows |
| **G4 Security** | E10, E11 pass; independent review (P9) with no unresolved high findings | Fix and re-review before release |

## 6. Interpreting failures

- **Distinguish OS limits from implementation bugs.** Reproduce failures with a minimal Apple/Google sample or with nRF Connect before blaming the platform.
- **Distinguish discovery failure from session failure.** Discovery depends on scheduling; session failures point to protocol or timeout design.
- Results from one OS version **do not generalise**. Re-run E1/E2 on each major iOS/Android release (TN3115 changed in iOS 26 [S11]).
