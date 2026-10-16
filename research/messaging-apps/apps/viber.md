# Viber — research report

**Accessed:** 2026-10-15. **Confidence:** Low–medium (vendor claims; no independent audit found).

| Item | Finding | Label |
|---|---|---|
| E2EE | Default for 1:1 and groups; color-coded lock; keys stay on device | REPORTED (Viber security page; vendor claim) |
| Offline recipient | The server holds encrypted undelivered messages "for a limited time" | REPORTED (vendor) |
| Disappearing messages | Self-destruct timer; screenshot **notification** (not prevention); third-party pages cite 1 min – 5 h (unverified) | REPORTED |
| Hidden chats | PIN-protected chat list entries | REPORTED |

**Lesson:** "hidden chats" (a local, PIN-gated conversation list) is a cheap local privacy control. Tier 3: VYRO LINK already uses OS lock plus Class C storage.

## Sources (accessed 2026-10-15)

- https://www.viber.com/security/ (vendor; medium for the claims, low for verification)
- https://www.iphoneincanada.ca/news/viber-encryption-hidden-chats (low–medium)
