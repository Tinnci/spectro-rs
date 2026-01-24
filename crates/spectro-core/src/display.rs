//! Display control module for calibration patch generation.
//!
//! This module handles the creation of system-level windows that bypass
//! operating system color management to display pure, raw RGB values.
//! This is critical for display calibration where the sensor must measure
//! the native response of the panel.

#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub use macos::NativeDisplay;

// Placeholder for other platforms
#[cfg(not(target_os = "macos"))]
pub struct NativeDisplay;

#[cfg(not(target_os = "macos"))]
impl NativeDisplay {
    pub fn new() -> Result<Self, String> {
        Err("Display control not implemented for this OS".to_string())
    }
    pub fn show_color(&self, _r: f32, _g: f32, _b: f32) {}
}

/// Interface for controlling the display output.
pub trait DisplayController {
    /// Create a new full-screen calibration window on the primary display.
    fn new() -> Result<Self, String>
    where
        Self: Sized;

    /// Display a pure color patch.
    ///
    /// Values should be in linear 0.0 - 1.0 range.
    /// The implementation guarantees bypassing ICC profiles/VCGT where possible.
    fn show_color(&self, r: f32, g: f32, b: f32);
}
