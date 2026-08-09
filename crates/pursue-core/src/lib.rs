//! Shared foundation primitives for PURSUE OS.
//!
//! This crate intentionally contains only small, well-tested primitives that
//! other foundation crates build on: the unified error type and hexadecimal
//! encoding used by content addressing.
//!
//! Security properties:
//! - No `unsafe` code is permitted in this crate (`unsafe_code = "deny"`).
//! - No network, filesystem, or privileged operations live here.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod error;
pub mod hex;

pub use error::{Error, Result};
