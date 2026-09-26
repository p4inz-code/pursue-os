//! PURSUE OS — Investigation Browser & Tor Foundation Crate
//!
//! Provides the headless core engine for investigative web research, Tor onion
//! routing, case-bound profile isolation, and verifiable web evidence capture.
//!
//! # Architecture & Capabilities
//! - **Session Management**: [`BrowserSession`], [`BrowserSessionId`], [`SessionStatus`].
//! - **Routing Modes**: [`RoutingMode`], [`TorConfig`].
//! - **Navigation & Safety**: [`ValidatedUrl`], [`NavigationRequest`], [`NavigationResult`].
//! - **Execution Abstraction**: [`BrowserEngine`], [`MockBrowserEngine`], [`NetworkEngine`].
//! - **Evidence Ingestion**: [`WebEvidenceCapturer`], [`CapturedWebEvidence`], [`WebArtifactKind`].
//! - **Runtime Service**: [`BrowserService`].
//! - **IPC Boundary**: [`BrowserHandler`].
//!
//! # Critical Privacy & Security Boundaries
//! - **No Complete Anonymity Guarantee**: PURSUE OS explicitly acknowledges that routing
//!   through Tor obscures transport-layer IP addresses and routes to `.onion` hidden
//!   services, but does NOT guarantee complete anonymity against browser fingerprinting,
//!   application-layer tracking, or investigator operational mistakes.
//! - **Fail-Closed Tor Routing**: If `RoutingMode::Tor` is requested and the Tor proxy
//!   is unavailable, the engine **fails closed**. It will NEVER fall back to direct networking.
//! - **No Silent Tor Fallback**: If `RoutingMode::Direct` is requested, requests will never
//!   silently route through Tor.
//! - **Case Profile Isolation**: Browser session profile directories (`cookies`, `cache`,
//!   `local storage`) are strictly isolated per case and session. Case A can never access
//!   or inherit browser state from Case B.
//! - **Evidence Immutability**: Captured web artifacts are content-addressed via SHA-256
//!   and permanently recorded in the case's hash-chained audit log.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod engine;
pub mod evidence;
pub mod ipc;
pub mod navigation;
pub mod routing;
pub mod service;
pub mod session;
pub mod session_id;
pub mod url;

// Re-exports for clean API ergonomics
pub use engine::{BrowserEngine, MockBrowserEngine, MockResponse, NetworkEngine};
pub use evidence::{CapturedWebEvidence, WebArtifactKind, WebEvidenceCapturer};
pub use ipc::BrowserHandler;
pub use navigation::{
    DEFAULT_MAX_RESPONSE_BYTES, DEFAULT_NAV_TIMEOUT_SECS, MAX_ALLOWED_RESPONSE_BYTES,
    MAX_NAV_TIMEOUT_SECS, NavigationRequest, NavigationResult, scrub_sensitive_headers,
};
pub use routing::{RoutingMode, TorConfig};
pub use service::BrowserService;
pub use session::{BrowserSession, SessionStatus};
pub use session_id::{BrowserSessionId, MAX_BROWSER_SESSION_ID_LEN};
pub use url::ValidatedUrl;
