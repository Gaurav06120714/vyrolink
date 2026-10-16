# LINE — research report

**Accessed:** 2026-10-15. **Confidence:** Medium.

| Item | Finding | Label |
|---|---|---|
| Letter Sealing v2 | Deployed in October 2019 after Isobe & Minematsu reported v1 forgery and impersonation attacks; encryption whitepaper v2.2 (December 2025) adds media E2EE | VERIFIED (LY Corp whitepaper v2.2 revision history) |
| Forward secrecy | Only "in some usage environments" (2019 report) | REPORTED |
| Independent analysis | "LINE-Break": a malicious server or TLS MITM can undermine integrity, authenticity and confidentiality; the **stateless design allows replay, reordering and blocking**; no origin authentication in groups enables impersonation | REPORTED (abstract; venue and year unconfirmed) |

**Lesson:** a **stateless** E2EE design (no ratchet, no replay window) allows replay and reordering. VYRO LINK must keep its dedup sets and its Olm ratchet, and must treat replays of *control* messages (edit/delete/react) as idempotent.

## Sources (accessed 2026-10-15)

- https://www.lycorp.co.jp/en/privacy-security/line-encryption-whitepaper-ver2.2.pdf (high)
- https://linecorp.com/en/security/encryption/2019h1 (high)
- https://www.v2security.dk/Koebenhavn6/session/4089915/line-break-cryptanalysis-and-reverse-engineering-of-letter-sealing (low–medium; metadata unverified)
