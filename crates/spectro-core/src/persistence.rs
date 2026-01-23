//! Persistence layer for spectrometer calibration data.
//!
//! This module handles saving and loading calibration factors to the local filesystem,
//! allowing devices to skip repeating calibration steps between sessions.

use crate::{Result, SpectroError};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Physics model parameters for sensor correction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicsConfig {
    pub y_bias: f64,
    pub y_sat: f64,
    pub t_dead: f64,
}

/// Calibration data for a specific device.
#[derive(Debug, Serialize, Deserialize)]
pub struct CalibrationData {
    /// The serial number of the device.
    pub serial: String,
    /// Timestamp of when the calibration was performed (UNIX timestamp).
    pub timestamp: u64,
    /// Dark reference readings.
    pub dark_ref: Vec<u16>,
    /// White calibration scaling factors.
    pub white_cal_factors: Vec<f32>,
    /// Optional physics-based sensor geometry.
    #[serde(default)]
    pub physics_config: Option<PhysicsConfig>,
}

/// Gets the directory where calibration data should be stored.
fn get_config_dir() -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "tinnci", "spectro-rs")
        .ok_or_else(|| SpectroError::Device("Could not determine config directory".into()))?;

    let path = dirs.config_dir();
    if !path.exists() {
        fs::create_dir_all(path)
            .map_err(|e| SpectroError::Device(format!("Failed to create config dir: {}", e)))?;
    }

    Ok(path.to_path_buf())
}

/// Gets the path to the calibration file for a specific device serial.
fn get_cal_path(serial: &str) -> Result<PathBuf> {
    let mut path = get_config_dir()?;
    path.push(format!("cal_{}.json", serial));
    Ok(path)
}

/// Saves calibration data for a device.
pub fn save_calibration(
    serial: &str,
    dark_ref: &[u16],
    factors: &[f32],
    physics: Option<PhysicsConfig>,
) -> Result<()> {
    let data = CalibrationData {
        serial: serial.to_string(),
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        dark_ref: dark_ref.to_vec(),
        white_cal_factors: factors.to_vec(),
        physics_config: physics,
    };

    let path = get_cal_path(serial)?;
    let json = serde_json::to_string_pretty(&data)
        .map_err(|e| SpectroError::Device(format!("Serialization error: {}", e)))?;

    fs::write(path, json)
        .map_err(|e| SpectroError::Device(format!("Failed to write calibration file: {}", e)))?;

    Ok(())
}

/// Loads calibration data for a device if it exists.
pub fn load_calibration(serial: &str) -> Result<Option<CalibrationData>> {
    let path = get_cal_path(serial)?;
    if !path.exists() {
        return Ok(None);
    }

    let json = fs::read_to_string(path)
        .map_err(|e| SpectroError::Device(format!("Failed to read calibration file: {}", e)))?;

    let data: CalibrationData = serde_json::from_str(&json)
        .map_err(|e| SpectroError::Device(format!("Deserialization error: {}", e)))?;

    Ok(Some(data))
}
