# Document 7 — Protocol and Data Model Specification (conceptual, v1-draft)

**Status:** conceptual specification for implementation planning; **not normative**. Byte layouts are proposals to be validated in the transport PoC (P3) and the security PoC (P4). All sizes are in bytes; integers are big-endian.

> The protocol has **four layers**:
> - **L0 — BLE discovery:** service UUID only.
> - **L1 — link session:** Noise NN-encrypted frames over GATT or L2CAP.
> - **L2 — envelopes:** what relays store and forward.
> - **L3 — end-to-end inner messages:** pairwise wrap + Olm.
>
> Relays implement L0–L2 only. They never parse L3.

---

## 1. L0 — Discovery announcement (BLE advertisement)

| Field | Value | Notes |
|---|---|---|
| Service UUID | One fixed 128-bit VYRO LINK UUID (to be generated at P3) | The only content. iOS background → overflow area [S10] |
| Local name | **Omitted** | Would be a stable identifier; not advertised in iOS background anyway |
| Service/manufacturer data | **None in v1** | iOS apps cannot set them [S10]. Keeping both platforms identical avoids asymmetric fingerprinting |

**Confidentiality:** none (public). **Authentication:** none. **Replay:** n/a.
Rationale: Doc 2 L1/L8 and Doc 6 T1.

## 2. L1 — Link session protocol

### 2.1 Framing

- A BLE connection carries **frames**: `len(u16) ‖ payload`.
- Transport fragmentation is below this layer: it is the GATT write/notify MTU chunking, or an L2CAP CoC stream.
- All frames after the handshake are Noise transport messages (max 65535 bytes per the Noise spec).
- Each Noise message carries one link message: `type(u8) ‖ body`.

### 2.2 Handshake

`Noise_NN_25519_ChaChaPoly_SHA256`, 2 messages. Fresh ephemeral keys per connection. **No static keys** (anonymous). The prologue is `"VL/link/v1"`, which binds the protocol version.

### 2.3 Link message types

| Type | Name | Direction | Purpose | Required fields | Optional | Auth | Conf. | Replay protection | Max size | Versioning |
|---|---|---|---|---|---|---|---|---|---|---|
| 0x01 | `HELLO` | both | Capability exchange | `link_versions[]`, `max_frame`, `l2cap_psm?`, `relay_willing(bool)`, `relay_class(u8: always_on/best_effort/endpoint_only, ADR-019)`, `free_capacity_bucket(u8)` | `transport_upgrade_offers[]` | Noise session (anonymous) | Noise | Noise nonces | 64 | Pick the highest common `link_version`; unknown fields ignored (TLV tail) |
| 0x02 | `SUMMARY` | both | Offer carried envelopes + known vaccines | list of `{envelope_id(16), tag(16), expiry(u32), bucket(u8), spray(u8 flag: budget>1)}`; list of `ack_commitment(16)` known-delivered | GCS/Bloom filter variant (v2) | Noise | Noise | Noise | ≤ 8 KiB per message, paged | Per `link_version` |
| 0x03 | `REQUEST` | both | Ask for envelopes (as recipient: matches own tags; as relay: willing to carry) | `envelope_id[]`, `role[]` (DELIVER/CARRY) | — | Noise | Noise | Noise | ≤ 2 KiB | — |
| 0x04 | `ENVELOPE` | both | Transfer one L2 envelope | `envelope` (§3) | — | Noise | Noise | Noise + L2 dedup | 2048 + 62 | L2 version in header |
| 0x05 | `CUSTODY` | both | Hop-level result per envelope | `{envelope_id, status: ACCEPTED/REJECTED_FULL/REJECTED_POLICY/DUPLICATE/DELIVERED_LOCAL}` | `retry_after_s` | Noise | Noise | Noise | ≤ 2 KiB | — |
| 0x06 | `VACCINE` | both | Delivery proofs | `ack_secret(16)[]` with `expiry` | — | Hash commitment (anyone can verify) | Noise | Expiry + dedup | ≤ 2 KiB | — |
| 0x07 | `PING`/`PONG` | both | Connection health | `t_send(u32 ms)` | `rssi(i8)` | Noise | Noise | Noise | 16 | — |
| 0x08 | `BYE` | both | Graceful close | `reason(u8)` | — | Noise | Noise | — | 8 | — |

**Session state machine (per connection):**

`CONNECTED → NOISE_HS → HELLO_EXCHANGED → SUMMARY_EXCHANGED → TRANSFERRING → (VACCINE) → BYE/DISCONNECTED`

- The ~10 s iOS background wake budget [S09] means the order matters. **HELLO, VACCINE, then deliveries to the recipient go first, and carry transfers go last.**

## 3. L2 — Envelope (what relays store)

### 3.1 Header (62 bytes, v1 proposal)

| Off | Size | Field | Mutable by relays? | In AAD? | Notes |
|---|---|---|---|---|---|
| 0 | 1 | `version` = 0x01 | no | yes | Relays carry only versions they understand |
| 1 | 1 | `type` (0x01 DATA, 0x02 RECEIPT) | no | yes | Both types are routed identically (receipts are indistinguishable by size bucket) |
| 2 | 1 | `flags` (bit0: vaccine-enabled) | no | yes | Reserved bits must be 0 |
| 3 | 1 | `hop_count` | **yes** | no | Incremented per transfer |
| 4 | 1 | `hop_limit` | no | yes | Default 8 |
| 5 | 1 | `copy_budget` | **yes** | no | Binary spray-and-wait (Doc 5 §5); relays clamp to ≤ 8 |
| 6 | 2 | `reserved` | — | yes | 0 |
| 8 | 16 | `envelope_id` | no | yes | 128-bit random; dedup key |
| 24 | 16 | `recipient_tag` | no | yes | Doc 6 §4 |
| 40 | 4 | `expiry` (unix minutes) | no | yes | ≤ now + local max |
| 44 | 16 | `ack_commitment` | no | yes | `Trunc16(SHA-256("VL/ack/v1" ‖ ack_secret))`; zero if vaccine disabled |
| 60 | 2 | `body_len` | no | yes | ∈ {256, 512, 1024, 2048} |
| 62 | n | `body` | no | — | `nonce(12) ‖ ChaCha20-Poly1305(K_wrap(dir), inner, AAD=header-immutables) ‖ tag(16)` (ADR-014), zero-padded inside the plaintext to `body_len` |

**Visible to relays:** everything in the header. **Not visible:** sender, recipient identity, timestamps finer than expiry, content, message type beyond DATA/RECEIPT.

There is no sender identity and no fine-grained creation timestamp in clear. The expiry is coarse (minutes), and senders should add random jitter of ±30 min (proposal) so the expiry does not reveal the creation time precisely.

### 3.2 Relay processing rules (normative intent)

1. Reject if: unknown `version`/`type`, `body_len` not in the bucket set, `expiry` ≤ now or > now + local_max, `hop_count ≥ hop_limit`, or `copy_budget` = 0.
2. Clamp `copy_budget` to `min(copy_budget, 8)`.
3. Drop it if `envelope_id` was seen (dedup set retained until expiry) or its `ack_commitment` is in the vaccine set.
4. Apply quotas (Doc 6 §7) → `CUSTODY` status.
5. Never decompress, decrypt or parse the body.

## 4. L3 — Inner messages (end-to-end)

### 4.1 Inner header (inside the wrap)

| Field | Size | Notes |
|---|---|---|
| `inner_version` | 1 | 0x01 |
| `inner_type` | 1 | see §4.2 |
| `message_id` | 16 | Stable across retries/re-encryption; recipient dedup |
| `ack_secret` | 16 | Random per message (DATA only) |
| `sent_at` | 8 | Sender clock (ms), authenticated; display only |
| `payload_len` | 2 | Length before padding |
| `payload` | n | Olm message (DATA) or receipt body (RECEIPT) |

### 4.2 Inner message types

| Type | Name | Layer protection | Purpose | Required fields | Optional | Replay | Max size | Expiry |
|---|---|---|---|---|---|---|---|---|
| 0x10 | `TEXT` | Wrap + Olm | User text | UTF-8 text | `reply_to(message_id)` | Olm + message_id | ≈1.8 KiB plaintext (2 KiB bucket minus overheads; exact figure set in P4) | Envelope expiry |
| 0x20 | `RECEIPT_DELIVERED` | Wrap only (works while locked) | Device received it | `refs[] (message_id)`, `received_at` | — | message_id set | 256 bucket | 72 h |
| 0x21 | `RECEIPT_READ` | Wrap + Olm | Read | `refs[]` | — | same | 256 | 72 h |
| 0x22 | `RECEIPT_UNDECRYPTABLE` | Wrap only | Outer unwrap ok, Olm decrypt failed → sender re-sends with a fresh session | `refs[]`, `reason` | — | message_id set | 256 | 72 h |
| 0x30 | `KEY_UPDATE` | Wrap + Olm, signed by fingerprint key | New OTKs / AK rotation | `otk[]`, `ak_pub?`, `sig` | `valid_from_epoch` | message_id | 1024 | Longer (7 d) |
| 0x31 | `REVOKE` | Wrap + Olm, signed | Identity retirement → new card | `new_card_hash`, `sig` | `reason` | message_id | 1024 | 7 d |
| 0x40 | `PROFILE` | Wrap + Olm | Display name/avatar hash | `display_name` | — | message_id | 512 | 7 d |

## 5. Pairing Card (QR, out-of-band)

| Field | Size | Notes |
|---|---|---|
| `card_version` | 1 | 0x01 |
| `olm_identity_key` | 32 | Curve25519 |
| `olm_fingerprint_key` | 32 | Ed25519 |
| `ak_pub` | 32 | X25519 addressing key |
| `otk_count` + `otk[]` | 1 + 32·n | n = 5 proposal |
| `nonce` | 16 | Freshness |
| `display_name` | ≤ 32 | Optional, UTF-8 |
| `signature` | 64 | Ed25519 over all above |

≈ 420 bytes at n = 5, which fits a binary QR code (sizing to be confirmed with the chosen scanner library, P4).

## 6. Data model (local, conceptual)

### 6.1 Class C store (unlocked only)

| Record | Fields | Notes |
|---|---|---|
| `Identity` | olm_account_pickle (encrypted), created_at, card_hash | One per device in MVP |
| `Contact` | contact_id (local UUID), display_name, olm_identity_key, olm_fingerprint_key, ak_pub, verified(bool, safety number), paired_at, state (active/revoked) | Social graph: Class C |
| `OlmSession` | contact_id, session_pickle, last_used | Multiple per contact allowed |
| `Message` | message_id, contact_id, direction, body, sent_at, state (§7), envelope_id(s), expiry | Plaintext lives here only |

### 6.2 Class R store (after first unlock)

| Record | Fields | What a relay needs? |
|---|---|---|
| `PairKeys` | contact_ref (opaque local ID), K_tag_in, K_tag_out, K_wrap_in, K_wrap_out | No: own recipient/sender role only |
| `TagTable` | tag → (contact_ref, epoch) for active epochs | No |
| `CarriedEnvelope` | envelope bytes, received_at, from_link_session (ephemeral), copy_budget, hop_count, expiry | **Yes**: this is all a relay needs |
| `OutboxEnvelope` | envelope bytes, message_ref, attempts, sprayed_to_count, next_retry_at | Sender only |
| `SeenSet` | envelope_id → expiry | **Yes** (dedup) |
| `VaccineSet` | ack_commitment → expiry (+ ack_secret) | **Yes** |
| `QuotaLedger` | per-session and per-tag counters (short-lived) | **Yes** |
| `PendingInbound` | own envelopes received while locked (ciphertext) | No: decrypt on unlock |
| `EncounterStats` (optional, off in MVP) | daily counts only, no identifiers | Diagnostics only |

**Relays need only:** `CarriedEnvelope`, `SeenSet`, `VaccineSet`, `QuotaLedger`. Nothing about who anyone is.

## 7. Delivery-state definitions (shared with Doc 9)

> **Superseded by [Doc 18 §7.4](18-development-blueprint.md)** (final state model). Most important change: `DELIVERED_DIRECT` is **removed**, because a `DELIVERED_LOCAL` custody claim is unauthenticated (any relay sees tags) and is diagnostic only (ADR-015). The table below is kept for history.

| State | Precise meaning | Shown to user as |
|---|---|---|
| `QUEUED` | Encrypted envelope in the local outbox; no neighbour has accepted a copy | "Waiting for nearby devices" |
| `HANDED_OFF` | ≥ 1 neighbour returned `CUSTODY=ACCEPTED`. **Not delivery.** A relay can still drop it. | "On its way (carried by k devices)" |
| `DELIVERED_DIRECT` | The recipient itself accepted the envelope over a direct link (`DELIVERED_LOCAL` custody status from a tag match) | "Delivered" |
| `DELIVERED` | Wrap-authenticated `RECEIPT_DELIVERED` received from the recipient | "Delivered" |
| `READ` | `RECEIPT_READ` received (if enabled) | "Read" |
| `EXPIRED` | Expiry passed with no delivery evidence | "Not delivered (expired)" |
| `FAILED` | Local error (e.g. keys missing, envelope rejected by policy everywhere) | "Failed — tap for details" |
| `UNKNOWN` | Expired *after* handoff without a receipt: it may have been delivered and the receipt lost | "Delivery unknown" |

**Why handoff ≠ delivery:**
- Custody is a promise by an anonymous stranger who may drop the envelope, run out of storage, never meet the recipient, or be malicious (T2, T13).
- Only an authenticated receipt from the recipient proves delivery.
- Bridgefy-style UX that conflates them misleads users at exactly the moments that matter.

## 8. Interaction of fields

- `envelope_id` (dedup at relays) ≠ `message_id` (dedup at recipient). A re-encrypted retry gets a new `envelope_id` with the same `message_id`.
- `expiry` bounds dedup memory, vaccine memory, skipped-key retention, and tag-table windows. **One clock, many uses**, so every store can be garbage-collected by expiry.
- `hop_limit` is a safety net. `copy_budget` is the real spread control.
- `ack_commitment` binds the vaccine to one envelope family (same `message_id` across retries → reuse the same `ack_secret` so one vaccine clears all copies).
- Clock skew: relays reject `expiry > now + local_max`. Devices without a network-synced clock may drift, so local_max should include a skew margin (e.g. +2 h; to be measured).

## 9. Versioning, compatibility and migration

| Layer | Strategy |
|---|---|
| L0 | New UUID only for incompatible changes (avoid: it splits the network) |
| L1 | `HELLO` negotiates `link_version`; messages use TLV tails, so unknown fields are ignored; unknown message types get `BYE(unsupported)` only if mandatory |
| L2 | `version` byte; relays carry only known versions. **Rule: a new envelope version must be carried by ≥ N releases before senders emit it** (staged rollout), because relays are the slowest to update |
| L3 | `inner_version`; capabilities learned from contacts' `PROFILE`/`KEY_UPDATE`; send the highest mutually supported version |
| Local DB | Versioned schema with forward-only migrations; Class R can be rebuilt (dropping carried mail is acceptable), Class C migrations must be loss-free and tested on real data |
| Pairing card | `card_version`; old cards accepted for ≥ 1 year |
