//! Calibration wizard component for ColorMunki device.
//!
//! Refactored to separate State, View, and Logic.

use crossbeam_channel::Sender;
use eframe::egui;

use crate::shared::DeviceCommand;
use crate::t;
use crate::theme::{
    LayoutConfig, border_color, contrast_fill_color, error_color, muted_text_color, success_color,
    warning_color,
};

// ============================================================================
// 1. State Definitions
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CalibrationStep {
    #[default]
    RotateDial,
    PlaceOnTile,
    Calibrating,
    Complete,
    Failed, // Explicit failed state
}

#[derive(Default)]
pub struct CalibrationState {
    pub show: bool,
    pub step: CalibrationStep,
    pub error_msg: Option<String>,
}

// ============================================================================
// 2. The Wizard Component (Controller)
// ============================================================================

pub struct CalibrationWizard {
    pub state: CalibrationState,
}

impl Default for CalibrationWizard {
    fn default() -> Self {
        Self::new()
    }
}

impl CalibrationWizard {
    pub fn new() -> Self {
        Self {
            state: CalibrationState::default(),
        }
    }

    pub fn start(&mut self) {
        self.state.show = true;
        self.state.step = CalibrationStep::RotateDial;
        self.state.error_msg = None;
    }

    pub fn close(&mut self) {
        self.state.show = false;
        self.state.step = CalibrationStep::RotateDial;
    }

    pub fn on_calibration_success(&mut self) {
        self.state.step = CalibrationStep::Complete;
        self.state.error_msg = None;
    }

    pub fn on_calibration_error(&mut self, msg: String) {
        self.state.step = CalibrationStep::Failed;
        self.state.error_msg = Some(msg);
    }

    /// Primary render entry point
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        cmd_tx: &Sender<DeviceCommand>,
        is_busy: &mut bool,
        // We now handle errors via internal state, but keep this for backward compat bridging
        _status_msg: &str,
        layout: &LayoutConfig,
    ) {
        if !self.state.show {
            return;
        }

        let mut next_step = None;
        let mut close_requested = false;
        let mut retry_requested = false;

        egui::Window::new(t!("gui-cal-title"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([400.0, 500.0])
            .open(&mut self.state.show)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(layout.spacing);

                    // Render Progress Bar
                    render_progress_bar(ui, self.state.step);

                    ui.add_space(layout.spacing * 2.0);
                    ui.separator();
                    ui.add_space(layout.spacing * 2.0);

                    // Render Current Step Content
                    match self.state.step {
                        CalibrationStep::RotateDial => {
                            if let Some(action) = render_step_rotate_dial(ui, layout) {
                                match action {
                                    StepAction::Next => {
                                        next_step = Some(CalibrationStep::PlaceOnTile)
                                    }
                                    StepAction::QuickCalibrate => {
                                        *is_busy = true;
                                        cmd_tx.send(DeviceCommand::Calibrate).ok();
                                        next_step = Some(CalibrationStep::Calibrating);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        CalibrationStep::PlaceOnTile => {
                            if let Some(action) = render_step_place_on_tile(ui, layout) {
                                match action {
                                    StepAction::Back => {
                                        next_step = Some(CalibrationStep::RotateDial)
                                    }
                                    StepAction::Calibrate => {
                                        *is_busy = true;
                                        cmd_tx.send(DeviceCommand::Calibrate).ok();
                                        next_step = Some(CalibrationStep::Calibrating);
                                    }
                                    _ => {}
                                }
                            }
                        }
                        CalibrationStep::Calibrating => {
                            render_step_calibrating(ui, layout);
                        }
                        CalibrationStep::Complete => {
                            if let Some(StepAction::Finish) = render_step_complete(ui, layout) {
                                close_requested = true;
                            }
                        }
                        CalibrationStep::Failed => {
                            let msg = self.state.error_msg.as_deref().unwrap_or("Unknown Error");
                            if let Some(action) = render_step_failed(ui, msg, layout) {
                                match action {
                                    StepAction::Retry => retry_requested = true,
                                    StepAction::Cancel => close_requested = true,
                                    _ => {}
                                }
                            }
                        }
                    }
                });
            });

        // Handle State Transitions
        if let Some(step) = next_step {
            self.state.step = step;
        }
        if retry_requested {
            *is_busy = true;
            cmd_tx.send(DeviceCommand::Calibrate).ok();
            self.state.step = CalibrationStep::Calibrating;
            self.state.error_msg = None;
        }
        if close_requested {
            self.close();
        }
    }
}

// ============================================================================
// 3. View Components (Pure Rendering)
// ============================================================================

enum StepAction {
    Next,
    Back,
    Calibrate,
    QuickCalibrate,
    Retry,
    Cancel,
    Finish,
}

fn render_progress_bar(ui: &mut egui::Ui, current_step: CalibrationStep) {
    ui.horizontal(|ui| {
        let steps = [
            (CalibrationStep::RotateDial, "Dial"),
            (CalibrationStep::PlaceOnTile, "Position"),
            (CalibrationStep::Calibrating, "Calibrate"),
            (CalibrationStep::Complete, "Done"),
        ];

        // Determine progress index
        let active_idx = match current_step {
            CalibrationStep::RotateDial => 0,
            CalibrationStep::PlaceOnTile => 1,
            CalibrationStep::Calibrating | CalibrationStep::Failed => 2,
            CalibrationStep::Complete => 3,
        };

        for (i, (_, label)) in steps.iter().enumerate() {
            let is_active = i == active_idx;
            let is_done = i < active_idx;
            let is_error = current_step == CalibrationStep::Failed && i == 2;

            let color = if is_error {
                error_color(&ui.ctx().style().visuals)
            } else if is_active {
                contrast_fill_color(&ui.ctx().style().visuals)
            } else if is_done {
                success_color(&ui.ctx().style().visuals)
            } else {
                muted_text_color(&ui.ctx().style().visuals)
            };

            let icon = if is_error {
                "❌"
            } else if is_done {
                "✓"
            } else {
                ""
            };
            let text = format!("{}{}. {}", icon, i + 1, label);

            ui.label(egui::RichText::new(text).color(color).strong());

            if i < steps.len() - 1 {
                ui.label(
                    egui::RichText::new("→").color(muted_text_color(&ui.ctx().style().visuals)),
                );
            }
        }
    });
}

fn render_step_rotate_dial(ui: &mut egui::Ui, layout: &LayoutConfig) -> Option<StepAction> {
    ui.label(
        egui::RichText::new("Step 1: Rotate the Dial")
            .size(20.0)
            .strong(),
    );
    ui.add_space(layout.spacing * 2.0);

    render_device_dial(ui, "CALIBRATE", 180.0);

    ui.add_space(layout.spacing * 2.0);
    ui.label("Rotate dial to the");
    ui.label(
        egui::RichText::new("CALIBRATION POSITION")
            .color(warning_color(&ui.ctx().style().visuals))
            .strong(),
    );
    ui.label("(Look for the small PILL/RECTANGLE icon)");
    ui.add_space(layout.spacing * 2.0);

    let mut action = None;
    ui.horizontal(|ui| {
        if ui
            .button(egui::RichText::new("Next Step →").size(16.0))
            .clicked()
        {
            action = Some(StepAction::Next);
        }
        ui.add_space(layout.spacing);
        if ui
            .button(egui::RichText::new(t!("gui-quick-calibrate")).color(egui::Color32::LIGHT_BLUE))
            .clicked()
        {
            action = Some(StepAction::QuickCalibrate);
        }
    });

    ui.add_space(8.0);
    ui.label(
        egui::RichText::new("Use 'Quick Calibrate' if device is already positioned.")
            .small()
            .italics()
            .color(ui.visuals().weak_text_color()),
    );

    action
}

fn render_step_place_on_tile(ui: &mut egui::Ui, layout: &LayoutConfig) -> Option<StepAction> {
    ui.label(
        egui::RichText::new("Step 2: Position the Device")
            .size(20.0)
            .strong(),
    );
    ui.add_space(layout.spacing * 2.0);

    // Graphic: Tile + Device
    let (rect, _) = ui.allocate_exact_size(egui::vec2(150.0, 100.0), egui::Sense::hover());
    let painter = ui.painter();
    let tile_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(120.0, 80.0));
    painter.rect_filled(tile_rect, 8.0, egui::Color32::WHITE);
    painter.rect_stroke(
        tile_rect,
        8.0,
        egui::Stroke::new(2.0, muted_text_color(&ui.ctx().style().visuals)),
    );

    let device_pos = tile_rect.center() - egui::vec2(0.0, 10.0);
    painter.circle_filled(device_pos, 25.0, border_color(&ui.ctx().style().visuals));
    painter.circle_stroke(
        device_pos,
        25.0,
        egui::Stroke::new(2.0, muted_text_color(&ui.ctx().style().visuals)),
    );

    ui.add_space(layout.spacing * 2.0);
    ui.label("Place the ColorMunki on the");
    ui.label(
        egui::RichText::new("WHITE CALIBRATION TILE")
            .color(contrast_fill_color(&ui.ctx().style().visuals))
            .strong(),
    );
    ui.add_space(layout.spacing * 2.0);

    let mut action = None;
    ui.horizontal(|ui| {
        if ui.button("← Back").clicked() {
            action = Some(StepAction::Back);
        }
        if ui
            .button(egui::RichText::new("Start Calibration").size(16.0).strong())
            .clicked()
        {
            action = Some(StepAction::Calibrate);
        }
    });
    action
}

fn render_step_calibrating(ui: &mut egui::Ui, layout: &LayoutConfig) {
    ui.label(
        egui::RichText::new("Step 3: Calibrating...")
            .size(20.0)
            .strong(),
    );
    ui.add_space(layout.spacing * 3.0);
    ui.spinner();
    ui.add_space(layout.spacing * 2.0);
    ui.label("Please wait while the device calibrates...");
    ui.add_space(layout.spacing);
    ui.label(
        egui::RichText::new("Do not move the device")
            .italics()
            .color(warning_color(&ui.ctx().style().visuals)),
    );
}

fn render_step_failed(
    ui: &mut egui::Ui,
    error_msg: &str,
    layout: &LayoutConfig,
) -> Option<StepAction> {
    ui.colored_label(
        error_color(&ui.ctx().style().visuals),
        egui::RichText::new(t!("gui-cal-failed"))
            .size(20.0)
            .strong(),
    );
    ui.add_space(layout.spacing);

    // Error details box
    egui::Frame::none()
        .fill(ui.visuals().faint_bg_color)
        .stroke(egui::Stroke::new(
            1.0,
            error_color(&ui.ctx().style().visuals),
        ))
        .inner_margin(8.0)
        .rounding(4.0)
        .show(ui, |ui| {
            ui.label(egui::RichText::new(error_msg).monospace().size(12.0));
        });

    ui.add_space(layout.spacing * 2.0);

    let mut action = None;
    ui.horizontal(|ui| {
        if ui.button("🔄 Retry").clicked() {
            action = Some(StepAction::Retry);
        }
        if ui.button("Cancel").clicked() {
            action = Some(StepAction::Cancel);
        }
    });
    action
}

fn render_step_complete(ui: &mut egui::Ui, layout: &LayoutConfig) -> Option<StepAction> {
    ui.label(
        egui::RichText::new("✅ Calibration Complete!")
            .size(24.0)
            .strong()
            .color(success_color(&ui.ctx().style().visuals)),
    );
    ui.add_space(layout.spacing * 3.0);
    ui.label("Your ColorMunki is now calibrated and ready for measurements.");
    ui.add_space(layout.spacing * 3.0);

    if ui
        .button(egui::RichText::new("Finish").size(16.0))
        .clicked()
    {
        return Some(StepAction::Finish);
    }
    None
}

// Re-use existing dial renderer (kept logic same, just pure function now)
pub fn render_device_dial(ui: &mut egui::Ui, highlight_position: &str, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let painter = ui.painter();
    let center = rect.center();
    let outer_radius = size / 2.0 - 10.0;

    // Draw outer ring
    painter.circle_stroke(
        center,
        outer_radius,
        egui::Stroke::new(3.0, egui::Color32::from_gray(140)),
    );

    // Highlight positions
    let positions = [
        (
            "REFLECTIVE",
            std::f32::consts::FRAC_PI_2,
            egui::Color32::from_rgb(100, 180, 255),
            false,
        ),
        (
            "CALIBRATE",
            std::f32::consts::FRAC_PI_4,
            egui::Color32::YELLOW,
            true,
        ),
        (
            "PROJECTOR",
            0.0,
            egui::Color32::from_rgb(255, 120, 120),
            false,
        ),
        (
            "AMBIENT",
            -std::f32::consts::FRAC_PI_2,
            egui::Color32::from_rgb(150, 255, 150),
            false,
        ),
    ];

    for (name, angle, color, is_capsule) in &positions {
        let is_highlighted = highlight_position.eq_ignore_ascii_case(name);
        let marker_pos =
            center + egui::vec2(angle.cos() * outer_radius, angle.sin() * outer_radius);
        let base_color = if is_highlighted {
            *color
        } else {
            egui::Color32::from_gray(100)
        };

        if *is_capsule {
            painter.rect_filled(
                egui::Rect::from_center_size(marker_pos, egui::vec2(16.0, 8.0)),
                4.0,
                base_color,
            );
        } else {
            painter.circle_filled(marker_pos, 5.0, base_color);
        }

        if is_highlighted {
            painter.line_segment([center, marker_pos], egui::Stroke::new(3.0, *color));
        }
    }

    painter.circle_filled(center, 4.0, contrast_fill_color(&ui.ctx().style().visuals));
}

/// Render a small dial check reminder
pub fn render_dial_check(ctx: &egui::Context, mode_name: &str, layout: &LayoutConfig) {
    egui::Window::new("⚙️ Dial Check")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::RIGHT_BOTTOM, [-20.0, -80.0])
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("Check Dial Position").strong());
                render_device_dial(ui, mode_name, 100.0);
                ui.add_space(layout.spacing * 0.5);
                ui.label(
                    egui::RichText::new(format!("Set dial to: {}", mode_name.to_uppercase()))
                        .small(),
                );
            });
        });
}
