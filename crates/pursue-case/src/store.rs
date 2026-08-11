//! Case stores: the persistence boundary for cases.
//!
//! [`CaseStore`] is the trait-based contract for creating, loading, and
//! persisting cases. [`InMemoryCaseStore`] provides a deterministic
//! in-memory implementation for tests and small workloads; the file-backed
//! [`crate::file_store::FileCaseStore`] provides persistence, per-case
//! isolation, and verification on load.
//!
//! Workflow: create a case with [`CaseStore::create_case`], mutate the
//! returned [`Case`] through its audited methods (attach/detach evidence,
//! metadata updates, close/reopen), persist with [`CaseStore::save_case`],
//! and reload with [`CaseStore::load_case`]. Every mutation is recorded in
//! the case's hash-chained audit log; the store never fabricates events.

use std::collections::BTreeMap;

use pursue_core::{Error, Result};
use pursue_evidence::AuditLog;

use crate::{Case, CaseId};

/// A store for cases.
///
/// Implementations persist cases so that audited operations survive a
/// reload. Integrity guarantees:
///
/// - `save_case` refuses to persist a case whose audit chain fails
///   verification (no broken provenance is ever written).
/// - `load_case` on the file-backed store verifies the case manifest, the
///   audit chain, and the case's evidence store before returning the case;
///   any tampering or missing state fails closed with an error.
pub trait CaseStore {
    /// Creates and persists a new open case, recording its `case.created`
    /// audit event. Fails with [`Error::InvalidInput`] if the id is taken.
    fn create_case(&mut self, id: CaseId, title: &str, created_by: &str) -> Result<Case>;

    /// Loads a case, verifying its integrity. Fails with [`Error::NotFound`]
    /// if no such case exists.
    fn load_case(&self, id: &CaseId) -> Result<Case>;

    /// Persists `case` (e.g., after audited operations). Fails with
    /// [`Error::NotFound`] if the case does not exist in the store.
    fn save_case(&mut self, case: &Case) -> Result<()>;

    /// Whether a case with this id exists.
    fn contains_case(&self, id: &CaseId) -> bool;

    /// The verified audit log (provenance history) of a case.
    fn audit_log(&self, id: &CaseId) -> Result<AuditLog>;
}

/// An in-memory case store.
///
/// Deterministic and suitable for tests and small workloads. Cases are keyed
/// by `CaseId`; evidence references are preserved as content addresses, but
/// there is no evidence store in memory — the check that attached evidence
/// actually exists is a [`crate::file_store::FileCaseStore`] guarantee.
#[derive(Default)]
pub struct InMemoryCaseStore {
    cases: BTreeMap<CaseId, Case>,
}

impl InMemoryCaseStore {
    /// Creates an empty in-memory case store.
    pub fn new() -> Self {
        Self::default()
    }
}

impl CaseStore for InMemoryCaseStore {
    fn create_case(&mut self, id: CaseId, title: &str, created_by: &str) -> Result<Case> {
        if self.cases.contains_key(&id) {
            return Err(Error::InvalidInput(format!("case {id} already exists")));
        }
        let case = Case::new(id, title, created_by)?;
        self.cases.insert(case.id().clone(), case.clone());
        Ok(case)
    }

    fn load_case(&self, id: &CaseId) -> Result<Case> {
        self.cases
            .get(id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("case {id} not found")))
    }

    fn save_case(&mut self, case: &Case) -> Result<()> {
        if !self.cases.contains_key(case.id()) {
            return Err(Error::NotFound(format!("case {} not found", case.id())));
        }
        case.audit_log().verify()?;
        self.cases.insert(case.id().clone(), case.clone());
        Ok(())
    }

    fn contains_case(&self, id: &CaseId) -> bool {
        self.cases.contains_key(id)
    }

    fn audit_log(&self, id: &CaseId) -> Result<AuditLog> {
        Ok(self.load_case(id)?.audit_log().clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ACTION_CREATED, ACTION_EVIDENCE_ATTACHED, CaseStatus};
    use pursue_evidence::ContentAddress;

    fn id(n: u32) -> CaseId {
        CaseId::new(&format!("case-{n}")).unwrap()
    }

    fn sample() -> ContentAddress {
        ContentAddress::hash(b"payload")
    }

    #[test]
    fn create_then_load_roundtrip() {
        let mut store = InMemoryCaseStore::new();
        let created = store
            .create_case(id(1), "Operation Alpha", "investigator-1")
            .unwrap();
        let loaded = store.load_case(&id(1)).unwrap();
        assert_eq!(loaded, created);
        assert_eq!(loaded.title(), "Operation Alpha");
        assert_eq!(loaded.created_by(), "investigator-1");
        assert_eq!(loaded.status(), CaseStatus::Open);
        assert_eq!(loaded.audit_log().entries()[0].action, ACTION_CREATED);
        loaded.audit_log().verify().unwrap();
    }

    #[test]
    fn create_duplicate_is_rejected() {
        let mut store = InMemoryCaseStore::new();
        store.create_case(id(1), "title", "alice").unwrap();
        let err = store.create_case(id(1), "other", "alice").unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));
    }

    #[test]
    fn load_missing_is_not_found() {
        let store = InMemoryCaseStore::new();
        assert!(matches!(
            store.load_case(&id(9)).unwrap_err(),
            Error::NotFound(_)
        ));
        assert!(!store.contains_case(&id(9)));
    }

    #[test]
    fn save_persists_audited_operations() {
        let mut store = InMemoryCaseStore::new();
        let mut case = store.create_case(id(1), "title", "alice").unwrap();
        case.attach(&sample(), "alice").unwrap();
        case.set_notes("notes", "alice").unwrap();
        case.close("alice").unwrap();
        store.save_case(&case).unwrap();
        let loaded = store.load_case(&id(1)).unwrap();
        assert_eq!(loaded.status(), CaseStatus::Closed);
        assert_eq!(loaded.notes(), "notes");
        assert_eq!(loaded.evidence_addresses().count(), 1);
        assert_eq!(loaded.audit_log().len(), 4);
        loaded.audit_log().verify().unwrap();
    }

    #[test]
    fn save_unknown_case_is_not_found() {
        let mut store = InMemoryCaseStore::new();
        store.create_case(id(1), "title", "alice").unwrap();
        let stranger = Case::new(id(2), "title", "alice").unwrap();
        let err = store.save_case(&stranger).unwrap_err();
        assert!(matches!(err, Error::NotFound(_)));
    }

    #[test]
    fn audit_log_reads_verified_history() {
        let mut store = InMemoryCaseStore::new();
        let mut case = store.create_case(id(1), "title", "alice").unwrap();
        case.attach(&sample(), "alice").unwrap();
        store.save_case(&case).unwrap();
        let log = store.audit_log(&id(1)).unwrap();
        assert_eq!(log.len(), 2);
        assert_eq!(log.entries()[1].action, ACTION_EVIDENCE_ATTACHED);
        log.verify().unwrap();
    }

    #[test]
    fn audit_log_missing_case_is_not_found() {
        let store = InMemoryCaseStore::new();
        assert!(matches!(
            store.audit_log(&id(7)).unwrap_err(),
            Error::NotFound(_)
        ));
    }

    #[test]
    fn loaded_cases_are_independent_copies() {
        let mut store = InMemoryCaseStore::new();
        store.create_case(id(1), "title", "alice").unwrap();
        let mut loaded = store.load_case(&id(1)).unwrap();
        loaded.set_title("changed", "alice").unwrap();
        let again = store.load_case(&id(1)).unwrap();
        assert_eq!(again.title(), "title");
    }

    #[test]
    fn in_memory_behavior_is_deterministic() {
        let mut a = InMemoryCaseStore::new();
        let mut b = InMemoryCaseStore::new();
        for store in [&mut a, &mut b] {
            let mut case = store.create_case(id(1), "title", "alice").unwrap();
            case.attach(&sample(), "alice").unwrap();
            case.close("alice").unwrap();
            store.save_case(&case).unwrap();
        }
        assert_eq!(a.load_case(&id(1)).unwrap(), b.load_case(&id(1)).unwrap());
    }
}
