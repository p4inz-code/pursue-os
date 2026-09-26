//! Browser service lifecycle implementation.
//!
//! Provides [`BrowserService`], conforming to [`pursue_runtime::service::Service`]
//! to manage the lifecycle of browser sessions within the PURSUE OS runtime.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use pursue_case::CaseId;
use pursue_core::{Error, Result};
use pursue_runtime::service::{Service, ServiceContext};

use crate::routing::RoutingMode;
use crate::session::BrowserSession;
use crate::session_id::BrowserSessionId;

/// Managed runtime service supervising investigative browser sessions.
#[derive(Clone)]
pub struct BrowserService {
    sessions: Arc<RwLock<BTreeMap<BrowserSessionId, BrowserSession>>>,
    base_storage_dir: PathBuf,
}

impl BrowserService {
    /// Creates a new browser service instance with a given base profile directory.
    pub fn new(base_storage_dir: PathBuf) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(BTreeMap::new())),
            base_storage_dir,
        }
    }

    /// Registers a new active browser session.
    pub fn create_session(
        &self,
        id: BrowserSessionId,
        case_id: CaseId,
        actor: &str,
        mode: RoutingMode,
    ) -> Result<BrowserSession> {
        let mut map = self
            .sessions
            .write()
            .map_err(|_| Error::ServiceFailure("browser session lock poisoned".into()))?;

        if map.contains_key(&id) {
            return Err(Error::InvalidInput(format!(
                "browser session '{id}' already exists"
            )));
        }

        let session =
            BrowserSession::new(id.clone(), case_id, actor, mode, &self.base_storage_dir)?;
        map.insert(id, session.clone());
        Ok(session)
    }

    /// Retrieves an existing browser session by ID.
    pub fn get_session(&self, id: &BrowserSessionId) -> Result<BrowserSession> {
        let map = self
            .sessions
            .read()
            .map_err(|_| Error::ServiceFailure("browser session lock poisoned".into()))?;

        map.get(id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("browser session '{id}' not found")))
    }

    /// Terminates an active browser session.
    pub fn terminate_session(&self, id: &BrowserSessionId) -> Result<()> {
        let mut map = self
            .sessions
            .write()
            .map_err(|_| Error::ServiceFailure("browser session lock poisoned".into()))?;

        let session = map
            .get_mut(id)
            .ok_or_else(|| Error::NotFound(format!("browser session '{id}' not found")))?;

        session.terminate()
    }

    /// Returns a list of all active browser sessions.
    pub fn list_active_sessions(&self) -> Result<Vec<BrowserSession>> {
        let map = self
            .sessions
            .read()
            .map_err(|_| Error::ServiceFailure("browser session lock poisoned".into()))?;

        Ok(map.values().filter(|s| s.is_active()).cloned().collect())
    }

    /// Returns the base storage directory.
    pub fn base_storage_dir(&self) -> &Path {
        &self.base_storage_dir
    }
}

impl Service for BrowserService {
    fn name(&self) -> &'static str {
        "browser"
    }

    fn init(&mut self, _ctx: &ServiceContext) -> Result<()> {
        // Ensure profile storage root directory exists
        let profile_root = self.base_storage_dir.join("profiles");
        std::fs::create_dir_all(&profile_root).map_err(Error::Io)?;
        Ok(())
    }

    fn start(&mut self, _ctx: &ServiceContext) -> Result<()> {
        Ok(())
    }

    fn run(&mut self, ctx: &ServiceContext) -> Result<()> {
        while !ctx.shutdown_requested() {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        // Terminate all remaining active browser sessions on shutdown
        if let Ok(mut map) = self.sessions.write() {
            for session in map.values_mut() {
                if session.is_active() {
                    let _ = session.terminate();
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_get_and_terminate_session() {
        let service = BrowserService::new(PathBuf::from("/tmp/pursue-service-test"));
        let sid = BrowserSessionId::new("sess-service-1").unwrap();
        let cid = CaseId::new("case-service-1").unwrap();

        let session = service
            .create_session(sid.clone(), cid.clone(), "investigator", RoutingMode::Tor)
            .unwrap();
        assert_eq!(session.id(), &sid);
        assert_eq!(session.mode(), RoutingMode::Tor);

        let retrieved = service.get_session(&sid).unwrap();
        assert_eq!(retrieved, session);

        // Terminate
        assert!(service.terminate_session(&sid).is_ok());
        let after_term = service.get_session(&sid).unwrap();
        assert!(after_term.status().is_terminated());

        // Duplicate registration fails
        assert!(
            service
                .create_session(sid, cid, "investigator", RoutingMode::Direct)
                .is_err()
        );
    }

    #[test]
    fn runtime_lifecycle_terminates_active_sessions_on_shutdown() {
        let mut service = BrowserService::new(PathBuf::from("/tmp/pursue-shutdown-test"));
        let sid = BrowserSessionId::new("sess-shutdown").unwrap();
        let cid = CaseId::new("case-shutdown").unwrap();

        service
            .create_session(sid.clone(), cid, "investigator", RoutingMode::Direct)
            .unwrap();

        let flag = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let ctx = ServiceContext::new(pursue_runtime::log::Logger::default(), flag);

        assert!(service.init(&ctx).is_ok());
        assert!(service.start(&ctx).is_ok());
        assert!(service.shutdown().is_ok());

        let session = service.get_session(&sid).unwrap();
        assert!(session.status().is_terminated());
    }
}
