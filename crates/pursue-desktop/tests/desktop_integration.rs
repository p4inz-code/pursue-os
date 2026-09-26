//! Comprehensive integration tests for `pursue-desktop` (Phase 2) and `build/config` (Phase 1B).

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

    let shared_router = Arc::new(Mutex::new(router));
    (shared_router, file_store, mock_executor, mock_engine)
}

#[test]
fn test_desktop_state_initialization() {
    let state = DesktopState::new();
    assert_eq!(state.active_tab, DesktopTab::Cases);
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
    assert_eq!(app.state.active_tab, DesktopTab::Cases);

    app.state.active_tab = DesktopTab::Terminal;
    assert_eq!(app.state.active_tab, DesktopTab::Terminal);

    app.state.active_tab = DesktopTab::Browser;
    assert_eq!(app.state.active_tab, DesktopTab::Browser);

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
}
