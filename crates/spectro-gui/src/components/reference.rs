use crate::t;
use eframe::egui;
use spectro_rs::colorimetry::Lab;

pub struct ReferenceContext<'a> {
    pub show: &'a mut bool,
    pub reference_lab: &'a mut Option<Lab>,
    pub delta_e_tolerance: &'a mut f32,
    pub ref_input_l: &'a mut f32,
    pub ref_input_a: &'a mut f32,
    pub ref_input_b: &'a mut f32,
    pub current_lab: Option<Lab>,
}

pub fn render_reference_window(ctx: &egui::Context, ui_ctx: &mut ReferenceContext) {
    if !*ui_ctx.show {
        return;
    }

    egui::Window::new(t!("gui-set-ref-color"))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui.label(t!("gui-enter-lab"));
            ui.add_space(10.0);

            egui::Grid::new("ref_input_grid")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.label("L*:");
                    ui.add(
                        egui::DragValue::new(ui_ctx.ref_input_l)
                            .range(0.0..=100.0)
                            .speed(0.5),
                    );
                    ui.end_row();
                    ui.label("a*:");
                    ui.add(
                        egui::DragValue::new(ui_ctx.ref_input_a)
                            .range(-128.0..=128.0)
                            .speed(0.5),
                    );
                    ui.end_row();
                    ui.label("b*:");
                    ui.add(
                        egui::DragValue::new(ui_ctx.ref_input_b)
                            .range(-128.0..=128.0)
                            .speed(0.5),
                    );
                    ui.end_row();
                });

            ui.add_space(5.0);
            ui.label("ΔE Tolerance:");
            ui.add(egui::Slider::new(ui_ctx.delta_e_tolerance, 0.5..=10.0).suffix(" ΔE"));

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui
                    .button(format!("✓ {}", t!("gui-set-reference")))
                    .clicked()
                {
                    *ui_ctx.reference_lab = Some(Lab {
                        l: *ui_ctx.ref_input_l,
                        a: *ui_ctx.ref_input_a,
                        b: *ui_ctx.ref_input_b,
                    });
                    *ui_ctx.show = false;
                }
                if ui.button(t!("gui-use-current")).clicked()
                    && let Some(lab) = ui_ctx.current_lab
                {
                    *ui_ctx.ref_input_l = lab.l;
                    *ui_ctx.ref_input_a = lab.a;
                    *ui_ctx.ref_input_b = lab.b;
                    *ui_ctx.reference_lab = Some(lab);
                    *ui_ctx.show = false;
                }
                if ui.button(t!("gui-clear")).clicked() {
                    *ui_ctx.reference_lab = None;
                }
                if ui.button(t!("gui-cancel")).clicked() {
                    *ui_ctx.show = false;
                }
            });
        });
}
