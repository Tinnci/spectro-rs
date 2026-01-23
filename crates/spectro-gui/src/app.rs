//! Graphical User Interface for spectro-rs.
//!
//! This module implements the main application window using the [`eframe`] framework.
//! It features a **Simple/Expert dual-mode** design:
//!
//! - **Simple Mode**: Large color swatch, Pass/Fail display, key metrics only.
//! - **Expert Mode**: Full spectral plot, EEPROM data viewer, raw sensor values.

use crossbeam_channel::{Receiver, Sender, unbounded};
use eframe::egui;
use spectro_rs::{Illuminant, MeasurementMode, Observer, colorimetry::Lab};
use std::time::{Duration, Instant};

use crate::backend;
use crate::calibration::CalibrationWizard;
use crate::components::history::{HistoryAction, HistoryContext, render_history};
use crate::components::reference::{ReferenceContext, render_reference_window};
use crate::components::settings::{
    DebugSettingsContext, SettingsContext, render_debug_settings_window, render_settings_window,
};
use crate::inspector::{DeviceInspector, InspectorContext};
use crate::shared::{DeviceCommand, ExtendedDeviceInfo, MeasurementEntry, UIUpdate};
use crate::t;
use crate::theme::{
    ThemeConfig, disconnected_color, panel_bg_color, panel_bg_dark_color, success_color,
};
use crate::views::expert::{ExpertViewContext, render_expert_workspace};
use crate::views::simple::{SimpleViewContext, render_simple_workspace};

// ============================================================================
// Application State
// ============================================================================

pub struct SpectroApp {
    // Communication
    cmd_tx: Sender<DeviceCommand>,
    update_rx: Receiver<UIUpdate>,

    // Device State
    device_info: ExtendedDeviceInfo,
    is_connected: bool,
    status_msg: String,
    is_busy: bool,
    is_calibrated: bool,

    // Measurement State
    selected_mode: MeasurementMode,
    last_result: Option<spectro_rs::spectrum::MeasurementResult>,
    last_tm30: Option<spectro_rs::tm30::TM30Metrics>,
    measurement_history: Vec<MeasurementEntry>,

    // Reference/Standard for comparison
    reference_lab: Option<Lab>,
    delta_e_tolerance: f32,

    // Reference input dialog state
    ref_input_l: f32,
    ref_input_a: f32,
    ref_input_b: f32,

    // UI State
    is_expert_mode: bool,
    show_reference_input: bool,
    show_settings: bool,
    show_debug_settings: bool,
    show_history_panel: bool,
    show_history_detached: bool,
    inspector: DeviceInspector,

    // Theme and UX
    theme_config: ThemeConfig,
    theme_dirty: bool,

    // Continuous measurement
    is_continuous: bool,
    continuous_interval: f32, // seconds
    last_measurement_time: Option<Instant>,

    // Algorithm calculation settings
    selected_illuminant: Illuminant,
    selected_observer: Observer,

    // Calibration wizard (extracted component)
    calibration_wizard: CalibrationWizard,
}

// ============================================================================
// Application Implementation
// ============================================================================

impl SpectroApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load theme configuration
        let theme_config = ThemeConfig::load_or_default("spectro_theme.json");
        theme_config.apply_to_ctx(&cc.egui_ctx);

        let (cmd_tx, cmd_rx) = unbounded();
        let (update_tx, update_rx) = unbounded();

        // Spawn the hardware worker thread
        backend::spawn_backend_thread(cmd_rx, update_tx);

        // Auto-connect on startup
        cmd_tx.send(DeviceCommand::Connect).ok();

        Self {
            cmd_tx,
            update_rx,
            device_info: ExtendedDeviceInfo::default(),
            is_connected: false,
            status_msg: "🚀 Initializing...".into(),
            is_busy: false,
            is_calibrated: false,
            selected_mode: MeasurementMode::Reflective,
            last_result: None,
            last_tm30: None,
            measurement_history: Vec::new(),
            reference_lab: None,
            delta_e_tolerance: 2.0,
            ref_input_l: 50.0,
            ref_input_a: 0.0,
            ref_input_b: 0.0,
            is_expert_mode: false,
            show_reference_input: false,
            show_settings: false,
            show_debug_settings: false,
            show_history_panel: true,
            show_history_detached: false,
            inspector: DeviceInspector::new(),
            theme_config,
            theme_dirty: false,
            is_continuous: false,
            continuous_interval: 2.0,
            last_measurement_time: None,
            selected_illuminant: Illuminant::D65,
            selected_observer: Observer::CIE1931_2,
            calibration_wizard: CalibrationWizard::new(),
        }
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    fn get_current_lab(&self) -> Option<Lab> {
        self.last_result.as_ref().map(|res| res.lab)
    }

    fn calculate_delta_e(&self, lab: &Lab) -> Option<f32> {
        self.reference_lab
            .as_ref()
            .map(|ref_lab| lab.delta_e_2000(ref_lab))
    }

    fn calculate_delta_e_76(&self, lab: &Lab) -> Option<f32> {
        self.reference_lab
            .as_ref()
            .map(|ref_lab| lab.delta_e_76(ref_lab))
    }

    fn add_to_history(&mut self, result: spectro_rs::spectrum::MeasurementResult) {
        let lab = result.lab;
        let delta_e = self
            .reference_lab
            .as_ref()
            .map(|ref_lab| lab.delta_e_2000(ref_lab));

        let entry = MeasurementEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            mode: self.selected_mode,
            result, // Using the new consolidated result
            delta_e,
        };

        self.measurement_history.insert(0, entry);
        // Keep only last 50 measurements
        if self.measurement_history.len() > 50 {
            self.measurement_history.pop();
        }
    }

    /// Export the measurement history to a CSV file.
    fn export_history_csv(&self) {
        if self.measurement_history.is_empty() {
            return;
        }

        let file_path = rfd::FileDialog::new()
            .add_filter("CSV File", &["csv"])
            .set_file_name("measurements.csv")
            .save_file();

        if let Some(path) = file_path {
            let mut csv = String::from("Timestamp,Mode,L*,a*,b*,DeltaE\n");
            for entry in &self.measurement_history {
                csv.push_str(&format!(
                    "{},{:?},{:.4},{:.4},{:.4},{}\n",
                    entry.timestamp,
                    entry.mode,
                    entry.result.lab.l,
                    entry.result.lab.a,
                    entry.result.lab.b,
                    entry.delta_e.map(|e| e.to_string()).unwrap_or_default()
                ));
            }

            if let Err(e) = std::fs::write(path, csv) {
                eprintln!("Failed to write CSV: {}", e);
            }
        }
    }

    /// Export the measurement history to a JSON file.
    fn export_history_json(&self) {
        if self.measurement_history.is_empty() {
            return;
        }

        let file_path = rfd::FileDialog::new()
            .add_filter("JSON File", &["json"])
            .set_file_name("measurements.json")
            .save_file();

        if let Some(path) = file_path
            && let Ok(json) = serde_json::to_string_pretty(&self.measurement_history)
            && let Err(e) = std::fs::write(path, json)
        {
            eprintln!("Failed to write JSON: {}", e);
        }
    }

    // NOTE: render_device_dial and render_calibration_wizard have been
    // extracted to crate::calibration::CalibrationWizard

    /// Export the measurement history to a CGATS (.ti3) file.
    fn export_history_cgats(&self) {
        if self.measurement_history.is_empty() {
            return;
        }

        let file_path = rfd::FileDialog::new()
            .add_filter("CGATS File", &["ti3", "txt"])
            .set_file_name("measurements.ti3")
            .save_file();

        if let Some(path) = file_path {
            let mut cgats = String::new();
            cgats.push_str("CTI3\n\n");
            cgats.push_str("DESCRIPTOR \"Argyll Device Measurement data\"\n");
            cgats.push_str("ORIGINATOR \"spectro-rs\"\n");
            cgats.push_str(&format!(
                "CREATED \"{}\"\n\n",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
            ));

            // Define fields: ID, Lab, XYZ, and Spectral data
            cgats.push_str("NUMBER_OF_FIELDS 47\n");
            cgats.push_str("BEGIN_DATA_FORMAT\n");
            cgats.push_str("SAMPLE_ID SAMPLE_NAME LAB_L LAB_A LAB_B XYZ_X XYZ_Y XYZ_Z ");
            for wl in (380..=780).step_by(10) {
                cgats.push_str(&format!("SPEC_{} ", wl));
            }
            cgats.push_str("\nEND_DATA_FORMAT\n\n");

            cgats.push_str(&format!(
                "NUMBER_OF_SETS {}\n",
                self.measurement_history.len()
            ));
            cgats.push_str("BEGIN_DATA\n");

            for (i, entry) in self.measurement_history.iter().enumerate() {
                cgats.push_str(&format!(
                    "{} \"{}\" {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} ",
                    i + 1,
                    entry.timestamp,
                    entry.result.lab.l,
                    entry.result.lab.a,
                    entry.result.lab.b,
                    entry.result.xyz.x,
                    entry.result.xyz.y,
                    entry.result.xyz.z
                ));

                for val in &entry.result.spectrum.values {
                    cgats.push_str(&format!("{:.6} ", val));
                }
                cgats.push('\n');
            }

            cgats.push_str("END_DATA\n");

            if let Err(e) = std::fs::write(path, cgats) {
                eprintln!("Failed to write CGATS: {}", e);
            }
        }
    }
}

// ============================================================================
// eframe::App Implementation
// ============================================================================

impl eframe::App for SpectroApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // === Unified Theme Application (SSoT) ===
        if self.theme_dirty {
            self.theme_config.apply_to_ctx(ctx);
            let _ = self.theme_config.save("spectro_theme.json");
            self.theme_dirty = false;
        }

        // Handle updates from hardware thread
        while let Ok(update) = self.update_rx.try_recv() {
            match update {
                UIUpdate::Connected(info) => {
                    self.device_info = info;
                    self.is_connected = true;
                    self.is_busy = false;
                }
                UIUpdate::Status(msg) => {
                    if msg.contains("Calibration successful") {
                        self.is_calibrated = true;
                        self.calibration_wizard.on_calibration_success();
                    }
                    self.status_msg = msg;
                    self.is_busy = false;
                }
                UIUpdate::Result(data, tm30) => {
                    self.add_to_history(data.clone());
                    self.last_result = Some(data);
                    self.last_tm30 = tm30.map(|b| *b);
                    self.is_busy = false;
                }
                UIUpdate::Error(err) => {
                    self.status_msg = err;
                    self.is_busy = false;
                    // Keep the wizard open so the user can see the error
                }
                UIUpdate::Disconnected => {
                    self.is_connected = false;
                    self.status_msg = "⚠️ Device disconnected".into();
                }
            }
        }

        // === Dynamic Window Size Management (SSoT) ===
        let mut min_width = self.theme_config.layout.window_min_width;
        let min_height = self.theme_config.layout.window_min_height;

        if self.is_expert_mode {
            if self.show_history_panel && !self.show_history_detached {
                min_width += self.theme_config.layout.history_min_width;
            }
            if self.inspector.visible && !self.inspector.is_detached {
                min_width += self.theme_config.layout.inspector_min_width;
            }
        }

        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(egui::vec2(
            min_width, min_height,
        )));

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
                    ui.label(egui::RichText::new("🌈 spectro-rs").size(20.0).strong());

                    ui.separator();

                    // Device status
                    if self.is_connected {
                        ui.colored_label(success_color(&ui.ctx().style().visuals), "●");
                        if let Some(ref info) = self.device_info.basic {
                            ui.label(format!("{} ({})", info.model, info.serial));
                        }
                    } else {
                        ui.colored_label(disconnected_color(&ui.ctx().style().visuals), "●");
                        ui.label(t!("gui-not-connected"));
                    }

                    // Right-aligned controls
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        // Theme toggle
                        if ui.button(self.theme_config.mode.label()).clicked() {
                            self.theme_config.mode = self.theme_config.mode.next();
                            self.theme_dirty = true;
                        }

                        ui.separator();

                        // Expert mode toggle
                        let toggle_text = if self.is_expert_mode {
                            format!("🔬 {}", t!("gui-expert"))
                        } else {
                            format!("🎨 {}", t!("gui-simple"))
                        };
                        if ui
                            .selectable_label(self.is_expert_mode, toggle_text)
                            .clicked()
                        {
                            self.is_expert_mode = !self.is_expert_mode;
                        }

                        if self.is_expert_mode {
                            ui.separator();

                            // Inspector toggle
                            let inspector_btn = if self.inspector.visible {
                                egui::RichText::new("🔍").strong()
                            } else {
                                egui::RichText::new("🔍").weak()
                            };
                            if ui
                                .selectable_label(self.inspector.visible, inspector_btn)
                                .on_hover_text(t!("gui-device-inspector"))
                                .clicked()
                            {
                                self.inspector.toggle();
                            }

                            // History toggle
                            let history_btn = if self.show_history_panel {
                                egui::RichText::new("📋").strong()
                            } else {
                                egui::RichText::new("📋").weak()
                            };
                            if ui
                                .selectable_label(self.show_history_panel, history_btn)
                                .on_hover_text(t!("gui-history-title"))
                                .clicked()
                            {
                                self.show_history_panel = !self.show_history_panel;
                            }
                        }

                        ui.separator();

                        // Settings button
                        if ui.button(format!("⚙ {}", t!("gui-settings"))).clicked() {
                            self.show_settings = !self.show_settings;
                        }

                        ui.separator();

                        // Status message
                        if self.is_busy {
                            ui.spinner();
                        }
                        ui.label(&self.status_msg);
                    });
                });
            });

        // === Handle continuous measurement ===
        if self.is_continuous && self.is_connected && !self.is_busy {
            let should_measure = match self.last_measurement_time {
                None => true,
                Some(last_time) => {
                    last_time.elapsed() >= Duration::from_secs_f32(self.continuous_interval)
                }
            };

            if should_measure {
                self.cmd_tx
                    .send(DeviceCommand::Measure(self.selected_mode))
                    .ok();
                self.last_measurement_time = Some(Instant::now());
                self.is_busy = true;
            }
        }

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
                        .selected_text(match self.selected_mode {
                            MeasurementMode::Reflective => format!("📄 {}", t!("gui-reflective")),
                            MeasurementMode::Emissive => format!("🖥️ {}", t!("gui-emissive")),
                            MeasurementMode::Ambient => format!("💡 {}", t!("gui-ambient")),
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.selected_mode,
                                MeasurementMode::Reflective,
                                format!("📄 {}", t!("gui-reflective")),
                            );
                            ui.selectable_value(
                                &mut self.selected_mode,
                                MeasurementMode::Emissive,
                                format!("🖥️ {}", t!("gui-emissive")),
                            );
                            ui.selectable_value(
                                &mut self.selected_mode,
                                MeasurementMode::Ambient,
                                format!("💡 {}", t!("gui-ambient")),
                            );
                        });

                    ui.separator();

                    // Main action buttons
                    let measure_btn = ui.add_enabled(
                        !self.is_busy && self.is_connected,
                        egui::Button::new(format!("🚀 {}", t!("gui-measure")))
                            .min_size(egui::vec2(100.0, 30.0)),
                    );
                    if measure_btn.clicked() {
                        self.is_busy = true;
                        self.cmd_tx
                            .send(DeviceCommand::Measure(self.selected_mode))
                            .ok();
                    }

                    let cal_btn = ui.add_enabled(
                        !self.is_busy && self.is_connected,
                        egui::Button::new(format!("🎯 {}", t!("gui-calibrate")))
                            .min_size(egui::vec2(100.0, 30.0)),
                    );
                    if cal_btn.clicked() {
                        self.calibration_wizard.start();
                    }

                    // Continuous measurement toggle
                    let continuous_label = if self.is_continuous {
                        format!("⏸️ {}", t!("gui-stop-loop"))
                    } else {
                        format!("▶️ {}", t!("gui-continuous"))
                    };
                    if ui
                        .add_enabled(
                            self.is_connected,
                            egui::Button::new(continuous_label).min_size(egui::vec2(120.0, 30.0)),
                        )
                        .clicked()
                    {
                        self.is_continuous = !self.is_continuous;
                        self.last_measurement_time = None;
                    }

                    // Continuous interval slider
                    if self.is_continuous {
                        ui.add(
                            egui::Slider::new(&mut self.continuous_interval, 0.5..=5.0)
                                .text(t!("gui-interval"))
                                .step_by(0.1),
                        );
                    }

                    // Reconnect button (only shown when disconnected)
                    if !self.is_connected
                        && ui.button(format!("🔌 {}", t!("gui-reconnect"))).clicked()
                    {
                        self.is_busy = true;
                        self.cmd_tx.send(DeviceCommand::Connect).ok();
                    }

                    ui.separator();

                    // Calibration status indicator
                    let (cal_color, cal_text) = if self.is_calibrated {
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
                            .button(if self.reference_lab.is_some() {
                                format!("📌 {}", t!("gui-reference-set"))
                            } else {
                                format!("📌 {}", t!("gui-set-reference"))
                            })
                            .clicked()
                        {
                            self.show_reference_input = !self.show_reference_input;
                        }
                    });
                });
            });

        // === Settings Window ===
        render_settings_window(
            ctx,
            &mut SettingsContext {
                show: &mut self.show_settings,
                show_debug: &mut self.show_debug_settings,
                selected_illuminant: &mut self.selected_illuminant,
                selected_observer: &mut self.selected_observer,
                theme_config: &mut self.theme_config,
                dirty: &mut self.theme_dirty,
            },
        );

        // === Debug Settings Window ===
        render_debug_settings_window(
            ctx,
            &mut DebugSettingsContext {
                show: &mut self.show_debug_settings,
                theme_config: &mut self.theme_config,
                dirty: &mut self.theme_dirty,
            },
        );

        // === Reference Input Window ===
        let current_lab = self.get_current_lab();
        render_reference_window(
            ctx,
            &mut ReferenceContext {
                show: &mut self.show_reference_input,
                reference_lab: &mut self.reference_lab,
                delta_e_tolerance: &mut self.delta_e_tolerance,
                ref_input_l: &mut self.ref_input_l,
                ref_input_a: &mut self.ref_input_a,
                ref_input_b: &mut self.ref_input_b,
                current_lab,
            },
        );

        // === Left Panel: History ===
        if self.is_expert_mode && self.show_history_panel {
            if self.show_history_detached {
                // Detached Viewport (Native Window)
                ctx.show_viewport_immediate(
                    egui::ViewportId::from_hash_of("history_viewport"),
                    egui::ViewportBuilder::default()
                        .with_title(t!("gui-history-title"))
                        .with_inner_size([self.theme_config.layout.history_min_width, 600.0]),
                    |ctx, class| {
                        if class == egui::ViewportClass::Root {
                            return;
                        }
                        egui::CentralPanel::default().show(ctx, |ui| {
                            let action = render_history(
                                ui,
                                &HistoryContext {
                                    history: &self.measurement_history,
                                    delta_e_tolerance: self.delta_e_tolerance,
                                    layout: &self.theme_config.layout,
                                    is_detached: true,
                                },
                            );
                            match action {
                                HistoryAction::Clear => self.measurement_history.clear(),
                                HistoryAction::ExportCsv => self.export_history_csv(),
                                HistoryAction::ExportJson => self.export_history_json(),
                                HistoryAction::ExportCgats => self.export_history_cgats(),
                                HistoryAction::Close => self.show_history_panel = false,
                                HistoryAction::Detach => {} // Already detached
                                HistoryAction::Attach => self.show_history_detached = false,
                                HistoryAction::None => {}
                            }
                        });

                        if ctx.input(|i| i.viewport().close_requested()) {
                            self.show_history_detached = false;
                        }
                    },
                );
            } else {
                // Embedded SidePanel
                egui::SidePanel::left("history_panel")
                    .resizable(true)
                    .default_width(self.theme_config.layout.history_default_width)
                    .min_width(self.theme_config.layout.history_min_width)
                    .max_width(250.0)
                    .show(ctx, |ui| {
                        let action = render_history(
                            ui,
                            &HistoryContext {
                                history: &self.measurement_history,
                                delta_e_tolerance: self.delta_e_tolerance,
                                layout: &self.theme_config.layout,
                                is_detached: false,
                            },
                        );
                        match action {
                            HistoryAction::Clear => self.measurement_history.clear(),
                            HistoryAction::ExportCsv => self.export_history_csv(),
                            HistoryAction::ExportJson => self.export_history_json(),
                            HistoryAction::ExportCgats => self.export_history_cgats(),
                            HistoryAction::Close => self.show_history_panel = false,
                            HistoryAction::Detach => self.show_history_detached = true,
                            HistoryAction::Attach => {} // Already attached
                            HistoryAction::None => {}
                        }
                    });
            }
        }

        // === Right Panel: Expert Inspector ===
        if self.is_expert_mode && self.inspector.visible {
            if self.inspector.is_detached {
                // Detached Viewport (Native Window)
                let mut is_detached = self.inspector.is_detached;
                ctx.show_viewport_immediate(
                    egui::ViewportId::from_hash_of("inspector_viewport"),
                    egui::ViewportBuilder::default()
                        .with_title(t!("gui-device-inspector"))
                        .with_inner_size([self.theme_config.layout.inspector_min_width, 600.0]),
                    |ctx, class| {
                        if class == egui::ViewportClass::Root {
                            return;
                        }
                        egui::CentralPanel::default().show(ctx, |ui| {
                            let insp_ctx = InspectorContext {
                                device_info: &self.device_info,
                                is_connected: self.is_connected,
                                is_calibrated: self.is_calibrated,
                                selected_mode: self.selected_mode,
                                last_result: self.last_result.as_ref(),
                                last_tm30: self.last_tm30.as_ref(),
                                history: &self.measurement_history,
                                layout: &self.theme_config.layout,
                            };
                            self.inspector.render(ui, &insp_ctx);
                        });

                        if ctx.input(|i| i.viewport().close_requested()) {
                            is_detached = false;
                        }
                    },
                );
                self.inspector.is_detached = is_detached;
            } else {
                // Embedded SidePanel
                egui::SidePanel::right("expert_panel")
                    .resizable(true)
                    .default_width(self.theme_config.layout.inspector_default_width)
                    .min_width(self.theme_config.layout.inspector_min_width)
                    .max_width(self.theme_config.layout.inspector_max_width)
                    .show(ctx, |ui| {
                        let ctx = InspectorContext {
                            device_info: &self.device_info,
                            is_connected: self.is_connected,
                            is_calibrated: self.is_calibrated,
                            selected_mode: self.selected_mode,
                            last_result: self.last_result.as_ref(),
                            last_tm30: self.last_tm30.as_ref(),
                            history: &self.measurement_history,
                            layout: &self.theme_config.layout,
                        };
                        self.inspector.render(ui, &ctx);
                    });
            }
        }

        // === Central Panel: Main Workspace ===
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(panel_bg_dark_color(&ctx.style().visuals))
                    .inner_margin(egui::Margin::symmetric(16.0, 0.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let available_width = ui.available_width();
                        let content_max_width = 1000.0_f32.min(available_width);

                        // Calculate horizontal offset to center the content block
                        let offset_x = (available_width - content_max_width) / 2.0;

                        // Create a centered content area with left-aligned children
                        ui.horizontal(|ui| {
                            ui.add_space(offset_x.max(0.0));
                            ui.vertical(|ui| {
                                ui.set_width(content_max_width);
                                ui.add_space(24.0);

                                if self.is_expert_mode {
                                    render_expert_workspace(
                                        ui,
                                        &ExpertViewContext {
                                            last_result: self.last_result.as_ref(),
                                            layout: &self.theme_config.layout,
                                        },
                                    );
                                } else {
                                    render_simple_workspace(
                                        ui,
                                        &SimpleViewContext {
                                            last_result: self.last_result.as_ref(),
                                            calculate_delta_e: Box::new(|lab| {
                                                self.calculate_delta_e(lab)
                                            }),
                                            calculate_delta_e_76: Box::new(|lab| {
                                                self.calculate_delta_e_76(lab)
                                            }),
                                            delta_e_tolerance: self.delta_e_tolerance,
                                            layout: &self.theme_config.layout,
                                        },
                                    );
                                }
                                ui.add_space(self.theme_config.layout.spacing * 4.0);
                            });
                        });
                    });

                // Calibration Wizard (extracted component)
                self.calibration_wizard.render(
                    ctx,
                    &self.cmd_tx,
                    &mut self.is_busy,
                    &self.status_msg,
                    &self.theme_config.layout,
                );

                // Mode Guidance reminder (if we're busy measuring and not in the wizard)
                if self.is_busy
                    && !self.calibration_wizard.show
                    && !self.status_msg.contains("Calibrate")
                {
                    let highlight = match self.selected_mode {
                        MeasurementMode::Reflective => "REFLECTIVE",
                        MeasurementMode::Emissive => "EMISSIVE",
                        MeasurementMode::Ambient => "AMBIENT",
                    };
                    CalibrationWizard::render_dial_check(ctx, highlight, &self.theme_config.layout);
                }
            });

        // Request continuous repaint for smooth animations
        ctx.request_repaint();
    }
}
