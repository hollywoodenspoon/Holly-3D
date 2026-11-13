//! Viewport wireframes for modifier gizmos: the FFD lattice cage, the UVW Map
//! projection gizmo, the Bend / Twist / Taper gizmo box and center, and the
//! Symmetry mirror plane and Battlefield blast zones (yellow, orange while their sub-object is active).

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

/// Deformer gizmo box, refitted only when the object re-evaluates.
#[derive(Default)]
pub struct BoxCache(Option<(u64, Mat4)>);

fn cross(g: &mut Gizmos<OverlayLines>, p: Vec3, size: f32, color: Color) {
    for d in [Vec3::X, Vec3::Y, Vec3::Z] {
        g.line(p - d * size, p + d * size, color);
    }
}

pub fn draw(g: &mut Gizmos<OverlayLines>, ed: &Editor, cache: &mut BoxCache) {
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
                    // Mark the top (V = 1) edge in green and the U direction
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
        Some(ModifierKind::Symmetry(s)) => {
            let color = if sub == Some(ModSub::Mirror) { ORANGE } else { YELLOW };
            let size = o.evaluated_mesh.bounds().map(|b| b.size().max_element() * 0.6).unwrap_or(1.0);
            let a = s.axis.min(2);
            let (u, v) = (s.mirror.transform_vector3(Vec3::AXES[(a + 1) % 3]), s.mirror.transform_vector3(Vec3::AXES[(a + 2) % 3]));
            let p = s.point();
            let c = [p - u * size - v * size, p + u * size - v * size, p + u * size + v * size, p - u * size + v * size]
                .map(|q| w.transform_point3(q));
            for k in 0..4 {
                g.line(c[k], c[(k + 1) % 4], color);
            }
            g.line(w.transform_point3(p), w.transform_point3(p + s.normal() * size * 0.3), color);
        }
        Some(ModifierKind::Battlefield(b)) => {
            // Blast zones: three great circles per sphere, orange when selected.
            let active = sub == Some(ModSub::BlastZones);
            for z in &b.zones {
                let color = if active && z.selected { ORANGE } else { YELLOW };
                let m = w * Mat4::from_scale_rotation_translation(Vec3::splat(z.radius * 2.0), Quat::IDENTITY, z.center);
                for axis in 0..3 {
                    circle(g, m, 0.0, axis, color);
                }
            }
        }
        Some(k) if k.deformer_gizmo().is_some() => {
            let (gz, center) = k.deformer_gizmo().unwrap_or_default();
            if cache.0.is_none_or(|(v, _)| v != o.version) {
                let input = o.active_level.map(|l| o.input_of(l));
                let fitted = input.and_then(|m| crate::modifiers::deformers::gizmo_box(&m, gz));
                cache.0 = fitted.map(|b| (o.version, b));
            }
            if let Some((_, b)) = cache.0 {
                cube(g, w * b, if sub == Some(ModSub::Gizmo) { ORANGE } else { YELLOW });
                let size = (w * b).transform_vector3(Vec3::ONE).length() * 0.06;
                cross(g, w.transform_point3(gz.transform_point3(center)), size, if sub == Some(ModSub::Center) { ORANGE } else { YELLOW });
            }
        }
        _ => {}
    }
}
