//! Physics-based sensor model for ColorMunki.
//!
//! Based on experimental characterization (2026-01-23), this device follows a
//! "Shifted Linear" model rather than an exponential saturation model.
//!
//! Behavior:
//! 1. Dead Zone: t < t_dead -> Output = y_bias + Noise
//! 2. Linear Zone: t >= t_dead -> Output = y_bias + k * (t - t_dead)
//! 3. Hard Clip: Output clamped at y_sat
//!
//! Mathematical Model:
//! k = (y - y_bias) / (t - t_dead)

use crate::Result;

/// Represents the physical parameters of the sensor.
#[derive(Debug, Clone)]
pub struct SensorModel {
    /// The dark current floor (bias offset), e.g., 4112.0
    pub y_bias: f64,
    /// The physical saturation limit, e.g., 12271.0
    pub y_sat: f64,
    /// The dead time before sensor becomes responsive, e.g., 0.007s
    pub t_dead: f64,
}

impl SensorModel {
    /// Create a new sensor model with specific parameters.
    pub fn new(y_bias: f64, y_sat: f64, t_dead: f64) -> Self {
        Self {
            y_bias,
            y_sat,
            t_dead,
        }
    }

    /// Forward Model: Predict sensor reading y given light intensity k and time t.
    ///
    /// y = y_bias + k * (t - t_dead)
    pub fn predict(&self, k: f64, t: f64) -> f64 {
        let t_eff = (t - self.t_dead).max(0.0);
        let predicted = self.y_bias + k * t_eff;
        predicted.min(self.y_sat)
    }

    /// Inverse Model: Solve for true light intensity k given reading y and time t.
    ///
    /// k = (y - y_bias) / (t - t_dead)
    pub fn solve_intensity(&self, y: f64, t: f64) -> Result<f64> {
        let t_eff = (t - self.t_dead).max(0.0);

        // Guard: Dead Zone
        if t_eff < 1e-4 {
            // Signal is unrecoverable in dead zone
            return Ok(0.0);
        }

        // Guard: Saturation
        // If we are at the rail, we cannot know the true intensity (it could be higher)
        if y >= self.y_sat - 10.0 {
            // Return a flag or just the lower bound?
            // For plotting purposes, we calculate the naive lower bound.
        }

        let signal = (y - self.y_bias).max(0.0);
        let k = signal / t_eff;

        Ok(k)
    }

    /// Estimate dead time using intersection of baseline and regression line.
    pub fn estimate_parameters(points: &[(f64, f64)], y_sat_hint: f64) -> Self {
        // 1. Estimate Bias: Average of points t < 0.005
        let dead_zone_points: Vec<_> = points.iter().filter(|(t, _)| *t < 0.006).collect();
        let y_bias = if !dead_zone_points.is_empty() {
            dead_zone_points.iter().map(|(_, y)| *y).sum::<f64>() / dead_zone_points.len() as f64
        } else if let Some(first) = points.first() {
            first.1
        } else {
            0.0
        };

        // 2. Scan for Linear Region
        // Look for points that have statistically significant signal over bias
        let signal_threshold = 200.0;
        let linear_points: Vec<_> = points
            .iter()
            .filter(|(_t, y)| *y > y_bias + signal_threshold && *y < y_sat_hint * 0.95)
            .collect();

        let mut t_dead = 0.007; // Default fallback

        // 3. Robust Linear Regression
        if linear_points.len() >= 3 {
            let n = linear_points.len() as f64;
            let sum_t: f64 = linear_points.iter().map(|(t, _)| *t).sum();
            let sum_y: f64 = linear_points.iter().map(|(_, y)| *y).sum();
            let sum_ty: f64 = linear_points.iter().map(|(t, y)| t * y).sum();
            let sum_tt: f64 = linear_points.iter().map(|(t, _)| t * t).sum();

            let denominator = n * sum_tt - sum_t * sum_t;
            if denominator.abs() > 1e-9 {
                let slope = (n * sum_ty - sum_t * sum_y) / denominator;
                let intercept = (sum_y - slope * sum_t) / n;

                // y = slope * t + intercept
                // At y = y_bias, t = (y_bias - intercept) / slope
                if slope > 0.1 {
                    t_dead = (y_bias - intercept) / slope;
                }
            }
        }

        Self::new(y_bias, y_sat_hint, t_dead)
    }
}

impl From<SensorModel> for crate::persistence::PhysicsConfig {
    fn from(m: SensorModel) -> Self {
        Self {
            y_bias: m.y_bias,
            y_sat: m.y_sat,
            t_dead: m.t_dead,
        }
    }
}

impl From<&SensorModel> for crate::persistence::PhysicsConfig {
    fn from(m: &SensorModel) -> Self {
        Self {
            y_bias: m.y_bias,
            y_sat: m.y_sat,
            t_dead: m.t_dead,
        }
    }
}
