use eframe::egui;
use spectro_rs::tm30::TM30Metrics;

pub struct Tm30Visualizer {
    pub metrics: TM30Metrics,
}

impl Tm30Visualizer {
    pub fn new(metrics: TM30Metrics) -> Self {
        Self { metrics }
    }

    pub fn ui(&self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            ui.heading("TM-30-18 Color Vector Graphic");

            let (rect, _response) =
                ui.allocate_at_least(egui::vec2(300.0, 300.0), egui::Sense::hover());
            let painter = ui.painter_at(rect);

            let center = rect.center();
            let radius = rect.width().min(rect.height()) * 0.4;

            // Draw background grid/circles
            painter.circle_stroke(
                center,
                radius,
                egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
            );
            painter.circle_stroke(
                center,
                radius * 0.5,
                egui::Stroke::new(0.5, egui::Color32::from_gray(40)),
            );

            // Draw axes
            painter.line_segment(
                [
                    center - egui::vec2(radius, 0.0),
                    center + egui::vec2(radius, 0.0),
                ],
                egui::Stroke::new(0.5, egui::Color32::from_gray(60)),
            );
            painter.line_segment(
                [
                    center - egui::vec2(0.0, radius),
                    center + egui::vec2(0.0, radius),
                ],
                egui::Stroke::new(0.5, egui::Color32::from_gray(60)),
            );

            // Scale factor for a' and b'
            // CAM02-UCS a', b' are typically in range [-50, 50] or so.
            // We need to find a good scale. Let's use a fixed scale for now.
            let scale = radius / 40.0;

            // Draw reference circle (it's actually a polygon of the 16 bins)
            let mut ref_points = Vec::new();
            for i in 0..16 {
                let x = center.x + self.metrics.bin_ref_a[i] * scale;
                let y = center.y - self.metrics.bin_ref_b[i] * scale; // Flip Y for screen coordinates
                ref_points.push(egui::pos2(x, y));
            }

            for i in 0..16 {
                let p1 = ref_points[i];
                let p2 = ref_points[(i + 1) % 16];
                painter.line_segment([p1, p2], egui::Stroke::new(2.0, egui::Color32::BLACK));
            }

            // Draw test polygon
            let mut test_points = Vec::new();
            for i in 0..16 {
                let x = center.x + self.metrics.bin_test_a[i] * scale;
                let y = center.y - self.metrics.bin_test_b[i] * scale;
                test_points.push(egui::pos2(x, y));
            }

            // Fill the test polygon with a semi-transparent color
            // We can use a simple triangulation or just draw lines
            for i in 0..16 {
                let p1 = test_points[i];
                let p2 = test_points[(i + 1) % 16];
                painter.line_segment(
                    [p1, p2],
                    egui::Stroke::new(3.0, egui::Color32::from_rgb(255, 0, 0)),
                );
            }

            // Draw shift arrows
            for i in 0..16 {
                let p_ref = ref_points[i];
                let p_test = test_points[i];
                painter.line_segment(
                    [p_ref, p_test],
                    egui::Stroke::new(1.0, egui::Color32::WHITE),
                );
            }

            // Draw bin labels
            for i in 0..16 {
                let angle = (i as f32 * 22.5 + 11.25).to_radians();
                let label_pos = center + egui::vec2(angle.cos(), -angle.sin()) * (radius + 15.0);
                painter.text(
                    label_pos,
                    egui::Align2::CENTER_CENTER,
                    (i + 1).to_string(),
                    egui::FontId::proportional(10.0),
                    egui::Color32::from_gray(180),
                );
            }

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("Rf: {:.1}", self.metrics.rf))
                        .strong()
                        .color(egui::Color32::WHITE),
                );
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new(format!("Rg: {:.1}", self.metrics.rg))
                        .strong()
                        .color(egui::Color32::WHITE),
                );
            });
            ui.label(format!(
                "CCT: {:.0} K (Duv: {:.4})",
                self.metrics.cct, self.metrics.duv
            ));

            ui.add_space(20.0);
            ui.heading("99 Color Evaluation Samples (CES)");
            ui.add_space(5.0);

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                        for (i, rgb) in self.metrics.ces_rgb.iter().enumerate() {
                            let color = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
                            let (rect, _response) =
                                ui.allocate_at_least(egui::vec2(20.0, 20.0), egui::Sense::hover());
                            ui.painter().rect_filled(rect, 2.0, color);

                            // Tooltip for each sample
                            _response.on_hover_text(format!("CES {:02}", i + 1));
                        }
                    });
                });

            ui.add_space(20.0);
            ui.heading("Hue Bin Details");
            ui.add_space(5.0);

            egui::Grid::new("tm30_bin_grid")
                .striped(true)
                .num_columns(4)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("Bin");
                    ui.label("Rf");
                    ui.label("Chroma Shift");
                    ui.label("Hue Shift");
                    ui.end_row();

                    for i in 0..16 {
                        ui.label(format!("{}", i + 1));
                        ui.label(format!("{:.1}", self.metrics.bin_rf[i]));

                        let chroma_shift = self.metrics.bin_chroma_shift[i] * 100.0;
                        let chroma_color = if chroma_shift > 0.0 {
                            egui::Color32::from_rgb(100, 255, 100)
                        } else {
                            egui::Color32::from_rgb(255, 100, 100)
                        };
                        ui.colored_label(chroma_color, format!("{:.1}%", chroma_shift));

                        ui.label(format!(
                            "{:.1}°",
                            self.metrics.bin_hue_shift[i].to_degrees()
                        ));
                        ui.end_row();
                    }
                });
        });
    }
}
