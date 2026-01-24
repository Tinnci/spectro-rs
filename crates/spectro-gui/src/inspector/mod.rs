//! Device Inspector panel component for spectro-gui.
//!
//! This module provides the Device Inspector side panel which shows detailed
//! device information, raw sensor data, algorithm details, chromaticity diagram,
//! color quality metrics (TM-30), and trend analysis.
//!
//! # Architecture
//! The inspector is a **passive view component** that receives immutable references
//! to the application state and renders the appropriate UI. It maintains its own
//! tab selection state but does not own any measurement data.

// Modules
pub mod algorithm;
pub mod chromaticity;
pub mod color_quality;
pub mod device_info;
pub mod raw_sensor;
pub mod trend;

use eframe::egui;

use crate::shared::{ExtendedDeviceInfo, MeasurementEntry};
use crate::t;
use spectro_rs::spectrum::MeasurementResult;
use spectro_rs::tm30::TM30Metrics;

// ============================================================================
// Inspector State
// ============================================================================

/// Tabs available in the Device Inspector panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InspectorTab {
    /// Device information and EEPROM calibration data
    #[default]
    DeviceInfo,
    /// Raw spectral sensor values
    RawSensor,
    /// Algorithm and color conversion pipeline details
    Algorithm,
    /// CIE 1931 xy chromaticity diagram
    Chromaticity,
    /// TM-30 color quality metrics
    ColorQuality,
    /// Trend analysis over measurement history
    Trend,
}

/// Device Inspector panel state and rendering logic.
///
/// This component is responsible for rendering the right-side inspector panel
/// in expert mode. It displays detailed technical information about the device
/// and measurements.
pub struct DeviceInspector {
    /// Currently selected tab
    pub tab: InspectorTab,
    /// Whether the inspector is detached in its own window
    pub is_detached: bool,
}

impl Default for DeviceInspector {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceInspector {
    /// Create a new DeviceInspector with default state.
    pub fn new() -> Self {
        Self {
            tab: InspectorTab::default(),
            is_detached: false,
        }
    }

    /// Render the inspector panel content.
    ///
    /// # Arguments
    /// * `ui` - The egui UI context
    /// * `ctx` - Inspector rendering context containing all required data
    /// * `is_visible` - Mutable reference to control panel visibility
    pub fn render(&mut self, ui: &mut egui::Ui, ctx: &InspectorContext, is_visible: &mut bool) {
        // Header with title and close button
        ui.horizontal(|ui| {
            ui.heading(t!("gui-device-inspector"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("⏵").on_hover_text(t!("gui-hide")).clicked() {
                    *is_visible = false;
                }

                let detach_icon = if self.is_detached { "📥" } else { "📤" };
                let detach_text = if self.is_detached {
                    t!("gui-attach")
                } else {
                    t!("gui-detach")
                };
                if ui.button(detach_icon).on_hover_text(detach_text).clicked() {
                    self.is_detached = !self.is_detached;
                }
            });
        });
        ui.add_space(ctx.layout.spacing);

        // Tab bar
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.tab,
                InspectorTab::DeviceInfo,
                format!("📱 {}", t!("gui-device")),
            );
            ui.selectable_value(
                &mut self.tab,
                InspectorTab::RawSensor,
                format!("📈 {}", t!("gui-raw-data")),
            );
            ui.selectable_value(
                &mut self.tab,
                InspectorTab::Algorithm,
                format!("🧮 {}", t!("gui-algorithm")),
            );
            ui.selectable_value(
                &mut self.tab,
                InspectorTab::Chromaticity,
                format!("🎯 {}", t!("gui-xy-diagram")),
            );
            ui.selectable_value(
                &mut self.tab,
                InspectorTab::ColorQuality,
                format!("🌈 {}", t!("gui-color-quality")),
            );
            ui.selectable_value(
                &mut self.tab,
                InspectorTab::Trend,
                format!("📈 {}", t!("gui-trend")),
            );
        });

        ui.separator();

        // Handle empty state for tabs that need data
        let needs_data = matches!(
            self.tab,
            InspectorTab::RawSensor
                | InspectorTab::Algorithm
                | InspectorTab::Chromaticity
                | InspectorTab::ColorQuality
                | InspectorTab::Trend
        );

        if needs_data && ctx.last_result.is_none() && ctx.history.is_empty() {
            self.render_empty_state(ui);
        } else {
            // Render the selected tab
            match self.tab {
                InspectorTab::DeviceInfo => {
                    egui::ScrollArea::vertical().show(ui, |ui| device_info::render(ui, ctx));
                }
                InspectorTab::RawSensor => raw_sensor::render(ui, ctx),
                InspectorTab::Algorithm => {
                    egui::ScrollArea::vertical().show(ui, |ui| algorithm::render(ui, ctx));
                }
                InspectorTab::Chromaticity => chromaticity::render(ui, ctx),
                InspectorTab::ColorQuality => color_quality::render(ui, ctx),
                InspectorTab::Trend => trend::render(ui, ctx),
            }
        }
    }

    /// Render centered empty state message.
    fn render_empty_state(&self, ui: &mut egui::Ui) {
        // Use available space to center the message both vertically and horizontally
        let available = ui.available_size();

        // Allocate the full available space
        ui.allocate_ui_with_layout(
            available,
            egui::Layout::centered_and_justified(egui::Direction::TopDown),
            |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new(t!("gui-no-measurement")).weak());
                });
            },
        );
    }
}

// ============================================================================
// Inspector Context
// ============================================================================

/// Context data required for rendering the Device Inspector.
///
/// This struct bundles all the immutable references needed by the inspector
/// to render its content. Using a context struct keeps the API clean and
/// makes it easy to add new data dependencies.
pub struct InspectorContext<'a> {
    /// Extended device information including EEPROM data
    pub device_info: &'a ExtendedDeviceInfo,
    /// Whether the device is currently connected
    pub is_connected: bool,
    /// Whether the device has been calibrated
    pub is_calibrated: bool,
    /// Currently selected measurement mode
    pub selected_mode: spectro_rs::MeasurementMode,
    /// Most recent measurement result
    pub last_result: Option<&'a MeasurementResult>,
    /// Most recent TM-30 metrics (for emissive measurements)
    pub last_tm30: Option<&'a TM30Metrics>,
    /// Measurement history
    pub history: &'a [MeasurementEntry],
    /// Global layout configuration
    pub layout: &'a crate::theme::LayoutConfig,
}
