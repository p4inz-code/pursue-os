# PURSUE OS — Case Foundation (Phase 1D)

> **Category:** B — implementation documentation (maintained with the code).
> **Scope:** Phase 1D case/evidence foundation boundary. This document locks
> the minimal implementation contract for the case model before implementation
> begins. See `docs/core/DECISION_RECORD.md` Part B (row B13) for the decision.

## Status

- This is a **preparation artifact**: the Phase 1D implementation boundary.
  No Phase 1D implementation has been started. The next full session
  implements the case model against this contract.
- Phase 1D scope (locked by `docs/core/MASTER_SPEC.md` §15): case model,
  evidence model, provenance model, integrity model, secure storage,
  investigator-controlled metadata.
- Explicitly **out of Phase 1D**: timelines, findings, reports, exports,
  investigation graphs/relationships as an implemented model, sources as a
  standalone entity, any UI, any AI involvement, any database/framework, and
  any change to `pursue-evidence` or `pursue-runtime`.

## What Phase 1D already has

The evidence foundation (`crates/pursue-evidence`) already implements the
evidence, provenance, integrity, and secure-storage primitives that Phase 1D
lists. Phase 1D does not rebuild them:

| Locked requirement (docs) | Existing primitive (`pursue-evidence`) |
|---|---|
| Evidence is the immutable source of truth (`EVIDENCE_MODEL`) | `EvidenceRecord` — immutable record, private fields, getters only |
| Evidence identifiable, content-addressed | `ContentAddress` — SHA-256, validated hex serde |
| Evidence verifiable; tampering detectable (`EVIDENCE_INTEGRITY`) | Verification on every read; `Error::IntegrityViolation` |
| Provenance traceable and append-only (`PROVENANCE_MODEL`) | `AuditLog` — append-only, hash-chained, `verify()` on read/load |
| Secure evidence storage (`STORAGE_MODEL`, `SECURITY_MODEL`) | `EvidenceStore` trait + `InMemoryStore` / `FileStore` (manifest + blobs, integrity verified on open) |
| Investigator-controlled source labels (`SOURCE_MODEL`) | `EvidenceRecord::source` |
| Shared typed errors (`pursue-core`) | `Error::{InvalidInput, NotFound, IntegrityViolation, ...}` |

**Consequence:** the remaining Phase 1D implementation work is **only the case
model** — the container, its identifier, investigator-controlled metadata, the
evidence relationship, and the case-level provenance/integrity wiring. All
existing functionality is reused, never duplicated.

## Locked contract (minimal)

### 1. Case identifier (`CaseId`)

- Immutable, validated ASCII identifier: charset `[A-Za-z0-9._-]`, trimmed
  length 1..=128, no whitespace, no NUL bytes.
- Must be safe to use as a filesystem path component (case-isolation layout).
- Validated on construction **and** on deserialization (mirrors the validated
  `ServiceId`/`MethodName` identifiers in `pursue-runtime::ipc`).
- **Not content-addressed.** Cases are mutable, investigator-controlled
  containers; only *evidence* is content-addressed (`EVIDENCE_MODEL`).
- serde (hex-free string form), `Display`, `Debug`, `Eq`, `Ord`, `Hash`.
- Generation policy (e.g., sequential `case-<n>` or investigator-assigned
  slugs) is an implementation detail; validation above is the contract.

### 2. Case (the container)

A case is the investigator-controlled container (`GLOSSARY`: "an organized
investigation container holding evidence, sources, notes, relationships,
timelines, findings, reports, and exports"). Minimal fields for Phase 1D:

- `id: CaseId`
- `title: String` — non-empty after trimming (investigator-controlled).
- `notes: String` — investigator-controlled metadata; free text in V1
  (`GLOSSARY`: "investigator notes — investigator-controlled metadata
  attached to a case").
- `status: CaseStatus`.
- `created_at_unix: u64` and `created_by: String` (non-empty actor) — fixed at
  creation, anchoring the case to provenance (`PROVENANCE_MODEL`).
- Evidence membership: a set of `ContentAddress` referencing the case's
  evidence store.
- `audit: AuditLog` — the case's provenance record.

### 3. Investigator-controlled metadata

- `title`, `notes`, and `status` change **only** through explicit, typed
  operations that append audit events with an actor and timestamp. There is no
  silent mutation and no AI write path (`EVIDENCE_INTEGRITY`: "investigator-
  controlled metadata must not be silently modified"; `MASTER_SPEC` §6).
- `created_by` and `created_at_unix` are immutable after creation.

### 4. Evidence ↔ case relationship

- The case references evidence **by content address**; evidence bytes are
  never copied into the case. The evidence store remains the source of truth.
- `attach(address, actor)` / `detach(address, actor)` — explicit, audited
  operations. Attach requires the address to exist in the case's evidence
  store (no dangling references). Attach of an already-attached address is an
  idempotent no-op (no duplicate audit event).
- Blob verification stays in `pursue-evidence` (verification on every read);
  the case layer never re-verifies or stores blob bytes.

### 5. Provenance relationship

- Each case carries its own `AuditLog` (reused unchanged from
  `pursue-evidence`); every case event appends `(timestamp, actor, action,
  subject)`.
- Action constants (V1 set): `case.created`, `case.title.updated`,
  `case.notes.updated`, `case.evidence.attached`, `case.evidence.detached`,
  `case.closed`, `case.reopened`. Evidence-related events carry
  `subject = Some(address)`; other events use `None`.
- This keeps provenance explicit and traceable to investigator actions without
  modifying the audit schema.

### 6. Integrity relationship

- A case is persisted as a version-gated manifest (version 1) containing
  metadata, status, evidence membership, and the audit log — mirroring the
  existing `FileStore` manifest pattern.
- On open, the case store: (1) verifies the case audit chain, failing with
  `Error::IntegrityViolation` on any alteration; (2) verifies every attached
  address exists in the case's evidence store; (3) opens the case's evidence
  store, which re-verifies blobs and its own audit chain.
- Tampering with case metadata or membership therefore breaks the chain and
  is detectable. Deterministic and fully offline.

### 7. Case lifecycle

- `CaseStatus { Open, Closed }` — the minimal lifecycle.
- Transitions are explicit and audited: `close(actor)` and `reopen(actor)`.
  No implicit transitions; every transition appends an audit event.
- Additional states are a later-phase extension and must not be invented now.

## Storage and case isolation

- New workspace crate: `crates/pursue-case` (per `DECISION_RECORD` B2, future
  crates slot into the same workspace).
- File layout (file-backed store): `<root>/cases/<case_id>/case.json` (case
  manifest) and `<root>/cases/<case_id>/evidence/` (an existing
  `pursue-evidence` `FileStore`).
- **Case isolation** (`SECURITY_MODEL` / `SECURITY_BOUNDARIES` / `DATA_MODEL`):
  each case lives entirely under its own directory; the `CaseStore` API
  addresses one case at a time and exposes no cross-case access. Isolation is
  structural (per-case layout) plus API-level.
- Encryption at rest, signing, and external anchoring of the chain remain
  later-phase concerns (`SECURITY_BOUNDARIES`: enforcement is
  implementation-defined in later phases).

## Technical approach

- `pursue-case` depends on `pursue-core` + `pursue-evidence`, plus `serde` /
  `serde_json` (workspace deps). **No new dependencies**, no `unsafe`
  (workspace lint `deny`), `missing_docs = warn`.
- **No database.** SQLite remains deferred (`DECISION_RECORD` B4); the
  trait-based store boundary must allow a DB-backed implementation later
  without rework.
- Trait-based storage mirroring `pursue-evidence`: a `CaseStore` trait with
  `InMemoryCaseStore` and `FileCaseStore` implementations; integrity
  verification on every open; tamper detection tested.
- Tests: unit tests (identifier validation, metadata rules, lifecycle
  transitions, audit recording, tamper detection) and integration tests
  (persistence across reload, case isolation), following `TESTING_STRATEGY.md`.

## Non-goals for Phase 1D

- No UI, no graph/reporting UI (`V1_SCOPE`: deferred).
- No AI involvement of any kind in the case layer.
- No database/framework; no new dependencies.
- No changes to `pursue-evidence` or `pursue-runtime` (reuse only; a change
  would require its own decision-record row and review).
- No timelines, findings, reports, exports, or graph/relationship entities
  (later phases; the container leaves room for them, it does not implement
  them).
- No standalone Source entity: source labels already live on
  `EvidenceRecord::source` (`SOURCE_MODEL`).
- No evidence or case deletion (later phase); no encryption at rest.
- No IPC/service exposure of cases yet; that decision belongs to the phase
  that builds case services.

## Starting point (next session)

1. Add `crates/pursue-case` to the workspace `members` list.
2. Implement `CaseId` with validation + unit tests.
3. Implement `Case`, `CaseStatus`, and the audited operations + unit tests.
4. Implement the `CaseStore` trait with `InMemoryCaseStore` and
   `FileCaseStore` (per-case layout, verification on open) + tamper-detection
   and isolation tests.
5. Validate with the commands below; inspect `git diff`; commit.

## Validation

```bash
cd "/c/Users/Admin/Desktop/Pursue-OS" && cargo fmt --all --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo build --workspace --release
```
