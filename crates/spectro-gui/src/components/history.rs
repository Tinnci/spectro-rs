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

                        // Create a unique ID for this row
                        let row_id = ui.id().with("history_row").with(idx);

                        // Main row container with accent bar if selected
                        let row_response = ui
                            .horizontal(|ui| {
                                // Accent bar for selected state (left edge)
                                if is_selected {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2(4.0, 60.0),
                                        egui::Sense::hover(),
                                    );
                                    ui.painter().rect_filled(
                                        rect,
                                        egui::Rounding::ZERO,
                                        ui.visuals().selection.bg_fill,
                                    );
                                    ui.add_space(4.0);
                                } else {
                                    ui.add_space(8.0);
                                }

                                // Enhanced color swatch with shadow effect
                                let swatch_size = 48.0;
                                let (swatch_rect, _) = ui.allocate_exact_size(
                                    egui::vec2(swatch_size, swatch_size),
                                    egui::Sense::hover(),
                                );

                                // Draw shadow
                                let shadow_rect = swatch_rect.translate(egui::vec2(1.0, 2.0));
                                ui.painter().rect_filled(
                                    shadow_rect,
                                    6.0,
                                    egui::Color32::from_black_alpha(30),
                                );

                                // Draw main color swatch
                                ui.painter().rect_filled(
                                    swatch_rect,
                                    6.0,
                                    egui::Color32::from_rgb(r, g, b),
                                );

                                // Stroke around swatch
                                ui.painter().rect_stroke(
                                    swatch_rect,
                                    6.0,
                                    egui::Stroke::new(1.0, egui::Color32::from_black_alpha(40)),
                                );

                                ui.add_space(12.0);

                                // Text content area
                                ui.vertical(|ui| {
                                    ui.add_space(2.0);

                                    // Mode icon and timestamp (secondary info)
                                    let mode_icon = match entry.mode {
                                        MeasurementMode::Reflective => "📄",
                                        MeasurementMode::Emissive => "🖥️",
                                        MeasurementMode::Ambient => "💡",
                                    };

                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} {}",
                                            mode_icon, entry.timestamp
                                        ))
                                        .size(11.0)
                                        .color(ui.visuals().weak_text_color()),
                                    );

                                    // Lab values (primary info)
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "L* {:.0}  a* {:.0}  b* {:.0}",
                                            lab.l, lab.a, lab.b
                                        ))
                                        .size(13.0)
                                        .strong(),
                                    );

                                    // Delta E if available
                                    if let Some(de) = entry.delta_e {
                                        let de_color = if de <= ui_ctx.delta_e_tolerance {
                                            success_color(&ui.ctx().style().visuals)
                                        } else {
                                            egui::Color32::from_rgb(255, 87, 51)
                                        };
                                        ui.label(
                                            egui::RichText::new(format!("ΔE*00 = {:.2}", de))
                                                .size(11.0)
                                                .color(de_color),
                                        );
                                    }
                                });
                            })
                            .response;

                        // Interaction handling
                        let interaction_rect = row_response.rect;
                        let interact_response =
                            ui.interact(interaction_rect, row_id, egui::Sense::click());

                        // Left click to select
                        if interact_response.clicked()
                            && !matches!(action, HistoryAction::Delete(_))
                        {
                            action = HistoryAction::Select(idx);
                        }

                        // Right-click context menu
                        interact_response.context_menu(|ui| {
                            if ui.button("🗑 Delete").clicked() {
                                action = HistoryAction::Delete(idx);
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui.button("📌 Set as Reference").clicked() {
                                // Future: HistoryAction::SetAsReference(idx)
                                ui.close_menu();
                            }
                            if ui.button("📝 Add Note").clicked() {
                                // Future: HistoryAction::AddNote(idx)
                                ui.close_menu();
                            }
                        });

                        // Hover effect
                        if interact_response.hovered() && !is_selected {
                            ui.painter().rect_stroke(
                                interaction_rect,
                                4.0,
                                egui::Stroke::new(1.5, ui.visuals().widgets.hovered.bg_fill),
                            );
                        }

                        // Separator between items
                        if idx < ui_ctx.history.len() - 1 {
                            ui.add_space(4.0);
                            ui.separator();
                            ui.add_space(4.0);
                        }
                    }
                });

            ui.add_space(ui_ctx.layout.spacing);
            ui.horizontal(|ui| {
                if ui.button("📄 CSV").clicked() {
                    action = HistoryAction::ExportCsv;
                }
                if ui.button("📋 JSON").clicked() {
                    action = HistoryAction::ExportJson;
                }
                if ui.button("🎨 CGATS").clicked() {
                    action = HistoryAction::ExportCgats;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(egui::RichText::new("🗑 Clear All").color(egui::Color32::RED))
                        .clicked()
                    {
                        action = HistoryAction::Clear;
                    }
                });
            });
        }
    });

    action
}
