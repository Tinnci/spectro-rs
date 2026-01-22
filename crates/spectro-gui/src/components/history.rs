use crate::shared::MeasurementEntry;
use crate::t;
use crate::theme::success_color;
use eframe::egui;
use spectro_rs::{MeasurementMode, colorimetry::XYZ};

pub struct HistoryContext<'a> {
    pub history: &'a [MeasurementEntry],
    pub delta_e_tolerance: f32,
    pub layout: &'a crate::theme::LayoutConfig,
    pub is_detached: bool,
}

#[derive(Default)]
pub enum HistoryAction {
    #[default]
    None,
    Clear,
    ExportCsv,
    ExportJson,
    ExportCgats,
    Close,
    Detach,
    Attach,
}

pub fn render_history(ui: &mut egui::Ui, ui_ctx: &HistoryContext) -> HistoryAction {
    let mut action = HistoryAction::None;

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.heading(t!("gui-history-title"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⏴").on_hover_text(t!("gui-hide")).clicked() {
                    action = HistoryAction::Close;
                }

                let detach_icon = if ui_ctx.is_detached { "📥" } else { "⇗" };
                let detach_text = if ui_ctx.is_detached {
                    t!("gui-attach")
                } else {
                    t!("gui-detach")
                };
                if ui.button(detach_icon).on_hover_text(detach_text).clicked() {
                    action = if ui_ctx.is_detached {
                        HistoryAction::Attach
                    } else {
                        HistoryAction::Detach
                    };
                }
            });
        });
        ui.separator();

        if ui_ctx.history.is_empty() {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new(t!("gui-no-data")).weak());
            });
        } else {
            egui::ScrollArea::vertical().show(ui, |ui| {
                for (idx, entry) in ui_ctx.history.iter().enumerate() {
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
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, 4.0, egui::Color32::from_rgb(r, g, b));

                        ui.vertical(|ui| {
                            // Show mode icon and timestamp
                            let mode_icon = match entry.mode {
                                MeasurementMode::Reflective => "📄",
                                MeasurementMode::Emissive => "🖥️",
                                MeasurementMode::Ambient => "💡",
                            };
                            ui.label(
                                egui::RichText::new(format!("{} {}", mode_icon, entry.timestamp))
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
                                let color = if de <= ui_ctx.delta_e_tolerance {
                                    success_color(&ui.ctx().style().visuals)
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

                    if idx < ui_ctx.history.len() - 1 {
                        ui.separator();
                    }
                }
            });

            ui.add_space(ui_ctx.layout.spacing);
            ui.horizontal(|ui| {
                if ui.button("CSV").clicked() {
                    action = HistoryAction::ExportCsv;
                }
                if ui.button("JSON").clicked() {
                    action = HistoryAction::ExportJson;
                }
                if ui.button("CGATS").clicked() {
                    action = HistoryAction::ExportCgats;
                }
                if ui.button("Clear").clicked() {
                    action = HistoryAction::Clear;
                }
            });
        }
    });

    action
}
