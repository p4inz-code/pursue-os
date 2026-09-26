//! Main desktop application shell implementing `eframe::App`.

use egui::{Color32, RichText};

use crate::client::IpcClient;
use crate::state::{DesktopState, DesktopTab};
use crate::views::{browser_view, case_view, status_bar, terminal_view};

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
        Self {
            client,
            state: DesktopState::new(),
        }
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

            ui.add_space(24.0);

            // Tab navigation buttons
            let tabs = [
                (DesktopTab::Cases, "Cases"),
                (DesktopTab::Terminal, "Terminal"),
                (DesktopTab::Browser, "Browser & Tor"),
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
            ui.label("• Chain-of-Custody: SHA-256 Content Hashing + Merkle Audit Log");
            ui.label("• IPC Security: Local Domain Socket Dispatch, privilege separation");
            ui.label("• UI Direct Access: FORBIDDEN — all actions dispatch through IPC Router");
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
            ui.label(RichText::new("IPC Subsystem Check").strong());
            if ui.button("Probe IPC Handlers").clicked() {
                // Check if terminal and browser handlers are responsive over IPC
                let term_probe = self.client.call(
                    "terminal",
                    "session.get",
                    serde_json::json!({ "session_id": "probe" }),
                );
                let browser_probe = self.client.call(
                    "browser",
                    "browser.session.get",
                    serde_json::json!({ "session_id": "probe" }),
                );

                // In router, if service is registered, response will be from handler (e.g. SessionNotFound).
                // If service is NOT registered, router error is ServiceNotFound.
                let term_registered = match &term_probe {
                    Err(e) => !e.to_string().contains("service not found"),
                    Ok(_) => true,
                };
                let browser_registered = match &browser_probe {
                    Err(e) => !e.to_string().contains("service not found"),
                    Ok(_) => true,
                };

                if term_registered && browser_registered {
                    self.state
                        .set_info("All backend IPC services verified and responsive.");
                } else {
                    self.state.set_error(format!(
                        "IPC Diagnostics: terminal={term_registered}, browser={browser_registered}"
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
            DesktopTab::Cases => {
                case_view::render(ui, &mut self.state, &*self.client);
            }
            DesktopTab::Terminal => {
                terminal_view::render(ui, &mut self.state, &*self.client);
            }
            DesktopTab::Browser => {
                browser_view::render(ui, &mut self.state, &*self.client);
            }
            DesktopTab::Settings => {
                self.render_settings(ui);
            }
        });
    }
}
