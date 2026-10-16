# Briar — research report

**Accessed:** 2026-10-15. **Source code inspected:** `briar/briar` (local clone, `d03d6fc`, 2026-10-09, GPL-3.0; studied only). Doc 03 already covers Briar's transports.

**Overall confidence:** High for the constants and APIs cited.

## 1. Why it matters

Briar is the most mature open-source offline messenger, running over Bluetooth, Wi-Fi and Tor. Its **Bramble** sync layer and **auto-delete** design solve the same "messages arrive late, out of order, via different paths" problem VYRO LINK has. Briar does **not** do multi-hop store-and-forward through strangers: it syncs only between contacts.

## 2. Capabilities

| Capability | Status | Label | Where |
|---|---|---|---|
| 1:1 private messages, image attachments | Yes | VERIFIED | `briar-core/.../messaging`, `attachment` |
| Max sync message body | **32 KiB**; image ≤ body − 100 B | VERIFIED | `SyncConstants.MAX_MESSAGE_BODY_LENGTH`, `MediaConstants.MAX_IMAGE_SIZE` |
| Disappearing messages ("auto-delete") | Yes. Timer 1 min – 365 days, **per contact**, carried in each message; a received timer is adopted **unless the local one changed more recently** (last-writer-wins by message timestamp) | VERIFIED | `AutoDeleteConstants.java`, `AutoDeleteManager.java` |
| Cleanup timer start | DB `startCleanupTimer(txn, messageId)` hook | VERIFIED | `bramble-api/.../cleanup/CleanupManager.java` |
| Private groups, forums, blogs, RSS | Yes | VERIFIED (modules) | `privategroup`, `forum`, `blog`, `feed` |
| Introductions (introduce two contacts) | Yes | VERIFIED | `introduction` module |
| Contact add | In person (QR, BQP key agreement) or remote via link | VERIFIED (module `keyagreement`); REPORTED (remote) | — |
| Briar Mailbox (always-on buffer reached over Tor) | Yes; extended transports listed as future | VERIFIED (README) | `briar-mailbox/README.md` |
| Reactions / edits / message deletion for everyone | Not found | UNKNOWN (bounded search: no such client module) | — |
| iOS app | None | REPORTED (long-standing) | — |

## 3. Architecture

- Bramble is a transport-agnostic sync layer of offers, requests and acks: per-contact, per-group message synchronisation with a database as the source of truth (VERIFIED, module layout under `bramble-core/.../sync`).
- Transports are plugins: Bluetooth, LAN/Wi-Fi, Tor, and removable drives (VERIFIED, `plugin`).

## 4. Security

- Transport key agreement is BQP; transport encryption uses the "BTP" stream protocol.
- Audited by Cure53 in 2017 (REPORTED, Doc 03/12).

## 5. Lessons

**Adopt:**
- **auto-delete semantics**: the timer travels with each message, uses last-writer-wins, and is bounded between 1 minute and 1 year;
- the "introductions" concept (Tier 3);
- a hard body cap with media sized to fit.

**Avoid:** Tor/internet transports (ADR-012); unbounded forum and blog sync.

## 6. Sources (accessed 2026-10-15)
- Source: `references/briar-briar` @ d03d6fc. Files: `bramble-api/.../sync/SyncConstants.java`, `briar-api/.../autodelete/AutoDeleteConstants.java`, `AutoDeleteManager.java`, `briar-api/.../attachment/MediaConstants.java`. High confidence.
- Briar Mailbox README: https://code.briarproject.org/briar/briar-mailbox (medium; undated cache).
- Doc 03 / Doc 12 (earlier evidence, Cure53 2017 audit).
