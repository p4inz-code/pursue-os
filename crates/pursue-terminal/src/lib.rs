//! Investigation Terminal foundation for PURSUE OS (Phase 1E).
//!
//! Provides the complete headless foundation for the Investigation Terminal flagship interface:
//!
//! - **Session model** ([`SessionId`], [`Session`], [`SessionStatus`]): Validated identifiers and
//!   session state bound to an active investigation [`pursue_case::CaseId`].
//! - **Command execution** ([`CommandRequest`], [`ExecutionResult`], [`CommandExecutor`]): Parameterized
//!   invocations with discrete arguments (`argv`), bounded stdout/stderr streams, and execution timeouts.
//! - **Executors** ([`MockExecutor`], [`ProcessExecutor`]): Deterministic in-memory simulation for tests
//!   and production OS process execution.
//! - **Evidence capture** ([`TerminalEvidenceCapturer`], [`CapturedEvidence`], [`StreamKind`]): Ingestion
//!   of raw output bytes directly into [`pursue_evidence::EvidenceStore`] and attachment to [`pursue_case::Case`].
//! - **ANSI utilities** ([`strip_ansi`]): Presentation helper preserving the invariant that raw bytes
//!   remain the evidentiary source of truth.
//! - **Service lifecycle** ([`TerminalService`]): Managed lifecycle conforming to
//!   [`pursue_runtime::service::Service`].
//! - **IPC dispatch** ([`TerminalHandler`]): IPC handler conforming to [`pursue_runtime::ipc::dispatch::Handler`].
//!
//! # Security Properties
//! - Unsafe code is forbidden (`#![deny(unsafe_code)]`).
//! - Missing documentation is warned (`#![warn(missing_docs)]`).
//! - Subprocesses are invoked directly without shell interpretation (`sh -c` / `cmd.exe /c`).
//! - Stream output buffers and execution timeouts are strictly bounded.
//! - Evidence bytes are preserved immutably and never duplicated inside case manifests.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod ansi;
pub mod command;
pub mod evidence;
pub mod executor;
pub mod ipc;
pub mod service;
pub mod session;
pub mod session_id;

pub use ansi::strip_ansi;
pub use command::{
    CommandRequest, DEFAULT_OUTPUT_LIMIT, DEFAULT_TIMEOUT_SECS, ExecutionResult, MAX_OUTPUT_LIMIT,
    MAX_TIMEOUT_SECS,
};
pub use evidence::{CapturedEvidence, StreamKind, TerminalEvidenceCapturer};
pub use executor::{CommandExecutor, MockExecutor, MockOutcome, ProcessExecutor};
pub use ipc::TerminalHandler;
pub use service::TerminalService;
pub use session::{Session, SessionStatus};
pub use session_id::{SESSION_ID_MAX_LEN, SESSION_ID_MIN_LEN, SessionId};

pub use pursue_core::{Error, Result};
