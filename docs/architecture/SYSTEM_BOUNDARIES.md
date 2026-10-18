# PURSUE OS: system boundaries

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A, Authoritative architecture boundaries.

## Evidence boundary

- Evidence is always the source of truth.
- AI must never silently alter evidence.
- Evidence integrity and provenance are foundational.
- Investigator actions and metadata remain investigator-controlled.

## AI boundary

- AI is assistant only.
- AI must not: manipulate evidence, silently modify sources, fabricate investigation facts, become the source of truth, or automate investigator judgment without consent.
- External AI APIs are not supported in V1 (local models only).
- A non-AI mode exists.

## Network / Tor boundary

- Tor is deeply integrated but not forced for every request.
- Network state must remain understandable to the investigator.
- No claim that Tor provides complete anonymity.

## User-Control boundary

- Browser and terminal remain investigator-controlled.
- Search engines are multiple, not a single forced provider.
- AI can be disabled in settings; AI mode has an on/off and startup prompt.

## Security boundary

- Privilege separation, secure storage, evidence integrity, network boundaries, Tor isolation, plugin isolation, AI boundaries, update security, package integrity, case isolation, secure defaults.
- Strong defaults ("S-class") with optional user additions (VPN, VM, additional isolation, advanced network configurations).
- No claim that the OS makes a user invulnerable.

## Repository boundary

- Apache-2.0 applies to PURSUE's own code only; third-party software may carry separate licenses.
- No fake claims (screenshots, badges, downloads, ISO links, benchmark/stability/user-count/completion claims) until they are real.
