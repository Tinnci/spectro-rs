use crate::Result;
use std::convert::TryInto;

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
    pub emis_coef: Vec<f32>,
    pub amb_coef: Vec<f32>,
    pub minsval: f64,
    pub optsval: f64,
    pub maxsval: f64,
    pub satlimit: f64,
    pub adctype: u8,
}

pub struct EepromParser;

impl EepromParser {
    pub fn parse(data: &[u8]) -> Result<MunkiConfig> {
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

        let mut emis_coef = Vec::with_capacity(36);
        for i in 0..36 {
            emis_coef.push(f32::from_bits(u32::from_le_bytes(
                data[5112 + i * 4..5112 + i * 4 + 4].try_into().unwrap(),
            )));
        }

        let mut amb_coef = Vec::with_capacity(36);
        for i in 0..36 {
            amb_coef.push(f32::from_bits(u32::from_le_bytes(
                data[5256 + i * 4..5256 + i * 4 + 4].try_into().unwrap(),
            )));
        }

        let minsval = u16::from_le_bytes(data[5400..5402].try_into().unwrap()) as f64;
        let optsval = u16::from_le_bytes(data[5402..5404].try_into().unwrap()) as f64;
        let maxsval = u16::from_le_bytes(data[5404..5406].try_into().unwrap()) as f64;
        let satlimit = u16::from_le_bytes(data[5406..5408].try_into().unwrap()) as f64;

        let adctype = if cal_version >= 6 { data[8168] } else { 0 };

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
            emis_coef,
            amb_coef,
            minsval,
            optsval,
            maxsval,
            satlimit,
            adctype,
        })
    }
}
