//! PURSUE OS Desktop Shell Entry Point.

#![deny(unsafe_code)]

use std::fs;
use std::sync::{Arc, Mutex};

use pursue_browser::{BrowserHandler, BrowserService};
use pursue_case::{CaseHandler, FileCaseStore};
use pursue_desktop::{PursueDesktopApp, RouterClient};
use pursue_report::ReportHandler;
use pursue_runtime::ipc::dispatch::Router;
use pursue_terminal::{TerminalHandler, TerminalService};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let is_headless = args.iter().any(|a| a == "--headless");

    // Case and evidence repository directory
    let case_storage = std::env::temp_dir().join("pursue-cases-store");
    let _ = fs::create_dir_all(&case_storage);
    let file_store = Arc::new(Mutex::new(FileCaseStore::open(&case_storage)?));

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
    let browser_storage = std::env::temp_dir().join("pursue-browser-profiles");
    let browser_service = BrowserService::new(browser_storage);
    let browser_handler = BrowserHandler::new(browser_service).with_file_store(file_store);
    router.register(Box::new(browser_handler))?;

    let client = RouterClient::new(Arc::new(Mutex::new(router)));

    if is_headless {
        println!("PURSUE OS Desktop Shell — Headless Verification Mode");
        println!("Backend IPC services successfully registered (case, report, terminal, browser).");
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
