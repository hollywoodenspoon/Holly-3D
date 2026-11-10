//! Center 3D viewports: layout, 3ds Max navigation, gizmo, selection,
//! marquee, modal tools and the right-click quad menu.

mod gizmo;
mod nav;
mod overlay;
mod region;

use crate::editor::Editor;
use crate::editor::select::SelectOp;
use crate::viewport::layout::ScreenRect;
use bevy::math::Vec2;
use bevy_egui::egui::{self, PointerButton, Sense};
use transform_gizmo_egui::Gizmo;

#[derive(Default)]
pub struct ViewportUiState {
    gizmos: [Gizmo; 4],
    /// Region being drawn: viewport and its points (drag start, lasso trail, fence corners).
    region: Option<(usize, Vec<Vec2>)>,
    /// The fence is waiting for more clicks (its first side is drawn).
    fence_open: bool,
    gizmo_dragging: bool,
    nav: Option<usize>,
    /// Viewport holding the current paint stroke.
    painting: Option<usize>,
}

fn v2(p: egui::Pos2) -> Vec2 {
    Vec2::new(p.x, p.y)
}

pub fn show(ui: &mut egui::Ui, ed: &mut Editor, st: &mut ViewportUiState, area: egui::Rect) {
    ed.viewports.layout(ScreenRect { min: v2(area.min), max: v2(area.max) }, 3.0);
    ed.sync_camera_views();
    let (mods, scroll, delta, mmb_down, pointer) =
        ui.input(|i| (i.modifiers, i.smooth_scroll_delta.y, i.pointer.delta(), i.pointer.middle_down(), i.pointer.hover_pos()));
    for vi in 0..4 {
        if !ed.viewports.views[vi].visible {
            continue;
        }
        let r = ed.viewports.views[vi].rect;
        let rect = egui::Rect::from_min_max(egui::pos2(r.min.x, r.min.y), egui::pos2(r.max.x, r.max.y));
        let resp = ui.interact(rect, egui::Id::new(("viewport", vi)), Sense::click_and_drag());
        let hovered = resp.hovered() || resp.dragged();
        if hovered && ui.input(|i| i.pointer.any_pressed()) {
            ed.viewports.active = vi;
        }
        // ---- navigation: MMB pan, Alt+MMB orbit, Ctrl+Alt+MMB zoom, wheel zoom
        if mmb_down && (st.nav == Some(vi) || (st.nav.is_none() && resp.hovered())) {
            st.nav = Some(vi);
            let d = Vec2::new(delta.x, delta.y);
            let h = r.size().y;
            let is_cam = ed.viewports.views[vi].cam_obj.is_some();
            let target = ed.viewports.views[vi].camera.target;
            // Camera views orbit around the camera's target (Orbit Camera).
            let pivot = if is_cam { target } else { ed.selection_center().unwrap_or(target) };
            let cam = &mut ed.viewports.views[vi].camera;
            let vp = &ed.prefs.viewport;
            if mods.alt && (mods.ctrl || mods.command) {
                cam.zoom(vp.zoom_steps(-d.y * 0.02));
            } else if mods.alt {
                cam.orbit(d * vp.orbit_speed, pivot);
            } else {
                cam.pan(d * vp.pan_speed, h);
            }
            ed.push_view_to_camera(vi);
        }
        if !mmb_down && st.nav == Some(vi) {
            st.nav = None;
        }
        if resp.hovered() && scroll.abs() > 0.0 {
            let steps = ed.prefs.viewport.zoom_steps(scroll / 60.0);
            ed.viewports.views[vi].camera.zoom(steps);
            ed.push_view_to_camera(vi);
        }
        // ---- Viewport Paint: left button paints, right-click exits
        if ed.paint.active {
            let (pressed, down) = ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_down()));
            if pressed && resp.hovered() && !mods.alt {
                if let Some(p) = pointer {
                    ed.paint_begin(vi, v2(p));
                    st.painting = Some(vi);
                }
            } else if down && st.painting == Some(vi) {
                let moved = delta.length_sq() > 0.0 || ed.paint.brush.tool == crate::paint::BrushTool::Blur;
                if let (Some(p), true) = (pointer, moved) {
                    ed.paint_drag(vi, v2(p));
                }
            }
            if !down && st.painting == Some(vi) {
                ed.paint_end();
                st.painting = None;
            }
            if resp.clicked_by(PointerButton::Secondary) {
                ed.cancel_tool();
            }
            overlay::draw(ui, ed, vi, rect, None, pointer.map(v2));
            continue;
        }
        // ---- Line tool: press = knot, drag = Bezier handles, right-click ends
        if matches!(ed.tool, crate::editor::Tool::Line { .. }) {
            let (pressed, down) = ui.input(|i| (i.pointer.primary_pressed(), i.pointer.primary_down()));
            if let Some(p) = pointer {
                if pressed && resp.hovered() && !mods.alt {
                    ed.line_press(vi, v2(p));
                } else if down && resp.dragged() {
                    ed.line_drag(vi, v2(p));
                }
            }
            if !down {
                ed.line_release();
            }
            if resp.clicked_by(PointerButton::Secondary) {
                ed.cancel_tool();
            }
            overlay::draw(ui, ed, vi, rect, None, pointer.map(v2));
            continue;
        }
        // ---- navigation buttons: left-drag zooms / pans / orbits
        if nav::drag(ui, ed, &resp, vi, r.size().y) {
            overlay::draw(ui, ed, vi, rect, None, pointer.map(v2));
            continue;
        }
        // ---- gizmo (active viewport only)
        let mut gizmo_hot = false;
        if vi == ed.viewports.active && ed.tool == crate::editor::Tool::None {
            gizmo_hot = gizmo::run(ui, ed, &mut st.gizmos[vi], vi, resp.hovered(), &mut st.gizmo_dragging);
        }
        // ---- clicks, marquee, tools
        let ctrl = mods.ctrl || mods.command;
        let op = if ctrl {
            SelectOp::Add
        } else if mods.alt {
            SelectOp::Subtract
        } else {
            SelectOp::Replace
        };
        if st.fence_open && st.region.as_ref().is_some_and(|r| r.0 == vi) {
            region::fence_clicks(ed, st, &resp, vi, op, pointer.map(v2));
            overlay::draw(ui, ed, vi, rect, st.region.as_ref().map(|r| r.1.as_slice()), pointer.map(v2));
            continue;
        }
        if !gizmo_hot && !st.gizmo_dragging {
            if resp.drag_started_by(PointerButton::Primary)
                && ed.tool == crate::editor::Tool::None
                && let Some(p) = resp.interact_pointer_pos()
            {
                st.region = Some((vi, vec![v2(p) - Vec2::new(delta.x, delta.y)]));
            }
            if resp.clicked_by(PointerButton::Primary)
                && let Some(p) = resp.interact_pointer_pos()
                && !ed.tool_click(vi, v2(p))
            {
                ed.click_select(vi, v2(p), op);
            }
            if resp.double_clicked() && ed.sub.is_edge_like() {
                ed.edit_selection(|m, _| m.select_loop());
            }
        }
        region::drag(ed, st, &resp, vi, op, pointer.map(v2));
        if resp.clicked_by(PointerButton::Secondary) && !ed.cancel_tool() {
            ed.quad_menu = resp.interact_pointer_pos().map(v2);
        }
        overlay::draw(ui, ed, vi, rect, st.region.as_ref().filter(|r| r.0 == vi).map(|r| r.1.as_slice()), pointer.map(v2));
    }
    nav::buttons(ui, ed, area);
}
