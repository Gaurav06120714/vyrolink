# Google Messages (RCS) — research report

**Accessed:** 2026-10-15. **Confidence:** Medium–high.

| Item | Finding | Label |
|---|---|---|
| Android↔Android E2EE | Google's own Signal-protocol-based RCS E2EE (earlier) | REPORTED |
| Cross-platform E2EE | **MLS** (GSMA Universal Profile 3.0); beta rollout from 2026-05-11 for iOS 26.5 + Google Messages with supported carriers; on by default where supported; lock icon | VERIFIED (blog.google); REPORTED (details) |
| Business (A2P) messaging | Not E2EE | REPORTED |
| Features | Reactions, replies, typing indicators, read receipts, high-resolution media, edit/unsend | REPORTED |

**Lesson:** MLS fits when a **delivery service orders commits**. That assumption fails offline (see `05-technology-algorithm-evaluation.md` §6).

## Sources (accessed 2026-10-15)

- https://blog.google/products-and-platforms/platforms/android/android-ios-end-to-end-encrypted-rcs-messaging (high)
- https://www.androidauthority.com/apple-ios-26-rcs-end-to-end-encryption-mls-protocol-3588258 (medium)
- https://messageflow.com/blog/rcs-end-to-end-encryption/ (low; vendor blog)
