# Document 21 — Wire Protocol v1 (normative)

**Status:** implemented in `core/crates/vl-proto` and `core/crates/vl-crypto` and pinned by golden vectors in `core/vectors/proto-v1.txt`.

- Where this document and Doc 07 differ, **this document and the code win**. Doc 07 is the earlier design.
- Any change to the bytes below must change the vectors deliberately (`VL_REGEN_VECTORS=1`).
- All integers are big-endian.

## 1. Layers

| Layer | Carried over | Content |
|---|---|---|
| L0 | BLE advertisement | One fixed 128-bit service UUID. No other data. |
| L1 framing | GATT write/notify chunks or an L2CAP stream | `len(u16, 1..=65535) ‖ payload`. The receiver buffers at most one frame. Zero or oversized length → drop the link. |
| L1 security | Inside frames | `Noise_NN_25519_ChaChaPoly_SHA256`, prologue `"VL/link/v1"`, empty handshake payloads; initiator = the device that opened the connection. |
| L1 messages | Noise transport plaintexts | §3 |
| L2 envelope | `ENVELOPE` link message | §2 |
| L3 inner | Envelope body plaintext | §4 |

**Size limits:**
- Largest link-message plaintext: **6,917 bytes** (a full SUMMARY).
- Largest frame on the wire: 6,933 (plus the 16-byte Noise tag).
- Largest envelope: 62 + 2,048 = **2,110 bytes**.

## 2. Envelope (L2)

| Offset | Size | Field | Rules |
|---|---|---|---|
| 0 | 1 | version | `0x01`; any other value → reject |
| 1 | 1 | kind | `0x01` DATA, `0x02` RECEIPT |
| 2 | 1 | flags | bit0 = vaccine enabled; other bits must be 0 |
| 3 | 1 | hop_count | **relay-mutable**; +1 on each relay accept |
| 4 | 1 | hop_limit | default 8 |
| 5 | 1 | copy_budget | **relay-mutable**; relays clamp to ≤ 8 |
| 6 | 2 | reserved | must be 0 |
| 8 | 16 | envelope_id | random |
| 24 | 16 | recipient_tag | §5 |
| 40 | 4 | expiry_min | unix minutes; sender adds ±30 min jitter |
| 44 | 16 | ack_commitment | `Trunc16(SHA-256("VL/ack/v1" ‖ ack_secret))`, zeros if no vaccine |
| 60 | 2 | body_len | ∈ {256, 512, 1024, 2048} |
| 62 | body_len | body | `nonce(12) ‖ ChaCha20-Poly1305(K_wrap, padded_inner, AAD) ‖ tag(16)` |

**AAD** = the 62-byte header with offsets 3 and 5 set to zero. Relays can update the mutable fields; any other header change makes decryption fail at the recipient.

**Relay admission**, applied in this order:
1. Decode succeeds.
2. `expiry > now`.
3. `expiry ≤ now + 7 d + 2 h skew`.
4. `hop_count < hop_limit`.
5. `copy_budget > 0`.
6. Not already seen or held, and no stored vaccine for its commitment.
7. **The relay requested it** (unsolicited envelopes are refused).
8. Within quotas: ≤ 32 envelopes and ≤ 64 KiB per session, ≤ 8 per recipient tag, ≤ 10 MiB carried in total.

## 3. Link messages (L1)

The first byte is the type. Every list starts with a `u16` count, which is ≤ 128 and checked **before** any allocation. Trailing bytes → reject.

| Type | Name | Body |
|---|---|---|
| 0x01 | HELLO | `n(u8, 1..=8) ‖ link_versions[n] ‖ max_frame(u16) ‖ relay_class(u8: 0 always-on, 1 best-effort, 2 endpoint-only) ‖ flags(u8: bit0 relay-willing, bit1 L2CAP offered) ‖ free_capacity_bucket(u8) ‖ l2cap_psm(u16)`, followed by optional TLVs (`type u8 ‖ len u8 ‖ value`, ≤ 256 bytes total) that receivers ignore |
| 0x02 | SUMMARY | `count ‖ {envelope_id 16, recipient_tag 16, expiry u32, bucket_index u8 (0..3), spray u8 (0/1)}* ‖ count ‖ {commitment 16}*`. The second list is reserved and sent empty: commitments without secrets are never trusted to purge anything. |
| 0x03 | REQUEST | `count ‖ {envelope_id 16, role u8: 0 deliver, 1 carry}*` |
| 0x04 | ENVELOPE | one complete envelope (§2) |
| 0x05 | CUSTODY | `count ‖ {envelope_id 16, status u8: 0 accepted, 1 rejected-full, 2 rejected-policy, 3 duplicate, 4 delivered-local}*` |
| 0x06 | VACCINE | `count ‖ {ack_secret 16, expiry u32}*` |
| 0x07 / 0x08 | PING / PONG | `t(u32)` |
| 0x09 | BYE | `reason(u8: 1 incompatible, 2 storage failure)` |

**Session order:**
1. Both sides send HELLO.
2. On receiving HELLO: send VACCINE (if any), then SUMMARY.
3. On SUMMARY: REQUEST deliver-role entries whose tag is ours, then carry-role entries within quotas.
4. On REQUEST: send ENVELOPE.
   - **Deliver** sends the envelope unchanged, and the sender keeps its copy.
   - **Carry** sends ⌊n/2⌋ of the copy budget and keeps ⌈n/2⌉, applied on CUSTODY accepted.
5. Every ENVELOPE gets a CUSTODY reply.
6. If local state changes mid-session (e.g. a receipt was just created), send VACCINE and SUMMARY again.

**`DELIVERED_LOCAL` custody is diagnostic only.** It never sets "Delivered" (ADR-015).

**Vaccines are stored only if they match an envelope the receiver holds.**

## 4. Inner message (L3)

`version(1)=0x01 ‖ kind(1) ‖ message_id(16) ‖ ack_secret(16) ‖ sent_at_ms(u64) ‖ payload_len(u16) ‖ payload ‖ zero padding to body_len − 28`. Non-zero padding → reject.

| Kind | Payload | Protection |
|---|---|---|
| 0x10 TEXT | `olm_type(u8: 0 pre-key, 1 normal) ‖ olm_message` (Olm v1, vodozemac); the Olm plaintext is UTF-8 text. **Legacy:** still decoded; new clients send CONTENT | wrap + Olm |
| 0x11 CONTENT | Same Olm payload framing as TEXT; the Olm plaintext is a **content event** (§4.1). Added 2026-10-15 (ADR-023) | wrap + Olm |
| 0x20 RECEIPT_DELIVERED | `n(u8, 1..=16) ‖ message_id[n] ‖ at_ms(u64)` | wrap only (works while locked) |
| 0x21 RECEIPT_READ | as above, inside Olm | wrap + Olm (feature off) |
| 0x22 RECEIPT_UNDECRYPTABLE | as 0x20 | wrap only |
| 0x30 / 0x31 / 0x40 | KEY_UPDATE / REVOKE / PROFILE | reserved; **not implemented** |

**Text limit:** 1,700 UTF-8 bytes (MVP). Measured: the largest pre-key text that fits the 2,048 bucket is 1,804 bytes.

### 4.1 Content events (inner kind 0x11, inside Olm)

`content_version(u8) = 0x01 ‖ ctype(u8) ‖ body`. Trailing bytes → reject. All integers big-endian.

| ctype | Name | Body | Rules |
|---|---|---|---|
| 0x01 | TEXT | `flags(u8: bit0 reply, bit1 forwarded; other bits 0) ‖ [reply_to 16, iff bit0] ‖ timer_secs(u32) ‖ text_len(u16) ‖ UTF-8 text` | `timer_secs` is 0 or 60 ..= 31,536,000; text ≤ 1,700 bytes (core limit); a reply carries **only** the target id |
| 0x02 | REACTION | `target 16 ‖ reaction(u8)` | 0 = remove my reaction; 1..8 = 👍 👎 😀 😂 😢 ❤️ 🚀 ✅; others → reject |
| 0x03 | EDIT | `target 16 ‖ seq(u8, 1..=10) ‖ text_len(u16) ‖ UTF-8 text (non-empty)` | — |
| 0x04 | DELETE | `target 16` | Delete-for-everyone request |
| 0x05 | TIMER | `timer_secs(u32)` | Conversation disappearing timer; same range as TEXT |

**Receiver semantics.** The event author is the authenticated contact (pairwise wrap + Olm session). `sent_at_ms` is the inner header's sender clock. Every rule is **order-independent**, so any arrival order gives the same final state:

| Event | Applies when | Merge rule |
|---|---|---|
| REACTION | The target is in the same conversation (either author) | Per (target, author), the greatest `(sent_at_ms, event message_id)` wins; 0 stays as a removal marker |
| EDIT | The target was sent **by the editor** and `0 ≤ edit.sent_at − target.sent_at ≤ 24 h` | Per (target, author), the greatest `(seq, sent_at_ms)` wins; no effect on a deleted target |
| DELETE | The target was sent **by the deleter** and `0 ≤ delete.sent_at − target.sent_at ≤ 72 h` | Idempotent tombstone. If the target arrives later, it is stored as a deleted placeholder (empty body) |
| TIMER, and TEXT `timer_secs` | — | The conversation timer follows the greatest `sent_at_ms` that carried a value (last-writer-wins) |

**Bounds and compatibility:**
- **Orphans:** events whose target has not arrived are kept for at most 7 d + 2 h, and at most 512 per contact.
- **Unknown ctype:** still acknowledged with RECEIPT_DELIVERED (decryption succeeded), but not displayed.
- **Malformed content:** RECEIPT_UNDECRYPTABLE.
- **Vectors:** `content_*` lines in `core/vectors/proto-v1.txt`.

**Timestamps (ADR-025):** the inner header's `sent_at_ms` is the sender's **hybrid logical clock**:
- the value is `max(now, last + 1)`;
- the clock absorbs received values up to 1 h ahead of physical time.

All LWW comparisons above use it.

### 4.2 Group content (ADR-024; added 2026-10-15)

| ctype | Name | Body | Rules |
|---|---|---|---|
| 0x06 | GROUP_STATE | `group_id 16 ‖ epoch u32 ‖ name_len u8 (≤ 32) ‖ name ‖ n u8 (1..=8) ‖ n × (fingerprint 32 ‖ name_len u8 (≤ 32) ‖ name)` | Duplicate fingerprints → reject. Accepted only from the group's creator, and only with an epoch strictly greater than the one held. A new group is accepted only if both the sender and I are in the list |
| 0x07 | GROUP_TEXT | `group_id 16 ‖ group_msg_id 16 ‖ epoch u32 ‖ flags u8 (bit0 reply) ‖ [reply_to 16] ‖ text_len u16 ‖ UTF-8 text` | Accepted if the author is a member in my current roster and `epoch ≤` mine. Otherwise held (≤ 64 per contact, TTL 7 d + 2 h) while the group is unknown or older; dropped if the author is not a member |
| 0x08 | GROUP_LEAVE | `group_id 16` | Sent to the creator; the creator issues `epoch + 1` without the leaver |

**Fingerprints** are the Ed25519 `olm_fingerprint_key` from pairing cards.

**Fan-out:** each member I am paired with receives an ordinary pairwise CONTENT envelope, so relays cannot distinguish group traffic.

**Vectors:** `content_group_*`.

### 4.3 Attachments (ADR-026; added 2026-10-16)

| Type | Name | Body | Rules |
|---|---|---|---|
| ctype 0x09 (inside Olm) | ATTACHMENT | `attachment_id 16 ‖ kind u8 (1 image, 2 file) ‖ total_len u32 ‖ sha256 32 ‖ key 32 ‖ flags u8 (bit0 reply) ‖ [reply_to 16] ‖ name_len u8 (≤ 64) ‖ name ‖ caption_len u16 (≤ 1,024) ‖ caption` | `total_len` is 1..=16,384 for images and 1..=65,536 for files. It is shown as a message whose text is the caption. If the target was deleted first (tombstone), no attachment is stored |
| inner kind 0x12 (not Olm) | ATTACHMENT_CHUNK | `attachment_id 16 ‖ index u16 ‖ len u16 ‖ ChaCha20-Poly1305(key, nonce = 0¹⁰ ‖ index_be16, aad = "VL/att/v1" ‖ attachment_id ‖ index_be16, chunk)` | Chunk plaintext ≤ 1,400 bytes (the last one may be shorter), so `len` is 17..=1,416. `index < 47`. Lengths are checked before allocation. A malformed payload produces `RECEIPT_UNDECRYPTABLE`; a valid one produces `RECEIPT_DELIVERED` and its vaccine |

**Receiver rules:**
- Sealed chunks are stored per contact, deduplicated, and capped at 256 KiB of partial bytes per contact. They expire after 7 d + 2 h.
- When the announcement and all `ceil(total_len / 1400)` chunks are present, each chunk is opened in index order. The whole must be exactly `total_len` bytes with the announced SHA-256; otherwise the chunks are discarded.
- Deleting the message (for me, or by the sender's delete) removes the attachment bytes.

**Sender:** "Delivered" for the message means the announcement's receipt. The UI says how many chunks have their own receipts until all do.

**Older versions:** ctype 0x09 is acknowledged and ignored (AC-W.2). Inner kind 0x12 fails to decode and is dropped without a receipt.

**Vectors:** `content_attachment_image`, `attachment_chunk_payload`, reject `chunk_payload_too_long`.

## 5. Keys, tags, pairing

**Pairing card:**
- Layout: `version 0x01 ‖ olm_identity_key 32 ‖ olm_fingerprint_key 32 ‖ addressing_key 32 ‖ n(u8, 1..=8) ‖ one_time_keys[n]×32 ‖ nonce 16 ‖ name_len(u8 ≤ 32) ‖ UTF-8 name ‖ Ed25519 signature 64` (the signature covers every preceding byte).
- Size: 383 bytes for 5 keys and a 4-character name.

**Pairing code (QR text, added 2026-10-15):**
- `"VL1:" ‖ Base45(card)`, Base45 as in RFC 9285. The alphabet is exactly the QR alphanumeric set, so a 383-byte card becomes 579 characters and is encoded in alphanumeric mode (EC level M).
- Decoders reject: input longer than 4,096 characters (checked before allocating), a missing `VL1:` prefix, characters outside the alphabet, a 3-character group above 65,535, a 2-character group above 255, and a dangling single character. Encoding the decoded bytes must reproduce the input (fuzzed in `decode_all`).
- A scanned code equal to this device's own identity is refused (`OwnPairingCode`).
- Implementation: `vl-proto/src/pairing_code.rs` (RFC 9285 examples as tests); `Node::pairing_code` / `complete_pairing_code`.

**Pairwise derivation:**
1. `K = HKDF-Extract(salt = SHA-256(min(Ha,Hb) ‖ max(Ha,Hb)), ikm = X25519(ak_self, ak_peer))`, where `H = SHA-256(full card bytes)`.
2. Directional keys are `HKDF-Expand(K, label ‖ H_sender ‖ H_recipient, 32)`, with labels `"VL/v1/tag"` and `"VL/v1/wrap"`.
3. Reject if the X25519 result is non-contributory or the two card hashes are equal.

**Tags:**
- `tag = Trunc16(HMAC-SHA256(K_tag, "VL/tag/v1" ‖ epoch_be32))`, with `epoch = floor(unix_seconds / 21600)` (6 h).
- Recipients precompute tags for every epoch in `[now − 7 d − 2 h, now + 6 h]`.

**ID code (ADR-027):** `Crockford-Base32(Trunc20(SHA-256("VL/id/v1" ‖ olm_fingerprint_key)))`: 32 characters from `0123456789ABCDEFGHJKMNPQRSTVWXYZ`. It is shown on the "My ID" screen (with a QR of the code) and on each contact. It is never transmitted.

**Re-pairing (ADR-028):** a scanned card whose `olm_fingerprint_key` matches an existing contact replaces that contact's card, pairwise keys, tags and one-time keys, and deletes its Olm sessions. It never creates a second contact.

**Safety number:** `SHA-512("VL/safety/v1" ‖ min(fpA,fpB) ‖ max(fpA,fpB))`. The first 60 bytes give twelve 5-byte groups, each rendered as `value mod 100000`.

## 6. Delivery evidence (sender side)

A sender marks a message **Delivered** only on either:
- **(a)** a RECEIPT_DELIVERED whose wrap verifies under `K_wrap(recipient→sender)` and whose refs include the message id; or
- **(b)** a vaccine whose secret hashes to the message's `ack_commitment` (ADR-020).

Both prove the recipient opened the outer wrap, because only the sender and the recipient can know those secrets. Relay custody never counts as delivery.

## 7. Bluetooth GATT binding (L0/L1, added 2026-10-15)

| Item | Value |
|---|---|
| Advertised service UUID (the only advertised data; no name, no identifiers) | `3B29F840-0F6B-4221-9AEB-88FF70CCF5CD` |
| Link characteristic | `60EAE795-7B2A-49EE-B8C2-7430FF5FF590`, properties write-without-response + notify |
| Direction | The connecting device (GATT client) is the Noise initiator and **writes**; the GATT server **notifies** |
| Byte stream | The L1 frame stream is split into chunks of at most `ATT_MTU − 3` (Android, after requesting 517) or `maximumWriteValueLength(.withoutResponse)` / `maximumUpdateValueLength` (iOS). The receiver passes chunks to `link_receive` in order; Rust's `Deframer` reassembles frames |
| Flow control | iOS: `canSendWriteWithoutResponse` / `peripheralIsReady`, and `updateValue` returning false / `peripheralManagerIsReady`. Android: one write or notification in flight, the next after `onCharacteristicWrite` / `onNotificationSent` |
| Session end | A session ends after 4 s with nothing to send either way, after 60 s in total, when Rust closes the link, or on disconnect. A peer is not reconnected for 45 s unless local mail was added |

Implemented in `app/packages/vl_host/ios/.../BleRelay.swift` and `android/.../BleRelay.kt`. **Not validated between physical devices** (Doc 14 Experiments A–C remain blocked).
