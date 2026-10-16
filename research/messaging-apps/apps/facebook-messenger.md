# Facebook Messenger — research report

**Accessed:** 2026-10-15. **Confidence:** Medium–high (Meta engineering posts).

| Item | Finding | Label |
|---|---|---|
| Default E2EE | Rolled out from December 2023, based on the Signal Protocol | VERIFIED (about.fb.com; engineering.fb.com 2023-12-06) |
| Labyrinth | Encrypted server-side history storage: ciphertexts are loaded on demand for multi-device; revoked devices cannot decrypt new data | VERIFIED (engineering.fb.com; whitepaper referenced) |
| Group E2EE | In testing at the December 2023 launch; current state UNKNOWN | REPORTED |
| Features | Reactions, replies, edit (15 min reported), unsend, vanish mode | REPORTED |

**Lesson:** encrypted history sync needs a server mailbox. That is out of scope for offline-first. The equivalent for VYRO LINK is a **local encrypted export/backup** (Tier 3, with a user-held key).

## Sources (accessed 2026-10-15)

- https://engineering.fb.com/2023/12/06/security/building-end-to-end-security-for-messenger/ (high)
- https://about.fb.com/news/2023/12/default-end-to-end-encryption-on-messenger/ (high)
- https://thehackernews.com/2023/12/meta-launches-default-end-to-end.html (medium)
