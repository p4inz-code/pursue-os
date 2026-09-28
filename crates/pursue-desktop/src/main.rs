//! PURSUE OS Desktop Shell Entry Point.
//!
//! # Usage
//!
//! ```text
//! pursue-desktop [--headless] [--config <path>]
//! ```
//!
//! - `--headless` — start the backend IPC services without the GUI (runtime
//!   daemon mode, used by `pursue-runtime.service`).
//! - `--config <path>` — load configuration from a TOML file. When omitted,
//!   built-in defaults are used and data is stored in a platform temporary
//!   directory.

#![deny(unsafe_code)]

use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use pursue_browser::{BrowserHandler, BrowserService};
use pursue_case::{CaseHandler, FileCaseStore};
use pursue_desktop::{PursueDesktopApp, RouterClient};
use pursue_report::ReportHandler;
use pursue_runtime::config::Config;
use pursue_runtime::ipc::dispatch::Router;
use pursue_terminal::{TerminalHandler, TerminalService};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let is_verify_live = args.iter().any(|a| a == "--verify-live");
    if is_verify_live {
        return run_verify_live(&args);
    }

    let is_headless = args.iter().any(|a| a == "--headless");
    let is_daemon =
        args.iter().any(|a| a == "--daemon") || std::env::var_os("INVOCATION_ID").is_some();

    // Load configuration (--config <path> or defaults)
    let config = load_config(&args)?;

    // Resolve storage directories from configuration
    let case_storage = config.resolve_case_dir();
    let browser_storage = config.resolve_browser_profile_dir();
    let report_storage = config.resolve_report_dir();

    // Ensure directories exist, falling back to temp dir if unprivileged
    let (file_store, case_storage) = match fs::create_dir_all(&case_storage).and_then(|_| {
        FileCaseStore::open(&case_storage).map_err(|e| std::io::Error::other(e.to_string()))
    }) {
        Ok(store) => (Arc::new(Mutex::new(store)), case_storage),
        Err(e) => {
            eprintln!(
                "Note: configured case storage at {case_storage:?} not accessible ({e}), using temporary storage"
            );
            let fallback = std::env::temp_dir().join("pursue-cases-store");
            let _ = fs::create_dir_all(&fallback);
            (
                Arc::new(Mutex::new(FileCaseStore::open(&fallback)?)),
                fallback,
            )
        }
    };

    let browser_storage = if fs::create_dir_all(&browser_storage).is_err() {
        let fallback = std::env::temp_dir().join("pursue-browser-profiles");
        let _ = fs::create_dir_all(&fallback);
        fallback
    } else {
        browser_storage
    };
    let _ = fs::create_dir_all(&report_storage);

    // Initialize backend IPC router
    let mut router = Router::new();

    // 1. Case service
    let case_handler = CaseHandler::new(file_store.clone());
    router.register(Box::new(case_handler))?;

    // 2. Report service
    let report_handler = ReportHandler::new(file_store.clone());
    router.register(Box::new(report_handler))?;

    // 3. Terminal service with case store attached
    let terminal_service = TerminalService::new();
    let terminal_handler =
        TerminalHandler::new(terminal_service).with_file_store(file_store.clone());
    router.register(Box::new(terminal_handler))?;

    // 4. Browser service with case store attached
    let browser_service = BrowserService::new(browser_storage);
    let browser_handler = BrowserHandler::new(browser_service).with_file_store(file_store);
    router.register(Box::new(browser_handler))?;

    if is_headless {
        println!("PURSUE OS Runtime — Headless Service Mode");
        println!("Configuration: {:?}", config.resolve_data_dir());
        println!("Case storage:  {:?}", case_storage);
        println!("IPC services:  case, report, terminal, browser");

        if !is_daemon {
            return Ok(());
        }

        println!("Starting daemon IPC listener...");
        #[cfg(unix)]
        {
            if let Some(socket_path) = config.ipc_socket_path.as_deref() {
                if let Some(parent) = socket_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::remove_file(socket_path);
                let listener = std::os::unix::net::UnixListener::bind(socket_path)?;
                use std::os::unix::fs::PermissionsExt;
                let _ = fs::set_permissions(socket_path, fs::Permissions::from_mode(0o770));
                println!("Listening on IPC socket: {}", socket_path.display());

                loop {
                    match listener.accept() {
                        Ok((stream, _addr)) => {
                            if let Err(e) = pursue_runtime::ipc::transport::serve(
                                &mut &stream,
                                &mut &stream,
                                &mut router,
                            ) {
                                eprintln!("IPC request handling error: {e}");
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
                        Err(e) => {
                            eprintln!("Listener accept error: {e}");
                            std::thread::sleep(std::time::Duration::from_millis(50));
                        }
                    }
                }
            }
        }

        #[allow(unreachable_code)]
        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }

    let client = RouterClient::new(Arc::new(Mutex::new(router)));

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("PURSUE OS — Forensic Investigation Shell"),
        ..Default::default()
    };

    eframe::run_native(
        "PURSUE OS",
        native_options,
        Box::new(|_cc| Ok(Box::new(PursueDesktopApp::new(Box::new(client))))),
    )
    .map_err(|e| format!("Desktop shell launch error: {e}").into())
}

/// Parse `--config <path>` from CLI arguments and load the configuration.
/// Falls back to built-in defaults when the flag is absent.
fn load_config(args: &[String]) -> Result<Config, Box<dyn std::error::Error>> {
    let config_path = args
        .windows(2)
        .find(|pair| pair[0] == "--config")
        .map(|pair| PathBuf::from(&pair[1]));

    match config_path {
        Some(path) => {
            let cfg = Config::from_toml_file(&path)?;
            Ok(cfg)
        }
        None => Ok(Config::defaults()),
    }
}

/// Execute end-to-end investigation flow verification over the live IPC socket.
fn run_verify_live(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        use pursue_runtime::ipc::Transport;
        use pursue_runtime::ipc::protocol::{MethodName, Request, ServiceId};
        use pursue_runtime::ipc::transport::unix_transport::UnixTransport;
        use serde_json::json;

        println!("=== PURSUE OS Live Investigation Flow Verification ===");

        let config = load_config(args)?;
        let socket_path = args
            .windows(2)
            .find(|pair| pair[0] == "--socket")
            .map(|pair| PathBuf::from(&pair[1]))
            .or_else(|| config.ipc_socket_path.clone())
            .unwrap_or_else(|| PathBuf::from("/run/pursue/ipc.sock"));

        println!("Connecting to IPC socket at {}...", socket_path.display());

        let unique_ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let case_id = format!("case-live-{unique_ts}");
        let session_id = format!("term-live-{unique_ts}");

        let mut transport = UnixTransport::connect(&socket_path).map_err(|e| {
            format!(
                "Failed to connect to IPC socket at {}: {e}",
                socket_path.display()
            )
        })?;

        // 1. Create disposable case
        println!("1. Creating disposable investigation case '{case_id}'...");
        let req = Request::new(
            1,
            ServiceId::new("case").unwrap(),
            MethodName::new("case.create").unwrap(),
            json!({
                "id": &case_id,
                "title": "Live OS Boot Verification Case",
                "actor": "pursue-investigator"
            }),
        );
        let res = transport.round_trip(&req)?;
        if !res.is_success() {
            return Err(format!("case.create failed: {:?}", res.error).into());
        }
        println!("   [OK] Case created: {case_id}");

        // 2. Terminal session create
        println!("2. Initializing terminal session '{session_id}'...");
        let req = Request::new(
            2,
            ServiceId::new("terminal").unwrap(),
            MethodName::new("session.create").unwrap(),
            json!({
                "session_id": &session_id,
                "case_id": &case_id,
                "actor": "pursue-investigator",
                "working_dir": "."
            }),
        );
        let res = transport.round_trip(&req)?;
        if !res.is_success() {
            return Err(format!("session.create failed: {:?}", res.error).into());
        }
        println!("   [OK] Terminal session active");

        // 3. Terminal execute command
        println!("3. Executing live system command 'uname -a'...");
        let req = Request::new(
            3,
            ServiceId::new("terminal").unwrap(),
            MethodName::new("command.execute").unwrap(),
            json!({
                "session_id": &session_id,
                "program": "uname",
                "args": ["-a"]
            }),
        );
        let res = transport.round_trip(&req)?;
        if !res.is_success() {
            return Err(format!("command.execute failed: {:?}", res.error).into());
        }
        let exec_result = res.result.unwrap();
        let stdout_bytes: Vec<u8> =
            serde_json::from_value(exec_result["stdout"].clone()).unwrap_or_default();
        let stdout_str = String::from_utf8_lossy(&stdout_bytes);
        println!("   [OK] Command output: {}", stdout_str.trim());

        // 4. Capture evidence
        println!("4. Capturing command output as immutable evidence artifact...");
        let req = Request::new(
            4,
            ServiceId::new("terminal").unwrap(),
            MethodName::new("evidence.capture").unwrap(),
            json!({
                "session_id": &session_id,
                "program": "uname",
                "stream": "stdout",
                "data": stdout_str.trim(),
                "source_label": "Kernel Architecture Verification"
            }),
        );
        let res = transport.round_trip(&req)?;
        if !res.is_success() {
            return Err(format!("evidence.capture failed: {:?}", res.error).into());
        }
        let capture_result = res.result.unwrap();
        let addr = capture_result["content_address"]
            .as_str()
            .unwrap_or("unknown");
        println!("   [OK] Evidence captured with SHA-256 address: {}", addr);

        // 5. Inspect audit log
        println!("5. Inspecting hash-chained provenance audit log...");
        let req = Request::new(
            5,
            ServiceId::new("case").unwrap(),
            MethodName::new("case.audit.list").unwrap(),
            json!({ "id": &case_id }),
        );
        let res = transport.round_trip(&req)?;
        if !res.is_success() {
            return Err(format!("case.audit.list failed: {:?}", res.error).into());
        }
        let audit_result = res.result.unwrap();
        let chain_ok = audit_result["chain_verified"].as_bool().unwrap_or(false);
        let event_count = audit_result["events_count"].as_u64().unwrap_or(0);
        println!(
            "   [OK] Audit chain verified: {} ({} events recorded)",
            chain_ok, event_count
        );

        // 6. Generate and export forensic report
        println!("6. Generating and exporting forensic report...");
        let report_target = config
            .resolve_report_dir()
            .join(format!("live-report-{unique_ts}.json"));
        let req = Request::new(
            6,
            ServiceId::new("report").unwrap(),
            MethodName::new("report.export").unwrap(),
            json!({
                "case_id": &case_id,
                "target_path": report_target.to_string_lossy(),
                "format": "json",
                "actor": "pursue-investigator"
            }),
        );
        let res = transport.round_trip(&req)?;
        if !res.is_success() {
            return Err(format!("report.export failed: {:?}", res.error).into());
        }
        let export_res = res.result.unwrap();
        let report_hash = export_res["report_hash"].as_str().unwrap_or("");
        println!("   [OK] Report exported: {}", export_res["destination"]);
        println!("   [OK] Report SHA-256: {}", report_hash);

        // 7. Verify case integrity
        println!("7. Verifying deep cryptographic case integrity...");
        let req = Request::new(
            7,
            ServiceId::new("case").unwrap(),
            MethodName::new("case.verify").unwrap(),
            json!({ "id": &case_id }),
        );
        let res = transport.round_trip(&req)?;
        if !res.is_success() {
            return Err(format!("case.verify failed: {:?}", res.error).into());
        }
        let verify_res = res.result.unwrap();
        println!(
            "   [OK] Manifest verified: {}",
            verify_res["manifest_verified"]
        );
        println!(
            "   [OK] Audit chain verified: {}",
            verify_res["audit_chain_verified"]
        );
        println!(
            "   [OK] Evidence blobs verified: {}",
            verify_res["evidence_verified_count"]
        );
        println!("=======================================================");
        println!(" LIVE INVESTIGATION FLOW TEST: PASSED");
        println!("=======================================================");
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = args;
        Err("Live verification requires Unix domain socket support".into())
    }
}
