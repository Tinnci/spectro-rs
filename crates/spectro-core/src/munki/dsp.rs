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
        physics_model: Option<&crate::munki::physics::SensorModel>,
        high_gain: bool,
        mode: MeasurementMode,
    ) -> Result<SpectralData> {
        let offset = 6;

        // 1. Adaptive Dark Current Compensation
        let drift = if let Some(dark) = dark_ref {
            let curr_shield_avg: f64 = raw_data[0..4].iter().map(|&x| x as f64).sum::<f64>() / 4.0;
            let ref_shield_avg: f64 = dark[0..4].iter().map(|&x| x as f64).sum::<f64>() / 4.0;
            curr_shield_avg - ref_shield_avg
        } else {
            0.0
        };

        if drift.abs() > 0.1 {
            println!("DEBUG: Thermal drift detected: {:.2} counts", drift);
        }

        // Debug raw sensor stats
        if mode == MeasurementMode::Reflective {
            let min_raw = raw_data.iter().min().unwrap_or(&0);
            let max_raw = raw_data.iter().max().unwrap_or(&0);
            let avg_raw: f64 =
                raw_data.iter().map(|&x| x as f64).sum::<f64>() / raw_data.len() as f64;
            println!(
                "DEBUG: Raw Sensor Stats (Lamp ON): Min={}, Max={}, Avg={:.1}",
                min_raw, max_raw, avg_raw
            );

            if let Some(dark) = dark_ref {
                let min_dark = dark.iter().min().unwrap_or(&0);
                let max_dark = dark.iter().max().unwrap_or(&0);
                let avg_dark: f64 = dark.iter().map(|&x| x as f64).sum::<f64>() / dark.len() as f64;
                println!(
                    "DEBUG: Dark Ref Stats (Lamp OFF): Min={}, Max={}, Avg={:.1}",
                    min_dark, max_dark, avg_dark
                );
            }
        }

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

            // Subtract dark reference and compensate for drift
            if let Some(dark) = dark_ref
                && offset + i < dark.len()
            {
                // Drift compensation is always needed
                val -= drift;

                // If using Physics Model -> Do NOT subtract static dark frame (model handles bias).
                // If using Legacy Model  -> MUST subtract static dark frame.
                if physics_model.is_none() {
                    val -= dark[offset + i] as f64;
                }
            }

            // High-quality sensors should not have negative light
            val = val.max(0.0);

            if let Some(model) = physics_model {
                // --- PHYSICS PATH ---
                // 1. Linearize using physical transfer function: k = (y - bias) / (t - dead)
                let k_rate = model
                    .solve_intensity(val, integration_time_sec)
                    .unwrap_or(0.0);

                // 2. Map to legacy energy scale using P1 (Gain) and P0 (Offset)
                let ideal_counts = k_rate * integration_time_sec;
                val = ideal_counts * polys[1] as f64 + polys[0] as f64;
            } else {
                // --- LEGACY PATH ---
                let mut lval = polys[3] as f64;
                lval = lval * val + polys[2] as f64;
                lval = lval * val + polys[1] as f64;
                lval = lval * val + polys[0] as f64;
                val = lval;
            }

            // Scale by integration time to get per-second rate
            linearized.push((val * scale) as f32);
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

            values.push(sum.max(0.0));
        }

        // 4. Blind Zone Compensation (Reflective Mode Only)
        // The sensor relies on a White LED which has no energy in UV (<420nm) and IR (>690nm).
        // In these "Blind Zones", the signal is 0, which results in Reflectance = 0 (Black).
        // This causes artifacts (e.g. White Tile looks Green because Blue/Red bands are black).
        // Fix: Use nearest-neighbor interpolation for bands where White Calibration Signal was too low.
        if mode == MeasurementMode::Reflective
            && let Some(factors) = white_factors
        {
            // Signal-Gated Spectral Extrapolation
            // Instead of checking factors (which are inverted), we estimate the original White Signal.
            // White_Signal ~= Reference / Factor

            // 1. Calculate Peak Signal across all bands
            let max_white_signal = (0..factors.len())
                .map(|i| {
                    let white_ref = config.white_ref.get(i).copied().unwrap_or(0.96);
                    let factor = factors.get(i).copied().unwrap_or(1.0);
                    if factor > 1e-9 {
                        white_ref / factor
                    } else {
                        0.0
                    }
                })
                .fold(0.0f32, |a, b| a.max(b));

            // 2. Set dynamic threshold: only trust data above 25% of the peak
            // This ensures we anchor to the stable center of the blue peak, avoiding unstable slopes.
            let dynamic_threshold = max_white_signal * 0.25;
            println!(
                "DEBUG: Extrapolation Dynamic Threshold = {:.1} (Peak = {:.1})",
                dynamic_threshold, max_white_signal
            );

            // 3. Find the range of valid (non-blind) bands
            let mut first_valid = None;
            let mut last_valid = None;

            for (i, &factor) in factors.iter().enumerate().take(values.len()) {
                let white_ref = config.white_ref.get(i).copied().unwrap_or(0.96);

                // Avoid division by zero if factor is 0 (shouldn't happen)
                let signal = if factor > 1e-9 {
                    white_ref / factor
                } else {
                    0.0
                };

                if signal > dynamic_threshold {
                    if first_valid.is_none() {
                        first_valid = Some(i);
                    }
                    last_valid = Some(i);
                }
            }

            if let (Some(first), Some(last)) = (first_valid, last_valid) {
                println!(
                    "DEBUG: Blind Zone Extrapolation: Valid=[{}-{}], R[first]={:.4}",
                    first, last, values[first]
                );
                // We don't trust the very first point (first) as it's often on the unstable slope.
                // Use the next point inside (first + 1) for a more robust anchor.
                let safe_anchor_idx = (first + 1).min(last);
                let anchor_value = values[safe_anchor_idx];

                // Extrapolate UV/Blue (Left side) - include the 'first' band itself
                for v in values.iter_mut().take(first + 1) {
                    *v = anchor_value;
                }

                // Extrapolate IR/Red (Right side)
                let end_val = values[last];
                for v in values.iter_mut().skip(last + 1) {
                    *v = end_val;
                }
            }
        }

        // 5. Spectral Smoothing (Boxcar Filter)
        // Apply a 3-point sliding average [0.25, 0.5, 0.25] to smooth out random noise and improve Lab stability.
        // We do this after extrapolation so that the edges are already stable.
        if mode == MeasurementMode::Reflective {
            let mut smoothed = values.clone();
            let len = values.len();
            for i in 1..(len - 1) {
                smoothed[i] = 0.25 * values[i - 1] + 0.5 * values[i] + 0.25 * values[i + 1];
            }
            // Simple edge handling (replicate) or just leave them (since extrapolated)
            // We'll leave 0 and len-1 as they are (or extrapolated value).
            values = smoothed;
        }

        // Debug output to diagnose spectral processing
        if mode == MeasurementMode::Reflective {
            println!("\n=== DSP Processing (Reflective Mode) ===");
            println!(
                "Spectral reconstruction completed: {} wavelength bands",
                values.len()
            );
            println!(
                "First 5 raw values: [{:.8e}, {:.8e}, {:.8e}, {:.8e}, {:.8e}]",
                values.first().unwrap_or(&0.0),
                values.get(1).unwrap_or(&0.0),
                values.get(2).unwrap_or(&0.0),
                values.get(3).unwrap_or(&0.0),
                values.get(4).unwrap_or(&0.0)
            );
            println!(
                "Middle 5 (idx 15-19): [{:.8e}, {:.8e}, {:.8e}, {:.8e}, {:.8e}]",
                values.get(15).unwrap_or(&0.0),
                values.get(16).unwrap_or(&0.0),
                values.get(17).unwrap_or(&0.0),
                values.get(18).unwrap_or(&0.0),
                values.get(19).unwrap_or(&0.0)
            );
            println!(
                "Last  5 (idx 31-35): [{:.8e}, {:.8e}, {:.8e}, {:.8e}, {:.8e}]",
                values.get(31).unwrap_or(&0.0),
                values.get(32).unwrap_or(&0.0),
                values.get(33).unwrap_or(&0.0),
                values.get(34).unwrap_or(&0.0),
                values.get(35).unwrap_or(&0.0)
            );
            if let Some(factors) = white_factors {
                println!("White calibration applied: YES");
                println!(
                    "First 5 factors: [{:.8e}, {:.8e}, {:.8e}, {:.8e}, {:.8e}]",
                    factors.first().unwrap_or(&1.0),
                    factors.get(1).unwrap_or(&1.0),
                    factors.get(2).unwrap_or(&1.0),
                    factors.get(3).unwrap_or(&1.0),
                    factors.get(4).unwrap_or(&1.0)
                );
            } else {
                println!("White calibration applied: NO");
            }
            println!("===");
        }

        Ok(SpectralData::with_mode(values, mode))
    }
}
