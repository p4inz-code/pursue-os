//! Command execution requests and result models.
//!
//! Provides the data structures for invoking commands ([`CommandRequest`]) and
//! recording their execution outcomes ([`ExecutionResult`]).
//!
//! # Security Invariants
//! - Programs must be distinct executable names or paths; passing shell strings
//!   with embedded whitespace or shell metacharacters is rejected at the model
//!   boundary.
//! - Arguments remain discrete elements in an array (`argv`); shell expansion is
//!   not performed.
//! - Output buffer sizes and timeouts are strictly bounded to prevent resource
//!   exhaustion.
//! - This module defines data models only; it does not spawn processes or execute
//!   commands.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use pursue_core::{Error, Result};
use serde::{Deserialize, Serialize};

/// Default batch execution timeout (60 seconds).
pub const DEFAULT_TIMEOUT_SECS: u64 = 60;

/// Maximum permissible execution timeout (24 hours).
pub const MAX_TIMEOUT_SECS: u64 = 86_400;

/// Default stream output capture buffer limit (16 MiB).
pub const DEFAULT_OUTPUT_LIMIT: usize = 16 * 1024 * 1024;

/// Absolute maximum stream output capture buffer limit (64 MiB).
pub const MAX_OUTPUT_LIMIT: usize = 64 * 1024 * 1024;

/// A validated request to execute a program in a terminal session.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRequest {
    program: String,
    args: Vec<String>,
    working_dir: Option<PathBuf>,
    env_overrides: BTreeMap<String, String>,
    timeout_secs: Option<u64>,
    max_output_bytes: Option<usize>,
}

impl CommandRequest {
    /// Creates a new command request with default limits and no environment overrides.
    ///
    /// Validates that:
    /// - `program` is non-empty after trimming.
    /// - `program` does not contain whitespace (preventing accidentally passing
    ///   shell commands like `"ls -la"` as the program name).
    /// - Neither `program` nor any argument contains NUL bytes.
    pub fn new(program: &str, args: Vec<String>) -> Result<Self> {
        let program = program.trim();
        if program.is_empty() {
            return Err(Error::InvalidInput("program name must not be empty".into()));
        }
        if program.chars().any(char::is_whitespace) {
            return Err(Error::InvalidInput(format!(
                "program name {program:?} cannot contain whitespace; arguments must be passed separately"
            )));
        }
        if program.contains('\0') {
            return Err(Error::InvalidInput(
                "program name must not contain NUL bytes".into(),
            ));
        }
        for (idx, arg) in args.iter().enumerate() {
            if arg.contains('\0') {
                return Err(Error::InvalidInput(format!(
                    "argument at index {idx} contains forbidden NUL bytes"
                )));
            }
        }

        Ok(Self {
            program: program.to_string(),
            args,
            working_dir: None,
            env_overrides: BTreeMap::new(),
            timeout_secs: None,
            max_output_bytes: None,
        })
    }

    /// Sets an explicit working directory for this command.
    pub fn with_working_dir(mut self, dir: PathBuf) -> Result<Self> {
        if dir.as_os_str().is_empty() {
            return Err(Error::InvalidInput(
                "command working directory must not be empty".into(),
            ));
        }
        self.working_dir = Some(dir);
        Ok(self)
    }

    /// Sets environment variable overrides for this command.
    pub fn with_env_overrides(mut self, env: BTreeMap<String, String>) -> Result<Self> {
        for (k, v) in &env {
            let key = k.trim();
            if key.is_empty() {
                return Err(Error::InvalidInput(
                    "environment override variable name must not be empty".into(),
                ));
            }
            if key.contains('=') {
                return Err(Error::InvalidInput(format!(
                    "environment override variable name {k:?} cannot contain '='"
                )));
            }
            if key.contains('\0') || v.contains('\0') {
                return Err(Error::InvalidInput(format!(
                    "environment override variable {k:?} contains forbidden NUL bytes"
                )));
            }
        }
        self.env_overrides = env;
        Ok(self)
    }

    /// Sets an execution timeout in seconds. Must be between 1 and [`MAX_TIMEOUT_SECS`].
    pub fn with_timeout_secs(mut self, secs: u64) -> Result<Self> {
        if secs == 0 || secs > MAX_TIMEOUT_SECS {
            return Err(Error::InvalidInput(format!(
                "timeout_secs must be between 1 and {MAX_TIMEOUT_SECS}, got {secs}"
            )));
        }
        self.timeout_secs = Some(secs);
        Ok(self)
    }

    /// Sets the maximum output capture buffer size in bytes. Must be between 1 and [`MAX_OUTPUT_LIMIT`].
    pub fn with_max_output_bytes(mut self, bytes: usize) -> Result<Self> {
        if bytes == 0 || bytes > MAX_OUTPUT_LIMIT {
            return Err(Error::InvalidInput(format!(
                "max_output_bytes must be between 1 and {MAX_OUTPUT_LIMIT}, got {bytes}"
            )));
        }
        self.max_output_bytes = Some(bytes);
        Ok(self)
    }

    /// The executable program name or path.
    pub fn program(&self) -> &str {
        &self.program
    }

    /// The discrete argument list.
    pub fn args(&self) -> &[String] {
        &self.args
    }

    /// The explicit working directory, if overridden.
    pub fn working_dir(&self) -> Option<&Path> {
        self.working_dir.as_deref()
    }

    /// The environment variable overrides.
    pub fn env_overrides(&self) -> &BTreeMap<String, String> {
        &self.env_overrides
    }

    /// Configured execution timeout in seconds, if specified.
    pub fn timeout_secs(&self) -> Option<u64> {
        self.timeout_secs
    }

    /// The effective timeout in seconds (configured or default [`DEFAULT_TIMEOUT_SECS`]).
    pub fn effective_timeout_secs(&self) -> u64 {
        self.timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS)
    }

    /// Configured maximum output buffer size in bytes, if specified.
    pub fn max_output_bytes(&self) -> Option<usize> {
        self.max_output_bytes
    }

    /// The effective maximum output buffer size (configured or default [`DEFAULT_OUTPUT_LIMIT`]).
    pub fn effective_max_output_bytes(&self) -> usize {
        self.max_output_bytes.unwrap_or(DEFAULT_OUTPUT_LIMIT)
    }
}

/// The recorded result of a command execution.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionResult {
    command_id: String,
    exit_code: Option<i32>,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    started_at_unix: u64,
    finished_at_unix: u64,
    timed_out: bool,
    truncated: bool,
}

impl ExecutionResult {
    /// Constructs a new execution result.
    ///
    /// Validates that:
    /// - `command_id` is non-empty after trimming and contains no NUL bytes.
    /// - `finished_at_unix >= started_at_unix`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        command_id: &str,
        exit_code: Option<i32>,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
        started_at_unix: u64,
        finished_at_unix: u64,
        timed_out: bool,
        truncated: bool,
    ) -> Result<Self> {
        let command_id = command_id.trim();
        if command_id.is_empty() {
            return Err(Error::InvalidInput("command id must not be empty".into()));
        }
        if command_id.contains('\0') {
            return Err(Error::InvalidInput(
                "command id must not contain NUL bytes".into(),
            ));
        }
        if finished_at_unix < started_at_unix {
            return Err(Error::InvalidInput(format!(
                "finished_at_unix ({finished_at_unix}) cannot be before started_at_unix ({started_at_unix})"
            )));
        }

        Ok(Self {
            command_id: command_id.to_string(),
            exit_code,
            stdout,
            stderr,
            started_at_unix,
            finished_at_unix,
            timed_out,
            truncated,
        })
    }

    /// Unique identifier for this command invocation.
    pub fn command_id(&self) -> &str {
        &self.command_id
    }

    /// Process exit code (`None` if process terminated by signal or timeout).
    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    /// Raw bytes emitted to standard output.
    pub fn stdout(&self) -> &[u8] {
        &self.stdout
    }

    /// Raw bytes emitted to standard error.
    pub fn stderr(&self) -> &[u8] {
        &self.stderr
    }

    /// Unix timestamp (seconds) when execution started.
    pub fn started_at_unix(&self) -> u64 {
        self.started_at_unix
    }

    /// Unix timestamp (seconds) when execution finished.
    pub fn finished_at_unix(&self) -> u64 {
        self.finished_at_unix
    }

    /// Duration of execution in seconds.
    pub fn duration_secs(&self) -> u64 {
        self.finished_at_unix.saturating_sub(self.started_at_unix)
    }

    /// Whether the process was killed due to reaching the timeout.
    pub fn timed_out(&self) -> bool {
        self.timed_out
    }

    /// Whether process output was truncated because it exceeded the output buffer limit.
    pub fn is_truncated(&self) -> bool {
        self.truncated
    }

    /// Returns `true` if the command completed with an exit code of `0` and did not time out.
    pub fn is_success(&self) -> bool {
        !self.timed_out && self.exit_code == Some(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_command_request() {
        let req = CommandRequest::new("curl", vec!["-s".into(), "https://example.com".into()])
            .unwrap()
            .with_working_dir(PathBuf::from("/workspace"))
            .unwrap()
            .with_timeout_secs(30)
            .unwrap()
            .with_max_output_bytes(1024 * 1024)
            .unwrap();

        assert_eq!(req.program(), "curl");
        assert_eq!(req.args(), &["-s", "https://example.com"]);
        assert_eq!(req.working_dir(), Some(Path::new("/workspace")));
        assert_eq!(req.timeout_secs(), Some(30));
        assert_eq!(req.effective_timeout_secs(), 30);
        assert_eq!(req.max_output_bytes(), Some(1024 * 1024));
        assert_eq!(req.effective_max_output_bytes(), 1024 * 1024);
    }

    #[test]
    fn defaults_for_unspecified_options() {
        let req = CommandRequest::new("whoami", vec![]).unwrap();
        assert_eq!(req.working_dir(), None);
        assert!(req.env_overrides().is_empty());
        assert_eq!(req.timeout_secs(), None);
        assert_eq!(req.effective_timeout_secs(), DEFAULT_TIMEOUT_SECS);
        assert_eq!(req.max_output_bytes(), None);
        assert_eq!(req.effective_max_output_bytes(), DEFAULT_OUTPUT_LIMIT);
    }

    #[test]
    fn program_with_whitespace_is_rejected() {
        // Must reject shell-style combined strings like "ls -la"
        assert!(CommandRequest::new("ls -la", vec![]).is_err());
        assert!(CommandRequest::new("python script.py", vec![]).is_err());
        assert!(CommandRequest::new("sh\nc", vec![]).is_err());
    }

    #[test]
    fn empty_program_is_rejected() {
        assert!(CommandRequest::new("", vec![]).is_err());
        assert!(CommandRequest::new("   ", vec![]).is_err());
    }

    #[test]
    fn nul_bytes_rejected_in_program_or_args() {
        assert!(CommandRequest::new("ls\0", vec![]).is_err());
        assert!(CommandRequest::new("ls", vec!["-l\0".into()]).is_err());
    }

    #[test]
    fn timeout_and_output_limits_validation() {
        let req = CommandRequest::new("ls", vec![]).unwrap();
        assert!(req.clone().with_timeout_secs(0).is_err());
        assert!(req.clone().with_timeout_secs(MAX_TIMEOUT_SECS + 1).is_err());
        assert!(req.clone().with_max_output_bytes(0).is_err());
        assert!(
            req.clone()
                .with_max_output_bytes(MAX_OUTPUT_LIMIT + 1)
                .is_err()
        );
    }

    #[test]
    fn execution_result_construction_and_accessors() {
        let res = ExecutionResult::new(
            "cmd-01",
            Some(0),
            b"output data".to_vec(),
            b"warning message".to_vec(),
            100,
            105,
            false,
            false,
        )
        .unwrap();

        assert_eq!(res.command_id(), "cmd-01");
        assert_eq!(res.exit_code(), Some(0));
        assert_eq!(res.stdout(), b"output data");
        assert_eq!(res.stderr(), b"warning message");
        assert_eq!(res.started_at_unix(), 100);
        assert_eq!(res.finished_at_unix(), 105);
        assert_eq!(res.duration_secs(), 5);
        assert!(!res.timed_out());
        assert!(!res.is_truncated());
        assert!(res.is_success());
    }

    #[test]
    fn execution_result_failure_states() {
        // Non-zero exit code
        let fail = ExecutionResult::new("cmd-02", Some(1), vec![], vec![], 100, 101, false, false)
            .unwrap();
        assert!(!fail.is_success());
        assert_eq!(fail.exit_code(), Some(1));

        // Timed out with None exit code
        let timeout =
            ExecutionResult::new("cmd-03", None, vec![], vec![], 100, 160, true, false).unwrap();
        assert!(!timeout.is_success());
        assert!(timeout.timed_out());
        assert_eq!(timeout.exit_code(), None);

        // Truncated output
        let trunc = ExecutionResult::new(
            "cmd-04",
            Some(0),
            vec![0u8; 100],
            vec![],
            100,
            101,
            false,
            true,
        )
        .unwrap();
        assert!(trunc.is_success()); // Exit 0, but truncated
        assert!(trunc.is_truncated());
    }

    #[test]
    fn execution_result_invalid_timestamps_rejected() {
        assert!(
            ExecutionResult::new("cmd", Some(0), vec![], vec![], 100, 99, false, false).is_err()
        );
    }

    #[test]
    fn execution_result_empty_command_id_rejected() {
        assert!(ExecutionResult::new("", Some(0), vec![], vec![], 100, 100, false, false).is_err());
        assert!(
            ExecutionResult::new("  ", Some(0), vec![], vec![], 100, 100, false, false).is_err()
        );
    }

    #[test]
    fn command_request_and_execution_result_serde_roundtrip() {
        let req = CommandRequest::new("cat", vec!["file.txt".into()]).unwrap();
        let json_req = serde_json::to_string(&req).unwrap();
        let loaded_req: CommandRequest = serde_json::from_str(&json_req).unwrap();
        assert_eq!(loaded_req, req);

        let res = ExecutionResult::new(
            "cmd-roundtrip",
            Some(42),
            vec![1, 2, 3],
            vec![4, 5],
            1_700_000_000,
            1_700_000_002,
            false,
            false,
        )
        .unwrap();
        let json_res = serde_json::to_string(&res).unwrap();
        let loaded_res: ExecutionResult = serde_json::from_str(&json_res).unwrap();
        assert_eq!(loaded_res, res);
    }
}
