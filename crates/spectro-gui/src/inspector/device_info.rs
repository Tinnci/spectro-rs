use super::InspectorContext;
use crate::t;
use crate::theme::{error_color, muted_text_color, plot_line_color, success_color, warning_color};
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};

pub fn render(ui: &mut egui::Ui, ctx: &InspectorContext) {
    ui.add_space(ctx.layout.spacing * 0.5);

    // Basic Device Info
    ui.collapsing(t!("gui-device-info"), |ui| {
        egui::Grid::new("device_info_grid")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                if let Some(ref basic) = ctx.device_info.basic {
                    ui.label("Model:");
                    ui.label(&basic.model);
                    ui.end_row();
                    ui.label("Serial:");
                    ui.label(&basic.serial);
                    ui.end_row();
                    ui.label("Firmware:");
                    ui.label(&basic.firmware);
                    ui.end_row();
                } else {
                    ui.label(t!("gui-status"));
                    ui.colored_label(
                        warning_color(&ui.ctx().style().visuals),
                        t!("gui-not-connected"),
                    );
                    ui.end_row();
                }

                if let Some(cal_ver) = ctx.device_info.cal_version {
                    ui.label("Cal Version:");
                    ui.label(format!("0x{:04X}", cal_ver));
                    ui.end_row();
                }
            });
    });

    // EEPROM Calibration Data
    ui.collapsing(t!("gui-eeprom-cal"), |ui| {
        if let Some(ref white_ref) = ctx.device_info.white_ref {
            ui.label(t!("gui-white-ref"));

            // Mini plot of white reference
            let plot = Plot::new("white_ref_plot")
                .height(100.0)
                .show_axes([true, true])
                .include_y(0.0);

            let visuals = ui.ctx().style().visuals.clone();
            plot.show(ui, |plot_ui| {
                let points: PlotPoints = white_ref
                    .iter()
                    .enumerate()
                    .map(|(i, v)| [(380 + i * 10) as f64, *v as f64])
                    .collect();
                plot_ui.line(
                    Line::new(points)
                        .color(plot_line_color(&visuals))
                        .width(1.5),
                );
            });
        } else {
            ui.colored_label(
                muted_text_color(&ui.ctx().style().visuals),
                t!("gui-white-ref-not-avail"),
            );
        }

        ui.add_space(ctx.layout.spacing * 0.5);

        // Emissive calibration coefficients
        if let Some(ref emis) = ctx.device_info.emis_coef {
            ui.collapsing(t!("gui-emissive-coef"), |ui| {
                ui.label(format!("Count: {} bands", emis.len()));
                if !emis.is_empty() {
                    ui.label(format!(
                        "Range: {:.4} - {:.4}",
                        emis.iter().cloned().fold(f32::INFINITY, f32::min),
                        emis.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
                    ));
                }
            });
        }

        // Ambient calibration coefficients
        if let Some(ref amb) = ctx.device_info.amb_coef {
            ui.collapsing(t!("gui-ambient-coef"), |ui| {
                ui.label(format!("Count: {} bands", amb.len()));
                if !amb.is_empty() {
                    ui.label(format!(
                        "Range: {:.4} - {:.4}",
                        amb.iter().cloned().fold(f32::INFINITY, f32::min),
                        amb.iter().cloned().fold(f32::NEG_INFINITY, f32::max)
                    ));
                }
            });
        }

        ui.add_space(ctx.layout.spacing * 0.5);

        // Linearization polynomials
        if let Some(ref lin) = ctx.device_info.lin_normal {
            ui.label(format!("Lin (Normal): {:?}", lin));
        }
        if let Some(ref lin) = ctx.device_info.lin_high {
            ui.label(format!("Lin (High Gain): {:?}", lin));
        }
    });

    // Connection Status
    ui.collapsing("🔌 Connection Status", |ui| {
        egui::Grid::new("conn_status_grid")
            .num_columns(2)
            .spacing([20.0, 4.0])
            .show(ui, |ui| {
                ui.label("Connected:");
                if ctx.is_connected {
                    ui.colored_label(success_color(&ui.ctx().style().visuals), "Yes ✓");
                } else {
                    ui.colored_label(error_color(&ui.ctx().style().visuals), "No ✗");
                }
                ui.end_row();

                ui.label("Calibrated:");
                if ctx.is_calibrated {
                    ui.colored_label(success_color(&ui.ctx().style().visuals), "Yes ✓");
                } else {
                    ui.colored_label(warning_color(&ui.ctx().style().visuals), "No");
                }
                ui.end_row();

                ui.label("Mode:");
                ui.label(format!("{:?}", ctx.selected_mode));
                ui.end_row();
            });
    });
}
