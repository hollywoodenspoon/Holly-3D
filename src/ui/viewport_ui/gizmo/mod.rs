//! Ilex's Move / Rotate / Scale gizmos, wired to object transforms,
//! sub-object selections, pivots and the Slice Plane.
//!
//! Move: X/Y/Z arrows, plane squares and a centre square for the view plane.
//! Rotate: X/Y/Z rings, a grey view ring outside and a trackball inside.
//! Scale: X/Y/Z axes, plane strips and an inner triangle for uniform scale.

mod drag_math;
mod draw;
mod handles;
#[cfg(test)]
mod tests;

use crate::editor::snap::SnapKind;
use crate::editor::{CoordSystem, Editor, PivotMode, TransformMode};
use crate::scene::{ObjId, Xform};
use bevy::math::{Quat, Vec2, Vec3};
use bevy_egui::egui;
use drag_math::Delta;
use handles::{Gz, Handle, VIEW_RING, plane_axes};

/// Targets for the gizmo: object list (object mode) or a single frame.
#[derive(Clone)]
enum Targets {
    Objects(Vec<ObjId>, Vec<Xform>),
    Frame(Xform),
}

fn targets(ed: &Editor) -> Option<Targets> {
    if ed.active_mod_sub().is_some() {
        return ed.mod_sub_frame().map(Targets::Frame);
    }
    let local_rot = || match ed.coord {
        CoordSystem::Local => ed.current_obj().map(|o| o.transform.rotation).unwrap_or(Quat::IDENTITY),
        _ => Quat::IDENTITY,
    };
    if ed.spline_level().is_some() {
        let center = ed.selection_center()?;
        return Some(Targets::Frame(Xform { translation: center, rotation: local_rot(), scale: Vec3::ONE }));
    }
    if ed.in_sub_object() {
        if let Some(sp) = &ed.slice_plane {
            return Some(Targets::Frame(sp.xform));
        }
        let center = ed.selection_center()?;
        return Some(Targets::Frame(Xform { translation: center, rotation: local_rot(), scale: Vec3::ONE }));
    }
    let ids = ed.scene.selected();
    if ids.is_empty() {
        return None;
    }
    if ed.affect_pivot_only {
        let o = &ed.scene.objects[ids[0]];
        return Some(Targets::Frame(Xform { translation: o.transform.translation, ..Default::default() }));
    }
    let xs = ids.iter().map(|&k| ed.scene.objects[k].transform).collect();
    Some(Targets::Objects(ids, xs))
}

/// Gizmo centre and orientation for the targets.
fn place(ed: &Editor, t: &Targets) -> (Vec3, Quat) {
    let local = ed.coord == CoordSystem::Local;
    match t {
        Targets::Frame(x) => (x.translation, if local { x.rotation } else { Quat::IDENTITY }),
        Targets::Objects(_, xs) => {
            let c = xs.iter().map(|x| x.translation).sum::<Vec3>() / xs.len().max(1) as f32;
            (c, if local { xs[0].rotation } else { Quat::IDENTITY })
        }
    }
}

/// A drag in progress (it starts once the cursor has moved a little).
struct Drag {
    mode: TransformMode,
    handle: Handle,
    targets: Targets,
    center: Vec3,
    rot: Quat,
    grab: Vec2,
    started: bool,
    /// Move: grab point along the axis, or on the plane.
    s0: f32,
    p0: Vec3,
    /// Rotate ring: grab angle on the ring and the ring's screen tangent there.
    theta0: f32,
    tangent: Vec2,
    delta: Delta,
}

#[derive(Default)]
pub struct GizmoState {
    hot: Option<Handle>,
    drag: Option<Drag>,
}

fn effective_mode(ed: &Editor) -> TransformMode {
    let pivot_or_plane = ed.affect_pivot_only || ed.slice_plane.is_some() && ed.mode == TransformMode::Scale;
    if pivot_or_plane && ed.mode != TransformMode::Select { TransformMode::Move } else { ed.mode }
}

/// Runs the gizmo in `view`. Returns true while the pointer is over / dragging it.
pub fn run(ui: &egui::Ui, ed: &mut Editor, st: &mut GizmoState, view: usize, hovered: bool, dragging: &mut bool) -> bool {
    let (cursor, pressed, down, shift) = ui.input(|i| {
        let c = i.pointer.hover_pos().unwrap_or_default();
        (Vec2::new(c.x, c.y), i.pointer.primary_pressed(), i.pointer.primary_down(), i.modifiers.shift)
    });
    let mode = effective_mode(ed);
    let size = ed.prefs.viewport.gizmo_size.clamp(30.0, 300.0);
    let v = ed.viewports.views[view].clone();
    let painter = ui.painter().with_clip_rect(egui::Rect::from_min_max(egui::pos2(v.rect.min.x, v.rect.min.y), egui::pos2(v.rect.max.x, v.rect.max.y)));
    if st.drag.is_none() {
        st.hot = None;
        if mode == TransformMode::Select || !ed.prefs.viewport.show_gizmo {
            return false;
        }
        let Some(t) = targets(ed) else { return false };
        let (center, rot) = place(ed, &t);
        let Some(gz) = Gz::new(&v, center, rot, size) else { return false };
        st.hot = if hovered { gz.pick(mode, cursor) } else { None };
        draw::draw(&painter, &gz, mode, &draw::Look { hot: st.hot, sweep: None, readout: None });
        if let (true, Some(handle)) = (pressed, st.hot) {
            st.drag = Some(grab(&gz, mode, handle, t, cursor));
        }
        *dragging = false;
        return st.hot.is_some() || st.drag.is_some();
    }
    let d = st.drag.as_mut().expect("drag");
    if !d.started && down && (cursor - d.grab).length() > 3.0 {
        d.started = true;
        let start = match &d.targets {
            Targets::Frame(x) => x.matrix(),
            Targets::Objects(_, xs) => xs[0].matrix(),
        };
        ed.begin_drag(start, shift);
    }
    if d.started && down {
        let gz = Gz::new(&v, d.center, d.rot, size);
        if let Some(gz) = &gz {
            d.delta = delta(ed, &v, gz, d, cursor, view);
        }
        apply(ed, d);
    }
    // Draw at the moved position, highlighting the dragged handle.
    let moved = match d.delta {
        Delta::Move(m) => d.center + m,
        _ => d.center,
    };
    if let Some(gz) = Gz::new(&v, moved, d.rot, size) {
        let text = d.started.then(|| readout(d));
        let sweep = d.started.then(|| sweep(&gz, &v, d)).flatten();
        draw::draw(&painter, &gz, d.mode, &draw::Look { hot: Some(d.handle), sweep, readout: text.as_deref() });
    }
    if !down {
        if d.started {
            ed.end_drag();
        }
        st.drag = None;
    }
    *dragging = st.drag.as_ref().is_some_and(|d| d.started);
    true
}

/// Records where the drag grabbed the handle.
fn grab(gz: &Gz, mode: TransformMode, handle: Handle, targets: Targets, cursor: Vec2) -> Drag {
    let ray = gz.v.ray(cursor);
    let (mut s0, mut p0, mut theta0, mut tangent) = (0.0, gz.center, 0.0, Vec2::ZERO);
    let fwd = gz.v.camera.forward();
    match (mode, handle) {
        (TransformMode::Move, Handle::Axis(k)) => s0 = drag_math::axis_param(&ray, gz.center, gz.axes[k]).unwrap_or(0.0),
        (TransformMode::Move, Handle::Plane(k)) => p0 = drag_math::plane_hit(&ray, gz.center, gz.axes[k]).unwrap_or(gz.center),
        (TransformMode::Move, Handle::Screen) => p0 = drag_math::plane_hit(&ray, gz.center, fwd).unwrap_or(gz.center),
        (TransformMode::Rotate, Handle::Axis(k)) => {
            theta0 = gz.ring_hit(k, cursor).map(|h| h.1).unwrap_or(0.0);
            let at = gz.ring_point(k, theta0);
            let t3 = gz.axes[k].cross(at - gz.center).normalize_or_zero() * gz.len * 0.05;
            tangent = gz.s(at + t3) - gz.s(at);
            if tangent.length() < 0.5 {
                tangent = (cursor - gz.c2).perp();
            }
        }
        _ => {}
    }
    let delta = match mode {
        TransformMode::Rotate => Delta::Rotate(Vec3::Z, 0.0),
        TransformMode::Scale => Delta::Scale(Vec3::ONE),
        _ => Delta::Move(Vec3::ZERO),
    };
    Drag { mode, handle, targets, center: gz.center, rot: gz.rot, grab: cursor, started: false, s0, p0, theta0, tangent, delta }
}

/// The change for the current cursor position (with snaps).
fn delta(ed: &Editor, v: &crate::viewport::layout::ViewState, gz: &Gz, d: &Drag, cursor: Vec2, view: usize) -> Delta {
    let cam = &v.camera;
    let (c, axes) = (d.center, gz.axes);
    match d.mode {
        TransformMode::Rotate => {
            let r = gz.ring_px();
            let (axis, a) = match d.handle {
                Handle::Axis(k) => (axes[k], drag_math::ring_angle(d.grab, cursor, d.tangent, r)),
                Handle::ViewRing => (cam.forward(), drag_math::view_angle(gz.c2, d.grab, cursor)),
                _ => drag_math::trackball(cam.right(), cam.up(), cursor - d.grab, r),
            };
            let a = if ed.angle_snap { drag_math::snap(a, 5f32.to_radians()) } else { a };
            Delta::Rotate(axis, a)
        }
        TransformMode::Scale => {
            let screen_dir = |k: usize| gz.along(k, 1.0) - gz.c2;
            let f = match d.handle {
                Handle::Axis(k) => drag_math::scale_factor(d.grab, cursor, screen_dir(k), gz.px),
                Handle::Plane(k) => {
                    let (a, b) = plane_axes(k);
                    drag_math::scale_factor(d.grab, cursor, screen_dir(a) + screen_dir(b), gz.px)
                }
                _ => drag_math::uniform_factor(d.grab, cursor, gz.px),
            };
            let f = if ed.iface.percent_snap { drag_math::snap(f, 0.1).max(0.1) } else { f };
            let mut s = Vec3::ONE;
            match d.handle {
                Handle::Axis(k) => s[k] = f,
                Handle::Plane(k) => {
                    let (a, b) = plane_axes(k);
                    (s[a], s[b]) = (f, f);
                }
                _ => s = Vec3::splat(f),
            }
            Delta::Scale(s)
        }
        _ => {
            let ray = v.ray(cursor);
            let constrain = |m: Vec3| match d.handle {
                Handle::Axis(k) => axes[k] * m.dot(axes[k]),
                Handle::Plane(k) => m - axes[k] * m.dot(axes[k]),
                _ => m,
            };
            let raw = match d.handle {
                Handle::Axis(k) => drag_math::axis_param(&ray, c, axes[k]).map(|s| axes[k] * (s - d.s0)),
                Handle::Plane(k) => drag_math::plane_hit(&ray, c, axes[k]).map(|h| constrain(h - d.p0)),
                _ => drag_math::plane_hit(&ray, c, cam.forward()).map(|h| h - d.p0),
            };
            let mut m = raw.unwrap_or(match d.delta {
                Delta::Move(m) => m,
                _ => Vec3::ZERO,
            });
            if ed.snap {
                match ed.snap_at(view, cursor).filter(|h| h.kind != SnapKind::Grid) {
                    Some(h) => m = constrain(h.pos - c),
                    None if ed.snap_cfg.grid => m = drag_math::snap_move(m, d.rot, ed.snap_cfg.grid_size),
                    None => {}
                }
            }
            Delta::Move(m)
        }
    }
}

/// Applies the drag's change to its targets through the editor.
fn apply(ed: &mut Editor, d: &Drag) {
    match &d.targets {
        Targets::Objects(ids, xs) => {
            let own = ed.pivot_mode == PivotMode::PivotPoint;
            let local = ed.coord == CoordSystem::Local;
            let pairs: Vec<(ObjId, Xform)> = ids
                .iter()
                .zip(xs)
                .map(|(&id, x)| {
                    if !own {
                        return (id, d.delta.apply(x, d.center, d.rot));
                    }
                    // Each object about its own pivot (and own axes in Local).
                    let rot = if local { x.rotation } else { d.rot };
                    let delta = match (d.delta, d.handle) {
                        (Delta::Rotate(_, a), Handle::Axis(k)) if local => Delta::Rotate(x.rotation * Vec3::AXES[k], a),
                        (other, _) => other,
                    };
                    (id, delta.apply(x, x.translation, rot))
                })
                .collect();
            ed.drag_objects(&pairs);
        }
        Targets::Frame(x) => ed.drag_frame(d.delta.matrix(d.center, d.rot) * x.matrix()),
    }
}

/// Value shown next to the gizmo while dragging.
fn readout(d: &Drag) -> String {
    match d.delta {
        Delta::Move(m) => format!("{:.3}, {:.3}, {:.3}", m.x, m.y, m.z),
        Delta::Rotate(_, a) => format!("{:.1}°", a.to_degrees()),
        Delta::Scale(s) => {
            let f = match d.handle {
                Handle::Axis(k) => s[k],
                Handle::Plane(k) => s[plane_axes(k).0],
                _ => s.x,
            };
            format!("{:.0} %", f * 100.0)
        }
    }
}

/// Pie wedge for a ring or view-ring rotation.
fn sweep(gz: &Gz, v: &crate::viewport::layout::ViewState, d: &Drag) -> Option<draw::Sweep> {
    let Delta::Rotate(_, angle) = d.delta else { return None };
    match d.handle {
        Handle::Axis(k) => {
            let (a, b) = plane_axes(k);
            Some(draw::Sweep { center: d.center, u: gz.axes[a], w: gz.axes[b], radius: gz.len, from: d.theta0, angle })
        }
        Handle::ViewRing => {
            let g = d.grab - gz.c2;
            let (u, w) = (v.camera.right(), -v.camera.up());
            Some(draw::Sweep { center: d.center, u, w, radius: gz.len * VIEW_RING, from: g.y.atan2(g.x), angle })
        }
        _ => None,
    }
}
