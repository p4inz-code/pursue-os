//! Investigation Browser & Tor console view.

use egui::{Color32, RichText, Ui};

use crate::client::IpcClient;
use crate::state::{BrowserNavigationEntry, DesktopState};

/// Renders the Investigation Browser view tab.
pub fn render(ui: &mut Ui, state: &mut DesktopState, client: &dyn IpcClient) {
    ui.heading("Investigation Browser & Tor Research Console");
    ui.add_space(8.0);

    // Session & Routing Bar
    ui.horizontal(|ui| {
        let case_str = state
            .active_case
            .as_ref()
            .map(|c| c.id.as_str())
            .unwrap_or("NONE");
        ui.label(format!("Active Case: [{case_str}]"));

        ui.separator();

        let sess_str = state
            .browser_session_id
            .as_deref()
            .unwrap_or("Not Initialized");
        ui.label(format!("Session: [{sess_str}]"));

        ui.separator();

        ui.label(RichText::new("Routing Mode:").strong());
        let is_tor = state.browser_mode == "tor";
        if ui.selectable_label(is_tor, "Tor (Onion Routing)").clicked() {
            state.browser_mode = "tor".to_string();
            // Invalidate existing session to ensure clean routing separation
            state.browser_session_id = None;
        }
        if ui.selectable_label(!is_tor, "Direct (Clearweb)").clicked() {
            state.browser_mode = "direct".to_string();
            state.browser_session_id = None;
        }

        if state.browser_session_id.is_none() {
            if state.active_case.is_some() {
                if ui.button("Start Browser Session").clicked() {
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let sid = format!("browser-{}-{}", std::process::id(), ts);
                    let cid = state.active_case.as_ref().unwrap().id.clone();
                    let actor = state.investigator_id.clone();
                    let mode = state.browser_mode.clone();

                    match client.call(
                        "browser",
                        "browser.session.create",
                        serde_json::json!({
                            "session_id": sid,
                            "case_id": cid,
                            "actor": actor,
                            "mode": mode,
                        }),
                    ) {
                        Ok(_) => {
                            state.browser_session_id = Some(sid);
                            state.set_info(format!("Browser session started in {mode} mode."));
                        }
                        Err(e) => state.set_error(format!("Failed to create browser session: {e}")),
                    }
                }
            } else {
                ui.label(
                    RichText::new(
                        "Select an active case in the Cases tab to start browser session.",
                    )
                    .color(Color32::from_rgb(255, 180, 80))
                    .italics(),
                );
            }
        } else if ui.button("Terminate Session").clicked() {
            if let Some(sid) = state.browser_session_id.clone() {
                let _ = client.call(
                    "browser",
                    "browser.session.terminate",
                    serde_json::json!({ "session_id": sid }),
                );
                state.browser_session_id = None;
                state.set_info("Browser session terminated.");
            }
        }
    });

    if state.browser_mode == "tor" {
        ui.colored_label(
            Color32::from_rgb(100, 200, 255),
            "🔒 Tor Routing Enabled — Remote SOCKS5h DNS enforced; fails closed if Tor is unreachable.",
        );
    } else {
        ui.colored_label(
            Color32::from_rgb(255, 180, 80),
            "⚠️ Direct Networking Active — Local IP visible to remote servers. No onion routing.",
        );
    }

    ui.separator();
    ui.add_space(8.0);

    // URL Navigation Bar
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label("Target URL:");
            ui.add(
                egui::TextEdit::singleline(&mut state.browser_url)
                    .desired_width(420.0)
                    .hint_text("https://example.com or http://*.onion"),
            );

            let can_navigate =
                state.browser_session_id.is_some() && !state.browser_url.trim().is_empty();

            if ui
                .add_enabled(can_navigate, egui::Button::new("Navigate"))
                .clicked()
            {
                let sid = state.browser_session_id.as_ref().unwrap().clone();
                let url = state.browser_url.trim().to_string();

                let params = serde_json::json!({
                    "session_id": sid,
                    "url": url,
                });

                match client.call("browser", "browser.navigate", params) {
                    Ok(val) => {
                        let status_code = val["status_code"].as_u64().unwrap_or(0) as u16;
                        let mode = val["routing_mode"]
                            .as_str()
                            .unwrap_or("unknown")
                            .to_string();
                        let body_bytes = val["body"]
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_u64())
                                    .map(|b| b as u8)
                                    .collect::<Vec<u8>>()
                            })
                            .unwrap_or_default();
                        let body_preview = String::from_utf8_lossy(&body_bytes).to_string();

                        let mut headers_vec = Vec::new();
                        if let Some(h_obj) = val["headers"].as_object() {
                            for (k, v) in h_obj {
                                if let Some(v_str) = v.as_str() {
                                    headers_vec.push((k.clone(), v_str.to_string()));
                                }
                            }
                        }

                        state.browser_history.push(BrowserNavigationEntry {
                            url,
                            mode,
                            status_code,
                            headers: headers_vec,
                            body_preview,
                            captured: false,
                            raw_result: val,
                        });
                        state.set_info("Navigation succeeded.");
                    }
                    Err(e) => {
                        state.set_error(format!("Navigation Failed (Fail-Closed): {e}"));
                    }
                }
            }
        });
    });

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Navigation History & Captured Artifacts:").strong());
        if !state.browser_history.is_empty() && ui.button("Clear History").clicked() {
            state.browser_history.clear();
        }
    });

    let sid_opt = state.browser_session_id.clone();
    let mut capture_info = None;
    let mut capture_error = None;

    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .show(ui, |ui| {
            for (idx, entry) in state.browser_history.iter_mut().enumerate() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("[{}] {}", idx + 1, entry.url)).strong());
                        ui.label(format!("HTTP {}", entry.status_code));
                        ui.label(format!("Mode: {}", entry.mode));

                        if entry.captured {
                            ui.label(
                                RichText::new("[EVIDENCE STORED]")
                                    .color(Color32::GREEN)
                                    .strong(),
                            );
                        } else if ui.button("Capture as Web Evidence").clicked() {
                            if let Some(sid) = &sid_opt {
                                let cap_params = serde_json::json!({
                                    "session_id": sid,
                                    "result": entry.raw_result,
                                    "artifact": "page_content"
                                });
                                match client.call("browser", "browser.evidence.capture", cap_params)
                                {
                                    Ok(res) => {
                                        entry.captured = true;
                                        let addr = res["address"].as_str().unwrap_or("unknown");
                                        capture_info = Some(format!(
                                            "Stored web page to evidence blob {addr}."
                                        ));
                                    }
                                    Err(e) => capture_error = Some(format!("Capture failed: {e}")),
                                }
                            }
                        }
                    });

                    if !entry.body_preview.is_empty() {
                        let preview_slice = if entry.body_preview.len() > 300 {
                            format!("{}...", &entry.body_preview[..300])
                        } else {
                            entry.body_preview.clone()
                        };
                        ui.add(egui::Label::new(RichText::new(preview_slice).monospace()));
                    }
                });
            }
        });

    if let Some(info) = capture_info {
        state.set_info(info);
    }
    if let Some(err) = capture_error {
        state.set_error(err);
    }
}
