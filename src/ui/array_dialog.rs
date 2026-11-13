//! Tools -> Array... : the classic Array dialog (Incremental /
//! Totals grid, Type of Object, 1D / 2D / 3D dimensions, live Preview).

use super::widgets::{dim, max_spinner, max_spinner_range, toggle_button};
use crate::editor::array::ArraySettings;
use crate::editor::{Editor, PivotMode};
use crate::scene::CloneKind;
use bevy::math::Vec3;
use bevy_egui::egui::{self, RichText};

/// Three right-click-resettable spinners for one Vec3; true when changed.
fn vec3(ui: &mut egui::Ui, v: &mut Vec3, speed: f64, default: f32) -> bool {
    let mut changed = false;
    for i in 0..3 {
        ui.add_sized([58.0, 20.0], |ui: &mut egui::Ui| {
            let r = max_spinner(ui, &mut v[i], speed, default);
            changed |= r.changed();
            r
        });
    }
    changed
}

fn center_label(p: PivotMode) -> &'static str {
    match p {
        PivotMode::PivotPoint => "Use Pivot Point Center",
        PivotMode::SelectionCenter => "Use Selection Center",
        PivotMode::TransformCenter => "Use Transform Coordinate Center",
    }
}

fn transformation(ui: &mut egui::Ui, s: &mut ArraySettings) {
    egui::Grid::new("array_xf").num_columns(9).spacing([4.0, 4.0]).show(ui, |ui| {
        ui.label("");
        ui.label(dim("Incremental"));
        for _ in 0..2 {
            ui.label("");
        }
        ui.label("");
        ui.label(dim("Totals"));
        ui.end_row();
        ui.label("");
        for _ in 0..2 {
            for a in ["X", "Y", "Z"] {
                ui.label(dim(a));
            }
            ui.label("");
        }
        ui.end_row();
        let (mut tm, mut tr, mut ts) = s.totals();
        let mut totals_changed = false;
        ui.label("Move");
        vec3(ui, &mut s.move_inc, 0.01, 0.0);
        ui.label(dim("units"));
        totals_changed |= vec3(ui, &mut tm, 0.02, 0.0);
        ui.end_row();
        ui.label("Rotate");
        vec3(ui, &mut s.rotate_inc, 0.5, 0.0);
        ui.label(dim("deg"));
        totals_changed |= vec3(ui, &mut tr, 1.0, 0.0);
        ui.end_row();
        ui.label("Scale");
        if s.uniform {
            let old = s.scale_inc.x;
            let mut u = Vec3::splat(old);
            vec3(ui, &mut u, 0.5, 100.0);
            let v = u.to_array().into_iter().find(|&c| c != old).unwrap_or(old);
            s.scale_inc = Vec3::splat(v);
        } else {
            vec3(ui, &mut s.scale_inc, 0.5, 100.0);
        }
        ui.label(dim("%"));
        totals_changed |= vec3(ui, &mut ts, 1.0, 100.0);
        ui.end_row();
        if totals_changed {
            s.set_totals(tm, tr, ts);
        }
    });
    ui.horizontal(|ui| {
        ui.checkbox(&mut s.re_orient, "Re-Orient");
        ui.checkbox(&mut s.uniform, "Uniform");
    });
}

fn dimensions(ui: &mut egui::Ui, s: &mut ArraySettings) {
    egui::Grid::new("array_dims").num_columns(5).spacing([4.0, 4.0]).show(ui, |ui| {
        ui.label("");
        ui.label(dim("Count"));
        ui.label(dim("Incremental Row / Layer Offsets (X, Y, Z)"));
        ui.end_row();
        for d in 1..=3u8 {
            ui.radio_value(&mut s.dims, d, format!("{d}D"));
            ui.add_enabled_ui(s.dims >= d, |ui| {
                ui.add_sized([50.0, 20.0], |ui: &mut egui::Ui| {
                    max_spinner_range(ui, &mut s.count[d as usize - 1], 0.1, if d == 1 { 5 } else { 1 }, 1..=500)
                });
            });
            match d {
                2 => ui.add_enabled_ui(s.dims >= 2, |ui| vec3(ui, &mut s.row_offset, 0.01, 0.0)),
                3 => ui.add_enabled_ui(s.dims >= 3, |ui| vec3(ui, &mut s.layer_offset, 0.01, 0.0)),
                _ => ui.add_enabled_ui(true, |_| false),
            };
            ui.end_row();
        }
    });
}

pub fn show(ctx: &egui::Context, ed: &mut Editor) {
    let Some(mut s) = ed.array_dialog.as_ref().map(|d| d.settings.clone()) else { return };
    let pivot = ed.pivot_mode;
    let mut result = None;
    egui::Window::new("Array")
        .collapsible(false)
        .resizable(false)
        .default_pos(ctx.content_rect().center() - egui::vec2(300.0, 220.0))
        .show(ctx, |ui| {
            ui.label(RichText::new("Array Transformation").strong());
            ui.label(dim(&format!("World Coordinates ({})", center_label(pivot))));
            ui.add_space(4.0);
            transformation(ui, &mut s);
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Type of Object:");
                for k in CloneKind::ALL {
                    if toggle_button(ui, s.kind == k, k.label()).clicked() {
                        s.kind = k;
                    }
                }
            });
            ui.separator();
            ui.label(RichText::new("Array Dimensions").strong());
            dimensions(ui, &mut s);
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("Total in Array: {}", s.total())).strong());
                ui.checkbox(&mut s.preview, "Preview");
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                if ui.button("Reset All Parameters").clicked() {
                    s = ArraySettings { preview: s.preview, ..Default::default() };
                }
                if ui.button("  OK  ").clicked() {
                    result = Some(true);
                }
                if ui.button("Cancel").clicked() {
                    result = Some(false);
                }
            });
        });
    if let Some(d) = &mut ed.array_dialog {
        d.settings = s;
    }
    match result {
        Some(true) => ed.confirm_array(),
        Some(false) => ed.cancel_array(),
        None => ed.update_array_preview(),
    }
}
