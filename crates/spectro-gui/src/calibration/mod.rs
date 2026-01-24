use spectro_rs::colorimetry::curves::{CalibrationSession, VideoCal};
use spectro_rs::colorimetry::{XYZ, illuminant};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CalibrationTarget {
    #[default]
    None,
    White,
    Black,
    Ramp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CalibrationFlowStep {
    #[default]
    Intro,
    Setup,
    Measure,
    Result,
}

#[derive(Default)]
pub struct CalibrationReadings {
    pub white_point: Option<XYZ>,
    pub black_point: Option<XYZ>,
    pub last_measured: Option<XYZ>,
}

pub struct CalibrationConfig {
    pub target_gamma: f32,
    pub patch_count: usize,
    pub auto_advance: bool,
}

impl Default for CalibrationConfig {
    fn default() -> Self {
        Self {
            target_gamma: 2.2,
            patch_count: 17,
            auto_advance: false,
        }
    }
}

/// The core logic controller for the display calibration workflow.
///
/// # Architecture (SSOT & SRP)
/// - **SRP**: This struct is responsible ONLY for the business logic and state management
///   of the calibration process. It does not handle rendering or specific UI events.
/// - **SSOT**: This struct serves as the Single Source of Truth for the calibration state.
///   The View layer should strictly observe this state and delegate actions to /// Commands that the Manager requests from the infrastructure (App/Hardware).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum ManagerRequest {
    #[default]
    None,
    Measure(spectro_rs::MeasurementMode),
    TestSensor,
}

pub struct DisplayCalibrationManager {
    pub step: CalibrationFlowStep,
    pub config: CalibrationConfig,
    pub readings: CalibrationReadings,

    // Internal Logic Session
    pub session: Option<CalibrationSession>,
    pub result: Option<VideoCal>,

    // Runtime State
    pub current_target: CalibrationTarget,
    pub is_measuring: bool,

    // User Interaction State
    pub waiting_for_user_position: bool,
    pub auto_start_timer: Option<f32>,

    // Request Queue
    pub pending_request: ManagerRequest,
}

impl Default for DisplayCalibrationManager {
    fn default() -> Self {
        Self {
            step: CalibrationFlowStep::Intro,
            config: CalibrationConfig::default(),
            readings: CalibrationReadings {
                white_point: None,
                black_point: None,
                last_measured: None,
            },
            session: None,
            result: None,
            current_target: CalibrationTarget::None,
            is_measuring: false,
            waiting_for_user_position: false,
            auto_start_timer: None,
            pending_request: ManagerRequest::None,
        }
    }
}

impl DisplayCalibrationManager {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Primary Interaction: User confirms sensor placement.
    pub fn confirm_user_position(&mut self) {
        if self.waiting_for_user_position {
            self.waiting_for_user_position = false;
            self.auto_start_timer = None;
            // Immediate transition: If we were waiting to measure, now we request it.
            if self.is_measuring {
                self.pending_request =
                    ManagerRequest::Measure(spectro_rs::MeasurementMode::Emissive); // Or Reflective based on config? Defaulting Emissive for Display Cal.
            }
        }
    }

    /// Primary Interaction: User wants to measure a specific target.
    pub fn prepare_measurement(&mut self, target: CalibrationTarget) {
        self.current_target = target;
        self.is_measuring = true;
        self.waiting_for_user_position = true;
        self.auto_start_timer = None;
        self.pending_request = ManagerRequest::None;
    }

    pub fn start_session(&mut self) {
        // Initialize the core session logic
        self.session = Some(CalibrationSession::new(
            self.config.target_gamma,
            illuminant::D65,
            self.config.patch_count,
        ));
        self.current_target = CalibrationTarget::Ramp;
        self.step = CalibrationFlowStep::Measure;

        self.is_measuring = true;

        // UX: Default to auto-advance, but WAIT for the first placement.
        self.config.auto_advance = true;
        self.waiting_for_user_position = true;
        self.auto_start_timer = None;
        self.pending_request = ManagerRequest::None;
    }

    pub fn can_start_characterization(&self) -> bool {
        self.readings.white_point.is_some()
    }

    pub fn get_status_text(&self) -> String {
        match self.step {
            CalibrationFlowStep::Intro => "Ready to begin".to_string(),
            CalibrationFlowStep::Setup => "Setup hardware".to_string(),
            CalibrationFlowStep::Measure => {
                if let Some((idx, total)) = self.get_progress() {
                    if self.waiting_for_user_position {
                        if let Some(t) = self.auto_start_timer {
                            format!("Starting in {:.1}s...", t)
                        } else {
                            "Waiting for start...".to_string()
                        }
                    } else {
                        format!("Measuring Patch {}/{}", idx + 1, total)
                    }
                } else {
                    "Initializing...".to_string()
                }
            }
            CalibrationFlowStep::Result => "Complete".to_string(),
        }
    }

    pub fn handle_measurement(&mut self, xyz: XYZ) {
        self.readings.last_measured = Some(xyz);

        match self.current_target {
            CalibrationTarget::White => {
                self.readings.white_point = Some(xyz);
                self.is_measuring = false;
                self.current_target = CalibrationTarget::None;
            }
            CalibrationTarget::Black => {
                self.readings.black_point = Some(xyz);
                self.is_measuring = false;
                self.current_target = CalibrationTarget::None;
            }
            CalibrationTarget::Ramp => {
                if let Some(session) = &mut self.session {
                    session.add_measurement(xyz);
                    self.is_measuring = false;

                    if session.is_complete() {
                        self.finish_calibration();
                    } else if self.config.auto_advance {
                        // Continuous FLow: Automatically queue next measurement without waiting
                        self.is_measuring = true;
                        self.waiting_for_user_position = false;
                        self.pending_request =
                            ManagerRequest::Measure(spectro_rs::MeasurementMode::Emissive);
                    }
                }
            }
            CalibrationTarget::None => {
                self.is_measuring = false;
            }
        }
    }

    fn finish_calibration(&mut self) {
        if let Some(session) = &self.session {
            self.result = Some(session.generate_cal());
            self.step = CalibrationFlowStep::Result;
            self.is_measuring = false;
            self.current_target = CalibrationTarget::None;
            self.config.auto_advance = false;
        }
    }

    pub fn get_current_ramp_level(&self) -> Option<f32> {
        self.session.as_ref().and_then(|s| s.current_level())
    }

    pub fn get_progress(&self) -> Option<(usize, usize)> {
        self.session.as_ref().map(|s| s.progress())
    }

    pub fn simulate_step(&mut self) {
        if self.current_target == CalibrationTarget::Ramp && self.session.is_some() {
            let level = self.get_current_ramp_level().unwrap_or(0.0);
            let y = 100.0 * level.powf(2.2);
            let sim_xyz = XYZ {
                x: y * 0.95,
                y,
                z: y * 1.08,
            };
            self.handle_measurement(sim_xyz);
        }
    }

    #[allow(clippy::collapsible_if)]
    /// State Machine Tick
    /// Should be called every frame. Handles timers and request emission.
    pub fn poll(&mut self, dt: f32) -> Option<ManagerRequest> {
        // 1. Update Timers
        if self.waiting_for_user_position {
            if let Some(timer) = &mut self.auto_start_timer {
                *timer -= dt;
                if *timer <= 0.0 {
                    self.confirm_user_position(); // Auto-confirm
                }
            }
        }

        // 2. Emit Requests
        // Separation of concerns: Manager decides WHEN to request.
        let req = self.pending_request.clone();
        if req != ManagerRequest::None {
            self.pending_request = ManagerRequest::None; // Consume
            return Some(req);
        }

        None
    }
}
