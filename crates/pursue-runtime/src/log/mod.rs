//! Structured logging foundation for PURSUE OS.
//!
//! # Design
//! - **Levels** — [`Level`]: `Error`, `Warn`, `Info`, `Debug`, `Trace`,
//!   filtered before any sink is touched.
//! - **Records** — [`Record`]: timestamped (Unix seconds + RFC 3339 UTC),
//!   leveled, component-scoped events with optional structured key/value
//!   fields.
//! - **Sinks** — pluggable destinations (see [`Sink`]); the default is JSON
//!   lines on standard error ([`StderrSink`]), keeping standard output clean.
//! - **Logger** — [`Logger`] combines level filtering, component scoping, and
//!   a sink; [`init_global`] installs the process-wide instance.
//!
//! # Security
//! Logs are diagnostics, **not** evidence. Services must never log raw
//! evidence contents, secrets, credentials, private keys, or authentication
//! tokens. Wrap sensitive values with [`redact`] so every output path renders
//! `***`. Nothing in this module reads file contents, environment secrets, or
//! evidence data.

mod level;
mod logger;
mod record;
mod redact;
mod sink;
mod timestamp;

pub use level::Level;
pub use logger::{DEFAULT_COMPONENT, Logger, LoggerBuilder, global, init_global, try_global};
pub use record::Record;
pub use redact::{Redacted, redact};
pub use sink::{JsonLinesSink, NullSink, Sink, StderrSink, TestSink};
