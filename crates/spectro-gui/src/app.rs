//! Graphical User Interface for spectro-rs.
//!
//! This module implements the main application window using the [`eframe`] framework.
//! It features a **Simple/Expert dual-mode** design:
//!
//! - **Simple Mode**: Large color swatch, Pass/Fail display, key metrics only.
//! - **Expert Mode**: Full spectral plot, EEPROM data viewer, raw sensor values.

use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui;
use egui_plot::{HLine, Legend, Line, Plot, PlotPoints, Points, VLine};
use spectro_rs::{
    colorimetry::{illuminant, Lab, XYZ, X_BAR_2, Y_BAR_2, Z_BAR_2},
    discover,
    tm30::calculate_tm30,
    BoxedSpectrometer, Illuminant, MeasurementMode, Observer, SpectralData,
};
use std::thread;
use std::time::{Duration, Instant};

use crate::calibration::CalibrationWizard;
use crate::shared::{DeviceCommand, ExtendedDeviceInfo, MeasurementEntry, UIUpdate};
use crate::theme::ThemeConfig;

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
    last_result: Option<SpectralData>,
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
    expert_tab: ExpertTab,
    show_reference_input: bool,
    show_settings: bool,

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

/// Tabs in the Expert panel
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ExpertTab {
    RawSensor,
    DeviceInfo,
    Algorithm,
    Chromaticity,
    ColorQuality,
    Trend,
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
                                    .send(UIUpdate::Status("✅ Device connected".into()))
                                    .ok();
                            }
                            Err(e) => {
                                update_tx
                                    .send(UIUpdate::Error(format!("❌ Discovery failed: {}", e)))
                                    .ok();
                            }
                        }
                    }

                    DeviceCommand::Calibrate => {
                        if let Some(ref mut d) = device {
                            update_tx
                                .send(UIUpdate::Status(
                                    "🎯 Calibrating... Place device on white tile".into(),
                                ))
                                .ok();

                            match d.calibrate() {
                                Ok(_) => {
                                    update_tx
                                        .send(UIUpdate::Status("✅ Calibration successful".into()))
                                        .ok();
                                }
                                Err(e) => {
                                    update_tx
                                        .send(UIUpdate::Error(format!(
                                            "❌ Calibration failed: {}",
                                            e
                                        )))
                                        .ok();
                                }
                            }
                        } else {
                            update_tx
                                .send(UIUpdate::Error("⚠️ No device connected".into()))
                                .ok();
                        }
                    }

                    DeviceCommand::Measure(mode) => {
                        if let Some(ref mut d) = device {
                            update_tx
                                .send(UIUpdate::Status("📊 Measuring...".into()))
                                .ok();

                            match d.measure(mode) {
                                Ok(data) => {
                                    let tm30 = if mode == MeasurementMode::Emissive {
                                        Some(Box::new(calculate_tm30(&data)))
                                    } else {
                                        None
                                    };
                                    update_tx.send(UIUpdate::Result(data, tm30)).ok();
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
                                        .send(UIUpdate::Error(format!(
                                            "❌ Measurement failed: {}",
                                            e
                                        )))
                                        .ok();
                                }
                            }
                        } else {
                            update_tx
                                .send(UIUpdate::Error("⚠️ No device connected".into()))
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
            expert_tab: ExpertTab::DeviceInfo,
            show_reference_input: false,
            show_settings: false,
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
        self.last_result.as_ref().map(|data| {
            let xyz = data.to_xyz_ext(self.selected_illuminant, self.selected_observer);
            let xyz_normalized = XYZ {
                x: xyz.x / 100.0,
                y: xyz.y / 100.0,
                z: xyz.z / 100.0,
            };
            xyz_normalized.to_lab(
                self.selected_illuminant
                    .get_white_point(self.selected_observer),
            )
        })
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

    fn get_pass_fail(&self, delta_e: f32) -> (bool, egui::Color32) {
        if delta_e <= self.delta_e_tolerance {
            (true, egui::Color32::from_rgb(50, 205, 50)) // Lime green
        } else {
            (false, egui::Color32::from_rgb(220, 53, 69)) // Red
        }
    }

    fn add_to_history(&mut self, data: SpectralData) {
        let lab = {
            let xyz = data.to_xyz_ext(self.selected_illuminant, self.selected_observer);
            let xyz_normalized = XYZ {
                x: xyz.x / 100.0,
                y: xyz.y / 100.0,
                z: xyz.z / 100.0,
            };
            xyz_normalized.to_lab(
                self.selected_illuminant
                    .get_white_point(self.selected_observer),
            )
        };
        let delta_e = self
            .reference_lab
            .as_ref()
            .map(|ref_lab| lab.delta_e_2000(ref_lab));

        let entry = MeasurementEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            mode: self.selected_mode,
            data,
            lab,
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
                    entry.lab.l,
                    entry.lab.a,
                    entry.lab.b,
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

        if let Some(path) = file_path {
            if let Ok(json) = serde_json::to_string_pretty(&self.measurement_history) {
                if let Err(e) = std::fs::write(path, json) {
                    eprintln!("Failed to write JSON: {}", e);
                }
            }
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
                let xyz = entry.data.to_xyz();
                cgats.push_str(&format!(
                    "{} \"{}\" {:.4} {:.4} {:.4} {:.4} {:.4} {:.4} ",
                    i + 1,
                    entry.timestamp,
                    entry.lab.l,
                    entry.lab.a,
                    entry.lab.b,
                    xyz.x,
                    xyz.y,
                    xyz.z
                ));

                for val in &entry.data.values {
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

            if let Some(data) = &self.last_result {
                let xyz = data.to_xyz();
                let y_max = xyz.y.max(0.01);
                let xyz_normalized = XYZ {
                    x: xyz.x / y_max,
                    y: xyz.y / y_max,
                    z: xyz.z / y_max,
                };
                let (r, g, b) = xyz_normalized.to_srgb();
                let lab = self.get_current_lab().unwrap();

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
                    egui::Color32::from_rgba_unmultiplied(0, 0, 0, 80),
                );

                // Main swatch
                painter.rect_filled(rect, 16.0, egui::Color32::from_rgb(r, g, b));

                // Border
                painter.rect_stroke(
                    rect,
                    16.0,
                    egui::Stroke::new(2.0, egui::Color32::from_rgb(60, 60, 80)),
                );

                ui.add_space(20.0);

                // === Pass/Fail Indicator ===
                if let Some(delta_e) = self.calculate_delta_e(&lab) {
                    let (passed, color) = self.get_pass_fail(delta_e);

                    let status_text = if passed { "✓ PASS" } else { "✗ FAIL" };
                    ui.colored_label(color, egui::RichText::new(status_text).size(48.0).strong());

                    ui.add_space(10.0);
                    ui.label(
                        egui::RichText::new(format!("ΔE*00 = {:.2}", delta_e))
                            .size(24.0)
                            .color(egui::Color32::GRAY),
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
                        .fill(egui::Color32::from_rgb(28, 28, 36))
                        .rounding(8.0)
                        .inner_margin(egui::Margin::same(16.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        egui::RichText::new("L*")
                                            .size(14.0)
                                            .color(egui::Color32::GRAY),
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
                                            .color(egui::Color32::GRAY),
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
                                            .color(egui::Color32::GRAY),
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
                        .color(egui::Color32::GRAY),
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
                        .color(egui::Color32::GRAY),
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
            if let Some(data) = &self.last_result {
                let points: PlotPoints = data
                    .wavelengths
                    .iter()
                    .zip(data.values.iter())
                    .map(|(w, v)| [*w as f64, *v as f64])
                    .collect();

                let line = Line::new(points)
                    .name("Measurement")
                    .color(egui::Color32::from_rgb(0, 255, 128))
                    .width(2.5);
                plot_ui.line(line);

                // Mark peak wavelength
                let peak_idx = data
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

        if let Some(data) = &self.last_result {
            let xyz = data.to_xyz();
            let xyz_for_lab = XYZ {
                x: xyz.x / 100.0,
                y: xyz.y / 100.0,
                z: xyz.z / 100.0,
            };
            let lab = xyz_for_lab.to_lab(illuminant::D65_2);
            let (chroma, hue) = (lab.chroma(), lab.hue());
            let cct = xyz.to_cct();

            // Peak and centroid
            let peak_idx = data
                .values
                .iter()
                .enumerate()
                .skip(4)
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(0);
            let peak_wl = 380 + peak_idx * 10;

            let total_power: f32 = data.values.iter().skip(4).sum();
            let centroid: f32 = data
                .values
                .iter()
                .enumerate()
                .skip(4)
                .map(|(i, v)| (380 + i * 10) as f32 * v)
                .sum::<f32>()
                / total_power.max(1e-6);

            ui.columns(3, |cols| {
                // Column 1: XYZ & Lab
                cols[0].group(|ui| {
                    ui.heading("CIE Color Spaces");
                    ui.add_space(5.0);
                    egui::Grid::new("xyz_lab_grid")
                        .num_columns(2)
                        .spacing([20.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("X:");
                            ui.label(format!("{:.3}", xyz.x));
                            ui.end_row();
                            ui.label("Y:");
                            ui.label(format!("{:.3}", xyz.y));
                            ui.end_row();
                            ui.label("Z:");
                            ui.label(format!("{:.3}", xyz.z));
                            ui.end_row();
                            ui.separator();
                            ui.separator();
                            ui.end_row();
                            ui.label("L*:");
                            ui.label(format!("{:.2}", lab.l));
                            ui.end_row();
                            ui.label("a*:");
                            ui.label(format!("{:.2}", lab.a));
                            ui.end_row();
                            ui.label("b*:");
                            ui.label(format!("{:.2}", lab.b));
                            ui.end_row();
                        });
                });

                // Column 2: LCh & CCT
                cols[1].group(|ui| {
                    ui.heading("Derived Values");
                    ui.add_space(5.0);
                    egui::Grid::new("lch_cct_grid")
                        .num_columns(2)
                        .spacing([20.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("C* (Chroma):");
                            ui.label(format!("{:.2}", chroma));
                            ui.end_row();
                            ui.label("h° (Hue):");
                            ui.label(format!("{:.1}°", hue));
                            ui.end_row();
                            ui.separator();
                            ui.separator();
                            ui.end_row();
                            ui.label("CCT:");
                            ui.label(format!("{:.0} K", cct));
                            ui.end_row();
                            ui.label("Peak λ:");
                            ui.label(format!("{} nm", peak_wl));
                            ui.end_row();
                            ui.label("Centroid:");
                            ui.label(format!("{:.1} nm", centroid));
                            ui.end_row();
                        });
                });

                // Column 3: sRGB
                cols[2].group(|ui| {
                    ui.heading("sRGB Output");
                    ui.add_space(5.0);

                    let y_max = xyz.y.max(0.01);
                    let xyz_norm = XYZ {
                        x: xyz.x / y_max,
                        y: xyz.y / y_max,
                        z: xyz.z / y_max,
                    };
                    let (r, g, b) = xyz_norm.to_srgb();

                    // Color preview
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(80.0, 40.0), egui::Sense::hover());
                    ui.painter()
                        .rect_filled(rect, 4.0, egui::Color32::from_rgb(r, g, b));

                    egui::Grid::new("rgb_grid")
                        .num_columns(2)
                        .spacing([20.0, 4.0])
                        .show(ui, |ui| {
                            ui.label("R:");
                            ui.label(format!("{}", r));
                            ui.end_row();
                            ui.label("G:");
                            ui.label(format!("{}", g));
                            ui.end_row();
                            ui.label("B:");
                            ui.label(format!("{}", b));
                            ui.end_row();
                            ui.label("Hex:");
                            ui.label(format!("#{:02X}{:02X}{:02X}", r, g, b));
                            ui.end_row();
                        });
                });
            });
        }
    }

    fn render_expert_inspector(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔬 Device Inspector");
        ui.add_space(10.0);

        // Tab bar
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.expert_tab, ExpertTab::DeviceInfo, "📱 Device");
            ui.selectable_value(&mut self.expert_tab, ExpertTab::RawSensor, "📈 Raw Data");
            ui.selectable_value(&mut self.expert_tab, ExpertTab::Algorithm, "🧮 Algorithm");
            ui.selectable_value(
                &mut self.expert_tab,
                ExpertTab::Chromaticity,
                "🎯 xy Diagram",
            );
            ui.selectable_value(
                &mut self.expert_tab,
                ExpertTab::ColorQuality,
                "🌈 Color Quality",
            );
            ui.selectable_value(&mut self.expert_tab, ExpertTab::Trend, "📈 Trend");
        });

        ui.separator();

        match self.expert_tab {
            ExpertTab::DeviceInfo => self.render_device_info_tab(ui),
            ExpertTab::RawSensor => self.render_raw_sensor_tab(ui),
            ExpertTab::Algorithm => self.render_algorithm_tab(ui),
            ExpertTab::Chromaticity => self.render_chromaticity_tab(ui),
            ExpertTab::ColorQuality => self.render_color_quality_tab(ui),
            ExpertTab::Trend => self.render_trend_tab(ui),
        }
    }

    fn render_trend_tab(&self, ui: &mut egui::Ui) {
        ui.add_space(5.0);
        ui.heading("📈 Measurement Trend");
        ui.add_space(10.0);

        if self.measurement_history.is_empty() {
            ui.label("No data to display.");
            return;
        }

        let plot = Plot::new("trend_plot")
            .view_aspect(2.0)
            .legend(Legend::default())
            .y_axis_label("Value")
            .x_axis_label("Samples (Newest to Oldest)");

        plot.show(ui, |plot_ui| {
            // L* Trend
            let l_points: PlotPoints = self
                .measurement_history
                .iter()
                .enumerate()
                .map(|(i, e)| [i as f64, e.lab.l as f64])
                .collect();
            plot_ui.line(Line::new(l_points).name("L*").color(egui::Color32::WHITE));

            // Delta E Trend (if reference exists)
            if self.reference_lab.is_some() {
                let de_points: PlotPoints = self
                    .measurement_history
                    .iter()
                    .enumerate()
                    .filter_map(|(i, e)| e.delta_e.map(|de| [i as f64, de as f64]))
                    .collect();
                plot_ui.line(Line::new(de_points).name("ΔE*00").color(egui::Color32::RED));
            }
        });

        ui.add_space(10.0);
        ui.label("This chart shows the stability of measurements over time. Useful for monitoring light source warm-up or drift.");
    }

    fn render_device_info_tab(&self, ui: &mut egui::Ui) {
        ui.add_space(5.0);

        // Basic Device Info
        ui.collapsing("📱 Device Information", |ui| {
            egui::Grid::new("device_info_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    if let Some(ref basic) = self.device_info.basic {
                        ui.label("Model:");
                        ui.label(&basic.model);
                        ui.end_row();
                        ui.label("Serial:");
                        ui.label(&basic.serial);
                        ui.end_row();
                        ui.label("Firmware:");
                        ui.label(&basic.firmware);
                        ui.end_row();
                    } else {
                        ui.label("Status:");
                        ui.colored_label(egui::Color32::YELLOW, "Not connected");
                        ui.end_row();
                    }

                    if let Some(cal_ver) = self.device_info.cal_version {
                        ui.label("Cal Version:");
                        ui.label(format!("0x{:04X}", cal_ver));
                        ui.end_row();
                    }
                });
        });

        // EEPROM Calibration Data
        ui.collapsing("📦 EEPROM Calibration", |ui| {
            if let Some(ref white_ref) = self.device_info.white_ref {
                ui.label("White Reference Spectrum:");

                // Mini plot of white reference
                let plot = Plot::new("white_ref_plot")
                    .height(100.0)
                    .show_axes([true, true])
                    .include_y(0.0);

                plot.show(ui, |plot_ui| {
                    let points: PlotPoints = white_ref
                        .iter()
                        .enumerate()
                        .map(|(i, v)| [(380 + i * 10) as f64, *v as f64])
                        .collect();
                    plot_ui.line(Line::new(points).color(egui::Color32::WHITE).width(1.5));
                });
            } else {
                ui.colored_label(egui::Color32::GRAY, "White reference data not available");
            }

            ui.add_space(5.0);

            // Emissive calibration coefficients
            if let Some(ref emis) = self.device_info.emis_coef {
                ui.collapsing("Emissive Coefficients", |ui| {
                    ui.label(format!("Count: {} bands", emis.len()));
                    if !emis.is_empty() {
                        ui.label(format!(
                            "Range: {:.4} - {:.4}",
                            emis.iter().cloned().fold(f32::INFINITY, f32::min),
                            emis.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
                        ));
                    }
                });
            }

            // Ambient calibration coefficients
            if let Some(ref amb) = self.device_info.amb_coef {
                ui.collapsing("Ambient Coefficients", |ui| {
                    ui.label(format!("Count: {} bands", amb.len()));
                    if !amb.is_empty() {
                        ui.label(format!(
                            "Range: {:.4} - {:.4}",
                            amb.iter().cloned().fold(f32::INFINITY, f32::min),
                            amb.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
                        ));
                    }
                });
            }

            ui.add_space(5.0);

            // Linearization polynomials
            if let Some(ref lin) = self.device_info.lin_normal {
                ui.label(format!("Lin (Normal): {:?}", lin));
            }
            if let Some(ref lin) = self.device_info.lin_high {
                ui.label(format!("Lin (High Gain): {:?}", lin));
            }
        });

        // Connection Status
        ui.collapsing("🔌 Connection Status", |ui| {
            egui::Grid::new("conn_status_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Connected:");
                    if self.is_connected {
                        ui.colored_label(egui::Color32::GREEN, "Yes ✓");
                    } else {
                        ui.colored_label(egui::Color32::RED, "No ✗");
                    }
                    ui.end_row();

                    ui.label("Calibrated:");
                    if self.is_calibrated {
                        ui.colored_label(egui::Color32::GREEN, "Yes ✓");
                    } else {
                        ui.colored_label(egui::Color32::YELLOW, "No");
                    }
                    ui.end_row();

                    ui.label("Mode:");
                    ui.label(format!("{:?}", self.selected_mode));
                    ui.end_row();
                });
        });
    }

    fn render_raw_sensor_tab(&self, ui: &mut egui::Ui) {
        ui.add_space(5.0);

        if let Some(data) = &self.last_result {
            ui.label(egui::RichText::new("Spectral Values (380-780nm, 10nm steps)").strong());
            ui.add_space(5.0);

            // Scrollable table of values
            egui::ScrollArea::vertical()
                .max_height(400.0)
                .show(ui, |ui| {
                    egui::Grid::new("raw_values_grid")
                        .num_columns(4)
                        .spacing([15.0, 2.0])
                        .striped(true)
                        .show(ui, |ui| {
                            // Header
                            ui.label(egui::RichText::new("λ (nm)").strong());
                            ui.label(egui::RichText::new("Value").strong());
                            ui.label(egui::RichText::new("λ (nm)").strong());
                            ui.label(egui::RichText::new("Value").strong());
                            ui.end_row();

                            // Values in two columns
                            for i in (0..data.values.len()).step_by(2) {
                                let wl1 = 380 + i * 10;
                                ui.label(format!("{}", wl1));
                                ui.label(format!("{:.6}", data.values[i]));

                                if i + 1 < data.values.len() {
                                    let wl2 = 380 + (i + 1) * 10;
                                    ui.label(format!("{}", wl2));
                                    ui.label(format!("{:.6}", data.values[i + 1]));
                                }
                                ui.end_row();
                            }
                        });
                });

            ui.add_space(10.0);

            // Statistics
            ui.collapsing("📊 Statistics", |ui| {
                let values = &data.values;
                let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
                let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
                let sum: f32 = values.iter().sum();
                let mean = sum / values.len() as f32;

                egui::Grid::new("stats_grid")
                    .num_columns(2)
                    .spacing([20.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("Min:");
                        ui.label(format!("{:.6}", min));
                        ui.end_row();
                        ui.label("Max:");
                        ui.label(format!("{:.6}", max));
                        ui.end_row();
                        ui.label("Mean:");
                        ui.label(format!("{:.6}", mean));
                        ui.end_row();
                        ui.label("Total:");
                        ui.label(format!("{:.6}", sum));
                        ui.end_row();
                    });
            });
        } else {
            ui.centered_and_justified(|ui| {
                ui.label("No measurement data available");
            });
        }
    }

    fn render_algorithm_tab(&self, ui: &mut egui::Ui) {
        ui.add_space(5.0);

        ui.collapsing("🎯 White Point Reference", |ui| {
            let wp = illuminant::D65_2;
            egui::Grid::new("wp_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Illuminant:");
                    ui.label("D65 (2° Observer)");
                    ui.end_row();
                    ui.label("Xn:");
                    ui.label(format!("{:.5}", wp.x));
                    ui.end_row();
                    ui.label("Yn:");
                    ui.label(format!("{:.5}", wp.y));
                    ui.end_row();
                    ui.label("Zn:");
                    ui.label(format!("{:.5}", wp.z));
                    ui.end_row();
                });
        });

        ui.collapsing("📐 Observer Functions", |ui| {
            ui.label("Currently using: CIE 1931 2° Standard Observer");
            ui.add_space(5.0);

            // Option to show CMF plot
            ui.horizontal(|ui| {
                ui.label("CMFs:");
                ui.label("x̄(λ), ȳ(λ), z̄(λ) from 380-780nm");
            });
        });

        ui.collapsing("🔄 Conversion Pipeline", |ui| {
            ui.label(egui::RichText::new("Data Flow:").strong());
            ui.add_space(5.0);

            let pipeline = [
                "1. Raw Sensor (128 pixels)",
                "   ↓ EEPROM Matrix Transform",
                "2. Spectral Data (36 bands)",
                "   ↓ Dark Subtraction",
                "3. Corrected Spectrum",
                "   ↓ CMF Integration",
                "4. CIE XYZ",
                "   ↓ Bradford Adaptation",
                "5. Lab (D65)",
            ];

            for step in pipeline {
                ui.label(egui::RichText::new(step).monospace());
            }
        });

        if let Some(data) = &self.last_result {
            ui.collapsing("🧪 Current Calculation", |ui| {
                let xyz = data.to_xyz();
                let xyz_norm = XYZ {
                    x: xyz.x / 100.0,
                    y: xyz.y / 100.0,
                    z: xyz.z / 100.0,
                };
                let lab = xyz_norm.to_lab(illuminant::D65_2);

                ui.label(format!("Mode: {:?}", data.mode));
                ui.add_space(5.0);

                egui::Grid::new("calc_grid")
                    .num_columns(2)
                    .spacing([20.0, 4.0])
                    .show(ui, |ui| {
                        ui.label("XYZ (raw):");
                        ui.label(format!("({:.3}, {:.3}, {:.3})", xyz.x, xyz.y, xyz.z));
                        ui.end_row();
                        ui.label("XYZ (norm):");
                        ui.label(format!(
                            "({:.4}, {:.4}, {:.4})",
                            xyz_norm.x, xyz_norm.y, xyz_norm.z
                        ));
                        ui.end_row();
                        ui.label("Lab:");
                        ui.label(format!("({:.2}, {:.2}, {:.2})", lab.l, lab.a, lab.b));
                        ui.end_row();
                    });
            });
        }
    }

    fn render_chromaticity_tab(&self, ui: &mut egui::Ui) {
        ui.add_space(5.0);
        ui.heading("🎯 CIE 1931 xy Chromaticity");
        ui.add_space(10.0);

        let plot = Plot::new("chromaticity_plot")
            .data_aspect(1.0)
            .view_aspect(1.0)
            .include_x(0.0)
            .include_x(0.8)
            .include_y(0.0)
            .include_y(0.9)
            .legend(Legend::default())
            .allow_zoom(true)
            .allow_drag(true);

        plot.show(ui, |plot_ui| {
            // 1. Draw Spectral Locus (Horseshoe)
            let mut locus_points = Vec::new();
            for i in 0..41 {
                let sum = X_BAR_2[i] + Y_BAR_2[i] + Z_BAR_2[i];
                if sum > 0.0 {
                    locus_points.push([(X_BAR_2[i] / sum) as f64, (Y_BAR_2[i] / sum) as f64]);
                }
            }
            // Close the horseshoe with the purple line (connect 380nm to 780nm)
            if !locus_points.is_empty() {
                locus_points.push(locus_points[0]);
            }

            plot_ui.line(
                Line::new(PlotPoints::from(locus_points))
                    .color(egui::Color32::from_gray(100))
                    .name("Spectral Locus"),
            );

            // 2. Draw D65 White Point
            let d65_x = 0.31272;
            let d65_y = 0.32903;
            plot_ui.points(
                Points::new(vec![[d65_x, d65_y]])
                    .color(egui::Color32::WHITE)
                    .shape(egui_plot::MarkerShape::Plus)
                    .name("D65"),
            );

            // 3. Draw History Trail (Faded)
            let history_points: Vec<[f64; 2]> = self
                .measurement_history
                .iter()
                .rev() // Draw from oldest to newest
                .map(|e| {
                    let xyz = e.data.to_xyz();
                    let (x, y) = xyz.to_chromaticity();
                    [x as f64, y as f64]
                })
                .collect();

            if history_points.len() > 1 {
                plot_ui.line(
                    Line::new(PlotPoints::from(history_points))
                        .color(egui::Color32::from_rgba_unmultiplied(100, 100, 100, 100))
                        .name("History Path"),
                );
            }

            // 4. Draw Current Point
            if let Some(data) = &self.last_result {
                let xyz = data.to_xyz();
                let (x, y) = xyz.to_chromaticity();
                plot_ui.points(
                    Points::new(vec![[x as f64, y as f64]])
                        .color(egui::Color32::RED)
                        .radius(4.0)
                        .name("Current Entry"),
                );
            }
        });

        ui.add_space(10.0);
        ui.label("The horseshoe-shaped region represents all colors visible to the human eye. The red dot indicates the most recent measurement.");
    }

    fn render_color_quality_tab(&self, ui: &mut egui::Ui) {
        ui.add_space(5.0);
        ui.heading("🌈 IES TM-30-18 Color Quality");
        ui.add_space(10.0);

        if let Some(metrics) = &self.last_tm30 {
            let visualizer = crate::tm30_gui::Tm30Visualizer::new(metrics.clone());
            visualizer.ui(ui);
        } else {
            ui.vertical_centered(|ui| {
                ui.add_space(50.0);
                ui.label("No TM-30 data available.");
                ui.label("Please take an Emissive measurement to see color quality metrics.");
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
                    .fill(egui::Color32::from_rgb(22, 22, 30))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Logo/Title
                    ui.label(egui::RichText::new("🌈 spectro-rs").size(20.0).strong());

                    ui.separator();

                    // Device status
                    if self.is_connected {
                        ui.colored_label(egui::Color32::from_rgb(50, 205, 50), "●");
                        if let Some(ref info) = self.device_info.basic {
                            ui.label(format!("{} ({})", info.model, info.serial));
                        }
                    } else {
                        ui.colored_label(egui::Color32::from_rgb(255, 100, 100), "●");
                        ui.label("Not connected");
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
                            "🔬 Expert"
                        } else {
                            "🎨 Simple"
                        };
                        if ui
                            .selectable_label(self.is_expert_mode, toggle_text)
                            .clicked()
                        {
                            self.is_expert_mode = !self.is_expert_mode;
                        }

                        ui.separator();

                        // Settings button
                        if ui.button("⚙ Settings").clicked() {
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
                    .fill(egui::Color32::from_rgb(22, 22, 30))
                    .inner_margin(egui::Margin::symmetric(12.0, 8.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Mode selector
                    egui::ComboBox::from_id_salt("mode_selector")
                        .selected_text(match self.selected_mode {
                            MeasurementMode::Reflective => "📄 Reflective",
                            MeasurementMode::Emissive => "🖥️ Emissive",
                            MeasurementMode::Ambient => "💡 Ambient",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.selected_mode,
                                MeasurementMode::Reflective,
                                "📄 Reflective (Paper)",
                            );
                            ui.selectable_value(
                                &mut self.selected_mode,
                                MeasurementMode::Emissive,
                                "🖥️ Emissive (Monitor)",
                            );
                            ui.selectable_value(
                                &mut self.selected_mode,
                                MeasurementMode::Ambient,
                                "💡 Ambient (Light)",
                            );
                        });

                    ui.separator();

                    // Main action buttons
                    let measure_btn = ui.add_enabled(
                        !self.is_busy && self.is_connected,
                        egui::Button::new("🚀 Measure").min_size(egui::vec2(100.0, 30.0)),
                    );
                    if measure_btn.clicked() {
                        self.is_busy = true;
                        self.cmd_tx
                            .send(DeviceCommand::Measure(self.selected_mode))
                            .ok();
                    }

                    let cal_btn = ui.add_enabled(
                        !self.is_busy && self.is_connected,
                        egui::Button::new("🎯 Calibrate").min_size(egui::vec2(100.0, 30.0)),
                    );
                    if cal_btn.clicked() {
                        self.calibration_wizard.start();
                    }

                    // Continuous measurement toggle
                    let continuous_label = if self.is_continuous {
                        "⏸️ Stop Loop"
                    } else {
                        "▶️ Continuous"
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
                                .text("interval (s)")
                                .step_by(0.1),
                        );
                    }

                    // Reconnect button (only shown when disconnected)
                    if !self.is_connected && ui.button("🔌 Reconnect").clicked() {
                        self.is_busy = true;
                        self.cmd_tx.send(DeviceCommand::Connect).ok();
                    }

                    ui.separator();

                    // Calibration status indicator
                    let (cal_color, cal_text) = if self.is_calibrated {
                        (egui::Color32::from_rgb(50, 205, 50), "✓ Calibrated")
                    } else {
                        (egui::Color32::from_rgb(255, 193, 7), "⚠ Needs Calibration")
                    };
                    ui.colored_label(cal_color, cal_text);

                    // Right side: Reference input toggle
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui
                            .button(if self.reference_lab.is_some() {
                                "📌 Reference Set"
                            } else {
                                "📌 Set Reference"
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
            egui::Window::new("⚙ Settings")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.heading("Colorimetry Standards");
                    ui.add_space(10.0);

                    egui::Grid::new("settings_grid")
                        .num_columns(2)
                        .spacing([20.0, 10.0])
                        .show(ui, |ui| {
                            ui.label("Illuminant:");
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

                            ui.label("Observer:");
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
                    ui.add_space(10.0);

                    ui.horizontal(|ui| {
                        if ui.button("Close").clicked() {
                            self.show_settings = false;
                        }
                    });
                });
        }

        // === Reference Input Window (Modal-like) ===
        if self.show_reference_input {
            egui::Window::new("📌 Set Reference Color")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Enter the target Lab values for comparison:");
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
                        if ui.button("✓ Set").clicked() {
                            self.reference_lab = Some(Lab {
                                l: self.ref_input_l,
                                a: self.ref_input_a,
                                b: self.ref_input_b,
                            });
                            self.show_reference_input = false;
                        }
                        if ui.button("Use Current").clicked() {
                            if let Some(lab) = self.get_current_lab() {
                                self.ref_input_l = lab.l;
                                self.ref_input_a = lab.a;
                                self.ref_input_b = lab.b;
                                self.reference_lab = Some(lab);
                                self.show_reference_input = false;
                            }
                        }
                        if ui.button("Clear").clicked() {
                            self.reference_lab = None;
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_reference_input = false;
                        }
                    });
                });
        }

        // === Left Panel: History (Expert mode only) ===
        if self.is_expert_mode && !self.measurement_history.is_empty() {
            egui::SidePanel::left("history_panel")
                .resizable(true)
                .default_width(200.0)
                .min_width(150.0)
                .show(ctx, |ui| {
                    ui.heading("📋 History");
                    ui.separator();

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for (idx, entry) in self.measurement_history.iter().enumerate() {
                            let lab = &entry.lab;
                            let xyz = entry.data.to_xyz();
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
                                            egui::Color32::GREEN
                                        } else {
                                            egui::Color32::RED
                                        };
                                        ui.colored_label(
                                            color,
                                            egui::RichText::new(format!("ΔE00={:.1}", de)).small(),
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
                });
        }

        // === Right Panel: Expert Inspector ===
        if self.is_expert_mode {
            egui::SidePanel::right("expert_panel")
                .resizable(true)
                .default_width(280.0)
                .min_width(200.0)
                .show(ctx, |ui| {
                    self.render_expert_inspector(ui);
                });
        }

        // === Central Panel: Main Workspace ===
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(18, 18, 24))
                    .inner_margin(egui::Margin::same(16.0)),
            )
            .show(ctx, |ui| {
                if self.is_expert_mode {
                    self.render_expert_workspace(ui);
                } else {
                    self.render_simple_workspace(ui);
                }

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
