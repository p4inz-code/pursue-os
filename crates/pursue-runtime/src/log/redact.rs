//! Redaction helper for sensitive values in log output.

use serde::{Serialize, Serializer};
use std::fmt;

/// A value whose contents are never revealed by [`Display`], [`Debug`], or
/// [`Serialize`].
///
/// The wrapped contents are retained internally so the value can still be
/// compared and, if an operator explicitly chooses, unwrapped — but every
/// public output path renders the fixed marker `***`. Use for secrets,
/// credentials, private keys, and authentication tokens.
#[derive(Clone, PartialEq, Eq)]
pub struct Redacted(String);

impl Redacted {
    /// Wraps a sensitive value.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

impl fmt::Debug for Redacted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Redacted(\"***\")")
    }
}

impl fmt::Display for Redacted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

impl Serialize for Redacted {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str("***")
    }
}

/// Wraps `value` in a [`Redacted`] marker for safe logging.
pub fn redact(value: impl Into<String>) -> Redacted {
    Redacted::new(value)
}

#[cfg(test)]
mod tests {
    use super::{Redacted, redact};

    #[test]
    fn display_and_debug_never_reveal_contents() {
        let value = redact("hunter2");
        assert_eq!(format!("{value}"), "***");
        assert_eq!(format!("{value:?}"), "Redacted(\"***\")");
    }

    #[test]
    fn serialization_never_reveals_contents() {
        let json = serde_json::to_string(&Redacted::new("sk-secret-token")).unwrap();
        assert_eq!(json, "\"***\"");
        assert!(!json.contains("secret"));
    }
}
