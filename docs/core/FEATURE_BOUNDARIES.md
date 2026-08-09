# PURSUE OS — Feature Boundaries

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative project decisions.

## Flagship Features (V1 focus)

1. **Investigation Terminal** — purpose-built command-line environment; mandatory for web scraping, information gathering, command-based investigation, OSINT tools, frameworks, automation under investigator control. Strong UX, syntax/error highlighting, optional AI assistance.
2. **Investigation Browser** — secure, configurable research browsing with integrated Tor capabilities; multiple search engines; investigator-controlled network state.

## Core Platform (V1 foundations)

- Case management (evidence, sources, provenance, notes, relationships, timelines, findings, reports, exports).
- Evidence collection, storage, provenance, and integrity (evidence is the source of truth).
- Investigation graphs and timelines.
- Reporting and export (ZIP, JSON; provenance/integrity preserved).
- Secure storage and privilege separation.
- Plugin foundation (extensible; ~5–7 curated tools per use case).
- Optional local AI assistance (assistant only; non-AI mode; no external APIs).
- Modular tool integration (integrate existing tools; official tools where necessary).

## Supporting Investigation Areas (covered via curated tooling)

OSINT, SOCMINT, GEOINT, CTI, DFIR, infrastructure intelligence, media analysis, secure research.

## Deferred / Later Phases

- Full terminal and full browser implementations (foundations first).
- Tor integration in detail (dedicated major phase, together with the terminal).
- Graph UI and reporting UI.
- Advanced AI configuration.
- Plugin marketplace / public plugin distribution (V2).
- Stronger V2 plugin system, official modules, downloadable advanced modules.
- Production release and final ISO.
