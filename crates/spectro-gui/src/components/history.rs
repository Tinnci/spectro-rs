use crate::shared::MeasurementEntry;
use crate::t;
use crate::theme::success_color;
use eframe::egui;
use spectro_rs::MeasurementMode;

pub struct HistoryContext<'a> {
    pub history: &'a [MeasurementEntry],
    pub delta_e_tolerance: f32,
    pub layout: &'a crate::theme::LayoutConfig,
    pub is_detached: bool,
    pub selected_index: Option<usize>,
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
    Select(usize),
    Delete(usize),
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

                let detach_icon = if ui_ctx.is_detached { "📥" } else { "📤" };
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
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for (idx, entry) in ui_ctx.history.iter().enumerate() {
                        let lab = &entry.result.lab;
                        let (r, g, b) = entry.result.rgb_u8();

                        let is_selected = ui_ctx.selected_index == Some(idx);
                        let bg_color = if is_selected {
                            ui.visuals().selection.bg_fill.gamma_multiply(0.3)
                        } else {
                            egui::Color32::TRANSPARENT
                        };

                        let response = egui::Frame::none()
                            .fill(bg_color)
                            .rounding(4.0)
                            .inner_margin(egui::Margin::same(4.0))
                            .show(ui, |ui| {
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
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} {}",
                                                    mode_icon, entry.timestamp
                                                ))
                                                .small(),
                                            );
                                            if is_selected {
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        if ui
                                                            .button(
                                                                egui::RichText::new("🗑")
                                                                    .small()
                                                                    .color(egui::Color32::RED),
                                                            )
                                                            .on_hover_text("Delete Entry")
                                                            .clicked()
                                                        {
                                                            action = HistoryAction::Delete(idx);
                                                        }
                                                        ui.label(
                                                            egui::RichText::new("👁").small().weak(),
                                                        );
                                                    },
                                                );
                                            }
                                        });

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
                                                egui::RichText::new(format!("ΔE00={:.1}", de))
                                                    .small(),
                                            );
                                        }
                                    });
                                });
                            })
                            .response;

                        let response =
                            ui.interact(response.rect, ui.id().with(idx), egui::Sense::click());
                        if response.clicked() {
                            action = HistoryAction::Select(idx);
                        }
                        if response.hovered() && !is_selected {
                            ui.painter().rect_stroke(
                                response.rect,
                                4.0,
                                egui::Stroke::new(1.0, ui.visuals().widgets.hovered.bg_fill),
                            );
                        }

                        if idx < ui_ctx.history.len() - 1 {
                            ui.add_space(2.0);
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
