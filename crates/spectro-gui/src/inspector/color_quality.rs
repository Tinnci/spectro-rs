use super::InspectorContext;
use crate::t;
use eframe::egui;

pub fn render(ui: &mut egui::Ui, ctx: &InspectorContext) {
    ui.add_space(5.0);
    ui.heading(t!("gui-color-quality-tm30"));
    ui.add_space(10.0);

    if let Some(metrics) = ctx.last_tm30 {
        let visualizer = crate::tm30_gui::Tm30Visualizer::new(metrics.clone());
        visualizer.ui(ui, ctx.layout);
    } else if ctx.last_result.is_some() {
        // Centered message for TM-30 not available
        let available = ui.available_size();
        ui.allocate_ui_with_layout(
            available,
            egui::Layout::centered_and_justified(egui::Direction::TopDown),
            |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("No TM-30 data available.").weak());
                    ui.label(
                        egui::RichText::new(
                            "Please take an Emissive measurement to see color quality metrics.",
                        )
                        .small()
                        .weak(),
                    );
                });
            },
        );
    }
}
