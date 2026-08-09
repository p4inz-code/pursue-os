//! Evidence stores with verification on every read.
//!
//! Both implementations re-hash content on `get` and fail with
//! [`Error::IntegrityViolation`] if bytes do not match their content address.
//! The file-backed store additionally verifies the whole audit chain and every
//! blob when it is opened.

use pursue_core::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::audit::AuditLog;
use crate::content_address::ContentAddress;
use crate::record::EvidenceRecord;

/// The audit action recorded when evidence is stored.
pub const ACTION_ACQUIRED: &str = "acquired";

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Common evidence-store operations.
pub trait EvidenceStore {
    /// Stores `data` as evidence, recording a `acquired` provenance event for
    /// `actor`. Returns the immutable evidence record.
    fn put(&mut self, data: &[u8], source: &str, actor: &str) -> Result<EvidenceRecord>;

    /// Returns the stored bytes, re-verifying that they hash to their content
    /// address. Errors with [`Error::IntegrityViolation`] on tampering.
    fn get(&self, address: &ContentAddress) -> Result<Vec<u8>>;

    /// Whether evidence with this address is present.
    fn contains(&self, address: &ContentAddress) -> bool;

    /// Returns the record for an address without reading the blob.
    fn record(&self, address: &ContentAddress) -> Result<EvidenceRecord>;

    /// The store's audit log (provenance events).
    fn audit_log(&self) -> &AuditLog;
}

/// An in-memory evidence store. Suitable for tests and small workloads.
#[derive(Default)]
pub struct InMemoryStore {
    blobs: HashMap<ContentAddress, Vec<u8>>,
    records: HashMap<ContentAddress, EvidenceRecord>,
    audit: AuditLog,
}

impl InMemoryStore {
    /// Creates an empty store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl EvidenceStore for InMemoryStore {
    fn put(&mut self, data: &[u8], source: &str, actor: &str) -> Result<EvidenceRecord> {
        let address = ContentAddress::hash(data);
        let record = EvidenceRecord::new(address, data.len() as u64, now_unix(), source)?;
        self.audit
            .append(now_unix(), actor, ACTION_ACQUIRED, Some(address))?;
        self.blobs.insert(address, data.to_vec());
        self.records.insert(address, record.clone());
        Ok(record)
    }

    fn get(&self, address: &ContentAddress) -> Result<Vec<u8>> {
        let blob = self
            .blobs
            .get(address)
            .ok_or_else(|| Error::NotFound(format!("evidence {address} not found")))?;
        let actual = ContentAddress::hash(blob);
        if &actual != address {
            return Err(Error::IntegrityViolation(format!(
                "stored bytes for {address} no longer match their content address"
            )));
        }
        Ok(blob.clone())
    }

    fn contains(&self, address: &ContentAddress) -> bool {
        self.blobs.contains_key(address)
    }

    fn record(&self, address: &ContentAddress) -> Result<EvidenceRecord> {
        self.records
            .get(address)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("record for {address} not found")))
    }

    fn audit_log(&self) -> &AuditLog {
        &self.audit
    }
}

/// Serialized store state persisted by [`FileStore`].
#[derive(Serialize, Deserialize)]
struct Manifest {
    version: u32,
    records: Vec<EvidenceRecord>,
    audit: AuditLog,
}

/// A file-backed evidence store.
///
/// Layout under the root directory:
/// - `manifest.json` — records and the audit log (audit chain verified on open).
/// - `blobs/<hex>.bin` — artifact bytes, verified on every read.
#[derive(Debug)]
pub struct FileStore {
    dir: PathBuf,
    records: HashMap<ContentAddress, EvidenceRecord>,
    audit: AuditLog,
}

impl FileStore {
    /// Opens the store rooted at `dir`, creating it if needed.
    ///
    /// On open, the audit chain and every blob are verified; any corruption or
    /// tampering yields [`Error::IntegrityViolation`].
    pub fn open(dir: &Path) -> Result<Self> {
        fs::create_dir_all(dir)?;
        let blobs_dir = dir.join("blobs");
        fs::create_dir_all(&blobs_dir)?;

        let manifest_path = dir.join("manifest.json");
        if !manifest_path.exists() {
            return Ok(Self {
                dir: dir.to_path_buf(),
                records: HashMap::new(),
                audit: AuditLog::new(),
            });
        }

        let text = fs::read_to_string(&manifest_path)?;
        let manifest: Manifest = serde_json::from_str(&text).map_err(|e| {
            Error::InvalidInput(format!(
                "corrupt manifest at {}: {e}",
                manifest_path.display()
            ))
        })?;
        if manifest.version != 1 {
            return Err(Error::InvalidInput(format!(
                "unsupported manifest version {}",
                manifest.version
            )));
        }
        // Integrity gate: the audit chain must verify before the store is usable.
        manifest.audit.verify().map_err(|e| {
            Error::IntegrityViolation(format!("audit log failed verification on load: {e}"))
        })?;

        let mut records = HashMap::with_capacity(manifest.records.len());
        for record in manifest.records {
            records.insert(*record.address(), record);
        }
        // Integrity gate: every recorded blob must exist and hash correctly.
        for address in records.keys() {
            let blob = fs::read(Self::blob_path(dir, address))
                .map_err(|_| Error::NotFound(format!("blob for {address} missing from store")))?;
            if ContentAddress::hash(&blob) != *address {
                return Err(Error::IntegrityViolation(format!(
                    "blob for {address} failed content verification on load"
                )));
            }
        }

        Ok(Self {
            dir: dir.to_path_buf(),
            records,
            audit: manifest.audit,
        })
    }

    fn blob_path(dir: &Path, address: &ContentAddress) -> PathBuf {
        dir.join("blobs").join(format!("{}.bin", address.to_hex()))
    }

    fn persist(&self) -> Result<()> {
        let manifest = Manifest {
            version: 1,
            records: self.records.values().cloned().collect(),
            audit: self.audit.clone(),
        };
        let text = serde_json::to_string_pretty(&manifest)
            .map_err(|e| Error::InvalidInput(format!("manifest serialization failed: {e}")))?;
        fs::write(self.dir.join("manifest.json"), text)?;
        Ok(())
    }
}

impl EvidenceStore for FileStore {
    fn put(&mut self, data: &[u8], source: &str, actor: &str) -> Result<EvidenceRecord> {
        let address = ContentAddress::hash(data);
        let record = EvidenceRecord::new(address, data.len() as u64, now_unix(), source)?;
        self.audit
            .append(now_unix(), actor, ACTION_ACQUIRED, Some(address))?;
        // Blob first, manifest last: a crash in between leaves an orphan blob,
        // never a manifest pointing at missing data.
        fs::write(Self::blob_path(&self.dir, &address), data)?;
        self.records.insert(address, record.clone());
        self.persist()?;
        Ok(record)
    }

    fn get(&self, address: &ContentAddress) -> Result<Vec<u8>> {
        let path = Self::blob_path(&self.dir, address);
        let blob = fs::read(&path)
            .map_err(|_| Error::NotFound(format!("evidence {address} not found")))?;
        let actual = ContentAddress::hash(&blob);
        if &actual != address {
            return Err(Error::IntegrityViolation(format!(
                "stored bytes for {address} no longer match their content address"
            )));
        }
        Ok(blob)
    }

    fn contains(&self, address: &ContentAddress) -> bool {
        Self::blob_path(&self.dir, address).exists()
    }

    fn record(&self, address: &ContentAddress) -> Result<EvidenceRecord> {
        self.records
            .get(address)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("record for {address} not found")))
    }

    fn audit_log(&self) -> &AuditLog {
        &self.audit
    }
}

#[cfg(test)]
mod tests {
    use super::{ACTION_ACQUIRED, EvidenceStore, FileStore, InMemoryStore};
    use crate::ContentAddress;

    #[test]
    fn in_memory_put_get_roundtrip() {
        let mut store = InMemoryStore::new();
        let data = b"artifact bytes";
        let record = store.put(data, "source-a", "alice").unwrap();
        assert_eq!(store.get(record.address()).unwrap(), data);
        assert!(store.contains(record.address()));
        assert_eq!(store.record(record.address()).unwrap(), record);
        assert_eq!(record.source(), "source-a");
        assert_eq!(record.size(), data.len() as u64);
    }

    #[test]
    fn content_addressing_is_deterministic() {
        let mut store = InMemoryStore::new();
        let a = store.put(b"same bytes", "s1", "alice").unwrap();
        let b = store.put(b"same bytes", "s2", "bob").unwrap();
        assert_eq!(a.address(), b.address());
    }

    #[test]
    fn missing_evidence_returns_not_found() {
        let store = InMemoryStore::new();
        let addr = ContentAddress::hash(b"ghost");
        assert!(!store.contains(&addr));
        assert!(store.get(&addr).is_err());
        assert!(store.record(&addr).is_err());
    }

    #[test]
    fn empty_source_is_rejected_and_nothing_is_stored() {
        let mut store = InMemoryStore::new();
        assert!(store.put(b"data", "  ", "alice").is_err());
        assert!(store.put(b"data", "", "alice").is_err());
        assert_eq!(store.audit_log().len(), 0);
    }

    #[test]
    fn every_put_records_an_acquired_audit_event() {
        let mut store = InMemoryStore::new();
        store.put(b"a", "s1", "alice").unwrap();
        store.put(b"b", "s2", "bob").unwrap();
        let log = store.audit_log();
        assert_eq!(log.len(), 2);
        log.verify().unwrap();
        assert_eq!(log.entries()[0].action, ACTION_ACQUIRED);
        assert_eq!(log.entries()[0].actor, "alice");
    }

    #[test]
    fn file_store_roundtrip_across_reload() {
        let dir = temp_dir("roundtrip");
        {
            let mut store = FileStore::open(&dir).unwrap();
            let record = store.put(b"persist me", "web", "alice").unwrap();
            assert_eq!(store.get(record.address()).unwrap(), b"persist me");
            assert!(store.audit_log().verify().is_ok());
        }
        // Reload from disk: everything must still verify.
        let store = FileStore::open(&dir).unwrap();
        assert_eq!(store.audit_log().len(), 1);
        store.audit_log().verify().unwrap();
        let addr = ContentAddress::hash(b"persist me");
        assert!(store.contains(&addr));
        assert_eq!(store.get(&addr).unwrap(), b"persist me");
        let _ = fs_remove(&dir);
    }

    #[test]
    fn file_store_detects_blob_tampering() {
        let dir = temp_dir("tampered-blob");
        let addr;
        {
            let mut store = FileStore::open(&dir).unwrap();
            let record = store.put(b"tamper me", "web", "alice").unwrap();
            addr = *record.address();
        }
        // Corrupt the blob on disk: the store must refuse to open.
        let blob = dir.join("blobs").join(format!("{}.bin", addr.to_hex()));
        std::fs::write(&blob, b"tampered!").unwrap();
        let err = FileStore::open(&dir).unwrap_err();
        assert!(err.to_string().contains("integrity violation"));
        let _ = fs_remove(&dir);
    }

    #[test]
    fn file_store_detects_tampered_manifest_on_load() {
        let dir = temp_dir("tampered-manifest");
        {
            let mut store = FileStore::open(&dir).unwrap();
            store.put(b"chain me", "web", "alice").unwrap();
        }
        // Tamper with the manifest: rewrite an audit actor.
        let manifest_path = dir.join("manifest.json");
        let text = std::fs::read_to_string(&manifest_path).unwrap();
        let tampered = text.replace("\"actor\": \"alice\"", "\"actor\": \"mallory\"");
        assert_ne!(text, tampered);
        std::fs::write(&manifest_path, tampered).unwrap();
        let err = FileStore::open(&dir).unwrap_err();
        assert!(err.to_string().contains("integrity violation"));
        let _ = fs_remove(&dir);
    }

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let unique = format!(
            "pursue-test-{label}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        std::env::temp_dir().join(unique)
    }

    fn fs_remove(dir: &std::path::Path) -> std::io::Result<()> {
        std::fs::remove_dir_all(dir)
    }
}
