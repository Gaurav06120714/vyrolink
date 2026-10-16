# Document 4 — Wireless Technology and Platform Feasibility

**Checked:** 2026-10-14
**Platform versions in scope:**
- **iOS 27.0.1** is current; it was released 2026-09-14, and 27.0.1 on 2026-09-28 [S53a, C].
- **Apple documentation reviewed is labelled iOS 26.** TN3115 rev. 2025-09-15; Wi-Fi Aware since 26.0. No iOS 27-specific Bluetooth change was found; that is unverified, so treat it as an open item.
- **Android 17 (API 37)**, stable since June 2026 [S53b].

> **Bottom line.**
> - **BLE is the only transport that is (a) available on both iOS and Android, (b) usable by third-party apps without per-peer user pairing UI, and (c) workable in the background on both platforms.** It is therefore the **primary transport**, for both discovery and data transfer.
> - Background behaviour is **asymmetric**:
>   - A backgrounded iPhone's advertisement is not decodable by Android [S15].
>   - A backgrounded iPhone can still discover a foreground-service Android, and other iPhones.
>   - A force-quit iPhone is out of the network until the user reopens the app [S11].
> - **Wi-Fi Aware** is now on both platforms (iOS 26+). On iOS it requires a **system pairing UI per device** [S12, S13], and iPhone↔Android interoperability is unproven [S16]. It is a *deferred contact-to-contact accelerator*, not a relay transport.
> - Everything else is single-platform or needs infrastructure or extra hardware.

---

## 1. Technology comparison matrix

Legend: ✅ supported/documented · ⚠️ conditional/limited · ❌ not available · ❓ unknown, must measure

| Dimension | **BLE (GATT / L2CAP CoC)** | BT Classic (RFCOMM) | Wi-Fi Direct | **Wi-Fi Aware (NAN)** | Apple P2P Wi-Fi / Multipeer | Google Nearby Connections | Infra Wi-Fi (shared AP) | Bluetooth Mesh | LoRa (e.g. Meshtastic) |
|---|---|---|---|---|---|---|---|---|---|
| Devices | iOS ✅ Android ✅ | Android ✅; iOS ❌ for apps (MFi only) (D) | Android ✅; iOS ❌ (no API, S14) | iOS 26+ on iPhone 12+ ✅ [S12]; Android 8+ with hardware ✅ [S21] | Apple only [S14] | Android ✅; iOS ⚠️ (immature packaging) [S26r] | both ✅ | embedded nodes; phones as proxy clients [S36] | dedicated radio |
| Discovery | Advertising + filtered scan | Inquiry (slow, visible) | P2P discovery | Publish/subscribe | Bonjour/AWDL | SDK advertise/discover | mDNS | Beacons | LoRa broadcast |
| Connection | GATT; L2CAP CoC (iOS 11+ [S69], Android API 29, LE only [S68]) (A) | RFCOMM socket | Group formation (GO negotiation) | Data path (iOS: **paired devices only**) | MC session / NWConnection | SDK connection + bandwidth upgrade | TCP/UDP | Provisioned network | Node-to-node packets |
| Arbitrary payloads | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ⚠️ small, network-key scoped | ⚠️ ~200 B frames (not re-verified) |
| Multi-hop | App-level only | App-level | App-level | App-level | App-level | App-level | App-level | Native (managed flood) | Native (firmware) |
| **iPhone ↔ Android** | ✅ (same GATT/L2CAP primitives; both roles on both sides) | ❌ | ❌ | ⚠️ standard-based, **reported interop failures** [S16] | ❌ | ⚠️ claimed by Google [S26] | ✅ (if both join same AP) | ❌ for phone relaying | ✅ via hardware |
| Background (app not visible) | iOS ⚠️ with background modes (see §3); Android ⚠️ with FGS | Android ⚠️ FGS | Android ⚠️ | iOS ✅ "foreground and background" for **paired** devices [S12]; Android ❓ | ❓ | ❓ | ⚠️ | n/a | radio always on |
| Terminated app | iOS: relaunch for pending BT events except after force-quit [S11]; Android: `PendingIntent` scan wake [S18] | ❌ | ❌ | ❓ | ❌ | ❓ | ❌ | n/a | n/a |
| Range | Environment-dependent; **must measure** (no single number) | must measure | must measure | must measure | — | — | AP-bound | — | km-scale claims (not verified here) |
| Throughput (published) | GATT WwR 2M PHY: iPhones ≈311–365 kbps; Pixel 3 / S10+ ≈1.2 Mbps [S49, C]; L2CAP CoC on phones: **no published cross-platform benchmark found** | — | high (not measured here) | "High-bandwidth, low-latency" [S12] | high | upgrades to fastest medium [S26] | high | very low | very low |
| Latency | Discovery: smartphone measurements roughly match models but vary more [S50]; connection setup ❓ | slow inquiry | ❓ | ❓ | ❓ | ❓ | low | — | seconds |
| Power | Scanning dominates; parameters drive the trade-off [S50, S51] | ❓ | ❓ | `realtime` mode "can drain battery" [S12] | ❓ | ❓ | low | — | radio-side |
| Connection limits | Not documented on iOS; bitchat caps at 6 central links [S27]; Android batches connection requests (≥10) [S18] | ~7 (D) | group-based | `getAvailableAwareResources()` [S21] | MC ≈8 peers (D) | strategy-dependent | — | — | — |
| Permissions | iOS: `NSBluetoothAlwaysUsageDescription`; Android 12+: SCAN/ADVERTISE/CONNECT [S17] | same | `NEARBY_WIFI_DEVICES` [S22] | iOS entitlement + service plist [S12]; Android `NEARBY_WIFI_DEVICES` [S21] | Local Network | multiple | Android 17 `ACCESS_LOCAL_NETWORK` [S23] | — | — |
| Link encryption | App's job (BLE pairing would prompt the user) | optional | WPA2 group | **Wi-Fi-layer encryption after pairing** [S12] | TLS opt. | "fully encrypted" [S26] | app's job | AES-CCM | AES-CTR |
| Infrastructure | none | none | none | none | none | none | **AP required** | provisioner | nodes |
| Complexity | medium (two native stacks) | n/a | high | medium-high (entitlement, pairing UX) | low (iOS only) | medium (Bazel/GMS) | low | high | hardware |
| **Suitability** | **Primary: discovery + data + relaying** | Reject | Reject (no iOS) | **Deferred: contact-to-contact bulk** | Deferred: iOS↔iOS bulk | Investigate: secondary adapter | Optional: same-AP fast path (later) | Reject | Alternative architecture D |

### Why "the same transport" ≠ interoperability

1. **Advertising formats differ.**
   - iOS apps can only advertise a local name and service UUIDs, and in the background only via the Apple-proprietary overflow area [S10].
   - Android apps can put arbitrary service data and manufacturer data in advertisements.
   - So a protocol that relies on *service data* in adverts works Android→Android but not iOS→Android.
2. **Discovery visibility is state-dependent and one-directional** (§2).
3. **Connection parameters are OS-controlled.** iOS apps cannot pick the PHY, and the iOS minimum connection interval (15 ms) differs from Android's (11.25 ms) [S52, C]. MTU negotiation outcomes differ.
4. **GATT server behaviour differs:** characteristic caching, service-changed handling, and write-without-response flow control.
5. **Address randomisation differs.** Re-identifying a peer across sessions via its address is unreliable, so peer identity must live at the application layer (D).

A **common application-level protocol** is therefore required regardless of transport (ADR-002, Doc 5).

---

## 2. Cross-platform discovery and connection matrix (BLE)

The VYRO LINK design rule: *every device both advertises (peripheral) and scans (central)*; a link forms if **either** side discovers the other and connects.

| Device X state → / Device Y state ↓ | Android, app foreground or FGS | iPhone, app foreground | iPhone, app background (not force-quit) | iPhone, force-quit |
|---|---|---|---|---|
| **Android FG/FGS** | ✅ both directions (A) | ✅ (A) | ✅ **iPhone discovers Android** (bg scan with service filter [S10b]); Android cannot decode the iPhone's background advert [S15] → link forms only if the iPhone initiates | ❌ (until the user opens the app) [S11] |
| **iPhone FG** | ✅ | ✅ | ✅ FG iPhone scanning for the UUID sees the overflow advert [S10] | ❌ |
| **iPhone BG** | (as above) | ✅ | ⚠️ **both background**: overflow-area discovery is documented, but scan interval increases and advert frequency may decrease [S09] → **latency unknown (D, E1)** | ❌ |
| **Android, process dead, no FGS** | ⚠️ `PendingIntent` scan can wake it for short work [S18] if the peer *advertises*. A background iPhone's advert is not matchable by Android (D: filter on overflow data impossible) | same | ❌ effectively | ❌ |

**Interpretation.**
- In a mixed crowd, **Android devices running a relay foreground service are the backbone**.
- Background iPhones act as *opportunistic clients and relays* toward Android devices and toward each other.
- Two backgrounded iPhones may find each other slowly.
- These are documented mechanisms whose **real-world rates are unknown**, so experiment E1 (Doc 8) is the first gate.

---

## 3. iOS feasibility by state (iPhone, app with `bluetooth-central` + `bluetooth-peripheral` background modes)

"Guaranteed" = documented OS behaviour; "best-effort" = documented as allowed but throttled or undefined in rate.

| # | State | Discovery | Forwarding | Persistence | Delivery to this user | Basis |
|---|---|---|---|---|---|---|
| 1 | Foreground | Full scan (duplicates allowed), advertise name + UUID | Full | DB available | Immediate | A [S09, S10] |
| 2 | Backgrounded | Scan **only with service UUIDs**; duplicates coalesced; longer scan interval. Advertise via overflow area (iOS-only discoverable). | Allowed during wake events; ~10 s per wake; throttling/kill if excessive | DB available if file protection class permits | Best-effort | A [S09, S10, S10b] |
| 3 | Screen locked | As 2 | As 2 | **Files with Complete protection become unreadable ~10 s after lock** [S63]. Relay store must use the "until first user authentication" class (Doc 6 §6) | Ciphertext stored; decrypted on unlock | A + design |
| 4 | Suspended / removed from memory by OS | System wakes or relaunches the app for pending BT events (scan match, connect, subscribed notification, advertising) | Resumes on wake | Restored via state restoration | Best-effort | A [S09, S11] |
| 5 | **Force-quit by the user** | **None** — no relaunch | None | Data stays on disk | **None until the user opens the app** | A [S11]. The iOS 26 AccessorySetupKit exception applies to accessories, not phone peers (D) |
| 6 | Device restarted | Relaunch for restoration **only after first unlock** | After first unlock | Class C data available after first unlock | After first unlock | A [S11 note 4] |
| 7 | Bluetooth turned off (Settings) | None; no relaunch on re-enable | None | Kept | None | A [S11 note 2] |
| 7b | Bluetooth toggled from Control Center | Table says "Yes", but note 5 limits iOS 26+ relaunch to AccessorySetupKit apps → **ambiguous; assume no for VYRO LINK (D)** | — | — | — | A/D |
| 8 | Permission revoked | No BLE access | None | Kept | None | A (implied) |
| 9 | Low Power Mode | **Undocumented** effect on third-party BLE background work | ❓ | ❓ | ❓ | D → measure (E5) |
| 10 | Peer out of range | Pending connect requests do not expire; completion wakes the app (pattern used by bitchat) | On reconnection | Kept | On next encounter | A (S09) + B (S27 code) |

**Can an iPhone reliably function as an unattended relay? No.**
- *Guaranteed:* nothing beyond event-driven wakes of ~10 s.
- *Plausible best-effort:* relaying while backgrounded and not force-quit, with throughput and duty cycle set by the OS.
- *Hard stop:* force-quit and Bluetooth power-off.

**Hypothesis to test (E2):** a backgrounded, locked iPhone carried for hours still completes a meaningful number of encounters.

## 4. Android feasibility by state (Android 12–17)

| # | State | Discovery | Forwarding | Persistence | Delivery | Basis |
|---|---|---|---|---|---|---|
| 1 | Foreground | Full | Full | Available | Immediate | A |
| 2 | Background **with FGS (`connectedDevice`)** | Scanning/advertising/GATT continue while the process lives. Use `ScanFilter`, because unfiltered scans stop when the screen is off (A [S67]; re-verified 2026-10-14, Doc 14 §3) and starts are throttled (~5/30 s, C [S25]) | Full | Available | Near-immediate | A [S18, S19] + C |
| 2b | Background **without** FGS | Process may be cached/killed; `PendingIntent` scan can wake for short work (Worker/Job) | Short bursts | Available | Best-effort | A [S18] |
| 3 | Screen locked | FGS continues; Doze/OEM policies may defer (D) | With FGS | Available (credential-encrypted storage after first unlock) | Best-effort | D → E5 |
| 4 | Killed by OS/OEM battery manager | FGS lowers the risk; OEM-specific kills are reported widely (C); GATT connections close on process death [S18] | Lost until restart | Kept | Next restart | A + C |
| 5 | **Force-stopped by the user** | App in "stopped state" (no broadcasts, alarms or `PendingIntent`s until opened) | None | Kept | None until opened | C (well-known platform behaviour, not re-verified in this session) |
| 6 | Restart | Can restart an FGS from `BOOT_COMPLETED`: `connectedDevice` is **not** on Android 15's prohibited list [S20] → probably allowed (D, verify) | After restart | Kept | Yes | A/D |
| 7 | Bluetooth off | None; resume on `ACTION_STATE_CHANGED` | None | Kept | None | A (D for details) |
| 8 | Permission revoked | No BLE | None | Kept | None | A |
| 9 | Battery saver | May restrict background work; OEM variance | ❓ | — | ❓ | D → E5 |
| 10 | Peer out of range | `autoConnect=true` reconnect when available [S18] | On reconnect | Kept | Next encounter | A |

**Policy risk:**
- Google Play requires every FGS type to be declared and justified, and all types are reviewed [S24].
- `connectedDevice` is described as "external devices" [S19]. Whether *phone-to-phone relaying* qualifies is a **policy interpretation that must be validated early** (R-07, Doc 10).
- `dataSync` is capped at 6 h/24 h on Android 15+ [S20], so it is unsuitable for always-on relaying.

## 5. Range, throughput, latency, power — what is known vs must be measured

| Quantity | Published evidence | Gap | Measurement (Doc 8) |
|---|---|---|---|
| Discovery latency | Smartphone measurements close to models but with higher variance; Wi-Fi traffic worsens it [S50] (Android 7-era) | iOS background latency; modern devices | E1 |
| Connection setup | none found | all | E1 |
| GATT throughput | iPhones ≈311–365 kbps; Android ≈1.2 Mbps (vendor test, older phones) [S49] | iOS 26/27, cross-platform pairs, L2CAP CoC | E3 |
| Envelope transfer time | — (2 KiB at 300 kbps ≈ 55 ms of airtime, by arithmetic; setup dominates — D) | real end-to-end | E3 |
| Range | no reliable single figure; environment-dominated | indoor/outdoor, body blocking | E4 |
| Energy (scan/advertise/connect/relay) | parameter-driven trade-offs; models deviate from measurements [S50, S51] | all, per OS state | E5 |
| Concurrent links | iOS undocumented; bitchat uses 6 [S27] | stability with N peers | E6 |

**No numeric targets are set until baseline measurements exist.** Doc 8 defines how targets are derived from the baselines.

## 6. Recommended transport strategy

1. **Primary (MVP): BLE, both roles, on both platforms.**
   - One fixed 128-bit service UUID; **no per-user data in adverts**.
   - GATT characteristic for framed transfer. L2CAP CoC is an *experimental upgrade* chosen per link after capability exchange (E3 decides).
2. **Android relay backbone:** opt-in "Relay mode" foreground service (`connectedDevice`) with an honest persistent notification. Subject to the Play policy check.
3. **iOS:**
   - Background modes plus state restoration plus wake-on-proximity pending connects (pattern from S27).
   - The UX educates users that **force-quitting stops relaying** (TN3115 recommends user education [S11]).
4. **Deferred secondary transports** (post-MVP, behind the transport abstraction):
   - Wi-Fi Aware between *paired contacts*. Pairing can piggyback on in-person QR pairing; DeviceDiscoveryUI is required on iOS [S13]. iPhone↔Android interop must be proven first.
   - Apple peer-to-peer (iOS↔iOS bulk).
   - Nearby Connections (investigate).
   - Same-AP LAN (needs the Android 17 local network permission [S23]).
5. **Alternative Architecture D:** dedicated always-on relay hardware (BLE or LoRa nodes) if phone density or iOS background limits make delivery rates unacceptable.

## 7. Interoperability testing that is mandatory before architecture approval

- Matrix of {iPhone models × iOS 26/27} × {Android OEMs (Pixel, Samsung, one budget OEM) × Android 14–17}. Each pair is tested in states {FG, BG, locked}, both initiator directions, MTU negotiation and fragmentation, and L2CAP CoC open/close (E1, E3, E6).
- Address-rotation behaviour: can an iPhone's pending connect to a previously seen Android peripheral complete after the Android address rotates? (E2)
