//! Browser IPC dispatch handler implementation.
//!
//! Provides [`BrowserHandler`], implementing [`pursue_runtime::ipc::dispatch::Handler`]
//! for integration into [`pursue_runtime::ipc::dispatch::Router`].
//!
//! # Supported IPC Methods
//! - `browser.session.create`: Creates an active browsing session.
//! - `browser.session.get`: Retrieves session status and metadata.
//! - `browser.session.terminate`: Terminates an active browsing session.
//! - `browser.navigate`: Dispatches a validated navigation request.
//! - `browser.evidence.capture`: Ingests web artifacts into the case evidence repository.

use std::str::FromStr;
use std::sync::{Arc, Mutex};

use pursue_case::{CaseId, CaseStore, FileCaseStore};
use pursue_runtime::ipc::dispatch::Handler;
use pursue_runtime::ipc::protocol::{IpcError, IpcErrorCode, MethodName, ServiceId};
use serde_json::{Value as JsonValue, json};

use crate::engine::{BrowserEngine, NetworkEngine};
use crate::evidence::{WebArtifactKind, WebEvidenceCapturer};
use crate::navigation::{NavigationRequest, NavigationResult};
use crate::routing::RoutingMode;
use crate::service::BrowserService;
use crate::session_id::BrowserSessionId;
use crate::url::ValidatedUrl;

/// IPC dispatch handler for the `browser` service.
pub struct BrowserHandler {
    service_id: ServiceId,
    service: BrowserService,
    engine: Arc<dyn BrowserEngine>,
    file_store: Option<Arc<Mutex<FileCaseStore>>>,
}

impl BrowserHandler {
    /// Creates a handler with default production network engine and no case store.
    pub fn new(service: BrowserService) -> Self {
        Self {
            service_id: ServiceId::new("browser").expect("valid service id"),
            service,
            engine: Arc::new(NetworkEngine::new()),
            file_store: None,
        }
    }

    /// Creates a handler with a custom engine (e.g. [`crate::engine::MockBrowserEngine`]).
    pub fn with_engine(service: BrowserService, engine: Arc<dyn BrowserEngine>) -> Self {
        Self {
            service_id: ServiceId::new("browser").expect("valid service id"),
            service,
            engine,
            file_store: None,
        }
    }

    /// Attaches a file case store for evidence capture over IPC.
    pub fn with_file_store(mut self, store: Arc<Mutex<FileCaseStore>>) -> Self {
        self.file_store = Some(store);
        self
    }
}

impl Handler for BrowserHandler {
    fn service_id(&self) -> &ServiceId {
        &self.service_id
    }

    fn handle(
        &mut self,
        method: &MethodName,
        params: &JsonValue,
    ) -> std::result::Result<JsonValue, IpcError> {
        match method.as_str() {
            "browser.session.create" => self.handle_session_create(params),
            "browser.session.get" => self.handle_session_get(params),
            "browser.session.terminate" => self.handle_session_terminate(params),
            "browser.navigate" => self.handle_navigate(params),
            "browser.evidence.capture" => self.handle_evidence_capture(params),
            unknown => Err(IpcError::new(
                IpcErrorCode::UnknownMethod,
                &format!("unknown browser method: {unknown}"),
            )
            .expect("non-empty error")),
        }
    }
}

impl BrowserHandler {
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
                    "missing required field 'session_id'",
                )
                .expect("valid error")
            })?;
        let session_id = BrowserSessionId::new(session_id_str).map_err(|e| {
            IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).expect("valid error")
        })?;

        let case_id_str = params
            .get("case_id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "missing required field 'case_id'",
                )
                .expect("valid error")
            })?;
        let case_id = CaseId::new(case_id_str).map_err(|e| {
            IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).expect("valid error")
        })?;

        // If file store is present, verify case exists
        if let Some(store) = &self.file_store {
            let guard = store.lock().map_err(|_| {
                IpcError::new(IpcErrorCode::Internal, "case store lock poisoned")
                    .expect("valid error")
            })?;
            if !guard.contains_case(&case_id) {
                return Err(IpcError::new(
                    IpcErrorCode::InvalidParams,
                    &format!("case '{case_id}' does not exist"),
                )
                .expect("valid error"));
            }
        }

        let actor = params
            .get("actor")
            .and_then(JsonValue::as_str)
            .unwrap_or("investigator");

        let mode_str = params
            .get("mode")
            .and_then(JsonValue::as_str)
            .unwrap_or("direct");
        let mode = RoutingMode::from_str(mode_str).map_err(|e| {
            IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).expect("valid error")
        })?;

        let session = self
            .service
            .create_session(session_id, case_id, actor, mode)
            .map_err(|e| {
                IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).expect("valid error")
            })?;

        serde_json::to_value(&session).map_err(|e| {
            IpcError::new(IpcErrorCode::Internal, &e.to_string()).expect("valid error")
        })
    }

    fn handle_session_get(&self, params: &JsonValue) -> std::result::Result<JsonValue, IpcError> {
        let session_id_str = params
            .get("session_id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "missing required field 'session_id'",
                )
                .expect("valid error")
            })?;
        let session_id = BrowserSessionId::new(session_id_str).map_err(|e| {
            IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).expect("valid error")
        })?;

        let session = self.service.get_session(&session_id).map_err(|e| {
            IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).expect("valid error")
        })?;

        serde_json::to_value(&session).map_err(|e| {
            IpcError::new(IpcErrorCode::Internal, &e.to_string()).expect("valid error")
        })
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
                    "missing required field 'session_id'",
                )
                .expect("valid error")
            })?;
        let session_id = BrowserSessionId::new(session_id_str).map_err(|e| {
            IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).expect("valid error")
        })?;

        self.service.terminate_session(&session_id).map_err(|e| {
            IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).expect("valid error")
        })?;

        Ok(json!({ "session_id": session_id_str, "status": "terminated" }))
    }

    fn handle_navigate(&self, params: &JsonValue) -> std::result::Result<JsonValue, IpcError> {
        let session_id_str = params
            .get("session_id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "missing required field 'session_id'",
                )
                .expect("valid error")
            })?;
        let session_id = BrowserSessionId::new(session_id_str).map_err(|e| {
            IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).expect("valid error")
        })?;

        let session = self.service.get_session(&session_id).map_err(|e| {
            IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).expect("valid error")
        })?;

        let url_str = params
            .get("url")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(IpcErrorCode::InvalidParams, "missing required field 'url'")
                    .expect("valid error")
            })?;
        let url = ValidatedUrl::parse(url_str).map_err(|e| {
            IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).expect("valid error")
        })?;

        let mut req = NavigationRequest::new(url);

        if let Some(timeout) = params.get("timeout_secs").and_then(JsonValue::as_u64) {
            req = req.with_timeout(timeout).map_err(|e| {
                IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).expect("valid error")
            })?;
        }

        if let Some(max_bytes) = params.get("max_response_bytes").and_then(JsonValue::as_u64) {
            req = req
                .with_max_response_bytes(max_bytes as usize)
                .map_err(|e| {
                    IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).expect("valid error")
                })?;
        }

        if let Some(headers_obj) = params.get("headers").and_then(JsonValue::as_object) {
            for (k, v) in headers_obj {
                if let Some(val_str) = v.as_str() {
                    req = req.with_header(k, val_str).map_err(|e| {
                        IpcError::new(IpcErrorCode::InvalidParams, &e.to_string())
                            .expect("valid error")
                    })?;
                }
            }
        }

        let result = self.engine.navigate(&session, &req).map_err(|e| {
            IpcError::new(IpcErrorCode::Internal, &e.to_string()).expect("valid error")
        })?;

        serde_json::to_value(&result).map_err(|e| {
            IpcError::new(IpcErrorCode::Internal, &e.to_string()).expect("valid error")
        })
    }

    fn handle_evidence_capture(
        &self,
        params: &JsonValue,
    ) -> std::result::Result<JsonValue, IpcError> {
        let store_arc = self.file_store.as_ref().ok_or_else(|| {
            IpcError::new(
                IpcErrorCode::Internal,
                "file store not configured for browser handler",
            )
            .expect("valid error")
        })?;

        let session_id_str = params
            .get("session_id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "missing required field 'session_id'",
                )
                .expect("valid error")
            })?;
        let session_id = BrowserSessionId::new(session_id_str).map_err(|e| {
            IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).expect("valid error")
        })?;

        let session = self.service.get_session(&session_id).map_err(|e| {
            IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).expect("valid error")
        })?;

        let result_val = params.get("result").ok_or_else(|| {
            IpcError::new(
                IpcErrorCode::InvalidParams,
                "missing required field 'result'",
            )
            .expect("valid error")
        })?;
        let result: NavigationResult = serde_json::from_value(result_val.clone()).map_err(|e| {
            IpcError::new(
                IpcErrorCode::InvalidParams,
                &format!("invalid result payload: {e}"),
            )
            .expect("valid error")
        })?;

        let artifact_str = params
            .get("artifact")
            .and_then(JsonValue::as_str)
            .unwrap_or("page_content");
        let kind = match artifact_str {
            "page_content" => WebArtifactKind::PageContent,
            "http_headers" => WebArtifactKind::HttpHeaders,
            "screenshot" => WebArtifactKind::Screenshot,
            "downloaded_file" => WebArtifactKind::DownloadedFile,
            other => {
                return Err(IpcError::new(
                    IpcErrorCode::InvalidParams,
                    &format!("unknown artifact kind '{other}'"),
                )
                .expect("valid error"));
            }
        };

        let custom_label = params.get("custom_label").and_then(JsonValue::as_str);

        let mut store_guard = store_arc.lock().map_err(|_| {
            IpcError::new(IpcErrorCode::Internal, "case store lock poisoned").expect("valid error")
        })?;

        let captured = WebEvidenceCapturer::capture_navigation_artifact(
            &mut store_guard,
            &session,
            &result,
            kind,
            custom_label,
        )
        .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).expect("valid error"))?;

        serde_json::to_value(&captured).map_err(|e| {
            IpcError::new(IpcErrorCode::Internal, &e.to_string()).expect("valid error")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::MockBrowserEngine;
    use std::path::PathBuf;

    #[test]
    fn ipc_handler_session_lifecycle_and_navigation() {
        let service = BrowserService::new(PathBuf::from("/tmp/pursue-ipc-test"));
        let mock_engine = Arc::new(MockBrowserEngine::new());
        let mut handler = BrowserHandler::with_engine(service, mock_engine);

        // 1. Create session
        let create_params = json!({
            "session_id": "sess-ipc-1",
            "case_id": "case-ipc-1",
            "actor": "analyst",
            "mode": "tor"
        });
        let create_res = handler
            .handle(
                &MethodName::new("browser.session.create").unwrap(),
                &create_params,
            )
            .unwrap();
        assert_eq!(create_res["id"], "sess-ipc-1");
        assert_eq!(create_res["mode"], "tor");

        // 2. Get session
        let get_params = json!({ "session_id": "sess-ipc-1" });
        let get_res = handler
            .handle(
                &MethodName::new("browser.session.get").unwrap(),
                &get_params,
            )
            .unwrap();
        assert_eq!(get_res["status"], "active");

        // 3. Navigate
        let nav_params = json!({
            "session_id": "sess-ipc-1",
            "url": "https://example.com/search?q=osint"
        });
        let nav_res = handler
            .handle(&MethodName::new("browser.navigate").unwrap(), &nav_params)
            .unwrap();
        assert_eq!(nav_res["status_code"], 200);
        assert_eq!(nav_res["routing_mode"], "tor");

        // 4. Terminate session
        let term_params = json!({ "session_id": "sess-ipc-1" });
        let term_res = handler
            .handle(
                &MethodName::new("browser.session.terminate").unwrap(),
                &term_params,
            )
            .unwrap();
        assert_eq!(term_res["status"], "terminated");
    }

    #[test]
    fn ipc_handler_rejects_malformed_inputs() {
        let service = BrowserService::new(PathBuf::from("/tmp/pursue-ipc-err-test"));
        let mut handler = BrowserHandler::new(service);

        // Missing session_id
        let err = handler
            .handle(
                &MethodName::new("browser.session.create").unwrap(),
                &json!({ "case_id": "case-1" }),
            )
            .unwrap_err();
        assert_eq!(err.code(), IpcErrorCode::InvalidParams);

        // Forbidden URL scheme in navigate
        let nav_err = handler
            .handle(
                &MethodName::new("browser.navigate").unwrap(),
                &json!({ "session_id": "s1", "url": "file:///etc/shadow" }),
            )
            .unwrap_err();
        assert_eq!(nav_err.code(), IpcErrorCode::InvalidParams);
    }
}
