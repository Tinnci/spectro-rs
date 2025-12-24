//! Graphical User Interface for spectro-rs.
//!
//! This module implements the main application window using the [`eframe`] framework.

use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui;
use egui_plot::{HLine, Line, Plot, PlotPoints};
use spectro_core::{discover, BoxedSpectrometer, MeasurementMode, SpectralData};
use std::thread;

/// Messages sent from the UI thread to the Device worker thread.
enum DeviceCommand {
    Connect,
    Calibrate,
    Measure(MeasurementMode),
}

/// Messages sent from the Device worker thread to the UI thread.
enum UIUpdate {
    Connected(String),    // Device model/serial
    Status(String),       // Status text
    Result(SpectralData), // Measurement result
    Error(String),        // Error message
}

pub struct SpectroApp {
    // Communication
    cmd_tx: Sender<DeviceCommand>,
    update_rx: Receiver<UIUpdate>,

    // App State
    device_info: String,
    status_msg: String,
    last_result: Option<SpectralData>,
    is_busy: bool,
}

impl SpectroApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Customize look and feel
        let mut visuals = egui::Visuals::dark();
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(15, 15, 15);
        cc.egui_ctx.set_visuals(visuals);

        let (cmd_tx, cmd_rx) = unbounded();
        let (update_tx, update_rx) = unbounded();

        // Spawn the hardware worker thread
        thread::spawn(move || {
            let mut device: Option<BoxedSpectrometer> = None;

            while let Ok(cmd) = cmd_rx.recv() {
                match cmd {
                    DeviceCommand::Connect => {
                        update_tx
                            .send(UIUpdate::Status("Searching for device...".into()))
                            .ok();
                        match discover() {
                            Ok(d) => {
                                let info = d
                                    .info()
                                    .map(|idx| format!("{} ({})", idx.model, idx.serial))
                                    .unwrap_or_else(|_| "Unknown Device".into());
                                device = Some(d);
                                update_tx.send(UIUpdate::Connected(info)).ok();
                                update_tx.send(UIUpdate::Status("Ready".into())).ok();
                            }
                            Err(e) => {
                                update_tx
                                    .send(UIUpdate::Error(format!("Discovery failed: {}", e)))
                                    .ok();
                            }
                        }
                    }
                    DeviceCommand::Calibrate => {
                        if let Some(ref mut d) = device {
                            update_tx
                                .send(UIUpdate::Status(
                                    "Calibrating... follow device instructions".into(),
                                ))
                                .ok();
                            match d.calibrate() {
                                Ok(_) => {
                                    update_tx
                                        .send(UIUpdate::Status("Calibration successful".into()))
                                        .ok();
                                }
                                Err(e) => {
                                    update_tx
                                        .send(UIUpdate::Error(format!("Calibration failed: {}", e)))
                                        .ok();
                                }
                            }
                        }
                    }
                    DeviceCommand::Measure(mode) => {
                        if let Some(ref mut d) = device {
                            update_tx.send(UIUpdate::Status("Measuring...".into())).ok();
                            match d.measure(mode) {
                                Ok(data) => {
                                    update_tx.send(UIUpdate::Result(data)).ok();
                                    update_tx
                                        .send(UIUpdate::Status("Measure complete".into()))
                                        .ok();
                                }
                                Err(e) => {
                                    update_tx
                                        .send(UIUpdate::Error(format!("Measurement failed: {}", e)))
                                        .ok();
                                }
                            }
                        }
                    }
                }
            }
        });

        // Auto-connect on startup
        cmd_tx.send(DeviceCommand::Connect).ok();

        Self {
            cmd_tx,
            update_rx,
            device_info: "Not Connected".into(),
            status_msg: "Initializing...".into(),
            last_result: None,
            is_busy: false,
        }
    }
}

impl eframe::App for SpectroApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle updates from hardware thread
        while let Ok(update) = self.update_rx.try_recv() {
            match update {
                UIUpdate::Connected(info) => self.device_info = info,
                UIUpdate::Status(msg) => {
                    self.status_msg = msg;
                    self.is_busy = false;
                }
                UIUpdate::Result(data) => self.last_result = Some(data),
                UIUpdate::Error(err) => {
                    self.status_msg = format!("Error: {}", err);
                    self.is_busy = false;
                }
            }
        }

        // --- Top Panel: Branding & Global Status ---
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("✨ spectro-rs");
                ui.separator();
                ui.label(format!("Device: {}", self.device_info));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.is_busy {
                        ui.spinner();
                    }
                    ui.label(&self.status_msg);
                });
            });
        });

        // --- Side Panel: Controls & Color Info ---
        egui::SidePanel::left("control_panel")
            .resizable(true)
            .default_width(280.0)
            .show(ctx, |ui| {
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    if ui.button("🚀 New Measurement").clicked() && !self.is_busy {
                        self.is_busy = true;
                        self.cmd_tx
                            .send(DeviceCommand::Measure(MeasurementMode::Reflective))
                            .ok();
                    }
                    if ui.button("🎯 Calibrate Sensor").clicked() && !self.is_busy {
                        self.is_busy = true;
                        self.cmd_tx.send(DeviceCommand::Calibrate).ok();
                    }
                });

                ui.separator();
                ui.heading("Color Analysis");

                if let Some(data) = &self.last_result {
                    let xyz = data.to_xyz();
                    let lab = xyz.to_lab(spectro_core::colorimetry::illuminant::D65_2);

                    // Color Swatch
                    let (r, g, b) = (100, 100, 100); // TODO: Implement Lab to sRGB conversion
                    let rect = ui
                        .allocate_exact_size(
                            egui::vec2(ui.available_width(), 100.0),
                            egui::Sense::hover(),
                        )
                        .0;
                    ui.painter()
                        .rect_filled(rect, 5.0, egui::Color32::from_rgb(r, g, b));

                    ui.add_space(10.0);
                    egui::Grid::new("color_grid")
                        .num_columns(2)
                        .spacing([40.0, 10.0])
                        .show(ui, |ui| {
                            ui.label("L* value:");
                            ui.label(format!("{:.2}", lab.l));
                            ui.end_row();
                            ui.label("a* value:");
                            ui.label(format!("{:.2}", lab.a));
                            ui.end_row();
                            ui.label("b* value:");
                            ui.label(format!("{:.2}", lab.b));
                            ui.end_row();
                            ui.separator();
                            ui.separator();
                            ui.end_row();
                            ui.label("CCT:");
                            ui.label(format!("{:.0} K", xyz.to_cct()));
                            ui.end_row();
                        });
                } else {
                    ui.label("No measurement data yet.");
                }
            });

        // --- Central Panel: Spectral Plot ---
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Spectral Power Distribution");

            let plot = Plot::new("spectral_plot")
                .view_aspect(2.0)
                .include_y(0.0)
                .include_y(1.2)
                .y_axis_label("Relative Intensity")
                .x_axis_label("Wavelength (nm)");

            plot.show(ui, |plot_ui| {
                if let Some(data) = &self.last_result {
                    let points: PlotPoints = data
                        .wavelengths
                        .iter()
                        .zip(data.values.iter())
                        .map(|(w, v)| [*w as f64, *v as f64])
                        .collect();

                    let line = Line::new(points)
                        .color(egui::Color32::from_rgb(0, 255, 128))
                        .width(2.0);
                    plot_ui.line(line);
                }
                // Reference lines
                plot_ui.hline(HLine::new(1.0).color(egui::Color32::DARK_GRAY));
            });
        });

        // Always request repaint for smooth animations
        ctx.request_repaint();
    }
}
