//! Command execution abstraction, deterministic mock executor, and process executor.
//!
//! Provides the [`CommandExecutor`] trait along with two concrete implementations:
//! - [`MockExecutor`]: In-memory, deterministic executor for unit/integration testing,
//!   simulating arbitrary stdout/stderr, exit codes, timeouts, and truncations without
//!   spawning real OS processes.
//! - [`ProcessExecutor`]: Production executor spawning real host processes via
//!   [`std::process::Command`]. Subprocesses are invoked directly with array-based arguments
//!   (`argv`); shell evaluation (`/bin/sh -c`, `cmd.exe /c`) is strictly forbidden.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use pursue_core::{Error, Result};

use crate::command::{CommandRequest, ExecutionResult};
use crate::session::Session;

/// Trait-based abstraction for executing commands within a terminal session.
pub trait CommandExecutor: Send + Sync {
    /// Executes `request` in the context of `session`, returning the bounded execution result.
    fn execute(&self, session: &Session, request: &CommandRequest) -> Result<ExecutionResult>;
}

static COMMAND_COUNTER: AtomicU64 = AtomicU64::new(1);

fn generate_command_id() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let count = COMMAND_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("cmd-{now}-{count}")
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// A simulated execution outcome configured in [`MockExecutor`].
#[derive(Clone, Debug)]
pub enum MockOutcome {
    /// Return an [`ExecutionResult`].
    Success {
        /// Exit code to report.
        exit_code: Option<i32>,
        /// Bytes for standard output.
        stdout: Vec<u8>,
        /// Bytes for standard error.
        stderr: Vec<u8>,
        /// Whether the mock should report timeout.
        timed_out: bool,
        /// Whether the mock should report output truncation.
        truncated: bool,
    },
    /// Fail immediately with an error (e.g. executable not found).
    Error(String),
}

/// In-memory, deterministic mock executor for tests.
///
/// Records all execution requests and returns pre-programmed responses based on program name
/// or fallback default. Never touches the host OS process table.
#[derive(Default, Clone)]
pub struct MockExecutor {
    responses: Arc<Mutex<BTreeMap<String, MockOutcome>>>,
    default_outcome: Arc<Mutex<Option<MockOutcome>>>,
    calls: Arc<Mutex<Vec<(String, CommandRequest)>>>,
}

impl MockExecutor {
    /// Creates a new empty mock executor.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a mock outcome for a specific program name.
    pub fn register(&self, program: &str, outcome: MockOutcome) {
        self.responses
            .lock()
            .unwrap()
            .insert(program.to_string(), outcome);
    }

    /// Sets the default fallback outcome when no program-specific rule matches.
    pub fn set_default(&self, outcome: MockOutcome) {
        *self.default_outcome.lock().unwrap() = Some(outcome);
    }

    /// Returns a recorded history of `(session_id, request)` calls.
    pub fn recorded_calls(&self) -> Vec<(String, CommandRequest)> {
        self.calls.lock().unwrap().clone()
    }
}

impl CommandExecutor for MockExecutor {
    fn execute(&self, session: &Session, request: &CommandRequest) -> Result<ExecutionResult> {
        if !session.is_active() {
            return Err(Error::InvalidInput(format!(
                "cannot execute in terminated session {}",
                session.id()
            )));
        }

        self.calls
            .lock()
            .unwrap()
            .push((session.id().to_string(), request.clone()));

        let outcome = {
            let map = self.responses.lock().unwrap();
            map.get(request.program())
                .cloned()
                .or_else(|| self.default_outcome.lock().unwrap().clone())
        };

        let now = now_unix();
        let cmd_id = generate_command_id();

        match outcome {
            Some(MockOutcome::Success {
                exit_code,
                stdout,
                stderr,
                timed_out,
                truncated,
            }) => ExecutionResult::new(
                &cmd_id, exit_code, stdout, stderr, now, now, timed_out, truncated,
            ),
            Some(MockOutcome::Error(msg)) => Err(Error::ServiceFailure(msg)),
            None => {
                // Default echo behavior: return exit 0 with empty streams
                ExecutionResult::new(&cmd_id, Some(0), vec![], vec![], now, now, false, false)
            }
        }
    }
}

/// Production process executor that directly spawns OS processes without shell interpretation.
#[derive(Default, Clone, Copy, Debug)]
pub struct ProcessExecutor;

impl ProcessExecutor {
    /// Creates a new process executor.
    pub fn new() -> Self {
        Self
    }
}

/// Reads a stream up to `limit` bytes. If further bytes remain, drains and discards them
/// to avoid blocking the child process pipe, and marks `truncated = true`.
fn read_bounded<R: Read + Send + 'static>(mut reader: R, limit: usize) -> (Vec<u8>, bool) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut truncated = false;

    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                let space = limit.saturating_sub(buf.len());
                if space >= n {
                    buf.extend_from_slice(&chunk[..n]);
                } else {
                    if space > 0 {
                        buf.extend_from_slice(&chunk[..space]);
                    }
                    truncated = true;
                    // Drain remaining output to prevent child buffer blockage
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(_) => break,
        }
    }

    (buf, truncated)
}

impl CommandExecutor for ProcessExecutor {
    fn execute(&self, session: &Session, request: &CommandRequest) -> Result<ExecutionResult> {
        if !session.is_active() {
            return Err(Error::InvalidInput(format!(
                "cannot execute in terminated session {}",
                session.id()
            )));
        }

        let started_at = now_unix();
        let cmd_id = generate_command_id();

        // Resolve working directory: request override or session default
        let working_dir: &Path = request
            .working_dir()
            .unwrap_or_else(|| session.working_dir());
        if !working_dir.exists() {
            return Err(Error::NotFound(format!(
                "working directory does not exist: {}",
                working_dir.display()
            )));
        }

        // Configure std::process::Command directly (never invoke via sh -c or cmd.exe)
        let mut cmd = Command::new(request.program());
        cmd.args(request.args());
        cmd.current_dir(working_dir);

        // Apply session environment then command overrides
        for (k, v) in session.env_vars() {
            cmd.env(k, v);
        }
        for (k, v) in request.env_overrides() {
            cmd.env(k, v);
        }

        cmd.stdin(Stdio::null());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| {
            Error::ServiceFailure(format!(
                "failed to spawn process {:?}: {e}",
                request.program()
            ))
        })?;

        let stdout_pipe = child.stdout.take().expect("stdout was piped");
        let stderr_pipe = child.stderr.take().expect("stderr was piped");

        let max_output = request.effective_max_output_bytes();

        // Spawn stream reader threads to prevent deadlock on pipe saturation
        let stdout_handle = thread::spawn(move || read_bounded(stdout_pipe, max_output));
        let stderr_handle = thread::spawn(move || read_bounded(stderr_pipe, max_output));

        let timeout = Duration::from_secs(request.effective_timeout_secs());
        let deadline = Instant::now() + timeout;
        let mut timed_out = false;
        let mut exit_code: Option<i32> = None;

        // Poll child with timeout
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    exit_code = status.code();
                    break;
                }
                Ok(None) => {
                    if Instant::now() >= deadline {
                        timed_out = true;
                        let _ = child.kill();
                        let _ = child.wait();
                        break;
                    }
                    thread::sleep(Duration::from_millis(15));
                }
                Err(e) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(Error::Io(e));
                }
            }
        }

        let (stdout, stdout_trunc) = stdout_handle
            .join()
            .map_err(|_| Error::ServiceFailure("stdout reader thread panicked".into()))?;
        let (stderr, stderr_trunc) = stderr_handle
            .join()
            .map_err(|_| Error::ServiceFailure("stderr reader thread panicked".into()))?;

        let finished_at = now_unix().max(started_at);
        let truncated = stdout_trunc || stderr_trunc;

        ExecutionResult::new(
            &cmd_id,
            exit_code,
            stdout,
            stderr,
            started_at,
            finished_at,
            timed_out,
            truncated,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use pursue_case::CaseId;

    use crate::session::Session;
    use crate::session_id::SessionId;

    fn sample_session() -> Session {
        Session::new(
            SessionId::new("test-session").unwrap(),
            CaseId::new("case-01").unwrap(),
            "analyst",
            std::env::temp_dir(),
            BTreeMap::new(),
        )
        .unwrap()
    }

    #[test]
    fn mock_executor_routes_and_records() {
        let mock = MockExecutor::new();
        mock.register(
            "whoami",
            MockOutcome::Success {
                exit_code: Some(0),
                stdout: b"investigator\n".to_vec(),
                stderr: vec![],
                timed_out: false,
                truncated: false,
            },
        );

        let session = sample_session();
        let req = CommandRequest::new("whoami", vec![]).unwrap();
        let res = mock.execute(&session, &req).unwrap();

        assert_eq!(res.exit_code(), Some(0));
        assert_eq!(res.stdout(), b"investigator\n");
        assert!(res.is_success());

        let recorded = mock.recorded_calls();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].1.program(), "whoami");
    }

    #[test]
    fn mock_executor_simulates_failure_and_timeout() {
        let mock = MockExecutor::new();
        mock.register(
            "hang",
            MockOutcome::Success {
                exit_code: None,
                stdout: vec![],
                stderr: b"process killed\n".to_vec(),
                timed_out: true,
                truncated: false,
            },
        );

        let session = sample_session();
        let req = CommandRequest::new("hang", vec![]).unwrap();
        let res = mock.execute(&session, &req).unwrap();

        assert!(!res.is_success());
        assert!(res.timed_out());
        assert_eq!(res.exit_code(), None);
        assert_eq!(res.stderr(), b"process killed\n");
    }

    #[test]
    fn mock_executor_rejects_terminated_session() {
        let mock = MockExecutor::new();
        let mut session = sample_session();
        session.terminate().unwrap();

        let req = CommandRequest::new("test", vec![]).unwrap();
        assert!(mock.execute(&session, &req).is_err());
    }

    #[test]
    fn process_executor_runs_real_command() {
        let executor = ProcessExecutor::new();
        let session = sample_session();

        // Use a cross-platform command available everywhere in Rust test environments
        // "cargo" or "rustc --version" is guaranteed to be present on dev & CI systems
        let req = CommandRequest::new("rustc", vec!["--version".into()]).unwrap();
        let res = executor.execute(&session, &req).unwrap();

        assert!(res.is_success());
        assert_eq!(res.exit_code(), Some(0));
        let out_str = String::from_utf8_lossy(res.stdout());
        assert!(out_str.contains("rustc"));
    }

    #[test]
    fn process_executor_nonexistent_program_fails_cleanly() {
        let executor = ProcessExecutor::new();
        let session = sample_session();
        let req = CommandRequest::new("definitely_nonexistent_executable_12345", vec![]).unwrap();
        let err = executor.execute(&session, &req).unwrap_err();
        assert!(matches!(err, Error::ServiceFailure(_)));
    }

    #[test]
    fn process_executor_invalid_working_dir_fails() {
        let executor = ProcessExecutor::new();
        let session = sample_session();
        let req = CommandRequest::new("rustc", vec!["--version".into()])
            .unwrap()
            .with_working_dir(PathBuf::from("/nonexistent/directory/path/123"))
            .unwrap();
        let err = executor.execute(&session, &req).unwrap_err();
        assert!(matches!(err, Error::NotFound(_)));
    }
}
