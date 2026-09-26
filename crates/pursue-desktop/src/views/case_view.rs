//! Case management view.
//!
//! Provides genuine IPC-mediated case operations:
//! - List, create, and switch active cases
//! - Edit case title and investigator notes with audited transactions
//! - Transition lifecycle (close and reopen)
//! - Perform deep cryptographic verification (manifest, hash chain, evidence)

use egui::{Color32, RichText, Ui};
use serde_json::json;

use crate::client::IpcClient;
use crate::state::{CaseDetail, CaseSummary, DesktopState};

fn refresh_cases(state: &mut DesktopState, client: &dyn IpcClient) {
    match client.call("case", "case.list", json!({})) {
        Ok(res) => {
            if let Some(arr) = res.as_array() {
                state.cases = arr
                    .iter()
                    .filter_map(|val| {
                        Some(CaseSummary {
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
        Err(e) => {
            state.set_error(format!("Failed to list cases: {e}"));
        }
    }
}

fn load_active_case_detail(case_id: &str, state: &mut DesktopState, client: &dyn IpcClient) {
    match client.call("case", "case.get", json!({ "id": case_id })) {
        Ok(res) => {
            let addrs = res
                .get("evidence_addresses")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|s| s.as_str().map(ToString::to_string))
                        .collect()
                })
                .unwrap_or_default();

            let detail = CaseDetail {
                id: res
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or(case_id)
                    .to_string(),
                title: res
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                notes: res
                    .get("notes")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                status: res
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("open")
                    .to_string(),
                created_by: res
                    .get("created_by")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                created_at_unix: res
                    .get("created_at_unix")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0),
                evidence_addresses: addrs,
                audit_count: res
                    .get("audit_events_count")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize,
                verified: res
                    .get("verified")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false),
            };

            state.edit_title = detail.title.clone();
            state.edit_notes = detail.notes.clone();
            state.active_case_detail = Some(detail);
        }
        Err(e) => {
            state.set_error(format!("Failed to load case detail: {e}"));
        }
    }
}

/// Renders the Case Management view tab.
pub fn render(ui: &mut Ui, state: &mut DesktopState, client: &dyn IpcClient) {
    ui.heading("Forensic Case Management & Operations");
    ui.add_space(8.0);

    // Initial load of cases list if empty
    if state.cases.is_empty() {
        refresh_cases(state, client);
    }

    ui.horizontal(|ui| {
        ui.label(RichText::new("Active Case Focus:").strong());
        if let Some(c) = &state.active_case {
            ui.label(
                RichText::new(format!("{} — {}", c.id, c.title))
                    .color(Color32::from_rgb(100, 200, 255))
                    .strong(),
            );
            let status_color = if c.status == "open" {
                Color32::GREEN
            } else {
                Color32::GRAY
            };
            ui.label(
                RichText::new(format!("[{}]", c.status.to_uppercase()))
                    .color(status_color)
                    .strong(),
            );
        } else {
            ui.label(
                RichText::new("No active case selected")
                    .color(Color32::from_rgb(255, 180, 80))
                    .italics(),
            );
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Refresh Cases List").clicked() {
                refresh_cases(state, client);
                let active_id = state.active_case.as_ref().map(|c| c.id.clone());
                if let Some(id) = active_id {
                    load_active_case_detail(&id, state, client);
                }
                state.set_info("Refreshed case listings from IPC backend.");
            }
        });
    });

    ui.separator();
    ui.add_space(8.0);

    // Create New Case Box
    ui.group(|ui| {
        ui.label(RichText::new("Initialize New Forensic Case").strong());
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label("Case ID:");
            ui.text_edit_singleline(&mut state.new_case_id);
            ui.label("Title:");
            ui.text_edit_singleline(&mut state.new_case_title);

            if ui.button("Create Case").clicked() {
                let id = state.new_case_id.trim().to_string();
                let title = state.new_case_title.trim().to_string();
                if id.is_empty() || title.is_empty() {
                    state.set_error("Case ID and Title must not be empty.");
                } else {
                    let actor = state.investigator_id.clone();
                    match client.call(
                        "case",
                        "case.create",
                        json!({ "id": id, "title": title, "actor": actor }),
                    ) {
                        Ok(res) => {
                            let created_id = res.get("id").and_then(|v| v.as_str()).unwrap_or(&id);
                            state.new_case_id.clear();
                            state.new_case_title.clear();
                            refresh_cases(state, client);

                            let summary = CaseSummary {
                                id: created_id.to_string(),
                                title,
                                created_by: actor,
                                status: "open".to_string(),
                                evidence_count: 0,
                                audit_count: 1,
                            };
                            state.active_case = Some(summary);
                            load_active_case_detail(created_id, state, client);
                            state.set_info(format!(
                                "Case '{created_id}' successfully created and activated."
                            ));
                        }
                        Err(e) => {
                            state.set_error(format!("Failed to create case: {e}"));
                        }
                    }
                }
            }
        });
    });

    ui.add_space(10.0);

    // Active Case Operations Panel
    if let Some(detail) = state.active_case_detail.clone() {
        let case_id = detail.id.clone();
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.heading(format!("Case Details: {}", detail.id));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Deep Verify Case Integrity").clicked() {
                        match client.call("case", "case.verify", json!({ "id": case_id })) {
                            Ok(res) => {
                                let verified = res.get("verified").and_then(|v| v.as_bool()).unwrap_or(false);
                                let ev_count = res.get("evidence_verified_count").and_then(|v| v.as_u64()).unwrap_or(0);
                                if verified {
                                    state.set_info(format!(
                                        "Case '{}' verified successfully: Manifest, hash-chain, and {} evidence blobs intact.",
                                        case_id, ev_count
                                    ));
                                } else {
                                    state.set_error(format!("Case '{}' verification reported failure.", case_id));
                                }
                            }
                            Err(e) => {
                                state.set_error(format!("Deep verification failed: {e}"));
                            }
                        }
                    }
                });
            });

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Status:").strong());
                let status_color = if detail.status == "open" {
                    Color32::GREEN
                } else {
                    Color32::GRAY
                };
                ui.label(RichText::new(detail.status.to_uppercase()).color(status_color).strong());

                ui.label(format!(" | Created by: {} | Created at: {} (Unix)", detail.created_by, detail.created_at_unix));

                if detail.status == "open" {
                    if ui.button("Close Case").clicked() {
                        let actor = state.investigator_id.clone();
                        match client.call("case", "case.close", json!({ "id": case_id, "actor": actor })) {
                            Ok(_) => {
                                refresh_cases(state, client);
                                load_active_case_detail(&case_id, state, client);
                                state.set_info(format!("Case '{case_id}' closed."));
                            }
                            Err(e) => state.set_error(format!("Failed to close case: {e}")),
                        }
                    }
                } else if ui.button("Reopen Case").clicked() {
                    let actor = state.investigator_id.clone();
                    match client.call("case", "case.reopen", json!({ "id": case_id, "actor": actor })) {
                        Ok(_) => {
                            refresh_cases(state, client);
                            load_active_case_detail(&case_id, state, client);
                            state.set_info(format!("Case '{case_id}' reopened."));
                        }
                        Err(e) => state.set_error(format!("Failed to reopen case: {e}")),
                    }
                }
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Title:");
                ui.text_edit_singleline(&mut state.edit_title);
                if ui.button("Save Title").clicked() {
                    let actor = state.investigator_id.clone();
                    let title = state.edit_title.trim().to_string();
                    match client.call("case", "case.update_title", json!({ "id": case_id, "title": title, "actor": actor })) {
                        Ok(_) => {
                            refresh_cases(state, client);
                            load_active_case_detail(&case_id, state, client);
                            state.set_info("Case title updated and audited.");
                        }
                        Err(e) => state.set_error(format!("Failed to update title: {e}")),
                    }
                }
            });

            ui.add_space(8.0);
            ui.label(RichText::new("Investigator Notes:").strong());
            ui.text_edit_multiline(&mut state.edit_notes);
            if ui.button("Save Notes").clicked() {
                let actor = state.investigator_id.clone();
                let notes = state.edit_notes.clone();
                match client.call("case", "case.update_notes", json!({ "id": case_id, "notes": notes, "actor": actor })) {
                    Ok(_) => {
                        load_active_case_detail(&case_id, state, client);
                        state.set_info("Case notes saved and audited.");
                    }
                    Err(e) => state.set_error(format!("Failed to update notes: {e}")),
                }
            }
        });
        ui.add_space(10.0);
    }

    // Registered Cases Table
    ui.label(RichText::new("Available Forensic Cases:").heading());
    if state.cases.is_empty() {
        ui.label("No cases available. Use the form above to initialize your first case.");
    } else {
        let mut target_case_to_select = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for c in &state.cases {
                let is_active = state.active_case.as_ref().map(|a| &a.id) == Some(&c.id);
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&c.id).monospace().strong());
                        ui.label(format!("— {}", c.title));
                        let color = if c.status == "open" {
                            Color32::GREEN
                        } else {
                            Color32::GRAY
                        };
                        ui.label(
                            RichText::new(format!("[{}]", c.status.to_uppercase()))
                                .color(color)
                                .strong(),
                        );
                        ui.label(format!(
                            "(Evidence: {}, Events: {})",
                            c.evidence_count, c.audit_count
                        ));

                        if is_active {
                            ui.label(
                                RichText::new("[ACTIVE FOCUS]")
                                    .color(Color32::GREEN)
                                    .strong(),
                            );
                        } else if ui.button("Select as Active").clicked() {
                            target_case_to_select = Some(c.clone());
                        }
                    });
                });
            }
        });

        if let Some(c) = target_case_to_select {
            state.active_case = Some(c.clone());
            load_active_case_detail(&c.id, state, client);
            state.set_info(format!("Switched active case to '{}'.", c.id));
        }
    }
}
