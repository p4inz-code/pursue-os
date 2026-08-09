# PURSUE OS — Master Specification

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative project decisions. Do not rewrite casually.

## 1. Project

- **Name:** PURSUE OS
- **Brand / Creator:** P4inz (Atharva Patil)
- **GitHub:** https://github.com/p4inz-code/pursue-os
- **License:** Apache License 2.0 (PURSUE's own code). Third-party software may carry separate licenses.
- **Status:** Planning 100% locked. Repository foundation complete. Implementation starting.

## 2. Core Purpose

PURSUE OS is an investigation-focused Linux operating system providing a dedicated operating environment for investigators covering:

- OSINT, SOCMINT, GEOINT, CTI, DFIR
- infrastructure intelligence
- media analysis
- evidence collection and storage
- provenance
- case management
- investigation graphs and timelines
- reporting
- secure research
- browser workflows
- terminal workflows
- Tor
- modular tools
- plugins
- optional AI assistance

**Core philosophy:** *One operating system for the investigation workflow.*

PURSUE is **not** intended to become:

- another Kali Linux clone
- a cyberpunk/hacker OS
- a generic Linux distribution
- a giant collection of random tools

It should feel purpose-built for investigation, treating investigation as a workflow rather than a collection of tools.

## 3. Identity Attributes

The project should be: powerful, secure, customizable, professional, approachable, investigator-focused, privacy-oriented, reliable, modular, extensible.

Presentation should be: clean, modern, professional, configurable, not intimidating, not cyberpunk, not a green-terminal aesthetic, not excessive hacker imagery. The UI must not create a prison-like feeling.

## 4. Flagship Features

Two flagship interfaces:

1. **Investigation Terminal** — a purpose-built command-line environment. Mandatory for web scraping, information gathering, command-based investigation, OSINT tools, frameworks, and automation under investigator control. Strong UX, syntax highlighting, error highlighting, optional AI assistance, future expansion.
2. **Investigation Browser** — secure, configurable browsing and research workflows with integrated Tor capabilities. Tor-based browsing for appropriate workflows; Brave-based or normal browser workflow when Tor isn't required; multiple search engines (DuckDuckGo, Brave Search, onion search; Ahmia and Yahoo discussed as options). Investigator-controlled; Tor must not be forced for every request; network state must remain understandable.

Terminal and browser together form the major investigation interface layer.

## 5. Tor

Tor is deeply integrated (not a manually installed extra application). It must be configurable, understandable, secure, user-controlled, and integrated with relevant investigation workflows.

PURSUE must not claim Tor provides complete anonymity. The system must clearly communicate what Tor protects and what it does not.

## 6. Case + Evidence

Strong case management supporting evidence, sources, provenance, investigator notes, relationships, timelines, findings, reports, exports.

**Evidence is always the source of truth.** AI must never silently alter evidence. Evidence integrity and provenance are foundational.

## 7. Reporting

Professional reporting with multiple formats and export options: ZIP, JSON, protected/obfuscated code where appropriate, secure export, investigation-friendly outputs. Reporting must preserve provenance and evidence integrity. Do not overengineer V1.

## 8. AI

- Optional, user-controlled: AI mode on/off, ask at startup, explain pros/cons, don't scare users, don't force AI, disable in settings.
- **V1:** external online AI APIs are NOT supported. Local models preferred; user may download one of several supported models.
- Model categories (approximately three): fast model, deep-thinking model, coding/technical model. (An uncensored/high-assistance option was discussed but must be handled carefully within security, legal and project boundaries.)
- **Assistant only.** MUST NOT: manipulate evidence, silently modify sources, fabricate investigation facts, become the source of truth, automate investigator judgment without consent.
- MAY: summarize investigator-selected information, explain terminal output, assist with organization, help navigate workflows, assist with search, help interpret outputs, provide optional recommendations, assist in the terminal.
- Non-AI mode must exist. AI is expected to be available in the terminal as an assistant.

## 9. Plugin System

- **V1:** strong foundation, user extensible, roughly 5–7 useful tools per use case initially, more can be added later.
- **V2:** stronger plugin system, additional official modules, downloadable advanced modules, future plugins posted publicly / on GitHub.
- The foundation must be designed correctly now so V2 does not require rebuilding everything.

## 10. Security

Security is first-class. Strong default security ("S-class" by default) with further configuration available. Users may optionally add VPN, VM, additional isolation, advanced network configurations. PURSUE must not pretend the OS makes a user invulnerable.

Security architecture areas: privilege separation, secure storage, evidence integrity, network boundaries, Tor isolation, plugin isolation, AI boundaries, update security, package integrity, case isolation, secure defaults.

## 11. Customization

Highly customizable, but default UX must remain simple. Advanced settings exist for experienced users (similar in spirit to Windows PowerToys-style optional customization). User-choice oriented.

## 12. Tool Strategy

Support as much of the investigator workflow as practical. Integrate existing tools where appropriate, provide official tools where necessary, ensure major frameworks are supported, avoid reinventing mature tools without reason. Initial default tools are curated (approximately 5–7 per use case), not a dump of every available tool. Hardware-specific limitations remain the user's responsibility.

## 13. Repository Status

Public repository at https://github.com/p4inz-code/pursue-os. README communicates: coming soon, active development, planning complete, repository foundation complete, implementation starting, no stable release, no production ISO. No fake screenshots, badges, download buttons, ISO links, benchmark claims, stability claims, user counts, or feature-completion claims until they actually exist.

## 14. What Not To Do

- Do not restart naming or product planning.
- Do not reconsider Apache-2.0 unless a real legal requirement appears.
- Do not redesign the entire repository or add dozens of unnecessary folders.
- Do not start with random UI mockups or install every OSINT tool immediately.
- Do not claim a stable release, create a fake ISO, or claim anonymity/security guarantees.
- Do not make AI autonomous or let AI alter evidence.
- Do not add external AI APIs to V1.
- Do not build every plugin before the core, overengineer V1, or assume unverified technical facts.

## 15. Implementation Phases (from handoff)

- **1A** Technical architecture lock
- **1B** Minimal bootable PURSUE base
- **1C** Core runtime (services, configuration, logging, secure storage, IPC boundaries, update/package foundations, test harness)
- **1D** Case/evidence foundation (case model, evidence model, provenance model, integrity model, secure storage, investigator-controlled metadata)
- **1E** Terminal foundation (flagship)
- **1F** Browser + Tor foundation (dedicated major phase; Tor-in-detail and terminal are to be handled together)
