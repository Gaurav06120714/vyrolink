# Document 9 — MVP Scope, Product Strategy and Competitive Analysis

## 1. Target users and use cases

The product must earn its place against the realities in Doc 4: best-effort delivery, no server, and limited iPhone background behaviour. The best-fit users are **groups who know each other, meet in person, and are temporarily without connectivity**. These are the use cases the evidence supports:

| Use case | Why it fits | Evidence / caveat |
|---|---|---|
| Crowded events where the cellular network saturates (festivals, stadiums) | High device density helps spray-and-wait; people pair with friends beforehand | Density and delivery are hypotheses (E9, E12) |
| Remote outdoor groups (hiking, expeditions, field research) | Small, known groups that are physically co-located but intermittently out of range | Low density, so long latency; LoRa (Architecture D) may fit better |
| Campus / workplace during outages | Moderate density; daily mobility patterns | Needs enough installs to form paths |
| Disaster preparedness | Infrastructure loss | **Safety-critical claims must be avoided.** Best-effort delivery cannot be marketed as emergency communication without validation |
| Privacy-sensitive local communication | No servers, E2E | Metadata limits (Doc 6 §9) must be communicated honestly |

**Not a fit:** reaching strangers; guaranteed or real-time messaging; large media; replacing emergency services.

## 2. Feature prioritisation

| Feature | Classification | Rationale |
|---|---|---|
| Offline 1:1 text | **Essential (MVP)** | Core value |
| Multi-hop relaying (spray-and-wait) | **Essential** | Core differentiator over direct-only apps |
| QR contact pairing | **Essential** | Only trustworthy key distribution without a server (ADR-005) |
| E2E encrypted conversations | **Essential** | — |
| Delivery receipts (Delivered) | **Essential** | Honest states (§5) |
| Offline queues | **Essential** | — |
| Automatic discovery | **Essential** | Without it there is no opportunistic relaying |
| User-controlled relay participation | **Essential** | Battery consent; Android FGS needs it (ADR-011) |
| Contact verification (safety number) | **Essential** (optional to use) | Defends against QR substitution |
| Message expiry | **Essential** | Bounds every store (Doc 7 §8) |
| Read receipts | After MVP (off by default) | Privacy cost |
| Private discovery beyond "no identity in adverts" (e.g. hiding the service UUID) | **Research-intensive** | A fixed UUID is required for OS-filtered background scans [S10b] |
| Group messaging | **Research-intensive** | MLS/DTN conflict (Doc 6 §11) |
| Images / files | Valuable after MVP; **research-intensive over BLE** | Throughput, storage abuse; needs Wi-Fi Aware/Apple P2P between contacts |
| Voice messages | Valuable after MVP (short clips only via a contact fast path) | Same |
| Location sharing | **Unsafe without additional protections** | Stalking risk; DULT lessons [S06, S07] |
| Anonymous communication with strangers | **Unsafe / inappropriate** for MVP | Spam, abuse, impersonation (Bridgefy [S28]) |
| Network visualisation (who is nearby, topology) | **Unsafe** as designed elsewhere | Leaks presence/topology (bitchat §8 [S27]) |
| Emergency/SOS broadcast | **Unsafe without protections and validation** | False expectation of delivery; abuse via broadcast |
| Optional internet-assisted sync | After MVP | Re-introduces infrastructure and metadata; ADR-012 |
| Multi-device per user | After MVP | Sesame-class complexity |

**Explicitly excluded from MVP:**
- media, groups, broadcast/public channels, location, nearby-people lists;
- internet bridge, multi-device, read receipts on by default;
- compression of any kind (decompression-bomb lesson).

## 3. User-experience principles

1. **Truthful delivery states.** Never show "sent" when the message only reached a relay (Doc 7 §7).
2. **Explain the physics.** "Messages travel when people carry phones near each other. This can take minutes or days."
3. **Relay consent is visible.** On Android this is the persistent notification; iOS gets an in-app explanation that force-quitting stops relaying [S11].
4. **No social exposure.** No lists of nearby users and no public profiles.
5. **Pair in person, verify optionally.**
6. **Battery transparency.** Show the measured relay cost (from E5) in settings.

## 4. Product limitations (to be stated in-product)

All the items in Doc 6 §10. In addition: delivery depends on other users' installations and on their app/battery settings.

## 5. Delivery-state definitions

These are defined normatively in Doc 7 §7. The product requirement that feeds the E1/E2 success thresholds must be expressed as follows. *"For two paired users within the same building with ≥ k relaying devices present, the median time-to-delivery should be ≤ T."* Here k and T are **set after** the E9 simulation and the E1/E5 baselines, not before.

## 6. Competitive and originality analysis

### 6.1 Comparison

| | Infrastructure independence | iOS + Android | Multi-hop | Privacy (relay metadata) | Security evidence | Background | Ease of use | Hardware | Delivery feedback | Open source |
|---|---|---|---|---|---|---|---|---|---|---|
| **bitchat** [S27] | Yes (+ optional Nostr) | Yes | Yes (flood + couriers) | Weak by own admission (static IDs, clear announces) | No independent audit found | iOS bg modes + wake-on-proximity; Android FGS | Open chat, no pairing needed | Phones | Delivery + read receipts | Yes (Unlicense / GPL) |
| **Briar** [S31] | Yes (+ Tor) | Android only | Contacts-only sync | Strong (Tor), contacts see each other | Cure53 2017 | Android FGS | Moderate | Phones | Yes | Yes (GPL) |
| **Bridgefy** [S28–S30] | Yes | Yes | Yes | Broken historically | Two academic breaks | Unknown | Easy | Phones | Unknown | No |
| **Meshtastic** [S33] | Yes | Via hardware | Yes (LoRa) | Channel-level | CVE history | Radio does it | Needs hardware setup | LoRa node per user | Partial | Yes (GPL) |
| **VYRO LINK (proposed)** | Yes | Yes | Yes (spray-and-wait) | **Rotating pairwise tags, no on-air identity, anonymous links** | Planned independent review | Same constraints as bitchat | Requires in-person pairing | Phones | Honest custody/delivered/unknown states | Licence TBD |

### 6.2 What is genuinely new?

**Almost nothing is novel in isolation.**

| VYRO LINK feature | Already solved by |
|---|---|
| BLE mesh across iOS/Android | bitchat |
| Spray-and-wait couriers + daily tags | bitchat |
| E2E with ratchets | Signal, Matrix (Olm) |
| QR pairing | Briar, Signal |
| DTN routing | Literature since 2000 |

**A defensible differentiation must be measurable:**
1. **Relay-metadata minimisation.** No stable sender ID, and no identity on air or in announcements. Measurable by E10, comparing linkability against bitchat's documented exposure.
2. **Forward secrecy for carried mail** via OTKs exchanged at pairing. bitchat lists this as future work [S27 §9].
3. **Truthful delivery semantics.** Measurable by E7 and a user-comprehension survey (E12).
4. **A single shared protocol core** across platforms, which reduces drift (ADR-002/004). Measurable by conformance tests.
5. **Published cross-platform background measurements** (E1/E2). These would be useful to the whole ecosystem.

**Honest strategic alternative:** bitchat is public-domain on iOS and active. Contributing items 1–2 upstream could deliver the same user value faster. Building VYRO LINK separately is justified only if (a) contacts-only, pairing-first trust is a deliberate product choice, which differs from bitchat's open-chat model, and (b) the licence and ownership goals require it. **Decision for the product owner.**

### 6.3 Underserved scenarios supported by evidence

The evidence for underserved demand is **weak**: no market research was performed in this package. The scenarios in §1 are engineering-plausible, not market-validated. **Recommended:** user interviews alongside P1 (Doc 11).
