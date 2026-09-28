//! Desktop application state model.
//!
//! Maintains the state for active investigator context, active cases,
//! evidence explorer, chronological timeline, hash-chained audit viewer,
//! reporting, terminal session histories, browser navigation contexts, and IPC service health.

use serde::{Deserialize, Serialize};

/// The primary navigation view tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesktopTab {
    /// Overview dashboard with investigation statistics and quick actions.
    #[default]
    Dashboard,
    /// Case management, metadata, and lifecycle operations.
    Cases,
    /// Evidence repository explorer and integrity verifier.
    Evidence,
    /// Unified chronological investigation timeline.
    Timeline,
    /// Cryptographic hash-chained audit trail viewer.
    Audit,
    /// Investigation terminal console.
    Terminal,
    /// Investigation browser & Tor research console.
    Browser,
    /// Forensic reporting, cryptographic sealing, and export.
    Reports,
    /// System and IPC service configuration.
    Settings,
}

/// Lightweight summary of a case for listing in the UI.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseSummary {
    /// The unique case identifier.
    pub id: String,
    /// The descriptive title.
    pub title: String,
    /// Investigator actor who created the case.
    pub created_by: String,
    /// Lifecycle status string ("open" or "closed").
    pub status: String,
    /// Count of attached evidence items.
    pub evidence_count: usize,
    /// Count of audit events.
    pub audit_count: usize,
}

/// Detailed representation of a selected case.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaseDetail {
    /// Case ID.
    pub id: String,
    /// Title.
    pub title: String,
    /// Notes.
    pub notes: String,
    /// Status ("open" or "closed").
    pub status: String,
    /// Creator actor.
    pub created_by: String,
    /// Creation timestamp (Unix seconds).
    pub created_at_unix: u64,
    /// Attached evidence content addresses (hex).
    pub evidence_addresses: Vec<String>,
    /// Audit events count.
    pub audit_count: usize,
    /// Deep verification status.
    pub verified: bool,
}

/// An attached evidence artifact item in the desktop state.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceItem {
    /// Content address in hex notation.
    pub address: String,
    /// Size in bytes.
    pub size: u64,
    /// Acquisition timestamp.
    pub acquired_at_unix: u64,
    /// Source provenance label.
    pub source: String,
    /// Cryptographic verification status.
    pub verified: bool,
    /// Content preview (text or hex dump).
    pub preview: Option<String>,
}

/// A recorded event in the cryptographic audit trail.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEventItem {
    /// Monotonic sequence number.
    pub seq: u64,
    /// Timestamp (Unix seconds).
    pub timestamp_unix: u64,
    /// Actor who performed the action.
    pub actor: String,
    /// Audited action name.
    pub action: String,
    /// Subject/content address if applicable.
    pub subject: Option<String>,
    /// Previous event hash.
    pub prev_hash: Option<String>,
    /// Current event SHA-256 hash.
    pub hash: String,
    /// Verification flag.
    pub verified: bool,
}

/// A unified chronological timeline event.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineItem {
    /// Sequence number.
    pub seq: u64,
    /// Timestamp (Unix seconds).
    pub timestamp_unix: u64,
    /// Category ("case", "evidence", "terminal", "browser", "audit").
    pub category: String,
    /// Actor.
    pub actor: String,
    /// Title.
    pub title: String,
    /// Summary description.
    pub summary: String,
    /// Optional reference/source ID.
    pub source_id: Option<String>,
    /// Verification status.
    pub verified: bool,
}

/// A recorded terminal execution entry in the desktop UI console.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminalConsoleEntry {
    /// Program name executed.
    pub program: String,
    /// Discrete arguments passed.
    pub args: Vec<String>,
    /// Raw stdout as UTF-8 string preview.
    pub stdout: String,
    /// Raw stderr as UTF-8 string preview.
    pub stderr: String,
    /// Process exit code.
    pub exit_code: Option<i32>,
    /// Whether execution timed out.
    pub timed_out: bool,
    /// Whether output buffer was truncated.
    pub truncated: bool,
    /// Whether output has been captured into case evidence.
    pub captured: bool,
    /// Raw JSON result object for IPC evidence capture.
    pub raw_result: serde_json::Value,
}

/// A recorded browser navigation entry in the desktop UI console.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserNavigationEntry {
    /// Target URL requested.
    pub url: String,
    /// Network routing mode employed (`"direct"` or `"tor"`).
    pub mode: String,
    /// HTTP response status code (e.g. 200, 404).
    pub status_code: u16,
    /// Safe, scrubbed response headers.
    pub headers: Vec<(String, String)>,
    /// Response body text preview.
    pub body_preview: String,
    /// Whether content has been captured into case evidence.
    pub captured: bool,
    /// Raw JSON result object for IPC evidence capture.
    pub raw_result: serde_json::Value,
}

/// Complete in-memory UI state for the PURSUE OS desktop application.
pub struct DesktopState {
    /// Currently active view tab.
    pub active_tab: DesktopTab,
    /// Investigator actor identity.
    pub investigator_id: String,

    // Case state
    /// Currently selected active case summary.
    pub active_case: Option<CaseSummary>,
    /// Full detail of selected active case.
    pub active_case_detail: Option<CaseDetail>,
    /// Known cases list.
    pub cases: Vec<CaseSummary>,
    /// New case creation ID input.
    pub new_case_id: String,
    /// New case creation title input.
    pub new_case_title: String,
    /// Edit notes buffer.
    pub edit_notes: String,
    /// Edit title buffer.
    pub edit_title: String,

    // Evidence explorer state
    /// Attached evidence items for active case.
    pub case_evidence_items: Vec<EvidenceItem>,
    /// Selected evidence address for inspection.
    pub selected_evidence_addr: Option<String>,
    /// Content preview of selected evidence item.
    pub selected_evidence_preview: Option<String>,

    // Audit viewer state
    /// Audit events for active case.
    pub case_audit_items: Vec<AuditEventItem>,
    /// Whether the full hash chain is currently verified.
    pub audit_chain_verified: bool,

    // Timeline state
    /// Aggregated chronological timeline items.
    pub case_timeline_items: Vec<TimelineItem>,

    // Report state
    /// Selected report format ("json" or "html").
    pub report_format: String,
    /// Target export path on disk.
    pub report_export_path: String,
    /// Generated report preview text.
    pub report_preview_content: Option<String>,
    /// Hash of last generated report.
    pub report_last_hash: Option<String>,
    /// Verification target path.
    pub report_verify_path: String,
    /// Verification status message.
    pub report_verify_result: Option<String>,

    // Terminal tab state
    /// Active terminal session ID.
    pub terminal_session_id: Option<String>,
    /// Terminal program input buffer.
    pub terminal_program: String,
    /// Terminal arguments input buffer.
    pub terminal_args: String,
    /// Execution log history.
    pub terminal_history: Vec<TerminalConsoleEntry>,

    // Browser tab state
    /// Active browser session ID.
    pub browser_session_id: Option<String>,
    /// Active routing mode (`"direct"` or `"tor"`).
    pub browser_mode: String,
    /// Target URL input buffer.
    pub browser_url: String,
    /// Navigation history.
    pub browser_history: Vec<BrowserNavigationEntry>,

    // Notifications & status
    /// Bottom status message.
    pub status_message: Option<String>,
    /// Whether the status message represents an error.
    pub is_error: bool,
    /// Active IPC endpoint information (e.g. Unix socket or in-process).
    pub ipc_endpoint_info: String,
    /// Whether backend IPC services are responding.
    pub ipc_online: bool,
}

impl Default for DesktopState {
    fn default() -> Self {
        Self {
            active_tab: DesktopTab::Dashboard,
            investigator_id: "investigator-01".to_string(),

            active_case: None,
            active_case_detail: None,
            cases: Vec::new(),
            new_case_id: String::new(),
            new_case_title: String::new(),
            edit_notes: String::new(),
            edit_title: String::new(),

            case_evidence_items: Vec::new(),
            selected_evidence_addr: None,
            selected_evidence_preview: None,

            case_audit_items: Vec::new(),
            audit_chain_verified: true,

            case_timeline_items: Vec::new(),

            report_format: "html".to_string(),
            report_export_path: "reports/investigation-report.html".to_string(),
            report_preview_content: None,
            report_last_hash: None,
            report_verify_path: "reports/investigation-report.html".to_string(),
            report_verify_result: None,

            terminal_session_id: None,
            terminal_program: String::new(),
            terminal_args: String::new(),
            terminal_history: Vec::new(),

            browser_session_id: None,
            browser_mode: "tor".to_string(),
            browser_url: "https://duckduckgo.com".to_string(),
            browser_history: Vec::new(),

            status_message: Some("Ready — IPC services connected".to_string()),
            is_error: false,
            ipc_endpoint_info: "In-Process Router".to_string(),
            ipc_online: true,
        }
    }
}

impl DesktopState {
    /// Creates a new state instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets an informational status message.
    pub fn set_info(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
        self.is_error = false;
    }

    /// Sets an error status message.
    pub fn set_error(&mut self, msg: impl Into<String>) {
        self.status_message = Some(msg.into());
        self.is_error = true;
    }

    /// Clears the status notification.
    pub fn clear_status(&mut self) {
        self.status_message = None;
        self.is_error = false;
    }

    /// Syncs timeline items from audit items.
    pub fn refresh_timeline_from_audit(&mut self) {
        let mut items = Vec::new();
        for entry in &self.case_audit_items {
            let (category, title, summary) = match entry.action.as_str() {
                "case.created" => (
                    "case".to_string(),
                    "Case Created".to_string(),
                    format!("Case initialized by '{}'", entry.actor),
                ),
                "case.title.updated" => (
                    "case".to_string(),
                    "Title Updated".to_string(),
                    format!("Case title changed by '{}'", entry.actor),
                ),
                "case.notes.updated" => (
                    "case".to_string(),
                    "Notes Updated".to_string(),
                    format!("Notes recorded by '{}'", entry.actor),
                ),
                "case.evidence.attached" => {
                    let addr = entry.subject.clone().unwrap_or_else(|| "unknown".into());
                    (
                        "evidence".to_string(),
                        "Evidence Attached".to_string(),
                        format!("Artifact attached: {addr}"),
                    )
                }
                "case.evidence.detached" => {
                    let addr = entry.subject.clone().unwrap_or_else(|| "unknown".into());
                    (
                        "evidence".to_string(),
                        "Evidence Detached".to_string(),
                        format!("Artifact detached: {addr}"),
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

            items.push(TimelineItem {
                seq: entry.seq,
                timestamp_unix: entry.timestamp_unix,
                category,
                actor: entry.actor.clone(),
                title,
                summary,
                source_id: entry.subject.clone(),
                verified: entry.verified,
            });
        }
        self.case_timeline_items = items;
    }
}
