use super::InspectorContext;
use eframe::egui;
use spectro_rs::colorimetry::{XYZ, illuminant};

pub fn render(ui: &mut egui::Ui, ctx: &InspectorContext) {
    ui.add_space(ctx.layout.spacing * 0.5);

    ui.collapsing("🎯 White Point Reference", |ui| {
        let wp = illuminant::D65_2;
        egui::Grid::new("wp_grid")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                ui.label("Illuminant:");
                ui.label("D65 (2° Observer)");
                ui.end_row();
                ui.label("Xn:");
                ui.label(format!("{:.5}", wp.x));
                ui.end_row();
                ui.label("Yn:");
                ui.label(format!("{:.5}", wp.y));
                ui.end_row();
                ui.label("Zn:");
                ui.label(format!("{:.5}", wp.z));
                ui.end_row();
            });
    });

    ui.collapsing("📐 Observer Functions", |ui| {
        ui.label("Currently using: CIE 1931 2° Standard Observer");
        ui.add_space(ctx.layout.spacing * 0.5);

        // Option to show CMF plot
        ui.horizontal(|ui| {
            ui.label("CMFs:");
            ui.label("x̄(λ), ȳ(λ), z̄(λ) from 380-780nm");
        });
    });

    ui.collapsing("🔄 Conversion Pipeline", |ui| {
        ui.label(egui::RichText::new("Data Flow:").strong());
        ui.add_space(ctx.layout.spacing * 0.5);

        let pipeline = [
            "1. Raw Sensor (128 pixels)",
            "   ↓ EEPROM Matrix Transform",
            "2. Spectral Data (36 bands)",
            "   ↓ Dark Subtraction",
            "3. Corrected Spectrum",
            "   ↓ CMF Integration",
            "4. CIE XYZ",
            "   ↓ Bradford Adaptation",
            "5. Lab (D65)",
        ];

        for step in pipeline {
            ui.label(egui::RichText::new(step).monospace());
        }
    });

    if let Some(data) = ctx.last_result {
        ui.collapsing("🧪 Current Calculation", |ui| {
            let xyz = data.xyz;
            let xyz_norm = XYZ {
                x: xyz.x / 100.0,
                y: xyz.y / 100.0,
                z: xyz.z / 100.0,
            };
            let lab = xyz_norm.to_lab(illuminant::D65_2);

            ui.label(format!("Mode: {:?}", data.spectrum.mode));
            ui.add_space(ctx.layout.spacing * 0.5);

            egui::Grid::new("calc_grid")
                .num_columns(2)
                .spacing([20.0, 4.0])
                .show(ui, |ui| {
                    ui.label("XYZ (raw):");
                    ui.label(format!("({:.3}, {:.3}, {:.3})", xyz.x, xyz.y, xyz.z));
                    ui.end_row();
                    ui.label("XYZ (norm):");
                    ui.label(format!(
                        "({:.4}, {:.4}, {:.4})",
                        xyz_norm.x, xyz_norm.y, xyz_norm.z
                    ));
                    ui.end_row();
                    ui.label("Lab:");
                    ui.label(format!("({:.2}, {:.2}, {:.2})", lab.l, lab.a, lab.b));
                    ui.end_row();
                });
        });

        if !data.spectrum.metadata.is_empty() {
            ui.collapsing("📡 Advanced DSP Diagnostics", |ui| {
                ui.add_space(ctx.layout.spacing * 0.5);
                egui::Grid::new("dsp_meta_grid")
                    .num_columns(2)
                    .spacing([20.0, 4.0])
                    .show(ui, |ui| {
                        let mut sorted_meta: Vec<_> = data.spectrum.metadata.iter().collect();
                        sorted_meta.sort_by_key(|(k, _)| *k);

                        for (key, val) in sorted_meta {
                            let label = key.replace('_', " ").to_uppercase();
                            ui.label(format!("{}:", label));
                            ui.label(egui::RichText::new(val).monospace().strong());
                            ui.end_row();
                        }
                    });
            });
        }
    }
}
