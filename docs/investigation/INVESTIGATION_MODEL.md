# PURSUE OS — Investigation Model

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative investigation requirements.

## The Workflow

PURSUE treats investigation as a workflow, not a collection of tools. From the README: investigators should be able to **research, collect, analyze, connect, preserve, and report** information from one system.

## Investigation Areas (locked)

- OSINT
- SOCMINT
- GEOINT
- CTI
- DFIR
- infrastructure intelligence
- media analysis
- evidence collection
- evidence storage
- provenance
- case management
- investigation graphs
- timelines
- reporting
- secure research
- browser workflows
- terminal workflows
- Tor
- modular tools
- plugins
- optional AI assistance

## Flagship Interfaces

The **Investigation Terminal** (command-based investigation, web scraping, information gathering, OSINT tools, frameworks, automation under investigator control) and the **Investigation Browser** (secure research browsing, Tor workflows, multiple search engines) form the major investigation interface layer.

## Principles

- Evidence is the source of truth throughout the workflow.
- Network state is understandable; Tor is not forced for every request.
- AI assists within the workflow but never alters evidence or sources.
- The environment should feel like an investigator's complete workstation.

## Implementation-Defined

Workflow orchestration details are implementation-defined (Category B) and locked in later phases.
