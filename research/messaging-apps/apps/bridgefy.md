# Bridgefy — research report

**Accessed:** 2026-10-15. **Confidence:** High for the 2021–2022 findings (peer-reviewed).

| Item | Finding | Label |
|---|---|---|
| 2020/2021 analysis | Users trackable; no authenticity; no effective confidentiality; DoS via crafted messages | VERIFIED (ePrint 2021/214) |
| 2022 "Breaking Bridgefy, again" (USENIX Security 2022) | After adopting libsignal: still trackable; **unauthenticated broadcasts → impersonation**; first exchange MITM-able with **no key verification**; a single crafted message removes a node from the network (DoS); a practical attack read **about half of the Signal-protected 1:1 messages** (fixed 2021-08-14) | VERIFIED (USENIX page; Royal Holloway press release) |
| Current state | UNKNOWN; no later audit found | UNKNOWN |

**Lessons (high value for VYRO LINK):**
- "Adopting libsignal is not enough": the **composition** (compression, broadcast, handshake bootstrapping) broke it. VYRO LINK has no compression (ADR, Doc 06), pairs in person (no MITM bootstrapping), uses authenticated link sessions, and fuzzes every decoder.
- A **single crafted message causing DoS** is the attack class our fuzzing and bounded decoders target. Keep the fuzz campaign in CI.

## Sources (accessed 2026-10-15)

- https://eprint.iacr.org/2021/214 (high)
- https://usenix.org/conference/usenixsecurity22/presentation/albrecht (high)
- https://royalholloway.ac.uk/about-us/news/royal-holloway-cryptographers-urge-caution-over-use-of-secure-messaging-app (high)
