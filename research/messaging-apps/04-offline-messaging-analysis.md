# 04 — Offline-Messaging Analysis

**Accessed:** 2026-10-15.

**Defining requirement:** privacy-preserving, encrypted, offline-first messaging, where nearby devices store and forward messages to recipients they meet later, with no internet.

## 1. How each offline-capable system behaves

| System | Reaches offline recipients via | Carriers | Lifetime | Copies | Delivery proof |
|---|---|---|---|---|---|
| bitchat | Mesh flood (TTL) while connected; **courier** carries for mutual favorites or verified peers; Nostr if online | Trusted peers only | 24 h | ≤ 8 (spray-and-wait) | Delivered/read receipts inside Noise (VERIFIED) |
| Briar | Direct contact encounter; Mailbox over Tor | Contacts only (no strangers) | Until synced | 1 | Sync acks |
| Meshtastic | LoRa managed flood; Store & Forward router | Any node | Router-configured | Flood | ACK |
| Reticulum | Propagation nodes | Nodes (anyone can run one) | Node-configured | Node replication | ? |
| FireChat | Live multi-hop only | Anyone nearby | Live | Flood | ? |
| Apple satellite | Satellite link | Apple infrastructure | — | — | iMessage receipts |
| **VYRO LINK** | **Spray-and-wait through any willing relay (opaque envelopes)** | **Strangers too** (relay class, quotas) | 72 h default, ≤ 7 d | **8 (ADR-021)** | **Receipt or vaccine preimage (ADR-020)** |

**VYRO LINK's difference:** strangers can carry, because envelopes are opaque, padded and tagged with unlinkable pairwise tags. That widens reach compared with bitchat's favorites-only courier and Briar's contacts-only sync. The cost is abuse exposure, which our quotas bound. This is consistent with Doc 22's simulation and is **not yet device-validated**.

## 2. What offline changes for each candidate feature

| Feature | Offline semantics | Feasible? |
|---|---|---|
| Replies | The target may not have arrived yet; render "reply to a message you don't have yet" | ✓ |
| Reactions | Arrive out of order; latest by sender time wins; may precede the target | ✓ |
| Edit | May arrive before the target or after it was read; must be **held**; window measured on the sender's clock | ✓ with bounds |
| Delete for everyone | May arrive before the target (tombstone); may never arrive (envelope expiry) | ✓, wording "requested" |
| Disappearing messages | Timer travels with each message; the local clock decides | ✓ |
| Read receipts | Extra envelopes; leaks reading time; weak delivery in sparse networks (Doc 20 §5.1) | Deferred (off by default; already an existing inner kind) |
| Typing indicators / presence | Meaningless over delay-tolerant routing and leak metadata | **Rejected** |
| Real-time calls | Need a continuous low-latency link | **Rejected** |
| Media / attachments | BLE goodput is small (iOS GATT reports 300–400 kbps notify; L2CAP 37–138 kbps measured device-to-device; forum data, REPORTED) | Small images only (Tier 2); large files deferred |
| Voice notes | ~6 kbps Opus → 45 KB/min, which is many envelopes | Tier 3 (short clips, direct encounters only) |
| Groups | Fan-out multiplies envelopes by member count | Tier 2, small groups (≤ 8) |
| Search, drafts, pins, bookmarks, blocking | Purely local | ✓ |
| Offline notifications | OS background limits (TN3115: force-quit iPhone not relaunched) | Native layer; Phase 7 |
| Local announcements (broadcast) | Unauthenticated broadcast enabled Bridgefy impersonation and spam | Deferred (needs a signed, rate-limited design) |

## 3. Network cost per feature

Every envelope is one of 256 / 512 / 1024 / 2048 bytes and sprays up to 8 copies. A control event (reaction, delete, edit) costs one envelope, which pads to 256 bytes. **Cost model:** user-visible traffic ≈ envelopes × copies × bucket.

| Event | Bucket | Envelopes | Notes |
|---|---|---|---|
| Text ≤ ~150 B (normal Olm message) | 256 | 1 | — |
| Reaction / delete | 256 | 1 | — |
| Edit (short) | 256–2048 | 1 | — |
| Receipt | 256 | 1 | Wrap-only |

**Battery:** the dominant cost is BLE radio time per encounter (Experiment C, **NOT TESTED**). Feature design therefore minimises *envelope count*. For example, a reaction "remove + add" is sent as one event, not two.
