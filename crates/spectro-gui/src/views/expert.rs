use crate::components::widgets::render_bento_item;
use crate::t;
use crate::theme::border_color;
use eframe::egui;
use egui_plot::{HLine, Legend, Line, Plot, PlotPoints, VLine};

pub struct ExpertViewContext<'a> {
    pub last_result: Option<&'a spectro_rs::spectrum::MeasurementResult>,
}

pub fn render_expert_workspace(ui: &mut egui::Ui, ui_ctx: &ExpertViewContext) {
    ui.heading("📊 Spectral Power Distribution");

    let plot = Plot::new("spectral_plot")
        .view_aspect(2.5)
        .height(200.0) // Minimum height regardless of aspect ratio
        .include_y(0.0)
        .include_x(380.0)
        .include_x(780.0)
        .legend(Legend::default().position(egui_plot::Corner::RightTop))
        .y_axis_label("Relative Intensity")
        .x_axis_label("Wavelength (nm)")
        .show_axes([true, true])
        .show_grid(true);

    plot.show(ui, |plot_ui| {
        // Draw current measurement
        if let Some(res) = ui_ctx.last_result {
            let points: PlotPoints = res
                .spectrum
                .wavelengths
                .iter()
                .zip(res.spectrum.values.iter())
                .map(|(w, v)| [*w as f64, *v as f64])
                .collect();

            let line = Line::new(points)
                .name("Measurement")
                .color(egui::Color32::from_rgb(0, 255, 128))
                .width(2.5);
            plot_ui.line(line);

            // Mark peak wavelength
            let peak_idx = res
                .spectrum
                .values
                .iter()
                .enumerate()
                .skip(4) // Skip noise below 420nm
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(i, _)| i)
                .unwrap_or(0);
            let peak_wl = 380.0 + peak_idx as f64 * 10.0;
            plot_ui.vline(
                VLine::new(peak_wl)
                    .color(egui::Color32::from_rgba_unmultiplied(255, 255, 0, 100))
                    .style(egui_plot::LineStyle::dashed_dense())
                    .name(format!("Peak: {}nm", peak_wl as i32)),
            );
        }

        // Reference line at 1.0
        plot_ui.hline(
            HLine::new(1.0)
                .color(egui::Color32::DARK_GRAY)
                .style(egui_plot::LineStyle::dashed_loose()),
        );

        // Color region markers (approximate visible spectrum boundaries)
        let color_regions = [
            (380.0, 440.0, "Violet", egui::Color32::from_rgb(148, 0, 211)),
            (440.0, 485.0, "Blue", egui::Color32::from_rgb(0, 0, 255)),
            (485.0, 500.0, "Cyan", egui::Color32::from_rgb(0, 255, 255)),
            (500.0, 565.0, "Green", egui::Color32::from_rgb(0, 255, 0)),
            (565.0, 590.0, "Yellow", egui::Color32::from_rgb(255, 255, 0)),
            (590.0, 625.0, "Orange", egui::Color32::from_rgb(255, 165, 0)),
            (625.0, 780.0, "Red", egui::Color32::from_rgb(255, 0, 0)),
        ];

        for (start, end, _name, color) in color_regions {
            let mid = (start + end) / 2.0;
            plot_ui.vline(
                VLine::new(mid)
                    .color(egui::Color32::from_rgba_unmultiplied(
                        color.r(),
                        color.g(),
                        color.b(),
                        30,
                    ))
                    .width(end as f32 - start as f32),
            );
        }
    });

    // === Multi-dimensional Data Dashboard ===
    ui.add_space(10.0);

    if let Some(res) = ui_ctx.last_result {
        let xyz = res.xyz;
        let lab = res.lab;
        let (chroma, hue) = (lab.chroma(), lab.hue());
        let cct = res.cct;

        // Responsive layout with intelligent grid calculation
        let spacing = 12.0;
        let card_width = 150.0; // Fixed width for visual consistency

        // Calculate optimal cards per row based on available width
        let available_width = ui.available_width();
        let cards_per_row = ((available_width + spacing) / (card_width + spacing))
            .floor()
            .max(1.0) as usize;

        // Pre-calculate total card count
        let total_cards: usize = if res.cri.is_some() { 6 } else { 5 };
        let rows = total_cards.div_ceil(cards_per_row);

        let mut card_index = 0;

        for row_idx in 0..rows {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(spacing, spacing);

                let cards_in_this_row = (cards_per_row).min(total_cards - card_index);

                for _ in 0..cards_in_this_row {
                    match card_index {
                        0 => {
                            render_bento_item(
                                ui,
                                t!("gui-bento-lab"),
                                card_width,
                                card_width,
                                |ui| {
                                    egui::Grid::new("bento_lab").show(ui, |ui| {
                                        ui.label("L*");
                                        ui.label(format!("{:.2}", lab.l));
                                        ui.end_row();
                                        ui.label("a*");
                                        ui.label(format!("{:.2}", lab.a));
                                        ui.end_row();
                                        ui.label("b*");
                                        ui.label(format!("{:.2}", lab.b));
                                        ui.end_row();
                                    });
                                },
                            );
                        }
                        1 => {
                            render_bento_item(
                                ui,
                                t!("gui-bento-xyz"),
                                card_width,
                                card_width,
                                |ui| {
                                    egui::Grid::new("bento_xyz").show(ui, |ui| {
                                        ui.label("X");
                                        ui.label(format!("{:.3}", xyz.x));
                                        ui.end_row();
                                        ui.label("Y");
                                        ui.label(format!("{:.3}", xyz.y));
                                        ui.end_row();
                                        ui.label("Z");
                                        ui.label(format!("{:.3}", xyz.z));
                                        ui.end_row();
                                    });
                                },
                            );
                        }
                        2 => {
                            render_bento_item(
                                ui,
                                t!("gui-bento-indices"),
                                card_width,
                                card_width,
                                |ui| {
                                    egui::Grid::new("bento_indices").show(ui, |ui| {
                                        ui.label(t!("gui-bento-chroma"));
                                        ui.label(format!("{:.1}", chroma));
                                        ui.end_row();
                                        ui.label(t!("gui-bento-hue"));
                                        ui.label(format!("{:.1}°", hue));
                                        ui.end_row();
                                        ui.label(t!("gui-bento-cct"));
                                        ui.label(format!("{:.0}K", cct));
                                        ui.end_row();
                                    });
                                },
                            );
                        }
                        3 => {
                            render_bento_item(
                                ui,
                                t!("gui-bento-peak"),
                                card_width,
                                card_width,
                                |ui| {
                                    ui.vertical_centered(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{:.1} nm",
                                                res.peak_wavelength()
                                            ))
                                            .size(24.0)
                                            .strong(),
                                        );
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "Centroid: {:.1}nm",
                                                res.centroid_wavelength()
                                            ))
                                            .weak(),
                                        );
                                    });
                                },
                            );
                        }
                        4 => {
                            render_bento_item(
                                ui,
                                t!("gui-bento-srgb"),
                                card_width,
                                card_width,
                                |ui| {
                                    ui.horizontal(|ui| {
                                        let (r, g, b) = res.rgb_u8();
                                        let (rect, _) = ui.allocate_at_least(
                                            egui::vec2(40.0, 40.0),
                                            egui::Sense::hover(),
                                        );
                                        ui.painter().rect_filled(
                                            rect,
                                            4.0,
                                            egui::Color32::from_rgb(r, g, b),
                                        );
                                        ui.painter().rect_stroke(
                                            rect,
                                            4.0,
                                            egui::Stroke::new(1.0, border_color(ui.visuals())),
                                        );
                                        ui.add_space(8.0);
                                        ui.vertical(|ui| {
                                            ui.label(format!("RGB: {}, {}, {}", r, g, b));
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "#{:02X}{:02X}{:02X}",
                                                    r, g, b
                                                ))
                                                .monospace()
                                                .weak(),
                                            );
                                        });
                                    });
                                },
                            );
                        }
                        5 => {
                            if let Some(cri) = res.cri {
                                render_bento_item(
                                    ui,
                                    t!("gui-bento-cri"),
                                    card_width,
                                    card_width,
                                    |ui| {
                                        ui.centered_and_justified(|ui| {
                                            ui.label(
                                                egui::RichText::new(format!("{:.0}", cri))
                                                    .size(28.0)
                                                    .strong(),
                                            );
                                        });
                                    },
                                );
                            }
                        }
                        _ => {}
                    }
                    card_index += 1;
                }
            });

            if row_idx < rows - 1 {
                ui.add_space(spacing);
            }
        }
    }
}
