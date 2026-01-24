mod app;
mod backend;

mod calibration;
mod components;
mod exporters;
mod i18n;
mod inspector;
mod shared;
mod state;
mod theme;
mod tm30_gui;
mod views;

use eframe::egui;
use spectro_rs::Result;

fn main() -> Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 700.0])
            .with_min_inner_size([800.0, 500.0])
            .with_icon(load_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "spectro-rs Suite",
        options,
        Box::new(|cc| Ok(Box::new(app::SpectroApp::new(cc)))),
    )
    .map_err(|e| spectro_rs::SpectroError::Device(format!("GUI runtime error: {}", e)))
}

fn load_icon() -> egui::IconData {
    let (icon_rgba, icon_width, icon_height) = {
        let image = image::load_from_memory(include_bytes!("../assets/app_icon.png"))
            .expect("Failed to load icon")
            .into_rgba8();
        let (width, height) = image.dimensions();
        let rgba = image.into_raw();
        (rgba, width, height)
    };

    egui::IconData {
        rgba: icon_rgba,
        width: icon_width,
        height: icon_height,
    }
}
