use super::InspectorContext;
use crate::theme::plot_line_color;
use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints};
use spectro_rs::colorimetry::{XYZ, illuminant};

pub fn render(ui: &mut egui::Ui, ctx: &InspectorContext) {
    ui.add_space(ctx.layout.spacing * 0.5);

    if ctx.history.is_empty() {
        // This case should be handled by the parent before calling render,
        // but handling it here just in case.
        return;
    }

    ui.heading("📈 Measurement Trend");
    ui.add_space(ctx.layout.spacing);

    // L* trend
    let l_points: PlotPoints = ctx
        .history
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let xyz = entry.result.xyz;
            let xyz_norm = XYZ {
                x: xyz.x / 100.0,
                y: xyz.y / 100.0,
                z: xyz.z / 100.0,
            };
            let lab = xyz_norm.to_lab(illuminant::D65_2);
            [i as f64, lab.l as f64]
        })
        .collect();

    let plot = Plot::new("trend_plot")
        .height(150.0)
        .show_axes([true, true])
        .legend(Legend::default());

    let visuals = ui.ctx().style().visuals.clone();
    plot.show(ui, |plot_ui| {
        plot_ui.line(
            Line::new(l_points)
                .color(plot_line_color(&visuals))
                .name("L*"),
        );
    });

    ui.add_space(ctx.layout.spacing);

    // Statistics summary
    if let Some(last) = ctx.history.last() {
        let xyz = last.result.xyz;
        let xyz_norm = XYZ {
            x: xyz.x / 100.0,
            y: xyz.y / 100.0,
            z: xyz.z / 100.0,
        };
        let lab = xyz_norm.to_lab(illuminant::D65_2);

        egui::Grid::new("trend_stats_grid")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                ui.label("Latest L*:");
                ui.label(format!("{:.2}", lab.l));
                ui.end_row();
                ui.label("Latest a*:");
                ui.label(format!("{:.2}", lab.a));
                ui.end_row();
                ui.label("Latest b*:");
                ui.label(format!("{:.2}", lab.b));
                ui.end_row();
                ui.label("Total Measurements:");
                ui.label(format!("{}", ctx.history.len()));
                ui.end_row();
            });
    }
}
