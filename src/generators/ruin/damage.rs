//! Light and Heavy damage. Light: panes crack (ID 23) or blow out, lit
//! windows go dark, bullet scars and soot streaks (ID 26) pepper the
//! walls, a little debris at the base. Heavy: a blown corner (a real hole
//! cut by six slice planes, floor slabs and rebar or charred joists left
//! exposed), shell holes, a caved roof, a 2-5 degree lean toward the side
//! that lost its support, and sandbags / tank traps (Modern) or a cart and
//! barrel barricade (Medieval) in front.

use super::RuinEra as Era;
use super::stress::{STOREY, Stress};
use crate::generators::kit::{Kit, ids};
use crate::mesh::{Aabb, PolyId};
use crate::modifiers::battlefield::debris;
use bevy::math::{Quat, Vec2, Vec3};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

pub(super) fn is_wall(id: u16) -> bool {
    matches!(id, ids::FACADE | ids::BRICK | ids::STUCCO | ids::PLASTER | ids::TRIM)
}

/// A flat decal quad on a surface (centre `c`, normal `n`), `w` x `h`,
/// turned `rot` in its plane, lifted 3 cm off it.
pub(super) fn decal(kit: &mut Kit, c: Vec3, n: Vec3, w: f32, h: f32, rot: f32, id: u16) {
    let t = if n.z.abs() > 0.9 { Vec3::X } else { Vec3::Z.cross(n).normalize() };
    let b = n.cross(t);
    let q = Quat::from_axis_angle(n, rot);
    let (u, v) = (q * t * w * 0.5, q * b * h * 0.5);
    let o = c + n * 0.03;
    kit.mat(id).quad(o - u - v, o + u - v, o + u + v, o - u + v);
}

/// A random point on polygon `pid` (fan triangle picked by area).
fn sample(kit: &Kit, pid: PolyId, r: &mut ChaCha8Rng) -> Vec3 {
    let pts: Vec<Vec3> = kit.m.polygons[pid].vertex_ids().map(|v| kit.m.pos(v)).collect();
    let i = r.gen_range(1..pts.len() - 1);
    let (mut a, mut b) = (r.gen_range(0.0..1.0f32), r.gen_range(0.0..1.0f32));
    if a + b > 1.0 {
        (a, b) = (1.0 - a, 1.0 - b);
    }
    pts[0] + (pts[i] - pts[0]) * a + (pts[i + 1] - pts[0]) * b
}

/// Light damage at `share` (0..1) of the panes and scar density.
pub fn light(kit: &mut Kit, b: &Aabb, era: Era, share: f32, r: &mut ChaCha8Rng) {
    let polys: Vec<PolyId> = kit.m.polygons.keys().collect();
    let mut walls = vec![];
    let mut budget = 500usize;
    for pid in polys {
        let (id, c, n) = (kit.m.polygons[pid].material_id, kit.m.poly_centroid(pid), kit.m.poly_normal(pid));
        match id {
            ids::GLASS | ids::LIT if r.gen_bool(share.clamp(0.0, 1.0) as f64) => {
                let corners: Vec<Vec3> = kit.m.polygons[pid].vertex_ids().map(|v| kit.m.pos(v)).collect();
                let area = kit.m.poly_area(pid);
                if r.gen_bool(0.5) {
                    kit.m.polygons[pid].material_id = ids::BROKEN_GLASS;
                } else {
                    kit.m.polygons.remove(pid);
                    if era == Era::Modern {
                        debris::shards(kit, &corners, r);
                    }
                }
                // Soot licks up from a blown-out window.
                if budget > 0 && n.z.abs() < 0.3 && area < 12.0 && r.gen_bool(0.45) {
                    let (w, h) = (area.sqrt() * 0.9, area.sqrt() * 1.6);
                    decal(kit, c + Vec3::Z * h * 0.45, n, w, h, r.gen_range(-0.1..0.1), ids::SCAR);
                    budget -= 1;
                }
            }
            ids::NEON | ids::LIT => kit.m.polygons[pid].material_id = ids::GLASS,
            id if is_wall(id) && n.z.abs() < 0.3 => walls.push((pid, kit.m.poly_area(pid), n)),
            _ => {}
        }
    }
    // Bullet scars: clusters of small dark pocks, more on big walls.
    for (pid, area, n) in walls {
        let k = ((area * share * 0.08) as usize + r.gen_bool((area * share * 0.08).fract().clamp(0.0, 1.0) as f64) as usize).min(budget);
        for _ in 0..k {
            let c = sample(kit, pid, r);
            for _ in 0..r.gen_range(2..7) {
                let off = Vec3::new(r.gen_range(-0.5..0.5), r.gen_range(-0.5..0.5), r.gen_range(-0.5..0.5));
                let s = r.gen_range(0.06..0.2);
                decal(kit, c + (off - n * off.dot(n)), n, s, s, r.gen_range(0.0..1.5), ids::SCAR);
            }
        }
        budget -= k;
    }
    // A little fallen debris at the foot of the walls.
    let foot = Vec2::new(b.min.x + b.max.x, b.min.y + b.max.y) * 0.5;
    let half = Vec2::new(b.max.x - b.min.x, b.max.y - b.min.y) * 0.5;
    for _ in 0..((share * 4.0) as usize + 1) {
        let side = Vec2::from_angle(r.gen_range(0.0..std::f32::consts::TAU));
        let at = (foot + side * (half + Vec2::splat(0.8))).extend(b.min.z);
        let n = r.gen_range(2..5);
        debris::rubble(kit, r, at, 0.9, n, ids::FACADE, false);
    }
}

/// Removes every polygon with its centroid inside the box `lo..hi` after
/// cutting the box's six faces through the polygons that overlap it.
/// `keep` spares polygons (by centroid and normal).
pub(super) fn bite(kit: &mut Kit, lo: Vec3, hi: Vec3, keep: impl Fn(Vec3, Vec3) -> bool) -> usize {
    let m = &mut kit.m;
    m.clear_selection();
    let near: Vec<PolyId> = m
        .polygons
        .keys()
        .filter(|&p| {
            let Some(bb) = crate::mesh::Aabb::from_points(m.polygons[p].vertex_ids().map(|v| m.pos(v))) else { return false };
            bb.max.cmpge(lo).all() && bb.min.cmple(hi).all()
        })
        .collect();
    if near.is_empty() {
        return 0;
    }
    near.iter().for_each(|&p| m.polygons[p].selected = true);
    for (p, n) in [(lo, Vec3::X), (hi, Vec3::X), (lo, Vec3::Y), (hi, Vec3::Y), (lo, Vec3::Z), (hi, Vec3::Z)] {
        crate::mesh::ops::slice::slice(m, p, n, true, false);
    }
    let gone: Vec<PolyId> = m
        .polygons
        .keys()
        .filter(|&p| m.polygons[p].selected)
        .filter(|&p| {
            let c = m.poly_centroid(p);
            c.cmpgt(lo).all() && c.cmplt(hi).all() && !keep(c, m.poly_normal(p))
        })
        .collect();
    for &p in &gone {
        m.polygons.remove(p);
    }
    m.clear_selection();
    gone.len()
}

/// Exposed floor slabs across a hole, with rebar (Modern) or charred
/// joists (Medieval) sticking out of their broken edges.
fn exposed(kit: &mut Kit, lo: Vec3, hi: Vec3, base: f32, era: Era, r: &mut ChaCha8Rng) {
    let mut z = base + STOREY;
    while z < hi.z {
        if z > lo.z {
            let inset = Vec3::new(r.gen_range(0.2..0.8), r.gen_range(0.2..0.8), 0.0);
            let (a, c) = ((lo + inset).with_z(z - 0.12), (hi - inset).with_z(z + 0.12));
            match era {
                Era::Modern => {
                    kit.mat(ids::FACADE).block(a, c);
                    let half = (c - a).truncate() * 0.5;
                    debris::rebar_tuft(kit, r, ((a + c) * 0.5).with_z(z + 0.1), half, 4);
                }
                Era::Medieval | Era::Auto => {
                    kit.mat(ids::CHARRED);
                    for i in 0..4 {
                        let t = (i as f32 + 0.5) / 4.0;
                        let y = a.y + (c.y - a.y) * t;
                        kit.beam(Vec3::new(a.x, y, z), Vec3::new(c.x - r.gen_range(0.0..1.5), y, z - r.gen_range(0.0..0.4)), 0.2, 0.24);
                    }
                }
                Era::Ancient => {
                    kit.mat(ids::FACADE).block(a, c.with_z(z));
                }
            }
        }
        z += STOREY;
    }
}

/// Heavy damage: blown corner, shell holes, caved roof, lean, barricade.
pub fn heavy(kit: &mut Kit, b: &Aabb, st: &Stress, era: Era, r: &mut ChaCha8Rng) {
    let size = b.max - b.min;
    let centre = (b.min + b.max) * 0.5;
    // Blown corner on the side that lost its support.
    let sx = if st.tilt.x >= 0.0 { 1.0 } else { -1.0 };
    let sy = if st.tilt.y >= 0.0 { 1.0 } else { -1.0 };
    let corner = Vec2::new(centre.x + sx * size.x * 0.5, centre.y + sy * size.y * 0.5);
    let rad = (size.x.min(size.y) * 0.35).clamp(1.2, 9.0);
    let zc = b.min.z + size.z * r.gen_range(0.15..0.45);
    let reach = Vec2::new(rad * 1.2, rad);
    let span = |c: f32, s: f32, d: f32| if s > 0.0 { (c - d, c + 0.5) } else { (c - 0.5, c + d) };
    let ((x0, x1), (y0, y1)) = (span(corner.x, sx, reach.x), span(corner.y, sy, reach.y));
    let (lo, hi) = (Vec3::new(x0, y0, (zc - rad).max(b.min.z + 0.4)), Vec3::new(x1, y1, zc + rad));
    if bite(kit, lo, hi, |_, _| false) > 0 {
        exposed(kit, lo, hi, b.min.z, era, r);
    }
    // Shell holes punched through single walls.
    for q in st.blasts.iter().take(3) {
        let n = if (q.x - b.min.x).abs() < 0.1 || (q.x - b.max.x).abs() < 0.1 { Vec3::X } else { Vec3::Y };
        let s = r.gen_range(0.8..1.6);
        let ext = Vec3::splat(s) - n * (s - 0.6);
        let z = q.z.max(b.min.z + s + 0.3);
        let at = q.with_z(z);
        bite(kit, at - ext, at + ext, |_, pn| pn.dot(n).abs() < 0.5);
    }
    // Caved roof: the top surfaces near a point fall in.
    let roof_at = centre.truncate() + st.tilt * size.truncate().min_element() * 0.2;
    let roof: Vec<PolyId> = kit
        .m
        .polygons
        .keys()
        .filter(|&p| {
            let id = kit.m.polygons[p].material_id;
            let c = kit.m.poly_centroid(p);
            let n = kit.m.poly_normal(p);
            let top = (id == ids::ROOF || id == ids::SLATE) || (n.z > 0.9 && c.z > b.max.z - 1.0);
            top && c.truncate().distance(roof_at) < size.truncate().max_element() * 0.3 + 1.0
        })
        .collect();
    for pid in roof {
        let corners: Vec<Vec3> = kit.m.polygons[pid].vertex_ids().map(|v| kit.m.pos(v)).collect();
        let n = kit.m.poly_normal(pid);
        kit.m.polygons.remove(pid);
        if era == Era::Medieval && n.z > 0.05 && n.z < 0.95 {
            debris::rafters(kit, &corners, n);
        } else if n.z > 0.9 {
            // A sagging slab hanging into the void.
            let c = corners.iter().copied().sum::<Vec3>() / corners.len() as f32;
            let sag = c - Vec3::Z * r.gen_range(0.8..2.0);
            kit.mat(ids::FACADE);
            for i in 0..corners.len() {
                let (a, bq) = (corners[i].lerp(c, 0.25), corners[(i + 1) % corners.len()].lerp(c, 0.25));
                kit.poly(&[a, bq, sag]);
            }
            if era == Era::Modern {
                debris::rebar_tuft(kit, r, sag, Vec2::splat(0.8), 3);
            }
        }
    }
    lean(kit, b, st, r.gen_range(2.0..5.0));
    barricade(kit, b, era, r);
}

/// Leans the whole building `deg` degrees toward the side that failed,
/// about its base edge on that side.
fn lean(kit: &mut Kit, b: &Aabb, st: &Stress, deg: f32) {
    let deg = deg * (0.5 + st.asym);
    let size = b.max - b.min;
    let centre = (b.min + b.max) * 0.5;
    let edge = centre.truncate() + st.tilt * Vec2::new(size.x, size.y).dot(st.tilt.abs()) * 0.5;
    let pivot = edge.extend(b.min.z);
    let axis = Vec3::Z.cross(st.tilt.extend(0.0)).normalize_or(Vec3::X);
    let q = Quat::from_axis_angle(axis, deg.to_radians());
    for v in kit.m.vertices.values_mut() {
        let h = (v.position.z - b.min.z).max(0.0);
        // Foundations stay put: the lean grows over the first storey.
        let k = (h / STOREY).min(1.0);
        let p = pivot + q * (v.position - pivot);
        v.position = v.position.lerp(p, k);
    }
}

/// Sandbags and Czech hedgehogs (Modern) or a cart, barrel and beam
/// barricade (Medieval) in front of the building.
fn barricade(kit: &mut Kit, b: &Aabb, era: Era, r: &mut ChaCha8Rng) {
    let front = Vec3::new((b.min.x + b.max.x) * 0.5, b.min.y - 3.0, b.min.z);
    match era {
        Era::Modern | Era::Auto => {
            kit.mat(ids::LOT);
            let n = ((b.max.x - b.min.x) / 0.7).clamp(4.0, 16.0) as i32;
            for row in 0..3 {
                for i in 0..(n - row) {
                    let x = front.x + (i as f32 - (n - row) as f32 * 0.5 + 0.5) * 0.62 + row as f32 * 0.31 - n as f32 * 0.05;
                    let rot = Quat::from_rotation_z(r.gen_range(-0.08..0.08));
                    kit.cuboid(Vec3::new(x, front.y, front.z + 0.13 + row as f32 * 0.24), Vec3::new(0.3, 0.2, 0.12), rot);
                }
            }
            kit.mat(ids::RUST);
            for i in 0..2 {
                let c = front + Vec3::new((i as f32 * 2.0 - 1.0) * ((b.max.x - b.min.x) * 0.5 + 1.5), -1.5, 0.6);
                for (a, d) in [(Vec3::X, Vec3::Z), (Vec3::Y, Vec3::Z), (Vec3::X + Vec3::Y, Vec3::Z)] {
                    let dir = (a + d * 0.6).normalize() * 0.9;
                    kit.beam(c - dir, c + dir, 0.12, 0.12);
                }
            }
        }
        Era::Medieval => debris::siege_clutter(kit, r, front, ((b.max.x - b.min.x) * 0.3).max(2.0)),
        Era::Ancient => {}
    }
}
