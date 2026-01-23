use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use spectro_rs::colorimetry::curves::{DisplayCalibrator, VideoCal};
use spectro_rs::colorimetry::{XYZ, illuminant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CalStep {
    #[default]
    Intro,
    Setup,
    Measure,
    Result,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationAction {
    RequestMeasurement,
    None,
}

pub struct DisplayCalibrationState {
    pub step: CalStep,
    pub current_patch_index: usize,
    pub total_patches: usize,
    pub target_gamma: f32,
    pub measurements: Vec<(f32, XYZ)>,
    pub generated_cal: Option<VideoCal>,
    pub is_measuring: bool,
    pub auto_advance: bool,
}

impl Default for DisplayCalibrationState {
    fn default() -> Self {
        Self {
            step: CalStep::Intro,
            current_patch_index: 0,
            total_patches: 17, // 0, 16, 32... 255
            target_gamma: 2.2,
            measurements: Vec::new(),
            generated_cal: None,
            is_measuring: false,
            auto_advance: false,
        }
    }
}

impl DisplayCalibrationState {
    pub fn handle_result(&mut self, xyz: XYZ) {
        if self.step == CalStep::Measure && self.is_measuring {
            let current_level = self.current_patch_index as f32 / (self.total_patches - 1) as f32;
            self.measurements.push((current_level, xyz));
            self.current_patch_index += 1;
            self.is_measuring = false;

            if self.current_patch_index >= self.total_patches {
                generate_calibration(self);
                self.step = CalStep::Result;
                self.auto_advance = false;
            }
        }
    }
}

pub struct CalibrationViewContext<'a> {
    pub layout: &'a crate::theme::LayoutConfig,
    pub state: &'a mut DisplayCalibrationState,
    pub is_connected: bool,
    pub is_busy: bool,
}

pub fn render_calibration_view(
    ui: &mut egui::Ui,
    ctx: &mut CalibrationViewContext,
) -> CalibrationAction {
    match ctx.state.step {
        CalStep::Intro => render_intro(ui, ctx),
        CalStep::Setup => render_setup(ui, ctx),
        CalStep::Measure => render_measure(ui, ctx),
        CalStep::Result => render_result(ui, ctx),
    }
}

fn render_intro(ui: &mut egui::Ui, ctx: &mut CalibrationViewContext) -> CalibrationAction {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new("🖥️").size(64.0));
            ui.heading("Display Calibration");
            ui.add_space(ctx.layout.spacing * 2.0);
            ui.label("This wizard will guide you through calibrating your display.");
            ui.label("It measures the display's response and generates a corrective LUT.");
            ui.add_space(ctx.layout.spacing * 2.0);

            if !ctx.is_connected {
                ui.colored_label(
                    egui::Color32::RED,
                    "Please connect your spectrometer first.",
                );
            }

            let start_btn = ui.add_enabled(
                ctx.is_connected,
                egui::Button::new("Begin Setup").min_size(egui::vec2(120.0, 40.0)),
            );
            if start_btn.clicked() {
                ctx.state.step = CalStep::Setup;
            }
        });
    });
    CalibrationAction::None
}

fn render_setup(ui: &mut egui::Ui, ctx: &mut CalibrationViewContext) -> CalibrationAction {
    ui.vertical_centered(|ui| {
        ui.heading("Calibration Settings");
        ui.add_space(20.0);

        egui::Grid::new("cal_settings_grid")
            .spacing([20.0, 10.0])
            .show(ui, |ui| {
                ui.label("Target Gamma:");
                ui.add(egui::Slider::new(&mut ctx.state.target_gamma, 1.0..=3.0).step_by(0.1));
                ui.end_row();

                ui.label("White Point:");
                ui.label("D65 (Fixed for now)");
                ui.end_row();

                ui.label("Samples:");
                ui.add(egui::Slider::new(&mut ctx.state.total_patches, 2..=33));
                ui.end_row();
            });

        ui.add_space(40.0);

        ui.horizontal(|ui| {
            if ui.button("<< Back").clicked() {
                ctx.state.step = CalStep::Intro;
            }
            if ui
                .add_enabled(ctx.is_connected, egui::Button::new("Start Measuring >>"))
                .clicked()
            {
                ctx.state.measurements.clear();
                ctx.state.current_patch_index = 0;
                ctx.state.step = CalStep::Measure;
            }
        });
    });
    CalibrationAction::None
}

fn render_measure(ui: &mut egui::Ui, ctx: &mut CalibrationViewContext) -> CalibrationAction {
    let current_level = ctx.state.current_patch_index as f32 / (ctx.state.total_patches - 1) as f32;
    let gray_val = (current_level * 255.0) as u8;
    let mut action = CalibrationAction::None;

    ui.vertical_centered(|ui| {
        ui.heading(format!(
            "Step {} of {}",
            ctx.state.current_patch_index + 1,
            ctx.state.total_patches
        ));

        ui.label(format!(
            "Drive Level: RGB({}, {}, {})",
            gray_val, gray_val, gray_val
        ));

        // Draw the color patch - Fullscreen-ish or at least large
        let size = egui::vec2(300.0, 300.0);
        let (rect, _response) = ui.allocate_at_least(size, egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 0.0, egui::Color32::from_gray(gray_val));

        ui.add_space(20.0);

        if ctx.is_busy {
            ui.spinner();
            ui.label("Measuring...");
        } else {
            // If we're in auto mode and not finished, trigger next measurement
            if ctx.state.auto_advance && ctx.state.step == CalStep::Measure {
                ctx.state.is_measuring = true;
                action = CalibrationAction::RequestMeasurement;
            }

            ui.horizontal(|ui| {
                if ui.button("📸 Measure Single").clicked() {
                    ctx.state.is_measuring = true;
                    ctx.state.auto_advance = false;
                    action = CalibrationAction::RequestMeasurement;
                }

                if ui.button("🚀 Auto Measure").clicked() {
                    ctx.state.is_measuring = true;
                    ctx.state.auto_advance = true;
                    action = CalibrationAction::RequestMeasurement;
                }

                if ui.button("💾 Simulate (Random)").clicked() {
                    ctx.state.auto_advance = false;
                    let simulated_y =
                        current_level.powf(2.4) * 120.0 + (rand::random::<f32>() * 2.0);
                    let simulated_xyz = XYZ {
                        x: simulated_y * 0.95,
                        y: simulated_y,
                        z: simulated_y * 1.08,
                    };

                    ctx.state.measurements.push((current_level, simulated_xyz));
                    ctx.state.current_patch_index += 1;

                    if ctx.state.current_patch_index >= ctx.state.total_patches {
                        generate_calibration(ctx.state);
                        ctx.state.step = CalStep::Result;
                    }
                }
            });
        }

        if ui.button("Back to Setup").clicked() {
            ctx.state.step = CalStep::Setup;
            ctx.state.is_measuring = false;
            ctx.state.auto_advance = false;
        }
    });

    action
}

fn generate_calibration(state: &mut DisplayCalibrationState) {
    let mut calibrator = DisplayCalibrator::new(state.target_gamma, illuminant::D65);
    for (input, xyz) in &state.measurements {
        calibrator.add_measurement(*input, *xyz);
    }
    state.generated_cal = Some(calibrator.generate_cal(256));
}

fn render_result(ui: &mut egui::Ui, ctx: &mut CalibrationViewContext) -> CalibrationAction {
    ui.vertical_centered(|ui| {
        ui.heading("Calibration Complete");
        ui.add_space(10.0);

        if let Some(cal) = &ctx.state.generated_cal {
            ui.label("Generated Video LUT (256 steps)");

            // Plot the curves
            let r_points: PlotPoints = cal
                .r
                .values
                .iter()
                .enumerate()
                .map(|(i, &v)| [i as f64 / 255.0, v as f64])
                .collect();

            Plot::new("cal_plot")
                .view_aspect(1.0)
                .height(300.0)
                .show(ui, |plot_ui| {
                    plot_ui.line(
                        Line::new(r_points)
                            .color(egui::Color32::WHITE)
                            .name("Curve"),
                    );
                    // Draw identity for reference
                    plot_ui.line(
                        Line::new(PlotPoints::from_iter(vec![[0.0, 0.0], [1.0, 1.0]]))
                            .color(egui::Color32::DARK_GRAY)
                            .style(egui_plot::LineStyle::Solid),
                    );
                });
        }

        ui.add_space(20.0);
        if ui.button("Export .cal file").clicked() {
            // Implementation for exporting CGATS .cal
        }

        if ui.button("Restart").clicked() {
            ctx.state.step = CalStep::Intro;
            ctx.state.measurements.clear();
            ctx.state.current_patch_index = 0;
            ctx.state.is_measuring = false;
        }
    });
    CalibrationAction::None
}
