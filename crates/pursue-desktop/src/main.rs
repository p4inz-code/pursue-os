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
    let is_headless = args.iter().any(|a| a == "--headless");

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

    let client = RouterClient::new(Arc::new(Mutex::new(router)));

    if is_headless {
        println!("PURSUE OS Runtime — Headless Service Mode");
        println!("Configuration: {:?}", config.resolve_data_dir());
        println!("Case storage:  {:?}", case_storage);
        println!("IPC services:  case, report, terminal, browser");
        return Ok(());
    }

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
