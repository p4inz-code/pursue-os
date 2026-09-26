//! Network routing modes and proxy configuration.
//!
//! Provides the core abstractions for distinguishing between standard direct networking
//! and Tor onion routing.
//!
//! # Critical Security & Privacy Invariants
//! - **No Complete Anonymity Guarantee**: PURSUE OS explicitly acknowledges that routing
//!   through Tor does NOT guarantee complete anonymity against advanced traffic analysis,
//!   browser fingerprinting, application-layer telemetry, or investigator operational error.
//! - **No Silent Fallback to Direct**: If `RoutingMode::Tor` is requested and the Tor proxy
//!   is unreachable, the system **fails closed**. It will NEVER fall back to direct networking.
//! - **No Silent Fallback to Tor**: If `RoutingMode::Direct` is requested, the system will
//!   never silently route requests through Tor.
//! - **Observable Network State**: The active routing mode is recorded in every navigation
//!   result and evidence provenance label.

use std::fmt;
use std::net::SocketAddr;
use std::str::FromStr;

use pursue_core::{Error, Result};
use serde::{Deserialize, Serialize};

/// The network routing mode selected for a browser session or request.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RoutingMode {
    /// Direct network communication via the host's standard network interface.
    Direct,
    /// Routed through the Tor onion routing network via a local SOCKS5 proxy.
    Tor,
}

impl RoutingMode {
    /// Returns `true` if this routing mode routes through Tor.
    pub fn is_tor(&self) -> bool {
        matches!(self, Self::Tor)
    }

    /// Returns `true` if this routing mode routes directly.
    pub fn is_direct(&self) -> bool {
        matches!(self, Self::Direct)
    }

    /// Returns a static string slice naming the mode.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Direct => "direct",
            Self::Tor => "tor",
        }
    }
}

impl fmt::Display for RoutingMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for RoutingMode {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "direct" => Ok(Self::Direct),
            "tor" => Ok(Self::Tor),
            other => Err(Error::InvalidInput(format!(
                "invalid routing mode '{other}'; expected 'direct' or 'tor'"
            ))),
        }
    }
}

/// Tor subsystem and SOCKS5 proxy configuration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TorConfig {
    /// SOCKS5 proxy address (default: `127.0.0.1:9050`).
    pub socks_proxy: SocketAddr,
    /// Optional Tor control port address (default: `127.0.0.1:9051`).
    pub control_port: Option<SocketAddr>,
    /// Connection timeout in seconds for Tor proxy handshakes.
    pub connect_timeout_secs: u64,
}

impl Default for TorConfig {
    fn default() -> Self {
        Self {
            socks_proxy: "127.0.0.1:9050".parse().expect("valid socket addr"),
            control_port: Some("127.0.0.1:9051".parse().expect("valid socket addr")),
            connect_timeout_secs: 15,
        }
    }
}

impl TorConfig {
    /// Creates a new Tor configuration with custom proxy endpoints.
    pub fn new(
        socks_proxy: SocketAddr,
        control_port: Option<SocketAddr>,
        connect_timeout_secs: u64,
    ) -> Result<Self> {
        if connect_timeout_secs == 0 {
            return Err(Error::InvalidInput(
                "Tor connect timeout must be greater than zero".into(),
            ));
        }
        Ok(Self {
            socks_proxy,
            control_port,
            connect_timeout_secs,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_mode_parsing_and_display() {
        assert_eq!(
            RoutingMode::from_str("direct").unwrap(),
            RoutingMode::Direct
        );
        assert_eq!(RoutingMode::from_str("tor").unwrap(), RoutingMode::Tor);
        assert_eq!(RoutingMode::from_str("TOR").unwrap(), RoutingMode::Tor);
        assert_eq!(
            RoutingMode::from_str("  Direct  ").unwrap(),
            RoutingMode::Direct
        );
        assert!(RoutingMode::from_str("vpn").is_err());

        assert_eq!(RoutingMode::Direct.to_string(), "direct");
        assert_eq!(RoutingMode::Tor.to_string(), "tor");
    }

    #[test]
    fn routing_mode_predicates() {
        assert!(RoutingMode::Tor.is_tor());
        assert!(!RoutingMode::Tor.is_direct());
        assert!(RoutingMode::Direct.is_direct());
        assert!(!RoutingMode::Direct.is_tor());
    }

    #[test]
    fn tor_config_defaults() {
        let cfg = TorConfig::default();
        assert_eq!(cfg.socks_proxy.to_string(), "127.0.0.1:9050");
        assert_eq!(cfg.control_port.unwrap().to_string(), "127.0.0.1:9051");
        assert_eq!(cfg.connect_timeout_secs, 15);
    }
}
