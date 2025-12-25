use crate::colorimetry::{weighting, XYZ, X_BAR_10, X_BAR_2, Y_BAR_10, Y_BAR_2, Z_BAR_10, Z_BAR_2};
use crate::WAVELENGTHS;
use crate::{Illuminant, Observer};

/// Measurement mode determines the calculation method for XYZ conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum MeasurementMode {
    /// Reflective measurement (objects like paper, color patches)
    /// Uses ASTM E308 weighting factors which include D65 SPD
    #[default]
    Reflective,
    /// Emissive measurement (light sources like displays, lamps)
    /// Uses direct CMF integration
    Emissive,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SpectralData {
    pub wavelengths: Vec<f32>,
    pub values: Vec<f32>,
    /// Measurement mode affects XYZ calculation method
    pub mode: MeasurementMode,
}

impl SpectralData {
    pub fn new(mut values: Vec<f32>) -> Self {
        // Pad with zeros if less than 41 points (common for 380-730nm devices like ColorMunki)
        while values.len() < 41 {
            values.push(0.0);
        }
        Self {
            wavelengths: WAVELENGTHS.to_vec(),
            values,
            mode: MeasurementMode::default(),
        }
    }

    /// Create spectral data with explicit measurement mode
    pub fn with_mode(mut values: Vec<f32>, mode: MeasurementMode) -> Self {
        while values.len() < 41 {
            values.push(0.0);
        }
        Self {
            wavelengths: WAVELENGTHS.to_vec(),
            values,
            mode,
        }
    }

    /// Set the measurement mode
    pub fn set_mode(&mut self, mode: MeasurementMode) {
        self.mode = mode;
    }

    /// Convert to XYZ using the standard 2-degree observer and D65.
    /// Default method for backward compatibility.
    pub fn to_xyz(&self) -> XYZ {
        self.to_xyz_ext(Illuminant::D65, Observer::CIE1931_2)
    }

    /// Convert to XYZ using specified illuminant and observer.
    ///
    /// For reflective measurements, uses ASTM E308 weighting factors when available.
    /// Currently supported: D65/2°, D50/2°.
    pub fn to_xyz_ext(&self, source: Illuminant, obs: Observer) -> XYZ {
        match self.mode {
            MeasurementMode::Reflective => {
                match (source, obs) {
                    (Illuminant::D65, Observer::CIE1931_2) => self.to_xyz_reflective_weighted(
                        &weighting::WX_D65_2_10,
                        &weighting::WY_D65_2_10,
                        &weighting::WZ_D65_2_10,
                        weighting::SUM_WY_D65_2_10,
                    ),
                    (Illuminant::D50, Observer::CIE1931_2) => self.to_xyz_reflective_weighted(
                        &weighting::WX_D50_2_10,
                        &weighting::WY_D50_2_10,
                        &weighting::WZ_D50_2_10,
                        weighting::SUM_WY_D50_2_10,
                    ),
                    // For other combinations, calculate weighting factors dynamically
                    _ => {
                        let spd = source.get_spd();
                        let (xb, yb, zb) = obs.get_cmfs();
                        let mut wx = [0.0f32; 41];
                        let mut wy = [0.0f32; 41];
                        let mut wz = [0.0f32; 41];
                        let mut sum_wy = 0.0f32;

                        for i in 0..41 {
                            wx[i] = spd[i] * xb[i];
                            wy[i] = spd[i] * yb[i];
                            wz[i] = spd[i] * zb[i];
                            sum_wy += wy[i];
                        }

                        self.to_xyz_reflective_weighted(&wx, &wy, &wz, sum_wy)
                    }
                }
            }
            MeasurementMode::Emissive => self.to_xyz_emissive_ext(obs),
        }
    }

    /// Convert reflectance to XYZ using provided weighting factors.
    fn to_xyz_reflective_weighted(
        &self,
        wx: &[f32; 41],
        wy: &[f32; 41],
        wz: &[f32; 41],
        sum_wy: f32,
    ) -> XYZ {
        let mut x = 0.0f32;
        let mut y = 0.0f32;
        let mut z = 0.0f32;

        for i in 0..41 {
            x += self.values[i] * wx[i];
            y += self.values[i] * wy[i];
            z += self.values[i] * wz[i];
        }

        // Normalize so that Y=100 for a perfect white diffuser
        let scale = 100.0 / sum_wy;

        XYZ {
            x: x * scale,
            y: y * scale,
            z: z * scale,
        }
    }

    /// Resample spectral data to a new wavelength range and step.
    /// Uses Sprague interpolation for high accuracy, which is recommended
    /// by the CIE for spectral data resampling.
    pub fn resample(&self, start: f32, end: f32, step: f32) -> Self {
        let mut new_values = Vec::new();
        let mut current_wl = start;

        // Pad values for Sprague (needs 2 before and 3 after)
        let mut padded_values = Vec::with_capacity(self.values.len() + 5);
        if !self.values.is_empty() {
            padded_values.push(self.values[0]);
            padded_values.push(self.values[0]);
            padded_values.extend_from_slice(&self.values);
            padded_values.push(*self.values.last().unwrap());
            padded_values.push(*self.values.last().unwrap());
            padded_values.push(*self.values.last().unwrap());
        } else {
            return Self {
                wavelengths: Vec::new(),
                values: Vec::new(),
                mode: self.mode,
            };
        }

        let orig_start = self.wavelengths[0];
        let orig_step = if self.wavelengths.len() > 1 {
            self.wavelengths[1] - self.wavelengths[0]
        } else {
            10.0
        };

        while current_wl <= end + 1e-3 {
            let t = (current_wl - orig_start) / orig_step;
            let i = t.floor() as i32;
            let x = t - i as f32;

            // i is the index of y0 in the original values
            // In padded_values, y0 is at index i + 2
            let idx = (i + 2) as usize;

            if idx < 2 || idx + 3 >= padded_values.len() {
                // Fallback to linear or clamping at edges
                let val = if current_wl <= orig_start {
                    self.values[0]
                } else if current_wl >= orig_start + (self.values.len() - 1) as f32 * orig_step {
                    *self.values.last().unwrap()
                } else {
                    let i_idx = i.max(0) as usize;
                    let v0 = self.values[i_idx];
                    let v1 = self.values[(i_idx + 1).min(self.values.len() - 1)];
                    v0 + x * (v1 - v0)
                };
                new_values.push(val);
            } else {
                let y = [
                    padded_values[idx - 2],
                    padded_values[idx - 1],
                    padded_values[idx],
                    padded_values[idx + 1],
                    padded_values[idx + 2],
                    padded_values[idx + 3],
                ];
                new_values.push(Self::sprague_interpolate(x, &y));
            }

            current_wl += step;
        }

        let mut wavelengths = Vec::new();
        let mut wl = start;
        while wl <= end + 1e-3 {
            wavelengths.push(wl);
            wl += step;
        }

        Self {
            wavelengths,
            values: new_values,
            mode: self.mode,
        }
    }

    /// Sprague interpolation for a point x in [0, 1] between y[2] and y[3].
    /// y must contain 6 points: y[-2], y[-1], y[0], y[1], y[2], y[3].
    fn sprague_interpolate(x: f32, y: &[f32; 6]) -> f32 {
        let x2 = x * x;
        let x3 = x2 * x;
        let x4 = x3 * x;
        let x5 = x4 * x;

        // Sprague coefficients matrix
        let a0 = y[2];
        let a1 = (2.0 * y[0] - 16.0 * y[1] + 16.0 * y[3] - 2.0 * y[4]) / 24.0;
        let a2 = (-y[0] + 16.0 * y[1] - 30.0 * y[2] + 16.0 * y[3] - y[4]) / 24.0;
        let a3 = (-9.0 * y[0] + 39.0 * y[1] - 70.0 * y[2] + 66.0 * y[3] - 33.0 * y[4] + 7.0 * y[5])
            / 120.0;
        let a4 = (13.0 * y[0] - 64.0 * y[1] + 126.0 * y[2] - 124.0 * y[3] + 61.0 * y[4]
            - 12.0 * y[5])
            / 120.0;
        let a5 = (-5.0 * y[0] + 25.0 * y[1] - 50.0 * y[2] + 50.0 * y[3] - 25.0 * y[4] + 5.0 * y[5])
            / 120.0;

        a0 + a1 * x + a2 * x2 + a3 * x3 + a4 * x4 + a5 * x5
    }

    /// Convert spectral power distribution to XYZ with specified observer.
    pub fn to_xyz_emissive_ext(&self, obs: Observer) -> XYZ {
        const STEP: f32 = 10.0;
        let (xb, yb, zb) = obs.get_cmfs();

        let mut x = 0.0f32;
        let mut y = 0.0f32;
        let mut z = 0.0f32;

        for i in 0..41 {
            x += self.values[i] * xb[i];
            y += self.values[i] * yb[i];
            z += self.values[i] * zb[i];
        }

        XYZ {
            x: x * STEP,
            y: y * STEP,
            z: z * STEP,
        }
    }

    /// Get the raw wavelengths and values as references.
    /// Used for spectral reconstruction and external processing.
    pub fn get_wavelength_data(&self) -> (Vec<f32>, Vec<f32>) {
        (self.wavelengths.clone(), self.values.clone())
    }

    /// Convert reflectance to XYZ using ASTM E308 weighting factors (D65/2°).
    /// This is the most accurate method for reflective measurements.
    ///
    /// The weighting factors already include:
    /// - D65 spectral power distribution
    /// - CIE 1931 2° standard observer CMFs
    /// - Proper normalization
    pub fn to_xyz_reflective_2(&self) -> XYZ {
        let mut x = 0.0f32;
        let mut y = 0.0f32;
        let mut z = 0.0f32;

        for i in 0..41 {
            x += self.values[i] * weighting::WX_D65_2_10[i];
            y += self.values[i] * weighting::WY_D65_2_10[i];
            z += self.values[i] * weighting::WZ_D65_2_10[i];
        }

        // ASTM E308 weights when summed for R=1.0 give ~10.683
        // Normalize so that Y=100 for a perfect white diffuser
        let scale = 100.0 / weighting::SUM_WY_D65_2_10;

        XYZ {
            x: x * scale,
            y: y * scale,
            z: z * scale,
        }
    }

    /// Convert spectral power distribution to XYZ for emissive sources (2° observer).
    /// Uses direct integration with CIE CMFs.
    ///
    /// # Output Units
    ///
    /// The output units depend on how the spectral data was processed:
    /// - If spectral values are in device-calibrated units (via EEPROM `emis_coef`),
    ///   the Y value approximates luminance in cd/m² (after proper device calibration).
    /// - For raw spectral power in W/sr/m²/nm, multiply Y by Km=683 lm/W for cd/m².
    ///
    /// Note: The ColorMunki's EEPROM `emis_coef` provides device-specific calibration
    /// that should produce results comparable to ArgyllCMS when properly applied.
    pub fn to_xyz_emissive_2(&self) -> XYZ {
        const STEP: f32 = 10.0; // 10nm wavelength step

        let mut x = 0.0f32;
        let mut y = 0.0f32;
        let mut z = 0.0f32;

        for i in 0..41 {
            x += self.values[i] * X_BAR_2[i];
            y += self.values[i] * Y_BAR_2[i];
            z += self.values[i] * Z_BAR_2[i];
        }

        // Integrate P(λ) * CMF(λ) * Δλ
        // No additional Km scaling - emis_coef from EEPROM provides calibration
        XYZ {
            x: x * STEP,
            y: y * STEP,
            z: z * STEP,
        }
    }

    /// Convert to XYZ using the 2-degree observer (CIE 1931).
    /// Legacy method - uses CMF integration (suitable for emissive sources)
    #[deprecated(
        since = "0.2.0",
        note = "Use to_xyz() with appropriate MeasurementMode"
    )]
    pub fn to_xyz_2(&self) -> XYZ {
        self.to_xyz_emissive_2()
    }

    /// Convert to XYZ using the 10-degree observer (CIE 1964).
    /// Uses CMF integration (suitable for emissive sources)
    pub fn to_xyz_10(&self) -> XYZ {
        const STEP: f32 = 10.0;

        let mut x = 0.0f32;
        let mut y = 0.0f32;
        let mut z = 0.0f32;

        for i in 0..41 {
            x += self.values[i] * X_BAR_10[i];
            y += self.values[i] * Y_BAR_10[i];
            z += self.values[i] * Z_BAR_10[i];
        }

        XYZ {
            x: x * STEP,
            y: y * STEP,
            z: z * STEP,
        }
    }

    /// Calculate the normalization constant k for reflectance mode.
    /// k = 100 / Σ(S(λ) * ȳ(λ) * Δλ)
    ///
    /// This is useful when you have raw illuminant SPD and CMF data
    /// and need to compute the normalization factor dynamically.
    ///
    /// # Arguments
    /// * `illuminant_spd` - Relative spectral power distribution of the illuminant
    /// * `y_bar` - Y color matching function values
    /// * `step` - Wavelength step in nm
    pub fn calculate_k(illuminant_spd: &[f32], y_bar: &[f32], step: f32) -> f32 {
        let sum_s_y: f32 = illuminant_spd
            .iter()
            .zip(y_bar.iter())
            .map(|(s, yb)| s * yb)
            .sum();
        100.0 / (sum_s_y * step)
    }
}

impl XYZ {
    pub fn to_chromaticity(&self) -> (f32, f32) {
        let sum = self.x + self.y + self.z;
        if sum < 1e-6 {
            return (0.3127, 0.3290);
        } // Default to D65 if zero
        (self.x / sum, self.y / sum)
    }

    /// Calculate Correlated Color Temperature (CCT) using McCamy's formula.
    pub fn to_cct(&self) -> f32 {
        let (x, y) = self.to_chromaticity();
        let n = (x - 0.3320) / (0.1858 - y);
        // McCamy's formula
        449.0 * n.powi(3) + 3525.0 * n.powi(2) + 6823.3 * n + 5524.33
    }
}

impl std::fmt::Display for SpectralData {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Spectral Data (380nm - 730nm, {:?} mode):", self.mode)?;
        for (w, v) in self.wavelengths.iter().zip(self.values.iter()) {
            writeln!(f, "  {:.0}nm: {:.6}", w, v)?;
        }
        Ok(())
    }
}
