//! Graphical User Interface for spectro-rs.
//!
//! This module implements the main application window using the [`eframe`] framework.
//! It features a **Simple/Expert dual-mode** design:
//!
//! - **Simple Mode**: Large color swatch, Pass/Fail display, key metrics only.
//! - **Expert Mode**: Full spectral plot, EEPROM data viewer, raw sensor values.

use crossbeam_channel::{Receiver, Sender, unbounded};
use eframe::egui;
use egui_plot::{HLine, Legend, Line, Plot, PlotPoints, VLine};
use spectro_rs::{
    BoxedSpectrometer, Illuminant, MeasurementMode, Observer,
    colorimetry::{Lab, XYZ},
    discover,
    tm30::calculate_tm30,
};
use std::thread;
use std::time::{Duration, Instant};

use crate::calibration::CalibrationWizard;
use crate::inspector::{DeviceInspector, InspectorContext};
use crate::shared::{DeviceCommand, ExtendedDeviceInfo, MeasurementEntry, UIUpdate};
use crate::t;
use crate::theme::{
    ThemeConfig, border_color, disconnected_color, info_panel_color, muted_text_color,
    overlay_shadow_color, panel_bg_color, panel_bg_dark_color, success_color,
};

// ============================================================================
// UI Helpers
// ============================================================================

fn render_bento_item<R>(
    ui: &mut egui::Ui,
    title: String,
    min_width: f32,
    max_width: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let visuals = &ui.ctx().style().visuals;

    // Use a group to create a self-contained widget that respects horizontal_wrapped
    ui.scope(|ui| {
        ui.set_min_width(min_width);
        ui.set_max_width(max_width);

        egui::Frame::none()
            .fill(info_panel_color(visuals))
            .stroke(egui::Stroke::new(1.0, border_color(visuals)))
            .rounding(6.0)
            .inner_margin(egui::Margin::same(12.0))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(title.to_uppercase())
                            .size(10.0)
                            .color(muted_text_color(visuals))
                            .strong(),
                    );
                    ui.add_space(4.0);
                    add_contents(ui)
                })
                .inner
            })
            .inner
    })
    .inner
}

#[expect(
    dead_code,
    reason = "Utility icon for planned descriptive tooltips to reduce UI text density"
)]
fn help_icon(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new("ⓘ").color(muted_text_color(ui.visuals())))
        .on_hover_text(text);
}

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
    show_history_panel: bool,
    inspector: DeviceInspector,

    // Theme and UX
    theme_config: ThemeConfig,

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
        let visuals = theme_config.to_visuals();
        cc.egui_ctx.set_visuals(visuals);

        let (cmd_tx, cmd_rx) = unbounded();
        let (update_tx, update_rx) = unbounded();

        // Spawn the hardware worker thread
        thread::spawn(move || {
            let mut device: Option<BoxedSpectrometer> = None;

            while let Ok(cmd) = cmd_rx.recv() {
                match cmd {
                    DeviceCommand::Connect => {
                        update_tx
                            .send(UIUpdate::Status("🔍 Searching for device...".into()))
                            .ok();

                        match discover() {
                            Ok(d) => {
                                // Get basic device info
                                let basic_info = d.info().ok();

                                // Build extended device info
                                // Note: In a real implementation, we'd expose EEPROM data
                                // through the Spectrometer trait. For now, we use defaults.
                                let ext_info = ExtendedDeviceInfo {
                                    basic: basic_info,
                                    cal_version: Some(0x0100), // Placeholder
                                    white_ref: None,           // Would come from EEPROM
                                    emis_coef: None,
                                    amb_coef: None,
                                    lin_normal: None,
                                    lin_high: None,
                                };

                                device = Some(d);
                                update_tx.send(UIUpdate::Connected(ext_info)).ok();
                                update_tx
                                    .send(UIUpdate::Status(t!("gui-status-connected")))
                                    .ok();
                            }
                            Err(_e) => {
                                update_tx
                                    .send(UIUpdate::Error(t!("gui-error-no-device")))
                                    .ok();
                            }
                        }
                    }

                    DeviceCommand::Calibrate => {
                        if let Some(ref mut d) = device {
                            update_tx
                                .send(UIUpdate::Status(t!("gui-status-calibrating")))
                                .ok();

                            match d.calibrate() {
                                Ok(_) => {
                                    update_tx
                                        .send(UIUpdate::Status(t!("gui-status-calibration-ok")))
                                        .ok();
                                }
                                Err(_e) => {
                                    update_tx
                                        .send(UIUpdate::Error(t!("gui-error-calibration-failed")))
                                        .ok();
                                }
                            }
                        } else {
                            update_tx
                                .send(UIUpdate::Error(t!("gui-error-no-device-short")))
                                .ok();
                        }
                    }

                    DeviceCommand::Measure(mode) => {
                        if let Some(ref mut d) = device {
                            update_tx
                                .send(UIUpdate::Status(t!("gui-status-measuring")))
                                .ok();

                            match d.measure(mode) {
                                Ok(data) => {
                                    let tm30 = if mode == MeasurementMode::Emissive {
                                        Some(Box::new(calculate_tm30(&data)))
                                    } else {
                                        None
                                    };
                                    let result = data.to_result();
                                    update_tx.send(UIUpdate::Result(result, tm30)).ok();
                                    update_tx
                                        .send(UIUpdate::Status("✅ Measurement complete".into()))
                                        .ok();
                                }
                                Err(e) => {
                                    // Check if it's a USB error (device disconnected)
                                    let err_str = format!("{}", e);
                                    if err_str.contains("USB") || err_str.contains("timeout") {
                                        device = None;
                                        update_tx.send(UIUpdate::Disconnected).ok();
                                    }
                                    update_tx
                                        .send(UIUpdate::Error(t!("gui-error-measurement-failed")))
                                        .ok();
                                }
                            }
                        } else {
                            update_tx
                                .send(UIUpdate::Error(t!("gui-error-no-device-short")))
                                .ok();
                        }
                    }
                }
            }
        });

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
            show_history_panel: true,
            inspector: DeviceInspector::new(),
            theme_config,
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

    fn get_pass_fail(&self, delta_e: f32, ctx: &egui::Context) -> (bool, egui::Color32) {
        if delta_e <= self.delta_e_tolerance {
            (true, success_color(&ctx.style().visuals))
        } else {
            (false, egui::Color32::from_rgb(220, 53, 69)) // Red
        }
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

    // ========================================================================
    // Simple Mode Rendering
    // ========================================================================

    fn render_simple_workspace(&self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(20.0);

            if let Some(res) = &self.last_result {
                let (r, g, b) = (
                    (res.rgb.0 * 255.0) as u8,
                    (res.rgb.1 * 255.0) as u8,
                    (res.rgb.2 * 255.0) as u8,
                );
                let lab = res.lab;

                // === Giant Color Swatch ===
                let available_size = ui.available_size();
                let swatch_size = available_size.x.min(available_size.y * 0.5).min(300.0);

                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(swatch_size, swatch_size),
                    egui::Sense::hover(),
                );

                // Draw color swatch with rounded corners and shadow
                let painter = ui.painter();

                // Shadow
                painter.rect_filled(
                    rect.translate(egui::vec2(4.0, 4.0)),
                    16.0,
                    overlay_shadow_color(&ui.ctx().style().visuals),
                );

                // Main swatch
                painter.rect_filled(rect, 16.0, egui::Color32::from_rgb(r, g, b));

                // Border
                painter.rect_stroke(
                    rect,
                    16.0,
                    egui::Stroke::new(2.0, border_color(&ui.ctx().style().visuals)),
                );

                ui.add_space(20.0);

                // === Pass/Fail Indicator ===
                if let Some(delta_e) = self.calculate_delta_e(&lab) {
                    let (passed, color) = self.get_pass_fail(delta_e, ui.ctx());

                    let status_text = if passed { "✓ PASS" } else { "✗ FAIL" };
                    ui.colored_label(color, egui::RichText::new(status_text).size(48.0).strong());

                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new(format!("ΔE*00 = {:.2}", delta_e))
                            .size(24.0)
                            .color(muted_text_color(&ui.ctx().style().visuals)),
                    );

                    if let Some(delta_e_76) = self.calculate_delta_e_76(&lab) {
                        ui.label(
                            egui::RichText::new(format!("ΔE*76 = {:.2}", delta_e_76))
                                .size(14.0)
                                .color(egui::Color32::DARK_GRAY),
                        );
                    }

                    ui.add_space(5.0);
                    ui.label(
                        egui::RichText::new(format!("Tolerance: ≤ {:.1}", self.delta_e_tolerance))
                            .size(14.0)
                            .color(egui::Color32::DARK_GRAY),
                    );
                }

                ui.add_space(20.0);

                // === Key Metrics (Large Font) ===
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 150.0);

                    egui::Frame::none()
                        .fill(info_panel_color(&ui.ctx().style().visuals))
                        .rounding(8.0)
                        .inner_margin(egui::Margin::same(16.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new("L*")
                                            .size(14.0)
                                            .color(muted_text_color(&ui.ctx().style().visuals)),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("{:.1}", lab.l))
                                            .size(28.0)
                                            .strong(),
                                    );
                                });
                                ui.add_space(20.0);
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new("a*")
                                            .size(14.0)
                                            .color(muted_text_color(&ui.ctx().style().visuals)),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("{:.1}", lab.a))
                                            .size(28.0)
                                            .strong(),
                                    );
                                });
                                ui.add_space(20.0);
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new("b*")
                                            .size(14.0)
                                            .color(muted_text_color(&ui.ctx().style().visuals)),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!("{:.1}", lab.b))
                                            .size(28.0)
                                            .strong(),
                                    );
                                });
                            });
                        });
                });

                ui.add_space(20.0);

                // === sRGB Value ===
                ui.label(
                    egui::RichText::new(format!("sRGB: ({}, {}, {})", r, g, b))
                        .size(16.0)
                        .color(muted_text_color(&ui.ctx().style().visuals)),
                );
                ui.label(
                    egui::RichText::new(format!("#{:02X}{:02X}{:02X}", r, g, b))
                        .size(14.0)
                        .color(egui::Color32::DARK_GRAY)
                        .monospace(),
                );
            } else {
                // No measurement yet
                ui.add_space(100.0);
                ui.label(
                    egui::RichText::new("📷")
                        .size(64.0)
                        .color(egui::Color32::from_rgb(80, 80, 100)),
                );
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new("No measurement yet")
                        .size(20.0)
                        .color(muted_text_color(&ui.ctx().style().visuals)),
                );
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("Click 'Measure' to take a reading")
                        .size(14.0)
                        .color(egui::Color32::DARK_GRAY),
                );
            }
        });
    }

    // ========================================================================
    // Expert Mode Rendering
    // ========================================================================

    fn render_expert_workspace(&self, ui: &mut egui::Ui) {
        ui.heading("📊 Spectral Power Distribution");

        let plot = Plot::new("spectral_plot")
            .view_aspect(2.5)
            .height(200.0) // Minimum height regardless of aspect ratio
            .include_y(0.0)
            .include_x(380.0)
            .include_x(780.0)
            .legend(Legend::default().position(egui_plot::Corner::RightTop))
            .y_axis_label("Relative Intensity")
            .x_axis_label("Wavelength (nm)")
            .show_axes([true, true])
            .show_grid(true);

        plot.show(ui, |plot_ui| {
            // Draw current measurement
            if let Some(res) = &self.last_result {
                let points: PlotPoints = res
                    .spectrum
                    .wavelengths
                    .iter()
                    .zip(res.spectrum.values.iter())
                    .map(|(w, v)| [*w as f64, *v as f64])
                    .collect();

                let line = Line::new(points)
                    .name("Measurement")
                    .color(egui::Color32::from_rgb(0, 255, 128))
                    .width(2.5);
                plot_ui.line(line);

                // Mark peak wavelength
                let peak_idx = res
                    .spectrum
                    .values
                    .iter()
                    .enumerate()
                    .skip(4) // Skip noise below 420nm
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                let peak_wl = 380.0 + peak_idx as f64 * 10.0;
                plot_ui.vline(
                    VLine::new(peak_wl)
                        .color(egui::Color32::from_rgba_unmultiplied(255, 255, 0, 100))
                        .style(egui_plot::LineStyle::dashed_dense())
                        .name(format!("Peak: {}nm", peak_wl as i32)),
                );
            }

            // Reference line at 1.0
            plot_ui.hline(
                HLine::new(1.0)
                    .color(egui::Color32::DARK_GRAY)
                    .style(egui_plot::LineStyle::dashed_loose()),
            );

            // Color region markers (approximate visible spectrum boundaries)
            let color_regions = [
                (380.0, 440.0, "Violet", egui::Color32::from_rgb(148, 0, 211)),
                (440.0, 485.0, "Blue", egui::Color32::from_rgb(0, 0, 255)),
                (485.0, 500.0, "Cyan", egui::Color32::from_rgb(0, 255, 255)),
                (500.0, 565.0, "Green", egui::Color32::from_rgb(0, 255, 0)),
                (565.0, 590.0, "Yellow", egui::Color32::from_rgb(255, 255, 0)),
                (590.0, 625.0, "Orange", egui::Color32::from_rgb(255, 165, 0)),
                (625.0, 780.0, "Red", egui::Color32::from_rgb(255, 0, 0)),
            ];

            for (start, end, _name, color) in color_regions {
                let mid = (start + end) / 2.0;
                plot_ui.vline(
                    VLine::new(mid)
                        .color(egui::Color32::from_rgba_unmultiplied(
                            color.r(),
                            color.g(),
                            color.b(),
                            30,
                        ))
                        .width(end as f32 - start as f32),
                );
            }
        });

        // === Multi-dimensional Data Dashboard ===
        ui.add_space(10.0);

        if let Some(res) = &self.last_result {
            let xyz = res.xyz;
            let lab = res.lab;
            let (chroma, hue) = (lab.chroma(), lab.hue());
            let cct = res.cct;

            // Responsive layout: define preferred card dimensions
            let spacing = 12.0;
            let min_card_width = 100.0;
            let max_card_width = 200.0;

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(spacing, spacing);

                // Bento 1: LAB
                render_bento_item(
                    ui,
                    t!("gui-bento-lab"),
                    min_card_width,
                    max_card_width,
                    |ui| {
                        egui::Grid::new("bento_lab").show(ui, |ui| {
                            ui.label("L*");
                            ui.label(format!("{:.2}", lab.l));
                            ui.end_row();
                            ui.label("a*");
                            ui.label(format!("{:.2}", lab.a));
                            ui.end_row();
                            ui.label("b*");
                            ui.label(format!("{:.2}", lab.b));
                            ui.end_row();
                        });
                    },
                );

                // Bento 2: XYZ
                render_bento_item(
                    ui,
                    t!("gui-bento-xyz"),
                    min_card_width,
                    max_card_width,
                    |ui| {
                        egui::Grid::new("bento_xyz").show(ui, |ui| {
                            ui.label("X");
                            ui.label(format!("{:.3}", xyz.x));
                            ui.end_row();
                            ui.label("Y");
                            ui.label(format!("{:.3}", xyz.y));
                            ui.end_row();
                            ui.label("Z");
                            ui.label(format!("{:.3}", xyz.z));
                            ui.end_row();
                        });
                    },
                );

                // Bento 3: Color Indices
                render_bento_item(ui, t!("gui-bento-indices"), 115.0, max_card_width, |ui| {
                    egui::Grid::new("bento_indices").show(ui, |ui| {
                        ui.label(t!("gui-bento-chroma"));
                        ui.label(format!("{:.1}", chroma));
                        ui.end_row();
                        ui.label(t!("gui-bento-hue"));
                        ui.label(format!("{:.1}°", hue));
                        ui.end_row();
                        ui.label(t!("gui-bento-cct"));
                        ui.label(format!("{:.0}K", cct));
                        ui.end_row();
                    });
                });

                // Bento 4: Peak Information
                render_bento_item(ui, t!("gui-bento-peak"), 115.0, max_card_width, |ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{:.1} nm", res.peak_wavelength()))
                                .size(24.0)
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(format!(
                                "Centroid: {:.1}nm",
                                res.centroid_wavelength()
                            ))
                            .weak(),
                        );
                    });
                });

                // Bento 5: sRGB
                render_bento_item(ui, t!("gui-bento-srgb"), 130.0, max_card_width, |ui| {
                    ui.horizontal(|ui| {
                        let (r, g, b) = res.rgb_u8();
                        let (rect, _) =
                            ui.allocate_at_least(egui::vec2(40.0, 40.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, 4.0, egui::Color32::from_rgb(r, g, b));
                        ui.painter().rect_stroke(
                            rect,
                            4.0,
                            egui::Stroke::new(1.0, border_color(ui.visuals())),
                        );
                        ui.add_space(8.0);
                        ui.vertical(|ui| {
                            ui.label(format!("RGB: {}, {}, {}", r, g, b));
                            ui.label(
                                egui::RichText::new(format!("#{:02X}{:02X}{:02X}", r, g, b))
                                    .monospace()
                                    .weak(),
                            );
                        });
                    });
                });

                // Bento 6: CRI (if available)
                if let Some(cri) = res.cri {
                    render_bento_item(ui, t!("gui-bento-cri"), 75.0, max_card_width, |ui| {
                        ui.centered_and_justified(|ui| {
                            ui.label(
                                egui::RichText::new(format!("{:.0}", cri))
                                    .size(28.0)
                                    .strong(),
                            );
                        });
                    });
                }
            });
        }
    }
}

// ============================================================================
// eframe::App Implementation
// ============================================================================

impl eframe::App for SpectroApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
                            let visuals = self.theme_config.to_visuals();
                            ctx.set_visuals(visuals);
                            // Persist the new theme choice
                            let _ = self.theme_config.save("spectro_theme.json");
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
        if self.show_settings {
            egui::Window::new(format!("⚙ {}", t!("gui-settings")))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.heading(t!("gui-colorimetry-standards"));
                    ui.add_space(10.0);

                    egui::Grid::new("settings_grid")
                        .num_columns(2)
                        .spacing([20.0, 10.0])
                        .show(ui, |ui| {
                            ui.label(t!("gui-illuminant"));
                            egui::ComboBox::from_id_salt("illuminant_selector_settings")
                                .selected_text(format!("{:?}", self.selected_illuminant))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.selected_illuminant,
                                        Illuminant::D65,
                                        "D65 (Daylight, sRGB)",
                                    );
                                    ui.selectable_value(
                                        &mut self.selected_illuminant,
                                        Illuminant::D50,
                                        "D50 (Print Industry)",
                                    );
                                    ui.selectable_value(
                                        &mut self.selected_illuminant,
                                        Illuminant::A,
                                        "A (Tungsten 2856K)",
                                    );
                                    ui.selectable_value(
                                        &mut self.selected_illuminant,
                                        Illuminant::F2,
                                        "F2 (Cool White Fluorescent)",
                                    );
                                    ui.selectable_value(
                                        &mut self.selected_illuminant,
                                        Illuminant::F7,
                                        "F7 (Daylight Fluorescent)",
                                    );
                                    ui.selectable_value(
                                        &mut self.selected_illuminant,
                                        Illuminant::F11,
                                        "F11 (TL84)",
                                    );
                                });
                            ui.end_row();

                            ui.label(t!("gui-observer"));
                            egui::ComboBox::from_id_salt("observer_selector_settings")
                                .selected_text(match self.selected_observer {
                                    Observer::CIE1931_2 => "2° (Standard)",
                                    Observer::CIE1964_10 => "10° (Supplementary)",
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.selected_observer,
                                        Observer::CIE1931_2,
                                        "2° (CIE 1931 Standard)",
                                    );
                                    ui.selectable_value(
                                        &mut self.selected_observer,
                                        Observer::CIE1964_10,
                                        "10° (CIE 1964 Large Field)",
                                    );
                                });
                            ui.end_row();
                        });

                    ui.add_space(20.0);
                    ui.separator();
                    ui.heading(t!("gui-language-title"));
                    ui.add_space(10.0);

                    egui::Grid::new("language_settings_grid")
                        .num_columns(2)
                        .spacing([20.0, 10.0])
                        .show(ui, |ui| {
                            ui.label(t!("gui-language"));
                            let old_lang = self.theme_config.language;
                            egui::ComboBox::from_id_salt("language_selector")
                                .selected_text(self.theme_config.language.label())
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        &mut self.theme_config.language,
                                        crate::i18n::Language::Auto,
                                        crate::i18n::Language::Auto.label(),
                                    );
                                    ui.selectable_value(
                                        &mut self.theme_config.language,
                                        crate::i18n::Language::EnUS,
                                        crate::i18n::Language::EnUS.label(),
                                    );
                                    ui.selectable_value(
                                        &mut self.theme_config.language,
                                        crate::i18n::Language::ZhCN,
                                        crate::i18n::Language::ZhCN.label(),
                                    );
                                });
                            // Apply language change immediately
                            if self.theme_config.language != old_lang {
                                crate::i18n::init(self.theme_config.language);
                                let _ = self.theme_config.save("spectro_theme.json");
                            }
                            ui.end_row();
                        });

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button(t!("gui-close")).clicked() {
                            self.show_settings = false;
                        }
                    });
                });
        }

        // === Reference Input Window (Modal-like) ===
        if self.show_reference_input {
            egui::Window::new(t!("gui-set-ref-color"))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label(t!("gui-enter-lab"));
                    ui.add_space(10.0);

                    egui::Grid::new("ref_input_grid")
                        .num_columns(2)
                        .spacing([10.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("L*:");
                            ui.add(
                                egui::DragValue::new(&mut self.ref_input_l)
                                    .range(0.0..=100.0)
                                    .speed(0.5),
                            );
                            ui.end_row();
                            ui.label("a*:");
                            ui.add(
                                egui::DragValue::new(&mut self.ref_input_a)
                                    .range(-128.0..=128.0)
                                    .speed(0.5),
                            );
                            ui.end_row();
                            ui.label("b*:");
                            ui.add(
                                egui::DragValue::new(&mut self.ref_input_b)
                                    .range(-128.0..=128.0)
                                    .speed(0.5),
                            );
                            ui.end_row();
                        });

                    ui.add_space(5.0);
                    ui.label("ΔE Tolerance:");
                    ui.add(
                        egui::Slider::new(&mut self.delta_e_tolerance, 0.5..=10.0).suffix(" ΔE"),
                    );

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui
                            .button(format!("✓ {}", t!("gui-set-reference")))
                            .clicked()
                        {
                            self.reference_lab = Some(Lab {
                                l: self.ref_input_l,
                                a: self.ref_input_a,
                                b: self.ref_input_b,
                            });
                            self.show_reference_input = false;
                        }
                        if ui.button(t!("gui-use-current")).clicked()
                            && let Some(lab) = self.get_current_lab()
                        {
                            self.ref_input_l = lab.l;
                            self.ref_input_a = lab.a;
                            self.ref_input_b = lab.b;
                            self.reference_lab = Some(lab);
                            self.show_reference_input = false;
                        }
                        if ui.button(t!("gui-clear")).clicked() {
                            self.reference_lab = None;
                        }
                        if ui.button(t!("gui-cancel")).clicked() {
                            self.show_reference_input = false;
                        }
                    });
                });
        }

        // === Left Panel: History (Expert mode only) ===
        if self.is_expert_mode && self.show_history_panel {
            egui::SidePanel::left("history_panel")
                .resizable(true)
                .default_width(180.0)
                .min_width(120.0)
                .max_width(250.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading(t!("gui-history-title"));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("⏴").on_hover_text(t!("gui-hide")).clicked() {
                                self.show_history_panel = false;
                            }
                        });
                    });
                    ui.separator();

                    if self.measurement_history.is_empty() {
                        ui.centered_and_justified(|ui| {
                            ui.label(egui::RichText::new(t!("gui-no-data")).weak());
                        });
                    } else {
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            for (idx, entry) in self.measurement_history.iter().enumerate() {
                                let lab = &entry.result.lab;
                                let xyz = entry.result.xyz;
                                let y_max = xyz.y.max(0.01);
                                let xyz_norm = XYZ {
                                    x: xyz.x / y_max,
                                    y: xyz.y / y_max,
                                    z: xyz.z / y_max,
                                };
                                let (r, g, b) = xyz_norm.to_srgb();

                                ui.horizontal(|ui| {
                                    // Color swatch
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(24.0, 24.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(
                                        rect,
                                        4.0,
                                        egui::Color32::from_rgb(r, g, b),
                                    );

                                    ui.vertical(|ui| {
                                        // Show mode icon and timestamp
                                        let mode_icon = match entry.mode {
                                            MeasurementMode::Reflective => "📄",
                                            MeasurementMode::Emissive => "🖥️",
                                            MeasurementMode::Ambient => "💡",
                                        };
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{} {}",
                                                mode_icon, entry.timestamp
                                            ))
                                            .small(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "L:{:.0} a:{:.0} b:{:.0}",
                                                lab.l, lab.a, lab.b
                                            ))
                                            .small(),
                                        );
                                        if let Some(de) = entry.delta_e {
                                            let color = if de <= self.delta_e_tolerance {
                                                success_color(&ui.ctx().style().visuals)
                                            } else {
                                                egui::Color32::RED
                                            };
                                            ui.colored_label(
                                                color,
                                                egui::RichText::new(format!("ΔE00={:.1}", de))
                                                    .small(),
                                            );
                                        }
                                    });
                                });

                                if idx < self.measurement_history.len() - 1 {
                                    ui.separator();
                                }
                            }
                        });

                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if ui.button("CSV").clicked() {
                                self.export_history_csv();
                            }
                            if ui.button("JSON").clicked() {
                                self.export_history_json();
                            }
                            if ui.button("CGATS").clicked() {
                                self.export_history_cgats();
                            }
                            if ui.button("Clear").clicked() {
                                self.measurement_history.clear();
                            }
                        });
                    }
                });
        }

        // === Right Panel: Expert Inspector ===
        if self.is_expert_mode && self.inspector.visible {
            egui::SidePanel::right("expert_panel")
                .resizable(true)
                .default_width(260.0)
                .min_width(160.0)
                .max_width(350.0)
                .show(ctx, |ui| {
                    let ctx = InspectorContext {
                        device_info: &self.device_info,
                        is_connected: self.is_connected,
                        is_calibrated: self.is_calibrated,
                        selected_mode: self.selected_mode,
                        last_result: self.last_result.as_ref(),
                        last_tm30: self.last_tm30.as_ref(),
                        history: &self.measurement_history,
                    };
                    self.inspector.render(ui, &ctx);
                });
        }

        // === Central Panel: Main Workspace ===
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(panel_bg_dark_color(&ctx.style().visuals))
                    .inner_margin(egui::Margin::same(16.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.is_expert_mode {
                        self.render_expert_workspace(ui);
                    } else {
                        self.render_simple_workspace(ui);
                    }
                });

                // Calibration Wizard (extracted component)
                self.calibration_wizard.render(
                    ctx,
                    &self.cmd_tx,
                    &mut self.is_busy,
                    &self.status_msg,
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
                    CalibrationWizard::render_dial_check(ctx, highlight);
                }
            });

        // Request continuous repaint for smooth animations
        ctx.request_repaint();
    }
}
