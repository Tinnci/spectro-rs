use crate::theme::{border_color, info_panel_color, muted_text_color};
use eframe::egui;

#[allow(dead_code)]
#[derive(Default)]
pub enum PanelSide {
    #[default]
    Left,
    Right,
}

#[macro_export]
macro_rules! render_dockable_panel {
    ($ctx:expr, $id:expr, $title:expr, $is_detached:expr, $is_visible:expr, $default_width:expr, $side:ident, $content:expr) => {
        if $is_detached {
            let viewport_id = egui::ViewportId::from_hash_of($id);
            $ctx.show_viewport_immediate(
                viewport_id,
                egui::ViewportBuilder::default()
                    .with_title($title)
                    .with_inner_size([$default_width, 600.0])
                    .with_min_inner_size([300.0, 400.0]),
                |ctx, class| {
                    if class == egui::ViewportClass::Embedded {
                        let mut open = true;
                        egui::Window::new($title)
                            .open(&mut open)
                            .show(ctx, $content);
                        if !open {
                            *$is_visible = false;
                        }
                    } else {
                        egui::CentralPanel::default().show(ctx, |ui| {
                            let f = $content; // Re-bind closure to allow calling
                            f(ui);
                        });
                        if ctx.input(|i| i.viewport().close_requested()) {
                            *$is_visible = false;
                        }
                    }
                },
            );
        } else {
            // Fully qualified path to PanelSide to resolve scope issues
            let side_enum = $crate::components::widgets::PanelSide::$side;
            let panel = match side_enum {
                $crate::components::widgets::PanelSide::Left => egui::SidePanel::left($id),
                $crate::components::widgets::PanelSide::Right => egui::SidePanel::right($id),
            };
            panel
                .resizable(true)
                .default_width($default_width)
                .show($ctx, $content);
        }
    };
}

pub fn render_bento_item<R>(
    ui: &mut egui::Ui,
    title: String,
    min_width: f32,
    max_width: f32,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    let visuals = &ui.ctx().style().visuals;

    ui.scope(|ui| {
        ui.set_min_width(min_width);
        ui.set_max_width(max_width);

        egui::Frame::none()
            .fill(info_panel_color(visuals))
            .stroke(egui::Stroke::new(1.0, border_color(visuals)))
            .rounding(6.0)
            .inner_margin(egui::Margin::same(ui.spacing().item_spacing.y))
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new(title.to_uppercase())
                            .size(10.0)
                            .color(muted_text_color(visuals))
                            .strong(),
                    );
                    ui.add_space(ui.spacing().item_spacing.y * 0.4);
                    add_contents(ui)
                })
                .inner
            })
            .inner
    })
    .inner
}

#[expect(
    dead_code,
    reason = "Utility icon for planned descriptive tooltips to reduce UI text density"
)]
pub fn help_icon(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new("ⓘ").color(muted_text_color(ui.visuals())))
        .on_hover_text(text);
}
