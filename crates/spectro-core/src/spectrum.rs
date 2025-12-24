use crate::colorimetry::{XYZ, X_BAR_10, X_BAR_2, Y_BAR_10, Y_BAR_2, Z_BAR_10, Z_BAR_2};
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

    /// Convert to XYZ using the standard 2-degree observer.
    pub fn to_xyz(&self) -> XYZ {
        self.to_xyz_2()
    }

    /// Convert to XYZ using the 2-degree observer (CIE 1931).
    pub fn to_xyz_2(&self) -> XYZ {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut z = 0.0;

        for i in 0..36 {
            x += self.values[i] * X_BAR_2[i];
            y += self.values[i] * Y_BAR_2[i];
            z += self.values[i] * Z_BAR_2[i];
        }

        // Normalization factor for Y=100.
        let k = 100.0 / 10.6821;

        XYZ {
            x: x * k,
            y: y * k,
            z: z * k,
        }
    }

    /// Convert to XYZ using the 10-degree observer (CIE 1964).
    pub fn to_xyz_10(&self) -> XYZ {
        let mut x = 0.0;
        let mut y = 0.0;
        let mut z = 0.0;

        for i in 0..36 {
            x += self.values[i] * X_BAR_10[i];
            y += self.values[i] * Y_BAR_10[i];
            z += self.values[i] * Z_BAR_10[i];
        }

        // Normalization factor for 10-degree observer.
        let k = 100.0 / 11.2319;

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
