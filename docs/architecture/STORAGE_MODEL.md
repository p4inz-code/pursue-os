# PURSUE OS — Storage Model

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A/B — storage *principles* are handoff-locked; storage *technology* is implementation-defined.

## Storage Principles (locked)

- **Evidence is the immutable source of truth.** Evidence storage must preserve integrity.
- **Secure storage** is a named security architecture area.
- **Case isolation** is a named security architecture area — cases must be kept separate.
- **Provenance** must be preserved with stored evidence.
- Evidence integrity is verifiable; tampering must be detectable.

## Storage Areas (locked)

- Evidence storage (collection and preservation of acquired artifacts).
- Case storage (cases with evidence, sources, notes, relationships, timelines, findings, reports, exports).
- Secure storage boundary (privilege separation).
- Investigator-controlled metadata storage.

## Implementation-Defined

Concrete storage technology (e.g., content-addressed blob store, SQLite, file layouts, encryption-at-rest) is implementation-defined and locked during Phases 1A–1D. The evidence foundation currently implements content addressing (SHA-256), immutable records, and an append-only hash-chained audit log (see DECISION_RECORD B3–B4).
