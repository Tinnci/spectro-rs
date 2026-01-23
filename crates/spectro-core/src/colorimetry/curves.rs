//! Color response curves and Display Calibration logic.
//!
//! This module provides tools for generating, manipulating, and applying
//! color transformation curves (LUTs), specifically for display calibration.
//!
//! Inspired by Argyll CMS and DisplayCAL algorithms.

use crate::colorimetry::XYZ;

/// Represents a 1D Look-Up Table (LUT) for a color channel.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Curve {
    pub values: Vec<f32>,
}

impl Curve {
    pub fn new(values: Vec<f32>) -> Self {
        Self { values }
    }

    /// Create a identity curve (linear mapping 0->0, 1->1).
    pub fn identity(size: usize) -> Self {
        let mut values = Vec::with_capacity(size);
        for i in 0..size {
            values.push(i as f32 / (size - 1) as f32);
        }
        Self { values }
    }

    /// Create a pure power gamma curve (y = x^gamma).
    pub fn gamma(size: usize, g: f32) -> Self {
        let mut values = Vec::with_capacity(size);
        for i in 0..size {
            let x = i as f32 / (size - 1) as f32;
            values.push(x.powf(g));
        }
        Self { values }
    }

    /// Interpolate value from the LUT.
    pub fn interpolate(&self, input: f32) -> f32 {
        let x = input.clamp(0.0, 1.0) * (self.values.len() - 1) as f32;
        let idx = x.floor() as usize;
        let t = x - idx as f32;

        if idx >= self.values.len() - 1 {
            return self.values[self.values.len() - 1];
        }

        let v0 = self.values[idx];
        let v1 = self.values[idx + 1];
        v0 + t * (v1 - v0)
    }

    /// Ensure the curve is monotonically increasing.
    pub fn make_monotonic(&mut self) {
        let mut last_val = 0.0;
        for val in self.values.iter_mut() {
            if *val < last_val {
                *val = last_val;
            }
            last_val = *val;
        }
    }
}

/// A set of three curves (R, G, B) for full color correction.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VideoCal {
    pub r: Curve,
    pub g: Curve,
    pub b: Curve,
}

/// Measured response of a display at a specific drive level.
#[derive(Debug, Clone)]
pub struct DisplayPatch {
    /// Normalized input level (0.0 - 1.0)
    pub input: f32,
    /// Measured XYZ result
    pub xyz: XYZ,
}

/// Core Display Calibration algorithm.
pub struct DisplayCalibrator {
    /// Target characteristics
    pub target_gamma: f32,
    pub target_white_point: XYZ,

    /// Measurements (Gray ramp)
    pub grey_ramp: Vec<DisplayPatch>,
}

impl DisplayCalibrator {
    pub fn new(target_gamma: f32, target_white_point: XYZ) -> Self {
        Self {
            target_gamma,
            target_white_point,
            grey_ramp: Vec::new(),
        }
    }

    pub fn add_measurement(&mut self, input: f32, xyz: XYZ) {
        self.grey_ramp.push(DisplayPatch { input, xyz });
        // Keep sorted by input level
        self.grey_ramp
            .sort_by(|a, b| a.input.partial_cmp(&b.input).unwrap());
    }

    /// Generate calibration curves (Video LUT) to match the target.
    pub fn generate_cal(&self, lut_size: usize) -> VideoCal {
        if self.grey_ramp.len() < 2 {
            // Not enough data, return identity
            return VideoCal {
                r: Curve::identity(lut_size),
                g: Curve::identity(lut_size),
                b: Curve::identity(lut_size),
            };
        }

        // 1. Characterization: Find measured Y (normalized) for each input
        let y_max = self.grey_ramp.last().unwrap().xyz.y;
        let mut measured_inputs = Vec::new();
        let mut measured_y = Vec::new();

        for patch in &self.grey_ramp {
            measured_inputs.push(patch.input);
            measured_y.push(patch.xyz.y / y_max);
        }

        // 2. Generate Curves
        let mut r_vals = Vec::with_capacity(lut_size);
        let mut g_vals = Vec::with_capacity(lut_size);
        let mut b_vals = Vec::with_capacity(lut_size);

        for i in 0..lut_size {
            let n = i as f32 / (lut_size - 1) as f32;

            // Target luminance for this level
            let target_y = n.powf(self.target_gamma);

            // Invert: Find the drive level that produces this target_y
            // Using binary search + linear interpolation on our measured characterization
            let drive_level =
                self.find_drive_level_for_luminance(&measured_inputs, &measured_y, target_y);

            r_vals.push(drive_level);
            g_vals.push(drive_level);
            b_vals.push(drive_level);
        }

        let mut r_curve = Curve::new(r_vals);
        let mut g_curve = Curve::new(g_vals);
        let mut b_curve = Curve::new(b_vals);

        r_curve.make_monotonic();
        g_curve.make_monotonic();
        b_curve.make_monotonic();

        VideoCal {
            r: r_curve,
            g: g_curve,
            b: b_curve,
        }
    }

    fn find_drive_level_for_luminance(&self, inputs: &[f32], values: &[f32], target: f32) -> f32 {
        if target <= values[0] {
            return inputs[0];
        }
        if target >= values[values.len() - 1] {
            return inputs[inputs.len() - 1];
        }

        let idx = match values.binary_search_by(|v| v.partial_cmp(&target).unwrap()) {
            Ok(i) => i,
            Err(i) => i,
        };

        if idx == 0 {
            return inputs[0];
        }
        if idx >= values.len() {
            return inputs[inputs.len() - 1];
        }

        let v0 = values[idx - 1];
        let v1 = values[idx];
        let t = (target - v0) / (v1 - v0);

        let i0 = inputs[idx - 1];
        let i1 = inputs[idx];

        i0 + t * (i1 - i0)
    }
}
