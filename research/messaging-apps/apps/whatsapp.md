# WhatsApp — research report

**Accessed:** 2026-10-15. **Method:** public documentation and reporting only.
- WhatsApp Desktop is installed on this Mac, but **it was not reverse-engineered**: Meta's terms prohibit it, and the task restricts us to lawful, authorised methods.

**Overall confidence:** Medium–high.

## 1. Capabilities

| Capability | Behaviour | Label | Source |
|---|---|---|---|
| E2EE protocol | Signal Protocol for messages, voice and video; whitepaper latest visible revision 2023-09-27 | VERIFIED | WhatsApp Security Whitepaper (PDF) |
| Multi-device | Each message is encrypted pairwise to every device ("client fan-out"); the server does not store messages after delivery | REPORTED | multi-device whitepaper coverage (2021) |
| Edit | Within **15 minutes**; text only | VERIFIED | blog.whatsapp.com (2023-05-22) |
| Delete for everyone | Window grew: 7 min → ~68 min → reported **~60 h** (2 d 12 h). Earlier recipient-side rule: a revoke arriving after 13 h 8 min 16 s is ignored | REPORTED (secondary, conflicting) | telecomtalk.info, phonearena.com, getkanal.com |
| Reactions, replies, voice notes, polls, channels, communities, disappearing messages, view-once | Yes | REPORTED (widely documented; not re-fetched) | — |
| Backups | Optional E2EE backups | REPORTED | EFF 2023 |
| Contact discovery | Phone-number address book upload | REPORTED | — |

## 2. Lessons

- The **recipient-side revoke deadline** ("ignore a delete that arrives too late") is a direct model for a store-and-forward network: deletes need an **expiry**, like envelopes.
- Edit and delete windows exist to limit abuse and confusion; VYRO LINK needs windows measured in **sender time**, checked against plausible clock skew.

**Avoid:** phone-number identity; server-mediated multi-device.

## 3. Sources (accessed 2026-10-15)

- https://www.whatsapp.com/security/WhatsApp-Security-Whitepaper.pdf (high)
- https://blog.whatsapp.com/now-you-can-edit-your-whatsapp-messages (high)
- https://telecomtalk.info/?p=477299 ; https://www.phonearena.com/news/whatsapp-delete-for-everyone-no-time-limit-under-development_id136159 ; https://getkanal.com/blog/delete-whatsapp-message-for-everyone (low–medium)
- https://techcrunch.com/2021/07/14/whatsapp-is-testing-multi-device-support-that-works-without-the-phone/ (medium)
- https://www.eff.org/deeplinks/2023/05/how-do-different-encrypted-messaging-apps-treat-deleted-messages (medium)
