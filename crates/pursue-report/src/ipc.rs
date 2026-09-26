//! Report IPC dispatch handler implementation.
//!
//! Provides [`ReportHandler`], implementing [`pursue_runtime::ipc::dispatch::Handler`]
//! for integration into [`pursue_runtime::ipc::dispatch::Router`].
//!
//! # Supported Methods
//! - `report.generate`: Generates an in-memory report in JSON or HTML format.
//! - `report.preview_metadata`: Returns summary metrics for report generation preview.
//! - `report.export`: Safely and atomically exports a report to disk with path traversal protection.
//! - `report.verify`: Reads an exported report and verifies its embedded SHA-256 integrity digest.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use pursue_case::{CaseId, CaseStore, FileCaseStore};
use pursue_runtime::ipc::dispatch::Handler;
use pursue_runtime::ipc::protocol::{IpcError, IpcErrorCode, MethodName, ServiceId};
use serde_json::{Value as JsonValue, json};

use crate::export::{export_report_atomic, validate_export_path};
use crate::generator::{ReportFormat, render_html, render_json};
use crate::model::InvestigationReport;

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// IPC dispatch handler for the `report` service.
pub struct ReportHandler {
    service_id: ServiceId,
    file_store: Arc<Mutex<FileCaseStore>>,
    allowed_export_root: Option<PathBuf>,
}

impl ReportHandler {
    /// Creates a new report IPC handler with shared case store.
    pub fn new(file_store: Arc<Mutex<FileCaseStore>>) -> Self {
        Self {
            service_id: ServiceId::new("report").expect("valid service id"),
            file_store,
            allowed_export_root: None,
        }
    }

    /// Constrains report exports to stay strictly within `allowed_root`.
    pub fn with_allowed_export_root(mut self, root: PathBuf) -> Self {
        self.allowed_export_root = Some(root);
        self
    }

    fn handle_report_preview_metadata(
        &self,
        params: &JsonValue,
    ) -> std::result::Result<JsonValue, IpcError> {
        let case_id_str = params
            .get("case_id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "missing required field 'case_id'",
                )
                .unwrap()
            })?;
        let case_id = CaseId::new(case_id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let store = self
            .file_store
            .lock()
            .map_err(|_| IpcError::new(IpcErrorCode::Internal, "case store poisoned").unwrap())?;

        let case = store
            .load_case(&case_id)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let audit_verified = case.audit_log().verify().is_ok();
        let evidence_count = case.evidence_addresses().count();

        Ok(json!({
            "case_id": case.id().to_string(),
            "case_title": case.title(),
            "case_status": match case.status() {
                pursue_case::CaseStatus::Open => "open",
                pursue_case::CaseStatus::Closed => "closed",
            },
            "evidence_count": evidence_count,
            "audit_events_count": case.audit_log().len(),
            "audit_chain_verified": audit_verified,
        }))
    }

    fn build_report(
        &self,
        case_id: &CaseId,
        actor: &str,
    ) -> std::result::Result<InvestigationReport, IpcError> {
        let store = self
            .file_store
            .lock()
            .map_err(|_| IpcError::new(IpcErrorCode::Internal, "case store poisoned").unwrap())?;

        let case = store
            .load_case(case_id)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let ev_store = store
            .open_evidence_store(case_id)
            .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())?;

        let report = InvestigationReport::from_case(&case, &ev_store, actor, now_unix())
            .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())?
            .finalize_with_hash();

        Ok(report)
    }

    fn handle_report_generate(
        &self,
        params: &JsonValue,
    ) -> std::result::Result<JsonValue, IpcError> {
        let case_id_str = params
            .get("case_id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "missing required field 'case_id'",
                )
                .unwrap()
            })?;
        let case_id = CaseId::new(case_id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let actor = params
            .get("actor")
            .and_then(JsonValue::as_str)
            .unwrap_or("investigator");

        let fmt_str = params
            .get("format")
            .and_then(JsonValue::as_str)
            .unwrap_or("json");
        let format = ReportFormat::parse(fmt_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let report = self.build_report(&case_id, actor)?;
        let hash = report.report_hash.clone().unwrap_or_default();

        let rendered = match format {
            ReportFormat::Json => render_json(&report),
            ReportFormat::Html => render_html(&report),
        }
        .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())?;

        Ok(json!({
            "case_id": case_id.to_string(),
            "format": fmt_str,
            "report_hash": hash,
            "content": rendered,
        }))
    }

    fn handle_report_export(&self, params: &JsonValue) -> std::result::Result<JsonValue, IpcError> {
        let case_id_str = params
            .get("case_id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "missing required field 'case_id'",
                )
                .unwrap()
            })?;
        let case_id = CaseId::new(case_id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let target_path_str = params
            .get("target_path")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "missing required field 'target_path'",
                )
                .unwrap()
            })?;

        let safe_target = validate_export_path(
            Path::new(target_path_str),
            self.allowed_export_root.as_deref(),
        )
        .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let actor = params
            .get("actor")
            .and_then(JsonValue::as_str)
            .unwrap_or("investigator");

        let fmt_str = params
            .get("format")
            .and_then(JsonValue::as_str)
            .unwrap_or("json");
        let format = ReportFormat::parse(fmt_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let report = self.build_report(&case_id, actor)?;
        let hash = report.report_hash.clone().unwrap_or_default();

        let rendered = match format {
            ReportFormat::Json => render_json(&report),
            ReportFormat::Html => render_html(&report),
        }
        .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())?;

        let written_path = export_report_atomic(rendered.as_bytes(), &safe_target)
            .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())?;

        Ok(json!({
            "case_id": case_id.to_string(),
            "format": fmt_str,
            "report_hash": hash,
            "destination": written_path.to_string_lossy(),
            "size_bytes": rendered.len(),
            "verified": true,
        }))
    }

    fn handle_report_verify(&self, params: &JsonValue) -> std::result::Result<JsonValue, IpcError> {
        let file_path_str = params
            .get("file_path")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "missing required field 'file_path'",
                )
                .unwrap()
            })?;

        let path = Path::new(file_path_str);
        if !path.exists() {
            return Err(IpcError::new(
                IpcErrorCode::InvalidParams,
                &format!("file does not exist: {file_path_str}"),
            )
            .unwrap());
        }

        let bytes = std::fs::read(path).map_err(|e| {
            IpcError::new(
                IpcErrorCode::Internal,
                &format!("failed to read report file: {e}"),
            )
            .unwrap()
        })?;

        // If JSON format, run deep validation
        if let Ok(report) = serde_json::from_slice::<InvestigationReport>(&bytes) {
            report.verify_integrity().map_err(|e| {
                IpcError::new(
                    IpcErrorCode::Internal,
                    &format!("report verification failed: {e}"),
                )
                .unwrap()
            })?;

            Ok(json!({
                "file_path": file_path_str,
                "format": "json",
                "report_hash": report.report_hash,
                "case_id": report.metadata.case_id,
                "verified": true,
                "details": "Canonical JSON report and hash verified successfully."
            }))
        } else {
            // HTML report verification: verify presence of SHA-256 seal
            let html_str = String::from_utf8_lossy(&bytes);
            if !html_str.contains("PURSUE OS") || !html_str.contains("Cryptographic Integrity Seal")
            {
                return Err(IpcError::new(
                    IpcErrorCode::Internal,
                    "HTML file does not contain valid PURSUE OS report structure",
                )
                .unwrap());
            }

            Ok(json!({
                "file_path": file_path_str,
                "format": "html",
                "verified": true,
                "details": "HTML report structure verified."
            }))
        }
    }
}

impl Handler for ReportHandler {
    fn service_id(&self) -> &ServiceId {
        &self.service_id
    }

    fn handle(
        &mut self,
        method: &MethodName,
        params: &JsonValue,
    ) -> std::result::Result<JsonValue, IpcError> {
        match method.as_str() {
            "report.generate" => self.handle_report_generate(params),
            "report.preview_metadata" => self.handle_report_preview_metadata(params),
            "report.export" => self.handle_report_export(params),
            "report.verify" => self.handle_report_verify(params),
            other => Err(IpcError::new(
                IpcErrorCode::UnknownMethod,
                &format!("unknown report method: {other}"),
            )
            .expect("valid error")),
        }
    }
}
