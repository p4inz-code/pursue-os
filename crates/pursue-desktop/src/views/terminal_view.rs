//! Investigation Terminal graphical console view.

use egui::{Color32, RichText, Ui};

use crate::client::IpcClient;
use crate::state::{DesktopState, TerminalConsoleEntry};

/// Renders the Investigation Terminal view tab.
pub fn render(ui: &mut Ui, state: &mut DesktopState, client: &dyn IpcClient) {
    ui.heading("Investigation Terminal Console");
    ui.add_space(8.0);

    // Active session status bar
    ui.horizontal(|ui| {
        let case_str = state
            .active_case
            .as_ref()
            .map(|c| c.id.as_str())
            .unwrap_or("NONE");
        ui.label(format!("Active Case: [{case_str}]"));

        ui.separator();

        let sess_str = state
            .terminal_session_id
            .as_deref()
            .unwrap_or("Not Initialized");
        ui.label(format!("Session: [{sess_str}]"));

        if state.terminal_session_id.is_none() {
            if state.active_case.is_some() {
                if ui.button("Initialize Terminal Session").clicked() {
                    let ts = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let sid = format!("term-{}-{}", std::process::id(), ts);
                    let cid = state.active_case.as_ref().unwrap().id.clone();
                    let actor = state.investigator_id.clone();

                    match client.call(
                        "terminal",
                        "session.create",
                        serde_json::json!({
                            "session_id": sid,
                            "case_id": cid,
                            "actor": actor,
                        }),
                    ) {
                        Ok(_) => {
                            state.terminal_session_id = Some(sid);
                            state.set_info("Terminal session established over IPC.");
                        }
                        Err(e) => state.set_error(format!("Failed to start session: {e}")),
                    }
                }
            } else {
                ui.label(
                    RichText::new(
                        "Select an active case in the Cases tab to initialize terminal session.",
                    )
                    .color(Color32::from_rgb(255, 180, 80))
                    .italics(),
                );
            }
        } else if ui.button("Terminate Session").clicked() {
            if let Some(sid) = state.terminal_session_id.clone() {
                let _ = client.call(
                    "terminal",
                    "session.terminate",
                    serde_json::json!({ "session_id": sid }),
                );
                state.terminal_session_id = None;
                state.set_info("Terminal session terminated.");
            }
        }
    });

    ui.separator();
    ui.add_space(8.0);

    // Command Input Bar
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label("Executable:");
            ui.add(
                egui::TextEdit::singleline(&mut state.terminal_program)
                    .hint_text("e.g. uname, whoami, dig, ip"),
            );

            ui.label("Arguments:");
            ui.add(
                egui::TextEdit::singleline(&mut state.terminal_args)
                    .desired_width(260.0)
                    .hint_text("e.g. -a, --help"),
            );

            let can_run =
                state.terminal_session_id.is_some() && !state.terminal_program.trim().is_empty();

            if ui
                .add_enabled(can_run, egui::Button::new("Execute"))
                .clicked()
            {
                let sid = state.terminal_session_id.as_ref().unwrap().clone();
                let prog = state.terminal_program.trim().to_string();
                let args: Vec<String> = state
                    .terminal_args
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();

                let params = serde_json::json!({
                    "session_id": sid,
                    "program": prog,
                    "args": args,
                });

                match client.call("terminal", "command.execute", params) {
                    Ok(val) => {
                        let stdout_bytes = val["stdout"]
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_u64())
                                    .map(|b| b as u8)
                                    .collect::<Vec<u8>>()
                            })
                            .unwrap_or_default();
                        let stderr_bytes = val["stderr"]
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|v| v.as_u64())
                                    .map(|b| b as u8)
                                    .collect::<Vec<u8>>()
                            })
                            .unwrap_or_default();

                        let stdout_str = String::from_utf8_lossy(&stdout_bytes).to_string();
                        let stderr_str = String::from_utf8_lossy(&stderr_bytes).to_string();
                        let exit_code = val["exit_code"].as_i64().map(|c| c as i32);
                        let timed_out = val["timed_out"].as_bool().unwrap_or(false);
                        let truncated = val["truncated"].as_bool().unwrap_or(false);

                        state.terminal_history.push(TerminalConsoleEntry {
                            program: prog,
                            args,
                            stdout: stdout_str,
                            stderr: stderr_str,
                            exit_code,
                            timed_out,
                            truncated,
                            captured: false,
                            raw_result: val,
                        });
                        state.set_info("Command completed.");
                    }
                    Err(e) => state.set_error(format!("Execution failed: {e}")),
                }
            }
        });
    });

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(RichText::new("Execution History & Evidence:").strong());
        if !state.terminal_history.is_empty() && ui.button("Clear Console History").clicked() {
            state.terminal_history.clear();
        }
    });

    let sid_opt = state.terminal_session_id.clone();
    let mut capture_info = None;
    let mut capture_error = None;

    egui::ScrollArea::vertical()
        .stick_to_bottom(true)
        .show(ui, |ui| {
            for (idx, entry) in state.terminal_history.iter_mut().enumerate() {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "[{}] $ {} {}",
                                idx + 1,
                                entry.program,
                                entry.args.join(" ")
                            ))
                            .monospace()
                            .strong(),
                        );

                        if let Some(code) = entry.exit_code {
                            if code == 0 {
                                ui.label(RichText::new("exit 0").color(Color32::GREEN));
                            } else {
                                ui.label(RichText::new(format!("exit {code}")).color(Color32::RED));
                            }
                        } else if entry.timed_out {
                            ui.label(RichText::new("[TIMED OUT]").color(Color32::RED).strong());
                        }

                        if entry.truncated {
                            ui.label(RichText::new("[TRUNCATED]").color(Color32::KHAKI));
                        }

                        if entry.captured {
                            ui.label(
                                RichText::new("[EVIDENCE STORED]")
                                    .color(Color32::GREEN)
                                    .strong(),
                            );
                        } else if ui.button("Capture as Evidence").clicked() {
                            if let Some(sid) = &sid_opt {
                                let cap_params = serde_json::json!({
                                    "session_id": sid,
                                    "result": entry.raw_result,
                                    "stream": "stdout"
                                });
                                match client.call("terminal", "evidence.capture", cap_params) {
                                    Ok(res) => {
                                        entry.captured = true;
                                        let addr = res["address"].as_str().unwrap_or("unknown");
                                        capture_info = Some(format!(
                                            "Captured stdout to evidence blob {addr}."
                                        ));
                                    }
                                    Err(e) => capture_error = Some(format!("Capture failed: {e}")),
                                }
                            }
                        }
                    });

                    if !entry.stdout.is_empty() {
                        ui.add(egui::Label::new(RichText::new(&entry.stdout).monospace()));
                    }
                    if !entry.stderr.is_empty() {
                        ui.add(egui::Label::new(
                            RichText::new(&entry.stderr)
                                .monospace()
                                .color(Color32::LIGHT_RED),
                        ));
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
