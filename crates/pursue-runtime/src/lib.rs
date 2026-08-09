//! Core runtime foundation for PURSUE OS (Phase 1C).
//!
//! Provides the in-process runtime boundary that core services build on:
//!
//! - [`config`] — typed configuration with explicit defaults, TOML loading,
//!   and validation.
//! - [`log`] — structured logging (levels, components, fields, sinks,
//!   redaction).
//! - [`service`] — minimal service lifecycle (`init -> start -> run ->
//!   shutdown`) with error propagation.
//! - [`ipc`] — typed request/response protocol, service dispatch, and
//!   platform-aware transports.
//!
//! Security properties:
//! - `unsafe` code is forbidden (`unsafe_code = "deny"`).
//! - No listeners are created implicitly; every transport is opt-in.
//! - Logging never captures evidence content; use [`log::redact`] for
//!   sensitive values.
//! - Configuration never contains secrets by design.
//!
//! See `docs/development/CORE_RUNTIME.md` for the implementation guide.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod config;
pub mod ipc;
pub mod log;
pub mod service;

pub use pursue_core::{Error, Result};
