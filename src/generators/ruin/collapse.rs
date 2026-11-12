//! Warzone Collapse and Apocalyptic Ruin. The fracture is a jagged
//! surface (the lowest of three tilted planes around the failed storey's
//! slab), cut through the mesh with Slice. Above it, Collapse either
//! topples the upper block over the edge on the side that lost support
//! (tall, lopsided failures) or pancakes it into stacked slabs on the
//! stump; Ruin drops everything above a low stump. The broken top gets
//! rebar and bent I-beams (Modern), charred timbers (Medieval) or moss
//! (Ancient).

use super::RuinEra as Era;
use super::damage::decal;
use super::stress::{STOREY, Stress};
use crate::generators::kit::{Kit, ids};
use crate::mesh::{Aabb, PolyId};
use crate::modifiers::battlefield::debris;
use bevy::math::{Mat4, Quat, Vec2, Vec3};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

/// Fracture surface: z = min over tilted planes through (centre, h_i).
struct Surface {
    c: Vec2,
    planes: Vec<(f32, Vec2)>,
}

impl Surface {
    fn new(c: Vec2, h: f32, rough: f32, r: &mut ChaCha8Rng) -> Self {
        let planes =
            (0..3).map(|_| (h + r.gen_range(-rough..rough), Vec2::new(r.gen_range(-0.35..0.35), r.gen_range(-0.35..0.35)))).collect();
        Self { c, planes }
    }
    fn z(&self, p: Vec2) -> f32 {
        self.planes.iter().map(|(h, s)| h + s.dot(p - self.c)).fold(f32::MAX, f32::min)
    }
    /// Cuts the mesh along every plane (only polygons spanning the band).
    fn cut(&self, kit: &mut Kit, band: (f32, f32)) {
        for (h, s) in &self.planes {
            let m = &mut kit.m;
            m.clear_selection();
            let span: Vec<PolyId> = m
                .polygons
                .keys()
                .filter(|&p| {
                    let (lo, hi) =
                        m.polygons[p].vertex_ids().map(|v| m.pos(v).z).fold((f32::MAX, f32::MIN), |(a, b), z| (a.min(z), b.max(z)));
                    hi > band.0 && lo < band.1
                })
                .collect();
            span.iter().for_each(|&p| m.polygons[p].selected = true);
            let n = Vec3::new(-s.x, -s.y, 1.0).normalize();
            crate::mesh::ops::slice::slice(m, self.c.extend(*h), n, true, false);
            m.clear_selection();
        }
    }
    /// Polygons whose centroid is above the surface.
    fn above(&self, kit: &Kit) -> Vec<PolyId> {
        kit.m
            .polygons
            .keys()
            .filter(|&p| {
                let c = kit.m.poly_centroid(p);
                c.z > self.z(c.truncate()) + 0.01
            })
            .collect()
    }
}

/// Rebar, I-beams, timbers or moss along the broken top.
fn broken_top(kit: &mut Kit, b: &Aabb, s: &Surface, era: Era, r: &mut ChaCha8Rng) {
    let half = Vec2::new(b.max.x - b.min.x, b.max.y - b.min.y) * 0.5;
    let n = ((half.x + half.y) * 0.5).clamp(3.0, 14.0) as usize;
    for _ in 0..n {
        let edge = r.gen_range(0..4);
        let t = r.gen_range(-0.9..0.9);
        let p = s.c
            + match edge {
                0 => Vec2::new(t * half.x, -half.y),
                1 => Vec2::new(half.x, t * half.y),
                2 => Vec2::new(t * half.x, half.y),
                _ => Vec2::new(-half.x, t * half.y),
            };
        let top = p.extend(s.z(p).min(b.max.z));
        match era {
            Era::Modern | Era::Auto => {
                debris::rebar_tuft(kit, r, top, Vec2::splat(0.3), 2);
                if r.gen_bool(0.25) {
                    // A bent I-beam: flange-sized bar with a kink.
                    let a = r.gen_range(0.0..std::f32::consts::TAU);
                    let mid = top + Vec3::Z * r.gen_range(0.8..2.0);
                    let end = mid + Vec3::new(a.cos(), a.sin(), -0.3) * r.gen_range(1.0..2.5);
                    kit.mat(ids::IRON).beam(top, mid, 0.22, 0.3);
                    kit.beam(mid, end, 0.22, 0.3);
                }
            }
            Era::Medieval => {
                if r.gen_bool(0.5) {
                    let a = r.gen_range(0.0..std::f32::consts::TAU);
                    let end = top + Vec3::new(a.cos(), a.sin(), r.gen_range(0.3..1.2)) * r.gen_range(1.0..2.6);
                    kit.mat(ids::CHARRED).beam(top - Vec3::Z * 0.2, end, 0.22, 0.26);
                }
            }
            Era::Ancient => {
                let up = (top.truncate() - s.c).normalize_or(Vec2::X).extend(0.0);
                decal(kit, top - up * 0.2, Vec3::Z, r.gen_range(0.6..1.6), r.gen_range(0.4..1.0), r.gen_range(0.0..3.0), ids::FOLIAGE);
            }
        }
    }
}

/// Warzone Collapse: the failed storey gives way.
pub fn collapse(kit: &mut Kit, b: &Aabb, st: &Stress, era: Era, r: &mut ChaCha8Rng) {
    let h = st.fracture(b, r);
    let c = Vec2::new(b.min.x + b.max.x, b.min.y + b.max.y) * 0.5;
    let s = Surface::new(c, h, STOREY * 0.6, r);
    s.cut(kit, (h - STOREY * 2.0, h + STOREY * 2.0));
    let upper = s.above(kit);
    let tall = b.max.z - h;
    kit.m.clear_selection();
    // Only a block taller than it is deep along the fall can topple (a
    // wide block on its side would stand taller than before).
    let depth = Vec2::new(b.max.x - b.min.x, b.max.y - b.min.y).dot(st.tilt.abs());
    if tall > 6.0 && tall > depth * 1.2 && r.gen_bool((0.35 + st.asym * 0.6).min(1.0) as f64) {
        // Topple over the failed edge: rotate about it until it lies down.
        upper.iter().for_each(|&p| kit.m.polygons[p].selected = true);
        let mut top = crate::mesh::ops::geometry::detach(&mut kit.m, false);
        let half = Vec2::new(b.max.x - b.min.x, b.max.y - b.min.y) * 0.5;
        let edge = c + st.tilt * half.dot(st.tilt.abs());
        let pivot = edge.extend(h);
        let axis = Vec3::Z.cross(st.tilt.extend(0.0)).normalize_or(Vec3::X);
        let ang = r.gen_range(80.0f32..100.0).to_radians();
        let rot = Mat4::from_translation(pivot) * Mat4::from_quat(Quat::from_axis_angle(axis, ang)) * Mat4::from_translation(-pivot);
        top.transform(rot);
        if let Some(tb) = top.bounds() {
            // Rests on the ground, half a metre buried, pushed clear.
            let off = Vec3::new(st.tilt.x, st.tilt.y, 0.0) * r.gen_range(0.5..2.0) + Vec3::Z * (b.min.z - 0.5 - tb.min.z);
            top.transform(Mat4::from_translation(off));
        }
        kit.append(&top);
    } else {
        // Pancake: the floors above drop onto the stump as tilted slabs.
        for &p in &upper {
            kit.m.polygons.remove(p);
        }
        let floors = ((tall / STOREY).round() as usize).clamp(1, 10);
        let half = Vec2::new(b.max.x - b.min.x, b.max.y - b.min.y) * 0.42;
        let low = s.planes.iter().map(|p| p.0).fold(f32::MAX, f32::min) - half.length() * 0.35;
        debris::slabs(kit, r, c.extend(low.max(b.min.z)), half, floors);
        if era == Era::Medieval {
            for i in 0..floors {
                let z = low.max(b.min.z) + 0.3 + i as f32 * 0.45;
                let a = r.gen_range(0.0..std::f32::consts::TAU);
                let d = Vec2::from_angle(a).extend(0.0) * half.max_element();
                kit.mat(ids::CHARRED).beam(c.extend(z) - d, c.extend(z) + d, 0.25, 0.25);
            }
        }
    }
    kit.m.remove_isolated_vertices();
    broken_top(kit, b, &s, era, r);
}

/// Apocalyptic Ruin: a jagged stump of the lowest storeys.
pub fn stump(kit: &mut Kit, b: &Aabb, st: &Stress, era: Era, r: &mut ChaCha8Rng) {
    let total = b.max.z - b.min.z;
    let h = b.min.z + (total * r.gen_range(0.12..0.3)).clamp(1.2_f32.min(total * 0.6), STOREY * 3.0 + 1.0);
    // The ruin stands taller on the side that kept its support.
    let c = Vec2::new(b.min.x + b.max.x, b.min.y + b.max.y) * 0.5 - st.tilt * 0.5;
    let s = Surface::new(c, h, (h - b.min.z) * 0.35, r);
    s.cut(kit, (b.min.z, h + h - b.min.z));
    for p in s.above(kit) {
        kit.m.polygons.remove(p);
    }
    kit.m.remove_isolated_vertices();
    broken_top(kit, b, &s, era, r);
}
