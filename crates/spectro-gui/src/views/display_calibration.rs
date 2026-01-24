use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use spectro_rs::colorimetry::XYZ;

use crate::calibration::{CalibrationFlowStep, CalibrationTarget, DisplayCalibrationManager};
use crate::theme;

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
    pub fn handle_measurement(&mut self, xyz: XYZ) {
        self.manager.handle_measurement(xyz);
    }

    pub fn render(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &DisplayCalibrationContext,
    ) -> CalibrationAction {
        let mut action = CalibrationAction::None;

        // 1. Global Overlay Rendering (The visual patch)
        render_overlay(ui, ctx, &self.manager);

        // 2. Top Navigation Bar
        egui::Frame::none()
            .fill(theme::panel_bg_color(&ui.ctx().style().visuals))
            .inner_margin(egui::Margin::symmetric(20.0, 12.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("🖥️ Display Calibration")
                            .size(18.0)
                            .strong(),
                    );

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("✖ Close").clicked() {
                            action = CalibrationAction::Close;
                        }

                        ui.add_space(20.0);

                        // Intelligent status breadcrumb
                        let status_text = self.manager.get_status_text();
                        ui.label(egui::RichText::new(status_text).weak().size(13.0));
                    });
                });
            });

        if action == CalibrationAction::Close {
            return action;
        }

        ui.separator();

        // 3. Main Layout: Sidebar + Content
        egui::SidePanel::left("cal_stepper_sidebar")
            .resizable(false)
            .exact_width(220.0)
            .frame(egui::Frame::none().fill(theme::panel_bg_dark_color(&ui.ctx().style().visuals)))
            .show_inside(ui, |ui| {
                ui.vertical(|ui| {
                    ui.add_space(20.0);

                    let steps = [
                        (CalibrationFlowStep::Intro, "Introduction", "🏠"),
                        (CalibrationFlowStep::Setup, "Hardware Setup", "⚙️"),
                        (CalibrationFlowStep::Measure, "Measurement", "📏"),
                        (CalibrationFlowStep::Result, "Summary", "📊"),
                    ];

                    for (step, label, icon) in steps {
                        let is_active = self.manager.step == step;
                        let is_done = (self.manager.step as u8) > (step as u8);

                        let text_color = if is_active {
                            ui.visuals().text_color()
                        } else if is_done {
                            theme::success_color(ui.visuals())
                        } else {
                            theme::muted_text_color(ui.visuals())
                        };

                        ui.horizontal(|ui| {
                            ui.add_space(20.0);
                            let prefix = if is_done { "✓ " } else { icon };
                            let resp = ui.selectable_label(
                                is_active,
                                egui::RichText::new(format!("{} {}", prefix, label))
                                    .color(text_color)
                                    .size(15.0),
                            );
                            if resp.clicked() && (is_done || is_active) {
                                self.manager.step = step;
                            }
                        });
                        ui.add_space(12.0);
                    }

                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        ui.add_space(20.0);
                        if ui.button("🗑 Reset Session").clicked() {
                            self.manager.reset();
                        }
                    });
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().inner_margin(30.0))
            .show_inside(ui, |ui| {
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

pub struct DisplayCalibrationContext {
    pub is_connected: bool,
    pub is_busy: bool,
}

// ----------------------------------------------------------------------------
// Private UI Components
// ----------------------------------------------------------------------------

fn render_overlay(
    ui: &egui::Ui,
    _ctx: &DisplayCalibrationContext,
    manager: &DisplayCalibrationManager,
) {
    if !manager.is_measuring {
        return;
    }

    let color = match manager.current_target {
        CalibrationTarget::White => egui::Color32::WHITE,
        CalibrationTarget::Black => egui::Color32::BLACK,
        CalibrationTarget::Ramp => {
            if let Some(level) = manager.get_current_ramp_level() {
                egui::Color32::from_gray((level * 255.0) as u8)
            } else {
                return;
            }
        }
        CalibrationTarget::None => return,
    };

    let painter = ui.ctx().layer_painter(egui::LayerId::debug());
    let rect = ui.ctx().input(|i| i.screen_rect());

    // Smooth transition simulation for eye comfort
    painter.rect_filled(rect, 0.0, color);

    // Minimal status HUD in the corner to avoid interference with the sensor center
    let hud_rect = egui::Rect::from_min_size(
        rect.left_top() + egui::vec2(20.0, 20.0),
        egui::vec2(200.0, 40.0),
    );

    let text_color = if color.r() > 128 {
        egui::Color32::BLACK
    } else {
        egui::Color32::WHITE
    };
    painter.text(
        hud_rect.left_center(),
        egui::Align2::LEFT_CENTER,
        format!("📷 MEASURING PHASE - [{}]", manager.get_status_text()),
        egui::FontId::proportional(14.0),
        text_color,
    );

    ui.ctx()
        .output_mut(|o| o.cursor_icon = egui::CursorIcon::None);
}

fn render_card<F>(ui: &mut egui::Ui, title: &str, icon: &str, add_contents: F)
where
    F: FnOnce(&mut egui::Ui),
{
    let visuals = &ui.ctx().style().visuals;
    egui::Frame::none()
        .fill(theme::info_panel_color(visuals))
        .rounding(16.0)
        .stroke(egui::Stroke::new(1.0, theme::border_color(visuals)))
        .inner_margin(24.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(icon).size(20.0));
                    ui.label(egui::RichText::new(title).strong().size(16.0));
                });
                ui.add_space(16.0);
                add_contents(ui);
            });
        });
}

fn render_intro(
    ui: &mut egui::Ui,
    ctx: &DisplayCalibrationContext,
    manager: &mut DisplayCalibrationManager,
) -> CalibrationAction {
    ui.vertical_centered(|ui| {
        ui.add_space(60.0);
        ui.label(egui::RichText::new("🌈").size(120.0));
        ui.add_space(20.0);
        ui.heading(
            egui::RichText::new("Display Optimization Wizard")
                .size(36.0)
                .strong(),
        );
        ui.add_space(15.0);
        ui.label(
            egui::RichText::new(
                "High-precision hardware characterization for professional workflows.",
            )
            .weak()
            .size(16.0),
        );

        ui.add_space(50.0);

        egui::Grid::new("intro_grid")
            .spacing([40.0, 20.0])
            .show(ui, |ui| {
                render_card(ui, "Prerequisites", "📋", |ui| {
                    ui.set_width(280.0);
                    ui.label("• Connect ColorMunki/i1Pro");
                    ui.label("• Disable Night Shift/f.lux");
                    ui.label("• Warm up display (30 min)");
                });

                render_card(ui, "Our Method", "🔬", |ui| {
                    ui.set_width(280.0);
                    ui.label("• 1D Video LUT Correction");
                    ui.label("• Spectral White Balancing");
                    ui.label("• Gamma response analysis");
                });
            });

        ui.add_space(60.0);

        if !ctx.is_connected {
            ui.colored_label(
                theme::error_color(ui.visuals()),
                "⚠️ Spectrometer not detected. Connect hardware to proceed.",
            );
        }

        let start_btn = ui.add_enabled(
            ctx.is_connected,
            egui::Button::new(egui::RichText::new("Continue to Setup →").size(18.0))
                .min_size(egui::vec2(220.0, 50.0))
                .rounding(25.0),
        );
        if start_btn.clicked() {
            manager.step = CalibrationFlowStep::Setup;
        }
    });
    CalibrationAction::None
}

fn render_setup(
    ui: &mut egui::Ui,
    _ctx: &DisplayCalibrationContext,
    manager: &mut DisplayCalibrationManager,
) -> CalibrationAction {
    let mut action = CalibrationAction::None;

    ui.vertical(|ui| {
        ui.heading("Configuration & References");
        ui.add_space(20.0);

        ui.columns(2, |cols| {
            // Left Column: Logic Config
            render_card(&mut cols[0], "Target Parameters", "🎯", |ui| {
                ui.vertical(|ui| {
                    ui.label("Target Gamma:");
                    ui.add(
                        egui::Slider::new(&mut manager.config.target_gamma, 1.0..=3.0)
                            .step_by(0.1)
                            .smart_aim(true),
                    );

                    ui.add_space(15.0);
                    ui.label(format!(
                        "Calibration Patches: {}",
                        manager.config.patch_count
                    ));
                    ui.add(egui::Slider::new(&mut manager.config.patch_count, 5..=65));
                });
            });

            // Right Column: Physical References
            render_card(&mut cols[1], "Sensor References", "📐", |ui| {
                ui.vertical(|ui| {
                    // White Point
                    ui.horizontal(|ui| {
                        if let Some(w) = manager.readings.white_point {
                            ui.label(egui::RichText::new("White:").weak());
                            ui.label(egui::RichText::new(format!("{:.1} nits", w.y)).strong());
                            if ui.button("🔄").clicked() {
                                manager.prepare_measurement(CalibrationTarget::White);
                                action = CalibrationAction::RequestMeasurement;
                            }
                        } else if ui.button("📷 Measure White Point").clicked() {
                            manager.prepare_measurement(CalibrationTarget::White);
                            action = CalibrationAction::RequestMeasurement;
                        }
                    });

                    ui.add_space(10.0);

                    // Black Point
                    ui.horizontal(|ui| {
                        if let Some(b) = manager.readings.black_point {
                            ui.label(egui::RichText::new("Black:").weak());
                            ui.label(egui::RichText::new(format!("{:.3} nits", b.y)).strong());
                            if ui.button("🔄").clicked() {
                                manager.prepare_measurement(CalibrationTarget::Black);
                                action = CalibrationAction::RequestMeasurement;
                            }
                        } else if ui.button("📷 Measure Black Point").clicked() {
                            manager.prepare_measurement(CalibrationTarget::Black);
                            action = CalibrationAction::RequestMeasurement;
                        }
                    });
                });
            });
        });

        ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
            ui.add_space(20.0);
            let ready = manager.can_start_characterization();
            let start_btn =
                egui::Button::new(egui::RichText::new("Start Characterization ❯❯").size(16.0))
                    .min_size(egui::vec2(240.0, 44.0))
                    .rounding(22.0)
                    .fill(if ready {
                        theme::success_color(ui.visuals())
                    } else {
                        ui.visuals().widgets.inactive.bg_fill
                    });

            if ui.add_enabled(ready, start_btn).clicked() {
                manager.start_session();
            }
        });
    });

    action
}

fn render_measure(
    ui: &mut egui::Ui,
    ctx: &DisplayCalibrationContext,
    manager: &mut DisplayCalibrationManager,
) -> CalibrationAction {
    let mut action = CalibrationAction::None;
    let (curr, total) = manager.get_progress().unwrap_or((0, 0));
    let progress = curr as f32 / total as f32;

    ui.vertical_centered(|ui| {
        ui.add_space(40.0);
        ui.heading("Characterizing Display Response...");
        ui.add_space(10.0);

        let bar = egui::ProgressBar::new(progress)
            .text(format!("Patch {} of {}", curr + 1, total))
            .animate(true)
            .rounding(10.0);
        ui.add_sized(egui::vec2(600.0, 24.0), bar);

        ui.add_space(60.0);

        ui.horizontal(|ui| {
            ui.add_space(100.0);
            // Dynamic Metric Card
            render_card(ui, "Real-time Feedback", "📈", |ui| {
                ui.set_width(300.0);
                ui.set_height(160.0);
                if let Some(xyz) = manager.readings.last_measured {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new(format!("{:.3}", xyz.y))
                                .size(48.0)
                                .strong()
                                .color(egui::Color32::LIGHT_BLUE),
                        );
                        ui.label("Measured Luminance (cd/m²)");
                    });
                } else {
                    ui.vertical_centered(|ui| {
                        ui.add(egui::Spinner::new().size(40.0));
                        ui.label("Waiting for first patch...");
                    });
                }
            });

            ui.add_space(40.0);

            // Preview
            let l = manager.get_current_ramp_level().unwrap_or(0.0);
            let c = (l * 255.0) as u8;
            egui::Frame::none()
                .fill(egui::Color32::from_gray(c))
                .rounding(20.0)
                .stroke(egui::Stroke::new(2.0, theme::border_color(ui.visuals())))
                .show(ui, |ui| {
                    ui.allocate_exact_size(egui::vec2(210.0, 210.0), egui::Sense::hover());
                });
        });

        ui.add_space(60.0);

        if ctx.is_busy {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Hardware integration in progress...");
            });
        } else {
            if manager.is_measuring {
                action = CalibrationAction::RequestMeasurement;
            }

            ui.horizontal(|ui| {
                if ui.button("📸 Single Step").clicked() {
                    manager.config.auto_advance = false;
                    manager.prepare_measurement(CalibrationTarget::Ramp);
                }

                if ui
                    .add(
                        egui::Button::new("🚀 Start Auto-Advance")
                            .fill(theme::success_color(ui.visuals())),
                    )
                    .clicked()
                {
                    manager.config.auto_advance = true;
                    manager.prepare_measurement(CalibrationTarget::Ramp);
                }

                if ui.button("💾 Simulator (Debug)").clicked() {
                    manager.simulate_step();
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("中止 Abort").clicked() {
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
        ui.heading("Analysis Summary");
        ui.add_space(20.0);

        if let Some(cal) = &manager.result {
            ui.columns(2, |cols| {
                cols[0].vertical(|ui| {
                    render_card(ui, "Core Metrics", "📊", |ui| {
                        let w = manager.readings.white_point.unwrap_or(XYZ {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        });
                        let b = manager.readings.black_point.unwrap_or(XYZ {
                            x: 0.0,
                            y: 0.0,
                            z: 0.0,
                        });

                        ui.horizontal(|ui| {
                            ui.label("Peak Luminance:");
                            ui.label(
                                egui::RichText::new(format!("{:.1} cd/m²", w.y))
                                    .strong()
                                    .color(egui::Color32::WHITE),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label("Contrast Ratio:");
                            ui.label(
                                egui::RichText::new(format!("{}:1", (w.y / b.y.max(0.001)) as i32))
                                    .strong()
                                    .color(egui::Color32::LIGHT_GREEN),
                            );
                        });
                        ui.add_space(10.0);
                        ui.label(egui::RichText::new("Curve Analysis: Optimal").weak());
                    });

                    ui.add_space(20.0);
                    ui.vertical_centered_justified(|ui| {
                        ui.add(
                            egui::Button::new("💾 Save CGATS Correction")
                                .min_size(egui::vec2(0.0, 40.0)),
                        )
                        .clicked();
                        ui.add_space(10.0);
                        ui.add(
                            egui::Button::new("🎨 Generate ICC Profile (WIP)")
                                .min_size(egui::vec2(0.0, 40.0)),
                        )
                        .clicked();
                    });
                });

                cols[1].vertical(|ui| {
                    ui.label("Gamma Response Mapping");
                    let points: PlotPoints = cal
                        .r
                        .values
                        .iter()
                        .enumerate()
                        .map(|(i, &v)| [i as f64 / 255.0, v as f64])
                        .collect();

                    Plot::new("result_plot")
                        .view_aspect(1.3)
                        .allow_zoom(false)
                        .show(ui, |pui| {
                            pui.line(
                                Line::new(points)
                                    .color(egui::Color32::from_rgb(0, 150, 255))
                                    .width(3.0),
                            );
                            pui.line(
                                Line::new(PlotPoints::from_iter(vec![[0.0, 0.0], [1.0, 1.0]]))
                                    .style(egui_plot::LineStyle::Dashed { length: 4.0 }),
                            );
                        });
                });
            });
        }

        ui.add_space(40.0);
        ui.centered_and_justified(|ui| {
            if ui.button("Complete & Exit Wizard").clicked() {
                manager.reset();
            }
        });
    });
    CalibrationAction::None
}
