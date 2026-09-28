//! IPC client layer for the PURSUE OS desktop.
//!
//! Provides [`IpcClient`] and [`RouterClient`] to dispatch typed RPC requests
//! across the PURSUE IPC boundary to the backend services (`terminal`, `browser`, etc.).
//!
//! # Security Invariant
//! The desktop UI never modifies backend storage or executes child processes directly.
//! All operations are mediated through validated IPC messages.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use pursue_core::{Error, Result};
use pursue_runtime::ipc::dispatch::Router;
use pursue_runtime::ipc::protocol::{MethodName, Request, Response, ServiceId};
use serde_json::{Value as JsonValue, json};

/// Trait abstracting communication with the PURSUE OS IPC subsystem.
pub trait IpcClient: Send + Sync {
    /// Dispatches an untyped RPC request to a service and method, returning JSON.
    fn call(&self, service: &str, method: &str, params: JsonValue) -> Result<JsonValue>;
}

/// An IPC client dispatching requests directly through an in-memory or socket-backed [`Router`].
pub struct RouterClient {
    router: Arc<Mutex<Router>>,
    next_id: AtomicU64,
}

impl RouterClient {
    /// Creates a new router client wrapping a shared [`Router`].
    pub fn new(router: Arc<Mutex<Router>>) -> Self {
        Self {
            router,
            next_id: AtomicU64::new(1),
        }
    }

    /// Dispatches a structured command execution request to the `terminal` service.
    pub fn execute_terminal_command(
        &self,
        session_id: &str,
        program: &str,
        args: &[String],
    ) -> Result<JsonValue> {
        let params = json!({
            "session_id": session_id,
            "program": program,
            "args": args,
        });
        self.call("terminal", "command.execute", params)
    }

    /// Creates a new terminal session.
    pub fn create_terminal_session(
        &self,
        session_id: &str,
        case_id: &str,
        actor: &str,
    ) -> Result<JsonValue> {
        let params = json!({
            "session_id": session_id,
            "case_id": case_id,
            "actor": actor,
        });
        self.call("terminal", "session.create", params)
    }

    /// Captures terminal command output as evidence.
    pub fn capture_terminal_evidence(
        &self,
        session_id: &str,
        result: JsonValue,
        stream: &str,
    ) -> Result<JsonValue> {
        let data = if stream == "stderr" {
            result.get("stderr").cloned().unwrap_or(JsonValue::Null)
        } else {
            result.get("stdout").cloned().unwrap_or(JsonValue::Null)
        };
        let params = json!({
            "session_id": session_id,
            "data": data,
            "stream": stream,
        });
        self.call("terminal", "evidence.capture", params)
    }

    /// Creates a new browser session.
    pub fn create_browser_session(
        &self,
        session_id: &str,
        case_id: &str,
        actor: &str,
        mode: &str,
    ) -> Result<JsonValue> {
        let params = json!({
            "session_id": session_id,
            "case_id": case_id,
            "actor": actor,
            "mode": mode,
        });
        self.call("browser", "browser.session.create", params)
    }

    /// Dispatches a browser navigation request.
    pub fn navigate_browser(&self, session_id: &str, url: &str) -> Result<JsonValue> {
        let params = json!({
            "session_id": session_id,
            "url": url,
        });
        self.call("browser", "browser.navigate", params)
    }

    /// Captures web navigation content as evidence.
    pub fn capture_browser_evidence(
        &self,
        session_id: &str,
        result: JsonValue,
        artifact: &str,
    ) -> Result<JsonValue> {
        let params = json!({
            "session_id": session_id,
            "result": result,
            "artifact": artifact,
        });
        self.call("browser", "browser.evidence.capture", params)
    }

    // --- Case Service Methods ---

    /// Creates a new forensic case.
    pub fn create_case(&self, id: &str, title: &str, actor: &str) -> Result<JsonValue> {
        let params = json!({
            "id": id,
            "title": title,
            "actor": actor,
        });
        self.call("case", "case.create", params)
    }

    /// Loads a case by ID.
    pub fn get_case(&self, id: &str) -> Result<JsonValue> {
        let params = json!({ "id": id });
        self.call("case", "case.get", params)
    }

    /// Lists summaries of all stored cases.
    pub fn list_cases(&self) -> Result<JsonValue> {
        self.call("case", "case.list", json!({}))
    }

    /// Updates investigator notes on an open case.
    pub fn update_case_notes(&self, id: &str, notes: &str, actor: &str) -> Result<JsonValue> {
        let params = json!({
            "id": id,
            "notes": notes,
            "actor": actor,
        });
        self.call("case", "case.update_notes", params)
    }

    /// Updates investigator title on an open case.
    pub fn update_case_title(&self, id: &str, title: &str, actor: &str) -> Result<JsonValue> {
        let params = json!({
            "id": id,
            "title": title,
            "actor": actor,
        });
        self.call("case", "case.update_title", params)
    }

    /// Closes an open case.
    pub fn close_case(&self, id: &str, actor: &str) -> Result<JsonValue> {
        let params = json!({
            "id": id,
            "actor": actor,
        });
        self.call("case", "case.close", params)
    }

    /// Reopens a closed case.
    pub fn reopen_case(&self, id: &str, actor: &str) -> Result<JsonValue> {
        let params = json!({
            "id": id,
            "actor": actor,
        });
        self.call("case", "case.reopen", params)
    }

    /// Runs deep integrity verification on a case.
    pub fn verify_case(&self, id: &str) -> Result<JsonValue> {
        let params = json!({ "id": id });
        self.call("case", "case.verify", params)
    }

    /// Lists evidence attached to a case.
    pub fn list_case_evidence(&self, id: &str) -> Result<JsonValue> {
        let params = json!({ "id": id });
        self.call("case", "case.evidence.list", params)
    }

    /// Reads and inspects a specific evidence artifact blob.
    pub fn read_case_evidence(&self, id: &str, address: &str) -> Result<JsonValue> {
        let params = json!({
            "id": id,
            "address": address,
        });
        self.call("case", "case.evidence.read", params)
    }

    /// Lists hash-chained audit log events for a case.
    pub fn list_case_audit(&self, id: &str) -> Result<JsonValue> {
        let params = json!({ "id": id });
        self.call("case", "case.audit.list", params)
    }

    // --- Report Service Methods ---

    /// Generates an in-memory report in JSON or HTML format.
    pub fn generate_report(&self, case_id: &str, actor: &str, format: &str) -> Result<JsonValue> {
        let params = json!({
            "case_id": case_id,
            "actor": actor,
            "format": format,
        });
        self.call("report", "report.generate", params)
    }

    /// Retrieves preview metrics for reporting.
    pub fn preview_report_metadata(&self, case_id: &str) -> Result<JsonValue> {
        let params = json!({ "case_id": case_id });
        self.call("report", "report.preview_metadata", params)
    }

    /// Atomically exports a forensic report to disk.
    pub fn export_report(
        &self,
        case_id: &str,
        actor: &str,
        format: &str,
        target_path: &str,
    ) -> Result<JsonValue> {
        let params = json!({
            "case_id": case_id,
            "actor": actor,
            "format": format,
            "target_path": target_path,
        });
        self.call("report", "report.export", params)
    }

    /// Verifies the cryptographic seal of an exported report on disk.
    pub fn verify_report(&self, file_path: &str) -> Result<JsonValue> {
        let params = json!({ "file_path": file_path });
        self.call("report", "report.verify", params)
    }
}

impl IpcClient for RouterClient {
    fn call(&self, service: &str, method: &str, params: JsonValue) -> Result<JsonValue> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let s_id = ServiceId::new(service)?;
        let m_name = MethodName::new(method)?;

        let request = Request::new(id, s_id, m_name, params);

        let mut router = self
            .router
            .lock()
            .map_err(|_| Error::ServiceFailure("router mutex poisoned".into()))?;

        let response: Response = router.handle(&request);

        if let Some(err) = response.error {
            return Err(Error::ServiceFailure(format!(
                "IPC Error [{}]: {}",
                err.code().as_str(),
                err.message()
            )));
        }

        response
            .result
            .ok_or_else(|| Error::ServiceFailure("empty response result from IPC service".into()))
    }
}

/// An IPC client communicating across a local Unix domain socket with the PURSUE OS daemon.
#[cfg(unix)]
pub struct SocketClient {
    socket_path: std::path::PathBuf,
    next_id: AtomicU64,
}

#[cfg(unix)]
impl SocketClient {
    /// Creates a new socket client connecting to the given Unix domain socket path.
    pub fn new(socket_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
            next_id: AtomicU64::new(1),
        }
    }

    /// Returns the socket path configured for this client.
    pub fn socket_path(&self) -> &std::path::Path {
        &self.socket_path
    }
}

#[cfg(unix)]
impl IpcClient for SocketClient {
    fn call(&self, service: &str, method: &str, params: JsonValue) -> Result<JsonValue> {
        use pursue_runtime::ipc::Transport;
        use pursue_runtime::ipc::transport::unix_transport::UnixTransport;

        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let s_id = ServiceId::new(service)?;
        let m_name = MethodName::new(method)?;
        let request = Request::new(id, s_id, m_name, params);

        let mut transport = match UnixTransport::connect(&self.socket_path) {
            Ok(t) => t,
            Err(_) => {
                // Short retry in case the socket was transiently busy
                std::thread::sleep(std::time::Duration::from_millis(50));
                UnixTransport::connect(&self.socket_path).map_err(|e| {
                    Error::ServiceFailure(format!(
                        "Failed to connect to IPC socket at {}: {e}",
                        self.socket_path.display()
                    ))
                })?
            }
        };

        let response = transport.round_trip(&request)?;
        if let Some(err) = response.error {
            return Err(Error::ServiceFailure(format!(
                "IPC Error [{}]: {}",
                err.code().as_str(),
                err.message()
            )));
        }

        response
            .result
            .ok_or_else(|| Error::ServiceFailure("empty response result from IPC service".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pursue_runtime::ipc::dispatch::Handler;
    use pursue_runtime::ipc::protocol::IpcError;

    struct EchoHandler {
        id: ServiceId,
    }

    impl Handler for EchoHandler {
        fn service_id(&self) -> &ServiceId {
            &self.id
        }

        fn handle(
            &mut self,
            _method: &MethodName,
            params: &JsonValue,
        ) -> std::result::Result<JsonValue, IpcError> {
            Ok(json!({ "echo": params }))
        }
    }

    #[test]
    fn router_client_roundtrip() {
        let mut router = Router::new();
        let h = EchoHandler {
            id: ServiceId::new("echo").unwrap(),
        };
        router.register(Box::new(h)).unwrap();

        let client = RouterClient::new(Arc::new(Mutex::new(router)));
        let res = client
            .call("echo", "test.ping", json!({ "msg": "hello" }))
            .unwrap();

        assert_eq!(res["echo"]["msg"], "hello");
    }
}
