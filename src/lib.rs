use thiserror::Error;

#[derive(Error, Debug)]
pub enum SpectroError {
    #[error("USB Communication Error: {0}")]
    Usb(#[from] rusb::Error),
    #[error("Calibration Error: {0}")]
    Calibration(String),
    #[error("Device Error: {0}")]
    Device(String),
    #[error("Mode Mismatch: {0}")]
    Mode(String),
}

pub type Result<T> = std::result::Result<T, SpectroError>;

pub const WAVELENGTHS: [f32; 36] = [
    380.0, 390.0, 400.0, 410.0, 420.0, 430.0, 440.0, 450.0, 460.0, 470.0, 480.0, 490.0, 500.0,
    510.0, 520.0, 530.0, 540.0, 550.0, 560.0, 570.0, 580.0, 590.0, 600.0, 610.0, 620.0, 630.0,
    640.0, 650.0, 660.0, 670.0, 680.0, 690.0, 700.0, 710.0, 720.0, 730.0,
];

pub mod colorimetry;
pub mod i18n;
pub mod munki;
pub mod spectrum;

pub trait Spectrometer {
    fn get_serial(&self) -> String;
    fn measure(&mut self) -> Result<spectrum::SpectralData>;
    fn calibrate(&mut self) -> Result<()>;
}
