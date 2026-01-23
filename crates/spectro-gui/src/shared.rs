//! Shared types for communication between UI and device worker threads.

use spectro_rs::{DeviceInfo, MeasurementMode, tm30::TM30Metrics};

// ============================================================================
// Device Information Structures
// ============================================================================

/// Extended device information including EEPROM data for Expert mode.
#[derive(Debug, Clone, Default)]
pub struct ExtendedDeviceInfo {
    /// Basic device info (model, serial, firmware)
    pub basic: Option<DeviceInfo>,
    /// Calibration version from EEPROM
    pub cal_version: Option<u16>,
    /// White reference spectrum from EEPROM (36 values)
    pub white_ref: Option<Vec<f32>>,
    /// Emissive calibration coefficients (36 values)
    pub emis_coef: Option<Vec<f32>>,
    /// Ambient calibration coefficients (36 values)
    pub amb_coef: Option<Vec<f32>>,
    /// Linearization polynomial (normal gain)
    pub lin_normal: Option<Vec<f32>>,
    /// Linearization polynomial (high gain)
    pub lin_high: Option<Vec<f32>>,
}

/// Measurement history entry
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MeasurementEntry {
    pub timestamp: String,
    pub mode: MeasurementMode,
    pub result: spectro_rs::spectrum::MeasurementResult,
    pub tm30: Option<TM30Metrics>,
    pub delta_e: Option<f32>,
}

// ============================================================================
// Communication Protocols
// ============================================================================

/// Messages sent from the UI thread to the Device worker thread.
pub enum DeviceCommand {
    Connect,
    Calibrate,
    Measure(MeasurementMode),
    TestSensor,
    CharacterizeSensor,
}

/// Messages sent from the Device worker thread to the UI thread.
pub enum UIUpdate {
    Connected(ExtendedDeviceInfo),
    Status(String),
    Result(
        spectro_rs::spectrum::MeasurementResult,
        Option<Box<TM30Metrics>>,
    ),
    TestResult(String),
    CharacterizationResult(String),
    Error(String),
    Disconnected,
}
