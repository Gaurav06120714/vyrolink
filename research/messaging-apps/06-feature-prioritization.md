# 06 — Feature Prioritization

**Decided:** 2026-10-15. **Basis:** apps/*, 01–05.

**Scoring:**
- 1–5 per criterion: user value (V), feasibility offline (F), complexity (C, 5 = simple), security impact (S, 5 = safe), storage cost (St, 5 = cheap), battery (B, 5 = cheap), iOS/Android restriction (P, 5 = none), core compatibility (K, 5 = no wire change).
- **Device-independent** = can be built and tested without phones.

## Tier 1 — essential (approved for implementation now)

| ID | Feature | V | F | C | S | St | B | P | K | Device-independent | Rationale |
|---|---|---|---|---|---|---|---|---|---|---|---|
| F-01 | In-person QR contact pairing | 5 | 5 | — | 5 | 5 | 5 | 4 | 5 | Core yes; camera UI no | **Already implemented** |
| F-02 | Transparent delivery states | 5 | 5 | — | 5 | 5 | 5 | 5 | 5 | Yes | **Already implemented** (ADR-015/020) |
| F-03 | Replies (quote by ID) | 5 | 5 | 4 | 5 | 5 | 5 | 5 | 4 (new kind) | Yes | Universal; tiny; Meshtastic/SimpleX evidence |
| F-04 | Reactions (8 fixed) | 4 | 5 | 4 | 5 | 5 | 4 | 5 | 4 | Yes | Universal; SimpleX fixed-set evidence; 1 envelope |
| F-05 | Delete for everyone (request) + delete for me | 4 | 4 | 3 | 4 | 5 | 4 | 5 | 4 | Yes | Expected; tombstones solve out-of-order arrival (Delta Chat) |
| F-06 | Local message search | 4 | 5 | 4 | 4 | 4 | 5 | 5 | 5 | Yes | Purely local; FTS5 already compiled in |
| F-07 | Drafts | 3 | 5 | 5 | 5 | 5 | 5 | 5 | 5 | Yes | Purely local |
| F-08 | Blocking | 4 | 5 | 4 | 5 | 5 | 5 | 5 | 5 | Yes | Safety baseline; saves bandwidth (stop requesting their tags) |
| F-09 | Disappearing messages | 4 | 5 | 3 | 4 | 5 | 5 | 4 | 4 | Yes | Strong privacy value; Briar semantics proven offline |

## Tier 2 — high value (approved in principle; after Tier 1)

| ID | Feature | Notes |
|---|---|---|
| F-10 | Edit (24 h, ≤ 10 edits, history marker) | Needs the orphan/hold logic; the wire design is shared with F-05 |
| F-11 | Pins / bookmarks (local) | Local-only; Signal-style pin expiry is optional |
| F-12 | Forwarding (new message + "forwarded" flag) | Local + a CONTENT flag |
| F-13 | Small groups (≤ 8) via pairwise fan-out | No new crypto; n× envelopes |
| F-14 | Small image attachments (≤ 16 KiB, chunked, resumable) | Needs chunk/manifest spec + simulator evaluation |
| F-15 | Offline notifications (native) | Phase 7; OS-bound |
| F-16 | Panic wipe | bitchat precedent; deletes keys + both DBs |
| F-17 | Privacy controls (per-contact preferences) | SimpleX model |
| F-18 | Receipt-return improvement | Simulator-first (Doc 20 §5.1) |

## Tier 3 — optional

| ID | Feature | Notes |
|---|---|---|
| F-19 | Voice notes (short, Opus) | Heavy on BLE; direct encounters |
| F-20 | Polls | Groups first |
| F-21 | Encrypted local export/backup | User-held key; replay-state caveat (Threema) |
| F-22 | "Paper" message via QR (Reticulum idea) | Zero-radio fallback |
| F-23 | Introductions (Briar) | Trust transfer; needs design |

## Deferred

| Feature | Why |
|---|---|
| Multi-device | Session dropped PFS for it; needs a server-free sync design |
| Post-quantum hybrid | No reviewed fit yet; tracked as R-NEW-1 |
| Megolm groups with a group tag | Needs a reviewed group-addressing design |
| Large file transfer / erasure coding | Await BLE throughput measurements (Experiment C) |
| Local announcements / broadcast | Bridgefy impersonation and spam evidence; needs a signed, rate-limited design |
| Read receipts | The kind exists; off by default; costs envelopes and leaks reading time |

## Rejected

| Feature | Why |
|---|---|
| Real-time voice/video calls | Need a continuous low-latency link; BLE plus intermittency cannot support them |
| Server or cloud sync, cloud backup, internet fallback | ADR-012 offline-only; metadata |
| Phone-number or address-book discovery | Identity leakage; in-person pairing only |
| Typing indicators, online presence, last seen | Meaningless over delay-tolerant routing; metadata |
| Channels / large public broadcast | Abuse and DoS surface |
| Screenshot blocking claims | Not enforceable on iOS |
| Compression of message content | Decompression-bomb history (Doc 06) |
