# PURSUE OS — Security Model

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative security requirements.

## Locked Requirements

- **Security is a first-class concern.** PURSUE provides strong default security — "S-class" by default — with further configuration available.
- Users may optionally add: VPN, VM, additional isolation, advanced network configurations.
- PURSUE must **not pretend the OS makes a user invulnerable**.
- Security must not depend on AI behavior.
- Evidence must remain traceable to its source and to investigator actions.

## Security Architecture Areas (locked)

- Privilege separation
- Secure storage
- Evidence integrity
- Network boundaries
- Tor isolation
- Plugin isolation
- AI boundaries
- Update security
- Package integrity
- Case isolation
- Secure defaults

## Security Posture

- Strong defaults with understandable, user-controlled configuration.
- Honest communication of what protections exist and what they do not guarantee (especially Tor: no complete-anonymity claim).

## Implementation-Defined

Concrete mechanisms (sandboxing, cryptographic storage, service hardening, update signing) are implementation-defined (Category B) and locked during Phases 1A–1C and the dedicated security work. The evidence foundation implements integrity primitives (content addressing + hash-chained audit log) as the first concrete security components.
