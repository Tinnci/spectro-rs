use rusb::{DeviceHandle, Direction, Recipient, RequestType, Result, UsbContext, request_type};
use std::convert::TryInto;
use std::time::Duration;

// Request Types
const REQ_TYPE_VENDOR_IN: u8 = 0xC0; // Dir: Dev->Host, Type: Vendor, Recip: Device
const REQ_TYPE_VENDOR_OUT: u8 = 0x40; // Dir: Host->Dev, Type: Vendor, Recip: Device

// Commands
const CMD_GET_VERSION: u8 = 0x85;
const CMD_GET_FIRMWARE: u8 = 0x86;
const CMD_GET_STATUS: u8 = 0x87;
const CMD_GET_CHIP_ID: u8 = 0x8A;
const CMD_GET_MEAS_STATE: u8 = 0x8F;
const CMD_TRIGGER_MEASURE: u8 = 0x80;

pub const MMF_LAMP: u8 = 0x01;
pub const MMF_SCAN: u8 = 0x02;
pub const MMF_HIGHGAIN: u8 = 0x04;
const CMD_SET_EEPROM_ADDR: u8 = 0x81;

#[derive(Debug, Clone)]
pub struct MunkiFirmwareInfo {
    pub fw_rev_major: u8,
    pub fw_rev_minor: u8,
    pub tick_duration: u32,
    pub min_int_count: u32,
    pub num_eeprom_blocks: u32,
    pub eeprom_block_size: u32,
}

impl MunkiFirmwareInfo {
    pub fn print_details(&self) {
        println!(
            "  - Firmware Revision: {}.{}",
            self.fw_rev_major, self.fw_rev_minor
        );
        println!("  - Tick Duration: {} us", self.tick_duration);
        println!(
            "  - Min Integration Time: {} us",
            self.min_int_count * self.tick_duration
        );
        println!(
            "  - EEPROM Storage: {} x {} bytes",
            self.num_eeprom_blocks, self.eeprom_block_size
        );
    }
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
    pub production_no: u32,
    pub rmtx_index: Vec<u32>,
    pub rmtx_coef: Vec<f32>,
    pub emtx_index: Vec<u32>,
    pub emtx_coef: Vec<f32>,
    pub lin_normal: Vec<f32>,
    pub lin_high: Vec<f32>,
    pub white_ref: Vec<f32>,
    pub emis_coef: Vec<f32>,
    pub amb_coef: Vec<f32>,
    pub proj_coef: Vec<f32>,
}

pub struct Munki<T: UsbContext> {
    handle: DeviceHandle<T>,
}

impl<T: UsbContext> Munki<T> {
    pub fn new(handle: DeviceHandle<T>) -> Self {
        Self { handle }
    }

    /// Claim the interface (usually 0)
    pub fn claim_interface(&mut self, interface: u8) -> Result<()> {
        self.handle.claim_interface(interface)
    }

    /// Get version string (Cmd 0x85)
    pub fn get_version_string(&self) -> Result<String> {
        let mut buf = [0u8; 100];
        let len = self.handle.read_control(
            REQ_TYPE_VENDOR_IN,
            CMD_GET_VERSION,
            0,
            0,
            &mut buf,
            Duration::from_secs(2),
        )?;
        // Convert to string and trim nulls
        let s = String::from_utf8_lossy(&buf[..len]);
        Ok(s.trim_matches(char::from(0)).to_string())
    }

    /// Get firmware info (Cmd 0x86)
    pub fn get_firmware_info(&self) -> Result<MunkiFirmwareInfo> {
        let mut buf = [0u8; 24];
        self.handle.read_control(
            REQ_TYPE_VENDOR_IN,
            CMD_GET_FIRMWARE,
            0,
            0,
            &mut buf,
            Duration::from_secs(2),
        )?;

        // Argyll says: _fwrev_maj = buf2int(&pbuf[0]); _fwrev_min = buf2int(&pbuf[4]);
        // buf2int reads 4 bytes.

        let fw_rev_major = u32::from_le_bytes(buf[0..4].try_into().unwrap()) as u8;
        let fw_rev_minor = u32::from_le_bytes(buf[4..8].try_into().unwrap()) as u8;
        let tick_duration = u32::from_le_bytes(buf[8..12].try_into().unwrap());
        let min_int_count = u32::from_le_bytes(buf[12..16].try_into().unwrap());
        let num_eeprom_blocks = u32::from_le_bytes(buf[16..20].try_into().unwrap());
        let eeprom_block_size = u32::from_le_bytes(buf[20..24].try_into().unwrap());

        Ok(MunkiFirmwareInfo {
            fw_rev_major,
            fw_rev_minor,
            tick_duration,
            min_int_count,
            num_eeprom_blocks,
            eeprom_block_size,
        })
    }

    /// Get status (Cmd 0x87)
    pub fn get_status(&self) -> Result<MunkiStatus> {
        let mut buf = [0u8; 2];
        self.handle.read_control(
            REQ_TYPE_VENDOR_IN,
            CMD_GET_STATUS,
            0,
            0,
            &mut buf,
            Duration::from_secs(2),
        )?;

        Ok(MunkiStatus {
            sensor_position: buf[0],
            button_state: buf[1],
        })
    }

    /// Get Chip ID (Cmd 0x8A)
    pub fn get_chip_id(&self) -> Result<Vec<u8>> {
        let mut buf = [0u8; 8];
        self.handle.read_control(
            REQ_TYPE_VENDOR_IN,
            CMD_GET_CHIP_ID,
            0,
            0,
            &mut buf,
            Duration::from_secs(2),
        )?;
        Ok(buf.to_vec())
    }

    /// Read EEPROM chunk
    pub fn read_eeprom(&self, addr: u32, size: u32) -> Result<Vec<u8>> {
        let mut params = [0u8; 8];
        params[0..4].copy_from_slice(&addr.to_le_bytes());
        params[4..8].copy_from_slice(&size.to_le_bytes());

        self.handle.write_control(
            REQ_TYPE_VENDOR_OUT,
            CMD_SET_EEPROM_ADDR,
            0,
            0,
            &params,
            Duration::from_secs(2),
        )?;

        let mut buf = vec![0u8; size as usize];
        // The endpoint is 0x81 for reading
        self.handle
            .read_interrupt(0x81, &mut buf, Duration::from_secs(5))?;

        Ok(buf)
    }

    pub fn get_calibration_size(&self) -> Result<u32> {
        let buf = self.read_eeprom(4, 4)?;
        Ok(u32::from_le_bytes(buf[0..4].try_into().unwrap()))
    }

    pub fn parse_eeprom(&self, data: &[u8]) -> std::result::Result<MunkiConfig, String> {
        if data.len() < 8169 {
            return Err(format!("EEPROM data too short: {} < 8169", data.len()));
        }

        // 1. Verify Checksum
        let stored_checksum = u32::from_le_bytes(data[8..12].try_into().unwrap());
        let mut sum: u32 = 0;
        let mut i = 0;
        while i < data.len() {
            if i == 8 {
                i += 4; // Skip the checksum field
                continue;
            }

            if i + 4 <= data.len() {
                sum = sum.wrapping_add(u32::from_le_bytes(data[i..i + 4].try_into().unwrap()));
                i += 4;
            } else {
                // Add remaining bytes (1 to 3 bytes)
                let mut last_bytes = [0u8; 4];
                let rem = data.len() - i;
                last_bytes[..rem].copy_from_slice(&data[i..]);
                sum = sum.wrapping_add(u32::from_le_bytes(last_bytes));
                break;
            }
        }

        if sum != stored_checksum {
            return Err(format!(
                "EEPROM Checksum mismatch: calculated {:08X}, stored {:08X}",
                sum, stored_checksum
            ));
        }

        // 2. Basic Info
        let cal_version = u16::from_le_bytes(data[0..2].try_into().unwrap());
        let prod_no = u32::from_le_bytes(data[12..16].try_into().unwrap());
        let ser_no_raw = &data[24..40];
        let serial_number = String::from_utf8_lossy(ser_no_raw)
            .trim_matches('\0')
            .to_string();

        // 3. Calibration Matrices (36 wavelengths)
        let mut rmtx_index = Vec::new();
        for i in 0..36 {
            rmtx_index.push(u32::from_le_bytes(
                data[40 + i * 4..40 + i * 4 + 4].try_into().unwrap(),
            ));
        }

        let mut rmtx_coef = Vec::new();
        for i in 0..(36 * 16) {
            let bits = u32::from_le_bytes(data[184 + i * 4..184 + i * 4 + 4].try_into().unwrap());
            rmtx_coef.push(f32::from_bits(bits));
        }

        let mut emtx_index = Vec::new();
        for i in 0..36 {
            emtx_index.push(u32::from_le_bytes(
                data[2488 + i * 4..2488 + i * 4 + 4].try_into().unwrap(),
            ));
        }

        let mut emtx_coef = Vec::new();
        for i in 0..(36 * 16) {
            let bits = u32::from_le_bytes(data[2632 + i * 4..2632 + i * 4 + 4].try_into().unwrap());
            emtx_coef.push(f32::from_bits(bits));
        }

        // 4. Linearization (Stored in reverse order)
        let mut lin_normal = Vec::new();
        for i in (0..4).rev() {
            let bits = u32::from_le_bytes(data[4936 + i * 4..4936 + i * 4 + 4].try_into().unwrap());
            lin_normal.push(f32::from_bits(bits));
        }

        let mut lin_high = Vec::new();
        for i in (0..4).rev() {
            let bits = u32::from_le_bytes(data[4952 + i * 4..4952 + i * 4 + 4].try_into().unwrap());
            lin_high.push(f32::from_bits(bits));
        }

        // 5. Coefficients
        let mut white_ref = Vec::new();
        for i in 0..36 {
            let bits = u32::from_le_bytes(data[4968 + i * 4..4968 + i * 4 + 4].try_into().unwrap());
            white_ref.push(f32::from_bits(bits));
        }

        let mut emis_coef = Vec::new();
        for i in 0..36 {
            let bits = u32::from_le_bytes(data[5112 + i * 4..5112 + i * 4 + 4].try_into().unwrap());
            emis_coef.push(f32::from_bits(bits));
        }

        let mut amb_coef = Vec::new();
        for i in 0..36 {
            let bits = u32::from_le_bytes(data[5256 + i * 4..5256 + i * 4 + 4].try_into().unwrap());
            amb_coef.push(f32::from_bits(bits));
        }

        let mut proj_coef = Vec::new();
        if cal_version >= 5 {
            for i in 0..36 {
                let bits =
                    u32::from_le_bytes(data[8024 + i * 4..8024 + i * 4 + 4].try_into().unwrap());
                proj_coef.push(f32::from_bits(bits));
            }
        }

        Ok(MunkiConfig {
            cal_version,
            serial_number,
            production_no: prod_no,
            rmtx_index,
            rmtx_coef,
            emtx_index,
            emtx_coef,
            lin_normal,
            lin_high,
            white_ref,
            emis_coef,
            amb_coef,
            proj_coef,
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

        self.handle.write_control(
            request_type(Direction::Out, RequestType::Vendor, Recipient::Device),
            CMD_TRIGGER_MEASURE,
            0,
            0,
            &pbuf,
            Duration::from_secs(2),
        )?;
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
                .read_interrupt(0x81, &mut buf[xferred..], timeout)?;
            if n == 0 {
                break;
            }
            xferred += n;
        }

        if xferred % bytes_per_read != 0 {
            return Err(rusb::Error::Other);
        }

        let actual_meas = xferred / bytes_per_read;
        for i in 0..actual_meas {
            let start = i * bytes_per_read;
            let mut reading = Vec::new();
            for j in 0..nsen {
                let off = start + j * 2;
                reading.push(u16::from_le_bytes(buf[off..off + 2].try_into().unwrap()));
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
        if readings.is_empty() {
            return Err(rusb::Error::NoDevice);
        }
        Ok(readings[0].clone())
    }
}
