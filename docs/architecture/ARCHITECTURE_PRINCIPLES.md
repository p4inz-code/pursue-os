# PURSUE OS — Architecture Principles

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative architecture principles.

1. **Workflow-first.** Architecture serves the investigation workflow (research → collect → analyze → connect → preserve → report), not a loose collection of tools.

2. **Evidence is the immutable source of truth.** Integrity and provenance are foundational; architecture must make tampering detectable and provenance verifiable.

3. **Privilege separation.** Components hold only the privileges they need; evidence and cases are protected behind boundaries.

4. **Security by default.** Strong defaults ("S-class"); optional further configuration; never claim invulnerability.

5. **User control.** Network state, Tor usage, AI, search engines, and customization remain under investigator control; simple defaults, advanced settings available.

6. **Modularity and extensibility.** A strong plugin foundation now so future extension (V2 plugin system) doesn't require rebuilding.

7. **Integrate before reinventing.** Use mature existing tools where appropriate; build officially only where necessary.

8. **AI is an assistant.** AI may summarize/explain/organize/recommend; never alter evidence, fabricate facts, or become the source of truth. Local models only in V1.

9. **No overengineering.** V1 implements the locked scope and nothing more.

10. **Honest, verifiable status.** Nothing is claimed implemented until validated; no fake release artifacts or claims.
