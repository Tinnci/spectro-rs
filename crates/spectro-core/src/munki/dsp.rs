use super::MunkiConfig;
use crate::spectrum::SpectralData;
use crate::{MeasurementMode, Result};

/// Handles digital signal processing for the ColorMunki.
///
/// This includes:
/// 1. Linearization (converting raw ADC counts to linear space)
/// 2. Dark current subtraction
/// 3. Spectral reconstruction (Sensor space -> Wavelength space)
/// 4. Calibration application (White balance, Ambient/Emissive scaling)
pub struct SignalProcessor;

impl SignalProcessor {
    /// Process raw sensor data into spectral data.
    #[allow(clippy::too_many_arguments)]
    pub fn process(
        config: &MunkiConfig,
        integration_time_sec: f64,
        raw_data: &[u16],
        dark_ref: Option<&[u16]>,
        white_factors: Option<&[f32]>,
        high_gain: bool,
        mode: MeasurementMode,
    ) -> Result<SpectralData> {
        let offset = 6;

        // 1. Linearize and subtract dark current
        // The sensor has 128 active pixels starting at offset 6 in the 137-pixel logical array
        let mut linearized = Vec::with_capacity(128);

        let polys = if high_gain {
            &config.lin_high
        } else {
            &config.lin_normal
        };

        let scale = 1.0 / integration_time_sec;

        for i in 0..128 {
            if offset + i >= raw_data.len() {
                break;
            }

            let mut val = raw_data[offset + i] as f64;

            // Subtract dark reference if available
            if let Some(dark) = dark_ref
                && offset + i < dark.len()
            {
                val -= dark[offset + i] as f64;
            }

            // Apply linearity polynomial (3rd order)
            // L(v) = p3*v^3 + p2*v^2 + p1*v + p0
            // Computed efficiently using Horner's method
            let mut lval = polys[3] as f64;
            lval = lval * val + polys[2] as f64;
            lval = lval * val + polys[1] as f64;
            lval = lval * val + polys[0] as f64;

            // Scale by integration time
            linearized.push((lval * scale) as f32);
        }

        // 2. Map processed sensor data to wavelengths (Matrix Multiplication)
        // Reconstruct 36 spectral bands (380nm - 730nm) from 128 sensor pixels
        let (mtx_index, mtx_coef) = if mode == MeasurementMode::Emissive {
            (&config.emtx_index, &config.emtx_coef)
        } else {
            (&config.rmtx_index, &config.rmtx_coef)
        };

        let mut values = Vec::with_capacity(36);
        for w in 0..36 {
            let idx = mtx_index[w] as usize;
            let mut sum = 0.0f32;

            // Each wavelength band is a weighted sum of up to 16 sensor pixels
            for k in 0..16 {
                if idx + k < linearized.len() {
                    sum += mtx_coef[w * 16 + k] * linearized[idx + k];
                }
            }

            // 3. Apply calibration / scaling factors based on mode
            match mode {
                MeasurementMode::Reflective => {
                    if let Some(factors) = white_factors
                        && w < factors.len()
                    {
                        sum *= factors[w];
                    }
                }
                MeasurementMode::Ambient => {
                    if w < config.amb_coef.len() {
                        sum *= config.amb_coef[w];
                    }
                }
                MeasurementMode::Emissive => {
                    if w < config.emis_coef.len() {
                        sum *= config.emis_coef[w];
                    }
                }
            }

            values.push(sum);
        }

        Ok(SpectralData::new(values))
    }
}
