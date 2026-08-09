//! Immutable evidence records.

use pursue_core::{Error, Result};
use serde::{Deserialize, Serialize};

use crate::ContentAddress;

/// A recorded evidence artifact.
///
/// Records are immutable: all fields are private and exposed through getters.
/// Integrity is anchored to the content address; size is fixed at
/// construction from the stored byte length.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceRecord {
    address: ContentAddress,
    size: u64,
    acquired_at_unix: u64,
    source: String,
}

impl EvidenceRecord {
    /// Creates a record. `source` must be non-empty after trimming.
    ///
    /// `data_len` is the byte length of the stored artifact — it is fixed at
    /// construction and never inferred from user input elsewhere.
    pub fn new(
        address: ContentAddress,
        data_len: u64,
        acquired_at_unix: u64,
        source: &str,
    ) -> Result<Self> {
        let source = source.trim();
        if source.is_empty() {
            return Err(Error::InvalidInput(
                "evidence source must not be empty".into(),
            ));
        }
        Ok(Self {
            address,
            size: data_len,
            acquired_at_unix,
            source: source.to_string(),
        })
    }

    /// The content address identifying this evidence.
    pub fn address(&self) -> &ContentAddress {
        &self.address
    }

    /// The recorded byte length of the artifact.
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Unix timestamp (seconds) when the evidence was recorded.
    pub fn acquired_at_unix(&self) -> u64 {
        self.acquired_at_unix
    }

    /// The investigator-supplied source label.
    pub fn source(&self) -> &str {
        &self.source
    }
}

#[cfg(test)]
mod tests {
    use super::EvidenceRecord;
    use crate::ContentAddress;

    fn sample() -> ContentAddress {
        ContentAddress::hash(b"payload")
    }

    #[test]
    fn construction_records_fields() {
        let record = EvidenceRecord::new(sample(), 7, 1_700_000_000, " web.archive.org ").unwrap();
        assert_eq!(record.address(), &sample());
        assert_eq!(record.size(), 7);
        assert_eq!(record.acquired_at_unix(), 1_700_000_000);
        assert_eq!(record.source(), "web.archive.org"); // trimmed
    }

    #[test]
    fn empty_source_is_rejected() {
        assert!(EvidenceRecord::new(sample(), 0, 1, "").is_err());
        assert!(EvidenceRecord::new(sample(), 0, 1, "   ").is_err());
    }

    #[test]
    fn serde_roundtrip_preserves_record() {
        let record = EvidenceRecord::new(sample(), 7, 1_700_000_000, "source").unwrap();
        let json = serde_json::to_string(&record).unwrap();
        let back: EvidenceRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(back, record);
    }
}
