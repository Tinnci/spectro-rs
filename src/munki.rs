use crate::Result;
use crate::spectrum::SpectralData;
use rusb::{DeviceHandle, Direction, Recipient, RequestType, UsbContext, request_type};
use std::convert::TryInto;
use std::time::Duration;

// Request Types
const REQ_TYPE_VENDOR_IN: u8 = 0xC0;
const REQ_TYPE_VENDOR_OUT: u8 = 0x40;

// Commands
const CMD_GET_VERSION: u8 = 0x85;
const CMD_GET_FIRMWARE: u8 = 0x86;
const CMD_GET_STATUS: u8 = 0x87;
const CMD_GET_CHIP_ID: u8 = 0x8A;
const CMD_TRIGGER_MEASURE: u8 = 0x80;
const CMD_SET_EEPROM_ADDR: u8 = 0x81;

pub const MMF_LAMP: u8 = 0x01;
pub const MMF_SCAN: u8 = 0x02;
pub const MMF_HIGHGAIN: u8 = 0x04;

#[derive(Debug, Clone)]
pub struct MunkiFirmwareInfo {
    pub fw_rev_major: u8,
    pub fw_rev_minor: u8,
    pub tick_duration: u32,
    pub min_int_count: u32,
    pub num_eeprom_blocks: u32,
    pub eeprom_block_size: u32,
}

#[derive(Debug, Clone)]
pub struct MunkiStatus {
    pub sensor_position: u8,
    pub button_state: u8,
}

impl MunkiStatus {
    pub fn position_name(&self) -> &'static str {
        match self.sensor_position {
            0 => "Projector",
            1 => "Surface",
            2 => "Calibration",
            3 => "Ambient",
            _ => "Unknown",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MunkiConfig {
    pub cal_version: u16,
    pub serial_number: String,
    pub _production_no: u32,
    pub rmtx_index: Vec<u32>,
    pub rmtx_coef: Vec<f32>,
    pub _emtx_index: Vec<u32>,
    pub _emtx_coef: Vec<f32>,
    pub lin_normal: Vec<f32>,
    pub lin_high: Vec<f32>,
    pub white_ref: Vec<f32>,
    pub _emis_coef: Vec<f32>,
    pub _amb_coef: Vec<f32>,
    pub _proj_coef: Vec<f32>,
}

pub struct Munki<T: UsbContext> {
    handle: DeviceHandle<T>,
    pub config: Option<MunkiConfig>,
    pub dark_ref: Option<Vec<u16>>,
    pub white_cal_factors: Option<Vec<f32>>,
}

impl<T: UsbContext> Munki<T> {
    pub fn new(handle: DeviceHandle<T>) -> Self {
        Self {
            handle,
            config: None,
            dark_ref: None,
            white_cal_factors: None,
        }
    }

    pub fn set_config(&mut self, config: MunkiConfig) {
        self.config = Some(config);
    }

    pub fn get_version_string(&self) -> Result<String> {
        let mut buf = [0u8; 100];
        let len = self
            .handle
            .read_control(
                REQ_TYPE_VENDOR_IN,
                CMD_GET_VERSION,
                0,
                0,
                &mut buf,
                Duration::from_secs(2),
            )
            .map_err(crate::SpectroError::Usb)?;
        let s = String::from_utf8_lossy(&buf[..len]);
        Ok(s.trim_matches(char::from(0)).to_string())
    }

    pub fn get_firmware_info(&self) -> Result<MunkiFirmwareInfo> {
        let mut buf = [0u8; 24];
        self.handle
            .read_control(
                REQ_TYPE_VENDOR_IN,
                CMD_GET_FIRMWARE,
                0,
                0,
                &mut buf,
                Duration::from_secs(2),
            )
            .map_err(crate::SpectroError::Usb)?;

        Ok(MunkiFirmwareInfo {
            fw_rev_major: u32::from_le_bytes(buf[0..4].try_into().unwrap()) as u8,
            fw_rev_minor: u32::from_le_bytes(buf[4..8].try_into().unwrap()) as u8,
            tick_duration: u32::from_le_bytes(buf[8..12].try_into().unwrap()),
            min_int_count: u32::from_le_bytes(buf[12..16].try_into().unwrap()),
            num_eeprom_blocks: u32::from_le_bytes(buf[16..20].try_into().unwrap()),
            eeprom_block_size: u32::from_le_bytes(buf[20..24].try_into().unwrap()),
        })
    }

    pub fn get_status(&self) -> Result<MunkiStatus> {
        let mut buf = [0u8; 2];
        self.handle
            .read_control(
                REQ_TYPE_VENDOR_IN,
                CMD_GET_STATUS,
                0,
                0,
                &mut buf,
                Duration::from_secs(2),
            )
            .map_err(crate::SpectroError::Usb)?;

        Ok(MunkiStatus {
            sensor_position: buf[0],
            button_state: buf[1],
        })
    }

    pub fn get_chip_id(&self) -> Result<Vec<u8>> {
        let mut buf = [0u8; 8];
        self.handle
            .read_control(
                REQ_TYPE_VENDOR_IN,
                CMD_GET_CHIP_ID,
                0,
                0,
                &mut buf,
                Duration::from_secs(2),
            )
            .map_err(crate::SpectroError::Usb)?;
        Ok(buf.to_vec())
    }

    pub fn read_eeprom(&self, addr: u32, size: u32) -> Result<Vec<u8>> {
        let mut params = [0u8; 8];
        params[0..4].copy_from_slice(&addr.to_le_bytes());
        params[4..8].copy_from_slice(&size.to_le_bytes());

        self.handle
            .write_control(
                REQ_TYPE_VENDOR_OUT,
                CMD_SET_EEPROM_ADDR,
                0,
                0,
                &params,
                Duration::from_secs(2),
            )
            .map_err(crate::SpectroError::Usb)?;

        let mut buf = vec![0u8; size as usize];
        self.handle
            .read_interrupt(0x81, &mut buf, Duration::from_secs(5))
            .map_err(crate::SpectroError::Usb)?;

        Ok(buf)
    }

    pub fn get_calibration_size(&self) -> Result<u32> {
        let buf = self.read_eeprom(4, 4)?;
        Ok(u32::from_le_bytes(buf[0..4].try_into().unwrap()))
    }

    pub fn parse_eeprom(&self, data: &[u8]) -> Result<MunkiConfig> {
        if data.len() < 8169 {
            return Err(crate::SpectroError::Calibration(format!(
                "EEPROM data too short: {} < 8169",
                data.len()
            )));
        }

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
        let prod_no = u32::from_le_bytes(data[12..16].try_into().unwrap());
        let serial_number = String::from_utf8_lossy(&data[24..40])
            .trim_matches('\0')
            .to_string();

        let mut rmtx_index = Vec::new();
        for i in 0..36 {
            rmtx_index.push(u32::from_le_bytes(
                data[40 + i * 4..40 + i * 4 + 4].try_into().unwrap(),
            ));
        }

        let mut rmtx_coef = Vec::new();
        for i in 0..(36 * 16) {
            rmtx_coef.push(f32::from_bits(u32::from_le_bytes(
                data[184 + i * 4..184 + i * 4 + 4].try_into().unwrap(),
            )));
        }

        let mut emtx_index = Vec::new();
        for i in 0..36 {
            emtx_index.push(u32::from_le_bytes(
                data[2488 + i * 4..2488 + i * 4 + 4].try_into().unwrap(),
            ));
        }

        let mut emtx_coef = Vec::new();
        for i in 0..(36 * 16) {
            emtx_coef.push(f32::from_bits(u32::from_le_bytes(
                data[2632 + i * 4..2632 + i * 4 + 4].try_into().unwrap(),
            )));
        }

        let mut lin_normal = Vec::new();
        for i in (0..4).rev() {
            lin_normal.push(f32::from_bits(u32::from_le_bytes(
                data[4936 + i * 4..4936 + i * 4 + 4].try_into().unwrap(),
            )));
        }

        let mut lin_high = Vec::new();
        for i in (0..4).rev() {
            lin_high.push(f32::from_bits(u32::from_le_bytes(
                data[4952 + i * 4..4952 + i * 4 + 4].try_into().unwrap(),
            )));
        }

        let mut white_ref = Vec::new();
        for i in 0..36 {
            white_ref.push(f32::from_bits(u32::from_le_bytes(
                data[4968 + i * 4..4968 + i * 4 + 4].try_into().unwrap(),
            )));
        }

        let mut emis_coef = Vec::new();
        for i in 0..36 {
            emis_coef.push(f32::from_bits(u32::from_le_bytes(
                data[5112 + i * 4..5112 + i * 4 + 4].try_into().unwrap(),
            )));
        }

        let mut amb_coef = Vec::new();
        for i in 0..36 {
            amb_coef.push(f32::from_bits(u32::from_le_bytes(
                data[5256 + i * 4..5256 + i * 4 + 4].try_into().unwrap(),
            )));
        }

        let mut proj_coef = Vec::new();
        if cal_version >= 5 {
            for i in 0..36 {
                proj_coef.push(f32::from_bits(u32::from_le_bytes(
                    data[8024 + i * 4..8024 + i * 4 + 4].try_into().unwrap(),
                )));
            }
        }

        Ok(MunkiConfig {
            cal_version,
            serial_number,
            _production_no: prod_no,
            rmtx_index,
            rmtx_coef,
            _emtx_index: emtx_index,
            _emtx_coef: emtx_coef,
            lin_normal,
            lin_high,
            white_ref,
            _emis_coef: emis_coef,
            _amb_coef: amb_coef,
            _proj_coef: proj_coef,
        })
    }

    pub fn trigger_measure(
        &self,
        int_clocks: u32,
        num_meas: u32,
        mode_flags: u8,
        hold_temp_duty: u8,
    ) -> Result<()> {
        let mut pbuf = [0u8; 12];
        pbuf[0] = if (mode_flags & MMF_LAMP) != 0 { 1 } else { 0 };
        pbuf[1] = if (mode_flags & MMF_SCAN) != 0 { 1 } else { 0 };
        pbuf[2] = if (mode_flags & MMF_HIGHGAIN) != 0 {
            1
        } else {
            0
        };
        pbuf[3] = hold_temp_duty;
        pbuf[4..8].copy_from_slice(&int_clocks.to_le_bytes());
        pbuf[8..12].copy_from_slice(&num_meas.to_le_bytes());

        self.handle
            .write_control(
                request_type(Direction::Out, RequestType::Vendor, Recipient::Device),
                CMD_TRIGGER_MEASURE,
                0,
                0,
                &pbuf,
                Duration::from_secs(2),
            )
            .map_err(crate::SpectroError::Usb)?;
        Ok(())
    }

    pub fn read_measurement(&self, num_meas: u32) -> Result<Vec<Vec<u16>>> {
        let nsen = 137;
        let bytes_per_read = nsen * 2;
        let total_bytes = bytes_per_read * num_meas as usize;
        let mut buf = vec![0u8; total_bytes];
        let mut xferred = 0;
        let mut readings = Vec::new();
        let timeout = Duration::from_secs(5);

        while xferred < total_bytes {
            let n = self
                .handle
                .read_interrupt(0x81, &mut buf[xferred..], timeout)
                .map_err(crate::SpectroError::Usb)?;
            if n == 0 {
                break;
            }
            xferred += n;
        }

        if xferred % bytes_per_read != 0 {
            return Err(crate::SpectroError::Device("Short read".into()));
        }

        for i in 0..(xferred / bytes_per_read) {
            let start = i * bytes_per_read;
            let mut reading = Vec::new();
            for j in 0..nsen {
                reading.push(u16::from_le_bytes(
                    buf[start + j * 2..start + j * 2 + 2].try_into().unwrap(),
                ));
            }
            readings.push(reading);
        }
        Ok(readings)
    }

    pub fn measure_spot(
        &self,
        int_time_sec: f64,
        tick_duration_sec: f64,
        lamp: bool,
        high_gain: bool,
    ) -> Result<Vec<u16>> {
        let int_clocks = (int_time_sec / tick_duration_sec).round() as u32;
        let mut flags = 0;
        if lamp {
            flags |= MMF_LAMP;
        }
        if high_gain {
            flags |= MMF_HIGHGAIN;
        }

        self.trigger_measure(int_clocks, 1, flags, 0)?;
        std::thread::sleep(Duration::from_millis((int_time_sec * 1000.0) as u64 + 200));

        let readings = self.read_measurement(1)?;
        readings
            .into_iter()
            .next()
            .ok_or(crate::SpectroError::Device("No data".into()))
    }

    pub fn process_spectrum(
        &self,
        raw_137: &[u16],
        int_time_sec: f64,
        high_gain: bool,
    ) -> Result<SpectralData> {
        let config = self
            .config
            .as_ref()
            .ok_or(crate::SpectroError::Calibration("No config".into()))?;
        let offset = 6;
        let mut linearized = Vec::with_capacity(128);
        let polys = if high_gain {
            &config.lin_high
        } else {
            &config.lin_normal
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

        let mut values = Vec::with_capacity(36);
        for w in 0..36 {
            let idx = config.rmtx_index[w] as usize;
            let mut sum = 0.0f32;
            for k in 0..16 {
                if idx + k < linearized.len() {
                    sum += config.rmtx_coef[w * 16 + k] * linearized[idx + k];
                }
            }
            if let Some(factors) = &self.white_cal_factors {
                sum *= factors[w];
            }
            values.push(sum);
        }

        Ok(SpectralData::new(values))
    }

    pub fn compute_white_calibration(
        &mut self,
        int_time_sec: f64,
        tick_duration_sec: f64,
    ) -> Result<()> {
        let status = self.get_status()?;
        if status.sensor_position != 2 {
            return Err(crate::SpectroError::Device(
                "Not in Calibration position".into(),
            ));
        }

        println!("  - Measuring white tile (Lamp ON)...");
        let raw_white = self.measure_spot(int_time_sec, tick_duration_sec, true, false)?;

        let old_factors = self.white_cal_factors.take();
        let spec = self.process_spectrum(&raw_white, int_time_sec, false)?;
        self.white_cal_factors = old_factors;

        let config = self
            .config
            .as_ref()
            .ok_or(crate::SpectroError::Calibration("No config".into()))?;
        let mut factors = Vec::with_capacity(36);
        for i in 0..36 {
            let measured = spec.values[i];
            let reference = config.white_ref[i];
            factors.push(if measured > 1e-6 {
                reference / measured
            } else {
                1.0
            });
        }

        self.white_cal_factors = Some(factors);
        Ok(())
    }
}
