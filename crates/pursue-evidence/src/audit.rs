//! Append-only, hash-chained audit log for provenance events.
//!
//! Each entry's `hash` is computed over its own contents *and* the previous
//! entry's hash, forming a chain. Altering any entry breaks every subsequent
//! link, which [`AuditLog::verify`] detects.

use pursue_core::{Error, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::ContentAddress;

/// A single audit entry in the hash chain.
///
/// Fields are public because this is a data record; the chain's integrity is
/// guaranteed by [`AuditLog::verify`], which detects any alteration.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    /// Monotonic sequence number (0-based, matches position in the log).
    pub seq: u64,
    /// Unix timestamp (seconds) of the event.
    pub timestamp_unix: u64,
    /// Actor responsible for the event (e.g., investigator id, "system").
    pub actor: String,
    /// Action performed (e.g., "acquired", "analyzed", "exported").
    pub action: String,
    /// Optional evidence subject this event refers to.
    pub subject: Option<ContentAddress>,
    /// Hash of the previous entry; `None` only for the first entry.
    pub prev_hash: Option<ContentAddress>,
    /// This entry's own chain hash.
    pub hash: ContentAddress,
}

/// The chain-hash function: SHA-256 over the entry fields and the previous
/// entry's hash, with length-ambiguous fields separated by `0x00`.
fn chain_hash(
    seq: u64,
    timestamp_unix: u64,
    actor: &str,
    action: &str,
    subject: Option<ContentAddress>,
    prev_hash: Option<ContentAddress>,
) -> ContentAddress {
    let mut hasher = Sha256::new();
    hasher.update(seq.to_le_bytes());
    hasher.update(timestamp_unix.to_le_bytes());
    hasher.update(actor.as_bytes());
    hasher.update([0u8]);
    hasher.update(action.as_bytes());
    hasher.update([0u8]);
    if let Some(subject) = subject {
        hasher.update(subject.to_hex().as_bytes());
    }
    hasher.update([0u8]);
    if let Some(prev) = prev_hash {
        hasher.update(prev.to_hex().as_bytes());
    }
    hasher.update([0u8]);
    let digest: [u8; 32] = hasher.finalize().into();
    ContentAddress::from_digest(digest)
}

/// An append-only audit log.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AuditLog {
    entries: Vec<AuditEntry>,
}

impl AuditLog {
    /// Creates an empty audit log.
    pub fn new() -> Self {
        Self::default()
    }

    /// Appends an event and returns its entry. Rejects empty actor/action.
    pub fn append(
        &mut self,
        timestamp_unix: u64,
        actor: &str,
        action: &str,
        subject: Option<ContentAddress>,
    ) -> Result<&AuditEntry> {
        let actor = actor.trim();
        let action = action.trim();
        if actor.is_empty() {
            return Err(Error::InvalidInput("audit actor must not be empty".into()));
        }
        if action.is_empty() {
            return Err(Error::InvalidInput("audit action must not be empty".into()));
        }
        if actor.contains('\0') || action.contains('\0') {
            return Err(Error::InvalidInput(
                "audit actor and action must not contain NUL bytes".into(),
            ));
        }
        let seq = self.entries.len() as u64;
        let prev_hash = self.entries.last().map(|entry| entry.hash);
        let hash = chain_hash(seq, timestamp_unix, actor, action, subject, prev_hash);
        self.entries.push(AuditEntry {
            seq,
            timestamp_unix,
            actor: actor.to_string(),
            action: action.to_string(),
            subject,
            prev_hash,
            hash,
        });
        Ok(self.entries.last().expect("just appended"))
    }

    /// Verifies the entire chain: sequence order, every recomputed hash, and
    /// every previous-link. Returns [`Error::IntegrityViolation`] on any
    /// alteration.
    pub fn verify(&self) -> Result<()> {
        let mut prev: Option<ContentAddress> = None;
        for (index, entry) in self.entries.iter().enumerate() {
            let expected_seq = index as u64;
            if entry.seq != expected_seq {
                return Err(Error::IntegrityViolation(format!(
                    "sequence mismatch at index {index}: expected {expected_seq}, found {}",
                    entry.seq
                )));
            }
            if entry.prev_hash != prev {
                return Err(Error::IntegrityViolation(format!(
                    "previous-link mismatch at seq {}",
                    entry.seq
                )));
            }
            let expected_hash = chain_hash(
                entry.seq,
                entry.timestamp_unix,
                &entry.actor,
                &entry.action,
                entry.subject,
                prev,
            );
            if expected_hash != entry.hash {
                return Err(Error::IntegrityViolation(format!(
                    "hash mismatch at seq {}",
                    entry.seq
                )));
            }
            prev = Some(entry.hash);
        }
        Ok(())
    }

    /// Returns all entries in append order.
    pub fn entries(&self) -> &[AuditEntry] {
        &self.entries
    }

    /// Number of entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the log is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[cfg(test)]
    fn entries_mut(&mut self) -> &mut Vec<AuditEntry> {
        &mut self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::AuditLog;
    use crate::ContentAddress;

    fn subject(n: u8) -> ContentAddress {
        ContentAddress::hash(&[n])
    }

    #[test]
    fn empty_log_verifies() {
        let log = AuditLog::new();
        assert!(log.is_empty());
        log.verify().unwrap();
    }

    #[test]
    fn appends_chain_and_verify_passes() {
        let mut log = AuditLog::new();
        log.append(1, "alice", "acquired", Some(subject(1)))
            .unwrap();
        log.append(2, "alice", "analyzed", Some(subject(1)))
            .unwrap();
        log.append(3, "bob", "exported", None).unwrap();
        assert_eq!(log.len(), 3);
        log.verify().unwrap();
    }

    #[test]
    fn sequence_is_monotonic() {
        let mut log = AuditLog::new();
        for i in 0..5 {
            log.append(i as u64, "system", "tick", None).unwrap();
        }
        let seqs: Vec<u64> = log.entries().iter().map(|e| e.seq).collect();
        assert_eq!(seqs, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn prev_hash_links_to_previous_entry() {
        let mut log = AuditLog::new();
        let first = log
            .append(1, "alice", "acquired", Some(subject(1)))
            .unwrap()
            .clone();
        let second = log.append(2, "alice", "analyzed", None).unwrap().clone();
        assert_eq!(first.prev_hash, None);
        assert_eq!(second.prev_hash, Some(first.hash));
    }

    #[test]
    fn altered_action_is_detected() {
        let mut log = AuditLog::new();
        log.append(1, "alice", "acquired", Some(subject(1)))
            .unwrap();
        log.append(2, "alice", "analyzed", None).unwrap();
        // Simulate tampering with an entry already in the chain.
        log.entries_mut()[1].action = "tampered".to_string();
        let err = log.verify().unwrap_err();
        assert!(err.to_string().contains("hash mismatch"));
    }

    #[test]
    fn altered_prev_link_is_detected() {
        let mut log = AuditLog::new();
        log.append(1, "alice", "acquired", Some(subject(1)))
            .unwrap();
        log.append(2, "alice", "analyzed", None).unwrap();
        log.entries_mut()[1].prev_hash = Some(subject(9));
        let err = log.verify().unwrap_err();
        assert!(err.to_string().contains("previous-link"));
    }

    #[test]
    fn altered_sequence_is_detected() {
        let mut log = AuditLog::new();
        log.append(1, "alice", "acquired", None).unwrap();
        log.entries_mut()[0].seq = 99;
        let err = log.verify().unwrap_err();
        assert!(err.to_string().contains("sequence mismatch"));
    }

    #[test]
    fn empty_actor_and_action_are_rejected() {
        let mut log = AuditLog::new();
        assert!(log.append(1, "", "acquired", None).is_err());
        assert!(log.append(1, "alice", "", None).is_err());
        assert!(log.append(1, "  ", "acquired", None).is_err());
    }

    #[test]
    fn nul_bytes_in_actor_or_action_are_rejected() {
        let mut log = AuditLog::new();
        assert!(log.append(1, "a\0b", "acquired", None).is_err());
        assert!(log.append(1, "alice", "acqui\0red", None).is_err());
    }

    #[test]
    fn serde_roundtrip_preserves_chain() {
        let mut log = AuditLog::new();
        log.append(1, "alice", "acquired", Some(subject(1)))
            .unwrap();
        log.append(2, "bob", "analyzed", None).unwrap();
        let json = serde_json::to_string(&log).unwrap();
        let back: AuditLog = serde_json::from_str(&json).unwrap();
        assert_eq!(back, log);
        back.verify().unwrap();
    }
}
