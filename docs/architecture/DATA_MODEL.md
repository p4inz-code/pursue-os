# PURSUE OS — Data Model

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A/B — entity *areas* are handoff-locked; schema detail is implementation-defined.

## Core Entities (locked by handoff)

- **Case** — the investigation container. Supports evidence, sources, provenance, investigator notes, relationships, timelines, findings, reports, exports.
- **Evidence** — collected information; the immutable source of truth.
- **Source** — the origin of information; provenance connects information to its source.
- **Provenance** — recorded origin and handling history.
- **Investigator notes** — investigator-controlled metadata.
- **Relationships** — connections between entities (investigation graphs).
- **Timelines** — chronological organization of investigation events.
- **Findings** — investigation conclusions anchored to evidence.
- **Reports** — professional outputs preserving provenance and evidence integrity (ZIP, JSON, protected/obfuscated code where appropriate).
- **Exports** — secure, investigation-friendly output forms.

## Principles

- Evidence is always the source of truth; findings and reports must remain traceable to evidence.
- AI must never silently alter source information or evidence.
- Case isolation: data belonging to one case is kept separate.

## Implementation-Defined

Exact schemas, identifiers, serialization formats, and indexes are implementation-defined (Category B) and locked during Phases 1C–1D. The evidence foundation defines the initial primitives: content addresses, evidence records, and audit events.
