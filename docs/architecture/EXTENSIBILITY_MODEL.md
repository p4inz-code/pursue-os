# PURSUE OS — Extensibility Model

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative extensibility requirements.

## Locked Requirements

- PURSUE should be **modular** and **extensible**.
- The investigation domain is treated broadly: support as much of the investigator workflow as practical.
- **Integrate existing tools where appropriate**; provide official tools where necessary; ensure major frameworks are supported.
- **Avoid reinventing mature tools** without reason.
- Initial default tools are **curated** (~5–7 per use case), not a dump of everything available; users can install more.
- **Highly customizable**, with a simple default UX and advanced settings for experienced users (PowerToys-style optional customization).
- The plugin foundation must be designed now so the V2 plugin system doesn't require rebuilding (see PLUGIN_ARCHITECTURE.md).

## Implementation-Defined

The specific extension points, plugin API, and customization surface are implementation-defined (Category B) and locked during the plugin/customization phases.
