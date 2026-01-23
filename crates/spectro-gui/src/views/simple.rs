use crate::theme::{
    border_color, info_panel_color, muted_text_color, overlay_shadow_color, success_color,
};
use eframe::egui;
use spectro_rs::colorimetry::Lab;

pub struct SimpleViewContext<'a> {
    pub last_result: Option<&'a spectro_rs::spectrum::MeasurementResult>,
    pub reference_lab: Option<Lab>,
    pub delta_e_tolerance: f32,
    pub layout: &'a crate::theme::LayoutConfig,
}

pub fn render_simple_workspace(ui: &mut egui::Ui, ui_ctx: &SimpleViewContext) {
    ui.vertical_centered(|ui| {
        ui.add_space(ui_ctx.layout.spacing * 2.0);

        if let Some(res) = ui_ctx.last_result {
            let (r, g, b) = (
                (res.rgb.0 * 255.0) as u8,
                (res.rgb.1 * 255.0) as u8,
                (res.rgb.2 * 255.0) as u8,
            );
            let lab = res.lab;

            // === Giant Color Swatch ===
            let available_size = ui.available_size();
            let swatch_size = available_size.x.min(available_size.y * 0.5).min(300.0);

            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(swatch_size, swatch_size), egui::Sense::hover());

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

            ui.add_space(ui_ctx.layout.spacing * 2.0);

            // === Pass/Fail Indicator ===
            if let Some(ref_lab) = ui_ctx.reference_lab {
                let delta_e = ref_lab.delta_e_2000(&lab);
                let passed = delta_e <= ui_ctx.delta_e_tolerance;
                let color = if passed {
                    success_color(&ui.ctx().style().visuals)
                } else {
                    egui::Color32::from_rgb(220, 53, 69) // Red
                };

                let status_text = if passed { "✓ PASS" } else { "✗ FAIL" };
                ui.colored_label(color, egui::RichText::new(status_text).size(48.0).strong());

                ui.add_space(ui_ctx.layout.spacing);
                ui.label(
                    egui::RichText::new(format!("ΔE*00 = {:.2}", delta_e))
                        .size(24.0)
                        .color(muted_text_color(&ui.ctx().style().visuals)),
                );

                let delta_e_76 = ref_lab.delta_e_76(&lab);
                ui.label(
                    egui::RichText::new(format!("ΔE*76 = {:.2}", delta_e_76))
                        .size(14.0)
                        .color(egui::Color32::DARK_GRAY),
                );

                ui.add_space(ui_ctx.layout.spacing * 0.5);
                ui.label(
                    egui::RichText::new(format!("Tolerance: ≤ {:.1}", ui_ctx.delta_e_tolerance))
                        .size(14.0)
                        .color(egui::Color32::DARK_GRAY),
                );
            }

            ui.add_space(ui_ctx.layout.spacing * 2.0);

            // === Key Metrics (Large Font) ===
            ui.horizontal(|ui| {
                ui.add_space(ui.available_width() / 2.0 - ui_ctx.layout.bento_min_width);

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
                            ui.add_space(ui_ctx.layout.spacing * 2.0);
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
                            ui.add_space(ui_ctx.layout.spacing * 2.0);
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

            ui.add_space(ui_ctx.layout.spacing * 2.0);

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
            ui.add_space(ui_ctx.layout.spacing * 10.0); // Large space before footer
            ui.label(
                egui::RichText::new("📷")
                    .size(64.0)
                    .color(egui::Color32::from_rgb(80, 80, 100)),
            );
            ui.add_space(ui_ctx.layout.spacing * 2.0);
            ui.label(
                egui::RichText::new("No measurement yet")
                    .size(20.0)
                    .color(muted_text_color(&ui.ctx().style().visuals)),
            );
            ui.add_space(ui_ctx.layout.spacing);
            ui.label(
                egui::RichText::new("Click 'Measure' to take a reading")
                    .size(14.0)
                    .color(egui::Color32::DARK_GRAY),
            );
        }
    });
}
