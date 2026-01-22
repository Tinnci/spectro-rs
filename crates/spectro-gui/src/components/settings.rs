use crate::t;
use crate::theme::ThemeConfig;
use eframe::egui;
use spectro_rs::{Illuminant, Observer};

pub struct SettingsContext<'a> {
    pub show: &'a mut bool,
    pub selected_illuminant: &'a mut Illuminant,
    pub selected_observer: &'a mut Observer,
    pub theme_config: &'a mut ThemeConfig,
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
                            ui.selectable_value(
                                ui_ctx.selected_illuminant,
                                Illuminant::D65,
                                "D65 (Daylight, sRGB)",
                            );
                            ui.selectable_value(
                                ui_ctx.selected_illuminant,
                                Illuminant::D50,
                                "D50 (Print Industry)",
                            );
                            ui.selectable_value(
                                ui_ctx.selected_illuminant,
                                Illuminant::A,
                                "A (Tungsten 2856K)",
                            );
                            ui.selectable_value(
                                ui_ctx.selected_illuminant,
                                Illuminant::F2,
                                "F2 (Cool White Fluorescent)",
                            );
                            ui.selectable_value(
                                ui_ctx.selected_illuminant,
                                Illuminant::F7,
                                "F7 (Daylight Fluorescent)",
                            );
                            ui.selectable_value(
                                ui_ctx.selected_illuminant,
                                Illuminant::F11,
                                "F11 (TL84)",
                            );
                        });
                    ui.end_row();

                    ui.label(t!("gui-observer"));
                    egui::ComboBox::from_id_salt("observer_selector_settings")
                        .selected_text(match ui_ctx.selected_observer {
                            Observer::CIE1931_2 => "2° (Standard)",
                            Observer::CIE1964_10 => "10° (Supplementary)",
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                ui_ctx.selected_observer,
                                Observer::CIE1931_2,
                                "2° (CIE 1931 Standard)",
                            );
                            ui.selectable_value(
                                ui_ctx.selected_observer,
                                Observer::CIE1964_10,
                                "10° (CIE 1964 Large Field)",
                            );
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

            ui.add_space(20.0);
            ui.separator();
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                if ui.button(t!("gui-close")).clicked() {
                    *ui_ctx.show = false;
                }
            });
        });
}
