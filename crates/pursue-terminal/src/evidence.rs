//! Terminal evidence capture layer.
//!
//! Transforms terminal command output streams into immutable forensic evidence:
//! 1. Hashes and stores raw process output bytes via [`pursue_evidence::EvidenceStore`].
//! 2. Associates the resulting [`pursue_evidence::ContentAddress`] with the active [`pursue_case::Case`].
//! 3. Records an explicit provenance event in the case's append-only hash-chained [`pursue_evidence::AuditLog`].
//!
//! # Invariants
//! - **Raw bytes preserved**: Streams are stored byte-for-byte as emitted by the process.
//! - **Zero byte duplication**: Artifact bytes reside exclusively in the evidence store;
//!   the case references only the 32-byte content address.
//! - **Case isolation**: Evidence is written strictly into the session's case storage boundary.

use pursue_case::{CaseId, CaseStore, FileCaseStore};
use pursue_core::{Error, Result};
use pursue_evidence::{ContentAddress, EvidenceRecord, EvidenceStore};
use serde::{Deserialize, Serialize};

use crate::command::{CommandRequest, ExecutionResult};
use crate::session::Session;
use crate::session_id::SessionId;

/// Designates which command output stream was captured.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StreamKind {
    /// Standard output stream.
    Stdout,
    /// Standard error stream.
    Stderr,
}

impl std::fmt::Display for StreamKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamKind::Stdout => write!(f, "stdout"),
            StreamKind::Stderr => write!(f, "stderr"),
        }
    }
}

/// Metadata and identity for captured terminal evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedEvidence {
    /// SHA-256 content address of the stored evidence blob.
    pub address: ContentAddress,
    /// The immutable evidence record metadata.
    pub record: EvidenceRecord,
    /// Which process output stream produced this evidence.
    pub stream_kind: StreamKind,
    /// Provenance source label attached to the evidence record.
    pub source_label: String,
}

/// Orchestrates capturing command execution output into the case evidence repository.
pub struct TerminalEvidenceCapturer;

impl TerminalEvidenceCapturer {
    /// Formats a safe provenance source label without leaking environment or sensitive arguments.
    pub fn build_source_label(
        session_id: &SessionId,
        program: &str,
        stream: StreamKind,
        exit_code: Option<i32>,
    ) -> String {
        let exit_str = match exit_code {
            Some(code) => code.to_string(),
            None => "terminated".to_string(),
        };
        format!("terminal://session/{session_id}/cmd/{program}?stream={stream}&exit={exit_str}")
    }

    /// Captures a stream from an [`ExecutionResult`] directly into a file-backed [`FileCaseStore`].
    pub fn capture_stream_into_file_store(
        store: &mut FileCaseStore,
        session: &Session,
        request: &CommandRequest,
        result: &ExecutionResult,
        stream: StreamKind,
        custom_label: Option<&str>,
    ) -> Result<CapturedEvidence> {
        let bytes = match stream {
            StreamKind::Stdout => result.stdout(),
            StreamKind::Stderr => result.stderr(),
        };

        let source_label = match custom_label {
            Some(label) if !label.trim().is_empty() => label.trim().to_string(),
            _ => Self::build_source_label(
                session.id(),
                request.program(),
                stream,
                result.exit_code(),
            ),
        };

        // Open the case's evidence store (re-verifying blobs and audit log on open)
        let mut evidence_store = store.open_evidence_store(session.case_id())?;

        // Ingest the raw bytes as an immutable evidence record
        let record = evidence_store.put(bytes, &source_label, session.actor())?;
        let address = *record.address();

        // Load the case and attach the content address
        let mut case = store.load_case(session.case_id())?;
        case.attach(&address, session.actor())?;

        // Save the updated case manifest (re-verifies audit chain and checks reference integrity)
        store.save_case(&case)?;

        Ok(CapturedEvidence {
            address,
            record,
            stream_kind: stream,
            source_label,
        })
    }

    /// Generalized capture method that accepts trait-based stores (ideal for tests and custom storage).
    #[allow(clippy::too_many_arguments)]
    pub fn capture_generic(
        evidence_store: &mut dyn EvidenceStore,
        case_store: &mut dyn CaseStore,
        case_id: &CaseId,
        actor: &str,
        session_id: &SessionId,
        program: &str,
        exit_code: Option<i32>,
        bytes: &[u8],
        stream: StreamKind,
        custom_label: Option<&str>,
    ) -> Result<CapturedEvidence> {
        if !case_store.contains_case(case_id) {
            return Err(Error::NotFound(format!("case {case_id} not found")));
        }

        let source_label = match custom_label {
            Some(label) if !label.trim().is_empty() => label.trim().to_string(),
            _ => Self::build_source_label(session_id, program, stream, exit_code),
        };

        let record = evidence_store.put(bytes, &source_label, actor)?;
        let address = *record.address();

        let mut case = case_store.load_case(case_id)?;
        case.attach(&address, actor)?;
        case_store.save_case(&case)?;

        Ok(CapturedEvidence {
            address,
            record,
            stream_kind: stream,
            source_label,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    use pursue_case::{CaseId, InMemoryCaseStore};
    use pursue_evidence::InMemoryStore;

    use crate::session::Session;
    use crate::session_id::SessionId;

    fn sample_session() -> Session {
        Session::new(
            SessionId::new("term-capture").unwrap(),
            CaseId::new("case-capture").unwrap(),
            "analyst",
            PathBuf::from("/workspace"),
            BTreeMap::new(),
        )
        .unwrap()
    }

    #[test]
    fn capture_generic_stores_blob_and_attaches_to_case() {
        let mut ev_store = InMemoryStore::new();
        let mut case_store = InMemoryCaseStore::new();

        let session = sample_session();
        case_store
            .create_case(
                session.case_id().clone(),
                "Operation Alpha",
                "investigator-1",
            )
            .unwrap();

        let output_bytes = b"sample command output bytes";
        let captured = TerminalEvidenceCapturer::capture_generic(
            &mut ev_store,
            &mut case_store,
            session.case_id(),
            session.actor(),
            session.id(),
            "curl",
            Some(0),
            output_bytes,
            StreamKind::Stdout,
            None,
        )
        .unwrap();

        // 1. Evidence was stored in EvidenceStore
        assert!(ev_store.contains(&captured.address));
        assert_eq!(ev_store.get(&captured.address).unwrap(), output_bytes);

        // 2. Case has address attached
        let case = case_store.load_case(session.case_id()).unwrap();
        let addresses: Vec<_> = case.evidence_addresses().collect();
        assert_eq!(addresses, vec![&captured.address]);

        // 3. Audit log contains case.evidence.attached event
        let audit = case.audit_log();
        assert_eq!(audit.len(), 2);
        assert_eq!(audit.entries()[1].action, "case.evidence.attached");
        assert_eq!(audit.entries()[1].subject, Some(captured.address));
        audit.verify().unwrap();

        // 4. Source label formatting
        assert!(captured.source_label.contains("curl"));
        assert!(captured.source_label.contains("term-capture"));
        assert!(captured.source_label.contains("stream=stdout"));
    }

    #[test]
    fn capture_missing_case_fails_with_not_found() {
        let mut ev_store = InMemoryStore::new();
        let mut case_store = InMemoryCaseStore::new();
        let session = sample_session();

        let err = TerminalEvidenceCapturer::capture_generic(
            &mut ev_store,
            &mut case_store,
            session.case_id(),
            session.actor(),
            session.id(),
            "cat",
            Some(1),
            b"",
            StreamKind::Stderr,
            None,
        )
        .unwrap_err();

        assert!(matches!(err, Error::NotFound(_)));
    }
}
