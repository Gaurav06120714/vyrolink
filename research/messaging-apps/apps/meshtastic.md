# Meshtastic — research report

**Accessed:** 2026-10-15. **Source code inspected:** `meshtastic/protobufs` @ `28f4f5f` (2026-10-12) and `meshtastic/firmware` @ `f7f66b4` (2026-10-14), both GPL-3.0, studied only. Doc 03 covers the radio side.

**Overall confidence:** High for the protobuf fields; medium for routing (docs and release notes).

## 1. Why it matters

Meshtastic is an off-grid LoRa mesh with phone apps over BLE. It is very bandwidth-constrained, so its feature encodings are deliberately tiny.

## 2. Capabilities

| Capability | Behaviour | Label |
|---|---|---|
| Replies | `Data.reply_id` (fixed32): the ID of the message replied to | VERIFIED `mesh.proto` |
| Reactions ("tapbacks") | `Data.emoji` flag: the payload is an emoji, and `reply_id` names the target | VERIFIED `mesh.proto` |
| Public-key DMs | X25519 + AES-CCM since 2.5; **trust-on-first-use** of a node's announced key | REPORTED (meshtastic.org blog/docs) |
| Downgrade bug | CVE-2025-53627: a missing `pki_encrypted` flag fell back to channel encryption silently | REPORTED (NVD) |
| Routing | Managed flooding (default); next-hop routing for DMs in 2.6 (preview) | REPORTED |
| Store & Forward module | A router node stores channel history for clients | REPORTED (Doc 03) |

## 3. Lessons

**Adopt:** replies and reactions as **small references to a message ID** rather than copies.

**Avoid:**
- trust-on-first-use without verification (VYRO LINK pairs in person);
- silent crypto downgrade. A decoder must never fall back. Our AEAD has no unencrypted path; keep it that way.

## 4. Sources (accessed 2026-10-15)
- Source: `references/meshtastic-protobufs` @ 28f4f5f, `meshtastic/mesh.proto` (fields `reply_id`, `emoji`). High confidence.
- https://meshtastic.org/blog/introducing-new-public-key-cryptography-in-v2_5/ (high).
- https://meshtastic.org/docs/overview/encryption (high).
- https://nvd.nist.gov/vuln/detail/CVE-2025-53627 (high).
