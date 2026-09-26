//! Evidence explorer view for PURSUE OS desktop.
//!
//! Provides inspection of cryptographic evidence items attached to the active case:
//! - Content address (SHA-256)
//! - Payload verification status
//! - Provenance metadata (source, acquisition time, size)
//! - Content preview (UTF-8 text or hex dump)

use egui::{Color32, RichText, Ui};
use serde_json::json;

use crate::client::IpcClient;
use crate::state::{DesktopState, EvidenceItem};

fn refresh_evidence(case_id: &str, state: &mut DesktopState, client: &dyn IpcClient) {
    match client.call("case", "case.evidence.list", json!({ "id": case_id })) {
        Ok(res) => {
            if let Some(arr) = res.as_array() {
                state.case_evidence_items = arr
                    .iter()
                    .filter_map(|val| {
                        Some(EvidenceItem {
                            address: val.get("address")?.as_str()?.to_string(),
                            size: val.get("size")?.as_u64()?,
                            acquired_at_unix: val.get("acquired_at_unix")?.as_u64()?,
                            source: val.get("source")?.as_str()?.to_string(),
                            verified: val.get("verified")?.as_bool().unwrap_or(true),
                            preview: None,
                        })
                    })
                    .collect();
            }
        }
        Err(e) => {
            state.set_error(format!("Failed to list case evidence: {e}"));
        }
    }
}

fn inspect_evidence(
    case_id: &str,
    address: &str,
    state: &mut DesktopState,
    client: &dyn IpcClient,
) {
    match client.call(
        "case",
        "case.evidence.read",
        json!({ "id": case_id, "address": address }),
    ) {
        Ok(res) => {
            let preview = res
                .get("preview")
                .and_then(|v| v.as_str())
                .unwrap_or("No preview available")
                .to_string();
            state.selected_evidence_addr = Some(address.to_string());
            state.selected_evidence_preview = Some(preview);
        }
        Err(e) => {
            state.set_error(format!("Failed to read evidence payload: {e}"));
        }
    }
}

/// Renders the Evidence Explorer view tab.
pub fn render(ui: &mut Ui, state: &mut DesktopState, client: &dyn IpcClient) {
    ui.heading("Evidence Repository Explorer");
    ui.add_space(8.0);

    let active_case_id = match &state.active_case {
        Some(c) => c.id.clone(),
        None => {
            ui.label(
                RichText::new("No active case selected. Select an active case in the Cases tab to inspect evidence.")
                    .color(Color32::from_rgb(255, 180, 80))
                    .italics(),
            );
            return;
        }
    };

    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Active Case: {}", active_case_id)).strong());
        ui.label(format!(
            "({} attached items)",
            state.case_evidence_items.len()
        ));

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Refresh Evidence List").clicked() {
                refresh_evidence(&active_case_id, state, client);
                state.set_info("Refreshed evidence list via IPC.");
            }
        });
    });

    // Auto-refresh evidence if empty but case has evidence
    if state.case_evidence_items.is_empty() {
        if let Some(c) = &state.active_case {
            if c.evidence_count > 0 {
                refresh_evidence(&active_case_id, state, client);
            }
        }
    }

    ui.separator();
    ui.add_space(8.0);

    if state.case_evidence_items.is_empty() {
        ui.label("No evidence artifacts attached to this case yet.");
        ui.label("Use the Terminal or Browser tools to acquire and capture verified evidence.");
    } else {
        let mut target_to_inspect = None;

        ui.horizontal(|ui| {
            // Left pane: Evidence list
            ui.vertical(|ui| {
                ui.set_width(450.0);
                ui.label(RichText::new("Attached Evidence Inventory").heading());
                ui.add_space(4.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for item in &state.case_evidence_items {
                        let is_selected = state.selected_evidence_addr.as_deref() == Some(&item.address);
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("{:.16}...", item.address))
                                        .monospace()
                                        .strong()
                                        .color(Color32::from_rgb(80, 200, 255)),
                                );

                                if item.verified {
                                    ui.label(RichText::new("[VERIFIED]").color(Color32::GREEN).strong());
                                } else {
                                    ui.label(RichText::new("[UNVERIFIED]").color(Color32::RED).strong());
                                }
                            });

                            ui.label(format!("Source: {} | Size: {} B", item.source, item.size));
                            ui.label(format!("Acquired at: {} (Unix)", item.acquired_at_unix));

                            if is_selected {
                                ui.label(RichText::new("CURRENTLY INSPECTING").color(Color32::YELLOW));
                            } else if ui.button("Inspect Payload").clicked() {
                                target_to_inspect = Some(item.address.clone());
                            }
                        });
                        ui.add_space(4.0);
                    }
                });
            });

            ui.separator();

            // Right pane: Evidence inspection
            ui.vertical(|ui| {
                ui.label(RichText::new("Artifact Payload Inspection").heading());
                ui.add_space(4.0);

                if let Some(addr) = &state.selected_evidence_addr {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("SHA-256 Digest:").strong());
                        ui.label(RichText::new(addr).monospace());
                    });

                    ui.add_space(8.0);
                    ui.label(RichText::new("Decoded Payload Preview:").strong());

                    if let Some(preview) = &state.selected_evidence_preview {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut preview.as_str())
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(f32::INFINITY)
                                    .desired_rows(18),
                            );
                        });
                    } else {
                        ui.label("No preview loaded.");
                    }
                } else {
                    ui.label("Select an evidence item from the left inventory to inspect its decoded content.");
                }
            });
        });

        if let Some(addr) = target_to_inspect {
            inspect_evidence(&active_case_id, &addr, state, client);
        }
    }
}
