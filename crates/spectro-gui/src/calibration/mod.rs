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
///   The View layer should strictly observe this state and delegate actions to it.
///
/// # 架构说明
/// - **单一职责原则 (SRP)**: 本结构体仅负责校准流程的业务逻辑和状态管理，不处理渲染或UI事件。
/// - **单一数据源 (SSOT)**: 本结构体是校准状态的唯一事实来源，视图层应严格遵循此状态。
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
        }
    }
}

impl DisplayCalibrationManager {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn prepare_measurement(&mut self, target: CalibrationTarget) {
        self.current_target = target;
        self.is_measuring = true;
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
                // If we are in the Ramp mode, we feed the measurement to the session
                if let Some(session) = &mut self.session {
                    session.add_measurement(xyz);
                    self.is_measuring = false;

                    if session.is_complete() {
                        self.finish_calibration();
                    } else if self.config.auto_advance {
                        // If auto-advance is on, immediately prepare for next
                        // But wait, in a real physical world, we need to show the color FIRST,
                        // then wait for the sensor.
                        // "prepare_measurement" typically just sets flags.
                        // The UI cycle will pick this up, show the color, and trigger measurement.
                        // We set is_measuring = true immediately?
                        // No, usually we want to Request a measurement.
                        // The Loop:
                        // 1. Manager says "Target: Ramp(Gray 50)" and "IsMeasuring: true" (waiting for result)
                        // 2. UI renders Gray 50
                        // 3. User (or Auto) clicks "Measure"
                        // 4. HW returns XYZ -> handle_measurement(XYZ)
                        // 5. Manager updates session.
                        // 6. IF Auto-advance: Manager says "Target: Ramp(Gray 60)" AND...
                        //    Wait, we need to trigger the HW to measure.
                        //    The Manager cannot trigger HW directly (it's passive).
                        //    So we just set state ready for next one.

                        // We will set is_measuring = true, implying we WANT a measurement for the *next* patch.
                        // But the previous patch just finished. The session moved to next patch index automatically?
                        // Yes, session.add_measurement() advances the internal cursor.

                        // So we just stay in Measuring state.
                        self.is_measuring = true;
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
            // Turn off auto-advance so we don't loop
            self.config.auto_advance = false;
        }
    }

    // Helper to get current ramp target color (0.0 - 1.0)
    pub fn get_current_ramp_level(&self) -> Option<f32> {
        self.session.as_ref().and_then(|s| s.current_level())
    }

    pub fn get_progress(&self) -> Option<(usize, usize)> {
        self.session.as_ref().map(|s| s.progress())
    }

    // Simulation logic for testing without HW
    pub fn simulate_step(&mut self) {
        if self.current_target == CalibrationTarget::Ramp && self.session.is_some() {
            let level = self.get_current_ramp_level().unwrap_or(0.0);
            // Simulate a generic gamma 2.2 display response
            // L = 100 * level^2.2
            let y = 100.0 * level.powf(2.2);
            let sim_xyz = XYZ {
                x: y * 0.95,
                y,
                z: y * 1.08,
            };

            self.handle_measurement(sim_xyz);
        }
    }
}
