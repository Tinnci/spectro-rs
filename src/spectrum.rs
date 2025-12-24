use crate::colorimetry::{XYZ, X_BAR, Y_BAR, Z_BAR};
use crate::WAVELENGTHS;

#[derive(Debug, Clone)]
pub struct SpectralData {
    pub wavelengths: Vec<f32>,
    pub values: Vec<f32>,
}

impl SpectralData {
    pub fn new(values: Vec<f32>) -> Self {
        Self {
            wavelengths: WAVELENGTHS.to_vec(),
            values,
        }
    }

    pub fn to_xyz(&self) -> XYZ {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut z = 0.0;

        for i in 0..36 {
            x += self.values[i] * X_BAR[i];
            y += self.values[i] * Y_BAR[i];
            z += self.values[i] * Z_BAR[i];
        }

        // Integration with 10nm step.
        // We typically normalize such that a flat 1.0 spectrum yields Y ≈ 100.
        // Sum of Y_BAR is ~10.68. Multiply by 10 (step) -> ~106.8.
        // To get 100, we use a normalization factor k = 100 / 106.82 ≈ 0.936
        let k = 100.0 / 10.6821;

        XYZ {
            x: x * k,
            y: y * k,
            z: z * k,
        }
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
        writeln!(f, "Spectral Data (380nm - 730nm):")?;
        for (w, v) in self.wavelengths.iter().zip(self.values.iter()) {
            writeln!(f, "  {:.0}nm: {:.6}", w, v)?;
        }
        Ok(())
    }
}
