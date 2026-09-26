//! Web evidence capture orchestration.
//!
//! Transforms web navigation artifacts into immutable cryptographic evidence:
//! 1. Hashes and stores raw web artifact bytes via [`pursue_evidence::EvidenceStore`].
//! 2. Associates the resulting [`pursue_evidence::ContentAddress`] with the active [`pursue_case::Case`].
//! 3. Appends an explicit audit event to the case's hash-chained [`pursue_evidence::AuditLog`].
//!
//! # Forensic Non-Repudiation Boundary
//! Cryptographic content addressing guarantees that the artifact stored inside PURSUE OS
//! has not been tampered with or modified since capture.
//! **It does not prove or claim that external web content is real-world factual truth.**

use std::fmt;

use pursue_case::{CaseId, CaseStore, FileCaseStore};
use pursue_core::{Error, Result};
use pursue_evidence::{ContentAddress, EvidenceRecord, EvidenceStore};
use serde::{Deserialize, Serialize};

use crate::navigation::NavigationResult;
use crate::routing::RoutingMode;
use crate::session::BrowserSession;
use crate::session_id::BrowserSessionId;
use crate::url::ValidatedUrl;

/// The type of web artifact captured as evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebArtifactKind {
    /// The raw response body (HTML, JSON, plain text).
    PageContent,
    /// Canonicalized HTTP response headers.
    HttpHeaders,
    /// Visual rendering or viewport screenshot.
    Screenshot,
    /// Raw binary file downloaded during browsing.
    DownloadedFile,
}

impl WebArtifactKind {
    /// Returns a string slice naming the artifact kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::PageContent => "page_content",
            Self::HttpHeaders => "http_headers",
            Self::Screenshot => "screenshot",
            Self::DownloadedFile => "downloaded_file",
        }
    }
}

impl fmt::Display for WebArtifactKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Metadata and identity for captured browser evidence.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapturedWebEvidence {
    /// SHA-256 content address of the stored evidence blob.
    pub address: ContentAddress,
    /// The immutable evidence record metadata.
    pub record: EvidenceRecord,
    /// The category of web artifact captured.
    pub artifact_kind: WebArtifactKind,
    /// Provenance source label attached to the evidence record.
    pub source_label: String,
    /// The URL from which this artifact originated.
    pub url: ValidatedUrl,
}

/// Orchestrates capturing web artifacts into the case evidence repository.
pub struct WebEvidenceCapturer;

impl WebEvidenceCapturer {
    /// Formats a structured provenance source URI.
    pub fn build_source_label(
        session_id: &BrowserSessionId,
        url: &ValidatedUrl,
        mode: RoutingMode,
        kind: WebArtifactKind,
        status_code: u16,
    ) -> String {
        format!(
            "browser://session/{session_id}/url/{}?mode={mode}&kind={kind}&status={status_code}",
            url.as_str()
        )
    }

    /// Captures a web artifact from a [`NavigationResult`] into a file-backed [`FileCaseStore`].
    pub fn capture_navigation_artifact(
        store: &mut FileCaseStore,
        session: &BrowserSession,
        result: &NavigationResult,
        kind: WebArtifactKind,
        custom_label: Option<&str>,
    ) -> Result<CapturedWebEvidence> {
        let bytes: Vec<u8> = match kind {
            WebArtifactKind::PageContent => result.body().to_vec(),
            WebArtifactKind::HttpHeaders => {
                // Canonical JSON serialization of safe (scrubbed) headers
                serde_json::to_vec_pretty(&result.safe_headers()).map_err(|e| {
                    Error::ServiceFailure(format!("failed to serialize headers: {e}"))
                })?
            }
            WebArtifactKind::Screenshot | WebArtifactKind::DownloadedFile => result.body().to_vec(),
        };

        let source_label = match custom_label {
            Some(label) if !label.trim().is_empty() => label.trim().to_string(),
            _ => Self::build_source_label(
                session.id(),
                result.final_url(),
                result.routing_mode(),
                kind,
                result.status_code(),
            ),
        };

        // Open the case's evidence store (re-verifying blobs and audit chain)
        let mut evidence_store = store.open_evidence_store(session.case_id())?;

        // Ingest the raw bytes as an immutable evidence record
        let record = evidence_store.put(&bytes, &source_label, session.actor())?;
        let address = *record.address();

        // Load the case and attach the content address
        let mut case = store.load_case(session.case_id())?;
        case.attach(&address, session.actor())?;

        // Persist updated case manifest
        store.save_case(&case)?;

        Ok(CapturedWebEvidence {
            address,
            record,
            artifact_kind: kind,
            source_label,
            url: result.final_url().clone(),
        })
    }

    /// Generalized capture method that accepts trait-based stores (ideal for tests).
    #[allow(clippy::too_many_arguments)]
    pub fn capture_generic(
        evidence_store: &mut dyn EvidenceStore,
        case_store: &mut dyn CaseStore,
        case_id: &CaseId,
        actor: &str,
        session_id: &BrowserSessionId,
        url: &ValidatedUrl,
        mode: RoutingMode,
        kind: WebArtifactKind,
        status_code: u16,
        bytes: &[u8],
        custom_label: Option<&str>,
    ) -> Result<CapturedWebEvidence> {
        if !case_store.contains_case(case_id) {
            return Err(Error::NotFound(format!("case {case_id} not found")));
        }

        let source_label = match custom_label {
            Some(label) if !label.trim().is_empty() => label.trim().to_string(),
            _ => Self::build_source_label(session_id, url, mode, kind, status_code),
        };

        let record = evidence_store.put(bytes, &source_label, actor)?;
        let address = *record.address();

        let mut case = case_store.load_case(case_id)?;
        case.attach(&address, actor)?;
        case_store.save_case(&case)?;

        Ok(CapturedWebEvidence {
            address,
            record,
            artifact_kind: kind,
            source_label,
            url: url.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pursue_case::InMemoryCaseStore;
    use pursue_evidence::InMemoryStore;

    #[test]
    fn capture_generic_stores_blob_and_attaches_to_case() {
        let mut evidence_store = InMemoryStore::new();
        let mut case_store = InMemoryCaseStore::new();

        let cid = CaseId::new("case-capture-test").unwrap();
        let sid = BrowserSessionId::new("sess-cap").unwrap();
        let actor = "analyst@pursue";

        case_store
            .create_case(cid.clone(), "Web Investigation Case", actor)
            .unwrap();

        let url = ValidatedUrl::parse("https://example.com/evidence.html").unwrap();
        let html_bytes = b"<!DOCTYPE html><html><body>Target intel</body></html>";

        let captured = WebEvidenceCapturer::capture_generic(
            &mut evidence_store,
            &mut case_store,
            &cid,
            actor,
            &sid,
            &url,
            RoutingMode::Tor,
            WebArtifactKind::PageContent,
            200,
            html_bytes,
            None,
        )
        .unwrap();

        assert_eq!(captured.artifact_kind, WebArtifactKind::PageContent);
        assert!(captured.source_label.contains("mode=tor"));
        assert!(captured.source_label.contains("status=200"));
        assert!(captured.source_label.contains("kind=page_content"));

        // Verify stored in evidence store
        assert!(evidence_store.contains(&captured.address));
        assert_eq!(evidence_store.get(&captured.address).unwrap(), html_bytes);

        // Verify attached to case
        let updated_case = case_store.load_case(&cid).unwrap();
        let addresses: Vec<_> = updated_case.evidence_addresses().collect();
        assert_eq!(addresses, vec![&captured.address]);
        assert!(updated_case.audit_log().verify().is_ok());
    }

    #[test]
    fn capture_fails_if_case_not_found() {
        let mut evidence_store = InMemoryStore::new();
        let mut case_store = InMemoryCaseStore::new();

        let cid = CaseId::new("case-missing").unwrap();
        let sid = BrowserSessionId::new("sess-missing").unwrap();
        let url = ValidatedUrl::parse("https://example.com").unwrap();

        let res = WebEvidenceCapturer::capture_generic(
            &mut evidence_store,
            &mut case_store,
            &cid,
            "analyst",
            &sid,
            &url,
            RoutingMode::Direct,
            WebArtifactKind::PageContent,
            200,
            b"data",
            None,
        );

        assert!(matches!(res, Err(Error::NotFound(_))));
    }
}
