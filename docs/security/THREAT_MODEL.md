# PURSUE OS — Threat Model

> **Source:** PURSUE OS master handoff (authoritative) — security areas. Restored 2026-08-09.
> **Category:** A (threat areas) / B (threat detail — implementation-defined, deepened during the security phase).

## Threat Areas (derived from locked security architecture)

The handoff does not enumerate a detailed threat model; the locked security areas imply the following threat categories, which must be addressed in later phases:

- **Evidence tampering** — alteration of evidence or provenance records (countered by integrity model: content addressing, hash-chained audit log, verification-on-read).
- **Privilege escalation / boundary crossing** — components exceeding their privilege separation (privilege separation, least privilege).
- **Case leakage / cross-case access** — case isolation violations.
- **Network exposure / deanonymization** — unintentional network contact during research; Tor isolation and network boundaries. **PURSUE must not claim Tor provides complete anonymity.**
- **Plugin abuse** — malicious or buggy plugins compromising the platform or evidence (plugin isolation).
- **AI boundary violations** — AI altering evidence, silently modifying sources, or fabricating facts (AI boundaries; assistant-only).
- **Supply chain / update compromise** — untrusted packages or updates (update security, package integrity).
- **Storage compromise / data leakage** — unauthorized reading of secure storage.
- **Malicious package or update behavior** — from SECURITY.md scope.

## Implementation-Defined

A full threat model with attacker profiles and mitigations is implementation-defined (Category B) and will be locked during the dedicated security phase. The current foundation implements the evidence-integrity countermeasure only.
