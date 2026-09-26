//! Status bar component for the PURSUE OS desktop shell.

use egui::{Color32, RichText, Ui};

use crate::client::IpcClient;
use crate::state::DesktopState;

/// Renders the bottom system and forensic status bar.
pub fn render(ui: &mut Ui, state: &DesktopState, _client: &dyn IpcClient) {
    ui.horizontal(|ui| {
        // System and User identity
        ui.label(
            RichText::new("PURSUE OS v0.1.0")
                .strong()
                .color(Color32::from_rgb(180, 180, 200)),
        );
        ui.separator();
        ui.label(RichText::new("User: pursue-analyst").color(Color32::from_rgb(150, 220, 150)));

        ui.separator();

        // Active Case indicator
        if let Some(case) = &state.active_case {
            ui.label(
                RichText::new(format!("Case: {}", case.id))
                    .color(Color32::from_rgb(100, 200, 255))
                    .strong(),
            );
        } else {
            ui.label(
                RichText::new("Case: None")
                    .color(Color32::from_rgb(255, 180, 80))
                    .italics(),
            );
        }

        ui.separator();

        // Security / Routing status
        ui.label(
            RichText::new("Tor Boundary: FAIL-CLOSED")
                .color(Color32::from_rgb(120, 220, 120))
                .strong(),
        );

        ui.separator();

        // Status message notification
        if let Some(msg) = &state.status_message {
            ui.label(RichText::new(msg).color(Color32::from_rgb(255, 230, 100)));
        }
    });
}
