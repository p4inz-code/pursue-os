//! Main desktop application shell implementing `eframe::App`.

use egui::{Color32, RichText};

use crate::client::IpcClient;
use crate::state::{DesktopState, DesktopTab};
use crate::views::{
    audit_view, browser_view, case_view, dashboard_view, evidence_view, report_view, status_bar,
    terminal_view, timeline_view,
};

/// The PURSUE OS primary desktop shell application.
pub struct PursueDesktopApp {
    /// Active IPC communication client.
    pub client: Box<dyn IpcClient>,
    /// Reactive desktop UI state.
    pub state: DesktopState,
}

impl PursueDesktopApp {
    /// Creates a new desktop application instance with the given IPC client.
    pub fn new(client: Box<dyn IpcClient>) -> Self {
        let mut state = DesktopState::new();
        if let Ok(user) = std::env::var("USER").or_else(|_| std::env::var("LOGNAME")) {
            if !user.trim().is_empty() {
                state.investigator_id = user.trim().to_string();
            }
        }
        Self { client, state }
    }

    /// Renders the top navigation header bar and tab selector.
    fn render_header(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(
                RichText::new("PURSUE OS")
                    .color(Color32::from_rgb(100, 200, 255))
                    .strong(),
            );
            ui.label(
                RichText::new("Forensic Investigation Shell")
                    .italics()
                    .color(Color32::from_rgb(180, 180, 200)),
            );

            ui.add_space(16.0);

            // Tab navigation buttons
            let tabs = [
                (DesktopTab::Dashboard, "Dashboard"),
                (DesktopTab::Cases, "Cases"),
                (DesktopTab::Evidence, "Evidence"),
                (DesktopTab::Timeline, "Timeline"),
                (DesktopTab::Audit, "Audit Trail"),
                (DesktopTab::Terminal, "Terminal"),
                (DesktopTab::Browser, "Browser & Tor"),
                (DesktopTab::Reports, "Reports"),
                (DesktopTab::Settings, "Settings & IPC"),
            ];

            for (tab, label) in tabs {
                let is_selected = self.state.active_tab == tab;
                let text = if is_selected {
                    RichText::new(label).strong().color(Color32::WHITE)
                } else {
                    RichText::new(label).color(Color32::LIGHT_GRAY)
                };

                if ui.button(text).clicked() {
                    self.state.active_tab = tab;
                    self.state.clear_status();
                }
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    RichText::new(format!("Analyst: {}", self.state.investigator_id))
                        .color(Color32::from_rgb(140, 220, 140)),
                );
            });
        });
    }

    /// Renders the System & IPC diagnostics settings tab.
    fn render_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("System & IPC Service Diagnostics");
        ui.add_space(8.0);

        ui.group(|ui| {
            ui.label(RichText::new("Security Boundary Contracts").strong());
            ui.label("• Tor Fail-Closed Boundary: ENFORCED (Direct fallback strictly forbidden)");
            ui.label("• DNS Privacy: Remote SOCKS5h Tor DNS Resolution Enforced");
            ui.label("• Chain-of-Custody: SHA-256 Content Hashing + Hash-Chained Audit Log");
            ui.label("• Path Safety: Strict Path Traversal Prevention on Export & Sessions");
            ui.label("• IPC Security: Local Domain Socket Dispatch, privilege separation");
            ui.label("• UI Direct Access: FORBIDDEN — all actions mediated through IPC Router");
        });

        ui.add_space(12.0);

        ui.group(|ui| {
            ui.label(RichText::new("Investigator Identity").strong());
            ui.horizontal(|ui| {
                ui.label("Current Actor ID:");
                ui.text_edit_singleline(&mut self.state.investigator_id);
            });
        });

        ui.add_space(12.0);

        ui.group(|ui| {
            ui.label(RichText::new("IPC Connection & Subsystem Health").strong());
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Active Endpoint:");
                ui.label(RichText::new(&self.state.ipc_endpoint_info).monospace().strong());
                let (status_text, status_color) = if self.state.ipc_online {
                    ("ONLINE", Color32::from_rgb(80, 220, 120))
                } else {
                    ("OFFLINE", Color32::from_rgb(255, 80, 80))
                };
                ui.label(RichText::new(format!("[{status_text}]")).color(status_color).strong());
            });

            ui.add_space(6.0);
            if ui.button("Run IPC Service Health Check").clicked() {
                let case_probe = self.client.call("case", "case.list", serde_json::json!({}));
                let report_probe = self.client.call("report", "report.preview_metadata", serde_json::json!({ "case_id": "probe" }));
                let term_probe = self.client.call("terminal", "session.get", serde_json::json!({ "session_id": "probe" }));
                let browser_probe = self.client.call("browser", "browser.session.get", serde_json::json!({ "session_id": "probe" }));

                let case_ok = case_probe.is_ok();
                let report_ok = match &report_probe {
                    Err(e) => !e.to_string().contains("service not found"),
                    Ok(_) => true,
                };
                let term_ok = match &term_probe {
                    Err(e) => !e.to_string().contains("service not found"),
                    Ok(_) => true,
                };
                let browser_ok = match &browser_probe {
                    Err(e) => !e.to_string().contains("service not found"),
                    Ok(_) => true,
                };

                let all_ok = case_ok && report_ok && term_ok && browser_ok;
                self.state.ipc_online = all_ok;

                if all_ok {
                    self.state
                        .set_info("All backend IPC services (case, report, terminal, browser) verified responsive.");
                } else {
                    self.state.set_error(format!(
                        "IPC Diagnostics: case={case_ok}, report={report_ok}, terminal={term_ok}, browser={browser_ok}"
                    ));
                }
            }
        });
    }
}

impl eframe::App for PursueDesktopApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // Top Header Navigation Bar
        egui::Panel::top("top_panel").show(ui, |ui| {
            ui.add_space(4.0);
            self.render_header(ui);
            ui.add_space(4.0);
        });

        // Bottom Forensic Status Bar
        egui::Panel::bottom("bottom_panel").show(ui, |ui| {
            ui.add_space(4.0);
            status_bar::render(ui, &self.state, &*self.client);
            ui.add_space(4.0);
        });

        // Central Investigation Workspace
        egui::CentralPanel::default().show(ui, |ui| match self.state.active_tab {
            DesktopTab::Dashboard => {
                dashboard_view::render(ui, &mut self.state, &*self.client);
            }
            DesktopTab::Cases => {
                case_view::render(ui, &mut self.state, &*self.client);
            }
            DesktopTab::Evidence => {
                evidence_view::render(ui, &mut self.state, &*self.client);
            }
            DesktopTab::Timeline => {
                timeline_view::render(ui, &mut self.state, &*self.client);
            }
            DesktopTab::Audit => {
                audit_view::render(ui, &mut self.state, &*self.client);
            }
            DesktopTab::Terminal => {
                terminal_view::render(ui, &mut self.state, &*self.client);
            }
            DesktopTab::Browser => {
                browser_view::render(ui, &mut self.state, &*self.client);
            }
            DesktopTab::Reports => {
                report_view::render(ui, &mut self.state, &*self.client);
            }
            DesktopTab::Settings => {
                self.render_settings(ui);
            }
        });
    }
}
