# Signal — research report

**Accessed:** 2026-10-15. **Overall confidence:** High for protocol facts (official blog, support pages, peer-reviewed papers).

## 1. Why it matters

Signal is the reference for E2EE messaging UX and cryptography. Our Olm layer (vodozemac) is a Double-Ratchet relative.

## 2. Capabilities

| Capability | Behaviour | Label | Source |
|---|---|---|---|
| Edit | Up to **10 edits within 24 h**; marked "Edited"; recipients can view edit history; Note to Self is editable forever | VERIFIED | support.signal.org "Edit Message" |
| Delete for everyone | 24 h per the official poll page; one third-party source says 3 h (likely outdated) | REPORTED (conflicting) | support.signal.org Polls; telecomtalk.info |
| Polls | Up to 10 options, optional multi-vote, the creator can end the poll, no editing after send; group launch Nov 2025 | VERIFIED (support page); REPORTED (date) | support.signal.org; alternativeto.net |
| Pinned messages | Up to 3 per chat; a 4th replaces the oldest; duration 24 h / 7 d / 30 d / forever (Jan 2026) | REPORTED | alternativeto.net news 2026-01 |
| Secure backups | Opt-in encrypted backups, rolled out 2025 (Android first, then iOS) | REPORTED / VERIFIED (Signal blog listing) | signal.org/blog |
| Disappearing messages | Per-chat timer, with a default timer for new chats | REPORTED (TechCrunch 2021) | — |
| Post-quantum | **PQXDH** (2023, initial key agreement) + **SPQR / Triple Ratchet** (2025-10-02; ML-KEM-768, chunked with erasure codes; formally analysed with Cryspen) | VERIFIED | signal.org/blog/spqr |
| Groups | Sender Keys (widely reported; early Signal used pairwise fan-out) | REPORTED | Wikipedia "Sender Keys"; EPFL analysis |
| Reactions, replies, voice notes, stickers, stories, usernames | Yes | REPORTED (widely documented, not re-fetched) | — |

## 3. Lessons

**Adopt:**
- a bounded edit **count** and **window** (number of edits, time limit), with edit history visible to recipients so edits are honest;
- pins with expiry;
- polls as a later tier.

**Note:** Signal's **PQ upgrade path** (PQXDH then SPQR) is what Olm/vodozemac does not have. VYRO LINK's long-lived offline envelopes are "harvest-now-decrypt-later" targets. This goes on the risk register as a deferred item (no custom crypto; adopt a vetted PQ hybrid when a reviewed Rust implementation fits).

## 4. Sources (accessed 2026-10-15)
- https://support.signal.org/hc/articles/6255134251546-Edit-Message (high).
- https://support.signal.org/hc/en-us/articles/9971667844506-Signal-Polls (high).
- https://signal.org/blog/spqr/ (high). PQXDH: https://eprint.iacr.org/2024/741 (high).
- https://alternativeto.net/news/2026/1/signal-now-lets-users-pin-messages-in-one-on-one-and-group-chats-with-time-limits (medium).
- https://telecomtalk.info/how-to-delete-messages-signal/343178/ (low; conflicts with the official page).
- https://en.wikipedia.org/wiki/Sender_Keys ; https://infoscience.epfl.ch/record/298766 (medium/high).
