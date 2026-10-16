# Session — research report

**Accessed:** 2026-10-15. **Confidence:** Medium (Privacy Guides news; community sources are lower).

| Item | Finding | Label |
|---|---|---|
| Identity | Account ID is a public key; no phone number | REPORTED |
| Routing | Onion requests over the Oxen service-node network (swarms store messages) | REPORTED |
| Forward secrecy | **Removed in 2021**, citing multi-device key-sync instability. "Session Protocol V2" (announced 2025-12-03) restores PFS and adds post-quantum encryption, using a shared `libsession` plus "config messages" for device sync; the V2 spec was not finalised (details promised for 2026) | REPORTED (privacyguides.org) |
| Deniability | Dropped in favour of long-term keys and signatures | REPORTED (community wiki; lower confidence) |

**Lesson:** this is a warning that **multi-device key synchronisation is hard enough that a privacy app gave up forward secrecy for it**. VYRO LINK keeps **single-device** identity for the MVP (deferred: multi-device), rather than weakening Olm.

## Sources (accessed 2026-10-15)

- https://www.privacyguides.org/news/2025/12/03/session-messenger-adds-pfs-pqe-and-other-improvements/ (medium–high)
- https://git.hackliberty.org/Git-Mirrors/thgtoa/commit/12b99c9ea9a1d5c2013affbc12a45a32329715b0 (low)
