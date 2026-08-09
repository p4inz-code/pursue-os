# PURSUE OS — Case Model

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative case requirements.

## Locked Requirements

PURSUE must have strong case management. A case supports:

- **Evidence** — collected artifacts; evidence is always the source of truth.
- **Sources** — origins of information, traceable via provenance.
- **Provenance** — recorded origin and handling history of everything in the case.
- **Investigator notes** — investigator-controlled metadata.
- **Relationships** — connections between entities.
- **Timelines** — chronological organization of investigation events.
- **Findings** — conclusions anchored to evidence.
- **Reports** — professional outputs preserving provenance and evidence integrity.
- **Exports** — secure, investigation-friendly output forms.

## Principles

- Evidence integrity and provenance are foundational.
- AI must never silently alter evidence or source information.
- Case isolation is a security requirement (see SECURITY_BOUNDARIES / SECURITY_MODEL).
- Cases remain investigator-controlled.

## Implementation-Defined

Concrete case schema, storage, and API are implementation-defined (Category B) and locked during Phase 1D (case/evidence foundation). The evidence foundation being built first provides the integrity primitives the case model will rest on.
