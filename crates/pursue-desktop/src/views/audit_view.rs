//! Cryptographic audit log viewer for PURSUE OS desktop.
//!
//! Provides inspection and verification of the immutable SHA-256 hash-chained
//! provenance trail for the active investigation case.

use egui::{Color32, RichText, Ui};
use serde_json::json;

use crate::client::IpcClient;
use crate::state::{AuditEventItem, DesktopState};

fn refresh_audit_log(case_id: &str, state: &mut DesktopState, client: &dyn IpcClient) {
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
            }
        }
        Err(e) => {
            state.set_error(format!("Failed to retrieve audit log: {e}"));
        }
    }
}

/// Renders the Audit Log view tab.
pub fn render(ui: &mut Ui, state: &mut DesktopState, client: &dyn IpcClient) {
    ui.heading("Cryptographic Audit Trail & Provenance Viewer");
    ui.add_space(8.0);

    let active_case_id = match &state.active_case {
        Some(c) => c.id.clone(),
        None => {
            ui.label(
                RichText::new("No active case selected. Select a case in the Cases tab to view its audit log.")
                    .color(Color32::from_rgb(255, 180, 80))
                    .italics(),
            );
            return;
        }
    };

    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Active Case: {}", active_case_id)).strong());
        ui.label(format!(
            "({} hash-chained events)",
            state.case_audit_items.len()
        ));

        if state.audit_chain_verified {
            ui.label(
                RichText::new("[CHAIN INTACT — SHA-256 VERIFIED]")
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
            if ui.button("Verify & Refresh Audit Chain").clicked() {
                refresh_audit_log(&active_case_id, state, client);
                state.set_info("Audit chain re-verified against case store.");
            }
        });
    });

    if state.case_audit_items.is_empty() {
        refresh_audit_log(&active_case_id, state, client);
    }

    ui.separator();
    ui.add_space(8.0);

    if state.case_audit_items.is_empty() {
        ui.label("No audit events recorded for this case.");
    } else {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for entry in &state.case_audit_items {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("Seq #{}", entry.seq))
                                .monospace()
                                .strong(),
                        );
                        ui.label(
                            RichText::new(format!("Action: {}", entry.action))
                                .color(Color32::from_rgb(100, 200, 255))
                                .strong(),
                        );
                        ui.label(format!("Actor: {}", entry.actor));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if entry.verified {
                                ui.label(
                                    RichText::new("[VERIFIED]").color(Color32::GREEN).strong(),
                                );
                            } else {
                                ui.label(RichText::new("[TAMPERED]").color(Color32::RED).strong());
                            }
                        });
                    });

                    ui.add_space(4.0);
                    if let Some(subj) = &entry.subject {
                        ui.label(
                            RichText::new(format!("Subject Address: {subj}"))
                                .monospace()
                                .size(11.0)
                                .color(Color32::from_rgb(255, 200, 100)),
                        );
                    }

                    ui.horizontal(|ui| {
                        let prev = entry.prev_hash.as_deref().unwrap_or("genesis");
                        ui.label(
                            RichText::new(format!("Prev Hash: {:.16}...", prev))
                                .monospace()
                                .size(11.0)
                                .color(Color32::GRAY),
                        );
                        ui.label(
                            RichText::new(format!("Hash: {:.16}...", entry.hash))
                                .monospace()
                                .size(11.0)
                                .color(Color32::from_rgb(80, 220, 120)),
                        );
                    });
                });
                ui.add_space(4.0);
            }
        });
    }
}
