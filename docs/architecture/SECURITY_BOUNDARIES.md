# PURSUE OS — Security Boundaries

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative security boundaries.

> **Note:** The master handoff references this document as `docs/security/SECURITY_BOUNDARIES.md`, but the repository structure places it at `docs/architecture/SECURITY_BOUNDARIES.md` (matching the committed tree). This location is authoritative; no duplicate is created.

## Locked Security Architecture Areas

- **Privilege separation** — least privilege between components.
- **Secure storage** — protected storage for evidence, cases, and secrets.
- **Evidence integrity** — evidence is the immutable source of truth; tampering must be detectable.
- **Network boundaries** — controlled and understandable network state.
- **Tor isolation** — Tor workflows isolated from the rest of the system.
- **Plugin isolation** — plugins must not be able to compromise the platform or alter evidence.
- **AI boundaries** — AI is assistant only; never alters evidence or becomes the source of truth.
- **Update security** — secure, integrity-checked updates.
- **Package integrity** — packages verified before use.
- **Case isolation** — cases kept separate from each other.
- **Secure defaults** — "S-class" by default, with further configuration available.

## Constraints

- Strong default security; users may optionally add VPN, VM, additional isolation, advanced network configurations.
- PURSUE never pretends the OS makes a user invulnerable.
- No claim that Tor provides complete anonymity.

## Implementation-Defined

Concrete enforcement mechanisms (sandboxing, IPC permissions, storage encryption) are implementation-defined (Category B) and locked during Phases 1A–1C and the security phase. See DECISION_RECORD Part B for foundation decisions.
