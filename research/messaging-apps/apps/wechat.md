# WeChat — research report

**Accessed:** 2026-10-15. **Confidence:** High for the transport findings (Citizen Lab primary report).

| Item | Finding | Label |
|---|---|---|
| E2EE | None for chats (server-readable) | REPORTED (widely documented) |
| MMTLS | A custom TLS 1.3 derivative with deterministic IVs and **no forward secrecy**. An inner legacy "business-layer" AES-CBC encryption leaks metadata (user IDs, request URIs). No practical attack was shown, because MMTLS wraps it. Tencent is moving to AES-GCM | VERIFIED (Citizen Lab 2024-10-15; PoPETs 2025) |

**Lesson:** this confirms the rule "**never invent cryptographic constructions**": home-grown modifications of standard protocols introduced the weaknesses. VYRO LINK uses Noise NN and Olm unmodified.

## Sources (accessed 2026-10-15)

- https://citizenlab.ca/2024/10/should-we-chat-too-security-analysis-of-wechats-mmtls-encryption-protocol/ (high)
- https://petsymposium.org/popets/2025/popets-2025-0163.pdf (high)
