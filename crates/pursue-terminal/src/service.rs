//! Terminal runtime service implementation.
//!
//! Provides [`TerminalService`], integrating the Investigation Terminal into the
//! [`pursue_runtime::service::Service`] lifecycle architecture (`init -> start -> run -> shutdown`).
//!
//! Tracks active terminal sessions in memory and ensures that all child processes and
//! open sessions are terminated cleanly when the runtime initiates shutdown.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use pursue_core::{Error, Result};
use pursue_runtime::service::{Service, ServiceContext};

use crate::session::Session;
use crate::session_id::SessionId;

/// Terminal investigation service managing session state and lifecycle.
#[derive(Clone, Default)]
pub struct TerminalService {
    sessions: Arc<Mutex<BTreeMap<SessionId, Session>>>,
}

impl TerminalService {
    /// Creates a new terminal service.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers and stores a new terminal session.
    ///
    /// Fails with [`Error::InvalidInput`] if a session with the same identifier already exists.
    pub fn register_session(&self, session: Session) -> Result<()> {
        let mut map = self.sessions.lock().unwrap();
        if map.contains_key(session.id()) {
            return Err(Error::InvalidInput(format!(
                "session {} already registered",
                session.id()
            )));
        }
        map.insert(session.id().clone(), session);
        Ok(())
    }

    /// Retrieves an immutable copy of a session by identifier.
    ///
    /// Fails with [`Error::NotFound`] if the session does not exist.
    pub fn get_session(&self, id: &SessionId) -> Result<Session> {
        self.sessions
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("session {id} not found")))
    }

    /// Terminates a registered session by identifier.
    pub fn terminate_session(&self, id: &SessionId) -> Result<()> {
        let mut map = self.sessions.lock().unwrap();
        let session = map
            .get_mut(id)
            .ok_or_else(|| Error::NotFound(format!("session {id} not found")))?;
        session.terminate()
    }

    /// Returns the number of currently registered sessions.
    pub fn session_count(&self) -> usize {
        self.sessions.lock().unwrap().len()
    }

    /// Returns `true` if a session with `id` is registered.
    pub fn has_session(&self, id: &SessionId) -> bool {
        self.sessions.lock().unwrap().contains_key(id)
    }
}

impl Service for TerminalService {
    fn name(&self) -> &str {
        "terminal"
    }

    fn init(&mut self, _ctx: &ServiceContext) -> Result<()> {
        Ok(())
    }

    fn start(&mut self, _ctx: &ServiceContext) -> Result<()> {
        Ok(())
    }

    fn run(&mut self, ctx: &ServiceContext) -> Result<()> {
        while !ctx.shutdown_requested() {
            thread::sleep(Duration::from_millis(50));
        }
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        let mut map = self.sessions.lock().unwrap();
        for session in map.values_mut() {
            if session.is_active() {
                let _ = session.terminate();
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use pursue_case::CaseId;
    use pursue_runtime::log::{Level, Logger, TestSink};
    use pursue_runtime::service::Runtime;

    fn sample_session(name: &str) -> Session {
        Session::new(
            SessionId::new(name).unwrap(),
            CaseId::new("case-service").unwrap(),
            "analyst",
            PathBuf::from("/workspace"),
            BTreeMap::new(),
        )
        .unwrap()
    }

    #[test]
    fn register_get_and_terminate_session() {
        let service = TerminalService::new();
        let s = sample_session("term-srv-01");
        service.register_session(s).unwrap();

        assert_eq!(service.session_count(), 1);
        assert!(service.has_session(&SessionId::new("term-srv-01").unwrap()));

        let fetched = service
            .get_session(&SessionId::new("term-srv-01").unwrap())
            .unwrap();
        assert!(fetched.is_active());

        service
            .terminate_session(&SessionId::new("term-srv-01").unwrap())
            .unwrap();
        let after = service
            .get_session(&SessionId::new("term-srv-01").unwrap())
            .unwrap();
        assert!(!after.is_active());
    }

    #[test]
    fn runtime_lifecycle_terminates_active_sessions_on_shutdown() {
        let logger = Logger::new(Level::Info, "test", Arc::new(TestSink::new())).unwrap();
        let runtime = Runtime::new(logger);

        let mut service = TerminalService::new();
        let s = sample_session("term-term-on-shutdown");
        service.register_session(s).unwrap();

        let srv_clone = service.clone();
        runtime.request_shutdown();
        runtime.run(&mut service).unwrap();

        let after = srv_clone
            .get_session(&SessionId::new("term-term-on-shutdown").unwrap())
            .unwrap();
        assert!(!after.is_active());
    }
}
