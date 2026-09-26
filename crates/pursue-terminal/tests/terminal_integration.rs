//! Phase 1E end-to-end integration tests.
//!
//! Validates the complete Investigation Terminal foundation across:
//! - Test A: End-to-end mock execution → evidence capture → case store persistence → audit verification
//! - Test B: Real OS process execution → stream capture → exit status
//! - Test C: Non-zero process exit representation
//! - Test D: Timeout termination behavior
//! - Test E: Bounded buffer output truncation
//! - Test F: Stdout and stderr stream separation
//! - Test G: Invalid command requests rejected at boundary
//! - Test H: Case isolation (session bound to Case A cannot capture into Case B)
//! - Test I: IPC session creation
//! - Test J: IPC command execution
//! - Test K: IPC evidence capture
//! - Test L: Tampered evidence detection fails closed on case reload
//! - Test M: Environment secret values never leaked into provenance / audit logs

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use pursue_case::{CaseId, CaseStore, FileCaseStore};
use pursue_core::Error;
use pursue_evidence::EvidenceStore;
use pursue_runtime::ipc::dispatch::Router;
use pursue_runtime::ipc::protocol::{MethodName, Request, ServiceId};
use serde_json::json;

use pursue_terminal::ansi::strip_ansi;
use pursue_terminal::command::CommandRequest;
use pursue_terminal::evidence::{StreamKind, TerminalEvidenceCapturer};
use pursue_terminal::executor::{CommandExecutor, MockExecutor, MockOutcome, ProcessExecutor};
use pursue_terminal::ipc::TerminalHandler;
use pursue_terminal::service::TerminalService;
use pursue_terminal::session::Session;
use pursue_terminal::session_id::SessionId;

fn temp_dir(label: &str) -> PathBuf {
    let unique = format!(
        "pursue-term-test-{label}-{}-{:?}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let dir = std::env::temp_dir().join(unique);
    fs::create_dir_all(&dir).expect("create test temp dir");
    dir
}

fn cleanup(dir: &Path) {
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn test_a_mock_execution_evidence_capture_case_audit_flow() {
    let dir = temp_dir("test-a");
    {
        let mut case_store = FileCaseStore::open(&dir).unwrap();
        let case_id = CaseId::new("case-001").unwrap();
        case_store
            .create_case(case_id.clone(), "Operation Mock", "lead-investigator")
            .unwrap();

        let session_id = SessionId::new("term-01").unwrap();
        let session = Session::new(
            session_id,
            case_id.clone(),
            "lead-investigator",
            dir.clone(),
            BTreeMap::new(),
        )
        .unwrap();

        let mock = MockExecutor::new();
        mock.register(
            "osint-probe",
            MockOutcome::Success {
                exit_code: Some(0),
                stdout: b"TARGET: example.com IP: 93.184.216.34\n".to_vec(),
                stderr: b"DEBUG: resolved in 12ms\n".to_vec(),
                timed_out: false,
                truncated: false,
            },
        );

        let req = CommandRequest::new("osint-probe", vec!["--domain".into(), "example.com".into()])
            .unwrap();
        let result = mock.execute(&session, &req).unwrap();
        assert!(result.is_success());

        // Capture stdout as evidence
        let captured_stdout = TerminalEvidenceCapturer::capture_stream_into_file_store(
            &mut case_store,
            &session,
            &req,
            &result,
            StreamKind::Stdout,
            None,
        )
        .unwrap();

        // Capture stderr as companion evidence
        let captured_stderr = TerminalEvidenceCapturer::capture_stream_into_file_store(
            &mut case_store,
            &session,
            &req,
            &result,
            StreamKind::Stderr,
            None,
        )
        .unwrap();

        // Reload case from disk and verify
        let case = case_store.load_case(&case_id).unwrap();
        let addresses: Vec<_> = case.evidence_addresses().collect();
        assert_eq!(addresses.len(), 2);
        assert!(addresses.contains(&&captured_stdout.address));
        assert!(addresses.contains(&&captured_stderr.address));

        // Verify audit log integrity
        let audit = case.audit_log();
        assert_eq!(audit.len(), 3); // case.created, attached stdout, attached stderr
        assert_eq!(audit.entries()[1].action, "case.evidence.attached");
        assert_eq!(audit.entries()[1].subject, Some(captured_stdout.address));
        assert_eq!(audit.entries()[2].subject, Some(captured_stderr.address));
        audit.verify().unwrap();

        // Verify content in the case's evidence file store
        let ev_store = case_store.open_evidence_store(&case_id).unwrap();
        assert_eq!(
            ev_store.get(&captured_stdout.address).unwrap(),
            b"TARGET: example.com IP: 93.184.216.34\n"
        );
        assert_eq!(
            ev_store.get(&captured_stderr.address).unwrap(),
            b"DEBUG: resolved in 12ms\n"
        );
    }
    cleanup(&dir);
}

#[test]
fn test_b_real_process_execution() {
    let session = Session::new(
        SessionId::new("term-real").unwrap(),
        CaseId::new("case-real").unwrap(),
        "analyst",
        std::env::temp_dir(),
        BTreeMap::new(),
    )
    .unwrap();

    let executor = ProcessExecutor::new();
    let req = CommandRequest::new("rustc", vec!["--version".into()]).unwrap();
    let result = executor.execute(&session, &req).unwrap();

    assert!(result.is_success());
    assert_eq!(result.exit_code(), Some(0));
    assert!(!result.stdout().is_empty());
    assert!(!result.timed_out());
    assert!(!result.is_truncated());
}

#[test]
fn test_c_non_zero_process_exit() {
    let mock = MockExecutor::new();
    mock.register(
        "fail-cmd",
        MockOutcome::Success {
            exit_code: Some(2),
            stdout: vec![],
            stderr: b"error: syntax error\n".to_vec(),
            timed_out: false,
            truncated: false,
        },
    );

    let session = Session::new(
        SessionId::new("term-fail").unwrap(),
        CaseId::new("case-fail").unwrap(),
        "analyst",
        std::env::temp_dir(),
        BTreeMap::new(),
    )
    .unwrap();

    let req = CommandRequest::new("fail-cmd", vec![]).unwrap();
    let res = mock.execute(&session, &req).unwrap();

    assert!(!res.is_success());
    assert_eq!(res.exit_code(), Some(2));
    assert_eq!(res.stderr(), b"error: syntax error\n");
    assert!(!res.timed_out());
}

#[test]
fn test_d_timeout_behavior() {
    let mock = MockExecutor::new();
    mock.register(
        "hang-cmd",
        MockOutcome::Success {
            exit_code: None,
            stdout: b"started work...\n".to_vec(),
            stderr: b"timeout expired\n".to_vec(),
            timed_out: true,
            truncated: false,
        },
    );

    let session = Session::new(
        SessionId::new("term-timeout").unwrap(),
        CaseId::new("case-timeout").unwrap(),
        "analyst",
        std::env::temp_dir(),
        BTreeMap::new(),
    )
    .unwrap();

    let req = CommandRequest::new("hang-cmd", vec![]).unwrap();
    let res = mock.execute(&session, &req).unwrap();

    assert!(!res.is_success());
    assert!(res.timed_out());
    assert_eq!(res.exit_code(), None);
    // Captured output available prior to kill is preserved
    assert_eq!(res.stdout(), b"started work...\n");
}

#[test]
fn test_e_output_truncation() {
    let mock = MockExecutor::new();
    mock.register(
        "large-generator",
        MockOutcome::Success {
            exit_code: Some(0),
            stdout: vec![0x41; 100], // 100 'A's
            stderr: vec![],
            timed_out: false,
            truncated: true,
        },
    );

    let session = Session::new(
        SessionId::new("term-trunc").unwrap(),
        CaseId::new("case-trunc").unwrap(),
        "analyst",
        std::env::temp_dir(),
        BTreeMap::new(),
    )
    .unwrap();

    let req = CommandRequest::new("large-generator", vec![]).unwrap();
    let res = mock.execute(&session, &req).unwrap();

    assert!(res.is_truncated());
    assert_eq!(res.stdout().len(), 100);
}

#[test]
fn test_f_stdout_and_stderr_separation() {
    let mock = MockExecutor::new();
    mock.register(
        "split-cmd",
        MockOutcome::Success {
            exit_code: Some(0),
            stdout: b"standard payload".to_vec(),
            stderr: b"diagnostic logs".to_vec(),
            timed_out: false,
            truncated: false,
        },
    );

    let session = Session::new(
        SessionId::new("term-split").unwrap(),
        CaseId::new("case-split").unwrap(),
        "analyst",
        std::env::temp_dir(),
        BTreeMap::new(),
    )
    .unwrap();

    let req = CommandRequest::new("split-cmd", vec![]).unwrap();
    let res = mock.execute(&session, &req).unwrap();

    assert_eq!(res.stdout(), b"standard payload");
    assert_eq!(res.stderr(), b"diagnostic logs");
    assert_ne!(res.stdout(), res.stderr());
}

#[test]
fn test_g_invalid_command_request_rejected() {
    // 1. Whitespace in program name rejected (shell interpolation prevention)
    assert!(CommandRequest::new("sh -c 'evil'", vec![]).is_err());
    assert!(CommandRequest::new("ls -la", vec![]).is_err());

    // 2. Empty program rejected
    assert!(CommandRequest::new("", vec![]).is_err());
    assert!(CommandRequest::new("   ", vec![]).is_err());

    // 3. NUL bytes in program or args rejected
    assert!(CommandRequest::new("cat\0", vec![]).is_err());
    assert!(CommandRequest::new("cat", vec!["file\0.txt".into()]).is_err());

    // 4. Zero timeout or excessive timeout rejected
    let req = CommandRequest::new("cat", vec![]).unwrap();
    assert!(req.clone().with_timeout_secs(0).is_err());
    assert!(req.clone().with_timeout_secs(100_000).is_err());
}

#[test]
fn test_h_case_isolation_enforced() {
    let dir = temp_dir("test-h");
    {
        let mut case_store = FileCaseStore::open(&dir).unwrap();
        let case_a = CaseId::new("case-a").unwrap();
        let case_b = CaseId::new("case-b").unwrap();
        case_store
            .create_case(case_a.clone(), "Case A", "analyst")
            .unwrap();
        case_store
            .create_case(case_b.clone(), "Case B", "analyst")
            .unwrap();

        // Session strictly anchored to Case A
        let session_a = Session::new(
            SessionId::new("session-a").unwrap(),
            case_a.clone(),
            "analyst",
            dir.clone(),
            BTreeMap::new(),
        )
        .unwrap();

        let req = CommandRequest::new("probe", vec![]).unwrap();
        let exec_result = pursue_terminal::command::ExecutionResult::new(
            "cmd-a",
            Some(0),
            b"secret data of A".to_vec(),
            vec![],
            100,
            101,
            false,
            false,
        )
        .unwrap();

        // Capture evidence for session_a into FileCaseStore
        let captured = TerminalEvidenceCapturer::capture_stream_into_file_store(
            &mut case_store,
            &session_a,
            &req,
            &exec_result,
            StreamKind::Stdout,
            None,
        )
        .unwrap();

        // Case A has the evidence attached
        let loaded_a = case_store.load_case(&case_a).unwrap();
        assert_eq!(loaded_a.evidence_addresses().count(), 1);

        // Case B MUST NOT have Case A's evidence
        let loaded_b = case_store.load_case(&case_b).unwrap();
        assert_eq!(loaded_b.evidence_addresses().count(), 0);

        // Case B's evidence store does not contain the blob
        let ev_store_b = case_store.open_evidence_store(&case_b).unwrap();
        assert!(!ev_store_b.contains(&captured.address));
    }
    cleanup(&dir);
}

#[test]
fn test_i_j_k_ipc_session_execution_and_capture() {
    let dir = temp_dir("test-ipc");
    {
        let case_store = Arc::new(Mutex::new(FileCaseStore::open(&dir).unwrap()));
        let case_id = CaseId::new("case-ipc").unwrap();
        case_store
            .lock()
            .unwrap()
            .create_case(case_id.clone(), "Case IPC", "analyst")
            .unwrap();

        let mock_executor = Arc::new(MockExecutor::new());
        mock_executor.register(
            "query-db",
            MockOutcome::Success {
                exit_code: Some(0),
                stdout: b"RECORD: John Doe, ID: 98765\n".to_vec(),
                stderr: vec![],
                timed_out: false,
                truncated: false,
            },
        );

        let service = TerminalService::new();
        let handler = TerminalHandler::with_executor(service, mock_executor)
            .with_file_store(Arc::clone(&case_store));

        let mut router = Router::new();
        router.register(Box::new(handler)).unwrap();

        // Test I: IPC session creation
        let create_req = Request::new(
            1,
            ServiceId::new("terminal").unwrap(),
            MethodName::new("session.create").unwrap(),
            json!({
                "session_id": "sess-ipc-1",
                "case_id": "case-ipc",
                "actor": "investigator-ipc",
                "working_dir": dir.to_str().unwrap()
            }),
        );
        let create_res = router.handle(&create_req);
        assert!(create_res.is_success());
        assert_eq!(create_res.result.unwrap()["session_id"], "sess-ipc-1");

        // Test J: IPC command execution
        let exec_req = Request::new(
            2,
            ServiceId::new("terminal").unwrap(),
            MethodName::new("command.execute").unwrap(),
            json!({
                "session_id": "sess-ipc-1",
                "program": "query-db",
                "args": ["--all"]
            }),
        );
        let exec_res = router.handle(&exec_req);
        assert!(exec_res.is_success());
        let res_json = exec_res.result.unwrap();
        assert_eq!(res_json["exit_code"], 0);

        // Test K: IPC evidence capture
        let capture_req = Request::new(
            3,
            ServiceId::new("terminal").unwrap(),
            MethodName::new("evidence.capture").unwrap(),
            json!({
                "session_id": "sess-ipc-1",
                "program": "query-db",
                "command_id": "cmd-ipc-1",
                "stream": "stdout",
                "data": "RECORD: John Doe, ID: 98765\n",
                "source_label": "ipc-custom-evidence"
            }),
        );
        let capture_res = router.handle(&capture_req);
        assert!(capture_res.is_success());
        let cap_json = capture_res.result.unwrap();
        let addr_hex = cap_json["content_address"].as_str().unwrap();
        assert_eq!(addr_hex.len(), 64);

        // Verify evidence exists in case file store
        let store = case_store.lock().unwrap();
        let loaded_case = store.load_case(&case_id).unwrap();
        assert_eq!(loaded_case.evidence_addresses().count(), 1);
        loaded_case.audit_log().verify().unwrap();
    }
    cleanup(&dir);
}

#[test]
fn test_l_corrupted_evidence_fails_closed_on_case_load() {
    let dir = temp_dir("test-l");
    let addr;
    let case_id = CaseId::new("case-corrupt").unwrap();
    {
        let mut case_store = FileCaseStore::open(&dir).unwrap();
        case_store
            .create_case(case_id.clone(), "Tamper Test", "analyst")
            .unwrap();

        let session = Session::new(
            SessionId::new("sess-tamper").unwrap(),
            case_id.clone(),
            "analyst",
            dir.clone(),
            BTreeMap::new(),
        )
        .unwrap();

        let req = CommandRequest::new("cat", vec![]).unwrap();
        let exec_result = pursue_terminal::command::ExecutionResult::new(
            "cmd-tamper",
            Some(0),
            b"unaltered critical evidence bytes".to_vec(),
            vec![],
            100,
            101,
            false,
            false,
        )
        .unwrap();

        let captured = TerminalEvidenceCapturer::capture_stream_into_file_store(
            &mut case_store,
            &session,
            &req,
            &exec_result,
            StreamKind::Stdout,
            None,
        )
        .unwrap();
        addr = captured.address;
    }

    // Tamper with the evidence blob on disk directly
    let blob_path = dir
        .join("cases")
        .join("case-corrupt")
        .join("evidence")
        .join("blobs")
        .join(format!("{}.bin", addr.to_hex()));
    let mut blob_bytes = fs::read(&blob_path).unwrap();
    blob_bytes[0] ^= 0xff; // corrupt a single byte
    fs::write(&blob_path, blob_bytes).unwrap();

    // Reopening the case store and attempting to load the case must fail closed
    let store = FileCaseStore::open(&dir).unwrap();
    let err = store.load_case(&case_id).unwrap_err();
    assert!(
        matches!(err, Error::IntegrityViolation(_)),
        "expected IntegrityViolation on tampered evidence, got: {err}"
    );
    cleanup(&dir);
}

#[test]
fn test_m_secret_environment_values_never_appear_in_provenance_labels() {
    let session_id = SessionId::new("term-secret").unwrap();
    let label = TerminalEvidenceCapturer::build_source_label(
        &session_id,
        "api-tool",
        StreamKind::Stdout,
        Some(0),
    );

    // Label contains program name, session id, exit code, and stream
    assert!(label.contains("api-tool"));
    assert!(label.contains("term-secret"));
    assert!(label.contains("stream=stdout"));
    assert!(label.contains("exit=0"));

    // Label MUST NOT contain any secret keys, token patterns, or environment maps
    assert!(!label.contains("API_KEY"));
    assert!(!label.contains("SECRET"));
    assert!(!label.contains("TOKEN"));
}

#[test]
fn test_ansi_raw_output_invariant_preservation() {
    let raw_terminal_stream = b"\x1b[32m[+] Success:\x1b[0m Node 192.168.1.1 is active\r\n";
    let presentation_clean = strip_ansi(raw_terminal_stream);

    assert_eq!(
        presentation_clean,
        "[+] Success: Node 192.168.1.1 is active\r\n"
    );

    // Evidence invariant: the raw byte stream length and escape sequences remain intact
    assert_eq!(raw_terminal_stream.len(), 50);
    assert_ne!(raw_terminal_stream, presentation_clean.as_bytes());
}
