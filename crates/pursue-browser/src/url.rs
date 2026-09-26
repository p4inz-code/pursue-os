//! URL validation and safety policing.
//!
//! Provides [`ValidatedUrl`], ensuring that all navigation targets conform to strict
//! security boundaries before network dispatch.
//!
//! # Security Boundaries
//! - Permitted schemes: `http://` and `https://` ONLY.
//! - Explicitly rejected dangerous schemes: `file://`, `data:`, `javascript:`,
//!   `vbscript:`, `blob:`, `about:`, `chrome:`, `ftp:`.
//! - Path traversal sequences and NUL bytes are rejected.
//! - Onion domain detection: addresses ending in `.onion` require `RoutingMode::Tor`.

use std::fmt;
use std::str::FromStr;

use pursue_core::{Error, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// A validated URL restricted to secure investigation protocols.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValidatedUrl {
    raw: String,
    scheme: String,
    host: String,
    port: u16,
    path_and_query: String,
    is_onion: bool,
}

impl ValidatedUrl {
    /// Parses and validates a raw URL string.
    pub fn parse(input: &str) -> Result<Self> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(Error::InvalidInput("URL must not be empty".into()));
        }
        if trimmed.contains('\0') {
            return Err(Error::InvalidInput(
                "URL contains forbidden NUL bytes".into(),
            ));
        }
        if trimmed.chars().any(char::is_control) {
            return Err(Error::InvalidInput(
                "URL contains unencoded control characters".into(),
            ));
        }

        // Split scheme: e.g. "https://example.com/path?query"
        let (scheme, rest) = match trimmed.find("://") {
            Some(idx) => {
                let s = &trimmed[..idx];
                let r = &trimmed[idx + 3..];
                (s.to_ascii_lowercase(), r)
            }
            None => {
                return Err(Error::InvalidInput(format!(
                    "missing scheme in URL '{trimmed}'; expected http:// or https://"
                )));
            }
        };

        // Strictly enforce permitted schemes
        if scheme != "http" && scheme != "https" {
            return Err(Error::InvalidInput(format!(
                "forbidden URL scheme '{scheme}://'; only http:// and https:// are permitted"
            )));
        }

        // Split authority from path: e.g. "user:pass@host:port/path?query"
        let (authority, path_and_query) = match rest.find('/') {
            Some(slash_idx) => (&rest[..slash_idx], &rest[slash_idx..]),
            None => (rest, "/"),
        };

        if authority.is_empty() {
            return Err(Error::InvalidInput(
                "URL authority/host must not be empty".into(),
            ));
        }

        // Strip userinfo if present: "user:pass@host:port"
        let host_port = match authority.rfind('@') {
            Some(at_idx) => &authority[at_idx + 1..],
            None => authority,
        };

        // Split host and port: e.g. "example.com:8080" or "[::1]:8080" or "example.onion"
        let (host, port) = if host_port.starts_with('[') {
            // IPv6 literal
            match host_port.find(']') {
                Some(bracket_end) => {
                    let h = &host_port[1..bracket_end];
                    let remainder = &host_port[bracket_end + 1..];
                    let p = if let Some(stripped) = remainder.strip_prefix(':') {
                        stripped.parse::<u16>().map_err(|_| {
                            Error::InvalidInput(format!("invalid port in '{host_port}'"))
                        })?
                    } else if scheme == "https" {
                        443
                    } else {
                        80
                    };
                    (h.to_ascii_lowercase(), p)
                }
                None => return Err(Error::InvalidInput("unterminated IPv6 host".into())),
            }
        } else {
            match host_port.find(':') {
                Some(colon_idx) => {
                    let h = &host_port[..colon_idx];
                    let p_str = &host_port[colon_idx + 1..];
                    let p = p_str.parse::<u16>().map_err(|_| {
                        Error::InvalidInput(format!("invalid port number in '{host_port}'"))
                    })?;
                    (h.to_ascii_lowercase(), p)
                }
                None => {
                    let p = if scheme == "https" { 443 } else { 80 };
                    (host_port.to_ascii_lowercase(), p)
                }
            }
        };

        if host.is_empty() {
            return Err(Error::InvalidInput("URL host must not be empty".into()));
        }

        let is_onion = host.ends_with(".onion");

        // Reconstruct canonical representation (without userinfo and omitting default ports)
        let raw = if (scheme == "https" && port == 443) || (scheme == "http" && port == 80) {
            format!("{scheme}://{host}{path_and_query}")
        } else {
            format!("{scheme}://{host}:{port}{path_and_query}")
        };

        Ok(Self {
            raw,
            scheme,
            host,
            port,
            path_and_query: path_and_query.to_string(),
            is_onion,
        })
    }

    /// Returns the canonical validated URL string.
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// Returns the URL scheme (`http` or `https`).
    pub fn scheme(&self) -> &str {
        &self.scheme
    }

    /// Returns the host name or IP string.
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns the destination port.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Returns the path and query string (e.g. `/index.html?q=osint`).
    pub fn path_and_query(&self) -> &str {
        &self.path_and_query
    }

    /// Returns `true` if this address targets a Tor `.onion` hidden service.
    pub fn is_onion(&self) -> bool {
        self.is_onion
    }
}

impl fmt::Display for ValidatedUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.raw)
    }
}

impl FromStr for ValidatedUrl {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse(s)
    }
}

impl Serialize for ValidatedUrl {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        self.raw.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ValidatedUrl {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_http_and_https_urls() {
        let u1 = ValidatedUrl::parse("https://example.com/path?key=val").unwrap();
        assert_eq!(u1.scheme(), "https");
        assert_eq!(u1.host(), "example.com");
        assert_eq!(u1.port(), 443);
        assert_eq!(u1.path_and_query(), "/path?key=val");
        assert!(!u1.is_onion());

        let u2 = ValidatedUrl::parse("http://example.com:8080/").unwrap();
        assert_eq!(u2.scheme(), "http");
        assert_eq!(u2.port(), 8080);
    }

    #[test]
    fn detects_onion_domains() {
        let u = ValidatedUrl::parse(
            "http://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion/",
        )
        .unwrap();
        assert!(u.is_onion());
        assert_eq!(
            u.host(),
            "duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion"
        );
    }

    #[test]
    fn rejects_forbidden_schemes() {
        assert!(ValidatedUrl::parse("file:///etc/passwd").is_err());
        assert!(ValidatedUrl::parse("data:text/html,<h1>attack</h1>").is_err());
        assert!(ValidatedUrl::parse("javascript:alert(1)").is_err());
        assert!(ValidatedUrl::parse("about:blank").is_err());
        assert!(ValidatedUrl::parse("chrome://settings").is_err());
        assert!(ValidatedUrl::parse("ftp://ftp.example.com").is_err());
        assert!(ValidatedUrl::parse("gopher://gopher.example.com").is_err());
    }

    #[test]
    fn rejects_malformed_urls() {
        assert!(ValidatedUrl::parse("").is_err());
        assert!(ValidatedUrl::parse("   ").is_err());
        assert!(ValidatedUrl::parse("example.com").is_err());
        assert!(ValidatedUrl::parse("https://").is_err());
        assert!(ValidatedUrl::parse("https://example.com\0/bad").is_err());
    }

    #[test]
    fn userinfo_is_stripped_from_canonical_url() {
        let u = ValidatedUrl::parse("https://secret_user:secret_pass@example.com/api").unwrap();
        assert!(!u.as_str().contains("secret_user"));
        assert!(!u.as_str().contains("secret_pass"));
        assert_eq!(u.host(), "example.com");
    }
}
