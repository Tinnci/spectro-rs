//! Calibration wizard component for ColorMunki device.
//!
//! This module provides a step-by-step graphical wizard to guide users
//! through the calibration process, including dial positioning guidance.

use crossbeam_channel::Sender;
use eframe::egui;

use crate::shared::DeviceCommand;

// ============================================================================
// Calibration State
// ============================================================================

/// Calibration wizard steps
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CalibrationStep {
    /// Step 1: Rotate the dial to calibration position
    #[default]
    RotateDial,
    /// Step 2: Place device on white tile
    PlaceOnTile,
    /// Step 3: Performing calibration
    Calibrating,
    /// Step 4: Calibration complete
    Complete,
}

/// Calibration wizard state and rendering logic.
pub struct CalibrationWizard {
    /// Whether the wizard modal is visible
    pub show: bool,
    /// Current step in the calibration process
    pub step: CalibrationStep,
}

impl Default for CalibrationWizard {
    fn default() -> Self {
        Self::new()
    }
}

impl CalibrationWizard {
    pub fn new() -> Self {
        Self {
            show: false,
            step: CalibrationStep::RotateDial,
        }
    }

    /// Start the calibration wizard from the beginning.
    pub fn start(&mut self) {
        self.show = true;
        self.step = CalibrationStep::RotateDial;
    }

    /// Close the wizard and reset state.
    pub fn close(&mut self) {
        self.show = false;
        self.step = CalibrationStep::RotateDial;
    }

    /// Called when calibration completes successfully.
    pub fn on_calibration_success(&mut self) {
        self.step = CalibrationStep::Complete;
    }

    /// Render the step-by-step calibration wizard.
    ///
    /// # Arguments
    /// * `ctx` - The egui context
    /// * `cmd_tx` - Channel to send device commands
    /// * `is_busy` - Mutable reference to busy state flag
    /// * `status_msg` - Current status message (for error display)
    pub fn render(
        &mut self,
        ctx: &egui::Context,
        cmd_tx: &Sender<DeviceCommand>,
        is_busy: &mut bool,
        status_msg: &str,
    ) {
        if !self.show {
            return;
        }

        egui::Window::new("🎯 Instrument Calibration")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([400.0, 480.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);

                    // Step Indicator
                    self.render_step_indicator(ui);

                    ui.add_space(20.0);
                    ui.separator();
                    ui.add_space(20.0);

                    match self.step {
                        CalibrationStep::RotateDial => {
                            self.render_step_rotate_dial(ui, cmd_tx, is_busy);
                        }
                        CalibrationStep::PlaceOnTile => {
                            self.render_step_place_on_tile(ui, cmd_tx, is_busy);
                        }
                        CalibrationStep::Calibrating => {
                            self.render_step_calibrating(ui, cmd_tx, is_busy, status_msg);
                        }
                        CalibrationStep::Complete => {
                            self.render_step_complete(ui);
                        }
                    }

                    ui.add_space(20.0);

                    // Cancel button (available in all steps except Complete)
                    if self.step != CalibrationStep::Complete
                        && self.step != CalibrationStep::Calibrating
                        && ui.button("Cancel").clicked()
                    {
                        self.close();
                    }
                });
            });
    }

    fn render_step_indicator(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let steps = ["Dial", "Position", "Calibrate", "Done"];
            let current_idx = self.step as usize;

            for (i, step) in steps.iter().enumerate() {
                let is_active = i == current_idx;
                let is_done = i < current_idx;

                let color = if is_active {
                    egui::Color32::WHITE
                } else if is_done {
                    egui::Color32::from_rgb(50, 205, 50)
                } else {
                    egui::Color32::GRAY
                };

                let text = if is_done {
                    format!("✓ {}", step)
                } else {
                    format!("{}. {}", i + 1, step)
                };

                ui.label(egui::RichText::new(text).color(color).strong());
                if i < steps.len() - 1 {
                    ui.label(egui::RichText::new(" → ").color(egui::Color32::DARK_GRAY));
                }
            }
        });
    }

    fn render_step_rotate_dial(
        &mut self,
        ui: &mut egui::Ui,
        cmd_tx: &Sender<DeviceCommand>,
        is_busy: &mut bool,
    ) {
        ui.label(
            egui::RichText::new("Step 1: Rotate the Dial")
                .size(20.0)
                .strong(),
        );
        ui.add_space(20.0);

        Self::render_device_dial(ui, "CALIBRATE", 180.0);

        ui.add_space(20.0);
        ui.label("Rotate dial to the");
        ui.label(
            egui::RichText::new("CALIBRATION POSITION")
                .color(egui::Color32::YELLOW)
                .strong(),
        );
        ui.label("(Look for the small PILL/RECTANGLE icon)");
        ui.add_space(20.0);

        // Navigation Buttons
        ui.horizontal(|ui| {
            if ui
                .button(egui::RichText::new("Next Step →").size(16.0))
                .clicked()
            {
                self.step = CalibrationStep::PlaceOnTile;
            }

            ui.add_space(10.0);

            // Quick/Force Calibrate
            if ui
                .button(egui::RichText::new("⚡ Quick Calibrate").color(egui::Color32::LIGHT_BLUE))
                .clicked()
            {
                *is_busy = true;
                cmd_tx.send(DeviceCommand::Calibrate).ok();
                self.step = CalibrationStep::Calibrating;
            }
        });
        ui.label(
            egui::RichText::new("Use 'Quick Calibrate' if device is already positioned.")
                .small()
                .italics(),
        );
    }

    fn render_step_place_on_tile(
        &mut self,
        ui: &mut egui::Ui,
        cmd_tx: &Sender<DeviceCommand>,
        is_busy: &mut bool,
    ) {
        ui.label(
            egui::RichText::new("Step 2: Position the Device")
                .size(20.0)
                .strong(),
        );
        ui.add_space(20.0);

        // Simple graphic representation
        let (rect, _) = ui.allocate_exact_size(egui::vec2(150.0, 100.0), egui::Sense::hover());
        let painter = ui.painter();

        // Draw white tile representation
        let tile_rect = egui::Rect::from_center_size(rect.center(), egui::vec2(120.0, 80.0));
        painter.rect_filled(tile_rect, 8.0, egui::Color32::WHITE);
        painter.rect_stroke(tile_rect, 8.0, egui::Stroke::new(2.0, egui::Color32::GRAY));

        // Draw device representation (circle on top)
        let device_pos = tile_rect.center() - egui::vec2(0.0, 10.0);
        painter.circle_filled(device_pos, 25.0, egui::Color32::from_rgb(60, 60, 80));
        painter.circle_stroke(
            device_pos,
            25.0,
            egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 100, 120)),
        );

        ui.add_space(20.0);
        ui.label("Place the ColorMunki on the");
        ui.label(
            egui::RichText::new("WHITE CALIBRATION TILE")
                .color(egui::Color32::WHITE)
                .strong(),
        );
        ui.label("(Included with your device)");
        ui.add_space(20.0);

        ui.horizontal(|ui| {
            if ui.button("← Back").clicked() {
                self.step = CalibrationStep::RotateDial;
            }

            if ui
                .button(egui::RichText::new("Start Calibration").size(16.0).strong())
                .clicked()
            {
                *is_busy = true;
                cmd_tx.send(DeviceCommand::Calibrate).ok();
                self.step = CalibrationStep::Calibrating;
            }
        });
    }

    fn render_step_calibrating(
        &mut self,
        ui: &mut egui::Ui,
        cmd_tx: &Sender<DeviceCommand>,
        is_busy: &mut bool,
        status_msg: &str,
    ) {
        ui.label(
            egui::RichText::new("Step 3: Calibrating...")
                .size(20.0)
                .strong(),
        );
        ui.add_space(30.0);

        // Check for error state
        if status_msg.contains("❌") || status_msg.contains("failed") {
            ui.colored_label(egui::Color32::RED, "⚠️ Calibration Failed");
            ui.add_space(10.0);
            ui.label(status_msg);
            ui.add_space(20.0);

            if ui.button("🔄 Retry").clicked() {
                *is_busy = true;
                cmd_tx.send(DeviceCommand::Calibrate).ok();
            }
            if ui.button("Cancel").clicked() {
                self.close();
            }
        } else {
            ui.spinner();
            ui.add_space(20.0);
            ui.label("Please wait while the device calibrates...");
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("Do not move the device")
                    .italics()
                    .color(egui::Color32::YELLOW),
            );
        }
    }

    fn render_step_complete(&mut self, ui: &mut egui::Ui) {
        ui.label(
            egui::RichText::new("✅ Calibration Complete!")
                .size(24.0)
                .strong()
                .color(egui::Color32::from_rgb(50, 205, 50)),
        );
        ui.add_space(30.0);

        ui.label("Your ColorMunki is now calibrated and ready for measurements.");
        ui.add_space(10.0);
        ui.label("You can now rotate the dial back to your desired measurement mode.");

        ui.add_space(30.0);

        if ui
            .button(egui::RichText::new("Finish").size(16.0))
            .clicked()
        {
            self.close();
        }
    }

    /// Render a visual representation of the ColorMunki dial.
    ///
    /// The dial rotates on the right side from Bottom (Reflective) to Top (Ambient).
    pub fn render_device_dial(ui: &mut egui::Ui, highlight_position: &str, size: f32) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
        let painter = ui.painter();
        let center = rect.center();
        let outer_radius = size / 2.0 - 10.0;
        let inner_radius = outer_radius * 0.7;

        // Draw outer ring (dial housing)
        painter.circle_stroke(
            center,
            outer_radius,
            egui::Stroke::new(3.0, egui::Color32::from_gray(140)),
        );

        // Draw active range arc (Right side semicircle: -90 to +90 degrees)
        let arc_radius = outer_radius;
        let arc_start_angle = -std::f32::consts::FRAC_PI_2;
        let arc_end_angle = std::f32::consts::FRAC_PI_2;
        let arc_points: Vec<egui::Pos2> = (0..=50)
            .map(|i| {
                let t = i as f32 / 50.0;
                let angle = arc_start_angle + t * (arc_end_angle - arc_start_angle);
                center + egui::vec2(angle.cos() * arc_radius, angle.sin() * arc_radius)
            })
            .collect();
        painter.add(egui::Shape::Path(egui::epaint::PathShape::line(
            arc_points,
            egui::Stroke::new(2.0, egui::Color32::from_gray(80)),
        )));

        // Define physical layout:
        // 1. REFLECTIVE: Bottom (PI/2) - Circle
        // 2. CALIBRATE: Bottom-Right (PI/4) - CAPSULE/PILL shape
        // 3. PROJECTOR: Right (0) - Circle
        // 4. AMBIENT: Top (-PI/2) - Circle
        let positions = [
            (
                "REFLECTIVE",
                std::f32::consts::FRAC_PI_2,
                egui::Color32::from_rgb(100, 180, 255),
                false, // is_capsule
            ),
            (
                "CALIBRATE",
                std::f32::consts::FRAC_PI_4,
                egui::Color32::YELLOW,
                true, // is_capsule
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

        // Draw markers and labels
        for (name, angle, color, is_capsule) in &positions {
            let is_highlighted = highlight_position.eq_ignore_ascii_case(name)
                || (name == &"PROJECTOR" && highlight_position.eq_ignore_ascii_case("EMISSIVE"));

            let marker_pos =
                center + egui::vec2(angle.cos() * outer_radius, angle.sin() * outer_radius);

            let base_color = if is_highlighted {
                *color
            } else {
                egui::Color32::from_gray(100)
            };

            // Draw Marker (Capsule or Circle)
            if *is_capsule {
                let marker_rect = egui::Rect::from_center_size(marker_pos, egui::vec2(16.0, 8.0));
                painter.rect_filled(marker_rect, 4.0, base_color);
            } else {
                painter.circle_filled(marker_pos, 5.0, base_color);
            }

            // Draw Label
            let label_dist = outer_radius + 20.0;
            let label_pos = center + egui::vec2(angle.cos() * label_dist, angle.sin() * label_dist);
            let font = egui::FontId::proportional(if is_highlighted { 12.0 } else { 10.0 });

            let display_name = if *name == "PROJECTOR" {
                "PROJ/EMIS"
            } else {
                name
            };

            painter.text(
                label_pos,
                egui::Align2::CENTER_CENTER,
                display_name,
                font,
                if is_highlighted {
                    *color
                } else {
                    egui::Color32::from_gray(180)
                },
            );

            // Draw Selector Needle
            if is_highlighted {
                painter.line_segment([center, marker_pos], egui::Stroke::new(3.0, *color));
                painter.circle_filled(center, inner_radius * 0.2, *color);
            }
        }

        // Draw center pivot
        painter.circle_filled(center, 4.0, egui::Color32::WHITE);
        painter.circle_stroke(center, 4.0, egui::Stroke::new(1.0, egui::Color32::GRAY));
    }

    /// Render a small dial check reminder for measurement modes.
    pub fn render_dial_check(ctx: &egui::Context, mode_name: &str) {
        egui::Window::new("⚙️ Dial Check")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::RIGHT_BOTTOM, [-20.0, -80.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Check Dial Position").strong());
                    Self::render_device_dial(ui, mode_name, 100.0);
                    ui.add_space(5.0);
                    ui.label(
                        egui::RichText::new(format!("Set dial to: {}", mode_name.to_uppercase()))
                            .small(),
                    );
                });
            });
    }
}
