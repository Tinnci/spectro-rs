/// Theme management system for spectro-gui.
/// Supports light/dark mode switching with persistent storage.
use egui::{Color32, Visuals};
use serde::{Deserialize, Serialize};

/// Available theme modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ThemeMode {
    Light,
    #[default]
    Dark,
    Auto, // System preference (future)
}

impl ThemeMode {
    pub fn to_visuals(self) -> Visuals {
        match self {
            ThemeMode::Light => create_light_theme(),
            ThemeMode::Dark => create_dark_theme(),
            ThemeMode::Auto => create_dark_theme(), // Default to dark for now
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            ThemeMode::Light => "🌞 Light",
            ThemeMode::Dark => "🌙 Dark",
            ThemeMode::Auto => "🔄 Auto",
        }
    }

    pub fn next(&self) -> Self {
        match self {
            ThemeMode::Light => ThemeMode::Dark,
            ThemeMode::Dark => ThemeMode::Light,
            ThemeMode::Auto => ThemeMode::Light,
        }
    }
}

/// Create light theme for spectro-gui
fn create_light_theme() -> Visuals {
    let mut visuals = Visuals::light();

    // Customize for spectro measurement context
    visuals.override_text_color = Some(Color32::from_rgb(40, 40, 40));

    // Window styling
    visuals.window_fill = Color32::from_rgb(250, 250, 250);
    visuals.window_stroke.color = Color32::from_rgb(200, 200, 200);

    // Panel styling
    visuals.panel_fill = Color32::from_rgb(245, 245, 245);

    // Button styling
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(230, 230, 230);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(220, 220, 220);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(210, 210, 210);
    visuals.widgets.active.bg_fill = Color32::from_rgb(100, 180, 255);

    visuals
}

/// Create dark theme for spectro-gui (optimized for measurement environment)
fn create_dark_theme() -> Visuals {
    let mut visuals = Visuals::dark();

    // Professional dark color scheme for spectrophotometry
    visuals.window_fill = Color32::from_rgb(32, 32, 32);
    visuals.window_stroke.color = Color32::from_rgb(80, 80, 80);

    // Panel with subtle tint
    visuals.panel_fill = Color32::from_rgb(40, 40, 40);

    // Button colors
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(60, 60, 60);
    visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(50, 50, 50);

    visuals.widgets.hovered.bg_fill = Color32::from_rgb(80, 80, 80);
    visuals.widgets.active.bg_fill = Color32::from_rgb(100, 150, 220);

    // Text
    visuals.override_text_color = Some(Color32::from_rgb(240, 240, 240));

    visuals
}

// ============================================================================
// Theme-Aware Color Helpers
// ============================================================================

/// Get success color (green) that works on both themes
pub fn success_color(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        Color32::from_rgb(50, 205, 50) // Lime green on dark
    } else {
        Color32::from_rgb(34, 139, 34) // Forest green on light
    }
}

/// Get warning/highlight color (yellow/orange) that works on both themes
#[expect(dead_code, reason = "Utility color for planned UI highlights")]
pub fn highlight_color(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        Color32::from_rgb(255, 200, 50) // Golden yellow on dark
    } else {
        Color32::from_rgb(200, 120, 0) // Dark orange on light
    }
}

/// Get line/stroke color for plots that adapts to theme
pub fn plot_line_color(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        Color32::from_rgb(200, 200, 200)
    } else {
        Color32::from_rgb(60, 60, 60)
    }
}

/// Get a contrasting color for graphical elements (dial center, etc.)
pub fn contrast_fill_color(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        Color32::WHITE
    } else {
        Color32::from_rgb(60, 60, 60)
    }
}

/// Get panel background color with proper contrast
pub fn panel_bg_color(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        Color32::from_rgb(22, 22, 30)
    } else {
        Color32::from_rgb(245, 245, 248)
    }
}

/// Get secondary/darker panel background color
pub fn panel_bg_dark_color(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        Color32::from_rgb(18, 18, 24)
    } else {
        Color32::from_rgb(235, 235, 240)
    }
}

/// Get info panel background color (for metric displays)
pub fn info_panel_color(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        Color32::from_rgb(28, 28, 36)
    } else {
        Color32::from_rgb(250, 250, 252)
    }
}

/// Get border/stroke color for UI elements
pub fn border_color(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        Color32::from_rgb(60, 60, 80)
    } else {
        Color32::from_rgb(180, 180, 190)
    }
}

/// Get muted/secondary text color
pub fn muted_text_color(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        Color32::GRAY
    } else {
        Color32::from_rgb(100, 100, 110)
    }
}

/// Get error/danger color
pub fn error_color(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        Color32::from_rgb(255, 100, 100)
    } else {
        Color32::from_rgb(200, 50, 50)
    }
}

/// Get warning color (yellow/amber)
pub fn warning_color(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        Color32::YELLOW
    } else {
        Color32::from_rgb(180, 130, 0)
    }
}

/// Get connected indicator color (green dot)
#[expect(
    dead_code,
    reason = "Indicator color for future device connection status UI"
)]
pub fn connected_color(_visuals: &Visuals) -> Color32 {
    Color32::from_rgb(50, 205, 50) // Lime green visible on both
}

/// Get disconnected indicator color (red dot)
pub fn disconnected_color(_visuals: &Visuals) -> Color32 {
    Color32::from_rgb(255, 100, 100)
}

/// Get color for overlay shadows (dark mode uses black, light mode uses subtle gray)
pub fn overlay_shadow_color(visuals: &Visuals) -> Color32 {
    if visuals.dark_mode {
        Color32::from_rgba_unmultiplied(0, 0, 0, 80)
    } else {
        Color32::from_rgba_unmultiplied(0, 0, 0, 30)
    }
}

/// Layout constants and spacing configuration (Single Source of Truth)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub window_min_width: f32,
    pub window_min_height: f32,
    pub history_min_width: f32,
    pub history_default_width: f32,
    pub inspector_min_width: f32,
    pub inspector_default_width: f32,
    pub inspector_max_width: f32,
    pub bento_min_width: f32,
    pub spacing: f32,
}

impl Default for LayoutConfig {
    fn default() -> Self {
        Self {
            window_min_width: 450.0,
            window_min_height: 500.0,
            history_min_width: 120.0,
            history_default_width: 180.0,
            inspector_min_width: 160.0,
            inspector_default_width: 260.0,
            inspector_max_width: 350.0,
            bento_min_width: 150.0,
            spacing: 12.0,
        }
    }
}

/// Theme configuration with persistence
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub mode: ThemeMode,
    #[serde(default)]
    pub language: crate::i18n::Language,
    #[serde(default)]
    pub layout: LayoutConfig,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        ThemeConfig {
            mode: ThemeMode::Dark,
            language: crate::i18n::Language::Auto,
            layout: LayoutConfig::default(),
        }
    }
}

impl ThemeConfig {
    /// Load theme from config file
    pub fn load_or_default(config_path: &str) -> Self {
        std::fs::read_to_string(config_path)
            .ok()
            .and_then(|content| serde_json::from_str::<ThemeConfig>(&content).ok())
            .unwrap_or_default()
    }

    /// Save theme to config file
    pub fn save(&self, config_path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(self)?;
        let _ = std::fs::create_dir_all(
            std::path::Path::new(config_path)
                .parent()
                .unwrap_or(std::path::Path::new(".")),
        );
        std::fs::write(config_path, json)?;
        Ok(())
    }

    /// Apply all theme settings to egui::Context
    pub fn apply_to_ctx(&self, ctx: &egui::Context) {
        // Apply Visuals
        ctx.set_visuals(self.to_visuals());

        // Apply Spacing
        ctx.style_mut(|style| {
            style.spacing.item_spacing = egui::vec2(self.layout.spacing, self.layout.spacing);
            style.spacing.window_margin = egui::Margin::same(self.layout.spacing);
        });

        // Load comprehensive emoji font to eliminate tofu
        load_comprehensive_emoji_fonts(ctx);
    }

    /// Get current visuals
    pub fn to_visuals(&self) -> Visuals {
        self.mode.to_visuals()
    }
}

/// Load comprehensive emoji font to eliminate tofu (□) characters
fn load_comprehensive_emoji_fonts(ctx: &egui::Context) {
    use std::sync::atomic::{AtomicBool, Ordering};

    // Ensure fonts are only loaded once
    static FONTS_LOADED: AtomicBool = AtomicBool::new(false);

    if FONTS_LOADED.swap(true, Ordering::Relaxed) {
        return; // Already loaded
    }

    let mut fonts = egui::FontDefinitions::default();

    // Embed the complete Noto Emoji font
    fonts.font_data.insert(
        "NotoEmoji-Complete".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/NotoEmoji-Regular.ttf")),
    );

    // Insert emoji font at high priority in all font families (after default UI fonts)
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(1, "NotoEmoji-Complete".to_owned());

    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .insert(1, "NotoEmoji-Complete".to_owned());

    ctx.set_fonts(fonts);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_mode_cycling() {
        let mut mode = ThemeMode::Light;
        mode = mode.next();
        assert_eq!(mode, ThemeMode::Dark);
        mode = mode.next();
        assert_eq!(mode, ThemeMode::Light);
    }

    #[test]
    fn test_theme_persistence() {
        let config = ThemeConfig {
            mode: ThemeMode::Light,
            language: crate::i18n::Language::Auto,
            layout: LayoutConfig::default(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ThemeConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.mode, deserialized.mode);
    }
}
