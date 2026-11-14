//! Brush Strokes rollout: painted normals. Each stroke tilts the surface
//! one way, so lighting turns into brush marks in the viewport and in F9.

use super::edit;
use super::params::num;
use crate::editor::Editor;
use crate::materials::brush::{MAX_LENGTH, PRESETS};
use crate::ui::widgets::{dim, max_spinner_range, rollout};
use bevy_egui::egui;

pub fn rollout_ui(ui: &mut egui::Ui, ed: &mut Editor) {
    let on = ed.editing_material().brush.enabled;
    let mut to_all = false;
    rollout(ui, "Brush Strokes (Painted Normals)", on, |ui| {
        edit(ed, |m, t| {
            let b = &mut m.brush;
            t.r(ui.checkbox(&mut b.enabled, "Painted Normals")).on_hover_text("Repaint this material's normals as brush strokes");
            ui.horizontal_wrapped(|ui| {
                for (name, look) in &PRESETS {
                    if ui.button(*name).on_hover_text("Turn Painted Normals on with this look").clicked() {
                        *b = look.clone();
                        (t.started, t.changed) = (true, true);
                    }
                }
            });
            ui.add_enabled_ui(b.enabled, |ui| {
                egui::Grid::new("mat_brush").num_columns(2).spacing([10.0, 6.0]).min_col_width(110.0).show(ui, |ui| {
                    num(ui, t, "Stroke Size", &mut b.size, 0.005..=0.5, 0.06);
                    num(ui, t, "Stroke Length", &mut b.length, 1.0..=MAX_LENGTH, 2.4);
                    num(ui, t, "Strength", &mut b.strength, 0.0..=1.0, 0.6);
                    num(ui, t, "Angle", &mut b.angle, -180.0..=180.0, 0.0);
                    num(ui, t, "Angle Jitter", &mut b.jitter, 0.0..=180.0, 35.0);
                    num(ui, t, "Bristles", &mut b.bristles, 0.0..=1.0, 0.25);
                    ui.label("Seed");
                    ui.horizontal(|ui| {
                        t.r(max_spinner_range(ui, &mut b.seed, 0.2, 1, 0..=9999));
                        if ui.button("🎲").on_hover_text("New random seed").clicked() {
                            b.seed = rand::random::<u32>() % 10_000;
                            (t.started, t.changed) = (true, true);
                        }
                    });
                    ui.end_row();
                });
            });
            to_all = ui.button("Apply to All Materials").on_hover_text("Give every material in the scene these brush settings").clicked();
            ui.label(dim("Stroke Size is a fraction of one UV repeat (or one World tile). Angle 0 runs along U. Strokes stick to the model and re-light with the scene."));
            None
        });
    });
    if to_all {
        ed.brush_to_all_materials();
    }
}
