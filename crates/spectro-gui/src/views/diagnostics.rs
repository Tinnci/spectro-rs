use crate::{app::SpectroApp, shared::DeviceCommand};
use eframe::egui;

pub fn render_diagnostics_view(app: &mut SpectroApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("🩺 Sensor Diagnostics");
        ui.add_space(10.0);

        ui.label("This tool runs a comprehensive test of the sensor linear range and dark current noise.");
        ui.label("⚠️ Please ensure the device is in the Calibration (White Tile) position before running.");

        ui.add_space(20.0);

        if app.is_busy {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(app.status_msg.clone());
            });
        } else if ui.button("▶️ Run Diagnostics").clicked() {
            if !app.is_connected {
                app.status_msg = "Device not connected".into();
            } else {
                app.is_busy = true;
                app.cmd_tx.send(DeviceCommand::TestSensor).ok();
                app.diagnostics_report = None;
            }
        }

        ui.add_space(20.0);

        if let Some(report) = &mut app.diagnostics_report {
            ui.group(|ui| {
                ui.label("Diagnostic Results:");
                ui.add_space(5.0);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.add(
                        egui::TextEdit::multiline(report)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .desired_rows(20)
                            .lock_focus(true),
                    );
                });
            });
        }
    });
}
