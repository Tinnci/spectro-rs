//! ColorMunki spectrometer driver.
//!
//! This module provides the [`Munki`] struct, which implements the
//! [`Spectrometer`](crate::device::Spectrometer) trait for X-Rite ColorMunki
//! devices (Original and Design models).

use crate::device::{DeviceInfo, DevicePosition, DeviceStatus, Spectrometer};
use crate::spectrum::SpectralData;
use crate::transport::Transport;
use crate::{MeasurementMode, Result};
use std::convert::TryInto;
use std::time::Duration;

// USB Commands
const CMD_GET_VERSION: u8 = 0x85;
const CMD_GET_FIRMWARE: u8 = 0x86;
const CMD_GET_STATUS: u8 = 0x87;
const CMD_TRIGGER_MEASURE: u8 = 0x80;
const CMD_SET_EEPROM_ADDR: u8 = 0x81;

// Measurement mode flags
const MMF_LAMP: u8 = 0x01;
const MMF_HIGHGAIN: u8 = 0x04;

// Interrupt endpoint for data reads
const EP_DATA_IN: u8 = 0x81;

/// Firmware information from the ColorMunki device.
#[derive(Debug, Clone)]
pub struct MunkiFirmwareInfo {
    pub fw_rev_major: u8,
    pub fw_rev_minor: u8,
    pub tick_duration: u32,
    pub min_int_count: u32,
    pub num_eeprom_blocks: u32,
    pub eeprom_block_size: u32,
}

/// Internal configuration data parsed from device EEPROM.
#[derive(Debug, Clone)]
pub struct MunkiConfig {
    pub cal_version: u16,
    pub serial_number: String,
    pub rmtx_index: Vec<u32>,
    pub rmtx_coef: Vec<f32>,
    pub emtx_index: Vec<u32>,
    pub emtx_coef: Vec<f32>,
    pub lin_normal: Vec<f32>,
    pub lin_high: Vec<f32>,
    pub white_ref: Vec<f32>,
    pub amb_coef: Vec<f32>,
}

/// ColorMunki spectrometer driver.
///
/// This struct implements the [`Spectrometer`] trait for ColorMunki devices.
/// It uses a generic [`Transport`] for communication, allowing it to work
/// with USB, mock transports for testing, or future transport implementations.
///
/// # Example
///
/// ```ignore
/// use spectro_rs::transport::UsbTransport;
/// use spectro_rs::munki::Munki;
///
/// let context = rusb::Context::new()?;
/// // ... find and open device ...
/// let transport = UsbTransport::new(handle);
/// let mut munki = Munki::new(transport)?;
///
/// munki.calibrate()?;
/// let spectrum = munki.measure(MeasurementMode::Reflective)?;
/// ```
pub struct Munki<T: Transport> {
    transport: T,
    config: MunkiConfig,
    firmware: MunkiFirmwareInfo,
    dark_ref: Option<Vec<u16>>,
    white_cal_factors: Option<Vec<f32>>,
}

impl<T: Transport> Munki<T> {
    /// Creates a new Munki instance from a transport.
    ///
    /// This initializes the device by reading firmware info and EEPROM configuration.
    ///
    /// # Arguments
    /// * `transport` - The transport to use for communication.
    ///
    /// # Errors
    /// Returns an error if the device cannot be initialized or EEPROM is invalid.
    pub fn new(transport: T) -> Result<Self> {
        let firmware = Self::read_firmware_info(&transport)?;
        let config = Self::read_and_parse_eeprom(&transport)?;

        // Try to load existing calibration data for this device
        let mut dark_ref = None;
        let mut white_cal_factors = None;

        if let Ok(Some(cal)) = crate::persistence::load_calibration(&config.serial_number) {
            // Basic validation: ensure the lengths match what we expect
            if cal.dark_ref.len() == 137 && cal.white_cal_factors.len() == 36 {
                println!(
                    "Loaded calibration data for device {}",
                    config.serial_number
                );
                dark_ref = Some(cal.dark_ref);
                white_cal_factors = Some(cal.white_cal_factors);
            }
        }

        Ok(Self {
            transport,
            config,
            firmware,
            dark_ref,
            white_cal_factors,
        })
    }

    /// Returns a reference to the underlying transport.
    pub fn transport(&self) -> &T {
        &self.transport
    }

    /// Returns a reference to the device configuration.
    pub fn config(&self) -> &MunkiConfig {
        &self.config
    }

    /// Returns a reference to the firmware information.
    pub fn firmware(&self) -> &MunkiFirmwareInfo {
        &self.firmware
    }

    // ========================================================================
    // Low-level device communication
    // ========================================================================

    fn read_firmware_info(transport: &T) -> Result<MunkiFirmwareInfo> {
        let mut buf = [0u8; 24];
        transport.control_read(CMD_GET_FIRMWARE, 0, 0, &mut buf, Duration::from_secs(2))?;

        Ok(MunkiFirmwareInfo {
            fw_rev_major: u32::from_le_bytes(buf[0..4].try_into().unwrap()) as u8,
            fw_rev_minor: u32::from_le_bytes(buf[4..8].try_into().unwrap()) as u8,
            tick_duration: u32::from_le_bytes(buf[8..12].try_into().unwrap()),
            min_int_count: u32::from_le_bytes(buf[12..16].try_into().unwrap()),
            num_eeprom_blocks: u32::from_le_bytes(buf[16..20].try_into().unwrap()),
            eeprom_block_size: u32::from_le_bytes(buf[20..24].try_into().unwrap()),
        })
    }

    fn read_eeprom(transport: &T, addr: u32, size: u32) -> Result<Vec<u8>> {
        let mut params = [0u8; 8];
        params[0..4].copy_from_slice(&addr.to_le_bytes());
        params[4..8].copy_from_slice(&size.to_le_bytes());

        transport.control_write(CMD_SET_EEPROM_ADDR, 0, 0, &params, Duration::from_secs(2))?;

        let mut buf = vec![0u8; size as usize];
        transport.interrupt_read(EP_DATA_IN, &mut buf, Duration::from_secs(5))?;

        Ok(buf)
    }

    fn read_and_parse_eeprom(transport: &T) -> Result<MunkiConfig> {
        // Read calibration data size
        let size_buf = Self::read_eeprom(transport, 4, 4)?;
        let size = u32::from_le_bytes(size_buf[0..4].try_into().unwrap());

        // Read full calibration data
        let data = Self::read_eeprom(transport, 0, size)?;
        Self::parse_eeprom(&data)
    }

    fn parse_eeprom(data: &[u8]) -> Result<MunkiConfig> {
        if data.len() < 8169 {
            return Err(crate::SpectroError::Calibration(format!(
                "EEPROM data too short: {} < 8169",
                data.len()
            )));
        }

        // Verify checksum
        let stored_checksum = u32::from_le_bytes(data[8..12].try_into().unwrap());
        let mut sum: u32 = 0;
        let mut i = 0;
        while i < data.len() {
            if i == 8 {
                i += 4;
                continue;
            }
            if i + 4 <= data.len() {
                sum = sum.wrapping_add(u32::from_le_bytes(data[i..i + 4].try_into().unwrap()));
                i += 4;
            } else {
                let mut last_bytes = [0u8; 4];
                let rem = data.len() - i;
                last_bytes[..rem].copy_from_slice(&data[i..]);
                sum = sum.wrapping_add(u32::from_le_bytes(last_bytes));
                break;
            }
        }

        if sum != stored_checksum {
            return Err(crate::SpectroError::Calibration(format!(
                "Checksum mismatch: {:08X} vs {:08X}",
                sum, stored_checksum
            )));
        }

        let cal_version = u16::from_le_bytes(data[0..2].try_into().unwrap());
        let serial_number = String::from_utf8_lossy(&data[24..40])
            .trim_matches('\0')
            .to_string();

        // Parse reflective matrix
        let mut rmtx_index = Vec::with_capacity(36);
        for i in 0..36 {
            rmtx_index.push(u32::from_le_bytes(
                data[40 + i * 4..40 + i * 4 + 4].try_into().unwrap(),
            ));
        }

        let mut rmtx_coef = Vec::with_capacity(36 * 16);
        for i in 0..(36 * 16) {
            rmtx_coef.push(f32::from_bits(u32::from_le_bytes(
                data[184 + i * 4..184 + i * 4 + 4].try_into().unwrap(),
            )));
        }

        // Parse emissive matrix
        let mut emtx_index = Vec::with_capacity(36);
        for i in 0..36 {
            emtx_index.push(u32::from_le_bytes(
                data[2488 + i * 4..2488 + i * 4 + 4].try_into().unwrap(),
            ));
        }

        let mut emtx_coef = Vec::with_capacity(36 * 16);
        for i in 0..(36 * 16) {
            emtx_coef.push(f32::from_bits(u32::from_le_bytes(
                data[2632 + i * 4..2632 + i * 4 + 4].try_into().unwrap(),
            )));
        }

        // Parse linearization polynomials (in reverse order)
        let mut lin_normal = Vec::with_capacity(4);
        for i in (0..4).rev() {
            lin_normal.push(f32::from_bits(u32::from_le_bytes(
                data[4936 + i * 4..4936 + i * 4 + 4].try_into().unwrap(),
            )));
        }

        let mut lin_high = Vec::with_capacity(4);
        for i in (0..4).rev() {
            lin_high.push(f32::from_bits(u32::from_le_bytes(
                data[4952 + i * 4..4952 + i * 4 + 4].try_into().unwrap(),
            )));
        }

        // Parse reference data
        let mut white_ref = Vec::with_capacity(36);
        for i in 0..36 {
            white_ref.push(f32::from_bits(u32::from_le_bytes(
                data[4968 + i * 4..4968 + i * 4 + 4].try_into().unwrap(),
            )));
        }

        let mut amb_coef = Vec::with_capacity(36);
        for i in 0..36 {
            amb_coef.push(f32::from_bits(u32::from_le_bytes(
                data[5256 + i * 4..5256 + i * 4 + 4].try_into().unwrap(),
            )));
        }

        Ok(MunkiConfig {
            cal_version,
            serial_number,
            rmtx_index,
            rmtx_coef,
            emtx_index,
            emtx_coef,
            lin_normal,
            lin_high,
            white_ref,
            amb_coef,
        })
    }

    fn get_version_string(&self) -> Result<String> {
        let mut buf = [0u8; 100];
        let len =
            self.transport
                .control_read(CMD_GET_VERSION, 0, 0, &mut buf, Duration::from_secs(2))?;
        let s = String::from_utf8_lossy(&buf[..len]);
        Ok(s.trim_matches(char::from(0)).to_string())
    }

    fn get_raw_status(&self) -> Result<(u8, u8)> {
        let mut buf = [0u8; 2];
        self.transport
            .control_read(CMD_GET_STATUS, 0, 0, &mut buf, Duration::from_secs(2))?;
        Ok((buf[0], buf[1]))
    }

    fn trigger_measure(&self, int_clocks: u32, num_meas: u32, mode_flags: u8) -> Result<()> {
        let mut pbuf = [0u8; 12];
        pbuf[0] = if (mode_flags & MMF_LAMP) != 0 { 1 } else { 0 };
        pbuf[1] = 0; // Scan mode disabled
        pbuf[2] = if (mode_flags & MMF_HIGHGAIN) != 0 {
            1
        } else {
            0
        };
        pbuf[3] = 0; // hold_temp_duty
        pbuf[4..8].copy_from_slice(&int_clocks.to_le_bytes());
        pbuf[8..12].copy_from_slice(&num_meas.to_le_bytes());

        self.transport
            .control_write(CMD_TRIGGER_MEASURE, 0, 0, &pbuf, Duration::from_secs(2))?;
        Ok(())
    }

    fn read_measurement(&self, num_meas: u32) -> Result<Vec<Vec<u16>>> {
        const NSEN: usize = 137;
        let bytes_per_read = NSEN * 2;
        let total_bytes = bytes_per_read * num_meas as usize;
        let mut buf = vec![0u8; total_bytes];
        let mut xferred = 0;
        let timeout = Duration::from_secs(5);

        while xferred < total_bytes {
            let n = self
                .transport
                .interrupt_read(EP_DATA_IN, &mut buf[xferred..], timeout)?;
            if n == 0 {
                break;
            }
            xferred += n;
        }

        if xferred % bytes_per_read != 0 {
            return Err(crate::SpectroError::Device("Short read".into()));
        }

        let mut readings = Vec::new();
        for i in 0..(xferred / bytes_per_read) {
            let start = i * bytes_per_read;
            let mut reading = Vec::with_capacity(NSEN);
            for j in 0..NSEN {
                reading.push(u16::from_le_bytes(
                    buf[start + j * 2..start + j * 2 + 2].try_into().unwrap(),
                ));
            }
            readings.push(reading);
        }
        Ok(readings)
    }

    fn measure_spot(&self, lamp: bool, high_gain: bool) -> Result<Vec<u16>> {
        let tick_sec = self.firmware.tick_duration as f64 * 1e-6;
        let int_time_sec =
            (self.firmware.min_int_count * self.firmware.tick_duration) as f64 * 1e-6;
        let int_clocks = (int_time_sec / tick_sec).round() as u32;

        let mut flags = 0;
        if lamp {
            flags |= MMF_LAMP;
        }
        if high_gain {
            flags |= MMF_HIGHGAIN;
        }

        self.trigger_measure(int_clocks, 1, flags)?;
        std::thread::sleep(Duration::from_millis((int_time_sec * 1000.0) as u64 + 200));

        let readings = self.read_measurement(1)?;
        readings
            .into_iter()
            .next()
            .ok_or(crate::SpectroError::Device("No data".into()))
    }

    fn process_spectrum(
        &self,
        raw_137: &[u16],
        high_gain: bool,
        mode: MeasurementMode,
    ) -> Result<SpectralData> {
        let int_time_sec =
            (self.firmware.min_int_count * self.firmware.tick_duration) as f64 * 1e-6;
        let offset = 6;
        let mut linearized = Vec::with_capacity(128);
        let polys = if high_gain {
            &self.config.lin_high
        } else {
            &self.config.lin_normal
        };
        let scale = 1.0 / int_time_sec;

        for i in 0..128 {
            let mut val = raw_137[offset + i] as f64;
            if let Some(dark) = &self.dark_ref {
                val -= dark[offset + i] as f64;
            }

            let mut lval = polys[3] as f64;
            lval = lval * val + polys[2] as f64;
            lval = lval * val + polys[1] as f64;
            lval = lval * val + polys[0] as f64;
            linearized.push((lval * scale) as f32);
        }

        let (mtx_index, mtx_coef) = if mode == MeasurementMode::Emissive {
            (&self.config.emtx_index, &self.config.emtx_coef)
        } else {
            (&self.config.rmtx_index, &self.config.rmtx_coef)
        };

        let mut values = Vec::with_capacity(36);
        for w in 0..36 {
            let idx = mtx_index[w] as usize;
            let mut sum = 0.0f32;
            for k in 0..16 {
                if idx + k < linearized.len() {
                    sum += mtx_coef[w * 16 + k] * linearized[idx + k];
                }
            }

            match mode {
                MeasurementMode::Reflective => {
                    if let Some(factors) = &self.white_cal_factors {
                        sum *= factors[w];
                    }
                }
                MeasurementMode::Ambient => {
                    sum *= self.config.amb_coef[w];
                }
                MeasurementMode::Emissive => {}
            }

            values.push(sum);
        }

        Ok(SpectralData::new(values))
    }

    fn perform_calibration(&mut self) -> Result<()> {
        let (pos, _) = self.get_raw_status()?;
        if pos != 2 {
            return Err(crate::SpectroError::Device(
                "Not in Calibration position. Please turn dial to white tile position.".into(),
            ));
        }

        // Dark frame calibration (lamp off)
        let raw_dark = self.measure_spot(false, false)?;
        self.dark_ref = Some(raw_dark);

        // White tile calibration (lamp on)
        let raw_white = self.measure_spot(true, false)?;

        // Process without white calibration factors
        let old_factors = self.white_cal_factors.take();
        let spec = self.process_spectrum(&raw_white, false, MeasurementMode::Reflective)?;
        self.white_cal_factors = old_factors;

        // Compute calibration factors
        let mut factors = Vec::with_capacity(36);
        for i in 0..36 {
            let measured = spec.values[i];
            let reference = self.config.white_ref[i];
            factors.push(if measured > 1e-6 {
                reference / measured
            } else {
                1.0
            });
        }

        self.white_cal_factors = Some(factors);

        // Persist calibration data
        if let Some(dark) = &self.dark_ref {
            if let Some(white) = &self.white_cal_factors {
                let _ =
                    crate::persistence::save_calibration(&self.config.serial_number, dark, white);
            }
        }

        Ok(())
    }
}

// ============================================================================
// Spectrometer Trait Implementation
// ============================================================================

impl<T: Transport> Spectrometer for Munki<T> {
    fn info(&self) -> Result<DeviceInfo> {
        let version = self.get_version_string().unwrap_or_default();
        Ok(DeviceInfo {
            model: "ColorMunki".to_string(),
            serial: self.config.serial_number.clone(),
            firmware: format!(
                "{}.{} ({version})",
                self.firmware.fw_rev_major, self.firmware.fw_rev_minor
            ),
        })
    }

    fn status(&self) -> Result<DeviceStatus> {
        let (pos, btn) = self.get_raw_status()?;
        let position = match pos {
            0 => DevicePosition::Projector,
            1 => DevicePosition::Surface,
            2 => DevicePosition::Calibration,
            3 => DevicePosition::Ambient,
            _ => DevicePosition::Unknown(pos),
        };

        Ok(DeviceStatus {
            position,
            button_pressed: btn != 0,
            is_calibrated: self.white_cal_factors.is_some(),
        })
    }

    fn calibrate(&mut self) -> Result<()> {
        self.perform_calibration()
    }

    fn measure(&mut self, mode: MeasurementMode) -> Result<SpectralData> {
        // Validate mode requirements
        if mode == MeasurementMode::Reflective && self.white_cal_factors.is_none() {
            return Err(crate::SpectroError::Calibration(
                "Reflective mode requires calibration first".into(),
            ));
        }

        // Validate dial position for ambient mode
        if mode == MeasurementMode::Ambient {
            let (pos, _) = self.get_raw_status()?;
            if pos != 1 && pos != 3 {
                return Err(crate::SpectroError::Mode(
                    "Ambient mode requires dial in Ambient position".into(),
                ));
            }
        }

        let (lamp, high_gain) = match mode {
            MeasurementMode::Reflective => (true, false),
            MeasurementMode::Emissive => (false, true),
            MeasurementMode::Ambient => (false, false),
        };

        let raw = self.measure_spot(lamp, high_gain)?;
        self.process_spectrum(&raw, high_gain, mode)
    }

    fn supported_modes(&self) -> Vec<MeasurementMode> {
        vec![
            MeasurementMode::Reflective,
            MeasurementMode::Emissive,
            MeasurementMode::Ambient,
        ]
    }

    fn is_calibrated(&self, mode: MeasurementMode) -> bool {
        match mode {
            MeasurementMode::Reflective => self.white_cal_factors.is_some(),
            // Emissive and Ambient don't require prior calibration
            MeasurementMode::Emissive | MeasurementMode::Ambient => true,
        }
    }
}
