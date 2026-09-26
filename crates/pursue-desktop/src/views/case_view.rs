//! Case management view.

use egui::{Color32, RichText, Ui};

use crate::client::IpcClient;
use crate::state::{CaseSummary, DesktopState};

/// Renders the Case Management view tab.
pub fn render(ui: &mut Ui, state: &mut DesktopState, _client: &dyn IpcClient) {
    ui.heading("Forensic Case Management");
    ui.add_space(8.0);

    ui.horizontal(|ui| {
        ui.label(RichText::new("Active Case:").strong());
        if let Some(c) = &state.active_case {
            ui.label(
                RichText::new(format!("{} — {}", c.id, c.title))
                    .color(Color32::from_rgb(100, 200, 255))
                    .strong(),
            );
        } else {
            ui.label(
                RichText::new("No active case selected")
                    .color(Color32::from_rgb(255, 180, 80))
                    .italics(),
            );
        }
    });

    ui.separator();
    ui.add_space(8.0);

    // Create New Case Box
    ui.group(|ui| {
        ui.label(RichText::new("Create New Investigation Case").strong());
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
                    let summary = CaseSummary {
                        id: id.clone(),
                        title,
                        created_by: state.investigator_id.clone(),
                        evidence_count: 0,
                        audit_count: 1,
                    };
                    state.cases.push(summary.clone());
                    state.active_case = Some(summary);
                    state.new_case_id.clear();
                    state.new_case_title.clear();
                    state.set_info(format!("Case '{id}' created and set as active."));
                }
            }
        });
    });

    ui.add_space(12.0);
    ui.label(RichText::new("Registered Cases:").heading());

    if state.cases.is_empty() {
        ui.label("No cases available. Create a case above to begin investigation.");
    } else {
        let mut selected_case = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for c in &state.cases {
                let is_active = state.active_case.as_ref().map(|a| &a.id) == Some(&c.id);
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&c.id).monospace().strong());
                        ui.label(format!("— {}", c.title));
                        ui.label(format!(
                            "(Evidence: {}, Events: {})",
                            c.evidence_count, c.audit_count
                        ));

                        if is_active {
                            ui.label(RichText::new("[ACTIVE]").color(Color32::GREEN).strong());
                        } else if ui.button("Select as Active").clicked() {
                            selected_case = Some(c.clone());
                        }
                    });
                });
            }
        });
        if let Some(c) = selected_case {
            state.active_case = Some(c.clone());
            state.set_info(format!("Switched active case to '{}'.", c.id));
        }
    }
}
