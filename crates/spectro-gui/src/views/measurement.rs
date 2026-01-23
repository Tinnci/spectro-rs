use eframe::egui;
use spectro_rs::MeasurementMode;
use std::time::{Duration, Instant};

use crate::app::SpectroApp;
use crate::components::history::render_history;
use crate::components::reference::{ReferenceContext, render_reference_window};
use crate::components::settings::{
    DebugSettingsContext, SettingsContext, render_debug_settings_window, render_settings_window,
};
use crate::shared::DeviceCommand;
use crate::t;
use crate::theme::{disconnected_color, panel_bg_color, success_color};
use crate::views::expert::{ExpertViewContext, render_expert_workspace};
use crate::views::simple::{SimpleViewContext, render_simple_workspace};

/// Render the main measurement view (Simple/Expert modes)
pub fn render_measurement_view(app: &mut SpectroApp, ctx: &egui::Context) {
    // === Dynamic Window Size Management ===
    let mut min_width = app.theme_config.layout.window_min_width;
    let min_height = app.theme_config.layout.window_min_height;

    if app.is_expert_mode {
        if app.show_history_panel && !app.show_history_detached {
            min_width += app.theme_config.layout.history_min_width;
        }
        if app.inspector.visible && !app.inspector.is_detached {
            min_width += app.theme_config.layout.inspector_min_width;
        }
    }

    ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
        min_width, min_height,
    )));

    // === Handle continuous measurement ===
    if app.is_continuous && app.is_connected && !app.is_busy {
        let should_measure = match app.last_measurement_time {
            None => true,
            Some(last_time) => {
                last_time.elapsed() >= Duration::from_secs_f32(app.continuous_interval)
            }
        };

        if should_measure {
            app.cmd_tx
                .send(DeviceCommand::Measure(app.selected_mode))
                .ok();
            app.last_measurement_time = Some(Instant::now());
            app.is_busy = true;
        }
    }

    // === Top Panel: Branding & Mode Switch ===
    egui::TopBottomPanel::top("top_panel")
        .frame(
            egui::Frame::none()
                .fill(panel_bg_color(&ctx.style().visuals))
                .inner_margin(egui::Margin::symmetric(12.0, 8.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Logo/Title
                ui.label(
                    egui::RichText::new(format!("🌈 {}", "spectro-rs"))
                        .size(20.0)
                        .strong(),
                );

                ui.separator();

                // Device status
                if app.is_connected {
                    ui.colored_label(success_color(&ui.ctx().style().visuals), "●");
                    if let Some(ref info) = app.device_info.basic {
                        ui.label(format!("{} ({})", info.model, info.serial));
                    }
                } else {
                    ui.colored_label(disconnected_color(&ui.ctx().style().visuals), "●");
                    ui.label(t!("gui-not-connected"));
                }

                // Right-aligned controls
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Theme toggle
                    if ui.button(app.theme_config.mode.label()).clicked() {
                        app.theme_config.mode = app.theme_config.mode.next();
                        app.theme_dirty = true;
                    }

                    ui.separator();

                    // Expert mode toggle
                    let toggle_text = if app.is_expert_mode {
                        format!("🔬 {}", t!("gui-expert"))
                    } else {
                        format!("🎨 {}", t!("gui-simple"))
                    };
                    if ui
                        .selectable_label(app.is_expert_mode, toggle_text)
                        .clicked()
                    {
                        app.is_expert_mode = !app.is_expert_mode;
                    }

                    if app.is_expert_mode {
                        ui.separator();

                        // Inspector toggle
                        let inspector_btn = if app.inspector.visible {
                            egui::RichText::new("🔍").strong()
                        } else {
                            egui::RichText::new("🔍").weak()
                        };
                        if ui
                            .selectable_label(app.inspector.visible, inspector_btn)
                            .on_hover_text(t!("gui-device-inspector"))
                            .clicked()
                        {
                            app.inspector.toggle();
                        }

                        // History toggle
                        let history_btn = if app.show_history_panel {
                            egui::RichText::new("📋").strong()
                        } else {
                            egui::RichText::new("📋").weak()
                        };
                        if ui
                            .selectable_label(app.show_history_panel, history_btn)
                            .on_hover_text(t!("gui-history-title"))
                            .clicked()
                        {
                            app.show_history_panel = !app.show_history_panel;
                        }
                    }

                    ui.separator();

                    // Settings button
                    if ui.button(format!("⚙ {}", t!("gui-settings"))).clicked() {
                        app.show_settings = !app.show_settings;
                    }

                    ui.separator();

                    // Status message
                    if app.is_busy {
                        ui.spinner();
                    }
                    ui.label(&app.status_msg);
                });
            });
        });

    // === Bottom Panel: Action Bar ===
    egui::TopBottomPanel::bottom("bottom_panel")
        .frame(
            egui::Frame::none()
                .fill(panel_bg_color(&ctx.style().visuals))
                .inner_margin(egui::Margin::symmetric(12.0, 8.0)),
        )
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Mode selector
                egui::ComboBox::from_id_salt("mode_selector")
                    .selected_text(match app.selected_mode {
                        MeasurementMode::Reflective => format!("📄 {}", t!("gui-reflective")),
                        MeasurementMode::Emissive => format!("🖥️ {}", t!("gui-emissive")),
                        MeasurementMode::Ambient => format!("💡 {}", t!("gui-ambient")),
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut app.selected_mode,
                            MeasurementMode::Reflective,
                            format!("📄 {}", t!("gui-reflective")),
                        );
                        ui.selectable_value(
                            &mut app.selected_mode,
                            MeasurementMode::Emissive,
                            format!("🖥️ {}", t!("gui-emissive")),
                        );
                        ui.selectable_value(
                            &mut app.selected_mode,
                            MeasurementMode::Ambient,
                            format!("💡 {}", t!("gui-ambient")),
                        );
                    });

                ui.separator();

                // Main action buttons
                let measure_btn = ui.add_enabled(
                    !app.is_busy && app.is_connected,
                    egui::Button::new(format!("🚀 {}", t!("gui-measure")))
                        .min_size(egui::vec2(100.0, 30.0)),
                );
                if measure_btn.clicked() {
                    app.is_busy = true;
                    app.cmd_tx
                        .send(DeviceCommand::Measure(app.selected_mode))
                        .ok();
                }

                let cal_btn = ui.add_enabled(
                    !app.is_busy && app.is_connected,
                    egui::Button::new(format!("🎯 {}", t!("gui-calibrate")))
                        .min_size(egui::vec2(100.0, 30.0)),
                );
                if cal_btn.clicked() {
                    app.calibration_wizard.start();
                }

                // Continuous measurement toggle
                let continuous_label = if app.is_continuous {
                    format!("⏸️ {}", t!("gui-stop-loop"))
                } else {
                    format!("▶️ {}", t!("gui-continuous"))
                };
                if ui
                    .add_enabled(
                        app.is_connected,
                        egui::Button::new(continuous_label).min_size(egui::vec2(120.0, 30.0)),
                    )
                    .clicked()
                {
                    app.is_continuous = !app.is_continuous;
                    app.last_measurement_time = None;
                }

                // Continuous interval slider
                if app.is_continuous {
                    ui.add(
                        egui::Slider::new(&mut app.continuous_interval, 0.5..=5.0)
                            .text(t!("gui-interval"))
                            .step_by(0.1),
                    );
                }

                // Reconnect button (only shown when disconnected)
                if !app.is_connected && ui.button(format!("🔌 {}", t!("gui-reconnect"))).clicked()
                {
                    app.is_busy = true;
                    app.cmd_tx.send(DeviceCommand::Connect).ok();
                }

                ui.separator();

                // Calibration status indicator
                let (cal_color, cal_text) = if app.is_calibrated {
                    (
                        success_color(&ctx.style().visuals),
                        format!("✓ {}", t!("gui-calibrated")),
                    )
                } else {
                    (
                        egui::Color32::from_rgb(255, 193, 7),
                        format!("⚠ {}", t!("gui-needs-calibration")),
                    )
                };
                ui.colored_label(cal_color, cal_text);

                // Right side: Reference input toggle
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(if app.reference_lab.is_some() {
                            format!("📌 {}", t!("gui-reference-set"))
                        } else {
                            format!("📌 {}", t!("gui-set-reference"))
                        })
                        .clicked()
                    {
                        app.show_reference_input = !app.show_reference_input;
                    }
                });
            });
        });

    // === Inspector Panel ===
    if app.is_expert_mode && app.inspector.visible {
        let ctx_clone = ctx.clone();
        egui::SidePanel::right("inspector_panel")
            .resizable(true)
            .default_width(app.theme_config.layout.inspector_default_width)
            .show(&ctx_clone, |ui| {
                app.inspector.render(
                    ui,
                    &crate::inspector::InspectorContext {
                        device_info: &app.device_info,
                        is_connected: app.is_connected,
                        is_calibrated: app.is_calibrated,
                        selected_mode: app.selected_mode,
                        last_result: app.last_result.as_ref(),
                        last_tm30: app.last_tm30.as_ref(),
                        history: &app.measurement_history,
                        layout: &app.theme_config.layout,
                    },
                );
            });
    }

    // === History Panel ===
    let mut history_action = crate::components::history::HistoryAction::None;
    if app.is_expert_mode && app.show_history_panel {
        let ctx_clone = ctx.clone();
        egui::SidePanel::left("history_panel")
            .resizable(true)
            .default_width(app.theme_config.layout.history_default_width)
            .show(&ctx_clone, |ui| {
                history_action = render_history(
                    ui,
                    &crate::components::history::HistoryContext {
                        history: &app.measurement_history,
                        delta_e_tolerance: app.delta_e_tolerance,
                        layout: &app.theme_config.layout,
                        is_detached: app.show_history_detached,
                    },
                );
            });
    }

    match history_action {
        crate::components::history::HistoryAction::None => {}
        crate::components::history::HistoryAction::Clear => {
            app.measurement_history.clear();
        }
        crate::components::history::HistoryAction::ExportCsv => app.export_history_csv(),
        crate::components::history::HistoryAction::ExportJson => app.export_history_json(),
        crate::components::history::HistoryAction::ExportCgats => app.export_history_cgats(),
        crate::components::history::HistoryAction::Close => {
            app.show_history_panel = false;
        }
        crate::components::history::HistoryAction::Detach => {
            app.show_history_detached = true;
        }
        crate::components::history::HistoryAction::Attach => {
            app.show_history_detached = false;
        }
    }

    // === Central Workspace ===
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(app.theme_config.adjusted_bg_color(ctx)))
        .show(ctx, |ui| {
            if app.is_expert_mode {
                render_expert_workspace(
                    ui,
                    &ExpertViewContext {
                        last_result: app.last_result.as_ref(),
                        layout: &app.theme_config.layout,
                    },
                );
            } else {
                render_simple_workspace(
                    ui,
                    &SimpleViewContext {
                        last_result: app.last_result.as_ref(),
                        reference_lab: app.reference_lab,
                        delta_e_tolerance: app.delta_e_tolerance,
                        layout: &app.theme_config.layout,
                    },
                );
            }
        });

    // === Render Modal Windows ===

    // Settings Window
    render_settings_window(
        ctx,
        &mut SettingsContext {
            show: &mut app.show_settings,
            show_debug: &mut app.show_debug_settings,
            selected_illuminant: &mut app.selected_illuminant,
            selected_observer: &mut app.selected_observer,
            theme_config: &mut app.theme_config,
            dirty: &mut app.theme_dirty,
        },
    );

    // Debug Settings Window
    render_debug_settings_window(
        ctx,
        &mut DebugSettingsContext {
            show: &mut app.show_debug_settings,
            theme_config: &mut app.theme_config,
            dirty: &mut app.theme_dirty,
        },
    );

    // Reference Input Window
    let current_lab = app.get_current_lab();
    render_reference_window(
        ctx,
        &mut ReferenceContext {
            show: &mut app.show_reference_input,
            reference_lab: &mut app.reference_lab,
            delta_e_tolerance: &mut app.delta_e_tolerance,
            ref_input_l: &mut app.ref_input_l,
            ref_input_a: &mut app.ref_input_a,
            ref_input_b: &mut app.ref_input_b,
            current_lab,
        },
    );

    // Calibration Wizard (Modal)
    app.calibration_wizard.render(
        ctx,
        &app.cmd_tx,
        &mut app.is_busy,
        &app.status_msg,
        &app.theme_config.layout,
    );

    // Mode Guidance reminder (if we're busy measuring and not in the wizard)
    if app.is_busy && !app.calibration_wizard.show && !app.status_msg.contains("Calibrate") {
        let highlight = match app.selected_mode {
            spectro_rs::MeasurementMode::Reflective => "REFLECTIVE",
            spectro_rs::MeasurementMode::Emissive => "EMISSIVE",
            spectro_rs::MeasurementMode::Ambient => "AMBIENT",
        };
        crate::calibration::CalibrationWizard::render_dial_check(
            ctx,
            highlight,
            &app.theme_config.layout,
        );
    }
}
