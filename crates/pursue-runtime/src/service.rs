//! Minimal in-process service lifecycle.
//!
//! This is the application/service runtime boundary for PURSUE's Rust layer.
//! It is deliberately **not** a service manager: systemd remains the eventual
//! Linux supervisor (see `docs/architecture/SERVICE_MODEL.md`). A "service"
//! here is a single [`Service`] implementation driven through the phases
//! `init -> start -> run -> shutdown` by [`Runtime`].
//!
//! # Guarantees
//! - Once `init` succeeds, `shutdown` is always attempted, even if `start` or
//!   `run` fails.
//! - The first failing phase wins; its error is propagated to the caller.
//! - A shared shutdown flag lets long-running `run` implementations observe
//!   [`Runtime::request_shutdown`].

use pursue_core::{Error, Result};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::log::Logger;

/// The lifecycle state of a service as defined by the runtime contract.
///
/// The runtime itself does not track state (that would be a service manager);
/// this documents the states a service passes through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// Constructed but not yet initialized.
    Created,
    /// `init` succeeded.
    Initialized,
    /// `start` succeeded.
    Started,
    /// `run` is executing.
    Running,
    /// `shutdown` completed.
    Stopped,
    /// A lifecycle phase failed.
    Failed,
}

/// Context handed to services during their lifecycle.
///
/// Provides the shared logger and the runtime-wide shutdown flag. `run`
/// implementations should poll [`ServiceContext::shutdown_requested`] and
/// return promptly once it becomes true.
#[derive(Clone)]
pub struct ServiceContext {
    logger: Logger,
    shutdown: Arc<AtomicBool>,
}

impl ServiceContext {
    /// Creates a context from a logger and a shared shutdown flag.
    pub fn new(logger: Logger, shutdown: Arc<AtomicBool>) -> Self {
        Self { logger, shutdown }
    }

    /// The logger for this service's diagnostics.
    pub fn logger(&self) -> &Logger {
        &self.logger
    }

    /// Whether a shutdown has been requested for the runtime.
    pub fn shutdown_requested(&self) -> bool {
        self.shutdown.load(Ordering::Relaxed)
    }
}

/// A service driven through the PURSUE runtime lifecycle.
pub trait Service {
    /// Stable, unique name used in diagnostics.
    fn name(&self) -> &str;

    /// Validates dependencies and performs one-time initialization.
    fn init(&mut self, ctx: &ServiceContext) -> Result<()>;

    /// Acquires resources and prepares for `run`.
    fn start(&mut self, ctx: &ServiceContext) -> Result<()>;

    /// Performs the service's work. Should return once the work is complete
    /// or shutdown is requested.
    fn run(&mut self, ctx: &ServiceContext) -> Result<()>;

    /// Releases resources. Always called once `init` succeeded, even after a
    /// `start`/`run` failure.
    fn shutdown(&mut self) -> Result<()>;
}

/// Drives one or more services through their lifecycle.
///
/// Not a supervisor: no restart, no monitoring, no process spawning. For
/// system service supervision, PURSUE uses systemd on Linux.
#[derive(Clone)]
pub struct Runtime {
    logger: Logger,
    shutdown: Arc<AtomicBool>,
}

impl Runtime {
    /// Creates a runtime with its own shutdown flag.
    pub fn new(logger: Logger) -> Self {
        Self {
            logger,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// The shared shutdown flag (for services that need to poll it directly).
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.shutdown)
    }

    /// Requests a graceful shutdown of the current run.
    pub fn request_shutdown(&self) {
        self.shutdown.store(true, Ordering::Relaxed);
    }

    /// Runs a single service through `init -> start -> run -> shutdown`.
    ///
    /// On an `init` failure the service is left unstarted and `shutdown` is
    /// not called. After a successful `init`, `shutdown` is always attempted.
    pub fn run(&self, service: &mut dyn Service) -> Result<()> {
        let ctx = ServiceContext::new(self.logger.clone(), Arc::clone(&self.shutdown));
        service
            .init(&ctx)
            .map_err(|e| self.phase_error(service, "init", e))?;

        let failure = match service.start(&ctx) {
            Ok(()) => service
                .run(&ctx)
                .err()
                .map(|e| self.phase_error(service, "run", e)),
            Err(e) => Some(self.phase_error(service, "start", e)),
        };
        let shutdown_err = service
            .shutdown()
            .err()
            .map(|e| self.phase_error(service, "shutdown", e));

        match (failure, shutdown_err) {
            (Some(e), _) => Err(e),
            (None, Some(e)) => Err(e),
            (None, None) => Ok(()),
        }
    }

    /// Runs each service in order, stopping at the first failing service.
    ///
    /// The failing service's own `shutdown` is still attempted by
    /// [`Runtime::run`]; services after it are not started.
    pub fn run_all(&self, services: &mut [Box<dyn Service>]) -> Result<()> {
        for service in services {
            self.run(service.as_mut())?;
        }
        Ok(())
    }

    fn phase_error(&self, service: &dyn Service, phase: &str, err: Error) -> Error {
        Error::ServiceFailure(format!("{} failed during {phase}: {err}", service.name()))
    }
}

#[cfg(test)]
mod tests {
    use super::{Runtime, Service, ServiceContext};
    use crate::log::{Level, Logger, TestSink};
    use pursue_core::{Error, Result};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};

    /// Records lifecycle calls; optionally fails one phase.
    struct RecordingService {
        events: Arc<Mutex<Vec<String>>>,
        fail: Option<&'static str>,
    }

    impl RecordingService {
        fn new(events: Arc<Mutex<Vec<String>>>) -> Self {
            Self { events, fail: None }
        }

        fn failing_at(mut self, phase: &'static str) -> Self {
            self.fail = Some(phase);
            self
        }
    }

    impl Service for RecordingService {
        fn name(&self) -> &str {
            "recorder"
        }

        fn init(&mut self, _ctx: &ServiceContext) -> Result<()> {
            self.events.lock().unwrap().push("init".into());
            if self.fail == Some("init") {
                return Err(Error::InvalidInput("init exploded".into()));
            }
            Ok(())
        }

        fn start(&mut self, _ctx: &ServiceContext) -> Result<()> {
            self.events.lock().unwrap().push("start".into());
            if self.fail == Some("start") {
                return Err(Error::InvalidInput("start exploded".into()));
            }
            Ok(())
        }

        fn run(&mut self, _ctx: &ServiceContext) -> Result<()> {
            self.events.lock().unwrap().push("run".into());
            if self.fail == Some("run") {
                return Err(Error::InvalidInput("run exploded".into()));
            }
            Ok(())
        }

        fn shutdown(&mut self) -> Result<()> {
            self.events.lock().unwrap().push("shutdown".into());
            if self.fail == Some("shutdown") {
                return Err(Error::InvalidInput("shutdown exploded".into()));
            }
            Ok(())
        }
    }

    /// A service that records whether the shutdown flag was already set when
    /// it ran.
    struct FlagAwareService {
        observed: Arc<AtomicBool>,
    }

    impl Service for FlagAwareService {
        fn name(&self) -> &str {
            "flag-aware"
        }

        fn init(&mut self, _ctx: &ServiceContext) -> Result<()> {
            Ok(())
        }

        fn start(&mut self, _ctx: &ServiceContext) -> Result<()> {
            Ok(())
        }

        fn run(&mut self, ctx: &ServiceContext) -> Result<()> {
            self.observed
                .store(ctx.shutdown_requested(), Ordering::Relaxed);
            Ok(())
        }

        fn shutdown(&mut self) -> Result<()> {
            Ok(())
        }
    }

    fn runtime() -> Runtime {
        let logger = Logger::new(Level::Info, "test", Arc::new(TestSink::new())).unwrap();
        Runtime::new(logger)
    }

    #[test]
    fn happy_path_runs_full_lifecycle_in_order() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut service = RecordingService::new(Arc::clone(&events));
        runtime().run(&mut service).unwrap();
        assert_eq!(
            *events.lock().unwrap(),
            vec!["init", "start", "run", "shutdown"]
        );
    }

    #[test]
    fn init_failure_skips_shutdown() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut service = RecordingService::new(Arc::clone(&events)).failing_at("init");
        let err = runtime().run(&mut service).unwrap_err();
        assert!(err.to_string().contains("init"));
        assert_eq!(*events.lock().unwrap(), vec!["init"]);
    }

    #[test]
    fn start_failure_still_calls_shutdown() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut service = RecordingService::new(Arc::clone(&events)).failing_at("start");
        let err = runtime().run(&mut service).unwrap_err();
        assert!(err.to_string().contains("start"));
        assert_eq!(*events.lock().unwrap(), vec!["init", "start", "shutdown"]);
    }

    #[test]
    fn run_failure_still_calls_shutdown() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut service = RecordingService::new(Arc::clone(&events)).failing_at("run");
        let err = runtime().run(&mut service).unwrap_err();
        assert!(err.to_string().contains("run"));
        assert_eq!(
            *events.lock().unwrap(),
            vec!["init", "start", "run", "shutdown"]
        );
    }

    #[test]
    fn shutdown_failure_propagates() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut service = RecordingService::new(Arc::clone(&events)).failing_at("shutdown");
        let err = runtime().run(&mut service).unwrap_err();
        assert!(err.to_string().contains("shutdown"));
        assert_eq!(
            *events.lock().unwrap(),
            vec!["init", "start", "run", "shutdown"]
        );
    }

    #[test]
    fn error_names_the_service_and_phase() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut service = RecordingService::new(events).failing_at("run");
        let err = runtime().run(&mut service).unwrap_err();
        let message = err.to_string();
        assert!(message.contains("recorder"));
        assert!(message.contains("run"));
    }

    #[test]
    fn shutdown_flag_is_visible_to_services() {
        let observed = Arc::new(AtomicBool::new(false));
        let mut service = FlagAwareService {
            observed: Arc::clone(&observed),
        };
        let runtime = runtime();
        runtime.request_shutdown();
        runtime.run(&mut service).unwrap();
        assert!(observed.load(Ordering::Relaxed));
    }

    #[test]
    fn run_all_stops_at_first_failure() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let mut services: Vec<Box<dyn Service>> = vec![
            Box::new(RecordingService::new(Arc::clone(&events))),
            Box::new(RecordingService::new(Arc::clone(&events)).failing_at("run")),
            Box::new(RecordingService::new(Arc::clone(&events))),
        ];
        assert!(runtime().run_all(&mut services).is_err());
        assert_eq!(
            *events.lock().unwrap(),
            vec![
                "init", "start", "run", "shutdown", "init", "start", "run", "shutdown"
            ]
        );
    }
}
