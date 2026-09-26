//! Desktop application state model.
//!
//! Maintains the state for active investigator context, active cases,
//! terminal session histories, browser navigation contexts, and IPC service health.

use serde::{Deserialize, Serialize};

/// The primary navigation view tabs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesktopTab {
    /// Case management, metadata, evidence, and audit logs.
    #[default]
    Cases,
    /// Investigation terminal console.
    Terminal,
    /// Investigation browser & Tor research console.
    Browser,
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
    /// Count of attached evidence items.
    pub evidence_count: usize,
    /// Count of audit events.
    pub audit_count: usize,
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
    /// Currently selected active case ID.
    pub active_case: Option<CaseSummary>,
    /// Known cases list.
    pub cases: Vec<CaseSummary>,
    /// New case creation ID input.
    pub new_case_id: String,
    /// New case creation title input.
    pub new_case_title: String,

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
}

impl Default for DesktopState {
    fn default() -> Self {
        Self {
            active_tab: DesktopTab::Cases,
            investigator_id: "investigator-01".to_string(),
            active_case: None,
            cases: Vec::new(),
            new_case_id: String::new(),
            new_case_title: String::new(),

            terminal_session_id: None,
            terminal_program: "echo".to_string(),
            terminal_args: "PURSUE OS Terminal Active".to_string(),
            terminal_history: Vec::new(),

            browser_session_id: None,
            browser_mode: "tor".to_string(),
            browser_url: "https://duckduckgo.com".to_string(),
            browser_history: Vec::new(),

            status_message: Some("Ready — IPC services connected".to_string()),
            is_error: false,
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
}
