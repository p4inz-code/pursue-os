//! Browser session data model and lifecycle.
//!
//! Provides [`BrowserSession`] and [`SessionStatus`], representing an active or
//! terminated investigative browsing session bound to a specific case.
//!
//! # Profile Isolation Invariant
//! Each browser session is assigned an isolated filesystem profile directory:
//! `profiles/<case_id>/<session_id>/`.
//! Cookies, cache, local storage, and state remain strictly inside this boundary.
//! Sessions associated with Case A can never access or inherit state from Case B.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use pursue_case::CaseId;
use pursue_core::{Error, Result};
use serde::{Deserialize, Serialize};

use crate::routing::RoutingMode;
use crate::session_id::BrowserSessionId;

/// Current lifecycle status of a browser session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// The session is active and accepting navigation requests.
    Active,
    /// The session has been terminated; no further requests will be processed.
    Terminated,
}

impl SessionStatus {
    /// Returns `true` if the session is active.
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    /// Returns `true` if the session is terminated.
    pub fn is_terminated(&self) -> bool {
        matches!(self, Self::Terminated)
    }
}

/// A forensic browser session bound to a single case and routing mode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrowserSession {
    id: BrowserSessionId,
    case_id: CaseId,
    actor: String,
    mode: RoutingMode,
    profile_dir: PathBuf,
    status: SessionStatus,
    created_at: u64,
    terminated_at: Option<u64>,
}

impl BrowserSession {
    /// Creates a new active browser session.
    ///
    /// # Arguments
    /// - `id`: Validated session identifier.
    /// - `case_id`: The case owning this session.
    /// - `actor`: Investigator identifier initiating the session.
    /// - `mode`: The routing mode (`Direct` or `Tor`).
    /// - `base_storage_dir`: Base directory under which the isolated profile will be created.
    pub fn new(
        id: BrowserSessionId,
        case_id: CaseId,
        actor: &str,
        mode: RoutingMode,
        base_storage_dir: &Path,
    ) -> Result<Self> {
        let actor = actor.trim();
        if actor.is_empty() {
            return Err(Error::InvalidInput(
                "actor identifier must not be empty".into(),
            ));
        }

        // Profile directory: <base>/profiles/<case_id>/<session_id>
        let profile_dir = base_storage_dir
            .join("profiles")
            .join(case_id.as_str())
            .join(id.as_str());

        let created_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        Ok(Self {
            id,
            case_id,
            actor: actor.to_string(),
            mode,
            profile_dir,
            status: SessionStatus::Active,
            created_at,
            terminated_at: None,
        })
    }

    /// Returns the session identifier.
    pub fn id(&self) -> &BrowserSessionId {
        &self.id
    }

    /// Returns the owning case identifier.
    pub fn case_id(&self) -> &CaseId {
        &self.case_id
    }

    /// Returns the investigator actor string.
    pub fn actor(&self) -> &str {
        &self.actor
    }

    /// Returns the active routing mode.
    pub fn mode(&self) -> RoutingMode {
        self.mode
    }

    /// Returns the isolated profile directory path.
    pub fn profile_dir(&self) -> &Path {
        &self.profile_dir
    }

    /// Returns the current session status.
    pub fn status(&self) -> SessionStatus {
        self.status
    }

    /// Returns `true` if the session is active.
    pub fn is_active(&self) -> bool {
        self.status.is_active()
    }

    /// Returns the creation timestamp (seconds since Unix epoch).
    pub fn created_at(&self) -> u64 {
        self.created_at
    }

    /// Returns the termination timestamp, if terminated.
    pub fn terminated_at(&self) -> Option<u64> {
        self.terminated_at
    }

    /// Terminates the session. Once terminated, a session cannot be reactivated.
    pub fn terminate(&mut self) -> Result<()> {
        if self.status == SessionStatus::Terminated {
            return Err(Error::InvalidInput(format!(
                "session {} is already terminated",
                self.id
            )));
        }
        self.status = SessionStatus::Terminated;
        self.terminated_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(self.created_at),
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_construction_and_profile_isolation() {
        let sid = BrowserSessionId::new("sess-1").unwrap();
        let cid = CaseId::new("case-100").unwrap();
        let base = PathBuf::from("/tmp/pursue-test");

        let session = BrowserSession::new(
            sid.clone(),
            cid.clone(),
            "investigator@agency",
            RoutingMode::Tor,
            &base,
        )
        .unwrap();

        assert_eq!(session.id(), &sid);
        assert_eq!(session.case_id(), &cid);
        assert_eq!(session.actor(), "investigator@agency");
        assert_eq!(session.mode(), RoutingMode::Tor);
        assert!(session.is_active());
        assert_eq!(
            session.profile_dir(),
            base.join("profiles").join("case-100").join("sess-1")
        );
    }

    #[test]
    fn session_termination_lifecycle() {
        let sid = BrowserSessionId::new("sess-term").unwrap();
        let cid = CaseId::new("case-101").unwrap();
        let mut session = BrowserSession::new(
            sid,
            cid,
            "investigator@agency",
            RoutingMode::Direct,
            Path::new("/tmp"),
        )
        .unwrap();

        assert!(session.is_active());
        assert!(session.terminate().is_ok());
        assert!(session.status().is_terminated());
        assert!(!session.is_active());
        assert!(session.terminated_at().is_some());

        // Cannot terminate twice
        assert!(session.terminate().is_err());
    }

    #[test]
    fn rejects_empty_actor() {
        let sid = BrowserSessionId::new("sess-err").unwrap();
        let cid = CaseId::new("case-102").unwrap();
        assert!(
            BrowserSession::new(sid, cid, "  ", RoutingMode::Direct, Path::new("/tmp")).is_err()
        );
    }
}
