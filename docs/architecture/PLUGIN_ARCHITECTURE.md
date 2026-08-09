# PURSUE OS — Plugin Architecture

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative plugin requirements.

## Locked Requirements

- **V1:** the plugin foundation must be strong; users should be able to extend later; roughly 5–7 useful tools per use case initially; users can add more later; advanced customization can be introduced later.
- **V2:** stronger plugin system; additional official modules; downloadable advanced modules; future plugins can be posted publicly / on GitHub.
- **Critical requirement:** the plugin foundation must be designed correctly now so V2 does not require rebuilding everything.
- **Plugin isolation** is a named security architecture area (see SECURITY_MODEL).

## Principles

- Modular architecture; plugins extend the investigation platform.
- Curated official tooling first (~5–7 per use case); more via plugins.
- Plugin behavior must be isolated and must never silently alter evidence (AI boundaries apply within plugins too).

## Implementation-Defined

The plugin API, manifest format, loading mechanism, and isolation mechanism are implementation-defined (Category B) and will be locked during the plugin foundation phase. No plugin API is built in the current foundation phase — only the architectural requirement is recorded.
