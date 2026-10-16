# Document 2 — Apple Find My: Technical Research

**Status:** research only · **Checked:** 2026-10-14 · **Evidence levels:** A official · B reproducible/code-inspected · C credible unverified · D hypothesis (see Doc 12)

> **Bottom line.** Find My is a *one-way, crowd-sourced, infrastructure-dependent* reporting system. Phones relay tiny encrypted observations **to Apple's servers over the internet**; nothing is forwarded phone-to-phone, and there is no public API to send third-party payloads through it. VYRO LINK cannot use it as a transport, and must not try (abuse such as "Send My" [S04] is out of scope). What *is* reusable is the privacy engineering: identity-unlinkable rotating identifiers derived from a secret only the owner holds, encryption-to-recipient by untrusted finders, and an untrusted relay that stores ciphertext indexed by an opaque tag.

---

## 1. Public architecture (Level A unless noted)

### 1.1 Roles

| Role | Who | What it does | Source |
|---|---|---|---|
| **Owner device** | The user's own Apple devices signed into the same Apple Account | Generates the master key material, later derives the same rotating keys, queries reports, decrypts them | S01 |
| **Lost device / accessory** ("device being located") | Offline iPhone/Mac/AirTag/Find My network accessory | Broadcasts the *current* derived public key over BLE | S01, S08 |
| **Finder** ("reporting device") | Any nearby iPhone/iPad/Mac with offline finding enabled | Hears the beacon, encrypts **its own location** to the broadcast key, uploads via **its own internet connection** | S01 |
| **Apple servers** | Apple | Store encrypted reports indexed by a hash of the public key; cannot decrypt; designed not to link finder and owner | S01 |

### 1.2 Key material and rotation (S01, Level A)

- On enabling offline finding, the device generates an **EC P-224 key pair {d, P}** plus a **256-bit secret SK₀** and counter *i*. "This private key pair and the secret are never sent to Apple"; they sync only among the owner's devices via end-to-end-encrypted iCloud Keychain.
- Rotation: SKᵢ = KDF(SKᵢ₋₁, "update"); (uᵢ, vᵢ) = KDF(SKᵢ, "diversify"); dᵢ = uᵢ·d + vᵢ; Pᵢ = uᵢ·P + vᵢ·G.
- "Approximately every 15 minutes, the public key is replaced by a new one" so the user "can't be tracked by a persistent identifier."
- P-224 is chosen because "the public key representation can fit into a single Bluetooth payload."

### 1.3 Reporting and retrieval (S01, Level A)

1. Lost device broadcasts Pᵢ.
2. Finder encrypts its location to Pᵢ and uploads. "The traffic sent to Apple by finder devices contains no authentication information."
3. "Apple doesn't know who the finder is or whose device has been found" and "retains no information that would allow anyone to correlate the finder and owner."
4. Owner re-derives the Pᵢ for the relevant time window, queries by key hash, decrypts locally.

### 1.4 Where the internet sits

| Stage | Medium | Internet needed? |
|---|---|---|
| Beacon → finder | BLE advertisement, connectionless | No |
| Finder → Apple | Finder's Wi-Fi/cellular | **Yes** |
| Apple → owner | Owner's internet | **Yes** |

**Consequence for VYRO LINK:** Find My's "offline" refers only to the *lost device*. The delivery path is infrastructure-based. That is the opposite of VYRO LINK's goal: no hop in VYRO LINK may depend on a server.

### 1.5 Anti-tracking protections

- Since 2024-05-13 Apple and Google have shipped cross-platform unwanted-tracker alerts (iOS 17.5, Android 6.0+) based on the joint *Detecting Unwanted Location Trackers* specification. IETF DULT standardisation continues; the threat-model draft -05 is dated 2026-08-06 [S06, S07] (A).
- Third-party tags join only through the **MFi Find My network accessory program**, whose specification is under NDA [S08] (A).

### 1.6 What Find My is *not*

| Thing | Relationship |
|---|---|
| AirTag | One accessory type on the network |
| Find My network accessory program | MFi licensing path for hardware vendors; not an app API |
| Ordinary BLE app (Core Bluetooth) | Separate; it cannot emit Apple's Offline Finding advertisement from an iPhone app, because `startAdvertising` only accepts local name and service UUIDs [S10] (A) |
| Messaging API | **None exists** |

---

## 2. Reverse-engineered and observed behaviour (with evidence levels)

| # | Finding | Evidence | Source | Notes |
|---|---|---|---|---|
| F1 | Offline-finding beacon is Apple manufacturer data `0x4C00`, type `0x12`, length `0x19`, status byte, then **22 bytes of the key**. The **remaining 6 key bytes are carried in the BLE random static address** (top two bits forced to `11`, original bits restored in payload byte 29). | **B** (code read) | S03 `Firmware/ESP32/main/openhaystack_main.c` (`set_addr_from_key`, `set_payload_from_key`) | Shows how a 28-byte P-224 x-coordinate fits a 31-byte legacy advertisement: the address is reused as key material |
| F2 | Report decryption = ECDH(private key, finder's ephemeral key) → KDF **SHA-256(shared ‖ be32(1) ‖ ephemeral pubkey)** → AES-GCM; latitude/longitude are int32 / 1e7 | **B** (code read) | S03 `FindMy/DecryptReports.swift` | ANSI X9.63-style KDF, matching the ECIES pattern |
| F3 | Reports are fetched by public-key hash, but **downloading requires an authenticated Apple account**; OpenHaystack uses a Mail plugin with Mail's entitlements, plus anisette data | **B** | S03 README; `AnisetteDataManager.swift` | Fetching depends on private Apple infrastructure. VYRO LINK must not replicate this. |
| F4 | OpenHaystack firmware broadcasts a **fixed key** (no rotation), so it is trackable | **B** | S03 README disclaimer | Shows that rotation is what provides the privacy |
| F5 | Two design/implementation flaws in the 2021 analysis: a **location-correlation attack** and unauthorised access to **up to 7 days of location history** (CVE-2020-9986, since fixed; Apple "partially addressed" the issues) | **B** | S02, S84, S03 | Taken from the PoPETs abstract page (S84); full text not re-read |
| F6 | **Send My**: the Find My network can carry arbitrary bits (a few bytes/s) from offline sensors; no rate limit was observed in 2021 | **C** | S04 | Shows crowd relays can be abused as covert channels. Whether it has been patched is unverified. |
| F7 | **nRootTag**: any Bluetooth device's existing address can be matched by searching for a key pair, making it trackable via Find My without root; >90 % success reported; the paper's appendix lists Apple patches (iOS 18.2 et al.), effective only once nearby finders update | **B** | S05 | Validation of reports by the finder is a security boundary |
| F8 | Rotation "approximately every 15 minutes" (official). Longer rotation intervals for separated AirTags are widely reported but **not officially documented here** | A / **D** for the longer intervals | S01 | Treat any non-15-minute figure as unverified |

### 2.1 Documented versus observed

- Apple documents the cryptographic intent (S01). Researchers documented the wire format and report pipeline (S02, S03), which **matches** the official description wherever both cover the same point.
- Undocumented officially, and only research-derived: the beacon byte layout (F1) and report-fetch authentication (F3). These could change at any time and are **not guaranteed features**.

---

## 3. Find My versus offline messaging

| Dimension | Apple Find My | VYRO LINK (target) |
|---|---|---|
| Primary purpose | Locate an owned device | Deliver private text between people |
| Device discovery | Finder passively hears connectionless beacons | Mutual discovery, then a connection |
| Direct communication | None (beacon only) | Required (GATT/L2CAP session) |
| Relay behaviour | Finder → **Apple server** (1 hop, infrastructure) | Phone → phone → … (multi-hop, no server) |
| Payload | Finder's location (~tens of bytes, generated by the finder) | Sender-generated ciphertext (≈0.25–2 KiB, MVP) |
| E2E encryption | Finder → owner (ECIES to the rotating key) | Sender → recipient, with forward secrecy desired |
| Recipient identification | Hash of the rotating public key; owner can recompute it | Opaque rotating tag computable only by sender and recipient (Doc 6) |
| Delivery acknowledgement | None (the owner pulls) | Required: authenticated, travels back through the network |
| Store-and-forward | Server stores; devices don't | Every relay stores and carries |
| Internet dependency | Finder and owner need internet | None |
| Background execution | System-level daemon with privileges unavailable to apps | App bound by Core Bluetooth / Android FGS rules (Doc 4) |
| Battery | OS-optimised, shared radio duty cycle | App-level scanning/advertising/connections; must be measured |
| Cross-platform | Apple-only finders (Android only detects trackers, via DULT) | iOS + Android required |
| Developer API | None (MFi accessory spec only) | Core Bluetooth / Android BLE |
| Privacy properties | Finder and owner anonymous to Apple; unlinkable beacons | Relays learn minimal metadata; beacons unlinkable to identity |
| Infrastructure | Apple's servers, iCloud Keychain sync | None in MVP |

### Why the security requirements differ

1. **Who generates the payload.** In Find My the *finder* generates the data and has nothing to gain by lying beyond pollution attacks. In messaging, the *sender* generates arbitrary payloads that relays must store, so relays are exposed to spam, storage exhaustion and malformed-input attacks (the Bridgefy decompression bomb, S28/S29).
2. **Delivery semantics.** Find My is best-effort, latest-wins and loss-tolerant. Messaging requires exactly-once presentation, deduplication across paths, ordering hints and acknowledgements that themselves need routing.
3. **Routing.** Find My is a star (everyone → Apple). Messaging over phones is a time-varying graph that needs routing decisions *at every hop* (Doc 5).
4. **Trust.** Apple is a single trusted-for-availability operator. VYRO LINK relays are anonymous strangers, any of whom may drop, replay, or flood traffic.
5. **Forward secrecy.** Find My stores location ciphertext on a server the owner trusts for availability. VYRO LINK stores message ciphertext on strangers' phones for hours, so key compromise later is more damaging, and forward secrecy matters more (Doc 6).

---

## 4. What VYRO LINK can learn

| # | Principle | How it maps to VYRO LINK | Classification |
|---|---|---|---|
| L1 | **Rotating, unlinkable on-air identifiers** derived from a secret | BLE advertisements carry only a fixed app service UUID and **no per-user identifier**. Per-message recipient tags rotate per epoch, derived via HMAC from a pairwise secret (Doc 6 §4) | Architectural principle, independently implementable. Note: on iOS the app **cannot control the BLE address**; the OS randomises it [S10] |
| L2 | **Separate identity from observable identifiers** | Identity keys never appear on air or in envelope headers; only tags and random envelope IDs do | Independently implementable |
| L3 | **Encrypt to the recipient so untrusted relays carry ciphertext** | E2E envelope; relays see opaque bytes (bitchat does the same [S27]) | Independently implementable |
| L4 | **Index by a hash the owner can recompute** | Recipient precomputes the tag table for its contacts × active epochs and matches incoming summaries | Independently implementable |
| L5 | **Minimise metadata in reports** (no finder authentication) | Relay-to-relay sessions are anonymous (ephemeral Noise NN); envelopes carry no sender ID | Independently implementable; trades off with abuse resistance (Doc 6 §7) |
| L6 | **Opportunistic, passive participation** | Phones relay when they happen to meet | Depends on platform permissions and background modes (Doc 4); best-effort on iOS |
| L7 | **Energy-efficient background scanning** | Use OS-filtered scans by service UUID; never periodic polling [S18]; duty-cycle | Depends on platform permissions; must be measured (Doc 8) |
| L8 | **Anti-tracking by design** | VYRO LINK must not create a stable beacon. Do not broadcast a static device ID. bitchat's self-declared weakness is exactly this [S27 §8] | Principle; testable with sniffing (Doc 13) |
| L9 | **Validate what relays accept** (lesson of nRootTag/Send My) | Strict envelope-size/expiry/budget validation; quotas; reject malformed input before parsing; no decompression | Independently implementable |
| L10 | **Use Apple's network to carry messages** | — | **Unsuitable: depends on Apple's private infrastructure, violates terms, and is an abuse pattern (S04)** |
| L11 | **Become a Find My accessory** | — | Unsuitable (MFi hardware program, location-only, NDA) |
| L12 | **Whether iPhones in pockets can reliably serve as relays** | Find My works because it is an OS service. Apps get fewer privileges (TN3115, S11) | **Hypothesis to validate experimentally** (Doc 8, E2) |

## 5. Inaccessible or unsuitable capabilities (explicit)

- Find My network as a message transport → **not available, not to be attempted.**
- Emitting Apple Offline-Finding advertisements from an iOS app → not possible with public APIs (S10).
- Retrieving location reports → requires an authenticated Apple account and private API (F3).
- The MFi accessory specification → NDA (S08).
- OS-level background privileges that `searchpartyd`-style system services enjoy (name not verified here) → not available to third-party apps.

## 6. Final architectural principles for VYRO LINK (2026-10-14)

| Principle | Decision | How |
|---|---|---|
| Separate cryptographic identity from anything observable on air | **Adopt** | Adverts carry only a fixed service UUID; identities live inside E2E layers (Doc 18 §7.1) |
| Rotating, unlinkable identifiers derived from a secret | **Adopt (adapted)** | Pairwise epoch-rotating recipient tags (ADR-009); the OS rotates BLE addresses ([S73]; interval unpublished) |
| Encrypt to the recipient so untrusted carriers hold only ciphertext | **Adopt** | Olm inside a pairwise wrap (ADR-007/014) |
| Index stored data by a value only the owner can recompute | **Adopt** | Recipient tag table (Doc 18 §9.2) |
| Anonymous, unauthenticated reporters/relays | **Adopt with limits** | Noise NN links. Abuse is bounded by budgets and quotas, not prevented (Sybil residual) |
| Validate everything relays accept (lesson of nRootTag / Send My) | **Adopt** | Fixed-header strict parsing, size/expiry/budget checks, no decompression |
| Anti-tracking by design (no stable beacon) | **Adopt** | No static IDs; experiment E10 verifies |
| OS-level, always-on crowd participation | **Reject as an assumption** | Third-party apps get best-effort background time only ([S09, S11]) |
| Relaying through Apple's Find My network / servers | **Reject** | Private infrastructure; abuse pattern [S04] |
| Find My accessory program (MFi) | **Reject** | Hardware/location-only, NDA [S08] |
| Per-message (not per-epoch) unlinkable tags | **Investigate** | S-R1 (table-size trade-off) |
| Rotation interval effects on long-term linkability given the fixed UUID | **Investigate** | S-R4 / E10 |

## 7. Open questions

1. How long iOS 26/27 keeps a backgrounded third-party BLE scanner effective, with the screen locked, over hours (Doc 8, E2) — **D**.
2. Whether tags rotating per epoch create correlation through message timing (traffic analysis), as studied for Find My — **D**.
