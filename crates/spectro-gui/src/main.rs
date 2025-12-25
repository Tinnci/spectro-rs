mod app;
mod tm30_gui;

use eframe::egui;
use spectro_rs::Result;

fn main() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "spectro-rs Suite",
        options,
        Box::new(|cc| Ok(Box::new(app::SpectroApp::new(cc)))),
    )
    .map_err(|e| spectro_rs::SpectroError::Device(format!("GUI runtime error: {}", e)))
}
