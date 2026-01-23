use crate::WAVELENGTHS;
use crate::colorimetry::{X_BAR_2, X_BAR_10, XYZ, Y_BAR_2, Y_BAR_10, Z_BAR_2, Z_BAR_10, weighting};
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
    /// Ambient light measurement (same as Emissive but typically with cosine corrector)
    Ambient,
}

/// A consolidated result object containing all standard colorimetric values.
/// This enforces Single Source of Truth by ensuring that derived values (XYZ, Lab, RGB)
/// are always calculated consistently alongside the spectral data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MeasurementResult {
    pub spectrum: SpectralData,
    /// CIE 1931 XYZ (D50 adapted)
    pub xyz: XYZ,
    /// CIE L*a*b* (D50 illuminant)
    pub lab: crate::colorimetry::Lab,
    /// sRGB (0-1 range, D65)
    pub rgb: (f32, f32, f32),
    /// Correlated Color Temperature (K)
    pub cct: f32,
    /// Color Rendering Index (Ra) - Placeholder for now
    pub cri: Option<f32>,
}

impl MeasurementResult {
    /// Get RGB values in 0-255 range
    pub fn rgb_u8(&self) -> (u8, u8, u8) {
        (
            (self.rgb.0.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.rgb.1.clamp(0.0, 1.0) * 255.0).round() as u8,
            (self.rgb.2.clamp(0.0, 1.0) * 255.0).round() as u8,
        )
    }

    /// Calculate the peak wavelength (nm)
    pub fn peak_wavelength(&self) -> f32 {
        let (idx, _) = self.spectrum.values.iter().enumerate().fold(
            (0, 0.0f32),
            |(max_idx, max_val), (idx, &val)| {
                if val > max_val {
                    (idx, val)
                } else {
                    (max_idx, max_val)
                }
            },
        );
        380.0 + idx as f32 * 10.0
    }

    /// Calculate the centroid wavelength (nm)
    pub fn centroid_wavelength(&self) -> f32 {
        let total: f32 = self.spectrum.values.iter().sum();
        if total < 1e-6 {
            return 550.0;
        }
        let weighted_sum: f32 = self
            .spectrum
            .values
            .iter()
            .enumerate()
            .map(|(i, v)| (380.0 + i as f32 * 10.0) * v)
            .sum();
        weighted_sum / total
    }
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

    /// Get the raw wavelengths and values as references.
    pub fn get_wavelength_data(&self) -> (Vec<f32>, Vec<f32>) {
        (self.wavelengths.clone(), self.values.clone())
    }
}

// ============================================================================
// Behavior Traits (SRP)
// ============================================================================

/// Trait for converting spectral data into various color spaces.
pub trait Colorimetry {
    fn to_xyz(&self) -> XYZ;
    fn to_xyz_ext(&self, source: Illuminant, obs: Observer) -> XYZ;
    fn to_xyz_reflective_2(&self) -> XYZ;
    fn to_xyz_emissive_2(&self) -> XYZ;
    fn to_xyz_emissive_ext(&self, obs: Observer) -> XYZ;
    fn to_xyz_10(&self) -> XYZ;
}

/// Trait for calculating light quality metrics.
pub trait QualityMetrics {
    fn calculate_cri(&self) -> Option<f32>;
}

impl Colorimetry for SpectralData {
    fn to_xyz(&self) -> XYZ {
        self.to_xyz_ext(Illuminant::D65, Observer::CIE1931_2)
    }

    fn to_xyz_ext(&self, source: Illuminant, obs: Observer) -> XYZ {
        match self.mode {
            MeasurementMode::Reflective => match (source, obs) {
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
            },
            MeasurementMode::Emissive | MeasurementMode::Ambient => self.to_xyz_emissive_ext(obs),
        }
    }

    fn to_xyz_emissive_ext(&self, obs: Observer) -> XYZ {
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

    fn to_xyz_reflective_2(&self) -> XYZ {
        let mut x = 0.0f32;
        let mut y = 0.0f32;
        let mut z = 0.0f32;
        for i in 0..41 {
            x += self.values[i] * weighting::WX_D65_2_10[i];
            y += self.values[i] * weighting::WY_D65_2_10[i];
            z += self.values[i] * weighting::WZ_D65_2_10[i];
        }
        let scale = 100.0 / weighting::SUM_WY_D65_2_10;
        XYZ {
            x: x * scale,
            y: y * scale,
            z: z * scale,
        }
    }

    fn to_xyz_emissive_2(&self) -> XYZ {
        const STEP: f32 = 10.0;
        let mut x = 0.0f32;
        let mut y = 0.0f32;
        let mut z = 0.0f32;
        for i in 0..41 {
            x += self.values[i] * X_BAR_2[i];
            y += self.values[i] * Y_BAR_2[i];
            z += self.values[i] * Z_BAR_2[i];
        }
        XYZ {
            x: x * STEP,
            y: y * STEP,
            z: z * STEP,
        }
    }

    fn to_xyz_10(&self) -> XYZ {
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
}

impl QualityMetrics for SpectralData {
    fn calculate_cri(&self) -> Option<f32> {
        let (ra, _) = crate::colorimetry::metrics::calculate_cri(self);
        Some(ra)
    }
}

impl SpectralData {
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
        let scale = 100.0 / sum_wy;
        XYZ {
            x: x * scale,
            y: y * scale,
            z: z * scale,
        }
    }

    pub fn resample(&self, start: f32, end: f32, step: f32) -> Self {
        let mut new_values = Vec::new();
        let mut current_wl = start;
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
            let idx = (i + 2) as usize;

            if idx < 2 || idx + 3 >= padded_values.len() {
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

    fn sprague_interpolate(x: f32, y: &[f32; 6]) -> f32 {
        let x2 = x * x;
        let x3 = x2 * x;
        let x4 = x3 * x;
        let x5 = x4 * x;
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

    pub fn calculate_k(illuminant_spd: &[f32], y_bar: &[f32], step: f32) -> f32 {
        let sum_s_y: f32 = illuminant_spd
            .iter()
            .zip(y_bar.iter())
            .map(|(s, yb)| s * yb)
            .sum();
        100.0 / (sum_s_y * step)
    }

    pub fn to_result(&self) -> MeasurementResult {
        let target_illuminant = Illuminant::D50;
        let observer = Observer::CIE1931_2;
        let xyz = if self.mode == MeasurementMode::Reflective {
            self.to_xyz_ext(target_illuminant, observer)
        } else {
            self.to_xyz_ext(Illuminant::D65, observer)
        };
        let wp = target_illuminant.get_white_point(observer);
        let lab = if self.mode == MeasurementMode::Reflective {
            xyz.to_lab(wp)
        } else {
            let adapted_xyz = crate::colorimetry::chromatic_adaptation::bradford_adapt(
                xyz,
                Illuminant::D65.get_white_point(observer),
                wp,
            );
            adapted_xyz.to_lab(wp)
        };
        let srgb_xyz = if self.mode == MeasurementMode::Reflective {
            crate::colorimetry::chromatic_adaptation::bradford_adapt(
                xyz,
                wp,
                Illuminant::D65.get_white_point(observer),
            )
        } else {
            xyz
        };
        let (r, g, b) = srgb_xyz.to_srgb();
        let rgb_float = (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0);
        let cct = xyz.to_cct();
        let cri = self.calculate_cri();
        MeasurementResult {
            spectrum: self.clone(),
            xyz,
            lab,
            rgb: rgb_float,
            cct,
            cri,
        }
    }
}

impl XYZ {
    pub fn to_chromaticity(&self) -> (f32, f32) {
        let sum = self.x + self.y + self.z;
        if sum < 1e-6 {
            return (0.3127, 0.3290);
        }
        (self.x / sum, self.y / sum)
    }
    pub fn to_cct(&self) -> f32 {
        let (x, y) = self.to_chromaticity();
        let n = (x - 0.3320) / (0.1858 - y);
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
