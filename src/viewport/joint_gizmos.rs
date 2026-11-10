//! Viewport wireframes for mech joints: the mount, a link back to the
//! parent's pivot, the hinge axis with its yellow angle-limit arc and
//! current-angle tick, the ball socket's swing cone, the slider rail and
//! the piston line. Joints of the selected parts are drawn bright.

use super::lines::OverlayLines;
use crate::assembly::{Axis, Frame, JointKind};
use crate::editor::{Editor, Tool};
use bevy::prelude::*;

const YELLOW: Color = Color::srgb(1.0, 0.86, 0.1);
const CYAN: Color = Color::srgb(0.2, 0.85, 1.0);
const DIM: Color = Color::srgba(0.85, 0.75, 0.3, 0.35);

/// A vector perpendicular to the joint axis (the arc's zero direction).
fn reference(axis: Axis) -> Vec3 {
    match axis {
        Axis::X => Vec3::Y,
        Axis::Y => Vec3::Z,
        Axis::Z => Vec3::X,
    }
}

fn arc(g: &mut Gizmos<OverlayLines>, w: Frame, axis: Axis, from: f32, to: f32, r: f32, color: Color) {
    let n = (((to - from).abs() / 6.0).ceil() as usize).clamp(2, 64);
    let at = |i: usize| {
        let a = (from + (to - from) * i as f32 / n as f32).to_radians();
        w.point(Quat::from_axis_angle(axis.vec(), a) * reference(axis) * r)
    };
    for i in 0..n {
        g.line(at(i), at(i + 1), color);
    }
}

fn mount(g: &mut Gizmos<OverlayLines>, w: Frame, r: f32, color: Color) {
    arc(g, w, Axis::Z, 0.0, 360.0, r * 0.25, color);
}

/// Draws every joint; `bright` ones belong to selected parts.
pub fn draw(g: &mut Gizmos<OverlayLines>, ed: &Editor) {
    let objs = &ed.scene.objects;
    for j in &ed.scene.assembly.joints {
        let (Some(c), Some(p)) = (objs.get(j.child), objs.get(j.parent)) else { continue };
        if c.hidden && p.hidden {
            continue;
        }
        let bright = c.selected || p.selected;
        let r = c.bounds().map_or(0.5, |b| b.size().max_element() * 0.45).clamp(0.1, 50.0);
        let pf = Frame::of(&p.transform);
        let w = pf * j.parent_frame;
        let col = if bright { YELLOW } else { DIM };
        match &j.kind {
            JointKind::Piston => {
                let h = Frame::of(&c.transform) * j.child_frame;
                g.line(h.pos, w.pos, if bright { CYAN } else { DIM });
                mount(g, h, r, col);
                mount(g, w, r, col);
                continue;
            }
            JointKind::Rigid => mount(g, w, r, col),
            JointKind::Revolute { axis, angle, limits } => {
                let a = w.rot * axis.vec() * r;
                g.line(w.pos - a, w.pos + a, col);
                if bright {
                    if limits.on {
                        arc(g, w, *axis, limits.min, limits.max, r, YELLOW);
                    } else {
                        arc(g, w, *axis, 0.0, 360.0, r, DIM);
                    }
                    g.line(w.pos, w.point(Quat::from_axis_angle(axis.vec(), angle.to_radians()) * reference(*axis) * r * 1.15), CYAN);
                }
            }
            JointKind::Ball { rot, cone, .. } => {
                mount(g, w, r, col);
                if bright {
                    let c = cone.min(179.0).to_radians();
                    let rim = |t: f32| w.point(Quat::from_rotation_z(t) * Quat::from_rotation_x(c) * Vec3::Z * r);
                    for k in 0..24 {
                        let (t0, t1) = (k as f32 / 24.0 * std::f32::consts::TAU, (k + 1) as f32 / 24.0 * std::f32::consts::TAU);
                        g.line(rim(t0), rim(t1), YELLOW);
                        if k % 6 == 0 {
                            g.line(w.pos, rim(t0), DIM);
                        }
                    }
                    g.line(w.pos, w.point(*rot * Vec3::Z * r * 1.15), CYAN);
                }
            }
            JointKind::Slider { axis, travel, limits } => {
                let (lo, hi) = if limits.on { (limits.min, limits.max) } else { (-r, r) };
                let d = w.rot * axis.vec();
                g.line(w.pos + d * lo, w.pos + d * hi, col);
                if bright {
                    let tick = w.rot * reference(*axis) * r * 0.2;
                    let at = w.pos + d * *travel;
                    g.line(at - tick, at + tick, CYAN);
                }
            }
        }
        if bright {
            g.line(pf.pos, w.pos, DIM);
        }
    }
    // The first J-tool pick, until the parent is clicked.
    if let Tool::Joint { first: Some(m) } = &ed.tool {
        let w = Frame::from_normal(m.point, m.normal);
        mount(g, w, 1.0, CYAN);
        g.line(m.point, m.point + m.normal * 0.5, CYAN);
    }
}
