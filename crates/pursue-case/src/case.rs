//! The case model: the investigator-controlled container.
//!
//! A [`Case`] holds investigator-controlled metadata (title, notes), creation
//! information, a lifecycle status, and references to evidence by
//! [`ContentAddress`]. Evidence bytes are never stored inside a case — the
//! evidence store remains the source of truth.
//!
//! Every state-changing operation appends an event to the case's hash-chained
//! [`AuditLog`] (reused unchanged from `pursue-evidence`), so no mutation can
//! happen silently. Operations validate before mutating: a failed operation
//! leaves the case unchanged.

use std::collections::BTreeSet;
use std::time::{SystemTime, UNIX_EPOCH};

use pursue_core::{Error, Result};
use pursue_evidence::{AuditLog, ContentAddress};
use serde::{Deserialize, Serialize};

use crate::CaseId;

/// Audit action recorded when a case is created.
pub const ACTION_CREATED: &str = "case.created";
/// Audit action recorded when the case title is changed.
pub const ACTION_TITLE_UPDATED: &str = "case.title.updated";
/// Audit action recorded when the case notes are changed.
pub const ACTION_NOTES_UPDATED: &str = "case.notes.updated";
/// Audit action recorded when evidence is attached to the case.
pub const ACTION_EVIDENCE_ATTACHED: &str = "case.evidence.attached";
/// Audit action recorded when evidence is detached from the case.
pub const ACTION_EVIDENCE_DETACHED: &str = "case.evidence.detached";
/// Audit action recorded when the case is closed.
pub const ACTION_CLOSED: &str = "case.closed";
/// Audit action recorded when a closed case is reopened.
pub const ACTION_REOPENED: &str = "case.reopened";

/// The lifecycle status of a case.
///
/// Transitions are explicit and audited: [`Case::close`] moves an `Open` case
/// to `Closed`, and [`Case::reopen`] moves a `Closed` case back to `Open`.
/// Any other transition is rejected as invalid input and changes nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CaseStatus {
    /// The case is active and being investigated.
    Open,
    /// The case has been closed; it may be reopened explicitly.
    Closed,
}

/// An investigator-controlled case container.
///
/// Fields are private; state changes happen only through the audited methods
/// ([`Case::attach`], [`Case::detach`], [`Case::set_title`],
/// [`Case::set_notes`], [`Case::close`], [`Case::reopen`]) or the
/// constructor. Serde round-trips preserve the case exactly, including its
/// audit log; integrity verification on load is the store layer's
/// responsibility (next phase).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Case {
    id: CaseId,
    title: String,
    notes: String,
    status: CaseStatus,
    created_at_unix: u64,
    created_by: String,
    evidence: BTreeSet<ContentAddress>,
    audit: AuditLog,
}

impl Case {
    /// Creates a new open case and records its `case.created` audit event.
    ///
    /// `title` and `created_by` must be non-empty after trimming. The case
    /// starts [`CaseStatus::Open`], with empty notes and no attached evidence.
    pub fn new(id: CaseId, title: &str, created_by: &str) -> Result<Self> {
        let title = title.trim();
        if title.is_empty() {
            return Err(Error::InvalidInput("case title must not be empty".into()));
        }
        let created_by = created_by.trim();
        if created_by.is_empty() {
            return Err(Error::InvalidInput("case creator must not be empty".into()));
        }
        let now = now_unix();
        let mut audit = AuditLog::new();
        audit.append(now, created_by, ACTION_CREATED, None)?;
        Ok(Self {
            id,
            title: title.to_string(),
            notes: String::new(),
            status: CaseStatus::Open,
            created_at_unix: now,
            created_by: created_by.to_string(),
            evidence: BTreeSet::new(),
            audit,
        })
    }

    /// The case identifier.
    pub fn id(&self) -> &CaseId {
        &self.id
    }

    /// The case title (investigator-controlled).
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The case notes (investigator-controlled metadata).
    pub fn notes(&self) -> &str {
        &self.notes
    }

    /// The current lifecycle status.
    pub fn status(&self) -> CaseStatus {
        self.status
    }

    /// Unix timestamp (seconds) when the case was created.
    pub fn created_at_unix(&self) -> u64 {
        self.created_at_unix
    }

    /// The investigator who created the case.
    pub fn created_by(&self) -> &str {
        &self.created_by
    }

    /// References to attached evidence, in ascending content-address order.
    pub fn evidence_addresses(&self) -> impl Iterator<Item = &ContentAddress> {
        self.evidence.iter()
    }

    /// The case's provenance record: its hash-chained audit log.
    pub fn audit_log(&self) -> &AuditLog {
        &self.audit
    }

    /// Attaches evidence to the case by content address.
    ///
    /// Idempotent: attaching an already-attached address is a no-op and does
    /// not record a duplicate audit event. Only the address is stored;
    /// evidence bytes remain in the evidence store (the source of truth).
    pub fn attach(&mut self, address: &ContentAddress, actor: &str) -> Result<()> {
        if self.evidence.contains(address) {
            return Ok(());
        }
        self.audit
            .append(now_unix(), actor, ACTION_EVIDENCE_ATTACHED, Some(*address))?;
        self.evidence.insert(*address);
        Ok(())
    }

    /// Detaches evidence from the case by content address.
    ///
    /// Fails with [`Error::NotFound`] if the address is not attached; the case
    /// is left unchanged.
    pub fn detach(&mut self, address: &ContentAddress, actor: &str) -> Result<()> {
        if !self.evidence.contains(address) {
            return Err(Error::NotFound(format!(
                "evidence {address} is not attached to case {}",
                self.id
            )));
        }
        self.audit
            .append(now_unix(), actor, ACTION_EVIDENCE_DETACHED, Some(*address))?;
        self.evidence.remove(address);
        Ok(())
    }

    /// Replaces the case title (investigator-controlled).
    ///
    /// The title must be non-empty after trimming. Setting the current title
    /// again is a no-op and records no event.
    pub fn set_title(&mut self, title: &str, actor: &str) -> Result<()> {
        let title = title.trim();
        if title.is_empty() {
            return Err(Error::InvalidInput("case title must not be empty".into()));
        }
        if title == self.title {
            return Ok(());
        }
        self.audit
            .append(now_unix(), actor, ACTION_TITLE_UPDATED, None)?;
        self.title = title.to_string();
        Ok(())
    }

    /// Replaces the case notes (investigator-controlled metadata).
    ///
    /// Notes are free text and may be empty (clearing). Setting the current
    /// notes again is a no-op and records no event.
    pub fn set_notes(&mut self, notes: &str, actor: &str) -> Result<()> {
        if notes == self.notes {
            return Ok(());
        }
        self.audit
            .append(now_unix(), actor, ACTION_NOTES_UPDATED, None)?;
        self.notes = notes.to_string();
        Ok(())
    }

    /// Closes the case, recording a `case.closed` audit event.
    ///
    /// Fails with [`Error::InvalidInput`] if the case is already closed.
    pub fn close(&mut self, actor: &str) -> Result<()> {
        if self.status == CaseStatus::Closed {
            return Err(Error::InvalidInput(format!(
                "case {} is already closed",
                self.id
            )));
        }
        self.audit.append(now_unix(), actor, ACTION_CLOSED, None)?;
        self.status = CaseStatus::Closed;
        Ok(())
    }

    /// Reopens a closed case, recording a `case.reopened` audit event.
    ///
    /// Fails with [`Error::InvalidInput`] if the case is already open.
    pub fn reopen(&mut self, actor: &str) -> Result<()> {
        if self.status == CaseStatus::Open {
            return Err(Error::InvalidInput(format!(
                "case {} is already open",
                self.id
            )));
        }
        self.audit
            .append(now_unix(), actor, ACTION_REOPENED, None)?;
        self.status = CaseStatus::Open;
        Ok(())
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u32) -> CaseId {
        CaseId::new(&format!("case-{n}")).unwrap()
    }

    fn sample() -> ContentAddress {
        ContentAddress::hash(b"payload")
    }

    #[test]
    fn new_case_defaults() {
        let case = Case::new(id(1), " Operation Alpha ", "investigator-1").unwrap();
        assert_eq!(case.id(), &id(1));
        assert_eq!(case.title(), "Operation Alpha");
        assert_eq!(case.notes(), "");
        assert_eq!(case.status(), CaseStatus::Open);
        assert_eq!(case.created_by(), "investigator-1");
        assert!(case.created_at_unix() > 0);
        assert_eq!(case.evidence_addresses().count(), 0);
        assert_eq!(case.audit_log().len(), 1);
        let entry = &case.audit_log().entries()[0];
        assert_eq!(entry.action, ACTION_CREATED);
        assert_eq!(entry.actor, "investigator-1");
        assert_eq!(entry.subject, None);
        case.audit_log().verify().unwrap();
    }

    #[test]
    fn new_rejects_empty_title() {
        assert!(Case::new(id(1), "", "alice").is_err());
        assert!(Case::new(id(1), "   ", "alice").is_err());
    }

    #[test]
    fn new_rejects_empty_creator() {
        assert!(Case::new(id(1), "title", "").is_err());
        assert!(Case::new(id(1), "title", "  ").is_err());
    }

    #[test]
    fn new_rejects_nul_in_creator() {
        assert!(Case::new(id(1), "title", "ali\x00ce").is_err());
    }

    #[test]
    fn attach_adds_content_address_and_records_event() {
        let mut case = Case::new(id(1), "title", "alice").unwrap();
        let addr = sample();
        case.attach(&addr, "alice").unwrap();
        assert_eq!(case.evidence_addresses().count(), 1);
        assert_eq!(case.evidence_addresses().next(), Some(&addr));
        assert_eq!(case.audit_log().len(), 2);
        let entry = &case.audit_log().entries()[1];
        assert_eq!(entry.action, ACTION_EVIDENCE_ATTACHED);
        assert_eq!(entry.actor, "alice");
        assert_eq!(entry.subject, Some(addr));
        case.audit_log().verify().unwrap();
    }

    #[test]
    fn attach_duplicate_is_idempotent_noop() {
        let mut case = Case::new(id(1), "title", "alice").unwrap();
        let addr = sample();
        case.attach(&addr, "alice").unwrap();
        case.attach(&addr, "alice").unwrap();
        assert_eq!(case.evidence_addresses().count(), 1);
        assert_eq!(case.audit_log().len(), 2); // one attach event only
    }

    #[test]
    fn detach_removes_address_and_records_event() {
        let mut case = Case::new(id(1), "title", "alice").unwrap();
        let addr = sample();
        case.attach(&addr, "alice").unwrap();
        case.detach(&addr, "bob").unwrap();
        assert_eq!(case.evidence_addresses().count(), 0);
        assert_eq!(case.audit_log().len(), 3);
        let entry = &case.audit_log().entries()[2];
        assert_eq!(entry.action, ACTION_EVIDENCE_DETACHED);
        assert_eq!(entry.actor, "bob");
        assert_eq!(entry.subject, Some(addr));
        case.audit_log().verify().unwrap();
    }

    #[test]
    fn detach_missing_address_is_not_found() {
        let mut case = Case::new(id(1), "title", "alice").unwrap();
        let err = case.detach(&sample(), "alice").unwrap_err();
        assert!(matches!(err, Error::NotFound(_)));
        assert_eq!(case.audit_log().len(), 1); // no event recorded
        assert_eq!(case.evidence_addresses().count(), 0);
    }

    #[test]
    fn evidence_references_are_content_addresses_only() {
        let mut case = Case::new(id(1), "title", "alice").unwrap();
        let addr = ContentAddress::hash(b"artifact bytes");
        case.attach(&addr, "alice").unwrap();
        let refs: Vec<_> = case.evidence_addresses().collect();
        assert_eq!(refs, vec![&addr]);
        // The case stores the identity (SHA-256 content address), never the
        // artifact bytes themselves.
        assert_eq!(addr.to_hex().len(), 64);
    }

    #[test]
    fn set_title_updates_and_records_event() {
        let mut case = Case::new(id(1), "old", "alice").unwrap();
        case.set_title(" new title ", "bob").unwrap();
        assert_eq!(case.title(), "new title");
        assert_eq!(case.audit_log().len(), 2);
        let entry = &case.audit_log().entries()[1];
        assert_eq!(entry.action, ACTION_TITLE_UPDATED);
        assert_eq!(entry.actor, "bob");
    }

    #[test]
    fn set_title_rejects_empty_without_change() {
        let mut case = Case::new(id(1), "title", "alice").unwrap();
        assert!(case.set_title("", "alice").is_err());
        assert!(case.set_title("   ", "alice").is_err());
        assert_eq!(case.title(), "title");
        assert_eq!(case.audit_log().len(), 1);
    }

    #[test]
    fn set_title_same_value_is_noop() {
        let mut case = Case::new(id(1), "title", "alice").unwrap();
        case.set_title(" title ", "alice").unwrap(); // trims to current value
        assert_eq!(case.audit_log().len(), 1);
    }

    #[test]
    fn set_notes_updates_and_records_event() {
        let mut case = Case::new(id(1), "title", "alice").unwrap();
        case.set_notes("investigator notes", "alice").unwrap();
        assert_eq!(case.notes(), "investigator notes");
        assert_eq!(case.audit_log().len(), 2);
        assert_eq!(case.audit_log().entries()[1].action, ACTION_NOTES_UPDATED);
        // Clearing notes is allowed.
        case.set_notes("", "alice").unwrap();
        assert_eq!(case.notes(), "");
        assert_eq!(case.audit_log().len(), 3);
    }

    #[test]
    fn set_notes_same_value_is_noop() {
        let mut case = Case::new(id(1), "title", "alice").unwrap();
        case.set_notes("n", "alice").unwrap();
        case.set_notes("n", "alice").unwrap();
        assert_eq!(case.audit_log().len(), 2);
    }

    #[test]
    fn close_transition_is_audited() {
        let mut case = Case::new(id(1), "title", "alice").unwrap();
        case.close("alice").unwrap();
        assert_eq!(case.status(), CaseStatus::Closed);
        assert_eq!(case.audit_log().entries()[1].action, ACTION_CLOSED);
        case.audit_log().verify().unwrap();
    }

    #[test]
    fn closing_closed_case_is_invalid() {
        let mut case = Case::new(id(1), "title", "alice").unwrap();
        case.close("alice").unwrap();
        let err = case.close("alice").unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));
        assert_eq!(case.audit_log().len(), 2); // no second close event
    }

    #[test]
    fn reopen_transition_is_audited() {
        let mut case = Case::new(id(1), "title", "alice").unwrap();
        case.close("alice").unwrap();
        case.reopen("bob").unwrap();
        assert_eq!(case.status(), CaseStatus::Open);
        let entry = &case.audit_log().entries()[2];
        assert_eq!(entry.action, ACTION_REOPENED);
        assert_eq!(entry.actor, "bob");
        case.audit_log().verify().unwrap();
    }

    #[test]
    fn reopening_open_case_is_invalid() {
        let mut case = Case::new(id(1), "title", "alice").unwrap();
        let err = case.reopen("alice").unwrap_err();
        assert!(matches!(err, Error::InvalidInput(_)));
        assert_eq!(case.audit_log().len(), 1);
    }

    #[test]
    fn operations_reject_empty_actor_without_mutation() {
        let mut case = Case::new(id(1), "title", "alice").unwrap();
        assert!(case.attach(&sample(), "  ").is_err());
        assert_eq!(case.evidence_addresses().count(), 0);
        assert!(case.close("").is_err());
        assert_eq!(case.status(), CaseStatus::Open);
        assert!(case.set_title("x", "").is_err());
        assert_eq!(case.title(), "title");
        assert_eq!(case.audit_log().len(), 1); // nothing was recorded
    }

    #[test]
    fn audit_chain_verifies_after_full_workflow() {
        let mut case = Case::new(id(1), "title", "alice").unwrap();
        case.attach(&sample(), "alice").unwrap();
        case.set_notes("notes", "alice").unwrap();
        case.set_title("renamed", "alice").unwrap();
        case.close("alice").unwrap();
        case.reopen("alice").unwrap();
        case.detach(&sample(), "alice").unwrap();
        case.audit_log().verify().unwrap();
        assert_eq!(case.audit_log().len(), 7);
    }

    #[test]
    fn serde_roundtrip_preserves_case() {
        let mut case = Case::new(id(1), "title", "alice").unwrap();
        let addr = sample();
        case.attach(&addr, "alice").unwrap();
        case.set_notes("notes", "alice").unwrap();
        case.close("alice").unwrap();
        let json = serde_json::to_string(&case).unwrap();
        let back: Case = serde_json::from_str(&json).unwrap();
        assert_eq!(back, case);
        assert_eq!(back.status(), CaseStatus::Closed);
        assert_eq!(back.evidence_addresses().count(), 1);
        back.audit_log().verify().unwrap();
    }
}
