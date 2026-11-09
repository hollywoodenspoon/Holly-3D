//! Shape rollouts: parametric shape Parameters, Interpolation, Rendering,
//! Editable Spline Selection / Geometry, plus Lathe and Extrude.

use super::HALF;
use crate::editor::Editor;
use crate::mesh::SubObject;
use crate::modifiers::{LatheAlign, ModSub, ModifierKind};
use crate::splines::edit::SplineLevel;
use crate::splines::{KnotType, ShapeObject, ShapeParams, ShapeSource};
use crate::ui::command::CONTENT_W;
use crate::ui::theme;
use crate::ui::widgets::{Track, action, dim, max_spinner_range, rollout};
use bevy_egui::egui::{self, RichText};

fn grid(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([10.0, 6.0]).show(ui, body);
}

/// Spinner row; angles (ranges past ±100) step faster than distances.
fn num_row(ui: &mut egui::Ui, t: &mut Track, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, default: f32) {
    ui.label(label);
    let speed = if *range.end() >= 180.0 && *range.end() <= 360.0 { 0.5 } else { 0.01 };
    t.r(max_spinner_range(ui, v, speed, default, range));
    ui.end_row();
}

fn u_row(ui: &mut egui::Ui, t: &mut Track, label: &str, v: &mut u32, default: u32, range: std::ops::RangeInclusive<u32>) {
    ui.label(label);
    t.r(max_spinner_range(ui, v, 0.1, default, range));
    ui.end_row();
}

/// Edits a clone of the shape base object, writing it back on change.
fn edit_shape_object(ui: &mut egui::Ui, ed: &mut Editor, body: impl FnOnce(&mut egui::Ui, &mut ShapeObject, &mut Track)) {
    let Some(id) = ed.current() else { return };
    let Some(mut so) = ed.scene.objects[id].shape_object().cloned() else { return };
    let mut t = Track::default();
    body(ui, &mut so, &mut t);
    if t.started {
        ed.checkpoint("Shape Parameters");
    }
    if t.changed {
        let o = &mut ed.scene.objects[id];
        if let Some(s) = o.shape_object_mut() {
            *s = so;
        }
        o.evaluate();
        ed.scene.propagate_from(id);
    }
}

fn param_rows(ui: &mut egui::Ui, p: &mut ShapeParams, t: &mut Track) {
    let big = 0.0..=1e4;
    match p {
        ShapeParams::Circle { radius } => num_row(ui, t, "Radius:", radius, big, 0.6),
        ShapeParams::Arc { radius, from, to, pie } => {
            num_row(ui, t, "Radius:", radius, big, 0.6);
            num_row(ui, t, "From:", from, -360.0..=360.0, 0.0);
            num_row(ui, t, "To:", to, -360.0..=360.0, 180.0);
            ui.label("");
            t.r(ui.checkbox(pie, "Pie Slice"));
            ui.end_row();
        }
        ShapeParams::NGon { radius, sides, circular } => {
            num_row(ui, t, "Radius:", radius, big, 0.6);
            u_row(ui, t, "Sides:", sides, 6, 3..=100);
            ui.label("");
            t.r(ui.checkbox(circular, "Circular"));
            ui.end_row();
        }
        ShapeParams::Rectangle { length, width, corner_radius } => {
            num_row(ui, t, "Length:", length, big.clone(), 1.0);
            num_row(ui, t, "Width:", width, big.clone(), 1.4);
            num_row(ui, t, "Corner Radius:", corner_radius, big, 0.0);
        }
        ShapeParams::Ellipse { length, width } => {
            num_row(ui, t, "Length:", length, big.clone(), 0.8);
            num_row(ui, t, "Width:", width, big, 1.4);
        }
        ShapeParams::Star { radius1, radius2, points, distortion } => {
            num_row(ui, t, "Radius 1:", radius1, big.clone(), 0.7);
            num_row(ui, t, "Radius 2:", radius2, big, 0.35);
            u_row(ui, t, "Points:", points, 6, 3..=100);
            num_row(ui, t, "Distortion:", distortion, -180.0..=180.0, 0.0);
        }
        ShapeParams::Helix { radius1, radius2, height, turns } => {
            num_row(ui, t, "Radius 1:", radius1, big.clone(), 0.5);
            num_row(ui, t, "Radius 2:", radius2, big.clone(), 0.5);
            num_row(ui, t, "Height:", height, -1e4..=1e4, 1.2);
            num_row(ui, t, "Turns:", turns, 0.0..=100.0, 3.0);
        }
    }
}

/// Base-level rollouts of any shape object.
pub fn shape_base(ui: &mut egui::Ui, ed: &mut Editor) {
    let editable = ed.editable_spline_selected();
    if editable {
        editable_spline(ui, ed);
    } else {
        rollout(ui, "Parameters", true, |ui| {
            edit_shape_object(ui, ed, |ui, so, t| {
                if let ShapeSource::Param(p) = &mut so.source {
                    grid(ui, "shape_params", |ui| param_rows(ui, p, t));
                }
            });
        });
        if action(ui, "Convert to Editable Spline", true, CONTENT_W) {
            ed.convert_to_editable_spline();
        }
    }
    rollout(ui, "Interpolation", false, |ui| {
        edit_shape_object(ui, ed, |ui, so, t| grid(ui, "shape_interp", |ui| u_row(ui, t, "Steps:", &mut so.steps, 6, 0..=100)));
    });
    rollout(ui, "Rendering", true, |ui| {
        edit_shape_object(ui, ed, |ui, so, t| {
            let r = &mut so.render;
            ui.horizontal(|ui| {
                t.r(ui.checkbox(&mut r.renderer, "Enable In Renderer"));
                t.r(ui.checkbox(&mut r.viewport, "Enable In Viewport"));
            });
            ui.horizontal(|ui| {
                t.r(ui.radio_value(&mut r.rectangular, false, "Radial"));
                t.r(ui.radio_value(&mut r.rectangular, true, "Rectangular"));
            });
            grid(ui, "shape_render", |ui| {
                if r.rectangular {
                    num_row(ui, t, "Length:", &mut r.length, 0.0..=100.0, 0.1);
                    num_row(ui, t, "Width:", &mut r.width, 0.0..=100.0, 0.05);
                    num_row(ui, t, "Angle:", &mut r.rect_angle, -360.0..=360.0, 0.0);
                } else {
                    num_row(ui, t, "Thickness:", &mut r.thickness, 0.0..=100.0, 0.05);
                    u_row(ui, t, "Sides:", &mut r.sides, 12, 3..=100);
                    num_row(ui, t, "Angle:", &mut r.angle, -360.0..=360.0, 0.0);
                }
            });
        });
        ui.label(dim("Turns any spline into a pipe, cable or bar."));
    });
}

fn level_buttons(ui: &mut egui::Ui, ed: &mut Editor) {
    ui.horizontal(|ui| {
        let w = CONTENT_W / 3.0 - 6.0;
        for (s, label) in [(SubObject::Vertex, "1: Vertex"), (SubObject::Edge, "2: Segment"), (SubObject::Border, "3: Spline")] {
            let on = ed.sub == s;
            let text = if on { RichText::new(label).color(theme::INPUT).strong() } else { RichText::new(label) };
            if ui.add_sized([w, 30.0], egui::Button::new(text).selected(on)).clicked() {
                ed.set_sub_object(s);
            }
        }
    });
}

fn spin_action(ui: &mut egui::Ui, label: &str, v: &mut f32, default: f32) -> bool {
    let mut go = false;
    ui.horizontal(|ui| {
        go = action(ui, label, true, HALF);
        max_spinner_range(ui, v, 0.005, default, 0.0..=1e3);
    });
    go
}

fn editable_spline(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Selection", true, |ui| {
        level_buttons(ui, ed);
        let counts = ed.current_obj().and_then(|o| o.shape.as_ref()).map(|s| (s.splines.len(), s.knot_count()));
        if let Some((s, k)) = counts {
            ui.label(dim(&format!("{s} spline(s), {k} vertices. Click / drag-select, then W/E/R.")));
        }
    });
    let Some(level) = ed.spline_level() else {
        return;
    };
    let mut ui_st = ed.spline_ui.clone();
    rollout(ui, "Geometry", true, |ui| match level {
        SplineLevel::Vertex => {
            ui.label(RichText::new("Vertex type:").strong());
            egui::Grid::new("knot_types").num_columns(2).spacing([6.0, 6.0]).show(ui, |ui| {
                for (i, k) in KnotType::ALL.iter().enumerate() {
                    if action(ui, k.label(), true, HALF) {
                        ed.set_knot_type(*k);
                    }
                    if i % 2 == 1 {
                        ui.end_row();
                    }
                }
            });
            ui.separator();
            if spin_action(ui, "Fillet", &mut ui_st.fillet, 0.1) {
                ed.edit_shape("Fillet", |s| {
                    let _ = s.chamfer_selected(ui_st.fillet, true);
                });
            }
            if spin_action(ui, "Chamfer", &mut ui_st.chamfer, 0.1) {
                ed.edit_shape("Chamfer", |s| {
                    let _ = s.chamfer_selected(ui_st.chamfer, false);
                });
            }
            if spin_action(ui, "Weld", &mut ui_st.weld, 0.05) {
                ed.edit_shape("Weld", |s| {
                    let _ = s.weld_selected(ui_st.weld);
                });
            }
            ui.horizontal(|ui| {
                if action(ui, "Make First", true, HALF) {
                    ed.edit_shape("Make First", |s| {
                        for (si, ki) in s.selected_knots(SplineLevel::Vertex).into_iter().take(1) {
                            s.splines[si].make_first(ki);
                        }
                    });
                }
                if action(ui, "Delete", true, HALF) {
                    ed.delete_selection();
                }
            });
        }
        SplineLevel::Segment => {
            ui.horizontal(|ui| {
                let go = action(ui, "Divide", true, HALF);
                max_spinner_range(ui, &mut ui_st.divide, 0.1, 1, 1..=100);
                if go {
                    ed.edit_shape("Divide", |s| s.divide_selected(ui_st.divide));
                }
            });
            if action(ui, "Delete", true, CONTENT_W) {
                ed.delete_selection();
            }
        }
        SplineLevel::Spline => {
            if spin_action(ui, "Outline", &mut ui_st.outline, 0.05) {
                ed.edit_shape("Outline", |s| s.outline_selected(ui_st.outline));
            }
            ui.horizontal(|ui| {
                for (i, l) in ["↔", "↕", "✣"].iter().enumerate() {
                    ui.radio_value(&mut ui_st.mirror_axis, i, *l);
                }
                ui.checkbox(&mut ui_st.mirror_copy, "Copy");
                if action(ui, "Mirror", true, 80.0) {
                    ed.edit_shape("Mirror", |s| s.mirror_selected(ui_st.mirror_axis, ui_st.mirror_copy));
                }
            });
            ui.horizontal(|ui| {
                if action(ui, "Reverse", true, HALF) {
                    ed.edit_shape("Reverse", |s| s.splines.iter_mut().filter(|s| s.selected).for_each(|s| s.reverse()));
                }
                if action(ui, "Close", true, HALF) {
                    ed.edit_shape("Close", |s| {
                        s.splines.iter_mut().filter(|s| s.selected && s.knots.len() > 2).for_each(|s| s.closed = true)
                    });
                }
            });
            if action(ui, "Delete", true, CONTENT_W) {
                ed.delete_selection();
            }
        }
    });
    ed.spline_ui = ui_st;
}

/// Edits the active modifier (Lathe / Extrude) with undo once per gesture.
fn mod_params(ui: &mut egui::Ui, ed: &mut Editor, body: impl FnOnce(&mut egui::Ui, &mut ModifierKind, &mut Track)) {
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

pub fn lathe(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Lathe Parameters", true, |ui| {
        let on = ed.active_mod_sub() == Some(ModSub::Axis);
        let text = if on { RichText::new("1: Axis").color(theme::INPUT).strong() } else { RichText::new("1: Axis") };
        if ui.add_sized([CONTENT_W, 28.0], egui::Button::new(text).selected(on)).on_hover_text("Move the revolution axis with W").clicked()
        {
            ed.set_mod_sub(Some(ModSub::Axis));
        }
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::Lathe(l) = k else { return };
            grid(ui, "lathe", |ui| {
                num_row(ui, t, "Degrees:", &mut l.degrees, 0.0..=360.0, 360.0);
                u_row(ui, t, "Segments:", &mut l.segments, 16, 3..=128);
            });
            ui.horizontal(|ui| {
                t.r(ui.checkbox(&mut l.weld_core, "Weld Core"));
                t.r(ui.checkbox(&mut l.flip_normals, "Flip Normals"));
            });
            ui.horizontal(|ui| {
                t.r(ui.checkbox(&mut l.cap_start, "Cap Start"));
                t.r(ui.checkbox(&mut l.cap_end, "Cap End"));
            });
            ui.label(RichText::new("Direction").strong());
            ui.horizontal(|ui| {
                for (i, n) in ["X", "Y", "Z"].iter().enumerate() {
                    t.r(ui.radio_value(&mut l.axis, i, *n));
                }
            });
            ui.label(RichText::new("Align").strong());
            ui.horizontal(|ui| {
                for (a, n) in [(LatheAlign::Min, "Min"), (LatheAlign::Center, "Center"), (LatheAlign::Max, "Max")] {
                    if ui.add_sized([CONTENT_W / 3.0 - 6.0, 24.0], egui::Button::new(n).selected(l.align == a)).clicked() {
                        l.align = a;
                        l.axis_offset = Default::default();
                        t.started = true;
                        t.changed = true;
                    }
                }
            });
        });
    });
}

pub fn extrude(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Extrude Parameters", true, |ui| {
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::Extrude(e) = k else { return };
            grid(ui, "extrude", |ui| {
                num_row(ui, t, "Amount:", &mut e.amount, -1e4..=1e4, 0.5);
                u_row(ui, t, "Segments:", &mut e.segments, 1, 1..=64);
            });
            ui.horizontal(|ui| {
                t.r(ui.checkbox(&mut e.cap_start, "Cap Start"));
                t.r(ui.checkbox(&mut e.cap_end, "Cap End"));
            });
            ui.label(dim("Closed splines get N-gon caps; open splines extrude as walls."));
        });
    });
}
