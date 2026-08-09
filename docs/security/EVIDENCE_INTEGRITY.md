# PURSUE OS — Evidence Integrity

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative evidence-integrity requirements.

## Locked Requirements

- **Evidence is always the source of truth.**
- **Evidence integrity and provenance are foundational.**
- **AI must never silently alter evidence.**
- Evidence must remain traceable to its source and to investigator actions.
- Case/evidence foundation (Phase 1D) must implement: case model, evidence model, provenance model, **integrity model**, secure storage, investigator-controlled metadata.

## Integrity Requirements

- Tampering with evidence must be **detectable**.
- Provenance records must be verifiable and append-only where required.
- Investigator-controlled metadata must not be silently modified (especially not by AI).
- Findings and reports must remain anchored to verified evidence.

## Implementation Status

The evidence foundation implements: SHA-256 content addressing (evidence identity), immutable evidence records, and an append-only hash-chained audit/event structure with verification-on-read and tamper detection. See DECISION_RECORD B3 and the `pursue-evidence` crate.
