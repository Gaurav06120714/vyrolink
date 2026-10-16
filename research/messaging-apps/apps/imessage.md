# Apple iMessage / Messages — research report

**Accessed:** 2026-10-15. **Method:** public Apple material and reporting.
- Messages.app is present on this Mac, but **it was not reverse-engineered**: Apple's licence prohibits it, and the task requires lawful methods.

**Confidence:** Medium–high.

| Item | Finding | Label |
|---|---|---|
| PQ3 | Post-quantum key establishment **and** ongoing rekeying ("Level 3"); classical algorithms kept for authentication; Tamarin analysis (ETH) and Stebila review | REPORTED (Apple Security blog quoted by Macworld/AppleInsider; Apple's own page not opened) |
| Contact Key Verification | Optional; detects server-side key substitution | REPORTED |
| Edit / unsend | Edit within **15 min**, up to **5 edits**, history visible. Unsend within **2 min** (cut from 15 in iOS 16 beta 4); not after read or for old clients | REPORTED (9to5Mac, AppleInsider, Lowyat) |
| Messages via satellite (iOS 18+) | iMessage and SMS with no cellular or Wi-Fi coverage; iPhone 14+; US/CA/MX/JP; E2EE kept with a custom compact protocol; tapbacks supported | VERIFIED (support.apple.com/120930); REPORTED (protocol detail) |
| Cross-platform RCS E2EE | MLS-based, beta from 2026-05-11 (iOS 26.5+ with carriers) | REPORTED (blog.google) |

**Lessons:**
- Satellite messaging shows that **small packets plus reduced features** (text, emoji, tapbacks only) is the right shape for constrained links. That matches our bucketed envelopes.
- The 2-minute unsend window shows how hard *retraction* is: once delivered, it is advisory only. VYRO LINK must word delete as "**request** deletion".

## Sources (accessed 2026-10-15)

- https://support.apple.com/120930 (high)
- https://www.macworld.com/article/2243580/imessage-security-protocol-post-quantum-cryptography-level-3.html (medium)
- https://appleinsider.com/articles/22/06/06/new-imessage-edit-and-unsend-features-have-15-minute-time-limit (medium)
- https://9to5mac.com/2022/07/27/ios-16-beta-4-unsend-edit-messages/ (medium)
- https://blog.google/products-and-platforms/platforms/android/android-ios-end-to-end-encrypted-rcs-messaging (high)
