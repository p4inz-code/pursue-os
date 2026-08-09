//! IPC / service boundary foundation for PURSUE OS.
//!
//! # Layers
//! - [`protocol`] — the typed, serde-based request/response contract, service
//!   identity, and a stable error model. Platform-independent.
//! - [`dispatch`] — routes requests to service handlers by identity.
//! - [`transport`] — platform-aware transports: an in-memory duplex channel
//!   that works on every platform, plus a Unix-domain-socket transport
//!   (`transport::unix_transport`) that is compiled everywhere but only
//!   *exercised* on Unix; CI validates it on ubuntu-latest.
//!
//! # Security
//! - Protocol and transport errors never carry evidence content, secrets, or
//!   credentials; messages are diagnostics only.
//! - Authorizing privileged operations is a *service-layer* concern; this
//!   boundary provides the typed channel, not the authority decision.
//! - Malformed frames are rejected at the protocol layer before dispatch.

pub mod dispatch;
pub mod protocol;
pub mod transport;

#[cfg(unix)]
pub use transport::unix_transport;

pub use dispatch::{Handler, Router};
pub use protocol::{IpcError, IpcErrorCode, MethodName, Request, Response, ServiceId};
pub use transport::{InMemoryTransport, Transport};
