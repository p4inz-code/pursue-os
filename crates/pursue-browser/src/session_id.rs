//! Browser session identifier model.
//!
//! Provides [`BrowserSessionId`], a validated newtype guaranteeing that browser
//! session identifiers are non-empty, length-bounded, and safe for use in
//! filesystem paths and provenance records.

use std::fmt;
use std::str::FromStr;

use pursue_core::{Error, Result};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Maximum permitted length for a browser session identifier in characters.
pub const MAX_BROWSER_SESSION_ID_LEN: usize = 64;

/// A validated browser session identifier.
///
/// # Validation Rules
/// - Non-empty after trimming surrounding whitespace.
/// - Length between 1 and 64 characters.
/// - Contains only ASCII alphanumeric characters, hyphens (`-`), and underscores (`_`).
/// - Path traversal sequences (`.` and `..`) are explicitly forbidden.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BrowserSessionId(String);

impl BrowserSessionId {
    /// Creates and validates a new [`BrowserSessionId`].
    ///
    /// Surrounding whitespace is trimmed before validation.
    pub fn new(raw: &str) -> Result<Self> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Err(Error::InvalidInput(
                "browser session identifier must not be empty".into(),
            ));
        }
        if trimmed.len() > MAX_BROWSER_SESSION_ID_LEN {
            return Err(Error::InvalidInput(format!(
                "browser session identifier length ({}) exceeds maximum permitted ({})",
                trimmed.len(),
                MAX_BROWSER_SESSION_ID_LEN
            )));
        }
        if trimmed == "." || trimmed == ".." {
            return Err(Error::InvalidInput(
                "browser session identifier must not be a path traversal token ('.' or '..')"
                    .into(),
            ));
        }
        for ch in trimmed.chars() {
            if !ch.is_ascii_alphanumeric() && ch != '-' && ch != '_' {
                return Err(Error::InvalidInput(format!(
                    "browser session identifier contains forbidden character '{ch}'; only [a-zA-Z0-9-_] are permitted"
                )));
            }
        }
        Ok(Self(trimmed.to_string()))
    }

    /// Returns the session identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for BrowserSessionId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for BrowserSessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for BrowserSessionId {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(s)
    }
}

impl Serialize for BrowserSessionId {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for BrowserSessionId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Self::new(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_ids_are_accepted() {
        assert!(BrowserSessionId::new("session-1").is_ok());
        assert!(BrowserSessionId::new("SESSION_2026_09").is_ok());
        assert!(BrowserSessionId::new("a").is_ok());
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        let id = BrowserSessionId::new("  session-trimmed  ").unwrap();
        assert_eq!(id.as_str(), "session-trimmed");
    }

    #[test]
    fn invalid_ids_are_rejected() {
        assert!(BrowserSessionId::new("").is_err());
        assert!(BrowserSessionId::new("   ").is_err());
        assert!(BrowserSessionId::new("session/sub").is_err());
        assert!(BrowserSessionId::new("session:1").is_err());
        assert!(BrowserSessionId::new("session.1").is_err());
        assert!(BrowserSessionId::new(".").is_err());
        assert!(BrowserSessionId::new("..").is_err());
    }

    #[test]
    fn length_limits_enforced() {
        let too_long = "a".repeat(65);
        assert!(BrowserSessionId::new(&too_long).is_err());

        let max_len = "a".repeat(64);
        assert!(BrowserSessionId::new(&max_len).is_ok());
    }

    #[test]
    fn serde_roundtrip() {
        let id = BrowserSessionId::new("session-abc-123").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        let parsed: BrowserSessionId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, parsed);
    }
}
