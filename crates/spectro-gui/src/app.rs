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
use std::time::Instant;

use crate::backend;
use crate::components::device_calibration::CalibrationWizard;
use crate::exporters::{self, HistoryExporter};
use crate::inspector::DeviceInspector;
use crate::shared::{DeviceCommand, ExtendedDeviceInfo, UIUpdate};
use crate::t;
use crate::theme::{ThemeConfig, panel_bg_dark_color};
use crate::views::display_calibration::{
    CalibrationAction, DisplayCalibrationContext, DisplayCalibrationView,
};
use crate::views::measurement::render_measurement_view;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AppView {
    #[default]
    Measurement,
    DisplayCalibration,
    Diagnostics,
}

// ============================================================================
// Application State
// ============================================================================

use crate::state::AppState;

pub struct SpectroApp {
    // Communication
    pub(crate) cmd_tx: Sender<DeviceCommand>,
    pub(crate) update_rx: Receiver<UIUpdate>,

    // Device State
    pub(crate) device_info: ExtendedDeviceInfo,
    pub(crate) is_connected: bool,
    pub(crate) status_msg: String,
    pub(crate) is_busy: bool,
    // Measurement State
    pub(crate) is_calibrated: bool,
    pub(crate) selected_mode: MeasurementMode,

    // Encapsulated Application State
    pub(crate) state: AppState,

    // UI State
    pub(crate) is_expert_mode: bool,
    pub(crate) show_reference_input: bool,
    pub(crate) show_settings: bool,
    pub(crate) show_debug_settings: bool,
    pub(crate) show_history_panel: bool,
    pub(crate) show_history_detached: bool,
    pub(crate) inspector: DeviceInspector,
    pub(crate) show_inspector: bool,
    pub(crate) diagnostics_report: Option<String>,

    // Theme and UX
    pub(crate) theme_config: ThemeConfig,
    pub(crate) theme_dirty: bool,
    pub(crate) current_view: AppView,

    // Continuous measurement
    pub(crate) is_continuous: bool,
    pub(crate) continuous_interval: f32, // seconds
    pub(crate) last_measurement_time: Option<Instant>,

    // Algorithm calculation settings
    pub(crate) selected_illuminant: Illuminant,
    pub(crate) selected_observer: Observer,

    // Calibration wizard (extracted component)
    pub(crate) calibration_wizard: CalibrationWizard,

    // Display Calibration State
    pub(crate) display_calibration: DisplayCalibrationView,
}

// ============================================================================
// Application Implementation
// ============================================================================

impl SpectroApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load theme configuration
        let theme_config = ThemeConfig::load_or_default("spectro_theme.json");

        // Initialize internationalization
        crate::i18n::init(theme_config.language);

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
            state: AppState::new(),
            is_expert_mode: false,
            show_reference_input: false,
            show_settings: false,
            show_debug_settings: false,
            show_history_panel: true,
            show_history_detached: false,
            inspector: DeviceInspector::new(),
            show_inspector: true,
            theme_config,
            theme_dirty: false,
            current_view: AppView::default(),
            is_continuous: false,
            continuous_interval: 2.0,
            last_measurement_time: None,
            selected_illuminant: Illuminant::D65,
            selected_observer: Observer::CIE1931_2,
            calibration_wizard: CalibrationWizard::new(),
            display_calibration: DisplayCalibrationView::default(),
            diagnostics_report: None,
        }
    }

    // ========================================================================
    // Helper Methods
    // ========================================================================

    fn export_with<T: HistoryExporter>(&self, exporter: T) {
        if self.state.history.is_empty() {
            return;
        }

        let file_path = rfd::FileDialog::new()
            .add_filter(exporter.name(), &exporter.extensions())
            .set_file_name(exporter.default_filename())
            .save_file();

        if let Some(path) = file_path
            && let Err(e) = exporter.export(&self.state.history, &path)
        {
            eprintln!("Failed to export {}: {}", exporter.name(), e);
        }
    }

    pub(crate) fn get_current_lab(&self) -> Option<Lab> {
        self.state.current_lab()
    }

    /// Export the measurement history to a CSV file.
    pub(crate) fn export_history_csv(&self) {
        self.export_with(exporters::CsvExporter);
    }

    /// Export the measurement history to a JSON file.
    pub(crate) fn export_history_json(&self) {
        self.export_with(exporters::JsonExporter);
    }

    // NOTE: render_device_dial and render_calibration_wizard have been
    // extracted to crate::calibration::CalibrationWizard

    /// Export the measurement history to a CGATS (.ti3) file.
    pub(crate) fn export_history_cgats(&self) {
        self.export_with(exporters::CgatsExporter);
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
                    let tm30_val = tm30.map(|b| *b);
                    if self.current_view == AppView::DisplayCalibration {
                        self.display_calibration.handle_measurement(data.xyz);
                    } else {
                        self.state
                            .add_measurement(data, tm30_val, self.selected_mode);
                    }
                    self.is_busy = false;
                }
                UIUpdate::Error(err) => {
                    // Critical Update: Forward specific calibration errors to the wizard UI
                    if self.calibration_wizard.state.show {
                        self.calibration_wizard.on_calibration_error(err.clone());
                    }
                    self.status_msg = err;
                    self.is_busy = false;
                }
                UIUpdate::Disconnected => {
                    self.is_connected = false;
                    self.status_msg = "⚠️ Device disconnected".into();
                }
                UIUpdate::TestResult(report) => {
                    self.diagnostics_report = Some(report);
                    self.is_busy = false;
                }
                UIUpdate::CharacterizationResult(csv) => {
                    self.diagnostics_report = Some(csv);
                    self.is_busy = false;
                }
            }
        }

        // === Sidebar Navigation ===
        egui::SidePanel::left("main_sidebar")
            .resizable(false)
            .exact_width(60.0)
            .frame(egui::Frame::none().fill(panel_bg_dark_color(&ctx.style().visuals)))
            .show(ctx, |ui| {
                ui.add_space(12.0);
                ui.vertical_centered(|ui| {
                    let btn_size = egui::vec2(40.0, 40.0);

                    // Measurement View
                    let is_measure = self.current_view == AppView::Measurement;
                    let measure_btn = ui
                        .add(
                            egui::Button::new(egui::RichText::new("📏").size(24.0))
                                .min_size(btn_size)
                                .selected(is_measure),
                        )
                        .on_hover_text(t!("gui-measure"));

                    if measure_btn.clicked() {
                        self.current_view = AppView::Measurement;
                    }

                    ui.add_space(12.0);

                    // Display Calibration View
                    let is_cal = self.current_view == AppView::DisplayCalibration;
                    let cal_btn = ui
                        .add(
                            egui::Button::new(egui::RichText::new("🖥️").size(24.0))
                                .min_size(btn_size)
                                .selected(is_cal),
                        )
                        .on_hover_text("Display Calibration");

                    if cal_btn.clicked() {
                        self.current_view = AppView::DisplayCalibration;
                    }

                    ui.add_space(12.0);

                    // Diagnostics View
                    let is_diag = self.current_view == AppView::Diagnostics;
                    let diag_btn = ui
                        .add(
                            egui::Button::new(egui::RichText::new("🩺").size(24.0))
                                .min_size(btn_size)
                                .selected(is_diag),
                        )
                        .on_hover_text("Sensor Diagnostics");

                    if diag_btn.clicked() {
                        self.current_view = AppView::Diagnostics;
                    }
                });
            });

        // === Main Content Area ===
        match self.current_view {
            AppView::Measurement => {
                render_measurement_view(self, ctx);
            }
            AppView::DisplayCalibration => {
                let cal_ctx = DisplayCalibrationContext {
                    layout: &self.theme_config.layout,
                    is_connected: self.is_connected,
                    is_busy: self.is_busy,
                };
                egui::CentralPanel::default().show(ctx, |ui| {
                    let action = self.display_calibration.render(ui, &cal_ctx);
                    match action {
                        CalibrationAction::RequestMeasurement => {
                            self.is_busy = true;
                            self.cmd_tx
                                .send(DeviceCommand::Measure(
                                    spectro_rs::MeasurementMode::Emissive,
                                ))
                                .ok();
                        }
                        CalibrationAction::None => {}
                    }
                });
            }
            AppView::Diagnostics => {
                crate::views::diagnostics::render_diagnostics_view(self, ctx);
            }
        }

        // Request continuous repaint for smooth animations
        ctx.request_repaint();
    }
}
