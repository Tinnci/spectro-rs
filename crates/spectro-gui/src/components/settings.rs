use crate::t;
use crate::theme::ThemeConfig;
use eframe::egui;
use spectro_rs::{Illuminant, Observer};

pub struct SettingsContext<'a> {
    pub show: &'a mut bool,
    pub show_debug: &'a mut bool,
    pub selected_illuminant: &'a mut Illuminant,
    pub selected_observer: &'a mut Observer,
    pub theme_config: &'a mut ThemeConfig,
    pub dirty: &'a mut bool,
}

pub struct DebugSettingsContext<'a> {
    pub show: &'a mut bool,
    pub theme_config: &'a mut ThemeConfig,
    pub dirty: &'a mut bool,
}

pub fn render_settings_window(ctx: &egui::Context, ui_ctx: &mut SettingsContext) {
    if !*ui_ctx.show {
        return;
    }

    egui::Window::new(format!("⚙ {}", t!("gui-settings")))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.heading(t!("gui-colorimetry-standards"));
            ui.add_space(10.0);

            egui::Grid::new("settings_grid")
                .num_columns(2)
                .spacing([20.0, 10.0])
                .show(ui, |ui| {
                    ui.label(t!("gui-illuminant"));
                    egui::ComboBox::from_id_salt("illuminant_selector_settings")
                        .selected_text(format!("{:?}", ui_ctx.selected_illuminant))
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_value(
                                    ui_ctx.selected_illuminant,
                                    Illuminant::D65,
                                    "D65 (Daylight, sRGB)",
                                )
                                .changed()
                            {
                                *ui_ctx.dirty = true;
                            }
                            if ui
                                .selectable_value(
                                    ui_ctx.selected_illuminant,
                                    Illuminant::D50,
                                    "D50 (Print Industry)",
                                )
                                .changed()
                            {
                                *ui_ctx.dirty = true;
                            }
                            if ui
                                .selectable_value(
                                    ui_ctx.selected_illuminant,
                                    Illuminant::A,
                                    "A (Tungsten 2856K)",
                                )
                                .changed()
                            {
                                *ui_ctx.dirty = true;
                            }
                            if ui
                                .selectable_value(
                                    ui_ctx.selected_illuminant,
                                    Illuminant::F2,
                                    "F2 (Cool White Fluorescent)",
                                )
                                .changed()
                            {
                                *ui_ctx.dirty = true;
                            }
                            if ui
                                .selectable_value(
                                    ui_ctx.selected_illuminant,
                                    Illuminant::F7,
                                    "F7 (Daylight Fluorescent)",
                                )
                                .changed()
                            {
                                *ui_ctx.dirty = true;
                            }
                            if ui
                                .selectable_value(
                                    ui_ctx.selected_illuminant,
                                    Illuminant::F11,
                                    "F11 (TL84)",
                                )
                                .changed()
                            {
                                *ui_ctx.dirty = true;
                            }
                        });
                    ui.end_row();

                    ui.label(t!("gui-observer"));
                    egui::ComboBox::from_id_salt("observer_selector_settings")
                        .selected_text(match ui_ctx.selected_observer {
                            Observer::CIE1931_2 => "2° (Standard)",
                            Observer::CIE1964_10 => "10° (Supplementary)",
                        })
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_value(
                                    ui_ctx.selected_observer,
                                    Observer::CIE1931_2,
                                    "2° (CIE 1931 Standard)",
                                )
                                .changed()
                            {
                                *ui_ctx.dirty = true;
                            }
                            if ui
                                .selectable_value(
                                    ui_ctx.selected_observer,
                                    Observer::CIE1964_10,
                                    "10° (CIE 1964 Large Field)",
                                )
                                .changed()
                            {
                                *ui_ctx.dirty = true;
                            }
                        });
                    ui.end_row();
                });

            ui.add_space(20.0);
            ui.separator();
            ui.heading(t!("gui-language-title"));
            ui.add_space(10.0);

            egui::Grid::new("language_settings_grid")
                .num_columns(2)
                .spacing([20.0, 10.0])
                .show(ui, |ui| {
                    ui.label(t!("gui-language"));
                    let old_lang = ui_ctx.theme_config.language;
                    egui::ComboBox::from_id_salt("language_selector")
                        .selected_text(ui_ctx.theme_config.language.label())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut ui_ctx.theme_config.language,
                                crate::i18n::Language::Auto,
                                crate::i18n::Language::Auto.label(),
                            );
                            ui.selectable_value(
                                &mut ui_ctx.theme_config.language,
                                crate::i18n::Language::EnUS,
                                crate::i18n::Language::EnUS.label(),
                            );
                            ui.selectable_value(
                                &mut ui_ctx.theme_config.language,
                                crate::i18n::Language::ZhCN,
                                crate::i18n::Language::ZhCN.label(),
                            );
                        });
                    // Apply language change immediately
                    if ui_ctx.theme_config.language != old_lang {
                        crate::i18n::init(ui_ctx.theme_config.language);
                        let _ = ui_ctx.theme_config.save("spectro_theme.json");
                    }
                    ui.end_row();
                });

            ui.add_space(24.0);
            ui.separator();
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                if ui.button(t!("gui-close")).clicked() {
                    *ui_ctx.show = false;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button("🛠 Debug Settings")
                        .on_hover_text("Technical and UI layout settings")
                        .clicked()
                    {
                        *ui_ctx.show_debug = true;
                    }
                });
            });
        });
}

pub fn render_debug_settings_window(ctx: &egui::Context, ui_ctx: &mut DebugSettingsContext) {
    if !*ui_ctx.show {
        return;
    }

    egui::Window::new("🛠 Debug Settings")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.heading("📏 UI Layout");
            ui.add_space(10.0);

            egui::Grid::new("layout_settings_grid")
                .num_columns(2)
                .spacing([20.0, 10.0])
                .show(ui, |ui| {
                    ui.label("Item Spacing");
                    if ui
                        .add(
                            egui::DragValue::new(&mut ui_ctx.theme_config.layout.spacing)
                                .speed(0.5)
                                .range(2.0..=32.0),
                        )
                        .clicked()
                        || ui
                            .add(egui::Slider::new(
                                &mut ui_ctx.theme_config.layout.spacing,
                                2.0..=32.0,
                            ))
                            .changed()
                    {
                        *ui_ctx.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Bento Min Width");
                    if ui
                        .add(
                            egui::DragValue::new(&mut ui_ctx.theme_config.layout.bento_min_width)
                                .speed(1.0)
                                .range(100.0..=400.0),
                        )
                        .changed()
                    {
                        *ui_ctx.dirty = true;
                    }
                    ui.end_row();

                    ui.label("History Panel (Min)");
                    if ui
                        .add(
                            egui::DragValue::new(&mut ui_ctx.theme_config.layout.history_min_width)
                                .speed(1.0)
                                .range(150.0..=400.0),
                        )
                        .changed()
                    {
                        *ui_ctx.dirty = true;
                    }
                    ui.end_row();

                    ui.label("Inspector Panel (Min)");
                    if ui
                        .add(
                            egui::DragValue::new(
                                &mut ui_ctx.theme_config.layout.inspector_min_width,
                            )
                            .speed(1.0)
                            .range(200.0..=500.0),
                        )
                        .changed()
                    {
                        *ui_ctx.dirty = true;
                    }
                    ui.end_row();
                });

            ui.add_space(20.0);
            ui.separator();
            ui.heading("🔍 Font Diagnostics");
            ui.add_space(10.0);
            crate::components::font_diag::render_font_diagnostics(ui);

            ui.add_space(24.0);
            ui.separator();
            ui.add_space(12.0);

            ui.horizontal(|ui| {
                if ui.button(t!("gui-close")).clicked() {
                    *ui_ctx.show = false;
                }
            });
        });
}
