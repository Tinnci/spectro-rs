//! Auto-exposure logic for the ColorMunki.
//!
//! This module adheres to the Single Responsibility Principle by encapsulating
//! all logic related to calculating optimal integration times. It is pure logic
//! and does not interact with the hardware directly.

/// Outcomes of an exposure adjustment calculation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ExposureAction {
    /// The current reading is satisfactory.
    Success,
    /// The reading is too low or too high; try again with new duration.
    Retry(f64),
    /// Saturation or non-linearity detected, revert to previous result which was better.
    Undo,
    /// Unable to optimize further (e.g., limits reached); use current result.
    MaxRetriesReached,
}

/// Manages the state and logic for auto-exposure calculations.
///
/// It holds the configuration (targets, limits) and deciding logic,
/// independent of the mechanism used to acquire the measurement.
pub struct AutoExposure {
    target_counts: f64,
    min_counts: f64,
    max_counts: f64,
    min_time_sec: f64,
    max_time_sec: f64,
    max_retries: usize,
    last_peak: Option<f64>,
    last_time: Option<f64>,
}

impl AutoExposure {
    /// Create a new auto-exposure calculator with the device's minimum integration time.
    pub fn new(min_time_sec: f64, target_counts: f64, max_counts: f64) -> Self {
        Self {
            target_counts,
            // Accept anything between 85% and 120% of target to force high dynamic range
            min_counts: target_counts * 0.85,
            max_counts,

            min_time_sec,
            max_time_sec: 6.0,
            max_retries: 4,
            last_peak: None,
            last_time: None,
        }
    }

    /// Calculate the next step based on the peak measurement value.
    pub fn calculate_next(
        &mut self,
        current_time: f64,
        peak_value: u16,
        retry_idx: usize,
    ) -> ExposureAction {
        let peak = peak_value as f64;

        if retry_idx >= self.max_retries {
            return ExposureAction::Success; // Force success to avoid getting stuck
        }

        // 1. Check for HARD Saturation (Clipping)
        if peak >= self.max_counts {
            let new_time = (current_time * 0.5).max(self.min_time_sec);
            return ExposureAction::Retry(new_time);
        }

        // 2. Check for Target Range Success
        if peak >= self.min_counts {
            return ExposureAction::Success;
        }

        // 3. Signal is too low, need more time.
        // Simple linear extrapolation: NewTime = CurrTime * (Target / Peak)
        let safe_peak = peak.max(100.0); // Avoid huge multipliers on noise
        let factor = self.target_counts / safe_peak;
        let mut new_time = current_time * factor;

        // Clamp to physical limits
        new_time = new_time.clamp(self.min_time_sec, self.max_time_sec);

        // Optimization: If we are already at max time, stop.
        if current_time >= self.max_time_sec {
            return ExposureAction::Success;
        }

        // Optimization: If change is negligible, stop.
        if (new_time - current_time).abs() < 0.001 {
            return ExposureAction::Success;
        }

        self.last_peak = Some(peak);
        self.last_time = Some(current_time);

        ExposureAction::Retry(new_time)
    }

    pub fn min_time(&self) -> f64 {
        self.min_time_sec
    }
}
