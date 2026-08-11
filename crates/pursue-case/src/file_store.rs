//! File-backed case store with per-case isolation and verification on load.
//!
//! Layout under the store root:
//! - `cases/<case_id>/case.json` — a version-gated manifest holding the
//!   serialized [`Case`] plus `case_hash`, the content address (SHA-256) of
//!   its canonical serialization.
//! - `cases/<case_id>/evidence/` — the case's evidence store, an existing
//!   `pursue-evidence` [`FileStore`] reused unchanged (no evidence-storage
//!   logic is duplicated here).
//!
//! Every load verifies, in order: manifest format/version, the case's
//! integrity hash, the audit chain, and the evidence store (blobs + audit
//! chain, re-verified on open). Any mismatch — tampered metadata, a rewritten
//! audit entry, a corrupted blob, or a dangling evidence reference — fails
//! closed with [`Error::IntegrityViolation`] (or [`Error::NotFound`] for
//! missing state). Case ids are validated by [`CaseId`] and additionally
//! guarded against the `.` and `..` path components, so one case can never
//! address another case's directory.

use std::fs;
use std::path::{Path, PathBuf};

use pursue_core::{Error, Result};
use pursue_evidence::{AuditLog, ContentAddress, EvidenceStore, FileStore};
use serde::{Deserialize, Serialize};

use crate::{Case, CaseId, CaseStore};

/// The current case manifest format version.
const MANIFEST_VERSION: u32 = 1;

/// A case manifest: a version envelope around the serialized case plus an
/// integrity hash over its canonical serialization.
///
/// `case_hash` anchors the *entire* case — metadata, status, evidence
/// membership, and audit history. This makes tampering with any field
/// detectable, including fields the audit chain alone does not cover
/// (e.g., a silently edited title without a corresponding audit event).
#[derive(Serialize, Deserialize)]
struct CaseManifest {
    version: u32,
    case: Case,
    case_hash: ContentAddress,
}

/// A file-backed case store.
///
/// Create with [`FileCaseStore::open`]. Cases are verified individually when
/// loaded; [`FileCaseStore::open_evidence_store`] opens the (verified)
/// evidence store of a case.
#[derive(Debug)]
pub struct FileCaseStore {
    root: PathBuf,
}

impl FileCaseStore {
    /// Opens the store rooted at `dir`, creating the `cases/` directory if
    /// needed. Existing cases are verified individually when loaded.
    pub fn open(dir: &Path) -> Result<Self> {
        fs::create_dir_all(dir.join("cases"))?;
        Ok(Self {
            root: dir.to_path_buf(),
        })
    }

    /// Opens (and verifies) the evidence store of a case.
    ///
    /// The evidence store is the existing `pursue-evidence` `FileStore` under
    /// `cases/<case_id>/evidence/`; blobs and its audit chain are re-verified
    /// on open, and the returned store can be used to record new evidence
    /// before attaching it to the case.
    pub fn open_evidence_store(&self, id: &CaseId) -> Result<FileStore> {
        let dir = self.evidence_dir(id)?;
        FileStore::open(&dir)
    }

    fn case_dir(&self, id: &CaseId) -> Result<PathBuf> {
        if id.as_str() == "." || id.as_str() == ".." {
            return Err(Error::InvalidInput(format!(
                "case id {:?} is not a valid directory name",
                id.as_str()
            )));
        }
        Ok(self.root.join("cases").join(id.as_str()))
    }

    fn manifest_path(&self, id: &CaseId) -> Result<PathBuf> {
        Ok(self.case_dir(id)?.join("case.json"))
    }

    fn evidence_dir(&self, id: &CaseId) -> Result<PathBuf> {
        Ok(self.case_dir(id)?.join("evidence"))
    }

    fn persist(&self, case: &Case) -> Result<()> {
        let canonical = serde_json::to_vec(case)
            .map_err(|e| Error::InvalidInput(format!("case serialization failed: {e}")))?;
        let manifest = CaseManifest {
            version: MANIFEST_VERSION,
            case: case.clone(),
            case_hash: ContentAddress::hash(&canonical),
        };
        let text = serde_json::to_string_pretty(&manifest)
            .map_err(|e| Error::InvalidInput(format!("case manifest serialization failed: {e}")))?;
        fs::write(self.manifest_path(case.id())?, text)?;
        Ok(())
    }

    fn read_manifest(&self, id: &CaseId) -> Result<CaseManifest> {
        let path = self.manifest_path(id)?;
        let text = fs::read_to_string(&path)
            .map_err(|_| Error::NotFound(format!("case {id} not found")))?;
        let manifest: CaseManifest = serde_json::from_str(&text).map_err(|e| {
            Error::InvalidInput(format!("corrupt case manifest at {}: {e}", path.display()))
        })?;
        if manifest.version != MANIFEST_VERSION {
            return Err(Error::InvalidInput(format!(
                "unsupported case manifest version {}",
                manifest.version
            )));
        }
        // Integrity gate: the whole case (metadata, status, membership,
        // audit history) must hash to the recorded value.
        let canonical = serde_json::to_vec(&manifest.case)
            .map_err(|e| Error::InvalidInput(format!("case serialization failed: {e}")))?;
        if ContentAddress::hash(&canonical) != manifest.case_hash {
            return Err(Error::IntegrityViolation(format!(
                "case {id} failed integrity verification on load"
            )));
        }
        // Integrity gate: the audit chain itself must verify.
        manifest.case.audit_log().verify().map_err(|e| {
            Error::IntegrityViolation(format!("audit log failed verification on load: {e}"))
        })?;
        Ok(manifest)
    }
}

impl CaseStore for FileCaseStore {
    fn create_case(&mut self, id: CaseId, title: &str, created_by: &str) -> Result<Case> {
        let dir = self.case_dir(&id)?;
        if dir.exists() {
            return Err(Error::InvalidInput(format!("case {id} already exists")));
        }
        let case = Case::new(id, title, created_by)?;
        // Create the case's evidence store first; a filesystem failure here
        // fails closed before any case manifest exists.
        FileStore::open(&dir.join("evidence"))?;
        self.persist(&case)?;
        Ok(case)
    }

    fn load_case(&self, id: &CaseId) -> Result<Case> {
        let manifest = self.read_manifest(id)?;
        // Integrity gate: every referenced evidence address must exist in the
        // case's evidence store, which itself re-verifies blobs and its audit
        // chain on open. Dangling references mean the persisted state is
        // inconsistent and must fail closed.
        let evidence_dir = self.evidence_dir(id)?;
        let evidence = FileStore::open(&evidence_dir)?;
        for address in manifest.case.evidence_addresses() {
            if !evidence.contains(address) {
                return Err(Error::IntegrityViolation(format!(
                    "case {} references evidence {address} missing from its evidence store",
                    manifest.case.id()
                )));
            }
        }
        Ok(manifest.case)
    }

    fn save_case(&mut self, case: &Case) -> Result<()> {
        // A case exists iff its manifest exists (same notion as
        // `contains_case`); a bare directory is not a case. Both path
        // helpers apply the `case_dir` guard.
        if !self.manifest_path(case.id())?.exists() {
            return Err(Error::NotFound(format!("case {} not found", case.id())));
        }
        // Never persist a broken audit chain.
        case.audit_log().verify()?;
        // Never persist dangling evidence references.
        let evidence_dir = self.evidence_dir(case.id())?;
        let evidence = FileStore::open(&evidence_dir)?;
        for address in case.evidence_addresses() {
            if !evidence.contains(address) {
                return Err(Error::NotFound(format!(
                    "evidence {address} is not stored in case {}'s evidence store",
                    case.id()
                )));
            }
        }
        self.persist(case)
    }

    fn contains_case(&self, id: &CaseId) -> bool {
        matches!(self.manifest_path(id), Ok(path) if path.exists())
    }

    fn audit_log(&self, id: &CaseId) -> Result<AuditLog> {
        Ok(self.load_case(id)?.audit_log().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ACTION_EVIDENCE_ATTACHED, ACTION_REOPENED, CaseStatus};

    fn id(n: u32) -> CaseId {
        CaseId::new(&format!("case-{n}")).unwrap()
    }

    fn sample() -> ContentAddress {
        ContentAddress::hash(b"payload")
    }

    fn temp_dir(label: &str) -> PathBuf {
        let unique = format!(
            "pursue-case-store-{label}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        std::env::temp_dir().join(unique)
    }

    fn cleanup(dir: &Path) {
        let _ = std::fs::remove_dir_all(dir);
    }

    fn blob_path(dir: &Path, addr: &ContentAddress) -> PathBuf {
        dir.join("cases")
            .join("case-1")
            .join("evidence")
            .join("blobs")
            .join(format!("{}.bin", addr.to_hex()))
    }

    #[test]
    fn create_load_roundtrip_across_reopen() {
        let dir = temp_dir("roundtrip");
        let created;
        {
            let mut store = FileCaseStore::open(&dir).unwrap();
            created = store
                .create_case(id(1), "Operation Alpha", "investigator-1")
                .unwrap();
            assert!(store.contains_case(&id(1)));
        }
        let store = FileCaseStore::open(&dir).unwrap();
        let loaded = store.load_case(&id(1)).unwrap();
        assert_eq!(loaded, created);
        assert_eq!(loaded.title(), "Operation Alpha");
        assert_eq!(loaded.status(), CaseStatus::Open);
        loaded.audit_log().verify().unwrap();
        cleanup(&dir);
    }

    #[test]
    fn metadata_persistence() {
        let dir = temp_dir("metadata");
        {
            let mut store = FileCaseStore::open(&dir).unwrap();
            let mut case = store.create_case(id(1), "old", "alice").unwrap();
            case.set_title("new title", "alice").unwrap();
            case.set_notes("investigator notes", "bob").unwrap();
            store.save_case(&case).unwrap();
        }
        let store = FileCaseStore::open(&dir).unwrap();
        let loaded = store.load_case(&id(1)).unwrap();
        assert_eq!(loaded.title(), "new title");
        assert_eq!(loaded.notes(), "investigator notes");
        assert_eq!(loaded.audit_log().len(), 3);
        loaded.audit_log().verify().unwrap();
        cleanup(&dir);
    }

    #[test]
    fn evidence_reference_persistence() {
        let dir = temp_dir("evidence-refs");
        let addr;
        {
            let mut store = FileCaseStore::open(&dir).unwrap();
            let mut case = store.create_case(id(1), "title", "alice").unwrap();
            let mut evidence = store.open_evidence_store(&id(1)).unwrap();
            addr = *evidence
                .put(b"artifact bytes", "example.org", "alice")
                .unwrap()
                .address();
            case.attach(&addr, "alice").unwrap();
            store.save_case(&case).unwrap();
        }
        let store = FileCaseStore::open(&dir).unwrap();
        let loaded = store.load_case(&id(1)).unwrap();
        let refs: Vec<_> = loaded.evidence_addresses().collect();
        assert_eq!(refs, vec![&addr]);
        // The blob is still present and verifies.
        let evidence = store.open_evidence_store(&id(1)).unwrap();
        assert_eq!(evidence.get(&addr).unwrap(), b"artifact bytes");
        cleanup(&dir);
    }

    #[test]
    fn lifecycle_persistence() {
        let dir = temp_dir("lifecycle");
        {
            let mut store = FileCaseStore::open(&dir).unwrap();
            let mut case = store.create_case(id(1), "title", "alice").unwrap();
            case.close("alice").unwrap();
            store.save_case(&case).unwrap();
        }
        let store = FileCaseStore::open(&dir).unwrap();
        assert_eq!(
            store.load_case(&id(1)).unwrap().status(),
            CaseStatus::Closed
        );
        cleanup(&dir);
    }

    #[test]
    fn audit_persistence() {
        let dir = temp_dir("audit");
        {
            let mut store = FileCaseStore::open(&dir).unwrap();
            let mut case = store.create_case(id(1), "title", "alice").unwrap();
            let mut evidence = store.open_evidence_store(&id(1)).unwrap();
            let addr = *evidence.put(b"x", "src", "alice").unwrap().address();
            case.attach(&addr, "alice").unwrap();
            case.set_notes("n", "alice").unwrap();
            case.close("alice").unwrap();
            case.reopen("bob").unwrap();
            store.save_case(&case).unwrap();
        }
        let store = FileCaseStore::open(&dir).unwrap();
        let log = store.audit_log(&id(1)).unwrap();
        let actions: Vec<&str> = log.entries().iter().map(|e| e.action.as_str()).collect();
        assert_eq!(
            actions,
            [
                "case.created",
                "case.evidence.attached",
                "case.notes.updated",
                "case.closed",
                "case.reopened"
            ]
        );
        assert_eq!(log.entries()[1].action, ACTION_EVIDENCE_ATTACHED);
        assert_eq!(log.entries()[4].action, ACTION_REOPENED);
        log.verify().unwrap();
        cleanup(&dir);
    }

    #[test]
    fn audit_tampering_is_detected() {
        let dir = temp_dir("tampered-audit");
        {
            let mut store = FileCaseStore::open(&dir).unwrap();
            let mut case = store.create_case(id(1), "title", "alice").unwrap();
            case.set_notes("n", "alice").unwrap();
            store.save_case(&case).unwrap();
        }
        // Rewrite an audit action inside the manifest.
        let manifest = dir.join("cases").join("case-1").join("case.json");
        let text = std::fs::read_to_string(&manifest).unwrap();
        let tampered = text.replace("\"case.notes.updated\"", "\"case.tampered\"");
        assert_ne!(text, tampered);
        std::fs::write(&manifest, tampered).unwrap();

        let store = FileCaseStore::open(&dir).unwrap();
        let err = store.load_case(&id(1)).unwrap_err();
        assert!(
            matches!(err, Error::IntegrityViolation(_)),
            "expected IntegrityViolation, got {err}"
        );
        cleanup(&dir);
    }

    #[test]
    fn case_metadata_tampering_is_detected() {
        let dir = temp_dir("tampered-metadata");
        {
            let mut store = FileCaseStore::open(&dir).unwrap();
            store.create_case(id(1), "original title", "alice").unwrap();
        }
        // Change the title without recording an audit event.
        let manifest = dir.join("cases").join("case-1").join("case.json");
        let text = std::fs::read_to_string(&manifest).unwrap();
        let tampered = text.replace("\"original title\"", "\"evil title\"");
        assert_ne!(text, tampered);
        std::fs::write(&manifest, tampered).unwrap();

        let store = FileCaseStore::open(&dir).unwrap();
        let err = store.load_case(&id(1)).unwrap_err();
        assert!(
            matches!(err, Error::IntegrityViolation(_)),
            "expected IntegrityViolation, got {err}"
        );
        cleanup(&dir);
    }

    #[test]
    fn evidence_tampering_is_detected() {
        let dir = temp_dir("tampered-evidence");
        let addr;
        {
            let mut store = FileCaseStore::open(&dir).unwrap();
            let mut case = store.create_case(id(1), "title", "alice").unwrap();
            let mut evidence = store.open_evidence_store(&id(1)).unwrap();
            addr = *evidence
                .put(b"tamper me", "example.org", "alice")
                .unwrap()
                .address();
            case.attach(&addr, "alice").unwrap();
            store.save_case(&case).unwrap();
        }
        // Flip one byte in the blob file.
        let blob = blob_path(&dir, &addr);
        let mut bytes = std::fs::read(&blob).unwrap();
        bytes[0] ^= 0xff;
        std::fs::write(&blob, &bytes).unwrap();

        let store = FileCaseStore::open(&dir).unwrap();
        let err = store.load_case(&id(1)).unwrap_err();
        assert!(
            matches!(err, Error::IntegrityViolation(_)),
            "expected IntegrityViolation, got {err}"
        );
        cleanup(&dir);
    }

    #[test]
    fn dangling_evidence_reference_is_rejected_on_save() {
        let dir = temp_dir("dangling");
        let mut store = FileCaseStore::open(&dir).unwrap();
        let mut case = store.create_case(id(1), "title", "alice").unwrap();
        case.attach(&sample(), "alice").unwrap(); // never stored as a blob
        let err = store.save_case(&case).unwrap_err();
        assert!(
            matches!(err, Error::NotFound(_)),
            "expected NotFound, got {err}"
        );
        cleanup(&dir);
    }

    #[test]
    fn deleted_referenced_blob_fails_on_load() {
        let dir = temp_dir("missing-blob");
        let addr;
        {
            let mut store = FileCaseStore::open(&dir).unwrap();
            let mut case = store.create_case(id(1), "title", "alice").unwrap();
            let mut evidence = store.open_evidence_store(&id(1)).unwrap();
            addr = *evidence.put(b"x", "src", "alice").unwrap().address();
            case.attach(&addr, "alice").unwrap();
            store.save_case(&case).unwrap();
        }
        // Delete the blob file behind the store's back.
        std::fs::remove_file(blob_path(&dir, &addr)).unwrap();
        let store = FileCaseStore::open(&dir).unwrap();
        assert!(store.load_case(&id(1)).is_err());
        cleanup(&dir);
    }

    #[test]
    fn case_isolation() {
        let dir = temp_dir("isolation");
        let addr;
        {
            let mut store = FileCaseStore::open(&dir).unwrap();
            let mut case_a = store.create_case(id(1), "A", "alice").unwrap();
            let mut evidence_a = store.open_evidence_store(&id(1)).unwrap();
            addr = *evidence_a
                .put(b"secret of A", "src", "alice")
                .unwrap()
                .address();
            case_a.attach(&addr, "alice").unwrap();
            store.save_case(&case_a).unwrap();
            store.create_case(id(2), "B", "alice").unwrap();
        }
        let store = FileCaseStore::open(&dir).unwrap();
        // Case B does not reference A's evidence...
        assert_eq!(
            store
                .load_case(&id(2))
                .unwrap()
                .evidence_addresses()
                .count(),
            0
        );
        // ...and B's evidence store does not contain A's blob.
        let evidence_b = store.open_evidence_store(&id(2)).unwrap();
        assert!(!evidence_b.contains(&addr));
        // A still has its evidence intact.
        assert_eq!(
            store
                .load_case(&id(1))
                .unwrap()
                .evidence_addresses()
                .count(),
            1
        );
        cleanup(&dir);
    }

    #[test]
    fn path_traversal_is_rejected() {
        let dir = temp_dir("traversal");
        let mut store = FileCaseStore::open(&dir).unwrap();
        let dotdot = CaseId::new("..").unwrap();
        let dot = CaseId::new(".").unwrap();
        assert!(matches!(
            store.load_case(&dotdot).unwrap_err(),
            Error::InvalidInput(_)
        ));
        assert!(matches!(
            store.create_case(dotdot, "title", "alice").unwrap_err(),
            Error::InvalidInput(_)
        ));
        assert!(matches!(
            store.load_case(&dot).unwrap_err(),
            Error::InvalidInput(_)
        ));
        // Nothing was written outside the cases directory.
        assert!(!dir.join("case.json").exists());
        cleanup(&dir);
    }

    #[test]
    fn missing_case_handling() {
        let dir = temp_dir("missing");
        let store = FileCaseStore::open(&dir).unwrap();
        assert!(matches!(
            store.load_case(&id(5)).unwrap_err(),
            Error::NotFound(_)
        ));
        assert!(matches!(
            store.audit_log(&id(5)).unwrap_err(),
            Error::NotFound(_)
        ));
        assert!(!store.contains_case(&id(5)));
        cleanup(&dir);
    }

    #[test]
    fn save_missing_case_is_not_found() {
        let dir = temp_dir("save-missing");
        let mut store = FileCaseStore::open(&dir).unwrap();
        store.create_case(id(1), "title", "alice").unwrap();
        let stranger = Case::new(id(2), "title", "alice").unwrap();
        let err = store.save_case(&stranger).unwrap_err();
        assert!(matches!(err, Error::NotFound(_)));
        cleanup(&dir);
    }

    #[test]
    fn duplicate_create_is_rejected() {
        let dir = temp_dir("duplicate");
        let mut store = FileCaseStore::open(&dir).unwrap();
        store.create_case(id(1), "title", "alice").unwrap();
        let err = store.create_case(id(1), "other", "alice").unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));
        cleanup(&dir);
    }

    #[test]
    fn resave_is_byte_identical() {
        let dir = temp_dir("deterministic");
        let manifest = dir.join("cases").join("case-1").join("case.json");
        let first;
        let second;
        {
            let mut store = FileCaseStore::open(&dir).unwrap();
            let mut case = store.create_case(id(1), "title", "alice").unwrap();
            case.set_notes("notes", "alice").unwrap();
            store.save_case(&case).unwrap();
            first = std::fs::read(&manifest).unwrap();
            let loaded = store.load_case(&id(1)).unwrap();
            store.save_case(&loaded).unwrap();
            second = std::fs::read(&manifest).unwrap();
        }
        assert_eq!(first, second);
        cleanup(&dir);
    }
}
