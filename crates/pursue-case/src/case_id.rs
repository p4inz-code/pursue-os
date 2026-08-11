//! Validated case identifiers.
//!
//! A [`CaseId`] is an immutable, validated ASCII identifier for a case. The
//! charset and length are locked by the Phase 1D boundary
//! (`docs/development/CASE_FOUNDATION.md`): `[A-Za-z0-9._-]`, trimmed length
//! 1..=128. The restricted charset makes identifiers safe to use as
//! filesystem path components, which the per-case storage layout (case
//! isolation) depends on.

use std::fmt;

use pursue_core::{Error, Result};
use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Minimum length of a case identifier (after trimming).
pub const CASE_ID_MIN_LEN: usize = 1;

/// Maximum length of a case identifier (after trimming).
pub const CASE_ID_MAX_LEN: usize = 128;

/// A validated case identifier.
///
/// Only ASCII letters, digits, `.`, `_` and `-` are permitted, and the
/// trimmed length must be between [`CASE_ID_MIN_LEN`] and
/// [`CASE_ID_MAX_LEN`]. Validation runs both on construction ([`CaseId::new`])
/// and on deserialization, so invalid identifiers cannot enter the system
/// through a wire or serialized format.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CaseId(String);

impl CaseId {
    /// Validates `id` and constructs a `CaseId`.
    ///
    /// Surrounding whitespace is trimmed, then the identifier must be non-empty,
    /// at most [`CASE_ID_MAX_LEN`] characters, and contain only ASCII letters,
    /// digits, `.`, `_` and `-`.
    pub fn new(id: &str) -> Result<Self> {
        let id = id.trim();
        if id.len() < CASE_ID_MIN_LEN {
            return Err(Error::InvalidInput("case id must not be empty".into()));
        }
        if id.len() > CASE_ID_MAX_LEN {
            return Err(Error::InvalidInput(format!(
                "case id exceeds maximum length {CASE_ID_MAX_LEN}"
            )));
        }
        let valid = id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-');
        if !valid {
            return Err(Error::InvalidInput(format!(
                "case id {id:?} contains invalid characters (allowed: A-Z a-z 0-9 . _ -)"
            )));
        }
        Ok(Self(id.to_string()))
    }

    /// Returns the identifier as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for CaseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl Serialize for CaseId {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for CaseId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        CaseId::new(&s).map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::{CASE_ID_MAX_LEN, CaseId};

    #[test]
    fn valid_ids_are_accepted() {
        for id in [
            "case-1",
            "case_1",
            "Case.ABC-1",
            "a",
            "12345",
            "case-abc.def_ghi-jkl",
        ] {
            assert!(CaseId::new(id).is_ok(), "expected {id:?} to be valid");
        }
    }

    #[test]
    fn invalid_ids_are_rejected() {
        for id in [
            "",
            "   ",
            "case id",
            "case/id",
            "case\\id",
            "case:id",
            "case@x",
            "case,1",
            "a\nb",
            "café",
            "case\x00x",
        ] {
            assert!(CaseId::new(id).is_err(), "expected {id:?} to be invalid");
        }
    }

    #[test]
    fn length_limits_are_enforced() {
        let max = "a".repeat(CASE_ID_MAX_LEN);
        assert!(CaseId::new(&max).is_ok());
        let over = "a".repeat(CASE_ID_MAX_LEN + 1);
        assert!(CaseId::new(&over).is_err());
    }

    #[test]
    fn surrounding_whitespace_is_trimmed() {
        let id = CaseId::new("  case-1  ").unwrap();
        assert_eq!(id.as_str(), "case-1");
    }

    #[test]
    fn display_uses_canonical_form() {
        let id = CaseId::new("case-1").unwrap();
        assert_eq!(id.to_string(), "case-1");
    }

    #[test]
    fn equality_is_value_based() {
        assert_eq!(
            CaseId::new("case-1").unwrap(),
            CaseId::new("case-1").unwrap()
        );
        assert_ne!(
            CaseId::new("case-1").unwrap(),
            CaseId::new("case-2").unwrap()
        );
    }

    #[test]
    fn ordering_is_deterministic() {
        assert!(CaseId::new("case-1").unwrap() < CaseId::new("case-2").unwrap());
    }

    #[test]
    fn hash_is_stable() {
        use std::collections::HashSet;
        let a = CaseId::new("case-1").unwrap();
        let b = CaseId::new("case-1").unwrap();
        let set: HashSet<CaseId> = HashSet::from([a, b]);
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn serde_roundtrip_as_string() {
        let id = CaseId::new("case-1").unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"case-1\"");
        let back: CaseId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn serde_rejects_invalid() {
        assert!(serde_json::from_str::<CaseId>("\"bad id\"").is_err());
        assert!(serde_json::from_str::<CaseId>("\"\"").is_err());
        assert!(serde_json::from_str::<CaseId>("\"case/id\"").is_err());
        assert!(serde_json::from_str::<CaseId>("42").is_err());
    }
}
