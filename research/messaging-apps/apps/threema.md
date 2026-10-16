# Threema — research report

**Accessed:** 2026-10-15. **Confidence:** Medium–high (official whitepaper and FAQ; ETH paper via press).

| Item | Finding | Label |
|---|---|---|
| Identity | Random 8-character Threema ID; no phone number required | REPORTED |
| **Verification levels** | Red: key from the server only. Orange: matched to an address-book entry. **Green: QR scanned in person** (`3mid:<ID>,<pubkeyhex>`); the app compares the scanned key with the directory's | VERIFIED (cryptography whitepaper; FAQ) |
| ETH Zurich 2023 | Seven attacks on the old protocol (e.g. nonce DB not migrated → replay/reflection; account cloning with physical access); fixed or superseded by **Ibex** (forward secrecy layer) on 2022-11-29 | REPORTED (Help Net Security, The Hacker News) |
| Multi-device | Desktop 2.0 beta 50 (2025-04-22): multi-device with Android; `libthreema` (a Rust core) in source | REPORTED (Threema changelog) |
| Open source | Apps open-sourced in 2020 | REPORTED |

**Lessons:**
- The **verification-level UI** (clearly showing *how* a key was obtained) is directly applicable. In VYRO LINK every contact is QR-paired in person (equivalent to green), and the "safety number compared" flag is our extra level.
- The ETH **nonce-database-not-migrated** finding warns that the replay protection state (our seen set and vaccines) must survive backup and restore, or be reset safely.

## Sources (accessed 2026-10-15)

- https://threema.com/assets/documents/cryptography_whitepaper.pdf (high)
- https://threema.com/faq/levels_expl (high)
- https://www.helpnetsecurity.com/2023/01/11/threema-vulnerabilities/ (medium)
- https://threema.com/en/changelog/threema-20-for-desktop-beta-50 (medium–high)
