//! PURSUE OS — Forensic Reporting and Safe Export Foundation (Phase 4).
//!
//! Provides deterministic report generation (canonical JSON and self-contained,
//! printable HTML), unified investigation timeline aggregation, hash-chained
//! audit verification, path traversal safety, secret scrubbing, and IPC service handlers.

#![deny(unsafe_code)]
#![warn(missing_docs)]

pub mod export;
pub mod generator;
pub mod ipc;
pub mod model;

pub use export::{export_report_atomic, scrub_secrets, validate_export_path};
pub use generator::{ReportFormat, escape_html, render_html, render_json};
pub use ipc::ReportHandler;
pub use model::{
    AuditReportSummary, EvidenceReportItem, InvestigationReport, ReportMetadata, TimelineReportItem,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    use pursue_case::{CaseId, CaseStore, FileCaseStore};
    use pursue_evidence::EvidenceStore;
    use pursue_runtime::ipc::dispatch::Handler;
    use pursue_runtime::ipc::protocol::MethodName;
    use serde_json::json;

    fn temp_test_env(name: &str) -> (PathBuf, Arc<Mutex<FileCaseStore>>) {
        let p = std::env::temp_dir().join(format!(
            "pursue-report-test-{}-{}-{}",
            std::process::id(),
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        let store = Arc::new(Mutex::new(FileCaseStore::open(&p).unwrap()));
        (p, store)
    }

    #[test]
    fn deterministic_json_generation_and_tampering_detection() {
        let (root, store_arc) = temp_test_env("det-json");
        let case_id = CaseId::new("CASE-REP-01").unwrap();

        // 1. Setup case with evidence
        {
            let mut store = store_arc.lock().unwrap();
            let mut case = store
                .create_case(case_id.clone(), "Operation Alpha", "investigator-1")
                .unwrap();
            case.set_notes("Primary investigation notes.", "investigator-1")
                .unwrap();

            let mut ev_store = store.open_evidence_store(&case_id).unwrap();
            let payload = b"Network artifact payload log 12345";
            let rec = ev_store
                .put(payload, "net.capture", "investigator-1")
                .unwrap();

            case.attach(rec.address(), "investigator-1").unwrap();
            store.save_case(&case).unwrap();
        }

        // 2. Generate report
        let store = store_arc.lock().unwrap();
        let case = store.load_case(&case_id).unwrap();
        let ev_store = store.open_evidence_store(&case_id).unwrap();

        let report1 = InvestigationReport::from_case(&case, &ev_store, "analyst-bob", 1700000100)
            .unwrap()
            .finalize_with_hash();

        let report2 = InvestigationReport::from_case(&case, &ev_store, "analyst-bob", 1700000100)
            .unwrap()
            .finalize_with_hash();

        // Test determinism
        assert_eq!(report1.report_hash, report2.report_hash);
        let json1 = render_json(&report1).unwrap();
        let json2 = render_json(&report2).unwrap();
        assert_eq!(json1, json2);

        // Verify valid integrity
        assert!(report1.verify_integrity().is_ok());

        // Tamper test: modify notes without updating hash
        let mut tampered = report1.clone();
        tampered.metadata.case_notes = "Tampered notes".into();
        assert!(tampered.verify_integrity().is_err());

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn self_contained_html_generation() {
        let (root, store_arc) = temp_test_env("html-gen");
        let case_id = CaseId::new("CASE-REP-HTML").unwrap();

        {
            let mut store = store_arc.lock().unwrap();
            let case = store
                .create_case(case_id.clone(), "HTML Report Test", "analyst-1")
                .unwrap();
            store.save_case(&case).unwrap();
        }

        let store = store_arc.lock().unwrap();
        let case = store.load_case(&case_id).unwrap();
        let ev_store = store.open_evidence_store(&case_id).unwrap();

        let report = InvestigationReport::from_case(&case, &ev_store, "analyst-1", 1700000200)
            .unwrap()
            .finalize_with_hash();

        let html = render_html(&report).unwrap();

        // HTML must be self contained - check that no external CDN/scripts are present
        assert!(!html.contains("http://"));
        assert!(!html.contains("https://"));
        assert!(!html.contains("<script"));
        assert!(html.contains("@media print"));
        assert!(html.contains("Cryptographic Integrity Seal"));
        assert!(html.contains("CASE-REP-HTML"));

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn path_traversal_rejection() {
        let p_rel_traversal = Path::new("sub/../../etc/passwd");
        assert!(validate_export_path(p_rel_traversal, None).is_err());

        let p_null_byte = Path::new("sub/test\0file.json");
        assert!(validate_export_path(p_null_byte, None).is_err());

        let temp_dir = std::env::temp_dir().join(format!("allowed-root-{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let safe = temp_dir.join("report.html");
        assert!(validate_export_path(&safe, Some(&temp_dir)).is_ok());

        let outside = std::env::temp_dir().join("outside.html");
        // Outside allowed root must be rejected
        assert!(validate_export_path(&outside, Some(&temp_dir)).is_err());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn secret_scrubbing_behavior() {
        let raw = "User auth header: Bearer secret_token_xyz_123; connect ok";
        let scrubbed = scrub_secrets(raw);
        assert!(!scrubbed.contains("secret_token_xyz_123"));
        assert!(scrubbed.contains("[REDACTED_SECRET]"));
    }

    #[test]
    fn report_ipc_handler_workflow() {
        let (root, store_arc) = temp_test_env("ipc-flow");
        let case_id = CaseId::new("CASE-REP-IPC").unwrap();

        {
            let mut store = store_arc.lock().unwrap();
            let case = store
                .create_case(case_id.clone(), "IPC Case", "analyst-1")
                .unwrap();
            store.save_case(&case).unwrap();
        }

        let mut handler = ReportHandler::new(store_arc.clone());

        // 1. Preview metadata
        let preview = handler
            .handle(
                &MethodName::new("report.preview_metadata").unwrap(),
                &json!({ "case_id": "CASE-REP-IPC" }),
            )
            .unwrap();
        assert_eq!(preview["case_id"], "CASE-REP-IPC");
        assert_eq!(preview["audit_chain_verified"], true);

        // 2. Generate JSON
        let gen_json = handler
            .handle(
                &MethodName::new("report.generate").unwrap(),
                &json!({ "case_id": "CASE-REP-IPC", "format": "json" }),
            )
            .unwrap();
        assert!(gen_json["report_hash"].as_str().unwrap().len() > 10);
        assert_eq!(gen_json["format"], "json");

        // 3. Export HTML
        let export_dir = root.join("exports");
        let export_file = export_dir.join("report.html");

        let export_res = handler
            .handle(
                &MethodName::new("report.export").unwrap(),
                &json!({
                    "case_id": "CASE-REP-IPC",
                    "format": "html",
                    "target_path": export_file.to_str().unwrap()
                }),
            )
            .unwrap();
        assert_eq!(export_res["verified"], true);
        assert!(export_file.exists());

        // 4. Verify file
        let verify_res = handler
            .handle(
                &MethodName::new("report.verify").unwrap(),
                &json!({ "file_path": export_file.to_str().unwrap() }),
            )
            .unwrap();
        assert_eq!(verify_res["verified"], true);

        let _ = fs::remove_dir_all(&root);
    }
}
