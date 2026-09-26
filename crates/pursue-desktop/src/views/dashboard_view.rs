//! Dashboard view for PURSUE OS desktop.
//!
//! Provides system overview, active case telemetry, evidence summary, and quick navigation.

use egui::{Color32, RichText, Ui};

use crate::client::IpcClient;
use crate::state::{DesktopState, DesktopTab};

/// Renders the Dashboard view tab.
pub fn render(ui: &mut Ui, state: &mut DesktopState, client: &dyn IpcClient) {
    ui.heading("PURSUE OS — Forensic Investigation Dashboard");
    ui.add_space(8.0);

    // Refresh telemetry on dashboard view
    if state.cases.is_empty() {
        if let Ok(res) = client.call("case", "case.list", serde_json::json!({})) {
            if let Some(arr) = res.as_array() {
                state.cases = arr
                    .iter()
                    .filter_map(|val| {
                        Some(crate::state::CaseSummary {
                            id: val.get("id")?.as_str()?.to_string(),
                            title: val.get("title")?.as_str()?.to_string(),
                            created_by: val.get("created_by")?.as_str()?.to_string(),
                            status: val.get("status")?.as_str()?.to_string(),
                            evidence_count: val.get("evidence_count")?.as_u64()? as usize,
                            audit_count: val.get("audit_events_count")?.as_u64()? as usize,
                        })
                    })
                    .collect();
            }
        }
    }

    // Telemetry Summary Cards
    ui.horizontal(|ui| {
        ui.group(|ui| {
            ui.set_min_width(140.0);
            ui.label(RichText::new("TOTAL CASES").size(11.0).color(Color32::GRAY));
            ui.label(
                RichText::new(format!("{}", state.cases.len()))
                    .size(22.0)
                    .strong()
                    .color(Color32::from_rgb(100, 200, 255)),
            );
        });

        let open_cases = state.cases.iter().filter(|c| c.status == "open").count();
        ui.group(|ui| {
            ui.set_min_width(140.0);
            ui.label(
                RichText::new("ACTIVE / OPEN")
                    .size(11.0)
                    .color(Color32::GRAY),
            );
            ui.label(
                RichText::new(format!("{open_cases}"))
                    .size(22.0)
                    .strong()
                    .color(Color32::from_rgb(80, 220, 120)),
            );
        });

        let total_evidence: usize = state.cases.iter().map(|c| c.evidence_count).sum();
        ui.group(|ui| {
            ui.set_min_width(140.0);
            ui.label(
                RichText::new("EVIDENCE ITEMS")
                    .size(11.0)
                    .color(Color32::GRAY),
            );
            ui.label(
                RichText::new(format!("{total_evidence}"))
                    .size(22.0)
                    .strong()
                    .color(Color32::from_rgb(255, 200, 80)),
            );
        });

        let total_audit: usize = state.cases.iter().map(|c| c.audit_count).sum();
        ui.group(|ui| {
            ui.set_min_width(140.0);
            ui.label(
                RichText::new("AUDIT EVENTS")
                    .size(11.0)
                    .color(Color32::GRAY),
            );
            ui.label(
                RichText::new(format!("{total_audit}"))
                    .size(22.0)
                    .strong()
                    .color(Color32::from_rgb(200, 140, 255)),
            );
        });

        ui.group(|ui| {
            ui.set_min_width(140.0);
            ui.label(RichText::new("IPC STATUS").size(11.0).color(Color32::GRAY));
            ui.label(
                RichText::new("ONLINE")
                    .size(22.0)
                    .strong()
                    .color(Color32::from_rgb(80, 220, 120)),
            );
        });
    });

    ui.add_space(16.0);
    ui.separator();
    ui.add_space(12.0);

    // Active Case Overview
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.heading("Active Case Focus");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Switch / Manage Cases").clicked() {
                    state.active_tab = DesktopTab::Cases;
                }
            });
        });
        ui.add_space(6.0);

        if let Some(c) = &state.active_case {
            ui.horizontal(|ui| {
                ui.label(RichText::new("ID:").strong());
                ui.label(RichText::new(&c.id).monospace().strong());
                ui.label(RichText::new("Title:").strong());
                ui.label(&c.title);
                ui.label(RichText::new("Status:").strong());
                let color = if c.status == "open" {
                    Color32::GREEN
                } else {
                    Color32::GRAY
                };
                ui.label(RichText::new(&c.status).color(color).strong());
            });

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(format!(
                    "Attached Evidence: {} items  |  Chained Audit Events: {}",
                    c.evidence_count, c.audit_count
                ));
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Open Evidence Explorer").clicked() {
                    state.active_tab = DesktopTab::Evidence;
                }
                if ui.button("View Investigation Timeline").clicked() {
                    state.active_tab = DesktopTab::Timeline;
                }
                if ui.button("Inspect Audit Trail").clicked() {
                    state.active_tab = DesktopTab::Audit;
                }
                if ui.button("Generate Case Report").clicked() {
                    state.active_tab = DesktopTab::Reports;
                }
            });
        } else {
            ui.label(
                RichText::new("No active investigation case is currently selected.")
                    .color(Color32::from_rgb(255, 180, 80))
                    .italics(),
            );
            ui.add_space(4.0);
            if ui.button("Select or Create a Case").clicked() {
                state.active_tab = DesktopTab::Cases;
            }
        }
    });

    ui.add_space(16.0);

    // Quick Action Tools
    ui.group(|ui| {
        ui.heading("Forensic Workbench Tools");
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            if ui
                .button(RichText::new("Terminal Console").size(14.0))
                .clicked()
            {
                state.active_tab = DesktopTab::Terminal;
            }
            if ui
                .button(RichText::new("Browser / Tor Console").size(14.0))
                .clicked()
            {
                state.active_tab = DesktopTab::Browser;
            }
            if ui
                .button(RichText::new("Evidence Explorer").size(14.0))
                .clicked()
            {
                state.active_tab = DesktopTab::Evidence;
            }
            if ui
                .button(RichText::new("Forensic Reports").size(14.0))
                .clicked()
            {
                state.active_tab = DesktopTab::Reports;
            }
        });
    });
}
