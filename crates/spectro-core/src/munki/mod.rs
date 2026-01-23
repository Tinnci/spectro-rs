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

pub mod dsp;
pub mod eeprom;
pub mod exposure;
pub use eeprom::MunkiConfig;

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
    physics_config: Option<physics::SensorModel>,
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
        let mut physics_config = None;

        if let Ok(Some(cal)) = crate::persistence::load_calibration(&config.serial_number) {
            // Basic validation: ensure the lengths match what we expect
            if cal.dark_ref.len() == 137 && cal.white_cal_factors.len() == 36 {
                println!(
                    "Loaded calibration data for device {}",
                    config.serial_number
                );
                dark_ref = Some(cal.dark_ref);
                white_cal_factors = Some(cal.white_cal_factors);

                if let Some(phys) = cal.physics_config {
                    println!(
                        "Loaded Sensor Physics Model: bias={:.1}, dead={:.6}s",
                        phys.y_bias, phys.t_dead
                    );
                    physics_config = Some(physics::SensorModel {
                        y_bias: phys.y_bias,
                        y_sat: phys.y_sat,
                        t_dead: phys.t_dead,
                    });
                }
            }
        }

        Ok(Self {
            transport,
            config,
            firmware,
            dark_ref,
            white_cal_factors,
            physics_config,
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

    fn flush_input(&self) -> Result<()> {
        let mut buf = [0u8; 64];
        let timeout = Duration::from_millis(10);
        // Try to read until we get a timeout or empty read to clear the pipe
        loop {
            match self.transport.interrupt_read(EP_DATA_IN, &mut buf, timeout) {
                Ok(n) if n > 0 => continue,
                _ => break,
            }
        }
        Ok(())
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
        eeprom::EepromParser::parse(&data)
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

    fn measure_integration(
        &self,
        duration_sec: f64,
        lamp: bool,
        high_gain: bool,
    ) -> Result<Vec<u16>> {
        // Clear any stale data from previous measurements
        self.flush_input()?;

        let tick_sec = self.firmware.tick_duration as f64 * 1e-6;
        let int_clocks = (duration_sec / tick_sec).round() as u32;

        let mut flags = 0;
        if lamp {
            flags |= MMF_LAMP;
        }
        if high_gain {
            flags |= MMF_HIGHGAIN;
        }

        self.trigger_measure(int_clocks, 1, flags)?;
        // Wait for measurement to complete.
        // ArgyllCMS uses ~150ms safety margin; we use 200ms for extra robustness.
        std::thread::sleep(Duration::from_millis((duration_sec * 1000.0) as u64 + 200));

        let readings = self.read_measurement(1)?;
        readings
            .into_iter()
            .next()
            .ok_or(crate::SpectroError::Device("No data".into()))
    }

    fn measure_scanned_average(
        &self,
        duration_sec: f64,
        lamp: bool,
        high_gain: bool,
        count: usize,
    ) -> Result<Vec<u16>> {
        let mut sum_buf = vec![0.0f64; 137];

        println!(
            "DEBUG: Oversampling {} frames at {:.4}s...",
            count, duration_sec
        );

        for _ in 0..count {
            let raw = self.measure_integration(duration_sec, lamp, high_gain)?;
            if raw.len() != 137 {
                return Err(crate::SpectroError::Device(
                    "Invalid sensor data length".into(),
                ));
            }
            for (i, val) in raw.iter().enumerate() {
                sum_buf[i] += *val as f64;
            }
        }

        let avg: Vec<u16> = sum_buf
            .iter()
            .map(|x| (x / count as f64).round() as u16)
            .collect();
        Ok(avg)
    }

    fn measure_spot(
        &self,
        lamp: bool,
        high_gain: bool,
        force_time: Option<f64>,
    ) -> Result<(Vec<u16>, f64)> {
        const OVERSAMPLE_COUNT: usize = 8;
        let mut min_time =
            (self.firmware.min_int_count as f64 * self.firmware.tick_duration as f64) * 1e-6;

        // If we have a physics model, the true minimum usable time is the dead zone
        if let Some(model) = &self.physics_config {
            min_time = min_time.max(model.t_dead + 0.001); // Add 1ms safety margin
        }

        // If a specific time is requested, use it directly (bypass AE)
        if let Some(t) = force_time {
            let raw = self.measure_scanned_average(t, lamp, high_gain, OVERSAMPLE_COUNT)?;
            return Ok((raw, t));
        }

        // Force high dynamic range (Target = 10500) as requested to maximize signal in UV/IR bands.
        // Also force min_time to 0.015s to ensure edge bands get enough light.
        let target_exposure = 10500.0;
        min_time = min_time.max(0.015);
        let mut ae = exposure::AutoExposure::new(min_time, target_exposure, self.config.satlimit);
        let mut current_time = min_time;

        // Start with a quick measurement
        let mut raw = self.measure_integration(current_time, lamp, high_gain)?;

        let mut last_good_time = current_time;

        for i in 0..5 {
            let max_val = raw.iter().max().copied().unwrap_or(0);

            match ae.calculate_next(current_time, max_val, i) {
                exposure::ExposureAction::Success | exposure::ExposureAction::MaxRetriesReached => {
                    break;
                }
                exposure::ExposureAction::Undo => {
                    if i > 0 {
                        println!(
                            "Auto-Exposure: Saturation detected. Reverting to time={:.4}s",
                            last_good_time
                        );
                        current_time = last_good_time;
                        break;
                    } else {
                        break;
                    }
                }
                exposure::ExposureAction::Retry(new_time) => {
                    println!(
                        "Auto-Exposure: Peak={}, Time={:.4}s -> Adjusting to {:.4}s",
                        max_val, current_time, new_time
                    );

                    last_good_time = current_time;
                    current_time = new_time;
                    raw = self.measure_integration(current_time, lamp, high_gain)?;
                }
            }
        }

        // Perform final production measurement with oversampling
        let final_raw =
            self.measure_scanned_average(current_time, lamp, high_gain, OVERSAMPLE_COUNT)?;
        Ok((final_raw, current_time))
    }

    fn process_spectrum(
        &self,
        raw_137: &[u16],
        high_gain: bool,
        mode: MeasurementMode,
        int_time_sec: f64,
    ) -> Result<SpectralData> {
        dsp::SignalProcessor::process(
            &self.config,
            int_time_sec,
            raw_137,
            self.dark_ref.as_deref(),
            self.white_cal_factors.as_deref(),
            self.physics_config.as_ref(),
            high_gain,
            mode,
        )
    }

    fn perform_calibration(&mut self) -> Result<()> {
        let (pos, _) = self.get_raw_status()?;
        if pos != 2 {
            return Err(crate::SpectroError::Device(
                "Not in Calibration position. Please turn dial to white tile position.".into(),
            ));
        }

        // Dark frame calibration (lamp off)
        let min_time =
            (self.firmware.min_int_count as f64 * self.firmware.tick_duration as f64) * 1e-6;
        let (raw_dark, _) = self.measure_spot(false, false, Some(min_time))?;
        // Check dark current quality
        let dark_avg: f32 = raw_dark.iter().map(|&x| x as f32).sum::<f32>() / raw_dark.len() as f32;
        let dark_max = raw_dark.iter().max().unwrap_or(&0);
        println!("\n=== Calibration Diagnostics ===");
        println!(
            "Dark Current stats: Avg={:.1}, Max={} (Should be < 1000 typically)",
            dark_avg, dark_max
        );

        self.dark_ref = Some(raw_dark);

        // White tile calibration (lamp on)
        // Use Auto-Exposure for white tile to get best signal
        let (raw_white, white_time) = self.measure_spot(true, false, None)?;
        // Check signal strength
        let white_avg: f32 =
            raw_white.iter().map(|&x| x as f32).sum::<f32>() / raw_white.len() as f32;
        let white_max = raw_white.iter().max().unwrap_or(&0);
        println!(
            "White Signal stats: Avg={:.1}, Max={} (Should be > 10000 typically)",
            white_avg, white_max
        );
        if *white_max < 1000 {
            println!("WARNING: Signal too low! Is the shutter open?");
        }

        // Process without white calibration factors
        let old_factors = self.white_cal_factors.take();
        let spec =
            self.process_spectrum(&raw_white, false, MeasurementMode::Reflective, white_time)?;
        self.white_cal_factors = old_factors;

        // Compute calibration factors
        // 1. Calculation: Generate all factors in strict order (0..36)
        // This prevents any ordering issues caused by split loops for logging.
        let factors: Vec<f32> = spec
            .values
            .iter()
            .zip(self.config.white_ref.iter())
            .map(|(&measured, &reference)| {
                if measured > 1e-6 {
                    reference / measured
                } else {
                    1.0
                }
            })
            .collect();

        // 2. Logging: Diagnostic output from the computed data
        println!("\n=== White Calibration Data (Safe Mode) ===");

        let print_band = |idx: usize, label: &str| {
            if idx < factors.len() {
                println!(
                    "  [{:02}] {:<6} Measured: {:.4e}, Ref: {:.4e}, Factor: {:.4e}",
                    idx, label, spec.values[idx], self.config.white_ref[idx], factors[idx]
                );
            }
        };

        println!("--- Low Range (UV/Blue) ---");
        for i in 0..5 {
            print_band(i, "380-420");
        }

        println!("--- Mid Range (Green/Yellow) ---");
        for i in 15..20 {
            print_band(i, "530-570");
        }

        println!("--- High Range (Red) ---");
        for i in 31..36 {
            print_band(i, "690-730");
        }

        println!("===");

        self.white_cal_factors = Some(factors);

        // Persist calibration data
        // Persist calibration data
        if let (Some(dark), Some(white)) = (&self.dark_ref, &self.white_cal_factors) {
            let _ = crate::persistence::save_calibration(
                &self.config.serial_number,
                dark,
                white,
                self.physics_config.clone().map(|p| p.into()),
            );
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

        let (raw, time) = self.measure_spot(lamp, high_gain, None)?;
        let mut result = self.process_spectrum(&raw, high_gain, mode, time)?;
        result
            .metadata
            .insert("oversampling".to_string(), "8x (Hardware)".to_string());
        Ok(result)
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

    fn test_sensor(&mut self) -> Result<String> {
        self.flush_input()?;
        let (pos, _) = self.get_raw_status()?;
        if pos != 2 {
            return Err(crate::SpectroError::Device(
                "Diagnostic requires device to be in Calibration (White Tile) position.".into(),
            ));
        }

        let mut report = String::from("=== ColorMunki Sensor Diagnostic Report ===\n");
        use std::fmt::Write;

        writeln!(report, "--- Hardware Configuration (from EEPROM) ---").unwrap();
        writeln!(report, "Serial Number: {}", self.config.serial_number).unwrap();
        writeln!(report, "ADC Type:      {}", self.config.adctype).unwrap();
        writeln!(report, "Target Min:    {:.0}", self.config.minsval).unwrap();
        writeln!(report, "Target Opt:    {:.0}", self.config.optsval).unwrap();
        writeln!(report, "Saturation:    {:.0}", self.config.satlimit).unwrap();
        writeln!(
            report,
            "Lin Normal:    [{:.4e}, {:.4e}, {:.4e}, {:.4e}]",
            self.config.lin_normal[0],
            self.config.lin_normal[1],
            self.config.lin_normal[2],
            self.config.lin_normal[3]
        )
        .unwrap();
        writeln!(
            report,
            "Lin High:      [{:.4e}, {:.4e}, {:.4e}, {:.4e}]",
            self.config.lin_high[0],
            self.config.lin_high[1],
            self.config.lin_high[2],
            self.config.lin_high[3]
        )
        .unwrap();
        writeln!(report).unwrap();

        writeln!(report, "--- Linearity Test (Lamp ON) ---").unwrap();

        let times = [
            0.010, 0.020, 0.040, 0.080, 0.160, 0.320, 0.640, 1.28, 2.0, 4.0,
        ];
        let mut baseline_rate = 0.0;

        writeln!(
            report,
            "{:<10} | {:<10} | {:<10} | {:<10} | Status",
            "Time(s)", "Peak", "Rate(cnt/s)", "Linearity"
        )
        .unwrap();
        writeln!(
            report,
            "---------------------------------------------------------------------"
        )
        .unwrap();

        for (i, &t) in times.iter().enumerate() {
            // Measure with LAMP ON, Norm Gain
            match self.measure_integration(t, true, false) {
                Ok(raw) => {
                    let peak = raw.iter().max().copied().unwrap_or(0) as f64;
                    let rate = peak / t;

                    if i == 0 {
                        baseline_rate = rate; // Set baseline from fastest measurement (usually linear)
                    }

                    let linearity = rate / baseline_rate;
                    let linearity_pct = linearity * 100.0;

                    let status = if peak > 65000.0 {
                        "SATURATED"
                    } else if linearity < 0.90 {
                        "NON-LINEAR"
                    } else {
                        "OK"
                    };

                    writeln!(
                        report,
                        "{:<10.3} | {:<10.0} | {:<10.0} | {:<9.1}% | {}",
                        t, peak, rate, linearity_pct, status
                    )
                    .unwrap();

                    // If saturated, no need to go much further, maybe one more to show it's stuck
                    if peak > 65400.0 {
                        writeln!(report, "Saturation limit reached. Stopping sweep.").unwrap();
                        break;
                    }
                }
                Err(e) => {
                    writeln!(report, "Error at {}s: {}", t, e).unwrap();
                }
            }
        }

        // Dark Current Test
        writeln!(report, "\n=== Dark Current Test (Lamp OFF) ===").unwrap();
        // Use 1.0s integration for a good noise floor check
        let dark_time = 0.5;
        match self.measure_integration(dark_time, false, false) {
            Ok(raw) => {
                let avg = raw.iter().map(|&x| x as f64).sum::<f64>() / raw.len() as f64;
                let max = raw.iter().max().copied().unwrap_or(0);
                let std_dev = (raw.iter().map(|&x| (x as f64 - avg).powi(2)).sum::<f64>()
                    / raw.len() as f64)
                    .sqrt();

                writeln!(report, "Integration Time: {:.1}s", dark_time).unwrap();
                writeln!(
                    report,
                    "Average Level:    {:.1} (Typical < 1000 at Min Time)",
                    avg
                )
                .unwrap();
                writeln!(report, "Max Level:        {}", max).unwrap();
                writeln!(report, "Std Deviation:    {:.2}", std_dev).unwrap();

                // Normalized noise
                let norm_noise = std_dev / dark_time;
                writeln!(report, "Noise Rate:       {:.1} cnt/s", norm_noise).unwrap();
            }
            Err(e) => writeln!(report, "Dark measurement failed: {}", e).unwrap(),
        }

        writeln!(report, "=== End of Report ===").unwrap();
        println!("{}", report);

        Ok(report)
    }

    fn characterize_sensor(&mut self) -> Result<String> {
        self.flush_input()?;
        let (pos, _) = self.get_raw_status()?;
        if pos != 2 {
            return Err(crate::SpectroError::Device(
                "Characterization requires device to be in Calibration (White Tile) position."
                    .into(),
            ));
        }

        let mut csv =
            String::from("Time(s),Counts,Rate,Ideal,Deviation(%),Corrected_Rate,Error(%)\n");
        let _tick = self.firmware.tick_duration as f64 * 1e-6;

        // Scan from min_time up to ~40ms
        let mut t = 0.001; // Start at 1ms
        let step = 0.0005;
        let max_scan = 0.040;

        let mut points = Vec::new();

        // 1. Data Acquisition
        while t <= max_scan {
            if let Ok(raw) = self.measure_integration(t, true, false) {
                let peak = raw.iter().max().copied().unwrap_or(0) as f64;
                points.push((t, peak));

                if (peak > 65400.0 || (peak > 12200.0 && peak < 13000.0 && t > 0.020)) && t > 0.015
                {
                    break;
                }
            }
            t += step;
        }

        // 2. Physics Modeling
        use crate::munki::physics::SensorModel;
        // Provide hint from EEPROM or observation
        let sat_hint = self.config.satlimit.max(12200.0);
        let model = SensorModel::estimate_parameters(&points, sat_hint);

        // 3. Analysis & Reporting
        // Calculate true intensity k from the most linear region (e.g. 10ms - 15ms)
        let mut k_sum = 0.0;
        let mut k_count = 0;

        for (pt_t, pt_y) in &points {
            if *pt_t >= 0.010
                && *pt_t <= 0.015
                && let Ok(k) = model.solve_intensity(*pt_y, *pt_t)
            {
                k_sum += k;
                k_count += 1;
            }
        }
        let true_k = if k_count > 0 {
            k_sum / k_count as f64
        } else {
            0.0
        };

        // Generate CSV rows with correction data
        let mut first = true;
        let mut baseline_rate = 0.0;

        for (pt_t, pt_y) in &points {
            let rate = pt_y / pt_t;
            if first && *pt_t >= 0.008 {
                // Start baseline AFTER dead zone
                baseline_rate = rate;
                first = false;
            }

            let ideal = if baseline_rate > 0.0 {
                pt_t * baseline_rate
            } else {
                *pt_y
            };
            let dev = if ideal > 0.0 {
                (pt_y - ideal) / ideal * 100.0
            } else {
                0.0
            };

            // Apply Physics Correction
            let corrected_k = model.solve_intensity(*pt_y, *pt_t).unwrap_or(0.0);
            let phys_err = if true_k > 0.0 {
                (corrected_k - true_k) / true_k * 100.0
            } else {
                0.0
            };

            use std::fmt::Write;
            writeln!(
                csv,
                "{:.6},{:.1},{:.1},{:.1},{:.2},{:.1},{:.2}",
                pt_t, pt_y, rate, ideal, dev, corrected_k, phys_err
            )
            .unwrap();
        }

        use std::fmt::Write;
        writeln!(csv, "\n=== Physics Model Analysis ===").unwrap();
        writeln!(csv, "Dead Time (t_dead): {:.6} s", model.t_dead).unwrap();
        writeln!(csv, "Bias Level (y_bias): {:.1}", model.y_bias).unwrap();
        writeln!(csv, "Saturation (y_sat):  {:.1}", model.y_sat).unwrap();
        writeln!(csv, "True Intensity (k):  {:.1} (Target Constant)", true_k).unwrap();

        // Update memory state
        self.physics_config = Some(model.clone());

        // Update file state if we have a full calibration set
        if let (Some(dark), Some(white)) = (&self.dark_ref, &self.white_cal_factors) {
            let _ = crate::persistence::save_calibration(
                &self.config.serial_number,
                dark,
                white,
                Some(model.into()),
            );
            println!("Physics parameters persisted to profile.");
        }

        println!("{}", csv);
        Ok(csv)
    }
}
pub mod physics;
