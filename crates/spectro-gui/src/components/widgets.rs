use crate::theme::{border_color, info_panel_color, muted_text_color};
use eframe::egui;

pub fn render_bento_item<R>(
    ui: &mut egui::Ui,
    title: String,
    min_width: f32,
    max_width: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let visuals = &ui.ctx().style().visuals;

    ui.scope(|ui| {
        ui.set_min_width(min_width);
        ui.set_max_width(max_width);

        egui::Frame::none()
            .fill(info_panel_color(visuals))
            .stroke(egui::Stroke::new(1.0, border_color(visuals)))
            .rounding(6.0)
            .inner_margin(egui::Margin::same(ui.spacing().item_spacing.y))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(title.to_uppercase())
                            .size(10.0)
                            .color(muted_text_color(visuals))
                            .strong(),
                    );
                    ui.add_space(ui.spacing().item_spacing.y * 0.4);
                    add_contents(ui)
                })
                .inner
            })
            .inner
    })
    .inner
}

#[expect(
    dead_code,
    reason = "Utility icon for planned descriptive tooltips to reduce UI text density"
)]
pub fn help_icon(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new("ⓘ").color(muted_text_color(ui.visuals())))
        .on_hover_text(text);
}
