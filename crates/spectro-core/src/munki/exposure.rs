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
    pub fn new(min_time_sec: f64) -> Self {
        Self {
            // Target around 75% of full scale (65535)
            // This leaves headroom for specular highlights or noise while optimizing SNR.
            target_counts: 49000.0,

            // Accept anything between 20k and 60k as "good enough" to avoid excessive retries.
            // If < 20k, SNR suffers. If > 60k, risk of saturation.
            min_counts: 20000.0,
            max_counts: 60000.0,

            min_time_sec,
            max_time_sec: 6.0, // Cap at 6 seconds to prevent extremely long hangs
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
        if retry_idx >= self.max_retries {
            return ExposureAction::MaxRetriesReached;
        }

        let peak = peak_value as f64;

        // Check for saturation (near 16-bit max)
        if peak > 65000.0 {
            // Saturated. Cut time in half or to min.
            let new_time = (current_time * 0.5).max(self.min_time_sec);
            return ExposureAction::Retry(new_time);
        }

        // Check if within acceptable range
        if peak >= self.min_counts && peak <= self.max_counts {
            return ExposureAction::Success;
        }

        // Check for saturation plateau or rate drop (non-linearity)
        // Correct physics: Counts should be proportional to Time. Rate = Counts/Time should be constant.
        // If Rate drops significantly, we are hitting saturation (soft or hard).
        let current_rate = peak / current_time;
        if let (Some(l_peak), Some(l_time)) = (self.last_peak, self.last_time) {
            let last_rate = l_peak / l_time;

            // If the rate dropped by more than 15%, the last reading was better (more linear).
            // Example: 4100/0.007 = 585k. 12271/0.08 = 153k. Rate dropped to 26%. UNDO!
            if current_rate < last_rate * 0.85 {
                return ExposureAction::Undo;
            }

            // Also check for Hard Plateau (Peak not moving but time is)
            let time_growth = current_time / l_time;
            let signal_growth = peak / l_peak;
            if time_growth > 1.2 && signal_growth < 1.05 && peak < self.target_counts {
                return ExposureAction::Undo;
            }
        }

        // Avoid division by zero
        let safe_peak = peak.max(100.0);

        // Calculate scaling factor
        let factor = self.target_counts / safe_peak;
        let mut new_time = current_time * factor;

        // Clamp to limits
        new_time = new_time.clamp(self.min_time_sec, self.max_time_sec);

        // If the change is very small (e.g. within 5%), it's not worth retrying
        // unless we are way off target.
        if (new_time - current_time).abs() / current_time < 0.05 {
            // We are close enough to the best we can do given limits
            return ExposureAction::Success;
        }

        // If we hit the rail (min or max) and we were already there, stop.
        if (new_time == self.min_time_sec && current_time == self.min_time_sec)
            || (new_time == self.max_time_sec && current_time == self.max_time_sec)
        {
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
