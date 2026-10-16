# Reticulum / LXMF / Sideband — research report

**Accessed:** 2026-10-15. **Confidence:** Medium (official repositories; third-party mirrors lower).

| Item | Finding | Label |
|---|---|---|
| Stack | Reticulum: a network stack over any half-duplex medium ≥ 5 bit/s with a 500-byte MTU (LoRa, packet radio, serial, Wi-Fi). LXMF: messaging format with E2EE and forward secrecy. Sideband: the end-user app | REPORTED (github.com/markqvist/Reticulum; PyPI) |
| Delivery modes | Opportunistic (~295 bytes, no link), direct (over a link), **propagated** (store-and-forward via propagation nodes that also sync with each other), and **paper** (QR / `lxm://` URIs) | REPORTED |
| Audit | "Has not been externally security audited" | REPORTED (project README) |

**Lessons:**
- **"Paper" message delivery via QR** is a genuinely offline fallback. VYRO LINK already uses QR for pairing; carrying a *message* by QR is a Tier 3 idea.
- Propagation nodes syncing with each other resembles our relay classes (ADR-019, always-on nodes).

## Sources (accessed 2026-10-15)

- https://github.com/markqvist/Reticulum (high)
- https://pypi.org/project/rns/ (medium)
- https://docsearch.algolia.com/mcp/docs/repo/markqvist/lxmf (medium)
