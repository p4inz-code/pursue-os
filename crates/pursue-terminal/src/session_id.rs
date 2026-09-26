//! Validated terminal session identifiers.
//!
//! A [`SessionId`] is an immutable, validated ASCII identifier for a terminal
//! session. The charset and length bounds (`[A-Za-z0-9._-]`, trimmed length
//! 1..=64) follow the project identifier conventions and ensure that identifiers
//! are safe to use as filesystem path components and IPC route parameters without
//! risk of path traversal.

use std::fmt;

use pursue_core::{Error, Result};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Minimum length of a session identifier (after trimming).
pub const SESSION_ID_MIN_LEN: usize = 1;

/// Maximum length of a session identifier (after trimming).
pub const SESSION_ID_MAX_LEN: usize = 64;

/// A validated terminal session identifier.
///
/// Only ASCII alphanumeric characters, `.`, `_` and `-` are permitted.
/// Path traversal components (`.` and `..`) are explicitly rejected.
/// Validation runs on construction ([`SessionId::new`]) and on deserialization,
/// preventing invalid identifiers from entering through network or storage.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SessionId(String);

impl SessionId {
    /// Validates `id` and constructs a `SessionId`.
    ///
    /// Surrounding whitespace is trimmed. The identifier must satisfy:
    /// - Non-empty, length between [`SESSION_ID_MIN_LEN`] and [`SESSION_ID_MAX_LEN`].
    /// - Only characters in `[A-Za-z0-9._-]`.
    /// - Must not be `.` or `..` (path traversal guard).
    pub fn new(id: &str) -> Result<Self> {
        let id = id.trim();
        if id.len() < SESSION_ID_MIN_LEN {
            return Err(Error::InvalidInput("session id must not be empty".into()));
        }
        if id.len() > SESSION_ID_MAX_LEN {
            return Err(Error::InvalidInput(format!(
                "session id exceeds maximum length {SESSION_ID_MAX_LEN}"
            )));
        }
        if id == "." || id == ".." {
            return Err(Error::InvalidInput(format!(
                "session id {id:?} is not allowed (path traversal guard)"
            )));
        }
        let valid = id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-');
        if !valid {
            return Err(Error::InvalidInput(format!(
                "session id {id:?} contains invalid characters (allowed: A-Z a-z 0-9 . _ -)"
            )));
        }
        Ok(Self(id.to_string()))
    }

    /// Returns the session identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for SessionId {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SessionId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        SessionId::new(&s).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::{SESSION_ID_MAX_LEN, SessionId};

    #[test]
    fn valid_ids_are_accepted() {
        for id in [
            "session-1",
            "session_alpha",
            "term.01",
            "a",
            "12345",
            "sess-abc.def_ghi-jkl",
        ] {
            assert!(SessionId::new(id).is_ok(), "expected {id:?} to be valid");
        }
    }

    #[test]
    fn invalid_ids_are_rejected() {
        for id in [
            "",
            "   ",
            "session id",
            "session/id",
            "session\\id",
            "session:id",
            "session@x",
            "session,1",
            "a\nb",
            "session\x00x",
            "café",
        ] {
            assert!(SessionId::new(id).is_err(), "expected {id:?} to be invalid");
        }
    }

    #[test]
    fn path_traversal_is_explicitly_rejected() {
        assert!(SessionId::new(".").is_err());
        assert!(SessionId::new("..").is_err());
        assert!(SessionId::new(" . ").is_err());
        assert!(SessionId::new(" .. ").is_err());
    }

    #[test]
    fn length_limits_are_enforced() {
        let max = "a".repeat(SESSION_ID_MAX_LEN);
        assert!(SessionId::new(&max).is_ok());
        let over = "a".repeat(SESSION_ID_MAX_LEN + 1);
        assert!(SessionId::new(&over).is_err());
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        let id = SessionId::new("  session-1  ").unwrap();
        assert_eq!(id.as_str(), "session-1");
    }

    #[test]
    fn display_and_debug_behave_expectedly() {
        let id = SessionId::new("session-42").unwrap();
        assert_eq!(id.to_string(), "session-42");
        assert!(format!("{id:?}").contains("session-42"));
    }

    #[test]
    fn equality_and_ordering() {
        let a = SessionId::new("session-a").unwrap();
        let b = SessionId::new("session-b").unwrap();
        let a2 = SessionId::new("session-a").unwrap();
        assert_eq!(a, a2);
        assert_ne!(a, b);
        assert!(a < b);
    }

    #[test]
    fn serde_roundtrip() {
        let id = SessionId::new("session-test").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"session-test\"");
        let parsed: SessionId = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, id);
    }

    #[test]
    fn serde_rejects_invalid() {
        assert!(serde_json::from_str::<SessionId>("\"\"").is_err());
        assert!(serde_json::from_str::<SessionId>("\"..\"").is_err());
        assert!(serde_json::from_str::<SessionId>("\"bad/id\"").is_err());
        assert!(serde_json::from_str::<SessionId>("123").is_err());
    }
}
