use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use spectro_rs::colorimetry::XYZ;

use crate::calibration::{CalibrationFlowStep, CalibrationTarget, DisplayCalibrationManager};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CalibrationAction {
    RequestMeasurement,
    Close,
    None,
}

#[derive(Default)]
pub struct DisplayCalibrationView {
    pub manager: DisplayCalibrationManager,
}

impl DisplayCalibrationView {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn handle_measurement(&mut self, xyz: XYZ) {
        self.manager.handle_measurement(xyz);
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &DisplayCalibrationContext,
    ) -> CalibrationAction {
        let mut action = CalibrationAction::None;

        // 2. Global Overlay Rendering (Top-Most Layer)
        render_overlay(ui, ctx, &self.manager);

        // 3. Top Header (Manually rendered to avoid nested Panel issues)
        egui::Frame::none()
            .fill(crate::theme::panel_bg_color(&ui.ctx().style().visuals))
            .inner_margin(egui::Margin::symmetric(12.0, 8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("🖥️ Display Calibration")
                            .size(16.0)
                            .strong(),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✖ Close").clicked() {
                            action = CalibrationAction::Close;
                        }
                    });
                });
            });

        if action == CalibrationAction::Close {
            return action;
        }

        // 4. Main Content (Fill remaining space)

        // Use embedded panels for robust layout (Sidebar + Central Content)
        egui::SidePanel::left("cal_workflow_sidebar")
            .resizable(false)
            .exact_width(200.0)
            .frame(egui::Frame::none())
            .show_inside(ui, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.set_width(ui.available_width());

                        // Workflow Header
                        egui::Frame::none()
                            .fill(crate::theme::panel_bg_dark_color(&ui.ctx().style().visuals))
                            .inner_margin(8.0)
                            .rounding(4.0)
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.heading("Workflow");
                            });

                        ui.add_space(10.0);

                        let steps = [
                            (CalibrationFlowStep::Intro, "🏠 Introduction"),
                            (CalibrationFlowStep::Setup, "⚙️ Setup"),
                            (CalibrationFlowStep::Measure, "📏 Measurement"),
                            (CalibrationFlowStep::Result, "📊 Summary"),
                        ];

                        for (step, label) in steps {
                            let is_active = self.manager.step == step;
                            let is_done = (self.manager.step as u8) > (step as u8);

                            ui.horizontal(|ui| {
                                let text = egui::RichText::new(if is_done {
                                    format!("✓ {}", label)
                                } else {
                                    label.to_string()
                                })
                                .color(if is_active {
                                    ui.visuals().text_color()
                                } else {
                                    crate::theme::muted_text_color(ui.visuals())
                                });

                                let response = ui.selectable_label(is_active, text);
                                if response.clicked() && (is_done || is_active) {
                                    self.manager.step = step;
                                }
                            });
                            ui.add_space(4.0);
                        }

                        ui.add_space(20.0);
                        if ui.button("🗑 Reset Session").clicked() {
                            self.manager.reset();
                        }
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show_inside(ui, |ui| {
                // Add left padding to separate from sidebar
                ui.add_space(12.0);

                // Draw vertical separator manually since we removed the Panel's default border/frame
                let sep_stroke = ui.style().visuals.widgets.noninteractive.bg_stroke;
                let rect = ui.max_rect();
                ui.painter()
                    .vline(rect.min.x - 6.0, rect.y_range(), sep_stroke);

                action = match self.manager.step {
                    CalibrationFlowStep::Intro => render_intro(ui, ctx, &mut self.manager),
                    CalibrationFlowStep::Setup => render_setup(ui, ctx, &mut self.manager),
                    CalibrationFlowStep::Measure => render_measure(ui, ctx, &mut self.manager),
                    CalibrationFlowStep::Result => render_result(ui, ctx, &mut self.manager),
                };
            });

        action
    }
}

pub struct DisplayCalibrationContext<'a> {
    pub layout: &'a crate::theme::LayoutConfig,
    pub is_connected: bool,
    pub is_busy: bool,
}

// Private helpers
fn render_overlay(
    ui: &mut egui::Ui,
    _ctx: &DisplayCalibrationContext,
    manager: &DisplayCalibrationManager,
) {
    let overlay_color = if manager.is_measuring {
        match manager.current_target {
            CalibrationTarget::White => Some(egui::Color32::WHITE),
            CalibrationTarget::Black => Some(egui::Color32::BLACK),
            CalibrationTarget::Ramp => {
                // Get current ramp color from session
                if let Some(level) = manager.get_current_ramp_level() {
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
        let text_color = if color.r() > 100 {
            egui::Color32::BLACK
        } else {
            egui::Color32::WHITE
        };

        let status_text = match manager.current_target {
            CalibrationTarget::Ramp => {
                if let Some((idx, total)) = manager.get_progress() {
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
}

fn render_intro(
    ui: &mut egui::Ui,
    ctx: &DisplayCalibrationContext,
    manager: &mut DisplayCalibrationManager,
) -> CalibrationAction {
    ui.vertical(|ui| {
        ui.add_space(40.0);
        ui.vertical_centered(|ui| {
            ui.label(egui::RichText::new("🖥️").size(96.0));
            ui.add_space(20.0);
            ui.heading(egui::RichText::new("Display Calibration Wizard").size(32.0).strong());
            ui.add_space(20.0);

            ui.label(
                egui::RichText::new("This automated process measures your display's response and generates a high-precision 1D Video LUT for accurate color reproduction.")
                    .weak()
            );

            ui.add_space(40.0);
            egui::Frame::none()
                .fill(ui.visuals().window_fill)
                .rounding(8.0)
                .inner_margin(20.0)
                .show(ui, |ui| {
                    ui.set_width(400.0);
                    ui.label("Prerequisites:");
                    ui.label("• Connect your spectrometer");
                    ui.label("• Disable any OS color management");
                    ui.label("• Let the display warm up for 30 minutes");
                });

            ui.add_space(40.0);

            if !ctx.is_connected {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 80, 80),
                    "⚠️ Spectrometer not detected. Please connect hardware to continue.",
                );
            }

            let start_btn = ui.add_enabled(
                ctx.is_connected,
                egui::Button::new("Get Started →")
                    .min_size(egui::vec2(160.0, 50.0))
                    .rounding(25.0),
            );
            if start_btn.clicked() {
                manager.step = CalibrationFlowStep::Setup;
            }
        });
    });
    CalibrationAction::None
}

fn render_setup(
    ui: &mut egui::Ui,
    ctx: &DisplayCalibrationContext,
    manager: &mut DisplayCalibrationManager,
) -> CalibrationAction {
    let mut action = CalibrationAction::None;
    let visuals = ui.ctx().style().visuals.clone();

    // 1. Sticky Footer (Rendered first, creates space at bottom)
    egui::TopBottomPanel::bottom("setup_footer")
        .frame(egui::Frame::none())
        .resizable(false)
        .show_inside(ui, |ui| {
            ui.add_space(12.0);

            // Action Buttons Row
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let is_ready = ctx.is_connected && manager.readings.white_point.is_some();
                let start_btn = egui::Button::new("Start Characterization >>")
                    .min_size(egui::vec2(220.0, 40.0))
                    .fill(if is_ready {
                        crate::theme::success_color(&visuals)
                    } else {
                        visuals.widgets.inactive.bg_fill
                    });

                if ui.add_enabled(is_ready, start_btn).clicked() {
                    manager.start_session();
                }

                if ui.button("<< Back").clicked() {
                    manager.step = CalibrationFlowStep::Intro;
                }

                // Warning Text (Left of buttons / Flex space)
                if manager.readings.white_point.is_none() {
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.colored_label(
                            egui::Color32::KHAKI,
                            "💡 White point measurement is mandatory.",
                        );
                    });
                }
            });

            ui.add_space(12.0);
            // Top separator for the footer
            ui.separator();
        });

    // 2. Main Content (Fills remaining space automatically)
    egui::CentralPanel::default()
        .frame(egui::Frame::none())
        .show_inside(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.heading("Calibration Settings");
                        ui.label(egui::RichText::new("Configure targets and measure white/black points before starting characterization.").weak());
                        ui.add_space(20.0);

                        // --- Bento Grid Layout ---
                        ui.horizontal(|ui| {
                            // Card 1: Parameters
                            render_card(ui, "🎯 Target Parameters", 150.0, |ui| {
                                ui.vertical(|ui| {
                                    ui.label("Target Gamma:");
                                    ui.add(egui::Slider::new(&mut manager.config.target_gamma, 1.0..=3.0).step_by(0.1));
                                    ui.add_space(12.0);
                                    ui.label(format!("Sample Points: {}", manager.config.patch_count));
                                    ui.add(egui::Slider::new(&mut manager.config.patch_count, 2..=33));
                                    ui.add_space(8.0);
                                    ui.label(egui::RichText::new("Higher counts yield smoother results.").size(10.0).weak());
                                });
                            });

                            ui.add_space(ctx.layout.spacing);

                            // Card 2: White Reference
                            let has_white = manager.readings.white_point.is_some();
                            render_card(ui, "⚪ White Reference", 150.0, |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(10.0);
                                    if let Some(w) = manager.readings.white_point {
                                        ui.label(egui::RichText::new(format!("{:.1}", w.y)).size(32.0).strong());
                                        ui.label("cd/m²");
                                    } else {
                                        ui.label(egui::RichText::new("---").size(32.0).weak());
                                        ui.label("Not Measured");
                                    }
                                    ui.add_space(10.0);
                                    let btn = egui::Button::new(if has_white { "Re-measure" } else { "Measure White" })
                                        .fill(if has_white { visuals.widgets.inactive.bg_fill } else { crate::theme::success_color(&visuals).gamma_multiply(0.5) });
                                    if ui.add(btn).clicked() {
                                        manager.prepare_measurement(CalibrationTarget::White);
                                        action = CalibrationAction::RequestMeasurement;
                                    }
                                });
                            });

                            ui.add_space(ctx.layout.spacing);

                            // Card 3: Black Reference
                            let has_black = manager.readings.black_point.is_some();
                            render_card(ui, "⚫ Black Reference", 150.0, |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.add_space(10.0);
                                    if let Some(b) = manager.readings.black_point {
                                        ui.label(egui::RichText::new(format!("{:.4}", b.y)).size(32.0).strong());
                                        ui.label("cd/m²");
                                    } else {
                                        ui.label(egui::RichText::new("---").size(32.0).weak());
                                        ui.label("Not Measured");
                                    }
                                    ui.add_space(10.0);
                                    if ui.button(if has_black { "Re-measure" } else { "Measure Black" }).clicked() {
                                        manager.prepare_measurement(CalibrationTarget::Black);
                                        action = CalibrationAction::RequestMeasurement;
                                    }
                                });
                            });
                        });
                    });
                });
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

fn render_measure(
    ui: &mut egui::Ui,
    ctx: &DisplayCalibrationContext,
    manager: &mut DisplayCalibrationManager,
) -> CalibrationAction {
    // If no session, go back to setup
    if manager.session.is_none() {
        return CalibrationAction::None;
    }

    let level = manager.get_current_ramp_level().unwrap_or(0.0);
    let (current_idx, total) = manager.get_progress().unwrap_or((0, 0));
    let progress = (current_idx as f32) / (total as f32);
    let gray_val = (level * 255.0) as u8;
    let mut action = CalibrationAction::None;

    ui.vertical(|ui| {
        ui.heading("Characterizing Display...");
        ui.add(egui::ProgressBar::new(progress).text(format!("{} / {}", current_idx + 1, total)));
        ui.add_space(20.0);

        ui.horizontal(|ui| {
            // Left: Patch Preview
            ui.vertical(|ui| {
                ui.set_width(320.0);
                let size = egui::vec2(300.0, 300.0);
                let (rect, _response) = ui.allocate_at_least(size, egui::Sense::hover());
                ui.painter()
                    .rect_filled(rect, 12.0, egui::Color32::from_gray(gray_val));
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(format!(
                        "Target: RGB({}, {}, {})",
                        gray_val, gray_val, gray_val
                    ))
                    .weak(),
                );
            });

            ui.add_space(40.0);

            // Right: Real-time Stats
            ui.vertical(|ui| {
                render_card(ui, "📈 Metrics", 200.0, |ui| {
                    ui.label("Measured Luminance:");
                    if let Some(xyz) = manager.readings.last_measured {
                        ui.label(
                            egui::RichText::new(format!("{:.3} cd/m²", xyz.y))
                                .size(24.0)
                                .strong()
                                .color(egui::Color32::LIGHT_BLUE),
                        );
                    } else {
                        ui.label(egui::RichText::new("---").size(24.0).weak());
                    }

                    ui.add_space(20.0);
                    ui.label("Remaining time:");
                    let est = (total - current_idx) * 2; // Rough estimate: 2s per patch
                    ui.label(egui::RichText::new(format!("~{}s", est)).weak());
                });
            });
        });

        ui.add_space(40.0);

        if ctx.is_busy {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Hardware busy...");
            });
        } else {
            // Check if we need to auto-trigger
            // If manager thinks we are measuring (waiting for measurement) but HW is NOT busy,
            // we should request measurement.
            if manager.is_measuring {
                action = CalibrationAction::RequestMeasurement;
            }

            ui.horizontal(|ui| {
                let single_btn = ui.button("📸 Single Measure");
                if single_btn.clicked() {
                    manager.config.auto_advance = false;
                    manager.prepare_measurement(CalibrationTarget::Ramp);
                    action = CalibrationAction::RequestMeasurement;
                }

                let auto_btn = ui.add(
                    egui::Button::new("🚀 Start Auto-Advance")
                        .fill(crate::theme::success_color(&ui.visuals().clone())),
                );
                if auto_btn.clicked() {
                    manager.config.auto_advance = true;
                    // Trigger first one
                    manager.prepare_measurement(CalibrationTarget::Ramp);
                    action = CalibrationAction::RequestMeasurement;
                }

                if ui.button("💾 Simulate Step").clicked() {
                    manager.config.auto_advance = false;
                    manager.simulate_step();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("Abort").clicked() {
                        manager.reset();
                    }
                });
            });
        }
    });

    action
}

fn render_result(
    ui: &mut egui::Ui,
    _ctx: &DisplayCalibrationContext,
    manager: &mut DisplayCalibrationManager,
) -> CalibrationAction {
    ui.vertical(|ui| {
        ui.heading("Calibration Summary");
        ui.add_space(10.0);

        if let Some(cal) = &manager.result {
            ui.horizontal(|ui| {
                // Left: Summary Stats
                ui.vertical(|ui| {
                    ui.set_width(200.0);
                    render_card(ui, "📋 Results", 100.0, |ui| {
                        ui.label("Peak White:");
                        let w_y = manager.readings.white_point.map(|xyz| xyz.y).unwrap_or(0.0);
                        ui.label(egui::RichText::new(format!("{:.1} nits", w_y)).strong());
                        ui.add_space(8.0);
                        ui.label("Contrast Ratio:");
                        let contrast = if let (Some(w), Some(b)) =
                            (manager.readings.white_point, manager.readings.black_point)
                        {
                            format!("{}:1", (w.y / b.y.max(0.0001)) as i32)
                        } else {
                            "N/A".to_string()
                        };
                        ui.label(egui::RichText::new(contrast).strong());
                    });

                    ui.add_space(20.0);
                    if ui.button("📁 Export CGATS .cal").clicked() {
                        // TODO
                    }
                    if ui.button("🔒 Apply to GPU (STUB)").clicked() {
                        // TODO
                    }
                });

                ui.add_space(20.0);

                // Right: Plot
                ui.vertical(|ui| {
                    ui.label("Gamma Correction Curve");
                    let r_points: PlotPoints = cal
                        .r
                        .values
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| [i as f64 / 255.0, v as f64])
                        .collect();

                    Plot::new("cal_plot")
                        .view_aspect(1.5)
                        .height(350.0)
                        .allow_zoom(false)
                        .allow_drag(false)
                        .show(ui, |plot_ui| {
                            plot_ui.line(
                                Line::new(r_points)
                                    .color(egui::Color32::from_rgb(100, 200, 255))
                                    .width(2.0)
                                    .name("Correction"),
                            );
                            plot_ui.line(
                                Line::new(PlotPoints::from_iter(vec![[0.0, 0.0], [1.0, 1.0]]))
                                    .color(egui::Color32::DARK_GRAY)
                                    .style(egui_plot::LineStyle::Dashed { length: 4.0 }),
                            );
                        });
                });
            });
        }

        ui.add_space(40.0);
        ui.separator();
        if ui.button("Finish & Restart Wizard").clicked() {
            manager.reset();
        }
    });
    CalibrationAction::None
}
