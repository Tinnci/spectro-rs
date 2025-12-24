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
