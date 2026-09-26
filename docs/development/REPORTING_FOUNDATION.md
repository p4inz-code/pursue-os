# PURSUE OS — Forensic Reporting & Safe Export Foundation (Phase 4)

**Phase**: 4 — Forensic Reporting & Safe Export Foundation  
**Primary Crates**: `crates/pursue-report`, `crates/pursue-desktop`  
**Status**: Implemented & Verified  

---

## 1. Architectural Mission

The PURSUE OS Reporting Foundation provides cryptographically verifiable, deterministic forensic investigation reports. It allows investigators to synthesize case evidence, chronological timeline milestones, and chain-of-custody audit logs into standardized report packages suitable for legal proceedings, peer review, and intelligence dissemination.

### Key Invariants
1. **Deterministic Representation**: Identical case contents yield bit-for-bit identical report structures and cryptographic SHA-256 digests.
2. **Self-Contained Printable HTML**: Generated HTML reports contain zero external dependencies (no external JS, no external stylesheets, no web fonts, no third-party CDNs). They are fully self-contained, responsive, and include `@media print` rules for paper and PDF printing.
3. **Cryptographic Sealing**: Reports carry an embedded SHA-256 digest computed over the canonical representation of the report. Any tampering with metadata, evidence descriptions, or timeline items invalidates the cryptographic seal.
4. **Path Traversal Protection**: Export destinations are strictly sanitized. Directory traversal attempts (`..`), null bytes, and writes outside authorized directories are rejected fail-closed.
5. **Atomic File Writes**: Reports are written to temporary files and atomically renamed to prevent incomplete or corrupted files on disk.
6. **Secret Scrubbing**: Sensitive strings (such as bearer tokens, API credentials, authorization headers) are automatically scrubbed from free-text fields before report compilation.

---

## 2. Core Domain Models (`pursue-report::model`)

- [`InvestigationReport`]: Top-level report container holding metadata, audit summary, evidence items, timeline items, and the embedded `report_hash`.
  - `compute_canonical_hash(&self) -> String`: Computes the SHA-256 digest of the canonical JSON structure with `report_hash: None`.
  - `finalize_with_hash(self) -> Self`: Computes and embeds the cryptographic report hash.
  - `verify_integrity(&self) -> Result<()>`: Validates that the recorded report hash matches the canonical content digest, that the audit chain is reported intact, and that all attached evidence items passed verification.
- [`ReportMetadata`]: Case ID, title, notes, status, creator, creation timestamp, generation timestamp, requesting investigator identity, and item counts.
- [`EvidenceReportItem`]: SHA-256 content address, payload size in bytes, acquisition timestamp, provenance source tag, payload verification status, UTF-8 flag, and content preview snippet.
- [`TimelineReportItem`]: Sequence number, timestamp, category (`case`, `evidence`, `terminal`, `browser`, `audit`), actor, title, summary, and reference ID.
- [`AuditReportSummary`]: Total events count, chain integrity boolean (`chain_intact`), genesis event hash, and chain head hash.

---

## 3. Report Formats & Rendering (`pursue-report::generator`)

### 3.1 Deterministic JSON (`render_json`)
Outputs canonical, pretty-printed JSON structure preserving exact field ordering, numerical precisions, and embedded cryptographic digests.

### 3.2 Standalone Forensic HTML (`render_html`)
Outputs a standalone HTML5 document featuring:
- **PURSUE OS Forensic Header**: Case title, status pill badge, case identifier, generator identity, and generation timestamp.
- **Cryptographic Integrity Seal**: SHA-256 canonical digest box, audit log provenance verification badge (`[VERIFIED INTACT]`), total audit count, and chain head hash.
- **Case Details & Notes**: Creator identity, creation timestamp, attached evidence count, and escaped investigator notes.
- **Evidence Inventory Table**: Content address (SHA-256), payload size, acquisition timestamp, provenance source, verification badge, and embedded preview box.
- **Chronological Timeline Table**: Milestone sequence, timestamp, category badge, actor, title, and summary.
- **Forensic Footer**: Cryptographic seal summary.
- **Print Optimization**: Embedded `@media print` CSS rules guaranteeing clear page breaks and high-contrast rendering for paper and PDF export.

---

## 4. Export Safety & Integrity (`pursue-report::export`)

- [`validate_export_path`]: Validates target paths against path traversal attacks. Rejects null bytes (`\0`), parent directory components (`..`), and optionally validates that the canonical target parent resides within an allowed export directory.
- [`scrub_secrets`]: Detects and redacts credential tokens (`bearer ...`, `token=...`, `password=...`, `secret=...`) from free-form text inputs.
- [`export_report_atomic`]: Creates a temporary file (`.tmp_report_<pid>_<timestamp>_<rand>.part`) in the destination directory, writes and flushes all bytes, syncs to disk (`sync_all()`), and renames the file to the target path atomically.

---

## 5. Report IPC Service: `report`

Registered on the IPC router via [`pursue_report::ReportHandler`].

| Method | Parameters | Response Payload | Description |
| :--- | :--- | :--- | :--- |
| `report.generate` | `case_id`, `actor`, `format` ("json" \| "html") | `{ case_id, format, report_hash, content }` | Compiles case report in memory and returns rendered string with SHA-256 seal. |
| `report.preview_metadata` | `case_id` | `{ case_id, case_title, case_status, evidence_count, audit_events_count, audit_chain_verified }` | Returns summary metrics for report preview before compilation. |
| `report.export` | `case_id`, `actor`, `format`, `target_path` | `{ case_id, format, report_hash, destination, size_bytes, verified }` | Safely validates target path, renders report, and writes atomically to disk. |
| `report.verify` | `file_path` | `{ file_path, format, report_hash, case_id, verified, details }` | Reads report from disk and cryptographically verifies its structural and hash integrity. |

---

## 6. Verification & Test Coverage

Verified across:
1. `crates/pursue-report`: 5 unit and integration tests covering determinism, HTML self-containment, path traversal rejection, secret scrubbing, and IPC service workflows.
2. `crates/pursue-desktop`: End-to-end integration tests verifying report generation, preview, export to disk, report verification, and detection of tampered report files.
