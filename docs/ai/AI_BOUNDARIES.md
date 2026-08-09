# PURSUE OS — AI Boundaries

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative AI boundaries.

## Role

AI in PURSUE is an **assistant, not an authority**. The source remains the source of truth.

## AI MUST NOT

- manipulate evidence
- silently modify sources
- fabricate investigation facts
- become the source of truth
- automate investigator judgment without consent
- silently mutate source information

## AI MAY

- summarize investigator-selected information
- explain terminal output
- assist with organization
- help navigate workflows
- assist with search
- help interpret outputs
- provide optional recommendations
- assist in the terminal (assistant only)

## Product Requirements (locked)

- AI is **optional** and user-controlled: AI mode on/off, ask at startup, explain pros/cons clearly, don't scare users, don't force AI, users can disable AI in settings.
- **V1: external online AI APIs are NOT supported.** Local model approach preferred; the user may download one of several supported models.
- Approximately three model categories: fast model, deep-thinking model, coding/technical model.
- An uncensored/high-assistance option was discussed; it must be handled carefully within security, legal and project boundaries.
- A **non-AI mode** must exist for users who do not want AI.
- Advanced AI configuration comes in later updates.

## Security Relationship

Security must not depend on AI behavior (see SECURITY_MODEL). AI operates inside the AI boundary and cannot affect evidence integrity.
