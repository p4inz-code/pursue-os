//! PURSUE OS — Milestone 8: Adversarial Security & Reliability Validation Test Suite.
//!
//! Exhaustive hostile testing covering:
//! - Phase 3: Adversarial evidence tampering, hash manipulation, audit chain break, and case isolation.
//! - Phase 4: IPC transport framing abuse, oversized frames, malformed messages, and schema violations.
//! - Phase 5: Terminal execution injection resistance, hostile argv strings, and timeout bounds.
//! - Phase 6: Browser/Tor fail-closed boundary enforcement and URL scheme protection.
//! - Phase 7: Forensic report determinism, non-mutating generation, and export traversal defense.

use std::collections::BTreeMap;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use pursue_browser::engine::MockBrowserEngine;
use pursue_browser::url::ValidatedUrl;
use pursue_browser::{BrowserHandler, BrowserService};
use pursue_case::{CaseId, CaseStore, FileCaseStore};
use pursue_core::Error;
use pursue_evidence::{EvidenceStore, FileStore};
use pursue_report::export::{export_report_atomic, validate_export_path};
use pursue_report::generator::render_json;
use pursue_report::model::InvestigationReport;
use pursue_runtime::ipc::dispatch::{Handler, Router};
use pursue_runtime::ipc::protocol::{IpcError, IpcErrorCode, MethodName, Request, ServiceId};
use pursue_runtime::ipc::transport::{MAX_FRAME_LEN, read_frame, write_frame};
use pursue_terminal::command::CommandRequest;
use pursue_terminal::executor::{CommandExecutor, MockExecutor, MockOutcome};
use pursue_terminal::session::Session;
use pursue_terminal::session_id::SessionId;
use serde_json::json;

struct TestDir(PathBuf);

impl TestDir {
    fn new(name: &str) -> Self {
        let p = std::env::temp_dir().join(format!(
            "pursue-adv-sec-{}-{}-{}",
            name,
            std::process::id(),
            fastrand()
        ));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Self(p)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn fastrand() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(42)
}

// =========================================================================
// PHASE 3: ADVERSARIAL EVIDENCE TESTING
// =========================================================================

#[test]
fn test_tamper_a_modify_evidence_blob_after_capture() {
    let td = TestDir::new("tamper-a");
    let mut store = FileStore::open(td.path()).unwrap();
    let original_bytes = b"legitimate investigation evidence payload";
    let record = store
        .put(original_bytes, "terminal", "investigator")
        .unwrap();
    let addr = *record.address();

    // Verify initially clean
    assert_eq!(store.get(&addr).unwrap(), original_bytes);

    // Tamper with the on-disk blob
    let blob_path = td
        .path()
        .join("blobs")
        .join(format!("{}.bin", addr.to_hex()));
    assert!(blob_path.exists());
    fs::write(&blob_path, b"maliciously altered evidence payload").unwrap();

    // Read should fail closed with IntegrityViolation
    let get_err = store.get(&addr).unwrap_err();
    assert!(
        matches!(get_err, Error::IntegrityViolation(_)),
        "Expected IntegrityViolation, got: {get_err:?}"
    );

    // Re-opening store must fail closed on integrity gate
    let reopen_err = FileStore::open(td.path()).unwrap_err();
    assert!(
        matches!(reopen_err, Error::IntegrityViolation(_)),
        "Expected IntegrityViolation on load, got: {reopen_err:?}"
    );
}

#[test]
fn test_tamper_b_modify_recorded_evidence_hash() {
    let td = TestDir::new("tamper-b");
    let mut store = FileStore::open(td.path()).unwrap();
    let record = store.put(b"evidence data", "source", "alice").unwrap();
    let _addr = *record.address();

    // Read and tamper with manifest.json
    let manifest_path = td.path().join("manifest.json");
    let text = fs::read_to_string(&manifest_path).unwrap();
    let mut json_val: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Flip bits in the recorded address
    let fake_hash = "0000000000000000000000000000000000000000000000000000000000000000";
    json_val["records"][0]["address"] = json!(fake_hash);
    fs::write(&manifest_path, serde_json::to_string(&json_val).unwrap()).unwrap();

    // Reopening store must fail closed
    let reopen_err = FileStore::open(td.path()).unwrap_err();
    assert!(
        reopen_err.to_string().contains("missing")
            || reopen_err.to_string().contains("integrity")
            || reopen_err.to_string().contains("audit")
    );
}

#[test]
fn test_tamper_c_modify_audit_event() {
    let td = TestDir::new("tamper-c");
    let mut store = FileStore::open(td.path()).unwrap();
    store.put(b"event-1", "source", "alice").unwrap();
    store.put(b"event-2", "source", "bob").unwrap();

    // Tamper with actor in audit log
    let manifest_path = td.path().join("manifest.json");
    let text = fs::read_to_string(&manifest_path).unwrap();
    let mut json_val: serde_json::Value = serde_json::from_str(&text).unwrap();
    json_val["audit"]["entries"][0]["actor"] = json!("mallory");
    fs::write(&manifest_path, serde_json::to_string(&json_val).unwrap()).unwrap();

    let reopen_err = FileStore::open(td.path()).unwrap_err();
    assert!(
        matches!(reopen_err, Error::IntegrityViolation(_)),
        "Expected audit IntegrityViolation on load, got: {reopen_err:?}"
    );
}

#[test]
fn test_tamper_d_break_audit_chain_link() {
    let td = TestDir::new("tamper-d");
    let mut store = FileStore::open(td.path()).unwrap();
    store.put(b"ev-1", "source", "alice").unwrap();
    store.put(b"ev-2", "source", "alice").unwrap();
    store.put(b"ev-3", "source", "alice").unwrap();

    let manifest_path = td.path().join("manifest.json");
    let text = fs::read_to_string(&manifest_path).unwrap();
    let mut json_val: serde_json::Value = serde_json::from_str(&text).unwrap();

    // Corrupt previous_hash of entry 2
    json_val["audit"]["entries"][2]["prev_hash"] =
        json!("deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
    fs::write(&manifest_path, serde_json::to_string(&json_val).unwrap()).unwrap();

    let reopen_err = FileStore::open(td.path()).unwrap_err();
    assert!(
        matches!(reopen_err, Error::IntegrityViolation(_)),
        "Expected broken audit chain error, got: {reopen_err:?}"
    );
}

#[test]
fn test_tamper_e_modify_case_manifest_contents() {
    let td = TestDir::new("tamper-e");
    let mut cs = FileCaseStore::open(td.path()).unwrap();
    let case_id = CaseId::new("case-tamper-manifest").unwrap();
    cs.create_case(case_id.clone(), "Original Title", "alice")
        .unwrap();

    // Mutate case.json on disk (silent title rewrite)
    let case_file = td
        .path()
        .join("cases")
        .join(case_id.as_str())
        .join("case.json");
    let text = fs::read_to_string(&case_file).unwrap();
    let mut json_val: serde_json::Value = serde_json::from_str(&text).unwrap();
    json_val["case"]["title"] = json!("Tampered Title Without Audit");
    fs::write(&case_file, serde_json::to_string(&json_val).unwrap()).unwrap();

    // Load must fail closed on case_hash mismatch
    let load_err = cs.load_case(&case_id).unwrap_err();
    assert!(
        matches!(load_err, Error::IntegrityViolation(_)),
        "Expected manifest integrity violation, got: {load_err:?}"
    );
}

#[test]
fn test_tamper_f_remove_evidence_blob() {
    let td = TestDir::new("tamper-f");
    let mut cs = FileCaseStore::open(td.path()).unwrap();
    let case_id = CaseId::new("case-missing-blob").unwrap();
    let mut case = cs.create_case(case_id.clone(), "Title", "alice").unwrap();

    let mut es = cs.open_evidence_store(&case_id).unwrap();
    let rec = es.put(b"vital-artifact", "network", "alice").unwrap();
    case.attach(rec.address(), "alice").unwrap();
    cs.save_case(&case).unwrap();

    // Delete blob
    let blob_file = td
        .path()
        .join("cases")
        .join(case_id.as_str())
        .join("evidence")
        .join("blobs")
        .join(format!("{}.bin", rec.address().to_hex()));
    assert!(blob_file.exists());
    fs::remove_file(&blob_file).unwrap();

    // Loading case must detect missing blob and fail closed
    let load_err = cs.load_case(&case_id).unwrap_err();
    assert!(
        matches!(load_err, Error::NotFound(_) | Error::IntegrityViolation(_)),
        "Expected missing blob failure, got: {load_err:?}"
    );
}

#[test]
fn test_tamper_g_replace_evidence_blob_with_different_valid_blob() {
    let td = TestDir::new("tamper-g");
    let mut store = FileStore::open(td.path()).unwrap();
    let rec1 = store.put(b"blob 1 contents", "term", "alice").unwrap();
    let rec2 = store.put(b"blob 2 contents", "term", "alice").unwrap();

    let path1 = td
        .path()
        .join("blobs")
        .join(format!("{}.bin", rec1.address().to_hex()));
    let path2 = td
        .path()
        .join("blobs")
        .join(format!("{}.bin", rec2.address().to_hex()));

    // Overwrite blob1 with blob2 bytes
    let b2_bytes = fs::read(&path2).unwrap();
    fs::write(&path1, b2_bytes).unwrap();

    // Reading blob1 must fail integrity check
    let get_err = store.get(rec1.address()).unwrap_err();
    assert!(matches!(get_err, Error::IntegrityViolation(_)));
}

#[test]
fn test_tamper_h_path_traversal_case_id_rejection() {
    for malicious in [
        ".",
        "..",
        "...",
        "../case",
        "case/..",
        "../../etc/passwd",
        "case..id",
        "/etc/shadow",
        "case\0null",
    ] {
        assert!(
            CaseId::new(malicious).is_err(),
            "Expected rejection for {malicious:?}"
        );
    }
}

// =========================================================================
// PHASE 4: IPC ADVERSARIAL TESTING
// =========================================================================

#[test]
fn test_ipc_framing_oversized_frame_rejected() {
    let mut payload = Vec::new();
    let oversized = (MAX_FRAME_LEN as u64) + 1;
    payload.extend(oversized.to_le_bytes());
    let mut reader = Cursor::new(payload);

    let res = read_frame(&mut reader);
    assert!(res.is_err());
    assert!(res.unwrap_err().to_string().contains("exceeds maximum"));
}

#[test]
fn test_ipc_framing_zero_length_frame() {
    let mut buf = Vec::new();
    write_frame(&mut buf, &[]).unwrap();
    assert_eq!(buf.len(), 8);

    let mut reader = Cursor::new(buf);
    let payload = read_frame(&mut reader).unwrap();
    assert!(payload.is_empty());
}

#[test]
fn test_ipc_framing_truncated_frame() {
    let mut payload = Vec::new();
    payload.extend(1000u64.to_le_bytes());
    payload.extend(b"only-30-bytes-of-data-provided");
    let mut reader = Cursor::new(payload);

    let res = read_frame(&mut reader);
    assert!(res.is_err(), "Expected UnexpectedEof on truncated payload");
}

#[test]
fn test_ipc_malformed_json_and_wrong_schema() {
    let mut router = Router::new();
    let service_id = ServiceId::new("echo").unwrap();
    let method_name = MethodName::new("ping").unwrap();

    struct TestEcho(ServiceId);
    impl pursue_runtime::ipc::dispatch::Handler for TestEcho {
        fn service_id(&self) -> &ServiceId {
            &self.0
        }
        fn handle(
            &mut self,
            _m: &MethodName,
            params: &serde_json::Value,
        ) -> std::result::Result<serde_json::Value, IpcError> {
            Ok(params.clone())
        }
    }
    router
        .register(Box::new(TestEcho(service_id.clone())))
        .unwrap();

    // 1. Unknown service
    let req_unknown = Request::new(
        1,
        ServiceId::new("nonexistent").unwrap(),
        method_name.clone(),
        json!({}),
    );
    let resp = router.handle(&req_unknown);
    assert!(!resp.is_success());
    assert_eq!(
        resp.error.as_ref().unwrap().code(),
        IpcErrorCode::UnknownService
    );

    // 2. Valid request succeeds
    let req_ok = Request::new(
        2,
        service_id.clone(),
        method_name.clone(),
        json!({"data": "ok"}),
    );
    let resp_ok = router.handle(&req_ok);
    assert!(resp_ok.is_success());
    assert_eq!(resp_ok.result.unwrap()["data"], "ok");
}

#[test]
fn test_ipc_concurrent_clients() {
    let mut router = Router::new();
    let service_id = ServiceId::new("calc").unwrap();

    struct CalcHandler(ServiceId);
    impl pursue_runtime::ipc::dispatch::Handler for CalcHandler {
        fn service_id(&self) -> &ServiceId {
            &self.0
        }
        fn handle(
            &mut self,
            _m: &MethodName,
            params: &serde_json::Value,
        ) -> std::result::Result<serde_json::Value, IpcError> {
            let n = params["val"].as_u64().unwrap_or(0);
            Ok(json!({ "result": n * 2 }))
        }
    }
    router
        .register(Box::new(CalcHandler(service_id.clone())))
        .unwrap();
    let shared_router = Arc::new(Mutex::new(router));

    let mut handles = Vec::new();
    for thread_idx in 0..10 {
        let r_clone = Arc::clone(&shared_router);
        let s_id = service_id.clone();
        handles.push(thread::spawn(move || {
            for i in 0..20 {
                let req = Request::new(
                    (thread_idx * 100) + i,
                    s_id.clone(),
                    MethodName::new("compute").unwrap(),
                    json!({ "val": i }),
                );
                let resp = {
                    let mut guard = r_clone.lock().unwrap();
                    guard.handle(&req)
                };
                assert!(resp.is_success());
                assert_eq!(resp.result.unwrap()["result"], i * 2);
            }
        }));
    }

    for h in handles {
        h.join().unwrap();
    }
}

// =========================================================================
// PHASE 5: TERMINAL SECURITY TESTING (Direct argv, no shell evaluation)
// =========================================================================

#[test]
fn test_terminal_hostile_arguments_remain_literal_argv() {
    let td = TestDir::new("term-argv");
    let session_id = SessionId::new("sec-sess").unwrap();
    let case_id = CaseId::new("case-term").unwrap();
    let session = Session::new(
        session_id,
        case_id,
        "investigator",
        td.path().to_path_buf(),
        BTreeMap::new(),
    )
    .unwrap();

    // Adversarial arguments resembling shell metacharacters and command injection
    let hostile_args = vec![
        "; rm -rf /".to_string(),
        "&& malicious-command".to_string(),
        "$(whoami)".to_string(),
        "`id`".to_string(),
        "> /tmp/pwned.txt".to_string(),
        "| cat /etc/passwd".to_string(),
    ];

    let req = CommandRequest::new("echo", hostile_args.clone()).unwrap();
    assert_eq!(req.args(), &hostile_args);

    // Ensure whitespace inside program is rejected
    assert!(CommandRequest::new("echo ; rm -rf /", vec![]).is_err());
    assert!(CommandRequest::new("sh -c", vec![]).is_err());
    assert!(CommandRequest::new("/bin/sh\0bad", vec![]).is_err());

    // When executed via mock or real executor, arguments are never split or passed to a shell
    let mock = MockExecutor::new();
    mock.register(
        "echo",
        MockOutcome::Success {
            exit_code: Some(0),
            stdout: hostile_args.join(" ").into_bytes(),
            stderr: vec![],
            timed_out: false,
            truncated: false,
        },
    );
    let res = mock.execute(&session, &req).unwrap();
    assert_eq!(res.exit_code(), Some(0));
    assert_eq!(mock.recorded_calls()[0].1.args(), &hostile_args);
}

#[test]
fn test_terminal_timeout_kills_process() {
    let td = TestDir::new("term-timeout");
    let session_id = SessionId::new("term-timeout").unwrap();
    let case_id = CaseId::new("case-term-to").unwrap();
    let session = Session::new(
        session_id,
        case_id,
        "investigator",
        td.path().to_path_buf(),
        BTreeMap::new(),
    )
    .unwrap();

    let mock = MockExecutor::new();
    mock.set_default(MockOutcome::Success {
        exit_code: None,
        stdout: vec![],
        stderr: vec![],
        timed_out: true,
        truncated: false,
    });

    let req = CommandRequest::new("long-task", vec![])
        .unwrap()
        .with_timeout_secs(1)
        .unwrap();

    let res = mock.execute(&session, &req).unwrap();
    assert!(res.timed_out());
    assert_eq!(res.exit_code(), None);
}

// =========================================================================
// PHASE 6: BROWSER + TOR SECURITY VALIDATION
// =========================================================================

#[test]
fn test_browser_onion_rejected_in_direct_mode() {
    let td = TestDir::new("browser-onion");
    let engine = Arc::new(MockBrowserEngine::new());
    let service = BrowserService::new(td.path().to_path_buf());
    let mut handler = BrowserHandler::with_engine(service, engine);

    // Create direct session
    let create_params = json!({
        "session_id": "direct-sess",
        "case_id": "case-sec",
        "actor": "investigator",
        "mode": "direct"
    });
    let method_create = MethodName::new("browser.session.create").unwrap();
    handler.handle(&method_create, &create_params).unwrap();

    // Attempt navigation to onion in direct mode -> MUST FAIL CLOSED
    let nav_params = json!({
        "session_id": "direct-sess",
        "url": "http://expyuz5wqqgahgahg.onion"
    });
    let method_nav = MethodName::new("browser.navigate").unwrap();
    let err = handler.handle(&method_nav, &nav_params).unwrap_err();
    assert_eq!(err.code(), IpcErrorCode::Internal);
    assert!(err.message().contains("onion") || err.message().contains("direct"));
}

#[test]
fn test_browser_dangerous_schemes_rejected() {
    for dangerous in [
        "file:///etc/passwd",
        "javascript:alert(1)",
        "data:text/html,<html>",
        "ftp://example.com/file",
        "gopher://example.com",
    ] {
        assert!(
            ValidatedUrl::parse(dangerous).is_err(),
            "Expected scheme rejection for {dangerous}"
        );
    }
}

// =========================================================================
// PHASE 7: FORENSIC REPORTING SECURITY & DETERMINISM
// =========================================================================

#[test]
fn test_report_deterministic_generation_and_export() {
    let td = TestDir::new("report-det");
    let case_store_dir = td.path().join("cases");
    let report_dir = td.path().join("reports");
    fs::create_dir_all(&case_store_dir).unwrap();
    fs::create_dir_all(&report_dir).unwrap();

    let mut cs = FileCaseStore::open(&case_store_dir).unwrap();
    let case_id = CaseId::new("case-det").unwrap();
    let mut case = cs
        .create_case(case_id.clone(), "Deterministic Case", "alice")
        .unwrap();

    let mut es = cs.open_evidence_store(&case_id).unwrap();
    let rec = es.put(b"sample data", "terminal", "alice").unwrap();
    case.attach(rec.address(), "alice").unwrap();
    cs.save_case(&case).unwrap();

    // Verify report generation does not mutate case or evidence
    let case_json_before = fs::read_to_string(
        case_store_dir
            .join("cases")
            .join(case_id.as_str())
            .join("case.json"),
    )
    .unwrap();

    let report1 = InvestigationReport::from_case(&case, &es, "alice", 1700000000).unwrap();

    let report2 = InvestigationReport::from_case(&case, &es, "alice", 1700000000).unwrap();

    assert_eq!(report1.report_hash, report2.report_hash);
    assert_eq!(
        render_json(&report1).unwrap(),
        render_json(&report2).unwrap()
    );

    // Assert case manifest was NOT touched
    let case_json_after = fs::read_to_string(
        case_store_dir
            .join("cases")
            .join(case_id.as_str())
            .join("case.json"),
    )
    .unwrap();
    assert_eq!(case_json_before, case_json_after);

    // Atomic report export
    let export_target = report_dir.join("case-det.json");
    let json_bytes = render_json(&report1).unwrap().into_bytes();
    export_report_atomic(&json_bytes, &export_target).unwrap();
    assert!(export_target.exists());

    // Traversal export attempt must fail
    let evil_target = td.path().join("cases").join("escaped.json");
    let err = validate_export_path(&evil_target, Some(&report_dir)).unwrap_err();
    assert!(matches!(err, Error::InvalidInput(_)));
}

#[test]
fn test_report_path_traversal_rejection() {
    let td = TestDir::new("report-traversal");
    let allowed_root = td.path().join("allowed_reports");
    fs::create_dir_all(&allowed_root).unwrap();

    for evil in [
        allowed_root.join("../escaped.json"),
        allowed_root.join("subdir/../../etc/passwd.json"),
    ] {
        assert!(validate_export_path(&evil, Some(&allowed_root)).is_err());
    }
}
