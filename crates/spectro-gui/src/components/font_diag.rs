use eframe::egui;

/// Critical emojis used in the application.
/// Using string slices to avoid 'multi-codepoint char literal' errors with variation selectors.
pub const CRITICAL_EMOJIS: &[&str] = &[
    "🔬", // Inspector
    "📋", // History
    "📈", // Trend
    "🎯", // Calibration/Target
    "⚙",  // Settings
    "🌞", // Light mode
    "🌙", // Dark mode
    "🔄", // Auto/Reset
    "📌", // Reference
    "📄", // Reflective
    "🖥",  // Emissive
    "💡", // Ambient
    "📥", // Attach
    "📤", // Detach
    "✅", // Success
    "⚠",  // Warning
    "❌", // Error
    "📸", // Measuring
];

/// Result of a font support check
pub struct FontDiagnostic {
    pub text: String,
    pub emoji_char: char,
    pub supported: bool,
}

/// Check support for all critical emojis
pub fn check_emoji_support(ctx: &egui::Context) -> Vec<FontDiagnostic> {
    ctx.fonts(|f| {
        CRITICAL_EMOJIS
            .iter()
            .map(|&s| {
                // Get the first char (base emoji)
                let c = s.chars().next().unwrap();
                FontDiagnostic {
                    text: s.to_string(),
                    emoji_char: c,
                    supported: f.has_glyph(&egui::FontId::proportional(14.0), c),
                }
            })
            .collect()
    })
}

/// Helper to render the diagnostic UI
pub fn render_font_diagnostics(ui: &mut egui::Ui) {
    let diagnostics = check_emoji_support(ui.ctx());
    let all_ok = diagnostics.iter().all(|d| d.supported);

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label("Emoji support status:");
            if all_ok {
                ui.colored_label(egui::Color32::from_rgb(100, 255, 100), "✅ All Good");
            } else {
                ui.colored_label(
                    egui::Color32::from_rgb(255, 100, 100),
                    "⚠️ Some Missing (Tofu)",
                );
            }
        });

        ui.add_space(8.0);

        egui::Grid::new("font_diagnostic_grid")
            .num_columns(5)
            .spacing([12.0, 12.0])
            .show(ui, |ui| {
                for (i, d) in diagnostics.iter().enumerate() {
                    let color = if d.supported {
                        egui::Color32::from_rgb(150, 255, 150)
                    } else {
                        egui::Color32::from_rgb(255, 100, 100)
                    };

                    ui.vertical_centered(|ui| {
                        if d.supported {
                            ui.label(egui::RichText::new(&d.text).size(24.0));
                        } else {
                            // Draw a red box for Tofu
                            let (rect, _) = ui
                                .allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
                            ui.painter().rect_stroke(
                                rect,
                                4.0,
                                egui::Stroke::new(1.0, egui::Color32::RED),
                            );
                        }
                        ui.label(
                            egui::RichText::new(format!("U+{:X}", d.emoji_char as u32))
                                .small()
                                .color(color),
                        );
                    });

                    if (i + 1) % 5 == 0 {
                        ui.end_row();
                    }
                }
            });

        if !all_ok {
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                ui.colored_label(egui::Color32::LIGHT_BLUE, "ℹ Tip:");
                ui.label("If you see red boxes, your system might be missing font glyphs.");
            });
            ui.label("Consider installing 'Noto Emoji' or a similar fallback font.");
        }
    });
}
