use crate::components::widgets::render_bento_item;
use crate::t;
use crate::theme::border_color;
use eframe::egui;
use egui_plot::{HLine, Legend, Line, Plot, PlotPoints, VLine};

pub struct ExpertViewContext<'a> {
    pub last_result: Option<&'a spectro_rs::spectrum::MeasurementResult>,
    pub layout: &'a crate::theme::LayoutConfig,
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
    ui.add_space(ui_ctx.layout.spacing);

    if let Some(res) = ui_ctx.last_result {
        let xyz = res.xyz;
        let lab = res.lab;
        let (chroma, hue) = (lab.chroma(), lab.hue());
        let cct = res.cct;

        // Fluid Responsive Grid calculation (similar to CSS: repeat(auto-fit, minmax(150px, 1fr)))
        let spacing = ui_ctx.layout.spacing;
        let min_card_width = ui_ctx.layout.bento_min_width;

        let available_width = ui.available_width();
        let cards_per_row = ((available_width + spacing) / (min_card_width + spacing))
            .floor()
            .max(1.0) as usize;

        // Distribute remaining space so cards fill the entire row
        let fluid_card_width =
            (available_width - (cards_per_row as f32 - 1.0) * spacing) / cards_per_row as f32;

        // Pre-calculate total card count
        let mut total_cards: usize = 5; // Base cards: Lab, XYZ, Indices, Peak, RGB
        if res.cri.is_some() {
            total_cards += 1;
        }
        if res.spectrum.mode == spectro_rs::spectrum::MeasurementMode::Emissive
            || res.spectrum.mode == spectro_rs::spectrum::MeasurementMode::Ambient
        {
            total_cards += 1;
        }

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
                                fluid_card_width,
                                fluid_card_width,
                                |ui| {
                                    let mut rows = vec![
                                        ("L*", format!("{:.2}", lab.l)),
                                        ("a*", format!("{:.2}", lab.a)),
                                        ("b*", format!("{:.2}", lab.b)),
                                    ];

                                    // For emissive, Lab isn't the primary reference, so we can add more context if needed
                                    if res.spectrum.mode
                                        == spectro_rs::spectrum::MeasurementMode::Emissive
                                    {
                                        rows.push(("Chroma", format!("{:.1}", chroma)));
                                    }

                                    for (label, value) in rows {
                                        ui.horizontal(|ui| {
                                            ui.label(label);
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.label(egui::RichText::new(value).strong());
                                                },
                                            );
                                        });
                                    }
                                },
                            );
                        }
                        1 => {
                            if res.spectrum.mode == spectro_rs::spectrum::MeasurementMode::Emissive
                                || res.spectrum.mode
                                    == spectro_rs::spectrum::MeasurementMode::Ambient
                            {
                                let (title, unit) = if res.spectrum.mode
                                    == spectro_rs::spectrum::MeasurementMode::Emissive
                                {
                                    (t!("gui-bento-luminance"), "cd/m²")
                                } else {
                                    (t!("gui-bento-illuminance"), "Lux")
                                };

                                render_bento_item(
                                    ui,
                                    title,
                                    fluid_card_width,
                                    fluid_card_width,
                                    |ui| {
                                        ui.vertical_centered(|ui| {
                                            ui.add_space(8.0);
                                            ui.label(
                                                egui::RichText::new(format!("{:.2}", xyz.y))
                                                    .size(28.0)
                                                    .strong(),
                                            );
                                            ui.label(egui::RichText::new(unit).weak());
                                        });
                                    },
                                );
                            } else {
                                // Default for reflective: XYZ
                                render_bento_item(
                                    ui,
                                    t!("gui-bento-xyz"),
                                    fluid_card_width,
                                    fluid_card_width,
                                    |ui| {
                                        let rows = [
                                            ("X", format!("{:.3}", xyz.x)),
                                            ("Y", format!("{:.3}", xyz.y)),
                                            ("Z", format!("{:.3}", xyz.z)),
                                        ];
                                        for (label, value) in rows {
                                            ui.horizontal(|ui| {
                                                ui.label(label);
                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        ui.label(
                                                            egui::RichText::new(value).strong(),
                                                        );
                                                    },
                                                );
                                            });
                                        }
                                    },
                                );
                            }
                        }
                        2 => {
                            // If we already showed Luminance/XYZ in slot 1, we might need to push XYZ here or similar
                            // Let's keep it simple and just show Indices
                            render_bento_item(
                                ui,
                                t!("gui-bento-indices"),
                                fluid_card_width,
                                fluid_card_width,
                                |ui| {
                                    let rows = vec![
                                        (t!("gui-bento-chroma"), format!("{:.1}", chroma)),
                                        (t!("gui-bento-hue"), format!("{:.1}°", hue)),
                                        (t!("gui-bento-cct"), format!("{:.0}K", cct)),
                                    ];
                                    if res.spectrum.mode
                                        == spectro_rs::spectrum::MeasurementMode::Emissive
                                    {
                                        // For emissive, du'v' is often more useful than Hue
                                        // (Placeholder for now)
                                    }

                                    for (label, value) in rows {
                                        ui.horizontal(|ui| {
                                            ui.label(label);
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    ui.label(egui::RichText::new(value).strong());
                                                },
                                            );
                                        });
                                    }
                                },
                            );
                        }
                        3 => {
                            render_bento_item(
                                ui,
                                t!("gui-bento-peak"),
                                fluid_card_width,
                                fluid_card_width,
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
                                fluid_card_width,
                                fluid_card_width,
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
                                    fluid_card_width,
                                    fluid_card_width,
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
