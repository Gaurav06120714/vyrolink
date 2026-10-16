# 02 — Architecture Comparison

**Accessed:** 2026-10-15. Labels as in the app reports.

## 1. Architecture families

| Family | Members | Transport | Ordering / sync authority | Offline behaviour |
|---|---|---|---|---|
| **Central server, pairwise E2EE** | WhatsApp, Signal, Messenger, Viber, Threema, Wire (pre-MLS), iMessage | TLS to a vendor server; server queues | **Server** orders and stores until delivery | None without internet |
| Central server, group E2EE (MLS) | Wire, Google/Apple RCS, Discord DAVE (calls) | Server or Delivery Service | **The DS must order MLS commits** (RFC 9420 assumption) | None |
| Central server, not E2EE | Telegram cloud chats, WeChat, Discord text, Instagram DM (2026) | Server | Server | None |
| Federated | Matrix/Element, Delta Chat (email) | Homeserver / IMAP-SMTP | Per-room DAG (Matrix); none (email, so out-of-order) | Store-and-forward via servers |
| Decentralised overlay | Session (onion + swarms), SimpleX (one-way queues on relays), Berty (libp2p) | Internet relays | Per-queue FIFO (SimpleX) | Server-side store-and-forward |
| **Local P2P, contacts only** | Briar | Bluetooth, Wi-Fi, Tor | Per-contact sync (offer/request/ack) | **Direct contact encounters only** |
| **Local mesh, flood/gossip** | bitchat, Bridgefy, FireChat, Meshtastic | BLE (bitchat, Bridgefy, FireChat), LoRa (Meshtastic) | None (TTL flood + dedup) | Multi-hop while connected; bitchat adds a **courier** store-and-forward |
| Delay-tolerant overlay | Reticulum/LXMF | Any medium | Propagation nodes | Store-and-forward |
| **VYRO LINK** | — | BLE GATT (+ L2CAP later) | **None; no ordering authority exists** | **Spray-and-wait store-and-forward through strangers** (opaque envelopes) |

## 2. What VYRO LINK cannot borrow, and why

| Mechanism in others | Requires | VYRO LINK status |
|---|---|---|
| Server-ordered edits/deletes | An ordering authority | **Replaced** by sender-authenticated events keyed to the target message ID, with order-independent merge rules (05 §4) |
| MLS groups | A DS that orders commits; forks otherwise (DMLS drafts) | **Rejected for MVP groups**; pairwise fan-out instead (05 §6) |
| Multi-device via server mailbox (Labyrinth, Signal linked devices) | Server | **Deferred**. Session shows the cost: it dropped PFS in 2021 for device sync |
| Real-time calls (DAVE, WebRTC) | Continuous low-latency link | **Rejected** |
| Contact discovery by phone number | Directory server | **Rejected** (in-person pairing only) |

## 3. Data-model patterns worth adopting (VERIFIED in source)

| Pattern | Source | How VYRO LINK uses it |
|---|---|---|
| Changes are new messages referencing a **target message ID** | Matrix (replacement/redaction events), SimpleX `XMsgUpdate/XMsgDel/XMsgReact {msgId}`, Meshtastic `reply_id` | Inner kind `CONTENT` with `target` (Doc 21 §4.1) |
| **Tombstone** for a delete that arrives before its target | Delta Chat `insert_tombstone` | `tombstones` table; a late original is stored as deleted |
| Per-user latest reaction replaces the earlier one | Delta Chat (XEP-0444) | Reactions keyed by (target, reactor), latest `sent_at` wins |
| Disappearing timer carried in every message, last-writer-wins | Briar auto-delete | `timer_secs` in every CONTENT text; the conversation timer follows the newest |
| Per-contact feature preferences | SimpleX | Deferred to Tier 2 (MVP: the timer is per conversation) |
| Creator-signed group roster + epoch key | bitchat groups | Considered; rejected for MVP (new shared-key design); pairwise fan-out chosen |
| Store-and-forward quotas by trust tier | bitchat courier | Already in vl-routing (per-tag, per-session, carry cap); trust tiers deferred |
| Solicited-only sync responses | bitchat RSR | Already in vl-routing (Doc 21 §2 rule 7) |

## 4. Core layering compared

| App | Shared core language | UI | Notes |
|---|---|---|---|
| Delta Chat | **Rust** (`chatmail/core`) + SQLite + FFI/JSON-RPC | Native/Electron | Closest structural match to VYRO LINK |
| Threema | `libthreema` (Rust, emerging) | Native | REPORTED |
| Wire | `core-crypto` (Rust) | Native | REPORTED |
| Element X | `matrix-rust-sdk` + UniFFI | SwiftUI / Compose | Same binding tool as VYRO LINK |
| Signal | `libsignal` (Rust) | Native | — |
| bitchat | Swift (iOS) / Kotlin (Android) duplicated | Native | Two implementations to keep in sync |
| **VYRO LINK** | **Rust** (vl-*) + SQLCipher + UniFFI | Flutter + native BLE hosts | ADR-013, ADR-022 |

**Conclusion:** the Rust-core + bindings architecture matches the industry direction (Signal, Element X, Wire, Threema, Delta Chat). **No change to ADR-013 or ADR-022 is warranted.**
