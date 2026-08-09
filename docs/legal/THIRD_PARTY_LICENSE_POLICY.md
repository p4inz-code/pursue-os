# PURSUE OS — Third-Party License Policy

> **Source:** PURSUE OS master handoff (authoritative). Restored 2026-08-09.
> **Category:** A — Authoritative licensing policy.

## Policy

- **Third-party software distributed with PURSUE may carry separate licenses.**
- Do not incorrectly claim that all third-party Linux packages are Apache-2.0.
- **Keep third-party licensing separate from PURSUE's Apache-2.0 license.**
- Apache-2.0 applies only to PURSUE's own code.

## Requirements

- Every third-party component distributed or vendored with PURSUE must be recorded in `THIRD_PARTY_NOTICES.md` with its license.
- License compatibility must be checked before adding components.
- Third-party license texts must be preserved where their licenses require it (e.g., MIT/Apache-2.0 notice requirements, GPL-family obligations).

## Current Status

The implementation foundation uses a deliberately small set of third-party Rust crates (see THIRD_PARTY_NOTICES.md). Foundation policy is to keep the dependency surface minimal.
