//! Floating 2D UV Editor for the Unwrap UVW modifier: toolbar + canvas.

mod canvas;

use crate::editor::Editor;
use crate::mesh::VertexId;
use crate::modifiers::unwrap_uvw::{UvSel, corner_vertices};
use crate::modifiers::{ModifierKind, UnwrapMod};
use crate::ui::theme;
use crate::ui::widgets::dim;
use bevy::math::Vec2;
use bevy_egui::egui::{self, RichText};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum UvTool {
    #[default]
    Move,
    Rotate,
    Scale,
}

/// View + interaction state of the UV editor (UI-only, not undoable).
pub struct UvEditorState {
    pub tool: UvTool,
    pub zoom: f32,
    pub pan: egui::Vec2,
    pub show_checker: bool,
    drag: Option<canvas::UvDrag>,
}

impl Default for UvEditorState {
    fn default() -> Self {
        Self { tool: UvTool::Move, zoom: 1.0, pan: egui::Vec2::ZERO, show_checker: true, drag: None }
    }
}

/// Runs `f` on the active Unwrap UVW (with the corner -> vertex table), then
/// re-evaluates the stack. `label` records an undo step first.
pub(crate) fn edit(ed: &mut Editor, label: Option<&str>, f: impl FnOnce(&mut UnwrapMod, &[Vec<VertexId>])) {
    let Some(id) = ed.current() else { return };
    let Some(level) = ed.scene.objects[id].active_level else { return };
    let input = ed.scene.objects[id].input_of(level);
    if !matches!(ed.scene.objects[id].active_modifier(), Some(ModifierKind::Unwrap(u)) if u.matches(&input)) {
        return;
    }
    if let Some(l) = label {
        ed.checkpoint(l);
    }
    let cv = corner_vertices(&input);
    let o = &mut ed.scene.objects[id];
    if let Some(ModifierKind::Unwrap(u)) = o.active_modifier_mut() {
        f(u, &cv);
    }
    o.evaluate();
    ed.scene.propagate_from(id);
}

/// Applies `f` to the selection, or to every corner when nothing is selected.
fn on_selection_or_all(u: &mut UnwrapMod, f: impl FnOnce(&mut UnwrapMod)) {
    if u.selected.is_empty() {
        u.selected = u.all_corners().collect();
        f(u);
        u.selected.clear();
    } else {
        f(u);
    }
}

fn mode_button<T: PartialEq + Copy>(ui: &mut egui::Ui, cur: &mut T, v: T, label: &str, tip: &str) -> bool {
    let on = *cur == v;
    let text = if on { RichText::new(label).color(theme::INPUT).strong() } else { RichText::new(label) };
    let clicked = ui.add(egui::Button::new(text).selected(on).min_size(egui::vec2(28.0, 24.0))).on_hover_text(tip).clicked();
    if clicked {
        *cur = v;
    }
    clicked
}

/// (label, tooltip, transform) for the one-click UV transforms.
type UvOp = (&'static str, &'static str, fn(&mut UnwrapMod));

fn toolbar(ui: &mut egui::Ui, ed: &mut Editor, st: &mut UvEditorState) {
    ui.horizontal_wrapped(|ui| {
        mode_button(ui, &mut st.tool, UvTool::Move, "Move", "Drag the UV selection");
        mode_button(ui, &mut st.tool, UvTool::Rotate, "Rotate", "Drag left/right to rotate about the selection center");
        mode_button(ui, &mut st.tool, UvTool::Scale, "Scale", "Drag left/right to scale about the selection center");
        ui.separator();
        let mut sel = match ed.current_obj().and_then(|o| o.active_modifier()) {
            Some(ModifierKind::Unwrap(u)) => u.sel_mode,
            _ => UvSel::Vertex,
        };
        let mut changed = mode_button(ui, &mut sel, UvSel::Vertex, "Vertex", "UV vertex selection");
        changed |= mode_button(ui, &mut sel, UvSel::Edge, "Edge", "UV edge selection");
        changed |= mode_button(ui, &mut sel, UvSel::Face, "Face", "UV face selection");
        if changed {
            edit(ed, None, |u, _| u.sel_mode = sel);
        }
        ui.separator();
        if ui.button("Relax").on_hover_text("Smooth UV vertices (borders pinned); all when nothing is selected").clicked() {
            edit(ed, Some("UV Relax"), |u, cv| u.relax(cv, 10, 0.5));
        }
        if ui.button("Stitch").on_hover_text("Weld selected UV seams that share a 3D vertex").clicked() {
            let mut n = 0;
            edit(ed, Some("UV Stitch"), |u, cv| n = u.stitch(cv));
            ed.status = format!("Stitch: {n} vertices welded");
        }
        let xf: [UvOp; 4] = [
            ("Flip H", "Mirror horizontally", |u| u.scale_selected(Vec2::new(-1.0, 1.0))),
            ("Flip V", "Mirror vertically", |u| u.scale_selected(Vec2::new(1.0, -1.0))),
            ("⟲ 90", "Rotate +90°", |u| u.rotate_selected(std::f32::consts::FRAC_PI_2)),
            ("⟳ 90", "Rotate -90°", |u| u.rotate_selected(-std::f32::consts::FRAC_PI_2)),
        ];
        for (label, tip, f) in xf {
            if ui.button(label).on_hover_text(tip).clicked() {
                edit(ed, Some(label), |u, _| on_selection_or_all(u, f));
            }
        }
        ui.separator();
        if ui.button("✨ Smart UV").on_hover_text("Smart UV Unwrap with the rollout's settings").clicked() {
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
    ui.horizontal(|ui| {
        ui.checkbox(&mut st.show_checker, "Checker");
        if let Some(o) = ed.current_obj_mut() {
            ui.checkbox(&mut o.checker, "Checker in Viewport");
        }
        if ui.button("Fit View").clicked() {
            (st.zoom, st.pan) = (1.0, egui::Vec2::ZERO);
        }
        ui.label(dim("Click / drag-select (Ctrl add, Alt subtract) · drag selection to transform · MMB pan · wheel zoom"));
    });
}

pub fn show(ctx: &egui::Context, ed: &mut Editor, st: &mut UvEditorState) {
    if !ed.uv_editor_open {
        return;
    }
    let mut open = true;
    egui::Window::new(RichText::new("Edit UVWs").strong())
        .id(egui::Id::new("uv_editor"))
        .open(&mut open)
        .default_size([640.0, 600.0])
        .min_size([360.0, 320.0])
        .resizable(true)
        .collapsible(true)
        .show(ctx, |ui| {
            let state = ed.current_obj().map(|o| match (o.active_modifier(), o.active_level) {
                (Some(ModifierKind::Unwrap(u)), Some(i)) => Some(u.matches(&o.input_of(i))),
                _ => None,
            });
            match state.flatten() {
                None => {
                    ui.label("Select an object and highlight an Unwrap UVW modifier in its stack.");
                    st.drag = None;
                }
                Some(false) => {
                    ui.label(RichText::new("Topology below changed — press Reset UVWs in the Unwrap UVW rollout.").color(theme::AMBER));
                    st.drag = None;
                }
                Some(true) => {
                    toolbar(ui, ed, st);
                    ui.separator();
                    canvas::show(ui, ed, st);
                }
            }
        });
    if !open {
        ed.uv_editor_open = false;
        st.drag = None;
    }
}
