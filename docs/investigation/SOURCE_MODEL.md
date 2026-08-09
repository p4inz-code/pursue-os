# PURSUE OS — Source Model

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative source requirements.

## Locked Requirements

- Cases support **sources** — the origins of information.
- **Provenance connects information to its source**; provenance is foundational.
- **AI must not silently mutate source information** (sources remain investigator-controlled).
- Investigation spans many source types: web, social media (SOCMINT), geospatial (GEOINT), infrastructure, media, threat intelligence (CTI), and DFIR artifacts.

## Principles

- The source remains the source of truth for the information collected from it.
- AI may assist with organizing/summarizing investigator-selected material but never silently modifies source content.
- Provenance records must make it possible to trace evidence back to its original source.

## Implementation-Defined

Source metadata schema and source-type handling are implementation-defined (Category B), locked during the investigation/case phases. The evidence foundation records source labels as part of evidence records.
