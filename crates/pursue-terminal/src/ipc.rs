//! Terminal IPC dispatch handler implementation.
//!
//! Provides [`TerminalHandler`], implementing [`pursue_runtime::ipc::dispatch::Handler`]
//! for integration into [`pursue_runtime::ipc::dispatch::Router`].
//!
//! # Supported Methods
//! - `session.create`: Validates parameters and creates an active terminal session.
//! - `session.get`: Retrieves session status and metadata.
//! - `session.terminate`: Terminates an active terminal session.
//! - `command.execute`: Dispatches a validated [`CommandRequest`] to the executor.
//! - `evidence.capture`: Ingests command output into the case evidence repository.
//!
//! All incoming IPC parameters are treated as untrusted and validated strictly before execution.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use pursue_case::{CaseId, FileCaseStore};
use pursue_runtime::ipc::dispatch::Handler;
use pursue_runtime::ipc::protocol::{IpcError, IpcErrorCode, MethodName, ServiceId};
use serde_json::{Value as JsonValue, json};

use crate::command::CommandRequest;
use crate::evidence::{StreamKind, TerminalEvidenceCapturer};
use crate::executor::{CommandExecutor, ProcessExecutor};
use crate::service::TerminalService;
use crate::session::Session;
use crate::session_id::SessionId;

/// IPC dispatch handler for the `terminal` service.
pub struct TerminalHandler {
    service_id: ServiceId,
    service: TerminalService,
    executor: Arc<dyn CommandExecutor>,
    file_store: Option<Arc<Mutex<FileCaseStore>>>,
}

impl TerminalHandler {
    /// Creates a handler with default process execution and no file store.
    pub fn new(service: TerminalService) -> Self {
        Self {
            service_id: ServiceId::new("terminal").expect("valid service id"),
            service,
            executor: Arc::new(ProcessExecutor::new()),
            file_store: None,
        }
    }

    /// Creates a handler with a custom executor (e.g. [`crate::executor::MockExecutor`] for tests).
    pub fn with_executor(service: TerminalService, executor: Arc<dyn CommandExecutor>) -> Self {
        Self {
            service_id: ServiceId::new("terminal").expect("valid service id"),
            service,
            executor,
            file_store: None,
        }
    }

    /// Attaches a file case store for evidence capture over IPC.
    pub fn with_file_store(mut self, store: Arc<Mutex<FileCaseStore>>) -> Self {
        self.file_store = Some(store);
        self
    }
}

impl Handler for TerminalHandler {
    fn service_id(&self) -> &ServiceId {
        &self.service_id
    }

    fn handle(
        &mut self,
        method: &MethodName,
        params: &JsonValue,
    ) -> std::result::Result<JsonValue, IpcError> {
        match method.as_str() {
            "session.create" => self.handle_session_create(params),
            "session.get" => self.handle_session_get(params),
            "session.terminate" => self.handle_session_terminate(params),
            "command.execute" => self.handle_command_execute(params),
            "evidence.capture" => self.handle_evidence_capture(params),
            unknown => Err(IpcError::new(
                IpcErrorCode::UnknownMethod,
                &format!("unknown terminal method: {unknown}"),
            )
            .expect("non-empty error")),
        }
    }
}

impl TerminalHandler {
    fn handle_session_create(
        &self,
        params: &JsonValue,
    ) -> std::result::Result<JsonValue, IpcError> {
        let session_id_str = params
            .get("session_id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "missing or invalid 'session_id'",
                )
                .unwrap()
            })?;
        let session_id = SessionId::new(session_id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let case_id_str = params
            .get("case_id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(IpcErrorCode::InvalidParams, "missing or invalid 'case_id'").unwrap()
            })?;
        let case_id = CaseId::new(case_id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let actor = params
            .get("actor")
            .and_then(JsonValue::as_str)
            .unwrap_or("investigator");

        let working_dir_str = params
            .get("working_dir")
            .and_then(JsonValue::as_str)
            .unwrap_or(".");
        let working_dir = PathBuf::from(working_dir_str);

        let mut env_vars = BTreeMap::new();
        if let Some(env_obj) = params.get("env").and_then(JsonValue::as_object) {
            for (k, v) in env_obj {
                if let Some(val_str) = v.as_str() {
                    env_vars.insert(k.clone(), val_str.to_string());
                }
            }
        }

        let session = Session::new(session_id, case_id, actor, working_dir, env_vars)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let id_out = session.id().to_string();
        let case_out = session.case_id().to_string();
        let actor_out = session.actor().to_string();

        self.service
            .register_session(session)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        Ok(json!({
            "session_id": id_out,
            "case_id": case_out,
            "actor": actor_out,
            "status": "active"
        }))
    }

    fn handle_session_get(&self, params: &JsonValue) -> std::result::Result<JsonValue, IpcError> {
        let session_id_str = params
            .get("session_id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "missing or invalid 'session_id'",
                )
                .unwrap()
            })?;
        let session_id = SessionId::new(session_id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let session = self
            .service
            .get_session(&session_id)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        serde_json::to_value(&session)
            .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())
    }

    fn handle_session_terminate(
        &self,
        params: &JsonValue,
    ) -> std::result::Result<JsonValue, IpcError> {
        let session_id_str = params
            .get("session_id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "missing or invalid 'session_id'",
                )
                .unwrap()
            })?;
        let session_id = SessionId::new(session_id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        self.service
            .terminate_session(&session_id)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        Ok(json!({
            "session_id": session_id.to_string(),
            "status": "terminated"
        }))
    }

    fn handle_command_execute(
        &self,
        params: &JsonValue,
    ) -> std::result::Result<JsonValue, IpcError> {
        let session_id_str = params
            .get("session_id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "missing or invalid 'session_id'",
                )
                .unwrap()
            })?;
        let session_id = SessionId::new(session_id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let session = self
            .service
            .get_session(&session_id)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let program = params
            .get("program")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(IpcErrorCode::InvalidParams, "missing or invalid 'program'").unwrap()
            })?;

        let mut args = Vec::new();
        if let Some(args_arr) = params.get("args").and_then(JsonValue::as_array) {
            for arg_val in args_arr {
                if let Some(arg_str) = arg_val.as_str() {
                    args.push(arg_str.to_string());
                } else {
                    return Err(IpcError::new(
                        IpcErrorCode::InvalidParams,
                        "args elements must be strings",
                    )
                    .unwrap());
                }
            }
        }

        let mut request = CommandRequest::new(program, args)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        if let Some(dir_str) = params.get("working_dir").and_then(JsonValue::as_str) {
            request = request
                .with_working_dir(PathBuf::from(dir_str))
                .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;
        }
        if let Some(timeout) = params.get("timeout_secs").and_then(JsonValue::as_u64) {
            request = request
                .with_timeout_secs(timeout)
                .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;
        }
        if let Some(limit) = params.get("max_output_bytes").and_then(JsonValue::as_u64) {
            request = request
                .with_max_output_bytes(limit as usize)
                .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;
        }

        let result = self
            .executor
            .execute(&session, &request)
            .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())?;

        serde_json::to_value(&result)
            .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())
    }

    fn handle_evidence_capture(
        &self,
        params: &JsonValue,
    ) -> std::result::Result<JsonValue, IpcError> {
        let store_arc = self.file_store.as_ref().ok_or_else(|| {
            IpcError::new(
                IpcErrorCode::Internal,
                "case store not configured for terminal handler",
            )
            .unwrap()
        })?;

        let session_id_str = params
            .get("session_id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "missing or invalid 'session_id'",
                )
                .unwrap()
            })?;
        let session_id = SessionId::new(session_id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let session = self
            .service
            .get_session(&session_id)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let program = params
            .get("program")
            .and_then(JsonValue::as_str)
            .unwrap_or("cmd");

        let stream_str = params
            .get("stream")
            .and_then(JsonValue::as_str)
            .unwrap_or("stdout");
        let stream = match stream_str {
            "stdout" => StreamKind::Stdout,
            "stderr" => StreamKind::Stderr,
            _ => {
                return Err(IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "stream must be 'stdout' or 'stderr'",
                )
                .unwrap());
            }
        };

        let raw_bytes = if let Some(bytes_str) = params.get("data").and_then(JsonValue::as_str) {
            bytes_str.as_bytes().to_vec()
        } else if let Some(bytes_arr) = params.get("data").and_then(JsonValue::as_array) {
            bytes_arr
                .iter()
                .filter_map(|v| v.as_u64().map(|b| b as u8))
                .collect()
        } else {
            return Err(IpcError::new(
                IpcErrorCode::InvalidParams,
                "missing or invalid 'data' payload",
            )
            .unwrap());
        };

        let custom_label = params.get("source_label").and_then(JsonValue::as_str);

        let req = CommandRequest::new(program, vec![])
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;
        let (stdout, stderr) = match stream {
            StreamKind::Stdout => (raw_bytes, vec![]),
            StreamKind::Stderr => (vec![], raw_bytes),
        };

        let exec_result = crate::command::ExecutionResult::new(
            "ipc-capture",
            Some(0),
            stdout,
            stderr,
            0,
            0,
            false,
            false,
        )
        .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())?;

        let mut store = store_arc.lock().unwrap();
        let captured = TerminalEvidenceCapturer::capture_stream_into_file_store(
            &mut store,
            &session,
            &req,
            &exec_result,
            stream,
            custom_label,
        )
        .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())?;

        Ok(json!({
            "content_address": captured.address.to_hex(),
            "size": captured.record.size(),
            "source_label": captured.source_label
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pursue_runtime::ipc::dispatch::Router;
    use pursue_runtime::ipc::protocol::Request;

    #[test]
    fn ipc_handler_session_lifecycle_and_execution() {
        let service = TerminalService::new();
        let handler = TerminalHandler::new(service);

        let mut router = Router::new();
        router.register(Box::new(handler)).unwrap();

        // 1. session.create
        let create_req = Request::new(
            1,
            ServiceId::new("terminal").unwrap(),
            MethodName::new("session.create").unwrap(),
            json!({
                "session_id": "ipc-session-01",
                "case_id": "case-01",
                "actor": "investigator-1",
                "working_dir": "."
            }),
        );
        let create_res = router.handle(&create_req);
        assert!(create_res.is_success());
        let result = create_res.result.unwrap();
        assert_eq!(result["session_id"], "ipc-session-01");
        assert_eq!(result["status"], "active");

        // 2. session.get
        let get_req = Request::new(
            2,
            ServiceId::new("terminal").unwrap(),
            MethodName::new("session.get").unwrap(),
            json!({ "session_id": "ipc-session-01" }),
        );
        let get_res = router.handle(&get_req);
        assert!(get_res.is_success());
        assert_eq!(get_res.result.unwrap()["status"], "active");

        // 3. session.terminate
        let term_req = Request::new(
            3,
            ServiceId::new("terminal").unwrap(),
            MethodName::new("session.terminate").unwrap(),
            json!({ "session_id": "ipc-session-01" }),
        );
        let term_res = router.handle(&term_req);
        assert!(term_res.is_success());
        assert_eq!(term_res.result.unwrap()["status"], "terminated");
    }

    #[test]
    fn ipc_handler_rejects_malformed_inputs() {
        let service = TerminalService::new();
        let handler = TerminalHandler::new(service);

        let mut router = Router::new();
        router.register(Box::new(handler)).unwrap();

        // Path traversal in session_id rejected
        let bad_req = Request::new(
            1,
            ServiceId::new("terminal").unwrap(),
            MethodName::new("session.create").unwrap(),
            json!({
                "session_id": "../evil",
                "case_id": "case-01"
            }),
        );
        let res = router.handle(&bad_req);
        assert!(!res.is_success());
        assert_eq!(res.error.unwrap().code(), IpcErrorCode::InvalidParams);
    }
}
