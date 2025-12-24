//! # spectro-rs
//!
//! A high-performance Rust driver for X-Rite ColorMunki spectrometers.
//!
//! This crate provides a safe, ergonomic interface for interacting with
//! ColorMunki (Original and Design) devices, supporting reflective, emissive,
//! and ambient measurement modes.
//!
//! ## Quick Start
//!
//! ```ignore
//! use spectro_rs::{discover, MeasurementMode};
//!
//! fn main() -> spectro_rs::Result<()> {
//!     // Find and connect to a device
//!     let mut device = discover()?;
//!     println!("Found: {:?}", device.info()?);
//!
//!     // Calibrate for reflective measurements
//!     device.calibrate()?;
//!
//!     // Measure and get spectral data
//!     let spectrum = device.measure(MeasurementMode::Reflective)?;
//!     let xyz = spectrum.to_xyz();
//!     println!("CIE XYZ: X={:.2}, Y={:.2}, Z={:.2}", xyz.x, xyz.y, xyz.z);
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! The crate is organized into several layers:
//!
//! - **Transport Layer** ([`transport`]): Abstracts low-level communication
//!   (USB, Bluetooth, etc.). See [`transport::Transport`] trait.
//!
//! - **Device Layer** ([`device`]): Defines the unified [`device::Spectrometer`]
//!   trait that all device implementations must follow.
//!
//! - **Device Implementations**: Concrete drivers like [`munki::Munki`] that
//!   implement the [`device::Spectrometer`] trait.
//!
//! - **Colorimetry** ([`colorimetry`], [`spectrum`]): Color science utilities
//!   for converting spectral data to various color spaces.

use rusb::{Context, UsbContext};
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

/// The error type for spectrometer operations.
#[derive(Error, Debug)]
pub enum SpectroError {
    /// USB communication error.
    #[error("USB Communication Error: {0}")]
    Usb(#[from] rusb::Error),

    /// Calibration-related error.
    #[error("Calibration Error: {0}")]
    Calibration(String),

    /// General device error.
    #[error("Device Error: {0}")]
    Device(String),

    /// Measurement mode mismatch.
    #[error("Mode Mismatch: {0}")]
    Mode(String),
}

/// A specialized [`Result`] type for spectrometer operations.
pub type Result<T> = std::result::Result<T, SpectroError>;

// ============================================================================
// Constants
// ============================================================================

/// Standard wavelength bands (380nm - 730nm in 10nm steps).
pub const WAVELENGTHS: [f32; 36] = [
    380.0, 390.0, 400.0, 410.0, 420.0, 430.0, 440.0, 450.0, 460.0, 470.0, 480.0, 490.0, 500.0,
    510.0, 520.0, 530.0, 540.0, 550.0, 560.0, 570.0, 580.0, 590.0, 600.0, 610.0, 620.0, 630.0,
    640.0, 650.0, 660.0, 670.0, 680.0, 690.0, 700.0, 710.0, 720.0, 730.0,
];

// ============================================================================
// Public Modules
// ============================================================================

pub mod colorimetry;
pub mod device;
pub mod i18n;
pub mod munki;
pub mod persistence;
pub mod spectrum;
pub mod transport;

// ============================================================================
// Re-exports for convenient API
// ============================================================================

pub use device::{BoxedSpectrometer, DeviceInfo, DevicePosition, DeviceStatus, Spectrometer};
pub use spectrum::SpectralData;
pub use transport::{Transport, UsbTransport};

// ============================================================================
// Types
// ============================================================================

/// Specifies the type of measurement to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementMode {
    /// Reflective measurement (paper, prints, materials).
    /// Requires prior calibration with white tile.
    Reflective,

    /// Emissive measurement (displays, monitors).
    /// Uses internal emissive matrix; no calibration required.
    Emissive,

    /// Ambient light measurement.
    /// Requires the diffuser attachment to be in place.
    Ambient,
}

// ============================================================================
// Discovery API
// ============================================================================

/// ColorMunki USB Vendor IDs.
const MUNKI_VIDS: [u16; 2] = [0x0765, 0x0971];
/// ColorMunki USB Product ID.
const MUNKI_PID: u16 = 0x2007;

/// Discovers and connects to the first available spectrometer.
///
/// This function scans USB devices for supported spectrometers and returns
/// a boxed [`Spectrometer`] trait object.
///
/// # Example
///
/// ```ignore
/// use spectro_rs::{discover, MeasurementMode};
///
/// let mut device = discover()?;
/// let spectrum = device.measure(MeasurementMode::Emissive)?;
/// ```
///
/// # Errors
///
/// Returns an error if no supported device is found, or if the device
/// cannot be opened/initialized.
pub fn discover() -> Result<BoxedSpectrometer> {
    let context = Context::new()?;
    discover_with_context(&context)
}

/// Discovers a spectrometer using a provided USB context.
///
/// This is useful if you need more control over USB enumeration or want
/// to reuse an existing context.
pub fn discover_with_context<T: UsbContext + 'static>(
    context: &T,
) -> Result<Box<dyn Spectrometer + Send>> {
    let devices = context.devices()?;

    for device in devices.iter() {
        let desc = device.device_descriptor()?;
        let vid = desc.vendor_id();
        let pid = desc.product_id();

        if MUNKI_VIDS.contains(&vid) && pid == MUNKI_PID {
            let handle = device.open()?;
            handle.claim_interface(0)?;

            let transport = transport::UsbTransport::new(handle);
            let munki = munki::Munki::new(transport)?;

            return Ok(Box::new(munki));
        }
    }

    Err(SpectroError::Device(
        "No supported spectrometer found. Ensure device is connected and drivers are installed."
            .into(),
    ))
}

/// Lists all detected spectrometer devices without connecting.
///
/// Returns a vector of (vendor_id, product_id, model_name) tuples.
pub fn list_devices() -> Result<Vec<(u16, u16, &'static str)>> {
    let context = Context::new()?;
    let devices = context.devices()?;
    let mut found = Vec::new();

    for device in devices.iter() {
        if let Ok(desc) = device.device_descriptor() {
            let vid = desc.vendor_id();
            let pid = desc.product_id();

            if MUNKI_VIDS.contains(&vid) && pid == MUNKI_PID {
                found.push((vid, pid, "ColorMunki"));
            }
            // Future: Add detection for i1Display Pro, Spyder, etc.
        }
    }

    Ok(found)
}
