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
