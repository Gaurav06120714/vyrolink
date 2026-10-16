# 10 — Attachment Transfer Design (F-14, small images)

**Status:** DESIGN. Not implemented. Approval is needed for the wire additions, which would be a new ADR before any code.

## 1. Requirements and constraints

- **Image size:** ≤ 16 KiB after on-device re-encoding.
  - Briar caps images at 32 KiB (VERIFIED, `MediaConstants`).
  - bitchat's courier excludes media (VERIFIED).
  - The BLE goodput is small and unmeasured (06).
- **Integrity:** every chunk must be authenticated and attributable to the sender, with **no new cryptography**.
- **Resumability:** a recipient may get chunks over several encounters, via different relays, in any order.
- **Privacy:** relays must not be able to tell an attachment from a text, so chunks are ordinary padded envelopes.

## 2. Options

| Option | Integrity | Resume | Dedup | Network cost | Verdict |
|---|---|---|---|---|---|
| One big envelope | Wrap + Olm | No | No | Exceeds the 2 KiB bucket | ✗ |
| **k chunks, each a CONTENT event inside Olm; a manifest lists SHA-256 per chunk** | Olm per chunk, plus the hash list in the manifest | Missing-chunk set | By content hash | k envelopes × spray copies | **Proposed** |
| Content-addressed, unencrypted chunk store (BitTorrent-style) | Hash only | Yes | Global | Lower | ✗: relays could learn content popularity and fingerprint images |
| Fountain codes (RaptorQ, RFC 6330) over chunks | Per symbol | Any k of n | — | ≈ k(1+ε) | Later: only if the simulator shows multi-path loss dominates |

## 3. Proposed content types (sketch)

- `ATTACHMENT_MANIFEST`: `attachment_id 16 ‖ mime u8 ‖ total_len u32 ‖ k u8 (≤ 12) ‖ k × SHA-256(chunk) ‖ caption`
- `ATTACHMENT_CHUNK`: `attachment_id 16 ‖ index u8 ‖ bytes (≤ 1,500)`

**Receiver behaviour:**
- Store chunks keyed by `(attachment_id, index)` in Class C (encrypted).
- Verify each chunk against the manifest hash.
- Assemble when complete.
- Bounds: per-contact quota (e.g. 256 KiB of partial attachments) and the orphan TTL.

**Cost model:** a 16 KiB image is about 11 envelopes of 2 KiB each. With L = 8 that is up to 88 relay copies, so attachments should use a **lower copy budget** (e.g. L = 2) and rely on direct delivery where possible. Simulate before choosing.

## 4. Tests required (from 08 acceptance criteria style)

- Out-of-order chunks.
- A chunk before its manifest.
- A tampered chunk is rejected by its hash.
- A duplicate chunk is idempotent.
- The quota is enforced.
- A partial attachment expires.
- A plaintext canary is not on disk.
- Fuzz the new decoders.
