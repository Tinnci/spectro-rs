//! Device Inspector panel component for spectro-gui.
//!
//! This module provides the Device Inspector side panel which shows detailed
//! device information, raw sensor data, algorithm details, chromaticity diagram,
//! color quality metrics (TM-30), and trend analysis.
//!
//! # Architecture
//! The inspector is a **passive view component** that receives immutable references
//! to the application state and renders the appropriate UI. It maintains its own
//! tab selection state but does not own any measurement data.

use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints, Points};

use crate::shared::{ExtendedDeviceInfo, MeasurementEntry};
use crate::t;
use crate::theme::{error_color, muted_text_color, plot_line_color, success_color, warning_color};
use spectro_rs::colorimetry::{X_BAR_2, XYZ, Y_BAR_2, Z_BAR_2, illuminant};
use spectro_rs::spectrum::MeasurementResult;
use spectro_rs::tm30::TM30Metrics;

// ============================================================================
// Inspector State
// ============================================================================

/// Tabs available in the Device Inspector panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InspectorTab {
    /// Device information and EEPROM calibration data
    #[default]
    DeviceInfo,
    /// Raw spectral sensor values
    RawSensor,
    /// Algorithm and color conversion pipeline details
    Algorithm,
    /// CIE 1931 xy chromaticity diagram
    Chromaticity,
    /// TM-30 color quality metrics
    ColorQuality,
    /// Trend analysis over measurement history
    Trend,
}

/// Device Inspector panel state and rendering logic.
///
/// This component is responsible for rendering the right-side inspector panel
/// in expert mode. It displays detailed technical information about the device
/// and measurements.
pub struct DeviceInspector {
    /// Currently selected tab
    pub tab: InspectorTab,
    /// Whether the panel is visible
    pub visible: bool,
}

impl Default for DeviceInspector {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceInspector {
    /// Create a new DeviceInspector with default state.
    pub fn new() -> Self {
        Self {
            tab: InspectorTab::default(),
            visible: true,
        }
    }

    /// Toggle panel visibility.
    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    /// Render the inspector panel content.
    ///
    /// # Arguments
    /// * `ui` - The egui UI context
    /// * `ctx` - Inspector rendering context containing all required data
    pub fn render(&mut self, ui: &mut egui::Ui, ctx: &InspectorContext) {
        // Header with title and close button
        ui.horizontal(|ui| {
            ui.heading(t!("gui-device-inspector"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⏵").on_hover_text(t!("gui-hide")).clicked() {
                    self.visible = false;
                }
            });
        });
        ui.add_space(10.0);

        // Tab bar
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.tab,
                InspectorTab::DeviceInfo,
                format!("📱 {}", t!("gui-device")),
            );
            ui.selectable_value(
                &mut self.tab,
                InspectorTab::RawSensor,
                format!("📈 {}", t!("gui-raw-data")),
            );
            ui.selectable_value(
                &mut self.tab,
                InspectorTab::Algorithm,
                format!("🧮 {}", t!("gui-algorithm")),
            );
            ui.selectable_value(
                &mut self.tab,
                InspectorTab::Chromaticity,
                format!("🎯 {}", t!("gui-xy-diagram")),
            );
            ui.selectable_value(
                &mut self.tab,
                InspectorTab::ColorQuality,
                format!("🌈 {}", t!("gui-color-quality")),
            );
            ui.selectable_value(
                &mut self.tab,
                InspectorTab::Trend,
                format!("📈 {}", t!("gui-trend")),
            );
        });

        ui.separator();

        // Handle empty state for tabs that need data
        let needs_data = matches!(
            self.tab,
            InspectorTab::RawSensor
                | InspectorTab::Algorithm
                | InspectorTab::Chromaticity
                | InspectorTab::ColorQuality
                | InspectorTab::Trend
        );

        if needs_data && ctx.last_result.is_none() && ctx.history.is_empty() {
            self.render_empty_state(ui);
        } else {
            // Render the selected tab
            match self.tab {
                InspectorTab::DeviceInfo => {
                    egui::ScrollArea::vertical()
                        .show(ui, |ui| self.render_device_info_tab(ui, ctx));
                }
                InspectorTab::RawSensor => self.render_raw_sensor_tab(ui, ctx),
                InspectorTab::Algorithm => {
                    egui::ScrollArea::vertical().show(ui, |ui| self.render_algorithm_tab(ui, ctx));
                }
                InspectorTab::Chromaticity => self.render_chromaticity_tab(ui, ctx),
                InspectorTab::ColorQuality => self.render_color_quality_tab(ui, ctx),
                InspectorTab::Trend => self.render_trend_tab(ui, ctx),
            }
        }
    }

    /// Render centered empty state message.
    fn render_empty_state(&self, ui: &mut egui::Ui) {
        // Use available space to center the message both vertically and horizontally
        let available = ui.available_size();

        // Allocate the full available space
        ui.allocate_ui_with_layout(
            available,
            egui::Layout::centered_and_justified(egui::Direction::TopDown),
            |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(t!("gui-no-measurement")).weak());
                });
            },
        );
    }

    // ========================================================================
    // Tab Renderers
    // ========================================================================

    fn render_device_info_tab(&self, ui: &mut egui::Ui, ctx: &InspectorContext) {
        ui.add_space(5.0);

        // Basic Device Info
        ui.collapsing(t!("gui-device-info"), |ui| {
            egui::Grid::new("device_info_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    if let Some(ref basic) = ctx.device_info.basic {
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
                        ui.label(t!("gui-status"));
                        ui.colored_label(
                            warning_color(&ui.ctx().style().visuals),
                            t!("gui-not-connected"),
                        );
                        ui.end_row();
                    }

                    if let Some(cal_ver) = ctx.device_info.cal_version {
                        ui.label("Cal Version:");
                        ui.label(format!("0x{:04X}", cal_ver));
                        ui.end_row();
                    }
                });
        });

        // EEPROM Calibration Data
        ui.collapsing(t!("gui-eeprom-cal"), |ui| {
            if let Some(ref white_ref) = ctx.device_info.white_ref {
                ui.label(t!("gui-white-ref"));

                // Mini plot of white reference
                let plot = Plot::new("white_ref_plot")
                    .height(100.0)
                    .show_axes([true, true])
                    .include_y(0.0);

                let visuals = ui.ctx().style().visuals.clone();
                plot.show(ui, |plot_ui| {
                    let points: PlotPoints = white_ref
                        .iter()
                        .enumerate()
                        .map(|(i, v)| [(380 + i * 10) as f64, *v as f64])
                        .collect();
                    plot_ui.line(
                        Line::new(points)
                            .color(plot_line_color(&visuals))
                            .width(1.5),
                    );
                });
            } else {
                ui.colored_label(
                    muted_text_color(&ui.ctx().style().visuals),
                    t!("gui-white-ref-not-avail"),
                );
            }

            ui.add_space(5.0);

            // Emissive calibration coefficients
            if let Some(ref emis) = ctx.device_info.emis_coef {
                ui.collapsing(t!("gui-emissive-coef"), |ui| {
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
            if let Some(ref amb) = ctx.device_info.amb_coef {
                ui.collapsing(t!("gui-ambient-coef"), |ui| {
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
            if let Some(ref lin) = ctx.device_info.lin_normal {
                ui.label(format!("Lin (Normal): {:?}", lin));
            }
            if let Some(ref lin) = ctx.device_info.lin_high {
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
                    if ctx.is_connected {
                        ui.colored_label(success_color(&ui.ctx().style().visuals), "Yes ✓");
                    } else {
                        ui.colored_label(error_color(&ui.ctx().style().visuals), "No ✗");
                    }
                    ui.end_row();

                    ui.label("Calibrated:");
                    if ctx.is_calibrated {
                        ui.colored_label(success_color(&ui.ctx().style().visuals), "Yes ✓");
                    } else {
                        ui.colored_label(warning_color(&ui.ctx().style().visuals), "No");
                    }
                    ui.end_row();

                    ui.label("Mode:");
                    ui.label(format!("{:?}", ctx.selected_mode));
                    ui.end_row();
                });
        });
    }

    fn render_raw_sensor_tab(&self, ui: &mut egui::Ui, ctx: &InspectorContext) {
        ui.add_space(5.0);

        if let Some(data) = ctx.last_result {
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
                            for i in (0..data.spectrum.values.len()).step_by(2) {
                                let wl1 = 380 + i * 10;
                                ui.label(format!("{}", wl1));
                                ui.label(format!("{:.6}", data.spectrum.values[i]));

                                if i + 1 < data.spectrum.values.len() {
                                    let wl2 = 380 + (i + 1) * 10;
                                    ui.label(format!("{}", wl2));
                                    ui.label(format!("{:.6}", data.spectrum.values[i + 1]));
                                }
                                ui.end_row();
                            }
                        });
                });

            ui.add_space(10.0);

            // Statistics
            ui.collapsing("📊 Statistics", |ui| {
                let values = &data.spectrum.values;
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
        }
    }

    fn render_algorithm_tab(&self, ui: &mut egui::Ui, ctx: &InspectorContext) {
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

        if let Some(data) = ctx.last_result {
            ui.collapsing("🧪 Current Calculation", |ui| {
                let xyz = data.xyz;
                let xyz_norm = XYZ {
                    x: xyz.x / 100.0,
                    y: xyz.y / 100.0,
                    z: xyz.z / 100.0,
                };
                let lab = xyz_norm.to_lab(illuminant::D65_2);

                ui.label(format!("Mode: {:?}", data.spectrum.mode));
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

    fn render_chromaticity_tab(&self, ui: &mut egui::Ui, ctx: &InspectorContext) {
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

        let visuals = ui.ctx().style().visuals.clone();
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
                    .color(plot_line_color(&visuals))
                    .shape(egui_plot::MarkerShape::Plus)
                    .name("D65"),
            );

            // 3. Draw History Trail (Faded)
            let history_points: Vec<[f64; 2]> = ctx
                .history
                .iter()
                .rev() // Draw from oldest to newest
                .map(|e| {
                    let xyz = e.result.xyz;
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
            if let Some(data) = ctx.last_result {
                let xyz = data.xyz;
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

    fn render_color_quality_tab(&self, ui: &mut egui::Ui, ctx: &InspectorContext) {
        ui.add_space(5.0);
        ui.heading(t!("gui-color-quality-tm30"));
        ui.add_space(10.0);

        if let Some(metrics) = ctx.last_tm30 {
            let visualizer = crate::tm30_gui::Tm30Visualizer::new(metrics.clone());
            visualizer.ui(ui);
        } else if ctx.last_result.is_some() {
            // Centered message for TM-30 not available
            let available = ui.available_size();
            ui.allocate_ui_with_layout(
                available,
                egui::Layout::centered_and_justified(egui::Direction::TopDown),
                |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("No TM-30 data available.").weak());
                        ui.label(
                            egui::RichText::new(
                                "Please take an Emissive measurement to see color quality metrics.",
                            )
                            .small()
                            .weak(),
                        );
                    });
                },
            );
        }
    }

    fn render_trend_tab(&self, ui: &mut egui::Ui, ctx: &InspectorContext) {
        ui.add_space(5.0);

        if ctx.history.is_empty() {
            self.render_empty_state(ui);
            return;
        }

        ui.heading("📈 Measurement Trend");
        ui.add_space(10.0);

        // L* trend
        let l_points: PlotPoints = ctx
            .history
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let xyz = entry.result.xyz;
                let xyz_norm = XYZ {
                    x: xyz.x / 100.0,
                    y: xyz.y / 100.0,
                    z: xyz.z / 100.0,
                };
                let lab = xyz_norm.to_lab(illuminant::D65_2);
                [i as f64, lab.l as f64]
            })
            .collect();

        let plot = Plot::new("trend_plot")
            .height(150.0)
            .show_axes([true, true])
            .legend(Legend::default());

        let visuals = ui.ctx().style().visuals.clone();
        plot.show(ui, |plot_ui| {
            plot_ui.line(
                Line::new(l_points)
                    .color(plot_line_color(&visuals))
                    .name("L*"),
            );
        });

        ui.add_space(10.0);

        // Statistics summary
        if let Some(last) = ctx.history.last() {
            let xyz = last.result.xyz;
            let xyz_norm = XYZ {
                x: xyz.x / 100.0,
                y: xyz.y / 100.0,
                z: xyz.z / 100.0,
            };
            let lab = xyz_norm.to_lab(illuminant::D65_2);

            egui::Grid::new("trend_stats_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Latest L*:");
                    ui.label(format!("{:.2}", lab.l));
                    ui.end_row();
                    ui.label("Latest a*:");
                    ui.label(format!("{:.2}", lab.a));
                    ui.end_row();
                    ui.label("Latest b*:");
                    ui.label(format!("{:.2}", lab.b));
                    ui.end_row();
                    ui.label("Total Measurements:");
                    ui.label(format!("{}", ctx.history.len()));
                    ui.end_row();
                });
        }
    }
}

// ============================================================================
// Inspector Context
// ============================================================================

/// Context data required for rendering the Device Inspector.
///
/// This struct bundles all the immutable references needed by the inspector
/// to render its content. Using a context struct keeps the API clean and
/// makes it easy to add new data dependencies.
pub struct InspectorContext<'a> {
    /// Extended device information including EEPROM data
    pub device_info: &'a ExtendedDeviceInfo,
    /// Whether the device is currently connected
    pub is_connected: bool,
    /// Whether the device has been calibrated
    pub is_calibrated: bool,
    /// Currently selected measurement mode
    pub selected_mode: spectro_rs::MeasurementMode,
    /// Most recent measurement result
    pub last_result: Option<&'a MeasurementResult>,
    /// Most recent TM-30 metrics (for emissive measurements)
    pub last_tm30: Option<&'a TM30Metrics>,
    /// Measurement history
    pub history: &'a [MeasurementEntry],
}
