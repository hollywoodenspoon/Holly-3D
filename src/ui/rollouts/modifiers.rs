//! Rollouts for FFD, UVW Map and Unwrap UVW modifiers.

use super::half;
use crate::editor::{Editor, Tool};
use crate::modifiers::{MapType, ModSub, ModifierKind};
use crate::ui::properties::content_w;
use crate::ui::theme;
use crate::ui::widgets::{Track, action, dim, max_spinner_range, rollout};
use bevy_egui::egui::{self, RichText};

/// Shows a parameter block on a clone of the active modifier and writes it back on change.
fn params(ui: &mut egui::Ui, ed: &mut Editor, body: impl FnOnce(&mut egui::Ui, &mut ModifierKind, &mut Track)) {
    let Some(id) = ed.current() else { return };
    let Some(level) = ed.scene.objects[id].active_level else { return };
    let mut kind = ed.scene.objects[id].modifiers[level].kind.clone();
    let mut t = Track::default();
    body(ui, &mut kind, &mut t);
    if t.started {
        ed.checkpoint("Modifier Parameters");
    }
    if t.changed {
        let o = &mut ed.scene.objects[id];
        o.modifiers[level].kind = kind;
        o.evaluate();
        ed.scene.propagate_from(id);
    }
}

pub(crate) fn sub_buttons(ui: &mut egui::Ui, ed: &mut Editor, subs: &[ModSub]) {
    ui.horizontal(|ui| {
        let w = content_w() / subs.len() as f32 - 8.0;
        for (i, &s) in subs.iter().enumerate() {
            let on = ed.active_mod_sub() == Some(s);
            let label = format!("{}: {}", i + 1, s.label());
            let text = if on { RichText::new(label).color(theme::input()).strong() } else { RichText::new(label) };
            if ui.add_sized([w, 28.0], egui::Button::new(text).selected(on)).clicked() {
                ed.set_mod_sub(Some(s));
            }
        }
    });
}

pub fn ffd(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "FFD Parameters", true, |ui| {
        sub_buttons(ui, ed, &[ModSub::ControlPoints, ModSub::Lattice]);
        params(ui, ed, |ui, k, t| {
            if let ModifierKind::Ffd(f) = k {
                ui.label(dim(&format!("Lattice: {0} x {0} x {0} control points", f.dim)));
                t.r(ui.checkbox(&mut f.only_in_volume, "Deform: Only In Volume"));
            }
        });
        ui.horizontal(|ui| {
            if action(ui, "Conform to Shape", true, half()) {
                ed.edit_active_modifier("Conform to Shape", |k, input| {
                    if let ModifierKind::Ffd(f) = k {
                        f.conform(input);
                    }
                });
            }
            if action(ui, "Reset", true, half()) {
                ed.edit_active_modifier("Reset FFD", |k, _| {
                    if let ModifierKind::Ffd(f) = k {
                        f.reset();
                    }
                });
            }
        });
        ui.horizontal(|ui| {
            let w = content_w() / 3.0 - 6.0;
            for (axis, label) in ["All X", "All Y", "All Z"].iter().enumerate() {
                if ui
                    .add_sized([w, 24.0], egui::Button::new(*label))
                    .on_hover_text("Extend the point selection along this lattice axis")
                    .clicked()
                {
                    ed.edit_active_modifier(label, |k, _| {
                        if let ModifierKind::Ffd(f) = k {
                            f.select_along(axis);
                        }
                    });
                }
            }
        });
        ui.label(dim("1: Control Points — click / drag-select, then W/E/R.  2: Lattice moves the cage."));
    });
}

fn spin(ui: &mut egui::Ui, t: &mut Track, label: &str, v: &mut f32, default: f32, range: std::ops::RangeInclusive<f32>) {
    ui.label(label);
    t.r(max_spinner_range(ui, v, 0.01, default, range));
}

pub fn uvw_map(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Parameters", true, |ui| {
        sub_buttons(ui, ed, &[ModSub::Gizmo]);
        params(ui, ed, |ui, k, t| {
            let ModifierKind::UvwMap(u) = k else { return };
            ui.label(RichText::new("Mapping:").strong());
            egui::Grid::new("uvw_types").num_columns(2).spacing([8.0, 2.0]).show(ui, |ui| {
                for (i, m) in MapType::ALL.iter().enumerate() {
                    t.r(ui.radio_value(&mut u.kind, *m, m.label()));
                    if i % 2 == 1 {
                        ui.end_row();
                    }
                }
            });
            ui.add_enabled_ui(u.kind == MapType::Cylindrical, |ui| t.r(ui.checkbox(&mut u.cap, "Cap")));
            egui::Grid::new("uvw_dims").num_columns(4).spacing([6.0, 4.0]).show(ui, |ui| {
                spin(ui, t, "Length:", &mut u.length, 1.0, 0.0001..=1e5);
                ui.end_row();
                spin(ui, t, "Width:", &mut u.width, 1.0, 0.0001..=1e5);
                ui.end_row();
                spin(ui, t, "Height:", &mut u.height, 1.0, 0.0001..=1e5);
                ui.end_row();
                spin(ui, t, "U Tile:", &mut u.u_tile, 1.0, -1e3..=1e3);
                t.r(ui.checkbox(&mut u.u_flip, "Flip"));
                ui.end_row();
                spin(ui, t, "V Tile:", &mut u.v_tile, 1.0, -1e3..=1e3);
                t.r(ui.checkbox(&mut u.v_flip, "Flip"));
                ui.end_row();
                spin(ui, t, "W Tile:", &mut u.w_tile, 1.0, -1e3..=1e3);
                t.r(ui.checkbox(&mut u.w_flip, "Flip"));
                ui.end_row();
            });
            ui.label(dim("W tiles 3D procedural maps (materials, Phase 4)."));
        });
    });
    rollout(ui, "Alignment", true, |ui| {
        let axis = match ed.current_obj().and_then(|o| o.active_modifier()) {
            Some(ModifierKind::UvwMap(u)) => u.axis,
            _ => return,
        };
        ui.horizontal(|ui| {
            for (i, name) in ["X", "Y", "Z"].iter().enumerate() {
                if ui.radio(axis == i, *name).clicked() {
                    ed.edit_active_modifier("Alignment", |k, input| {
                        if let ModifierKind::UvwMap(u) = k {
                            u.set_axis(i, input);
                        }
                    });
                }
            }
        });
        let gizmo_op = |ed: &mut Editor, label: &str, f: fn(&mut crate::modifiers::UvwMapMod, &crate::mesh::EditablePoly)| {
            ed.edit_active_modifier(label, |k, input| {
                if let ModifierKind::UvwMap(u) = k {
                    f(u, input);
                }
            });
        };
        ui.horizontal(|ui| {
            if action(ui, "Fit", true, half()) {
                gizmo_op(ed, "Fit", |u, i| u.fit(i));
            }
            if action(ui, "Center", true, half()) {
                gizmo_op(ed, "Center", |u, i| u.center_on(i));
            }
        });
        ui.horizontal(|ui| {
            if ui
                .add(egui::Button::new("Bitmap Fit").min_size(egui::vec2(half(), 24.0)))
                .on_hover_text("Pick a bitmap: the gizmo takes its aspect ratio")
                .clicked()
            {
                bitmap_fit(ed);
            }
            let on = ed.tool == Tool::NormalAlign;
            if ui.add(egui::Button::new("Normal Align").selected(on).min_size(egui::vec2(half(), 24.0))).clicked() {
                ed.tool = if on { Tool::None } else { Tool::NormalAlign };
            }
        });
        ui.horizontal(|ui| {
            if action(ui, "View Align", true, half()) {
                ed.view_align();
            }
            if action(ui, "Reset", true, half()) {
                gizmo_op(ed, "Reset Gizmo", |u, i| u.reset(i));
            }
        });
        checker_toggle(ui, ed);
    });
}

fn checker_toggle(ui: &mut egui::Ui, ed: &mut Editor) {
    if let Some(o) = ed.current_obj_mut() {
        ui.checkbox(&mut o.checker, "Show UV Checker in Viewport");
    }
}

pub fn unwrap(ui: &mut egui::Ui, ed: &mut Editor) {
    let mismatch = ed.current_obj().is_some_and(|o| match (o.active_modifier(), o.active_level) {
        (Some(ModifierKind::Unwrap(u)), Some(i)) => !u.matches(&o.input_of(i)),
        _ => false,
    });
    rollout(ui, "Edit UVs", true, |ui| {
        if action(ui, "Open UV Editor...", true, content_w()) {
            ed.uv_editor_open = true;
        }
        if action(ui, "Reset UVWs", true, content_w()) {
            ed.edit_active_modifier("Reset UVWs", |k, input| {
                if let ModifierKind::Unwrap(u) = k {
                    u.capture(input);
                }
            });
        }
        checker_toggle(ui, ed);
        if mismatch {
            ui.label(RichText::new("Topology below changed — press Reset UVWs.").color(theme::amber()).small());
        }
    });
    rollout(ui, "Smart UV Project", true, |ui| {
        params(ui, ed, |ui, k, t| {
            let ModifierKind::Unwrap(u) = k else { return };
            egui::Grid::new("smart_uv").num_columns(2).spacing([8.0, 4.0]).show(ui, |ui| {
                spin(ui, t, "Angle Limit:", &mut u.angle_limit, 66.0, 1.0..=89.0);
                ui.end_row();
                ui.label("Island Margin:");
                t.r(max_spinner_range(ui, &mut u.island_margin, 0.001, 0.02, 0.0..=0.25));
                ui.end_row();
            });
            t.r(ui.checkbox(&mut u.correct_aspect, "Correct Aspect"));
        });
        if action(ui, "✨ Smart UV Unwrap", true, content_w()) {
            let mut msg = String::new();
            ed.edit_active_modifier("Smart UV Unwrap", |k, input| {
                if let ModifierKind::Unwrap(u) = k {
                    msg = match u.smart_unwrap(input) {
                        Ok(n) => format!("Smart UV Unwrap: {n} charts packed into 0..1"),
                        Err(e) => e,
                    };
                }
            });
            ed.status = msg;
        }
    });
}

/// Bitmap Fit: pick an image and give the UVW Map gizmo its aspect ratio.
fn bitmap_fit(ed: &mut Editor) {
    let exts = ["png", "jpg", "jpeg", "bmp", "tga", "tif", "tiff", "exr", "hdr"];
    let Some(path) = rfd::FileDialog::new().add_filter("Images", &exts).pick_file() else { return };
    match image::image_dimensions(&path) {
        Ok((w, h)) => {
            ed.edit_active_modifier("Bitmap Fit", |k, _| {
                if let ModifierKind::UvwMap(u) = k {
                    u.bitmap_fit(w as f32 / h.max(1) as f32);
                }
            });
            ed.status = format!("Bitmap Fit: {w} x {h}");
        }
        Err(e) => ed.status = format!("Bitmap Fit: {e}"),
    }
}
