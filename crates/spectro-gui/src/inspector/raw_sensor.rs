use super::InspectorContext;
use eframe::egui;

pub fn render(ui: &mut egui::Ui, ctx: &InspectorContext) {
    ui.add_space(ctx.layout.spacing * 0.5);

    if let Some(data) = ctx.last_result {
        ui.label(egui::RichText::new("Spectral Values (380-780nm, 10nm steps)").strong());
        ui.add_space(ctx.layout.spacing * 0.5);

        // Scrollable table of values
        egui::ScrollArea::vertical()
            .max_height(400.0)
            .show(ui, |ui| {
                egui::Grid::new("raw_values_grid")
                    .num_columns(4)
                    .spacing([15.0, 2.0])
                    .striped(true)
                    .show(ui, |ui| {
                        // Header
                        ui.label(egui::RichText::new("λ (nm)").strong());
                        ui.label(egui::RichText::new("Value").strong());
                        ui.label(egui::RichText::new("λ (nm)").strong());
                        ui.label(egui::RichText::new("Value").strong());
                        ui.end_row();

                        // Values in two columns
                        for i in (0..data.spectrum.values.len()).step_by(2) {
                            let wl1 = 380 + i * 10;
                            ui.label(format!("{}", wl1));
                            ui.label(format!("{:.6}", data.spectrum.values[i]));

                            if i + 1 < data.spectrum.values.len() {
                                let wl2 = 380 + (i + 1) * 10;
                                ui.label(format!("{}", wl2));
                                ui.label(format!("{:.6}", data.spectrum.values[i + 1]));
                            }
                            ui.end_row();
                        }
                    });
            });

        ui.add_space(ctx.layout.spacing);

        // Statistics
        ui.collapsing("📊 Statistics", |ui| {
            let values = &data.spectrum.values;
            let min = values.iter().cloned().fold(f32::INFINITY, f32::min);
            let max = values.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
            let sum: f32 = values.iter().sum();
            let mean = sum / values.len() as f32;

            egui::Grid::new("stats_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Min:");
                    ui.label(format!("{:.6}", min));
                    ui.end_row();
                    ui.label("Max:");
                    ui.label(format!("{:.6}", max));
                    ui.end_row();
                    ui.label("Mean:");
                    ui.label(format!("{:.6}", mean));
                    ui.end_row();
                    ui.label("Total:");
                    ui.label(format!("{:.6}", sum));
                    ui.end_row();
                });
        });
    }
}
