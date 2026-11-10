//! Bottom Status & Transform Type-In bar (28px).

use super::theme;
use super::widgets::{dim, max_spinner, toggle_button};
use crate::editor::Editor;
use bevy::math::{Mat4, Vec3};
use bevy_egui::egui::{self, RichText};

fn thousands(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) -> egui::Rect {
    egui::Panel::bottom("status_bar")
        .resizable(false)
        .show_separator_line(false)
        .exact_size(28.0)
        .frame(egui::Frame::new().fill(theme::INPUT).inner_margin(egui::Margin::symmetric(10, 3)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                let (v, e, f) = match ed.current_obj() {
                    Some(o) if ed.in_sub_object() => o.level_mesh.counts(),
                    _ => ed.scene.stats(),
                };
                let sel = if ed.in_sub_object() {
                    let n = ed.current_obj().map(|o| o.level_mesh.selection_count(ed.sub)).unwrap_or(0);
                    format!("Selected: {n} {}{}", ed.sub.label(), if n == 1 { "" } else { "s" })
                } else {
                    let n = ed.scene.selected().len();
                    format!("Selected: {n} Object{}", if n == 1 { "" } else { "s" })
                };
                ui.label(RichText::new(sel).color(theme::AMBER));
                ui.label(dim(&format!("Verts: {}  |  Edges: {}  |  Polys: {}", thousands(v), thousands(e), thousands(f))));
                ui.separator();
                crate::ui::action_ui::button(ui, ed, crate::ui::actions::Action::LockSelection);
                ui.label(dim(&prompt(ed)));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(dim(&format!("Grid = {}", ed.snap_cfg.grid_size)))
                        .on_hover_text("Grid spacing (change it in Grid and Snap Settings)");
                    ui.separator();
                    typein(ui, ed);
                });
            });
        })
        .response
        .rect
}

/// What the current tool expects next (3ds Max's prompt line).
pub fn prompt(ed: &Editor) -> String {
    if let Some(label) = ed.tool.label() {
        return label.to_string();
    }
    if let Some(m) = ed.iface.nav_mode {
        return format!("{}  ·  right-click or Esc to stop", m.label());
    }
    if ed.iface.lock_selection {
        return "Selection is locked: press Space or the lock button to unlock".into();
    }
    if !ed.status.is_empty() {
        return ed.status.clone();
    }
    let n = ed.scene.selected().len();
    match ed.mode {
        _ if n == 0 => "Click or drag to select objects".into(),
        crate::editor::TransformMode::Select => "Click or drag to select; W / E / R to transform".into(),
        _ => "Drag the gizmo to transform; Shift+drag clones".into(),
    }
}

fn typein(ui: &mut egui::Ui, ed: &mut Editor) {
    let sub = ed.in_sub_object();
    let has_sel = if sub { ed.selection_center().is_some() } else { ed.current().is_some() };
    let id = egui::Id::new("typein_offset_values");
    let mut offset_vals: [f32; 3] = ui.data_mut(|d| *d.get_temp_mut_or_default::<[f32; 3]>(id));
    let abs = if sub {
        ed.selection_center().unwrap_or(Vec3::ZERO)
    } else {
        ed.current_obj().map(|o| o.transform.translation).unwrap_or(Vec3::ZERO)
    };
    let mut vals = if ed.typein_offset { offset_vals } else { abs.to_array() };
    let mut changed = false;
    let mut started = false;
    let mut stopped = false;
    ui.add_enabled_ui(has_sel, |ui| {
        for (i, axis) in ["Z:", "Y:", "X:"].iter().enumerate().map(|(k, a)| (2 - k, a)) {
            let r = max_spinner(ui, &mut vals[i], 0.01, 0.0);
            ui.label(*axis);
            changed |= r.changed();
            started |= r.drag_started() || r.gained_focus();
            stopped |= r.drag_stopped() || r.lost_focus() || (r.changed() && !r.dragged() && !r.has_focus());
        }
    });
    if toggle_button(ui, ed.typein_offset, if ed.typein_offset { "Off" } else { "Abs" })
        .on_hover_text("Absolute / Offset transform type-in")
        .clicked()
    {
        ed.typein_offset = !ed.typein_offset;
        offset_vals = [0.0; 3];
    }
    if has_sel && (started || changed) && ed.drag.is_none() {
        let frame = if sub { Mat4::from_translation(abs) } else { Mat4::IDENTITY };
        ed.begin_drag(frame, false);
    }
    if changed {
        if sub {
            let start = ed.drag.as_ref().map(|d| d.start.to_scale_rotation_translation().2).unwrap_or(abs);
            let target = if ed.typein_offset { start + Vec3::from_array(vals) } else { Vec3::from_array(vals) };
            ed.drag_frame(Mat4::from_translation(target));
        } else {
            let delta =
                if ed.typein_offset { Vec3::from_array(vals) - Vec3::from_array(offset_vals) } else { Vec3::from_array(vals) - abs };
            for k in ed.scene.selected() {
                ed.scene.objects[k].transform.translation += delta;
            }
        }
        if ed.typein_offset {
            offset_vals = vals;
        }
    }
    if stopped && ed.drag.is_some() {
        ed.end_drag();
        offset_vals = [0.0; 3];
    }
    ui.data_mut(|d| d.insert_temp(id, offset_vals));
}
