//! Terminal session model.
//!
//! A [`Session`] represents an investigator's interactive or scripted command-line
//! environment bound to an active investigation [`CaseId`]. All commands executed
//! in a session inherit the session's case context, working directory, and sanitized
//! environment.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use pursue_case::CaseId;
use pursue_core::{Error, Result};
use serde::{Deserialize, Serialize};

use crate::SessionId;

/// The lifecycle status of a terminal session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionStatus {
    /// The session is active and ready to execute commands.
    Active,
    /// The session has been terminated; no further commands may be executed.
    Terminated,
}

/// A terminal investigation session.
///
/// Encapsulates the session identifier, associated investigation case,
/// responsible investigator actor, initial working directory, sanitized
/// environment variables, and lifecycle status.
///
/// State changes occur only through audited methods (e.g. [`Session::terminate`]);
/// internal state is protected behind private fields with immutable accessors.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    id: SessionId,
    case_id: CaseId,
    actor: String,
    created_at_unix: u64,
    working_dir: PathBuf,
    env_vars: BTreeMap<String, String>,
    status: SessionStatus,
}

impl Session {
    /// Creates a new active session with the current system wall-clock timestamp.
    ///
    /// Validates that `actor` and `working_dir` are non-empty and that environment
    /// keys/values do not contain forbidden characters (such as NUL bytes).
    pub fn new(
        id: SessionId,
        case_id: CaseId,
        actor: &str,
        working_dir: PathBuf,
        env_vars: BTreeMap<String, String>,
    ) -> Result<Self> {
        Self::new_with_timestamp(id, case_id, actor, working_dir, env_vars, now_unix())
    }

    /// Creates a new active session with an explicit creation timestamp.
    ///
    /// Useful for deterministic testing and reproducible state reconstruction.
    pub fn new_with_timestamp(
        id: SessionId,
        case_id: CaseId,
        actor: &str,
        working_dir: PathBuf,
        env_vars: BTreeMap<String, String>,
        created_at_unix: u64,
    ) -> Result<Self> {
        let actor = actor.trim();
        if actor.is_empty() {
            return Err(Error::InvalidInput(
                "session actor must not be empty".into(),
            ));
        }
        if actor.contains('\0') {
            return Err(Error::InvalidInput(
                "session actor must not contain NUL bytes".into(),
            ));
        }
        if working_dir.as_os_str().is_empty() {
            return Err(Error::InvalidInput(
                "session working directory must not be empty".into(),
            ));
        }
        validate_env_vars(&env_vars)?;

        Ok(Self {
            id,
            case_id,
            actor: actor.to_string(),
            created_at_unix,
            working_dir,
            env_vars,
            status: SessionStatus::Active,
        })
    }

    /// The unique session identifier.
    pub fn id(&self) -> &SessionId {
        &self.id
    }

    /// The investigation case this session belongs to.
    pub fn case_id(&self) -> &CaseId {
        &self.case_id
    }

    /// The investigator actor responsible for this session.
    pub fn actor(&self) -> &str {
        &self.actor
    }

    /// Unix timestamp (seconds) when the session was created.
    pub fn created_at_unix(&self) -> u64 {
        self.created_at_unix
    }

    /// The session's working directory.
    pub fn working_dir(&self) -> &Path {
        &self.working_dir
    }

    /// The session's environment variables.
    pub fn env_vars(&self) -> &BTreeMap<String, String> {
        &self.env_vars
    }

    /// The current lifecycle status of the session.
    pub fn status(&self) -> SessionStatus {
        self.status
    }

    /// Returns `true` if the session is currently [`SessionStatus::Active`].
    pub fn is_active(&self) -> bool {
        self.status == SessionStatus::Active
    }

    /// Explicitly terminates the session.
    ///
    /// Fails with [`Error::InvalidInput`] if the session is already terminated.
    pub fn terminate(&mut self) -> Result<()> {
        if self.status == SessionStatus::Terminated {
            return Err(Error::InvalidInput(format!(
                "session {} is already terminated",
                self.id
            )));
        }
        self.status = SessionStatus::Terminated;
        Ok(())
    }
}

/// Validates that environment variable names and values adhere to POSIX and
/// cross-platform process execution safety standards.
fn validate_env_vars(env: &BTreeMap<String, String>) -> Result<()> {
    for (k, v) in env {
        let key = k.trim();
        if key.is_empty() {
            return Err(Error::InvalidInput(
                "environment variable name must not be empty".into(),
            ));
        }
        if key.contains('=') {
            return Err(Error::InvalidInput(format!(
                "environment variable name {k:?} cannot contain '='"
            )));
        }
        if key.contains('\0') || v.contains('\0') {
            return Err(Error::InvalidInput(format!(
                "environment variable {k:?} contains forbidden NUL bytes"
            )));
        }
    }
    Ok(())
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
    use std::path::PathBuf;

    fn test_id() -> SessionId {
        SessionId::new("term-01").unwrap()
    }

    fn test_case_id() -> CaseId {
        CaseId::new("case-alpha").unwrap()
    }

    #[test]
    fn valid_session_construction() {
        let mut env = BTreeMap::new();
        env.insert("PURSUE_MODE".into(), "investigation".into());
        let session = Session::new_with_timestamp(
            test_id(),
            test_case_id(),
            "investigator-1",
            PathBuf::from("/workspace"),
            env.clone(),
            1_700_000_000,
        )
        .unwrap();

        assert_eq!(session.id(), &test_id());
        assert_eq!(session.case_id(), &test_case_id());
        assert_eq!(session.actor(), "investigator-1");
        assert_eq!(session.working_dir(), Path::new("/workspace"));
        assert_eq!(session.created_at_unix(), 1_700_000_000);
        assert_eq!(session.status(), SessionStatus::Active);
        assert!(session.is_active());
        assert_eq!(
            session.env_vars().get("PURSUE_MODE").unwrap(),
            "investigation"
        );
    }

    #[test]
    fn empty_or_invalid_actor_rejected() {
        assert!(
            Session::new(
                test_id(),
                test_case_id(),
                "",
                PathBuf::from("/workspace"),
                BTreeMap::new(),
            )
            .is_err()
        );
        assert!(
            Session::new(
                test_id(),
                test_case_id(),
                "   ",
                PathBuf::from("/workspace"),
                BTreeMap::new(),
            )
            .is_err()
        );
        assert!(
            Session::new(
                test_id(),
                test_case_id(),
                "alice\x00x",
                PathBuf::from("/workspace"),
                BTreeMap::new(),
            )
            .is_err()
        );
    }

    #[test]
    fn empty_working_dir_rejected() {
        assert!(
            Session::new(
                test_id(),
                test_case_id(),
                "alice",
                PathBuf::from(""),
                BTreeMap::new(),
            )
            .is_err()
        );
    }

    #[test]
    fn invalid_env_vars_rejected() {
        let mut env_with_equal = BTreeMap::new();
        env_with_equal.insert("BAD=KEY".into(), "val".into());
        assert!(
            Session::new(
                test_id(),
                test_case_id(),
                "alice",
                PathBuf::from("/tmp"),
                env_with_equal,
            )
            .is_err()
        );

        let mut env_with_nul = BTreeMap::new();
        env_with_nul.insert("KEY".into(), "val\x00bad".into());
        assert!(
            Session::new(
                test_id(),
                test_case_id(),
                "alice",
                PathBuf::from("/tmp"),
                env_with_nul,
            )
            .is_err()
        );
    }

    #[test]
    fn termination_lifecycle() {
        let mut session = Session::new(
            test_id(),
            test_case_id(),
            "alice",
            PathBuf::from("/workspace"),
            BTreeMap::new(),
        )
        .unwrap();

        assert!(session.is_active());
        assert_eq!(session.status(), SessionStatus::Active);

        session.terminate().unwrap();
        assert!(!session.is_active());
        assert_eq!(session.status(), SessionStatus::Terminated);

        // Terminating already terminated session returns an error.
        assert!(session.terminate().is_err());
    }

    #[test]
    fn serde_roundtrip() {
        let mut env = BTreeMap::new();
        env.insert("FOO".into(), "bar".into());
        let session = Session::new_with_timestamp(
            test_id(),
            test_case_id(),
            "alice",
            PathBuf::from("/home/pursue"),
            env,
            1_700_000_123,
        )
        .unwrap();

        let json = serde_json::to_string(&session).unwrap();
        let loaded: Session = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded, session);
        assert_eq!(loaded.status(), SessionStatus::Active);
    }
}
