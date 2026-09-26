//! Forensic report generation and export view for PURSUE OS desktop.
//!
//! Provides report configuration, in-memory preview, cryptographic sealing,
//! atomic safe file export, and report verification.

use egui::{Color32, RichText, Ui};
use serde_json::json;

use crate::client::IpcClient;
use crate::state::DesktopState;

/// Renders the Forensic Reporting view tab.
pub fn render(ui: &mut Ui, state: &mut DesktopState, client: &dyn IpcClient) {
    ui.heading("Forensic Investigation Reporting & Cryptographic Export");
    ui.add_space(8.0);

    let active_case_id = match &state.active_case {
        Some(c) => c.id.clone(),
        None => {
            ui.label(
                RichText::new(
                    "No active case selected. Select a case in the Cases tab to generate reports.",
                )
                .color(Color32::from_rgb(255, 180, 80))
                .italics(),
            );
            return;
        }
    };

    ui.horizontal(|ui| {
        ui.label(RichText::new(format!("Active Case: {}", active_case_id)).strong());

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button("Check Report Metadata").clicked() {
                match client.call("report", "report.preview_metadata", json!({ "case_id": active_case_id })) {
                    Ok(res) => {
                        let ev_count = res.get("evidence_count").and_then(|v| v.as_u64()).unwrap_or(0);
                        let audit_count = res.get("audit_events_count").and_then(|v| v.as_u64()).unwrap_or(0);
                        let chain_ok = res.get("audit_chain_verified").and_then(|v| v.as_bool()).unwrap_or(false);
                        state.set_info(format!(
                            "Case report ready: {ev_count} evidence items, {audit_count} audit events, chain intact: {chain_ok}"
                        ));
                    }
                    Err(e) => state.set_error(format!("Failed to retrieve report preview metadata: {e}")),
                }
            }
        });
    });

    ui.separator();
    ui.add_space(8.0);

    // Generation Controls
    ui.group(|ui| {
        ui.heading("Report Generation Settings");
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.label(RichText::new("Format:").strong());
            ui.radio_value(
                &mut state.report_format,
                "html".to_string(),
                "HTML (Self-contained, printable)",
            );
            ui.radio_value(
                &mut state.report_format,
                "json".to_string(),
                "JSON (Canonical deterministic data)",
            );
        });

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button("Generate In-Memory Report Preview").clicked() {
                let actor = state.investigator_id.clone();
                let format = state.report_format.clone();
                match client.call(
                    "report",
                    "report.generate",
                    json!({ "case_id": active_case_id, "actor": actor, "format": format }),
                ) {
                    Ok(res) => {
                        let hash = res
                            .get("report_hash")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        let content = res
                            .get("content")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string();
                        state.report_last_hash = Some(hash.clone());
                        state.report_preview_content = Some(content);
                        state.set_info(format!(
                            "Generated report successfully. Cryptographic Hash: {hash}"
                        ));
                    }
                    Err(e) => {
                        state.set_error(format!("Report generation failed: {e}"));
                    }
                }
            }
        });
    });

    ui.add_space(10.0);

    // Export Controls
    ui.group(|ui| {
        ui.heading("Safe Report Export");
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.label("Export Destination Path:");
            ui.text_edit_singleline(&mut state.report_export_path);

            if ui.button("Export Report to Disk").clicked() {
                let actor = state.investigator_id.clone();
                let format = state.report_format.clone();
                let target = state.report_export_path.trim().to_string();
                if target.is_empty() {
                    state.set_error("Export target path must not be empty.");
                } else {
                    match client.call(
                        "report",
                        "report.export",
                        json!({
                            "case_id": active_case_id,
                            "actor": actor,
                            "format": format,
                            "target_path": target
                        }),
                    ) {
                        Ok(res) => {
                            let dest = res
                                .get("destination")
                                .and_then(|v| v.as_str())
                                .unwrap_or(&target);
                            let hash = res
                                .get("report_hash")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            let size = res.get("size_bytes").and_then(|v| v.as_u64()).unwrap_or(0);
                            state.report_last_hash = Some(hash.to_string());
                            state.set_info(format!(
                                "Report safely exported to '{dest}' ({size} bytes). Hash: {hash}"
                            ));
                        }
                        Err(e) => {
                            state.set_error(format!("Report export failed: {e}"));
                        }
                    }
                }
            }
        });
    });

    ui.add_space(10.0);

    // Verify Exported Report
    ui.group(|ui| {
        ui.heading("Verify Report File Integrity");
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.label("Report File Path:");
            ui.text_edit_singleline(&mut state.report_verify_path);

            if ui.button("Verify Report").clicked() {
                let file_path = state.report_verify_path.trim().to_string();
                if file_path.is_empty() {
                    state.set_error("Please enter a report file path to verify.");
                } else {
                    match client.call("report", "report.verify", json!({ "file_path": file_path }))
                    {
                        Ok(res) => {
                            let verified = res
                                .get("verified")
                                .and_then(|v| v.as_bool())
                                .unwrap_or(false);
                            let hash = res
                                .get("report_hash")
                                .and_then(|v| v.as_str())
                                .unwrap_or("N/A");
                            let details = res.get("details").and_then(|v| v.as_str()).unwrap_or("");
                            if verified {
                                state.report_verify_result =
                                    Some(format!("VERIFIED OK: {details} (Hash: {hash})"));
                                state.set_info(format!(
                                    "Report integrity verified OK for '{file_path}'."
                                ));
                            } else {
                                state.report_verify_result =
                                    Some("VERIFICATION FAILED".to_string());
                                state.set_error("Report verification failed.");
                            }
                        }
                        Err(e) => {
                            state.report_verify_result = Some(format!("ERROR: {e}"));
                            state.set_error(format!("Report verification error: {e}"));
                        }
                    }
                }
            }
        });

        if let Some(res) = &state.report_verify_result {
            ui.add_space(4.0);
            let color = if res.starts_with("VERIFIED OK") {
                Color32::GREEN
            } else {
                Color32::RED
            };
            ui.label(RichText::new(res).color(color).strong());
        }
    });

    ui.add_space(10.0);

    // Report Preview Pane
    if let Some(preview) = &state.report_preview_content {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.heading("Generated Report Preview");
                if let Some(hash) = &state.report_last_hash {
                    ui.label(
                        RichText::new(format!("SHA-256 Digest: {hash}"))
                            .monospace()
                            .color(Color32::from_rgb(100, 200, 255)),
                    );
                }
            });

            ui.add_space(4.0);
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut preview.as_str())
                        .font(egui::TextStyle::Monospace)
                        .desired_width(f32::INFINITY)
                        .desired_rows(16),
                );
            });
        });
    }
}
