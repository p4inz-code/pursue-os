# PURSUE OS — Evidence Model

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative evidence requirements.

## Locked Requirements

- **Evidence is always the source of truth.**
- Evidence integrity and provenance are foundational.
- AI must never silently alter evidence.
- Case/evidence foundation (Phase 1D) covers: evidence model, provenance model, integrity model, secure storage, investigator-controlled metadata.
- Evidence supports the investigation workflow: collection, storage, provenance, analysis (via graphs/timelines), and reporting.

## Evidence Properties (requirements)

- **Immutable** — evidence is never silently modified.
- **Identifiable** — each artifact has a verifiable identity (content-addressed in the foundation).
- **Traceable** — provenance records connect evidence to its source and to investigator actions.
- **Verifiable** — integrity checks must detect any alteration.
- **Investigator-controlled** — metadata and handling are controlled by the investigator, never silently changed by AI.

## Implementation Status

The evidence foundation implements content addressing (SHA-256), immutable evidence records, and an append-only hash-chained audit log. See the `pursue-evidence` crate and DECISION_RECORD B3.
