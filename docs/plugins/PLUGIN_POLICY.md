# PURSUE OS — Plugin Policy

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative plugin policy.

## Locked Policy

- **V1:** strong plugin foundation; users can extend later; roughly 5–7 useful tools per use case initially; users can add more later; advanced customization introduced later.
- **V2:** stronger plugin system; additional official modules; downloadable advanced modules; future plugins can be posted publicly / on GitHub.
- The foundation must be designed now so V2 does not require rebuilding everything (see PLUGIN_ARCHITECTURE.md).
- **Plugin isolation** is a security requirement: plugins must not compromise the platform or silently alter evidence.
- AI boundaries apply within plugins as everywhere else.

## Distribution

- V1: foundation + curated official tooling; no marketplace.
- V2+: official modules, downloadable advanced modules, public plugin distribution.

## Implementation-Defined

Plugin API, manifest, loading, and isolation mechanics are implementation-defined (Category B), locked during the plugin foundation phase. Not built in the current foundation phase.
