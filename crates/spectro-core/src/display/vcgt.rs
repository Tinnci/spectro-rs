#[cfg(target_os = "macos")]
#[path = "vcgt_macos.rs"]
pub mod macos;

#[cfg(target_os = "macos")]
pub use macos::VcgtController;

/// Interface for controlling Video Card Gamma Tables (VCGT).
pub trait GammaController {
    /// Load a 1D LUT into the display hardware.
    ///
    /// Tables should be normalized 0.0 - 1.0.
    fn set_gamma_tables(&self, red: &[f32], green: &[f32], blue: &[f32]) -> Result<(), String>;

    /// Reset tables to linear identity.
    fn reset_gamma(&self) -> Result<(), String> {
        // Default linear table
        let linear: Vec<f32> = (0..256).map(|i| i as f32 / 255.0).collect();
        self.set_gamma_tables(&linear, &linear, &linear)
    }
}

// Fallback
#[cfg(not(target_os = "macos"))]
pub struct VcgtController;

#[cfg(not(target_os = "macos"))]
impl VcgtController {
    pub fn new() -> Result<Self, String> {
        Err("VCGT not supported on this OS".into())
    }
}

#[cfg(not(target_os = "macos"))]
impl GammaController for VcgtController {
    fn set_gamma_tables(&self, _r: &[f32], _g: &[f32], _b: &[f32]) -> Result<(), String> {
        Ok(())
    }
}
