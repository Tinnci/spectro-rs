use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use spectro_rs::colorimetry::curves::{CalibrationSession, VideoCal};
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CalibrationTarget {
    #[default]
    None,
    White,
    Black,
    Ramp,
}

pub struct DisplayCalibrationState {
    pub step: CalStep,
    pub target_gamma: f32,
    pub total_patches: usize,
    pub session: Option<CalibrationSession>,
    pub generated_cal: Option<VideoCal>,
    pub white_luminance: Option<f32>,
    pub black_luminance: Option<f32>,
    pub last_measured_y: Option<f32>,
    pub current_target: CalibrationTarget,
    pub is_measuring: bool,
    pub auto_advance: bool,
}

impl Default for DisplayCalibrationState {
    fn default() -> Self {
        Self {
            step: CalStep::Intro,
            target_gamma: 2.2,
            total_patches: 17,
            session: None,
            generated_cal: None,
            white_luminance: None,
            black_luminance: None,
            last_measured_y: None,
            current_target: CalibrationTarget::None,
            is_measuring: false,
            auto_advance: false,
        }
    }
}

impl DisplayCalibrationState {
    pub fn handle_result(&mut self, xyz: XYZ) {
        match self.current_target {
            CalibrationTarget::White => {
                self.white_luminance = Some(xyz.y);
                self.is_measuring = false;
                self.current_target = CalibrationTarget::None;
            }
            CalibrationTarget::Black => {
                self.black_luminance = Some(xyz.y);
                self.is_measuring = false;
                self.current_target = CalibrationTarget::None;
            }
            CalibrationTarget::Ramp => {
                if let (Some(session), true) = (
                    &mut self.session,
                    self.step == CalStep::Measure && self.is_measuring,
                ) {
                    self.last_measured_y = Some(xyz.y);
                    session.add_measurement(xyz);
                    self.is_measuring = false;

                    if session.is_complete() {
                        self.generated_cal = Some(session.generate_cal());
                        self.step = CalStep::Result;
                        self.auto_advance = false;
                    }
                }
            }
            CalibrationTarget::None => {
                self.is_measuring = false;
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
    // 1. Render the active view
    let action = match ctx.state.step {
        CalStep::Intro => render_intro(ui, ctx),
        CalStep::Setup => render_setup(ui, ctx),
        CalStep::Measure => render_measure(ui, ctx),
        CalStep::Result => render_result(ui, ctx),
    };

    // 2. Global Overlay Rendering (Top-Most Layer)
    // We handle overlay logic here to ensure it covers EVERYTHING, including headers/sidebars if possible,
    // and to share logic between Setup (ref measure) and Measure (ramp measure) steps.
    let overlay_color = if ctx.state.is_measuring {
        match ctx.state.current_target {
            CalibrationTarget::White => Some(egui::Color32::WHITE),
            CalibrationTarget::Black => Some(egui::Color32::BLACK),
            CalibrationTarget::Ramp => {
                // Get current ramp color from session
                if let Some(session) = &ctx.state.session {
                    let level = session.current_level().unwrap_or(0.0);
                    let val = (level * 255.0) as u8;
                    Some(egui::Color32::from_gray(val))
                } else {
                    None
                }
            }
            CalibrationTarget::None => None,
        }
    } else {
        None
    };

    if let Some(color) = overlay_color {
        // Use the Debug layer which is always on top of CentralPanel, Windows, and Areas
        let painter = ui.ctx().layer_painter(egui::LayerId::debug());
        let screen_rect = ui.ctx().input(|i| i.screen_rect());

        // Fill entire screen
        painter.rect_filled(screen_rect, 0.0, color);

        // Draw status text in the center
        // We need to manually paint text since we are bypassing the layout system
        let text_color = if color.r() > 100 {
            egui::Color32::BLACK
        } else {
            egui::Color32::WHITE
        };

        let status_text = match ctx.state.current_target {
            CalibrationTarget::Ramp => {
                if let Some(session) = &ctx.state.session {
                    let (idx, total) = session.progress();
                    format!("Measuring Step {}/{}\nRGB: {}", idx + 1, total, color.r())
                } else {
                    "Measuring...".to_string()
                }
            }
            _ => "📷 Measuring Reference...".to_string(),
        };

        // Paint text at center
        let font_id = egui::FontId::proportional(24.0);
        painter.text(
            screen_rect.center(),
            egui::Align2::CENTER_CENTER,
            status_text,
            font_id,
            text_color,
        );

        // Hide cursor during measurement for full immersion
        ui.ctx()
            .output_mut(|o| o.cursor_icon = egui::CursorIcon::None);
    }

    action
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
    let mut action = CalibrationAction::None;
    let visuals = ui.ctx().style().visuals.clone();

    ui.vertical_centered(|ui| {
        ui.heading("Calibration Settings");
        ui.add_space(ctx.layout.spacing);

        // --- Bento Grid Layout ---
        ui.horizontal(|ui| {
            // Card 1: Parameters
            render_card(ui, "🎯 Target Parameters", 200.0, |ui| {
                ui.vertical(|ui| {
                    ui.label("Gamma:");
                    ui.add(egui::Slider::new(&mut ctx.state.target_gamma, 1.0..=3.0).step_by(0.1));
                    ui.add_space(8.0);
                    ui.label("Samples:");
                    ui.add(egui::Slider::new(&mut ctx.state.total_patches, 2..=33));
                });
            });

            ui.add_space(ctx.layout.spacing);

            // Card 2: White Reference
            render_card(ui, "⚪ White Reference", 180.0, |ui| {
                ui.vertical_centered(|ui| {
                    if let Some(w) = ctx.state.white_luminance {
                        ui.heading(format!("{:.1}", w));
                        ui.label("cd/m²");
                    } else {
                        ui.label("Not Measured");
                    }
                    ui.add_space(8.0);
                    if ui.button("📸 Measure").clicked() {
                        ctx.state.is_measuring = true;
                        ctx.state.current_target = CalibrationTarget::White;
                        action = CalibrationAction::RequestMeasurement;
                    }
                });
            });

            ui.add_space(ctx.layout.spacing);

            // Card 3: Black Reference
            render_card(ui, "⚫ Black Reference", 180.0, |ui| {
                ui.vertical_centered(|ui| {
                    if let Some(b) = ctx.state.black_luminance {
                        ui.heading(format!("{:.3}", b));
                        ui.label("cd/m²");
                    } else {
                        ui.label("Not Measured");
                    }
                    ui.add_space(8.0);
                    if ui.button("📸 Measure").clicked() {
                        ctx.state.is_measuring = true;
                        ctx.state.current_target = CalibrationTarget::Black;
                        action = CalibrationAction::RequestMeasurement;
                    }
                });
            });
        });

        ui.add_space(40.0);

        ui.horizontal(|ui| {
            if ui.button("<< Back").clicked() {
                ctx.state.step = CalStep::Intro;
            }
            let is_ready = ctx.is_connected && ctx.state.white_luminance.is_some();
            let start_btn = egui::Button::new("Start Device Characteristic >>")
                .min_size(egui::vec2(180.0, 40.0))
                .fill(if is_ready {
                    crate::theme::success_color(&visuals)
                } else {
                    visuals.widgets.inactive.bg_fill
                });

            if ui.add_enabled(is_ready, start_btn).clicked() {
                ctx.state.session = Some(CalibrationSession::new(
                    ctx.state.target_gamma,
                    illuminant::D65,
                    ctx.state.total_patches,
                ));
                ctx.state.current_target = CalibrationTarget::Ramp;
                ctx.state.step = CalStep::Measure;
            }
        });
        if ctx.state.white_luminance.is_none() {
            ui.add_space(8.0);
            ui.colored_label(egui::Color32::KHAKI, "⚠ Please measure white point first.");
        }
    });
    action
}

fn render_card<F>(ui: &mut egui::Ui, title: &str, min_height: f32, add_contents: F)
where
    F: FnOnce(&mut egui::Ui),
{
    let visuals = &ui.ctx().style().visuals;
    egui::Frame::none()
        .fill(crate::theme::info_panel_color(visuals))
        .rounding(12.0)
        .stroke(egui::Stroke::new(1.0, crate::theme::border_color(visuals)))
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            ui.set_min_height(min_height);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(title).strong().size(14.0));
                ui.add_space(8.0);
                add_contents(ui);
            });
        });
}

fn render_measure(ui: &mut egui::Ui, ctx: &mut CalibrationViewContext) -> CalibrationAction {
    let session = match &ctx.state.session {
        Some(s) => s,
        None => return CalibrationAction::None,
    };

    let current_level = session.current_level().unwrap_or(0.0);
    let (current_idx, total) = session.progress();
    let gray_val = (current_level * 255.0) as u8;
    let mut action = CalibrationAction::None;

    ui.vertical_centered(|ui| {
        ui.heading(format!("Step {} of {}", current_idx + 1, total));

        ui.label(format!(
            "Drive Level: RGB({}, {}, {})",
            gray_val, gray_val, gray_val
        ));

        if let Some(y) = ctx.state.last_measured_y {
            ui.label(
                egui::RichText::new(format!("Current: {:.2} cd/m²", y))
                    .color(egui::Color32::LIGHT_BLUE),
            );
        }

        // Static preview patch (only visible when NOT measuring, since overlay covers it otherwise)
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
                ctx.state.current_target = CalibrationTarget::Ramp;
                action = CalibrationAction::RequestMeasurement;
            }

            ui.horizontal(|ui| {
                if ui.button("📸 Measure Single").clicked() {
                    ctx.state.is_measuring = true;
                    ctx.state.auto_advance = false;
                    ctx.state.current_target = CalibrationTarget::Ramp;
                    action = CalibrationAction::RequestMeasurement;
                }

                if ui.button("🚀 Auto Measure").clicked() {
                    ctx.state.is_measuring = true;
                    ctx.state.auto_advance = true;
                    ctx.state.current_target = CalibrationTarget::Ramp;
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

                    if let Some(s) = &mut ctx.state.session {
                        ctx.state.last_measured_y = Some(simulated_xyz.y);
                        s.add_measurement(simulated_xyz);
                        if s.is_complete() {
                            ctx.state.generated_cal = Some(s.generate_cal());
                            ctx.state.step = CalStep::Result;
                        }
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
            ctx.state.session = None;
            ctx.state.is_measuring = false;
        }
    });
    CalibrationAction::None
}
