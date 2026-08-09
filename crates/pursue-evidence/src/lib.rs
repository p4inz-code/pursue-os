//! Evidence integrity foundation for PURSUE OS.
//!
//! Implements the core evidence-integrity primitives locked by the master
//! handoff ("evidence is always the source of truth"; "evidence integrity and
//! provenance are foundational"):
//!
//! - **Content addressing** — evidence identity derived from SHA-256.
//! - **Immutable evidence records** — recorded artifacts with investigator
//!   supplied source metadata.
//! - **Append-only, hash-chained audit log** — provenance events where each
//!   entry links to the previous one cryptographically, making tampering
//!   detectable.
//! - **Evidence stores** — in-memory and file-backed, with verification on
//!   every read and verification of the audit chain on load.
//!
//! Security notes (matching PURSUE policy):
//! - Hash chaining *detects* alteration unless the chain itself is rewritten;
//!   it does not *prevent* it. A fully privileged actor who can rewrite the
//!   audit chain can regenerate every hash, so detection requires the chain to
//!   be externally anchored (e.g., signing) — a later-phase concern. Corruption
//!   and non-chain-rewriting modification are detected on every read and load.
//! - No `unsafe` code is permitted in this crate (`unsafe_code = "deny"`).
//! - Nothing here provides or claims anonymity.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod audit;
pub mod content_address;
pub mod record;
pub mod store;

pub use audit::{AuditEntry, AuditLog};
pub use content_address::ContentAddress;
pub use record::EvidenceRecord;
pub use store::{EvidenceStore, FileStore, InMemoryStore};

pub use pursue_core::{Error, Result};
