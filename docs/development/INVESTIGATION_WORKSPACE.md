# PURSUE OS — Investigation Workspace Specification (Phase 3)

**Phase**: 3 — Investigation Workspace & Case Operations  
**Primary Crates**: `crates/pursue-case`, `crates/pursue-desktop`  
**Status**: Implemented & Verified  

---

## 1. Architectural Mission

The PURSUE OS Investigation Workspace provides the central operational environment for digital forensic and intelligence analysts. It integrates case lifecycle management, evidence exploration, chronological timeline synthesis, and cryptographic provenance verification into a single, cohesive desktop interface.

### Foundational Constraints & Locks
1. **Zero Direct UI Filesystem/Process/Database Access**: The desktop UI never touches raw disk storage, SQLite/databases, or processes directly. Every single operation is mediated through strictly validated typed IPC requests over the local `pursue-runtime` router boundary.
2. **Immutable Evidence Integrity**: Evidence artifacts remain strictly immutable in content-addressed storage. Cases reference evidence exclusively by cryptographic SHA-256 content address.
3. **Explicit Audited Operations**: No state change occurs silently. Case creation, metadata changes (title, notes), evidence attachments/detachments, and lifecycle transitions (close, reopen) append immutable events to the case's cryptographic SHA-256 hash-chain audit log.
4. **Strict Cross-Case Partitioning**: Artifacts and audit trails are completely isolated per case. Operations performed within Case A can never leak or attach to Case B.

---

## 2. IPC Service Architecture: `case`

The `case` service is registered on the IPC router via [`pursue_case::CaseHandler`] and wraps a shared thread-safe [`pursue_case::FileCaseStore`].

### Supported IPC Methods

| Method | Parameters | Response Payload | Description |
| :--- | :--- | :--- | :--- |
| `case.create` | `id`, `title`, `actor` | `{ id, title, notes, status, created_at_unix, created_by, verified }` | Validates inputs and initializes a new case with genesis audit event. |
| `case.get` | `id` | Case metadata, evidence addresses list, audit event count. | Loads case and verifies integrity on load. |
| `case.list` | *(empty)* | `[ { id, title, notes, status, created_at_unix, evidence_count, audit_events_count } ]` | Enumerates all stored cases. |
| `case.update_title` | `id`, `title`, `actor` | `{ id, title, audit_events_count }` | Updates investigator title and appends `case.title.updated` audit event. |
| `case.update_notes` | `id`, `notes`, `actor` | `{ id, notes, audit_events_count }` | Updates investigator notes and appends `case.notes.updated` audit event. |
| `case.close` | `id`, `actor` | `{ id, status: "closed", audit_events_count }` | Transitions open case to closed state, audited via `case.closed`. |
| `case.reopen` | `id`, `actor` | `{ id, status: "open", audit_events_count }` | Reopens closed case, audited via `case.reopened`. |
| `case.verify` | `id` | `{ id, verified: bool, manifest_verified, audit_chain_verified, evidence_verified_count }` | Performs deep cryptographic verification across manifest, hash-chain, and evidence blobs. |
| `case.evidence.list` | `id` | `[ { address, size, acquired_at_unix, source, verified } ]` | Lists evidence attached to case with verified statuses. |
| `case.evidence.read` | `id`, `address` | `{ address, size, source, is_utf8, preview, verified }` | Reads artifact, verifies SHA-256 match, and formats decoded preview. |
| `case.audit.list` | `id` | `{ case_id, chain_verified, events_count, events: [ ... ] }` | Returns complete provenance hash chain with per-event verification. |

---

## 3. Desktop Shell Components (`pursue-desktop`)

### 3.1 Investigation Dashboard (`dashboard_view.rs`)
- **System Telemetry Cards**: Real-time counts of Total Cases, Active/Open Cases, Attached Evidence Items, Chained Audit Events, and IPC Subsystem Status.
- **Active Case Glance**: Displays active case ID, status badge, title, investigator notes excerpt, and attached evidence count.
- **Quick Navigation**: Instant switching to Evidence Explorer, Timeline, Audit Trail, Terminal, and Reports.

### 3.2 Case Management View (`case_view.rs`)
- **Case Initialization**: Validated case identifier and title input with immediate activation.
- **Audited Case Editing**: In-place title editing and multiline notes journaling with explicit audited commits.
- **Lifecycle Controls**: One-click explicit Close and Reopen transitions.
- **Deep Verification**: One-click verification validating case manifest, SHA-256 hash chain, and every single attached evidence blob on disk.

### 3.3 Evidence Explorer (`evidence_view.rs`)
- **Inventory List**: Content address (SHA-256), payload size, acquisition timestamp, provenance source tag, and cryptographic status badge (`[VERIFIED]` / `[UNVERIFIED]`).
- **Inspection Pane**: Content inspector providing UTF-8 string rendering or formatted hex dump preview for binary evidence.

### 3.4 Investigation Timeline (`timeline_view.rs`)
- **Chronological Synthesis**: Unifies case creation, title edits, notes journaling, terminal evidence captures, browser evidence captures, and case lifecycle transitions.
- **Milestone Cards**: Sequence number, category tag (`CASE`, `EVIDENCE`, `TERMINAL`, `BROWSER`, `AUDIT`), actor, title, summary, and underlying content address reference.

### 3.5 Provenance Audit Viewer (`audit_view.rs`)
- **Cryptographic Chain Seal**: Top banner displaying real-time hash chain integrity verification status (`[CHAIN INTACT — SHA-256 VERIFIED]`).
- **Event Audit Log**: Detailed ledger displaying sequence number, action name, actor identity, subject address, previous SHA-256 hash, and current event hash.

---

## 4. Verification & Testing

The Investigation Workspace is verified through 11 end-to-end integration tests in `crates/pursue-desktop/tests/desktop_integration.rs` covering:
- IPC-mediated case lifecycle (create, get, list, edit title, edit notes, close, reopen).
- Deep cryptographic verification across manifest, audit chain, and evidence files.
- Cross-case isolation ensuring Case A and Case B maintain partitioned evidence and audit logs.
- Graceful handling of empty cases and invalid parameters.
