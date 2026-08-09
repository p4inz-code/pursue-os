//! Integration tests for the `pursue-evidence` public API.

mod common;

use common::{cleanup, temp_dir};
use pursue_evidence::{
    ContentAddress, Error, EvidenceRecord, EvidenceStore, FileStore, InMemoryStore,
};

#[test]
fn in_memory_store_end_to_end() {
    let mut store = InMemoryStore::new();
    let record = store
        .put(b"first artifact", "archive.org", "investigator-1")
        .unwrap();

    assert_eq!(record.source(), "archive.org");
    assert!(store.contains(record.address()));
    assert_eq!(store.get(record.address()).unwrap(), b"first artifact");
    assert_eq!(store.record(record.address()).unwrap(), record);
    store.audit_log().verify().unwrap();
    assert_eq!(store.audit_log().len(), 1);
}

#[test]
fn file_store_persistence_and_retrieval() {
    let dir = temp_dir("persistence");
    let address;
    {
        let mut store = FileStore::open(&dir).unwrap();
        let record = store
            .put(b"evidence payload", "example.org", "investigator-1")
            .unwrap();
        address = *record.address();
        assert!(store.audit_log().verify().is_ok());
    }
    // A brand-new store object reads the same bytes back from disk.
    let store = FileStore::open(&dir).unwrap();
    assert!(store.contains(&address));
    assert_eq!(store.get(&address).unwrap(), b"evidence payload");
    let record: EvidenceRecord = store.record(&address).unwrap();
    assert_eq!(record.source(), "example.org");
    store.audit_log().verify().unwrap();
    cleanup(&dir);
}

#[test]
fn identical_content_is_deduplicated_by_address() {
    let dir = temp_dir("dedup");
    let mut store = FileStore::open(&dir).unwrap();
    let a = store.put(b"identical", "s1", "investigator-1").unwrap();
    let b = store.put(b"identical", "s2", "investigator-1").unwrap();
    assert_eq!(a.address(), b.address()); // Two provenance events, one blob.
    assert_eq!(store.audit_log().len(), 2);
    assert_eq!(store.audit_log().entries()[1].subject, Some(*a.address()));
    cleanup(&dir);
}

#[test]
fn altered_blob_on_disk_is_detected() {
    let dir = temp_dir("blob-tamper");
    let address;
    {
        let mut store = FileStore::open(&dir).unwrap();
        let record = store
            .put(b"original bytes", "example.org", "investigator-1")
            .unwrap();
        address = *record.address();
    }
    // Flip one byte in the blob file: the store must refuse to open.
    let blob_path = dir.join("blobs").join(format!("{}.bin", address.to_hex()));
    let mut bytes = std::fs::read(&blob_path).unwrap();
    bytes[0] ^= 0xff;
    std::fs::write(&blob_path, &bytes).unwrap();

    let err = FileStore::open(&dir).unwrap_err();
    assert!(
        matches!(err, Error::IntegrityViolation(_)),
        "expected IntegrityViolation, got {err}"
    );
    cleanup(&dir);
}

#[test]
fn missing_evidence_is_reported_cleanly() {
    let dir = temp_dir("missing");
    let store = FileStore::open(&dir).unwrap();
    let ghost = ContentAddress::hash(b"does not exist");
    assert!(!store.contains(&ghost));
    let err = store.get(&ghost).unwrap_err();
    assert!(
        matches!(err, Error::NotFound(_)),
        "expected NotFound, got {err}"
    );
    cleanup(&dir);
}
