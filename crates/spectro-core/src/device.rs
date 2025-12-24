//! High-level spectrometer device abstraction.
//!
//! This module defines the [`Spectrometer`] trait, which provides a unified
//! interface for all supported spectrometer devices, regardless of their
//! underlying hardware or communication protocol.

use crate::spectrum::SpectralData;
use crate::{MeasurementMode, Result};

/// Information about a spectrometer device.
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// Human-readable device model name (e.g., "ColorMunki", "i1Display Pro").
    pub model: String,
    /// Device serial number.
    pub serial: String,
    /// Firmware version string.
    pub firmware: String,
}

/// The current status of a spectrometer device.
#[derive(Debug, Clone)]
pub struct DeviceStatus {
    /// The current physical position/mode of the device dial.
    pub position: DevicePosition,
    /// Whether a button is currently pressed.
    pub button_pressed: bool,
    /// Whether the device is calibrated and ready for measurement.
    pub is_calibrated: bool,
}

/// Physical position/mode selector on the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DevicePosition {
    /// Projector/display measurement position.
    Projector,
    /// Surface/reflective measurement position.
    Surface,
    /// Calibration tile position.
    Calibration,
    /// Ambient light measurement position (with diffuser).
    Ambient,
    /// Unknown or unsupported position.
    Unknown(u8),
}

impl DevicePosition {
    /// Returns a human-readable name for this position.
    pub fn name(&self) -> &'static str {
        match self {
            DevicePosition::Projector => "Projector",
            DevicePosition::Surface => "Surface",
            DevicePosition::Calibration => "Calibration",
            DevicePosition::Ambient => "Ambient",
            DevicePosition::Unknown(_) => "Unknown",
        }
    }
}

/// A unified interface for spectrometer devices.
///
/// This trait abstracts the differences between various spectrometer models
/// (ColorMunki, i1Display Pro, Spyder, etc.), allowing application code to
/// work with any supported device through a common API.
///
/// # Example
///
/// ```ignore
/// use spectro_rs::{discover, MeasurementMode, Spectrometer};
///
/// let mut device = discover()?;
/// println!("Found: {}", device.info()?.model);
///
/// device.calibrate()?;
/// let spectrum = device.measure(MeasurementMode::Emissive)?;
/// println!("Luminance: {:.2} cd/m²", spectrum.to_xyz().y);
/// ```
pub trait Spectrometer {
    /// Returns information about the connected device.
    fn info(&self) -> Result<DeviceInfo>;

    /// Returns the current status of the device.
    fn status(&self) -> Result<DeviceStatus>;

    /// Performs device calibration.
    ///
    /// For reflective measurements, this typically involves measuring a white
    /// reference tile. For emissive/ambient modes, a dark calibration may be
    /// performed.
    ///
    /// # Errors
    ///
    /// Returns an error if the device is not in the correct physical position
    /// for calibration, or if the calibration measurement fails.
    fn calibrate(&mut self) -> Result<()>;

    /// Performs a single-point measurement in the specified mode.
    ///
    /// # Arguments
    ///
    /// * `mode` - The type of measurement to perform.
    ///
    /// # Returns
    ///
    /// The measured spectral data, which can be converted to various color
    /// spaces (XYZ, Lab, etc.) using the methods on [`SpectralData`].
    ///
    /// # Errors
    ///
    /// Returns an error if the device is not calibrated (for modes that require
    /// calibration), or if the measurement fails.
    fn measure(&mut self, mode: MeasurementMode) -> Result<SpectralData>;

    /// Returns the supported measurement modes for this device.
    fn supported_modes(&self) -> Vec<MeasurementMode>;

    /// Returns whether the device is currently calibrated for the given mode.
    fn is_calibrated(&self, mode: MeasurementMode) -> bool;
}

/// A boxed spectrometer for dynamic dispatch.
///
/// This type alias makes it convenient to store different spectrometer
/// implementations in the same collection or return them from factory functions.
pub type BoxedSpectrometer = Box<dyn Spectrometer + Send>;
