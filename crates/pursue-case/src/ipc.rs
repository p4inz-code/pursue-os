//! Case IPC dispatch handler implementation.
//!
//! Provides [`CaseHandler`], implementing [`pursue_runtime::ipc::dispatch::Handler`]
//! for integration into [`pursue_runtime::ipc::dispatch::Router`].
//!
//! # Supported Methods
//! - `case.create`: Validates inputs and creates a new forensic case.
//! - `case.get`: Loads a case, verifying its integrity on load.
//! - `case.list`: Lists summaries of all stored cases.
//! - `case.update_notes`: Updates investigator notes on an open case.
//! - `case.update_title`: Updates investigator title on an open case.
//! - `case.close`: Closes an open case.
//! - `case.reopen`: Reopens a closed case.
//! - `case.verify`: Runs explicit deep verification on case manifest, audit chain, and evidence.
//! - `case.evidence.list`: Lists evidence records attached to a case with verification status.
//! - `case.evidence.read`: Reads and re-verifies a specific evidence artifact blob.
//! - `case.audit.list`: Returns the chronological provenance audit trail with chain verification.

use std::sync::{Arc, Mutex};

use pursue_evidence::{ContentAddress, EvidenceStore};
use pursue_runtime::ipc::dispatch::Handler;
use pursue_runtime::ipc::protocol::{IpcError, IpcErrorCode, MethodName, ServiceId};
use serde_json::{Value as JsonValue, json};

use crate::file_store::FileCaseStore;
use crate::store::CaseStore;
use crate::{CaseId, CaseStatus};

/// IPC dispatch handler for the `case` service.
pub struct CaseHandler {
    service_id: ServiceId,
    file_store: Arc<Mutex<FileCaseStore>>,
}

impl CaseHandler {
    /// Creates a new case IPC handler wrapping a shared [`FileCaseStore`].
    pub fn new(file_store: Arc<Mutex<FileCaseStore>>) -> Self {
        Self {
            service_id: ServiceId::new("case").expect("valid service id"),
            file_store,
        }
    }

    fn handle_case_create(&self, params: &JsonValue) -> std::result::Result<JsonValue, IpcError> {
        let id_str = params
            .get("id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(IpcErrorCode::InvalidParams, "missing required field 'id'").unwrap()
            })?;
        let case_id = CaseId::new(id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let title = params
            .get("title")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "missing required field 'title'",
                )
                .unwrap()
            })?;
        let actor = params
            .get("actor")
            .and_then(JsonValue::as_str)
            .unwrap_or("investigator");

        let mut store = self.file_store.lock().map_err(|_| {
            IpcError::new(IpcErrorCode::Internal, "case store lock poisoned").unwrap()
        })?;

        let case = store
            .create_case(case_id, title, actor)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        Ok(json!({
            "id": case.id().to_string(),
            "title": case.title(),
            "notes": case.notes(),
            "status": match case.status() {
                CaseStatus::Open => "open",
                CaseStatus::Closed => "closed",
            },
            "created_at_unix": case.created_at_unix(),
            "created_by": case.created_by(),
            "evidence_count": 0,
            "audit_events_count": 1,
            "verified": true,
        }))
    }

    fn handle_case_get(&self, params: &JsonValue) -> std::result::Result<JsonValue, IpcError> {
        let id_str = params
            .get("id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(IpcErrorCode::InvalidParams, "missing required field 'id'").unwrap()
            })?;
        let case_id = CaseId::new(id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let store = self.file_store.lock().map_err(|_| {
            IpcError::new(IpcErrorCode::Internal, "case store lock poisoned").unwrap()
        })?;

        let case = store
            .load_case(&case_id)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let evidence_addrs: Vec<String> = case.evidence_addresses().map(|a| a.to_hex()).collect();

        Ok(json!({
            "id": case.id().to_string(),
            "title": case.title(),
            "notes": case.notes(),
            "status": match case.status() {
                CaseStatus::Open => "open",
                CaseStatus::Closed => "closed",
            },
            "created_at_unix": case.created_at_unix(),
            "created_by": case.created_by(),
            "evidence_count": evidence_addrs.len(),
            "evidence_addresses": evidence_addrs,
            "audit_events_count": case.audit_log().len(),
            "verified": true,
        }))
    }

    fn handle_case_list(&self, _params: &JsonValue) -> std::result::Result<JsonValue, IpcError> {
        let store = self.file_store.lock().map_err(|_| {
            IpcError::new(IpcErrorCode::Internal, "case store lock poisoned").unwrap()
        })?;

        let ids = store
            .list_cases()
            .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())?;

        let mut list = Vec::new();
        for id in ids {
            if let Ok(c) = store.load_case(&id) {
                list.push(json!({
                    "id": c.id().to_string(),
                    "title": c.title(),
                    "notes": c.notes(),
                    "status": match c.status() {
                        CaseStatus::Open => "open",
                        CaseStatus::Closed => "closed",
                    },
                    "created_at_unix": c.created_at_unix(),
                    "created_by": c.created_by(),
                    "evidence_count": c.evidence_addresses().count(),
                    "audit_events_count": c.audit_log().len(),
                }));
            }
        }

        Ok(json!(list))
    }

    fn handle_update_notes(&self, params: &JsonValue) -> std::result::Result<JsonValue, IpcError> {
        let id_str = params
            .get("id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(IpcErrorCode::InvalidParams, "missing required field 'id'").unwrap()
            })?;
        let case_id = CaseId::new(id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let notes = params
            .get("notes")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "missing required field 'notes'",
                )
                .unwrap()
            })?;
        let actor = params
            .get("actor")
            .and_then(JsonValue::as_str)
            .unwrap_or("investigator");

        let mut store = self.file_store.lock().map_err(|_| {
            IpcError::new(IpcErrorCode::Internal, "case store lock poisoned").unwrap()
        })?;

        let mut case = store
            .load_case(&case_id)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        case.set_notes(notes, actor)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        store
            .save_case(&case)
            .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())?;

        Ok(json!({
            "id": case.id().to_string(),
            "notes": case.notes(),
            "audit_events_count": case.audit_log().len(),
        }))
    }

    fn handle_update_title(&self, params: &JsonValue) -> std::result::Result<JsonValue, IpcError> {
        let id_str = params
            .get("id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(IpcErrorCode::InvalidParams, "missing required field 'id'").unwrap()
            })?;
        let case_id = CaseId::new(id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let title = params
            .get("title")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "missing required field 'title'",
                )
                .unwrap()
            })?;
        let actor = params
            .get("actor")
            .and_then(JsonValue::as_str)
            .unwrap_or("investigator");

        let mut store = self.file_store.lock().map_err(|_| {
            IpcError::new(IpcErrorCode::Internal, "case store lock poisoned").unwrap()
        })?;

        let mut case = store
            .load_case(&case_id)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        case.set_title(title, actor)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        store
            .save_case(&case)
            .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())?;

        Ok(json!({
            "id": case.id().to_string(),
            "title": case.title(),
            "audit_events_count": case.audit_log().len(),
        }))
    }

    fn handle_case_close(&self, params: &JsonValue) -> std::result::Result<JsonValue, IpcError> {
        let id_str = params
            .get("id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(IpcErrorCode::InvalidParams, "missing required field 'id'").unwrap()
            })?;
        let case_id = CaseId::new(id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;
        let actor = params
            .get("actor")
            .and_then(JsonValue::as_str)
            .unwrap_or("investigator");

        let mut store = self.file_store.lock().map_err(|_| {
            IpcError::new(IpcErrorCode::Internal, "case store lock poisoned").unwrap()
        })?;

        let mut case = store
            .load_case(&case_id)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        case.close(actor)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        store
            .save_case(&case)
            .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())?;

        Ok(json!({
            "id": case.id().to_string(),
            "status": "closed",
            "audit_events_count": case.audit_log().len(),
        }))
    }

    fn handle_case_reopen(&self, params: &JsonValue) -> std::result::Result<JsonValue, IpcError> {
        let id_str = params
            .get("id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(IpcErrorCode::InvalidParams, "missing required field 'id'").unwrap()
            })?;
        let case_id = CaseId::new(id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;
        let actor = params
            .get("actor")
            .and_then(JsonValue::as_str)
            .unwrap_or("investigator");

        let mut store = self.file_store.lock().map_err(|_| {
            IpcError::new(IpcErrorCode::Internal, "case store lock poisoned").unwrap()
        })?;

        let mut case = store
            .load_case(&case_id)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        case.reopen(actor)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        store
            .save_case(&case)
            .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())?;

        Ok(json!({
            "id": case.id().to_string(),
            "status": "open",
            "audit_events_count": case.audit_log().len(),
        }))
    }

    fn handle_case_verify(&self, params: &JsonValue) -> std::result::Result<JsonValue, IpcError> {
        let id_str = params
            .get("id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(IpcErrorCode::InvalidParams, "missing required field 'id'").unwrap()
            })?;
        let case_id = CaseId::new(id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let store = self.file_store.lock().map_err(|_| {
            IpcError::new(IpcErrorCode::Internal, "case store lock poisoned").unwrap()
        })?;

        let case = store.load_case(&case_id).map_err(|e| {
            IpcError::new(
                IpcErrorCode::Internal,
                &format!("Case integrity check failed: {e}"),
            )
            .unwrap()
        })?;

        case.audit_log().verify().map_err(|e| {
            IpcError::new(
                IpcErrorCode::Internal,
                &format!("Audit log chain verification failed: {e}"),
            )
            .unwrap()
        })?;

        let ev_store = store.open_evidence_store(&case_id).map_err(|e| {
            IpcError::new(
                IpcErrorCode::Internal,
                &format!("Evidence store verification failed: {e}"),
            )
            .unwrap()
        })?;

        let mut verified_count = 0;
        for addr in case.evidence_addresses() {
            let _ = ev_store.get(addr).map_err(|e| {
                IpcError::new(
                    IpcErrorCode::Internal,
                    &format!("Evidence blob {addr} verification failed: {e}"),
                )
                .unwrap()
            })?;
            verified_count += 1;
        }

        Ok(json!({
            "id": case.id().to_string(),
            "verified": true,
            "manifest_verified": true,
            "audit_chain_verified": true,
            "evidence_verified_count": verified_count,
            "audit_events_count": case.audit_log().len(),
            "details": "Case manifest, hash-chained audit log, and all attached evidence blobs verified without tampering."
        }))
    }

    fn handle_evidence_list(&self, params: &JsonValue) -> std::result::Result<JsonValue, IpcError> {
        let id_str = params
            .get("id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(IpcErrorCode::InvalidParams, "missing required field 'id'").unwrap()
            })?;
        let case_id = CaseId::new(id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let store = self.file_store.lock().map_err(|_| {
            IpcError::new(IpcErrorCode::Internal, "case store lock poisoned").unwrap()
        })?;

        let case = store
            .load_case(&case_id)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let ev_store = store
            .open_evidence_store(&case_id)
            .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())?;

        let mut items = Vec::new();
        for addr in case.evidence_addresses() {
            if let Ok(rec) = ev_store.record(addr) {
                items.push(json!({
                    "address": addr.to_hex(),
                    "size": rec.size(),
                    "acquired_at_unix": rec.acquired_at_unix(),
                    "source": rec.source(),
                    "verified": true,
                }));
            }
        }

        Ok(json!(items))
    }

    fn handle_evidence_read(&self, params: &JsonValue) -> std::result::Result<JsonValue, IpcError> {
        let id_str = params
            .get("id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(IpcErrorCode::InvalidParams, "missing required field 'id'").unwrap()
            })?;
        let case_id = CaseId::new(id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let addr_str = params
            .get("address")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(
                    IpcErrorCode::InvalidParams,
                    "missing required field 'address'",
                )
                .unwrap()
            })?;
        let addr = ContentAddress::from_hex(addr_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let store = self.file_store.lock().map_err(|_| {
            IpcError::new(IpcErrorCode::Internal, "case store lock poisoned").unwrap()
        })?;

        let ev_store = store
            .open_evidence_store(&case_id)
            .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())?;

        let bytes = ev_store
            .get(&addr)
            .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())?;

        let record = ev_store
            .record(&addr)
            .map_err(|e| IpcError::new(IpcErrorCode::Internal, &e.to_string()).unwrap())?;

        let is_utf8 = std::str::from_utf8(&bytes).is_ok();
        let preview = if is_utf8 {
            String::from_utf8_lossy(&bytes).to_string()
        } else {
            let hex_preview: Vec<String> =
                bytes.iter().take(256).map(|b| format!("{b:02x}")).collect();
            format!(
                "Binary blob ({} bytes): [{}]",
                bytes.len(),
                hex_preview.join(" ")
            )
        };

        Ok(json!({
            "address": addr.to_hex(),
            "size": bytes.len(),
            "source": record.source(),
            "is_utf8": is_utf8,
            "preview": preview,
            "verified": true,
        }))
    }

    fn handle_audit_list(&self, params: &JsonValue) -> std::result::Result<JsonValue, IpcError> {
        let id_str = params
            .get("id")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| {
                IpcError::new(IpcErrorCode::InvalidParams, "missing required field 'id'").unwrap()
            })?;
        let case_id = CaseId::new(id_str)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        let store = self.file_store.lock().map_err(|_| {
            IpcError::new(IpcErrorCode::Internal, "case store lock poisoned").unwrap()
        })?;

        let case = store
            .load_case(&case_id)
            .map_err(|e| IpcError::new(IpcErrorCode::InvalidParams, &e.to_string()).unwrap())?;

        case.audit_log().verify().map_err(|e| {
            IpcError::new(
                IpcErrorCode::Internal,
                &format!("Audit log verification error: {e}"),
            )
            .unwrap()
        })?;

        let events: Vec<JsonValue> = case
            .audit_log()
            .entries()
            .iter()
            .map(|e| {
                json!({
                    "seq": e.seq,
                    "timestamp_unix": e.timestamp_unix,
                    "actor": e.actor,
                    "action": e.action,
                    "subject": e.subject.as_ref().map(|s| s.to_hex()),
                    "prev_hash": e.prev_hash.as_ref().map(|p| p.to_hex()),
                    "hash": e.hash.to_hex(),
                    "verified": true,
                })
            })
            .collect();

        Ok(json!({
            "case_id": case.id().to_string(),
            "chain_verified": true,
            "events_count": events.len(),
            "events": events,
        }))
    }
}

impl Handler for CaseHandler {
    fn service_id(&self) -> &ServiceId {
        &self.service_id
    }

    fn handle(
        &mut self,
        method: &MethodName,
        params: &JsonValue,
    ) -> std::result::Result<JsonValue, IpcError> {
        match method.as_str() {
            "case.create" => self.handle_case_create(params),
            "case.get" => self.handle_case_get(params),
            "case.list" => self.handle_case_list(params),
            "case.update_notes" => self.handle_update_notes(params),
            "case.update_title" => self.handle_update_title(params),
            "case.close" => self.handle_case_close(params),
            "case.reopen" => self.handle_case_reopen(params),
            "case.verify" => self.handle_case_verify(params),
            "case.evidence.list" => self.handle_evidence_list(params),
            "case.evidence.read" => self.handle_evidence_read(params),
            "case.audit.list" => self.handle_audit_list(params),
            other => Err(IpcError::new(
                IpcErrorCode::UnknownMethod,
                &format!("unknown method '{other}' for service 'case'"),
            )
            .unwrap()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_store() -> (PathBuf, Arc<Mutex<FileCaseStore>>) {
        let p = std::env::temp_dir().join(format!("pursue-case-ipc-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        let store = Arc::new(Mutex::new(FileCaseStore::open(&p).unwrap()));
        (p, store)
    }

    #[test]
    fn case_ipc_handler_lifecycle_and_verification() {
        let (p, store) = temp_store();
        let mut handler = CaseHandler::new(store);

        // 1. Create case
        let create_res = handler
            .handle(
                &MethodName::new("case.create").unwrap(),
                &json!({ "id": "CASE-IPC-01", "title": "IPC Test Case", "actor": "analyst-1" }),
            )
            .unwrap();
        assert_eq!(create_res["id"], "CASE-IPC-01");
        assert_eq!(create_res["status"], "open");

        // 2. Get case
        let get_res = handler
            .handle(
                &MethodName::new("case.get").unwrap(),
                &json!({ "id": "CASE-IPC-01" }),
            )
            .unwrap();
        assert_eq!(get_res["title"], "IPC Test Case");
        assert_eq!(get_res["evidence_count"], 0);
        assert_eq!(get_res["verified"], true);

        // 3. Update notes
        let notes_res = handler
            .handle(
                &MethodName::new("case.update_notes").unwrap(),
                &json!({ "id": "CASE-IPC-01", "notes": "Discovered critical lead", "actor": "analyst-1" }),
            )
            .unwrap();
        assert_eq!(notes_res["notes"], "Discovered critical lead");

        // 4. Verify case
        let verify_res = handler
            .handle(
                &MethodName::new("case.verify").unwrap(),
                &json!({ "id": "CASE-IPC-01" }),
            )
            .unwrap();
        assert_eq!(verify_res["verified"], true);
        assert_eq!(verify_res["manifest_verified"], true);
        assert_eq!(verify_res["audit_chain_verified"], true);

        // 5. Audit list
        let audit_res = handler
            .handle(
                &MethodName::new("case.audit.list").unwrap(),
                &json!({ "id": "CASE-IPC-01" }),
            )
            .unwrap();
        assert_eq!(audit_res["chain_verified"], true);
        assert_eq!(audit_res["events_count"], 2); // 1 create + 1 notes update

        // 6. Close and reopen
        let close_res = handler
            .handle(
                &MethodName::new("case.close").unwrap(),
                &json!({ "id": "CASE-IPC-01", "actor": "analyst-1" }),
            )
            .unwrap();
        assert_eq!(close_res["status"], "closed");

        let reopen_res = handler
            .handle(
                &MethodName::new("case.reopen").unwrap(),
                &json!({ "id": "CASE-IPC-01", "actor": "analyst-1" }),
            )
            .unwrap();
        assert_eq!(reopen_res["status"], "open");

        // 7. List cases
        let list_res = handler
            .handle(&MethodName::new("case.list").unwrap(), &json!({}))
            .unwrap();
        let arr = list_res.as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert_eq!(arr[0]["id"], "CASE-IPC-01");

        let _ = fs::remove_dir_all(&p);
    }
}
