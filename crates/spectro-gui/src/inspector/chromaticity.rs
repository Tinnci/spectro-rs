use super::InspectorContext;
use crate::theme::plot_line_color;
use eframe::egui;
use egui_plot::{Legend, Line, Plot, PlotPoints, Points};
use spectro_rs::colorimetry::{X_BAR_2, Y_BAR_2, Z_BAR_2};

pub fn render(ui: &mut egui::Ui, ctx: &InspectorContext) {
    ui.add_space(5.0);
    ui.heading("🎯 CIE 1931 xy Chromaticity");
    ui.add_space(10.0);

    let plot = Plot::new("chromaticity_plot")
        .data_aspect(1.0)
        .view_aspect(1.0)
        .include_x(0.0)
        .include_x(0.8)
        .include_y(0.0)
        .include_y(0.9)
        .legend(Legend::default())
        .allow_zoom(true)
        .allow_drag(true);

    let visuals = ui.ctx().style().visuals.clone();
    plot.show(ui, |plot_ui| {
        // 1. Draw Spectral Locus (Horseshoe)
        let mut locus_points = Vec::new();
        for i in 0..41 {
            let sum = X_BAR_2[i] + Y_BAR_2[i] + Z_BAR_2[i];
            if sum > 0.0 {
                locus_points.push([(X_BAR_2[i] / sum) as f64, (Y_BAR_2[i] / sum) as f64]);
            }
        }
        // Close the horseshoe with the purple line (connect 380nm to 780nm)
        if !locus_points.is_empty() {
            locus_points.push(locus_points[0]);
        }

        plot_ui.line(
            Line::new(PlotPoints::from(locus_points))
                .color(egui::Color32::from_gray(100))
                .name("Spectral Locus"),
        );

        // 2. Draw D65 White Point
        let d65_x = 0.31272;
        let d65_y = 0.32903;
        plot_ui.points(
            Points::new(vec![[d65_x, d65_y]])
                .color(plot_line_color(&visuals))
                .shape(egui_plot::MarkerShape::Plus)
                .name("D65"),
        );

        // 3. Draw History Trail (Faded)
        let history_points: Vec<[f64; 2]> = ctx
            .history
            .iter()
            .rev() // Draw from oldest to newest
            .map(|e| {
                let xyz = e.result.xyz;
                let (x, y) = xyz.to_chromaticity();
                [x as f64, y as f64]
            })
            .collect();

        if history_points.len() > 1 {
            plot_ui.line(
                Line::new(PlotPoints::from(history_points))
                    .color(egui::Color32::from_rgba_unmultiplied(100, 100, 100, 100))
                    .name("History Path"),
            );
        }

        // 4. Draw Current Point
        if let Some(data) = ctx.last_result {
            let xyz = data.xyz;
            let (x, y) = xyz.to_chromaticity();
            plot_ui.points(
                Points::new(vec![[x as f64, y as f64]])
                    .color(egui::Color32::RED)
                    .radius(4.0)
                    .name("Current Entry"),
            );
        }
    });

    ui.add_space(ctx.layout.spacing);
    ui.label("The horseshoe-shaped region represents all colors visible to the human eye. The red dot indicates the most recent measurement.");
}
