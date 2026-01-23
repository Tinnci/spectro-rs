use crate::shared::MeasurementEntry;
use spectro_rs::MeasurementMode;
use spectro_rs::colorimetry::Lab;
use spectro_rs::spectrum::MeasurementResult;
use spectro_rs::tm30::TM30Metrics;

/// Manages the application's measurement history and current state.
///
/// This struct encapsulates the logic for:
/// - Storing measurement history (with a limit)
/// - Caching the latest "live" measurement
/// - Providing the "active" result for the UI (which might be a history item)
/// - Maintaining the reference color for Delta E calculations
pub struct AppState {
    /// Full history of measurements
    pub history: Vec<MeasurementEntry>,

    /// The most recent measurement received from the device
    pub live_result: Option<MeasurementResult>,
    pub live_tm30: Option<TM30Metrics>,

    /// The currently selected/displayed result.
    /// This is either the `live_result` or a specific entry from `history`.
    pub active_result: Option<MeasurementResult>,
    pub active_tm30: Option<TM30Metrics>,

    /// Currently selected index in history (if any)
    pub selected_history_index: Option<usize>,

    /// Reference color for Delta E comparison
    pub reference_lab: Option<Lab>,
    pub delta_e_tolerance: f32,

    /// Input buffer for manual reference entry
    pub ref_input_l: f32,
    pub ref_input_a: f32,
    pub ref_input_b: f32,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            history: Vec::new(),
            live_result: None,
            live_tm30: None,
            active_result: None,
            active_tm30: None,
            selected_history_index: None,
            reference_lab: None,
            delta_e_tolerance: 2.0,
            ref_input_l: 50.0,
            ref_input_a: 0.0,
            ref_input_b: 0.0,
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new measurement to history.
    pub fn add_measurement(
        &mut self,
        result: MeasurementResult,
        tm30: Option<TM30Metrics>,
        mode: MeasurementMode,
    ) {
        let lab = result.lab;
        let delta_e = self
            .reference_lab
            .as_ref()
            .map(|ref_lab| lab.delta_e_2000(ref_lab));

        let entry = MeasurementEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            mode,
            result: result.clone(),
            tm30: tm30.clone(),
            delta_e,
        };

        // Insert at top
        self.history.insert(0, entry);

        // Adjust selection index if needed
        if let Some(ref mut idx) = self.selected_history_index {
            *idx += 1;
        }

        // Limit history size
        if self.history.len() > 50 {
            self.history.pop();
            // Deselect if we popped the selected item
            if self.selected_history_index == Some(50) {
                self.selected_history_index = None;
            }
        }

        // Update live result
        self.live_result = Some(result.clone());
        self.live_tm30 = tm30.clone();

        // Auto-switch active view to live result (unless user locked history view?)
        // Currently standard behavior is to show latest.
        self.active_result = Some(result);
        self.active_tm30 = tm30;
        self.selected_history_index = None;
    }

    /// Select a specific history entry to view.
    pub fn select_history_entry(&mut self, index: usize) {
        if let Some(entry) = self.history.get(index) {
            self.active_result = Some(entry.result.clone());
            self.active_tm30 = entry.tm30.clone();
            self.selected_history_index = Some(index);
        }
    }

    /// Reset view to the live measurement.
    pub fn view_live(&mut self) {
        self.active_result = self.live_result.clone();
        self.active_tm30 = self.live_tm30.clone();
        self.selected_history_index = None;
    }

    /// Get the Lab value of the currently active result.
    pub fn current_lab(&self) -> Option<Lab> {
        self.active_result.as_ref().map(|res| res.lab)
    }

    /// Clear all history.
    pub fn clear_history(&mut self) {
        self.history.clear();
        self.selected_history_index = None;
        // We keep live_result valid
        if self.live_result.is_some() {
            self.view_live();
        }
    }

    /// Remove a specific entry.
    pub fn remove_entry(&mut self, idx: usize) {
        if self.selected_history_index == Some(idx) {
            self.view_live();
        } else if let Some(ref mut selected) = self.selected_history_index
            && *selected > idx
        {
            *selected -= 1;
        }

        if idx < self.history.len() {
            self.history.remove(idx);
        }
    }
}
