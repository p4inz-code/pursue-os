//! Investigation timeline view for PURSUE OS desktop.
//!
//! Provides a unified chronological view of all investigation events:
//! case lifecycle transitions, evidence attachments, notes updates, and audit steps.

use egui::{Color32, RichText, Ui};
use serde_json::json;

use crate::client::IpcClient;
use crate::state::{AuditEventItem, DesktopState};

fn refresh_timeline(case_id: &str, state: &mut DesktopState, client: &dyn IpcClient) {
    match client.call("case", "case.audit.list", json!({ "id": case_id })) {
        Ok(res) => {
            if let Some(events_arr) = res.get("events").and_then(|v| v.as_array()) {
                state.case_audit_items = events_arr
                    .iter()
                    .filter_map(|val| {
                        Some(AuditEventItem {
                            seq: val.get("seq")?.as_u64()?,
                            timestamp_unix: val.get("timestamp_unix")?.as_u64()?,
                            actor: val.get("actor")?.as_str()?.to_string(),
                            action: val.get("action")?.as_str()?.to_string(),
                            subject: val
                                .get("subject")
                                .and_then(|v| v.as_str())
                                .map(ToString::to_string),
                            prev_hash: val
                                .get("prev_hash")
                                .and_then(|v| v.as_str())
                                .map(ToString::to_string),
                            hash: val.get("hash")?.as_str()?.to_string(),
                            verified: val.get("verified")?.as_bool().unwrap_or(true),
                        })
                    })
                    .collect();

                state.audit_chain_verified = res
                    .get("chain_verified")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                state.refresh_timeline_from_audit();
            }
        }
        Err(e) => {
            state.set_error(format!("Failed to retrieve audit timeline: {e}"));
        }
    }
}

/// Renders the Investigation Timeline view tab.
pub fn render(ui: &mut Ui, state: &mut DesktopState, client: &dyn IpcClient) {
    ui.heading("Unified Investigation Timeline");
    ui.add_space(8.0);

    let active_case_id = match &state.active_case {
        Some(c) => c.id.clone(),
        None => {
            ui.label(
                RichText::new(
                    "No active case selected. Select a case in the Cases tab to view its timeline.",
                )
                .color(Color32::from_rgb(255, 180, 80))
                .italics(),
            );
            return;
        }
    };

    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Active Case: {}", active_case_id)).strong());
        ui.label(format!(
            "({} chronological milestones)",
            state.case_timeline_items.len()
        ));

        if state.audit_chain_verified {
            ui.label(
                RichText::new("[PROVENANCE CHAIN VERIFIED]")
                    .color(Color32::GREEN)
                    .strong(),
            );
        } else {
            ui.label(
                RichText::new("[CHAIN VERIFICATION FAILED]")
                    .color(Color32::RED)
                    .strong(),
            );
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Refresh Timeline").clicked() {
                refresh_timeline(&active_case_id, state, client);
                state.set_info("Timeline updated from audit logs.");
            }
        });
    });

    if state.case_timeline_items.is_empty() {
        refresh_timeline(&active_case_id, state, client);
    }

    ui.separator();
    ui.add_space(8.0);

    if state.case_timeline_items.is_empty() {
        ui.label("No timeline events recorded for this case yet.");
    } else {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for item in &state.case_timeline_items {
                let badge_color = match item.category.as_str() {
                    "case" => Color32::from_rgb(70, 160, 240),
                    "evidence" => Color32::from_rgb(50, 200, 150),
                    "terminal" => Color32::from_rgb(180, 180, 180),
                    "browser" => Color32::from_rgb(190, 110, 240),
                    _ => Color32::GRAY,
                };

                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("#{}", item.seq))
                                .monospace()
                                .color(Color32::GRAY),
                        );
                        ui.label(
                            RichText::new(format!("[{}]", item.category.to_uppercase()))
                                .color(badge_color)
                                .strong(),
                        );
                        ui.label(RichText::new(&item.title).strong());
                        ui.label(RichText::new(format!("by {}", item.actor)).italics());
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(format!("Timestamp: {} (Unix)", item.timestamp_unix));
                        });
                    });

                    ui.add_space(2.0);
                    ui.label(&item.summary);

                    if let Some(src) = &item.source_id {
                        ui.label(
                            RichText::new(format!("Reference: {src}"))
                                .monospace()
                                .size(11.0)
                                .color(Color32::GRAY),
                        );
                    }
                });
                ui.add_space(4.0);
            }
        });
    }
}
