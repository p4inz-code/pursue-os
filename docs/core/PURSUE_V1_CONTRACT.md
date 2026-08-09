# PURSUE V1 — Contract

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative project decisions.

This document is the contract for what PURSUE V1 commits to. It is derived directly from the master handoff; details the handoff does not establish are explicitly left implementation-defined.

## 1. Identity Contract

- V1 is an investigation-focused Linux operating system: *one operating system for the investigation workflow*.
- V1 is not a Kali clone, a cyberpunk/hacker OS, a generic Linux distribution, or a random tool collection.
- Presentation is clean, modern, professional, configurable, and not intimidating. The UI must not feel prison-like.

## 2. Flagship Contract

- **Investigation Terminal** is a V1 flagship and mandatory for core investigation workflows (web scraping, information gathering, command-based investigation, OSINT tools, frameworks, automation under investigator control).
- **Investigation Browser** is a V1 flagship: secure, configurable research browsing with integrated Tor capabilities.
- Terminal and browser together form the major investigation interface layer.

## 3. Tor Contract

- Tor is deeply integrated, configurable, understandable, secure, and user-controlled — not just another installed application.
- Tor is not forced for every web request; network state must remain understandable.
- PURSUE V1 must not claim Tor provides complete anonymity.

## 4. Browser Contract

- Tor-based browsing for appropriate investigation workflows.
- Brave-based or normal browser workflow where Tor/.onion is not required.
- Multiple search engines: DuckDuckGo and Brave Search expected; onion search capability expected; Ahmia and Yahoo discussed as options.
- Browser remains investigator-controlled.

## 5. Terminal Contract

- Investigator-focused, command-based, powerful, configurable, strong UX.
- Syntax highlighting and error highlighting.
- Optional AI assistance (assistant only).
- Supports future expansion; advanced customization later.
- Default shell is polished; additional shells supported where technically useful on Linux. V1 must not blindly recreate Windows shells.

## 6. Case + Evidence Contract

- Strong case management: evidence, sources, provenance, investigator notes, relationships, timelines, findings, reports, exports.
- **Evidence is always the source of truth.**
- AI must never silently alter evidence.
- Evidence integrity and provenance are foundational.

## 7. Reporting Contract

- Professional reporting: multiple formats, export options (ZIP, JSON), protected/obfuscated code where appropriate, secure export.
- Reporting preserves provenance and evidence integrity.
- Not overengineered for V1.

## 8. AI Contract

- AI is optional and user-controlled (mode on/off, ask at startup, explainable, disableable).
- **External online AI APIs are NOT supported in V1.**
- Local model approach preferred; user may download one of several supported models.
- Approximately three model categories: fast, deep-thinking, coding/technical.
- AI is assistant only: never manipulates evidence, never silently modifies sources, never fabricates facts, never becomes the source of truth, never automates investigator judgment without consent.
- A non-AI mode exists.

## 9. Plugin Contract

- V1 ships a strong plugin foundation (user-extensible, roughly 5–7 useful tools per use case initially).
- Foundation designed so V2 (stronger plugin system, official modules, downloadable advanced modules, public plugins) does not require rebuilding.
- Plugin isolation is a security requirement.

## 10. Security Contract

- Strong default security ("S-class" by default), further configuration available.
- Optional user additions: VPN, VM, additional isolation, advanced network configurations.
- No claim that the OS makes a user invulnerable.
- Security architecture areas (from handoff): privilege separation, secure storage, evidence integrity, network boundaries, Tor isolation, plugin isolation, AI boundaries, update security, package integrity, case isolation, secure defaults.

## 11. Tool Contract

- Curated defaults: approximately 5–7 useful tools per use case initially; users can install more.
- Integrate existing tools where appropriate; official tools where necessary; support major frameworks; avoid reinventing mature tools.
- Hardware-specific limitations remain the user's responsibility.

## 12. Open Items (implementation-defined)

The following V1 details are not established by the handoff and are implementation-defined (to be locked during their phases): exact base distribution, build system, package strategy, desktop environment, language/runtime choices, core service architecture, UI framework, data/storage architecture, test framework, CI strategy, ISO/build architecture. These are Phase 1A topics.
