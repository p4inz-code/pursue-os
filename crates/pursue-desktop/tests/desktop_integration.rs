//! Comprehensive integration tests for `pursue-desktop` (Phase 2, Phase 5 OS Integration)
//! and `build/config` (Phase 1B, Phase 5, Phase 6).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use pursue_browser::engine::{MockBrowserEngine, MockResponse};
use pursue_browser::{BrowserHandler, BrowserService};
use pursue_case::{CaseStore, FileCaseStore};
use pursue_desktop::{DesktopState, DesktopTab, PursueDesktopApp, RouterClient};
use pursue_runtime::config::Config;
use pursue_runtime::ipc::dispatch::Router;
use pursue_terminal::executor::{MockExecutor, MockOutcome};
use pursue_terminal::{TerminalHandler, TerminalService};

struct TestDir(PathBuf);

impl TestDir {
    fn new(name: &str) -> Self {
        let p =
            std::env::temp_dir().join(format!("pursue-desk-test-{}-{}", name, std::process::id()));
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

type TestRouterBundle = (
    Arc<Mutex<Router>>,
    Arc<Mutex<FileCaseStore>>,
    Arc<MockExecutor>,
    Arc<MockBrowserEngine>,
);

fn setup_test_router(temp_dir: &Path) -> TestRouterBundle {
    let mut router = Router::new();

    // Setup isolated FileCaseStore
    let store_dir = temp_dir.join("cases");
    fs::create_dir_all(&store_dir).unwrap();
    let file_store = Arc::new(Mutex::new(FileCaseStore::open(&store_dir).unwrap()));

    // Setup terminal service + mock executor + IPC handler
    let terminal_service = TerminalService::new();
    let mock_executor = Arc::new(MockExecutor::new());
    let terminal_handler = TerminalHandler::with_executor(terminal_service, mock_executor.clone())
        .with_file_store(file_store.clone());
    router.register(Box::new(terminal_handler)).unwrap();

    // Setup browser service + mock engine + IPC handler
    let browser_profiles = temp_dir.join("browser-profiles");
    fs::create_dir_all(&browser_profiles).unwrap();
    let browser_service = BrowserService::new(browser_profiles);
    let mock_engine = Arc::new(MockBrowserEngine::new());
    mock_engine.set_tor_available(true);
    let browser_handler = BrowserHandler::with_engine(browser_service, mock_engine.clone())
        .with_file_store(file_store.clone());
    router.register(Box::new(browser_handler)).unwrap();

    // Setup case IPC handler
    let case_handler = pursue_case::CaseHandler::new(file_store.clone());
    router.register(Box::new(case_handler)).unwrap();

    // Setup report IPC handler
    let report_handler = pursue_report::ReportHandler::new(file_store.clone());
    router.register(Box::new(report_handler)).unwrap();

    let shared_router = Arc::new(Mutex::new(router));
    (shared_router, file_store, mock_executor, mock_engine)
}

#[test]
fn test_desktop_state_initialization() {
    let state = DesktopState::new();
    assert_eq!(state.active_tab, DesktopTab::Dashboard);
    assert_eq!(state.investigator_id, "investigator-01");
    assert!(state.active_case.is_none());
    assert!(state.cases.is_empty());
    assert_eq!(state.browser_mode, "tor");
    assert!(!state.is_error);
    assert!(state.status_message.is_some());
}

#[test]
fn test_desktop_app_tab_switching() {
    let temp = TestDir::new("tab-switch");
    let (router, _, _, _) = setup_test_router(temp.path());
    let client = Box::new(RouterClient::new(router));

    let mut app = PursueDesktopApp::new(client);
    assert_eq!(app.state.active_tab, DesktopTab::Dashboard);

    app.state.active_tab = DesktopTab::Cases;
    assert_eq!(app.state.active_tab, DesktopTab::Cases);

    app.state.active_tab = DesktopTab::Terminal;
    assert_eq!(app.state.active_tab, DesktopTab::Terminal);

    app.state.active_tab = DesktopTab::Browser;
    assert_eq!(app.state.active_tab, DesktopTab::Browser);

    app.state.active_tab = DesktopTab::Reports;
    assert_eq!(app.state.active_tab, DesktopTab::Reports);

    app.state.active_tab = DesktopTab::Settings;
    assert_eq!(app.state.active_tab, DesktopTab::Settings);
}

#[test]
fn test_desktop_ipc_terminal_execution_and_evidence_flow() {
    let temp = TestDir::new("term-flow");
    let (router, file_store, mock_exec, _) = setup_test_router(temp.path());
    let client = RouterClient::new(router);

    // Create a case directly in file store for evidence attachment
    let case_id = pursue_case::CaseId::new("CASE-DESK-01").unwrap();
    file_store
        .lock()
        .unwrap()
        .create_case(
            case_id.clone(),
            "Desktop Investigation Case",
            "investigator-01",
        )
        .unwrap();

    // 1. Establish Terminal Session via IPC
    let create_res = client
        .create_terminal_session("term-sess-01", "CASE-DESK-01", "investigator-01")
        .unwrap();
    assert_eq!(create_res["session_id"], "term-sess-01");
    assert_eq!(create_res["status"], "active");

    // 2. Program mock executor response
    mock_exec.register(
        "whois",
        MockOutcome::Success {
            exit_code: Some(0),
            stdout: b"Domain Name: EXAMPLE.COM\nRegistrar: Example Registrar\n".to_vec(),
            stderr: Vec::new(),
            timed_out: false,
            truncated: false,
        },
    );

    // 3. Execute command over IPC
    let exec_res = client
        .execute_terminal_command("term-sess-01", "whois", &["example.com".to_string()])
        .unwrap();

    assert_eq!(exec_res["exit_code"], 0);
    assert_eq!(exec_res["timed_out"], false);
    assert_eq!(exec_res["truncated"], false);

    let stdout_bytes: Vec<u8> = exec_res["stdout"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_u64().unwrap() as u8)
        .collect();
    let stdout_str = String::from_utf8(stdout_bytes).unwrap();
    assert!(stdout_str.contains("Domain Name: EXAMPLE.COM"));

    // 4. Ingest stdout as tamper-evident forensic evidence over IPC
    let cap_res = client
        .capture_terminal_evidence("term-sess-01", exec_res, "stdout")
        .unwrap();

    assert!(cap_res["content_address"].is_string());
    assert!(cap_res["size"].is_number());

    // 5. Verify the case in the store now has the evidence attached
    let loaded_case = file_store.lock().unwrap().load_case(&case_id).unwrap();
    assert_eq!(loaded_case.evidence_addresses().count(), 1);
    assert_eq!(loaded_case.audit_log().len(), 2); // 1 create + 1 attach
}

#[test]
fn test_desktop_ipc_browser_navigation_and_evidence_flow() {
    let temp = TestDir::new("browser-flow");
    let (router, file_store, _, mock_engine) = setup_test_router(temp.path());
    let client = RouterClient::new(router);

    // Create a case directly in file store for evidence attachment
    let case_id = pursue_case::CaseId::new("CASE-DESK-02").unwrap();
    file_store
        .lock()
        .unwrap()
        .create_case(
            case_id.clone(),
            "Web Tor Investigation Case",
            "investigator-01",
        )
        .unwrap();

    // 1. Establish Browser Session in Tor mode via IPC
    let create_res = client
        .create_browser_session("browser-sess-01", "CASE-DESK-02", "investigator-01", "tor")
        .unwrap();
    assert_eq!(create_res["id"], "browser-sess-01");
    assert_eq!(create_res["mode"], "tor");
    assert_eq!(create_res["status"], "active");

    // 2. Program mock HTTP/Tor response
    let target_url = "http://duckduckgogg42xjoc72x3sjasowoarfbgcmvfimaftt6twagswzczad.onion";
    let mut headers = BTreeMap::new();
    headers.insert("server".to_string(), "nginx".to_string());
    headers.insert("content-type".to_string(), "text/html".to_string());

    mock_engine.register(
        target_url,
        MockResponse::Success {
            status_code: 200,
            headers,
            body: b"<html><head><title>DuckDuckGo Onion</title></head><body>Search</body></html>"
                .to_vec(),
            final_url: None,
        },
    );

    // 3. Navigate over IPC
    let nav_res = client
        .navigate_browser("browser-sess-01", target_url)
        .unwrap();

    assert_eq!(nav_res["status_code"], 200);
    assert_eq!(nav_res["routing_mode"], "tor");

    // 4. Ingest HTML page as forensic evidence over IPC
    let cap_res = client
        .capture_browser_evidence("browser-sess-01", nav_res, "page_content")
        .unwrap();

    assert!(cap_res["address"].is_string());
    assert_eq!(cap_res["artifact_kind"], "page_content");

    // 5. Verify the case in the store has the evidence attached
    let loaded_case = file_store.lock().unwrap().load_case(&case_id).unwrap();
    assert_eq!(loaded_case.evidence_addresses().count(), 1);
    assert_eq!(loaded_case.audit_log().len(), 2);
}

#[test]
fn test_bootable_base_config_validity() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let config_path = repo_root.join("build/config/pursue-config.toml");
    assert!(
        config_path.exists(),
        "build/config/pursue-config.toml must exist"
    );

    let toml_str =
        fs::read_to_string(&config_path).expect("should read build/config/pursue-config.toml");

    let cfg = Config::from_toml_str(&toml_str)
        .expect("pursue-config.toml must parse successfully as valid Config");

    assert_eq!(cfg.version, 1);
    assert_eq!(cfg.log_level, pursue_runtime::log::Level::Info);
    assert_eq!(cfg.data_dir, Some(PathBuf::from("/var/lib/pursue")));
    assert_eq!(
        cfg.ipc_socket_path,
        Some(PathBuf::from("/run/pursue/ipc.sock"))
    );
    // Phase 5: deployment path fields
    assert_eq!(cfg.case_dir, Some(PathBuf::from("/var/lib/pursue/cases")));
    assert_eq!(
        cfg.browser_profile_dir,
        Some(PathBuf::from("/var/lib/pursue/profiles"))
    );
    assert_eq!(
        cfg.report_dir,
        Some(PathBuf::from("/var/lib/pursue/reports"))
    );
    assert_eq!(
        cfg.log_file,
        Some(PathBuf::from("/var/log/pursue/runtime.log"))
    );

    // Resolvers return the explicit values when set
    assert_eq!(
        cfg.resolve_case_dir(),
        PathBuf::from("/var/lib/pursue/cases")
    );
    assert_eq!(
        cfg.resolve_browser_profile_dir(),
        PathBuf::from("/var/lib/pursue/profiles")
    );
    assert_eq!(
        cfg.resolve_report_dir(),
        PathBuf::from("/var/lib/pursue/reports")
    );
    assert_eq!(
        cfg.resolve_log_file(),
        PathBuf::from("/var/log/pursue/runtime.log")
    );
}

#[test]
fn test_desktop_ipc_case_full_lifecycle_and_verification() {
    let temp = TestDir::new("case-lifecycle");
    let (router, _, _, _) = setup_test_router(temp.path());
    let client = RouterClient::new(router);

    // 1. Create case over IPC
    let create_res = client
        .create_case("CASE-INT-01", "Full Lifecycle Test", "investigator-01")
        .unwrap();
    assert_eq!(create_res["id"], "CASE-INT-01");
    assert_eq!(create_res["status"], "open");

    // 2. Get case over IPC
    let get_res = client.get_case("CASE-INT-01").unwrap();
    assert_eq!(get_res["title"], "Full Lifecycle Test");
    assert_eq!(get_res["evidence_count"], 0);

    // 3. Update title and notes over IPC
    let title_res = client
        .update_case_title("CASE-INT-01", "Updated Lifecycle Title", "investigator-01")
        .unwrap();
    assert_eq!(title_res["title"], "Updated Lifecycle Title");

    let notes_res = client
        .update_case_notes("CASE-INT-01", "Lead details recorded.", "investigator-01")
        .unwrap();
    assert_eq!(notes_res["notes"], "Lead details recorded.");

    // 4. Close and Reopen over IPC
    let close_res = client.close_case("CASE-INT-01", "investigator-01").unwrap();
    assert_eq!(close_res["status"], "closed");

    let reopen_res = client
        .reopen_case("CASE-INT-01", "investigator-01")
        .unwrap();
    assert_eq!(reopen_res["status"], "open");

    // 5. Deep verify over IPC
    let verify_res = client.verify_case("CASE-INT-01").unwrap();
    assert_eq!(verify_res["verified"], true);
    assert_eq!(verify_res["manifest_verified"], true);
    assert_eq!(verify_res["audit_chain_verified"], true);

    // 6. Inspect audit list over IPC
    let audit_res = client.list_case_audit("CASE-INT-01").unwrap();
    assert_eq!(audit_res["chain_verified"], true);
    // Events: created, title_updated, notes_updated, closed, reopened
    assert_eq!(audit_res["events_count"], 5);

    // 7. List cases over IPC
    let list_res = client.list_cases().unwrap();
    let arr = list_res.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["id"], "CASE-INT-01");
}

#[test]
fn test_cross_case_isolation_and_evidence_partitioning() {
    let temp = TestDir::new("case-isolation");
    let (router, file_store, mock_exec, mock_engine) = setup_test_router(temp.path());
    let client = RouterClient::new(router);

    // Create Case A and Case B
    client
        .create_case("CASE-ISO-A", "Case A Target", "analyst-a")
        .unwrap();
    client
        .create_case("CASE-ISO-B", "Case B Target", "analyst-b")
        .unwrap();

    // 1. Execute terminal command in Case A session and capture
    client
        .create_terminal_session("sess-a", "CASE-ISO-A", "analyst-a")
        .unwrap();
    mock_exec.register(
        "nmap",
        MockOutcome::Success {
            exit_code: Some(0),
            stdout: b"Port 80 open on host A".to_vec(),
            stderr: Vec::new(),
            timed_out: false,
            truncated: false,
        },
    );
    let exec_res = client
        .execute_terminal_command("sess-a", "nmap", &[])
        .unwrap();
    let cap_a = client
        .capture_terminal_evidence("sess-a", exec_res, "stdout")
        .unwrap();
    let addr_a = cap_a["content_address"].as_str().unwrap().to_string();

    // 2. Navigate browser in Case B session and capture
    client
        .create_browser_session("sess-b", "CASE-ISO-B", "analyst-b", "tor")
        .unwrap();
    let target_b = "http://target-b-service.onion";
    mock_engine.register(
        target_b,
        MockResponse::Success {
            status_code: 200,
            headers: BTreeMap::new(),
            body: b"Host B Portal".to_vec(),
            final_url: None,
        },
    );
    let nav_res = client.navigate_browser("sess-b", target_b).unwrap();
    let cap_b = client
        .capture_browser_evidence("sess-b", nav_res, "page_content")
        .unwrap();
    let addr_b = cap_b["address"].as_str().unwrap().to_string();

    // Verify Case A has only evidence A
    let ev_list_a = client.list_case_evidence("CASE-ISO-A").unwrap();
    let addrs_a: Vec<&str> = ev_list_a
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["address"].as_str().unwrap())
        .collect();
    assert_eq!(addrs_a.len(), 1);
    assert_eq!(addrs_a[0], addr_a);
    assert!(!addrs_a.contains(&addr_b.as_str()));

    // Verify Case B has only evidence B
    let ev_list_b = client.list_case_evidence("CASE-ISO-B").unwrap();
    let addrs_b: Vec<&str> = ev_list_b
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["address"].as_str().unwrap())
        .collect();
    assert_eq!(addrs_b.len(), 1);
    assert_eq!(addrs_b[0], addr_b);
    assert!(!addrs_b.contains(&addr_a.as_str()));

    // Verify independent audit chains
    let store = file_store.lock().unwrap();
    let case_a_loaded = store
        .load_case(&pursue_case::CaseId::new("CASE-ISO-A").unwrap())
        .unwrap();
    let case_b_loaded = store
        .load_case(&pursue_case::CaseId::new("CASE-ISO-B").unwrap())
        .unwrap();
    assert!(case_a_loaded.audit_log().verify().is_ok());
    assert!(case_b_loaded.audit_log().verify().is_ok());
}

#[test]
fn test_desktop_ipc_reporting_workflow_and_verification() {
    let temp = TestDir::new("report-workflow");
    let (router, _file_store, mock_exec, _) = setup_test_router(temp.path());
    let client = RouterClient::new(router);

    // Setup Case with Evidence
    client
        .create_case("CASE-REP-TEST", "Operation Reporting", "investigator-01")
        .unwrap();
    client
        .update_case_notes(
            "CASE-REP-TEST",
            "Critical forensic findings.",
            "investigator-01",
        )
        .unwrap();

    client
        .create_terminal_session("term-rep", "CASE-REP-TEST", "investigator-01")
        .unwrap();
    mock_exec.register(
        "exiftool",
        MockOutcome::Success {
            exit_code: Some(0),
            stdout: b"Camera Model: SuperCam 5000\nGPS: 48.8584 N, 2.2945 E\n".to_vec(),
            stderr: Vec::new(),
            timed_out: false,
            truncated: false,
        },
    );
    let exec_res = client
        .execute_terminal_command("term-rep", "exiftool", &[])
        .unwrap();
    let _ = client
        .capture_terminal_evidence("term-rep", exec_res, "stdout")
        .unwrap();

    // 1. Preview report metadata
    let preview = client.preview_report_metadata("CASE-REP-TEST").unwrap();
    assert_eq!(preview["case_id"], "CASE-REP-TEST");
    assert_eq!(preview["evidence_count"], 1);
    assert_eq!(preview["audit_chain_verified"], true);

    // 2. Generate in-memory JSON report
    let rep_json = client
        .generate_report("CASE-REP-TEST", "investigator-01", "json")
        .unwrap();
    assert_eq!(rep_json["format"], "json");
    let hash_json = rep_json["report_hash"].as_str().unwrap();
    assert!(!hash_json.is_empty());
    assert!(
        rep_json["content"]
            .as_str()
            .unwrap()
            .contains("SuperCam 5000")
    );

    // 3. Generate in-memory HTML report
    let rep_html = client
        .generate_report("CASE-REP-TEST", "investigator-01", "html")
        .unwrap();
    assert_eq!(rep_html["format"], "html");
    let html_content = rep_html["content"].as_str().unwrap();
    assert!(html_content.contains("<!DOCTYPE html>"));
    assert!(html_content.contains("Cryptographic Integrity Seal"));
    assert!(html_content.contains("SuperCam 5000"));
    assert!(!html_content.contains("<script"));

    // 4. Safe Export to Disk (JSON)
    let export_json_path = temp.path().join("exports/report.json");
    let exp_json_res = client
        .export_report(
            "CASE-REP-TEST",
            "investigator-01",
            "json",
            export_json_path.to_str().unwrap(),
        )
        .unwrap();
    assert_eq!(exp_json_res["verified"], true);
    assert!(export_json_path.exists());

    // 5. Verify exported report on disk
    let ver_res = client
        .verify_report(export_json_path.to_str().unwrap())
        .unwrap();
    assert_eq!(ver_res["verified"], true);
    assert_eq!(ver_res["report_hash"].as_str().unwrap(), hash_json);

    // 6. Safe Export to Disk (HTML)
    let export_html_path = temp.path().join("exports/report.html");
    let exp_html_res = client
        .export_report(
            "CASE-REP-TEST",
            "investigator-01",
            "html",
            export_html_path.to_str().unwrap(),
        )
        .unwrap();
    assert_eq!(exp_html_res["verified"], true);
    assert!(export_html_path.exists());

    // 7. Verify exported HTML report on disk
    let ver_html_res = client
        .verify_report(export_html_path.to_str().unwrap())
        .unwrap();
    assert_eq!(ver_html_res["verified"], true);

    // 8. Tamper detection test
    let tampered_path = temp.path().join("exports/report_tampered.json");
    fs::copy(&export_json_path, &tampered_path).unwrap();
    let original_json = fs::read_to_string(&tampered_path).unwrap();
    let altered_json = original_json.replace("SuperCam 5000", "TamperedCamera 9999");
    fs::write(&tampered_path, altered_json).unwrap();

    let tamper_check = client.verify_report(tampered_path.to_str().unwrap());
    assert!(
        tamper_check.is_err(),
        "Tampered report file must fail cryptographic verification"
    );
}

#[test]
fn test_report_export_path_traversal_rejection() {
    let temp = TestDir::new("export-traversal");
    let (router, _, _, _) = setup_test_router(temp.path());
    let client = RouterClient::new(router);

    client
        .create_case("CASE-TRAV", "Traversal Case", "analyst")
        .unwrap();

    let evil_path = "../../etc/shadow";
    let res = client.export_report("CASE-TRAV", "analyst", "json", evil_path);
    assert!(
        res.is_err(),
        "Path traversal target must be strictly rejected"
    );
}

#[test]
fn test_tor_fail_closed_behavior_maintained() {
    let temp = TestDir::new("tor-fail-closed");
    let (router, _, _, mock_engine) = setup_test_router(temp.path());
    let client = RouterClient::new(router);

    client
        .create_case("CASE-TOR", "Tor Security Case", "analyst")
        .unwrap();
    client
        .create_browser_session("sess-tor-down", "CASE-TOR", "analyst", "tor")
        .unwrap();

    // Disable Tor availability in mock engine
    mock_engine.set_tor_available(false);

    let nav_res = client.navigate_browser("sess-tor-down", "http://any-onion-site.onion");
    assert!(
        nav_res.is_err(),
        "Tor session MUST fail closed when Tor daemon is unavailable"
    );
    let err_str = nav_res.err().unwrap().to_string();
    assert!(
        err_str.contains("Tor") || err_str.contains("unavailable") || err_str.contains("IPC Error"),
        "Error message should reflect Tor failure: {err_str}"
    );
}

#[test]
fn test_empty_case_and_error_resiliency() {
    let temp = TestDir::new("error-resiliency");
    let (router, _, _, _) = setup_test_router(temp.path());
    let client = RouterClient::new(router);

    // 1. Non-existent case query
    let get_missing = client.get_case("NONEXISTENT-CASE");
    assert!(get_missing.is_err());

    // 2. Create valid empty case
    client
        .create_case("CASE-EMPTY", "Empty Case", "analyst")
        .unwrap();

    // Report generation on empty case succeeds with clean empty inventory
    let rep_res = client
        .generate_report("CASE-EMPTY", "analyst", "json")
        .unwrap();
    assert_eq!(rep_res["format"], "json");

    // Reopening an already open case fails cleanly
    let reopen_open = client.reopen_case("CASE-EMPTY", "analyst");
    assert!(reopen_open.is_err());

    // Closing case succeeds
    let close_res = client.close_case("CASE-EMPTY", "analyst");
    assert!(close_res.is_ok());

    // Closing an already closed case fails cleanly
    let close_closed = client.close_case("CASE-EMPTY", "analyst");
    assert!(close_closed.is_err());
}

// ===========================================================================
// Phase 5: OS Integration Tests
// ===========================================================================

#[test]
fn test_phase5_systemd_runtime_unit_references_pursue_desktop() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let unit_path = repo_root.join("build/systemd/pursue-runtime.service");
    assert!(unit_path.exists(), "pursue-runtime.service must exist");

    let content = fs::read_to_string(&unit_path).unwrap();

    // Decision A-011: single binary model — ExecStart must reference pursue-desktop
    assert!(
        content.contains("pursue-desktop"),
        "pursue-runtime.service ExecStart must reference pursue-desktop binary (A-011)"
    );
    assert!(
        content.contains("--headless"),
        "pursue-runtime.service must run in headless mode"
    );
    assert!(
        content.contains("--config"),
        "pursue-runtime.service must accept a config path"
    );

    // Security hardening
    assert!(content.contains("ProtectSystem=strict"));
    assert!(content.contains("NoNewPrivileges=true"));
    assert!(content.contains("ProtectHome=true"));
    assert!(content.contains("PrivateTmp=true"));
    assert!(content.contains("MemoryDenyWriteExecute=true"));
}

#[test]
fn test_phase5_no_spurious_service_binaries() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    // Decision A-011: terminal and browser run in-process, not as separate binaries.
    // Their systemd units must NOT exist.
    assert!(
        !repo_root
            .join("build/systemd/pursue-terminal.service")
            .exists(),
        "pursue-terminal.service must not exist (A-011: in-process handler)"
    );
    assert!(
        !repo_root
            .join("build/systemd/pursue-browser.service")
            .exists(),
        "pursue-browser.service must not exist (A-011: in-process handler)"
    );
}

#[test]
fn test_phase5_desktop_service_unit_exists() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let unit_path = repo_root.join("build/systemd/pursue-desktop.service");
    assert!(unit_path.exists(), "pursue-desktop.service must exist");

    let content = fs::read_to_string(&unit_path).unwrap();

    // Desktop runs as pursue-investigator (A-008)
    assert!(
        content.contains("User=pursue-investigator"),
        "Desktop must run as pursue-investigator"
    );
    // Depends on runtime
    assert!(
        content.contains("Requires=pursue-runtime.service"),
        "Desktop must depend on pursue-runtime.service"
    );
    // Security: zero direct access (A-007)
    assert!(content.contains("NoNewPrivileges=true"));
}

#[test]
fn test_phase5_sysusers_defines_both_accounts() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let sysusers_path = repo_root.join("build/config/sysusers.d-pursue.conf");
    assert!(sysusers_path.exists(), "sysusers config must exist");

    let content = fs::read_to_string(&sysusers_path).unwrap();

    // Decision A-008: two-user model
    assert!(
        content.contains("u pursue "),
        "sysusers must define 'pursue' system daemon user"
    );
    assert!(
        content.contains("u pursue-investigator"),
        "sysusers must define 'pursue-investigator' interactive user"
    );
    assert!(
        content.contains("/usr/sbin/nologin"),
        "'pursue' user must have nologin shell"
    );
    assert!(
        content.contains("/bin/bash"),
        "'pursue-investigator' must have bash shell"
    );
}

#[test]
fn test_phase5_tmpfiles_covers_all_storage_directories() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let tmpfiles_path = repo_root.join("build/config/tmpfiles.d-pursue.conf");
    assert!(tmpfiles_path.exists(), "tmpfiles config must exist");

    let content = fs::read_to_string(&tmpfiles_path).unwrap();

    // All deployment directories must be declared
    let required_dirs = [
        "/run/pursue",
        "/var/lib/pursue",
        "/var/lib/pursue/cases",
        "/var/lib/pursue/profiles",
        "/var/lib/pursue/reports",
        "/var/log/pursue",
    ];
    for dir in &required_dirs {
        assert!(
            content.contains(dir),
            "tmpfiles must declare directory: {dir}"
        );
    }
}

#[test]
fn test_phase5_packages_list_includes_sway() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let packages_path = repo_root.join("build/debian/packages.list");
    assert!(packages_path.exists(), "packages.list must exist");

    let content = fs::read_to_string(&packages_path).unwrap();
    let lines: Vec<&str> = content.lines().collect();

    // Decision A-002: Wayland-first desktop (Sway compositor)
    assert!(
        lines.iter().any(|l| l.trim() == "sway"),
        "packages.list must include sway (Decision A-002)"
    );
    // Tor must be present for fail-closed (A-006)
    assert!(
        lines.iter().any(|l| l.trim() == "tor"),
        "packages.list must include tor (Decision A-006)"
    );
}

#[test]
fn test_phase5_config_driven_service_initialization() {
    let temp = TestDir::new("config-init");
    let (router, _, _, _) = setup_test_router(temp.path());
    let client = RouterClient::new(router);

    // Verify all four in-process services are registered and responsive
    // Case service
    let case_res = client.create_case("CASE-CFG-01", "Config Test", "investigator-01");
    assert!(case_res.is_ok(), "Case service must be registered");

    // Terminal service
    let term_res = client.create_terminal_session("ts-01", "CASE-CFG-01", "investigator-01");
    assert!(term_res.is_ok(), "Terminal service must be registered");

    // Browser service
    let browser_res =
        client.create_browser_session("bs-01", "CASE-CFG-01", "investigator-01", "tor");
    assert!(browser_res.is_ok(), "Browser service must be registered");

    // Report service
    let report_res = client.generate_report("CASE-CFG-01", "investigator-01", "json");
    assert!(report_res.is_ok(), "Report service must be registered");
}

#[test]
fn test_phase5_preset_file_consistency() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let preset_path = repo_root.join("build/systemd/pursue.preset");
    assert!(preset_path.exists(), "pursue.preset must exist");

    let content = fs::read_to_string(&preset_path).unwrap();

    // Must enable runtime and desktop
    assert!(content.contains("enable pursue-runtime.service"));
    assert!(content.contains("enable pursue-desktop.service"));
    assert!(content.contains("enable tor.service"));

    // Must NOT enable non-existent terminal/browser services
    assert!(
        !content.contains("pursue-terminal.service"),
        "preset must not reference non-existent pursue-terminal.service"
    );
    assert!(
        !content.contains("pursue-browser.service"),
        "preset must not reference non-existent pursue-browser.service"
    );
}
