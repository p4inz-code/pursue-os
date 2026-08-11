//! Case foundation for PURSUE OS (Phase 1D).
//!
//! Implements the first component of the locked Phase 1D case/evidence
//! foundation (see `docs/development/CASE_FOUNDATION.md` and
//! `docs/core/DECISION_RECORD.md` B13):
//!
//! - **Validated case identifiers** — [`CaseId`], safe to use as filesystem
//!   path components, validated both on construction and on deserialization.
//! - **The case model** — [`Case`] as the investigator-controlled container:
//!   lifecycle status ([`CaseStatus`]), investigator-controlled metadata
//!   (title, notes), creation information, and evidence references by
//!   [`pursue_evidence::ContentAddress`] (never evidence bytes).
//! - **Audited operations** — every state-changing operation appends an event
//!   to the case's hash-chained [`pursue_evidence::AuditLog`], reused
//!   unchanged from `pursue-evidence`. No silent mutation is possible.
//! - **Case stores** — the [`CaseStore`] trait with [`InMemoryCaseStore`]
//!   (deterministic, for tests) and [`FileCaseStore`] (per-case isolation,
//!   persistence, and verification on load, reusing the existing
//!   `pursue-evidence` file store for evidence).
//!
//! Design rules (matching the locked boundary):
//! - Evidence remains the immutable source of truth; cases reference evidence
//!   only by content address and never duplicate bytes.
//! - No second evidence-integrity system: provenance and integrity primitives
//!   stay in `pursue-evidence`.
//! - No `unsafe` code (`unsafe_code = "deny"`), no database, no AI, no UI.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod case;
pub mod case_id;
pub mod file_store;
pub mod store;

pub use case::{
    ACTION_CLOSED, ACTION_CREATED, ACTION_EVIDENCE_ATTACHED, ACTION_EVIDENCE_DETACHED,
    ACTION_NOTES_UPDATED, ACTION_REOPENED, ACTION_TITLE_UPDATED, Case, CaseStatus,
};
pub use case_id::CaseId;
pub use file_store::FileCaseStore;
pub use store::{CaseStore, InMemoryCaseStore};

pub use pursue_core::{Error, Result};
