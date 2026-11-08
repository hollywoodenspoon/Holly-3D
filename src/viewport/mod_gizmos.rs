//! Viewport wireframes for modifier gizmos: the FFD lattice cage (yellow) and
//! the UVW Map projection gizmo (yellow, orange while its sub-object is active).

use super::lines::OverlayLines;
use crate::editor::Editor;
use crate::modifiers::{MapType, ModSub, ModifierKind};
use bevy::prelude::*;
use std::f32::consts::TAU;

const YELLOW: Color = Color::srgb(1.0, 0.86, 0.1);
const ORANGE: Color = Color::srgb(1.0, 0.55, 0.1);
const SEAM: Color = Color::srgb(0.25, 0.9, 0.35);

fn circle(g: &mut Gizmos<OverlayLines>, m: Mat4, z: f32, axis: usize, color: Color) {
    let n = 32;
    let at = |i: usize| {
        let a = i as f32 / n as f32 * TAU;
        let (c, s) = (a.cos() * 0.5, a.sin() * 0.5);
        let p = match axis {
            0 => Vec3::new(z, c, s),
            1 => Vec3::new(c, z, s),
            _ => Vec3::new(c, s, z),
        };
        m.transform_point3(p)
    };
    for i in 0..n {
        g.line(at(i), at(i + 1), color);
    }
}

fn cube(g: &mut Gizmos<OverlayLines>, m: Mat4, color: Color) {
    let c = |i: usize| m.transform_point3(Vec3::new((i & 1) as f32 - 0.5, (i >> 1 & 1) as f32 - 0.5, (i >> 2 & 1) as f32 - 0.5));
    for i in 0..8usize {
        for bit in [1, 2, 4] {
            if i & bit == 0 {
                g.line(c(i), c(i | bit), color);
            }
        }
    }
}

pub fn draw(g: &mut Gizmos<OverlayLines>, ed: &Editor) {
    let Some(o) = ed.current_obj() else { return };
    if o.hidden {
        return;
    }
    let w = o.world_matrix();
    let sub = ed.active_mod_sub();
    match o.active_modifier() {
        Some(ModifierKind::Ffd(f)) => {
            let color = if sub == Some(ModSub::Lattice) { ORANGE } else { YELLOW };
            for (a, b) in f.cage_edges() {
                g.line(w.transform_point3(f.point_pos(a)), w.transform_point3(f.point_pos(b)), color);
            }
        }
        Some(ModifierKind::UvwMap(u)) => {
            let color = if sub == Some(ModSub::Gizmo) { ORANGE } else { YELLOW };
            let m = w * u.gizmo_matrix();
            let p = |x: f32, y: f32, z: f32| m.transform_point3(Vec3::new(x, y, z));
            match u.kind {
                MapType::Planar | MapType::XyzToUvw | MapType::Face => {
                    let c = [p(-0.5, -0.5, 0.0), p(0.5, -0.5, 0.0), p(0.5, 0.5, 0.0), p(-0.5, 0.5, 0.0)];
                    for k in 0..4 {
                        g.line(c[k], c[(k + 1) % 4], color);
                    }
                    // 3ds Max marks the top (V = 1) edge in green and the U direction
                    g.line(c[2], c[3], SEAM);
                    g.line(p(0.0, 0.0, 0.0), p(0.0, 0.0, 0.15), color);
                }
                MapType::Cylindrical => {
                    circle(g, m, -0.5, 2, color);
                    circle(g, m, 0.5, 2, color);
                    for k in 0..4 {
                        let a = k as f32 / 4.0 * TAU;
                        let (x, y) = (a.cos() * 0.5, a.sin() * 0.5);
                        g.line(p(x, y, -0.5), p(x, y, 0.5), if k == 2 { SEAM } else { color });
                    }
                }
                MapType::Spherical | MapType::ShrinkWrap => {
                    for axis in 0..3 {
                        circle(g, m, 0.0, axis, color);
                    }
                    g.line(p(0.0, 0.0, 0.0), p(-0.5, 0.0, 0.0), SEAM);
                }
                MapType::Box => cube(g, m, color),
            }
        }
        _ => {}
    }
}
