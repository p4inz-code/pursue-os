//! Forensic investigation report data models.
//!
//! Provides structured models for representing full forensic case reports,
//! including metadata, verified evidence inventories, chronological timeline events,
//! and hash-chained audit provenance summaries.

use pursue_case::{Case, CaseStatus};
use pursue_core::{Error, Result};
use pursue_evidence::{ContentAddress, EvidenceStore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Metadata regarding the case and the report generation event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportMetadata {
    /// Case identifier.
    pub case_id: String,
    /// Case title at the time of report generation.
    pub case_title: String,
    /// Investigator notes.
    pub case_notes: String,
    /// Lifecycle status of the case.
    pub case_status: String,
    /// Identifier of the investigator who created the case.
    pub created_by: String,
    /// Timestamp when the case was created (seconds since Unix epoch).
    pub created_at_unix: u64,
    /// Timestamp when this report was generated.
    pub generated_at_unix: u64,
    /// Identifier of the actor/investigator who requested the report.
    pub generated_by: String,
    /// Total count of attached evidence items.
    pub evidence_count: usize,
    /// Total count of audit log events.
    pub audit_events_count: usize,
}

/// An entry describing an attached evidence artifact in the forensic report.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceReportItem {
    /// SHA-256 content address in hex notation.
    pub address: String,
    /// Size of the raw artifact payload in bytes.
    pub size_bytes: u64,
    /// Timestamp when evidence was originally acquired.
    pub acquired_at_unix: u64,
    /// Provenance source description.
    pub source: String,
    /// Cryptographic verification status of the artifact payload.
    pub verified: bool,
    /// Whether the payload represents valid UTF-8 text.
    pub is_utf8: bool,
    /// Safe preview of the artifact content (text preview or hex snippet).
    pub preview: Option<String>,
}

/// Chronological event item for the unified investigation timeline.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineReportItem {
    /// Monotonic sequence number in the case audit log.
    pub seq: u64,
    /// Timestamp of the event in seconds since Unix epoch.
    pub timestamp_unix: u64,
    /// Category of the event (e.g., "case", "evidence", "terminal", "browser", "audit").
    pub event_type: String,
    /// Actor who performed the action.
    pub actor: String,
    /// Brief descriptive title.
    pub title: String,
    /// Summary of the action or findings.
    pub summary: String,
    /// Associated source identifier or evidence address if applicable.
    pub source_id: Option<String>,
    /// Cryptographic verification status of this timeline step.
    pub verified: bool,
}

/// Summary of the cryptographic hash-chained audit trail.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditReportSummary {
    /// Total number of recorded audit events.
    pub total_events: usize,
    /// Whether the SHA-256 hash chain verified successfully with zero breaks.
    pub chain_intact: bool,
    /// SHA-256 digest of the genesis event.
    pub genesis_hash: Option<String>,
    /// SHA-256 digest of the latest event (audit log head).
    pub head_hash: Option<String>,
}

/// Complete forensic investigation report container.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InvestigationReport {
    /// Case and generation metadata.
    pub metadata: ReportMetadata,
    /// Audit log chain verification summary.
    pub audit_summary: AuditReportSummary,
    /// Verified evidence inventory.
    pub evidence_items: Vec<EvidenceReportItem>,
    /// Chronological investigation timeline.
    pub timeline_items: Vec<TimelineReportItem>,
    /// SHA-256 digest of the canonical report structure (None until finalized).
    pub report_hash: Option<String>,
}

impl InvestigationReport {
    /// Builds an unfinalized [`InvestigationReport`] directly from a verified [`Case`]
    /// and its corresponding [`EvidenceStore`].
    pub fn from_case<E: EvidenceStore>(
        case: &Case,
        evidence_store: &E,
        generated_by: &str,
        generated_at_unix: u64,
    ) -> Result<Self> {
        let generated_by = generated_by.trim();
        if generated_by.is_empty() {
            return Err(Error::InvalidInput(
                "report generated_by actor must not be empty".into(),
            ));
        }

        // Verify audit log chain
        let audit = case.audit_log();
        let chain_intact = audit.verify().is_ok();
        let entries = audit.entries();

        let genesis_hash = entries.first().map(|e| e.hash.to_hex());
        let head_hash = entries.last().map(|e| e.hash.to_hex());

        let audit_summary = AuditReportSummary {
            total_events: entries.len(),
            chain_intact,
            genesis_hash,
            head_hash,
        };

        // Collect and verify evidence items
        let mut evidence_items = Vec::new();
        for addr in case.evidence_addresses() {
            let record = evidence_store.record(addr)?;
            let bytes = evidence_store.get(addr)?;

            // Re-verify SHA-256 content address
            let computed_addr = ContentAddress::hash(&bytes);
            let verified = computed_addr == *addr;

            let is_utf8 = std::str::from_utf8(&bytes).is_ok();
            let preview = if is_utf8 {
                let s = String::from_utf8_lossy(&bytes);
                if s.chars().count() > 500 {
                    let truncated: String = s.chars().take(500).collect();
                    Some(format!("{truncated} ... [truncated]"))
                } else {
                    Some(s.to_string())
                }
            } else {
                let hex_sample: Vec<String> =
                    bytes.iter().take(64).map(|b| format!("{b:02x}")).collect();
                Some(format!(
                    "Binary payload ({} bytes): [{}]",
                    bytes.len(),
                    hex_sample.join(" ")
                ))
            };

            evidence_items.push(EvidenceReportItem {
                address: addr.to_hex(),
                size_bytes: record.size(),
                acquired_at_unix: record.acquired_at_unix(),
                source: record.source().to_string(),
                verified,
                is_utf8,
                preview,
            });
        }

        // Construct timeline items from audit entries
        let mut timeline_items = Vec::new();
        for entry in entries {
            let (event_type, title, summary) = match entry.action.as_str() {
                "case.created" => (
                    "case".to_string(),
                    "Case Created".to_string(),
                    format!("Case initialized by investigator '{}'", entry.actor),
                ),
                "case.title.updated" => (
                    "case".to_string(),
                    "Title Updated".to_string(),
                    format!("Case title updated by '{}'", entry.actor),
                ),
                "case.notes.updated" => (
                    "case".to_string(),
                    "Notes Updated".to_string(),
                    format!("Case notes updated by '{}'", entry.actor),
                ),
                "case.evidence.attached" => {
                    let addr_hex = entry
                        .subject
                        .as_ref()
                        .map(|s| s.to_hex())
                        .unwrap_or_else(|| "unknown".to_string());
                    (
                        "evidence".to_string(),
                        "Evidence Attached".to_string(),
                        format!("Evidence artifact attached: {addr_hex}"),
                    )
                }
                "case.evidence.detached" => {
                    let addr_hex = entry
                        .subject
                        .as_ref()
                        .map(|s| s.to_hex())
                        .unwrap_or_else(|| "unknown".to_string());
                    (
                        "evidence".to_string(),
                        "Evidence Detached".to_string(),
                        format!("Evidence artifact detached: {addr_hex}"),
                    )
                }
                "case.closed" => (
                    "case".to_string(),
                    "Case Closed".to_string(),
                    format!("Case closed by '{}'", entry.actor),
                ),
                "case.reopened" => (
                    "case".to_string(),
                    "Case Reopened".to_string(),
                    format!("Case reopened by '{}'", entry.actor),
                ),
                other => (
                    "audit".to_string(),
                    other.to_string(),
                    format!("Action '{other}' recorded by '{}'", entry.actor),
                ),
            };

            let source_id = entry.subject.as_ref().map(|s| s.to_hex());

            timeline_items.push(TimelineReportItem {
                seq: entry.seq,
                timestamp_unix: entry.timestamp_unix,
                event_type,
                actor: entry.actor.clone(),
                title,
                summary,
                source_id,
                verified: true,
            });
        }

        let metadata = ReportMetadata {
            case_id: case.id().to_string(),
            case_title: case.title().to_string(),
            case_notes: case.notes().to_string(),
            case_status: match case.status() {
                CaseStatus::Open => "open".to_string(),
                CaseStatus::Closed => "closed".to_string(),
            },
            created_by: case.created_by().to_string(),
            created_at_unix: case.created_at_unix(),
            generated_at_unix,
            generated_by: generated_by.to_string(),
            evidence_count: evidence_items.len(),
            audit_events_count: timeline_items.len(),
        };

        Ok(Self {
            metadata,
            audit_summary,
            evidence_items,
            timeline_items,
            report_hash: None,
        })
    }

    /// Computes the deterministic SHA-256 hash over the canonical JSON representation
    /// of the report with `report_hash: None`.
    pub fn compute_canonical_hash(&self) -> String {
        let mut cloned = self.clone();
        cloned.report_hash = None;
        let json_bytes = serde_json::to_vec(&cloned).expect("serialization is infallible");
        let mut hasher = Sha256::new();
        hasher.update(&json_bytes);
        let digest = hasher.finalize();
        format!("{digest:x}")
    }

    /// Finalizes the report by calculating and embedding its cryptographic SHA-256 hash.
    pub fn finalize_with_hash(mut self) -> Self {
        let hash = self.compute_canonical_hash();
        self.report_hash = Some(hash);
        self
    }

    /// Verifies the structural and cryptographic integrity of the report.
    pub fn verify_integrity(&self) -> Result<()> {
        let recorded_hash = self.report_hash.as_ref().ok_or_else(|| {
            Error::InvalidInput("report has not been finalized with a hash".into())
        })?;

        let expected_hash = self.compute_canonical_hash();
        if recorded_hash != &expected_hash {
            return Err(Error::IntegrityViolation(format!(
                "report hash mismatch: expected {expected_hash}, recorded {recorded_hash}"
            )));
        }

        if !self.audit_summary.chain_intact {
            return Err(Error::IntegrityViolation(
                "case audit hash-chain is reported broken or invalid".into(),
            ));
        }

        for ev in &self.evidence_items {
            if !ev.verified {
                return Err(Error::IntegrityViolation(format!(
                    "evidence item {} failed verification",
                    ev.address
                )));
            }
        }

        Ok(())
    }
}
