//! Medieval masonry parts: staggered 3D stone bricks, crenellations,
//! machicolations, arrow slits, portcullis grates, wall segments with a
//! recessed wall-walk, round towers and gatehouses. Every count comes from
//! the size and the block / merlon dimensions, never from a fixed number.

use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Quat, Vec2, Vec3};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Debug, PartialEq)]
pub struct Masonry {
    pub bricks: bool,
    /// Block width and height.
    pub brick: Vec2,
    /// How far blocks stand proud of the mortar core.
    pub depth: f32,
    pub crenels: bool,
    pub merlon: f32,
    /// Merlon width : crenel width.
    pub ratio: f32,
    pub merlon_h: f32,
}

impl Default for Masonry {
    fn default() -> Self {
        Self { bricks: true, brick: Vec2::new(0.8, 0.4), depth: 0.07, crenels: true, merlon: 0.9, ratio: 1.5, merlon_h: 1.1 }
    }
}

pub fn course_count(h: f32, brick_h: f32) -> usize {
    (h / brick_h.max(0.01)).floor().max(1.0) as usize
}

pub fn bricks_per_course(len: f32, brick_w: f32) -> usize {
    (len / brick_w.max(0.01)).round().max(1.0) as usize
}

/// Merlons along `len`: one merlon + one crenel per period.
pub fn merlon_count(len: f32, merlon: f32, ratio: f32) -> usize {
    let period = merlon * (1.0 + 1.0 / ratio.max(0.05));
    (len / period).floor().max(1.0) as usize
}

/// Staggered blocks over a face lying at local y = 0 (facing -Y), spanning
/// x in -w/2..w/2 and z in z0..z0+h. Returns the block count.
pub fn brick_face(kit: &mut Kit, w: f32, z0: f32, h: f32, m: &Masonry) -> usize {
    let courses = course_count(h, m.brick.y);
    let ch = h / courses as f32;
    let n = bricks_per_course(w, m.brick.x);
    let bw = w / n as f32;
    let gap = (m.brick.y * 0.08).min(0.05);
    kit.mat(ids::FACADE);
    let mut count = 0;
    for c in 0..courses {
        let z = z0 + c as f32 * ch;
        let shift = if c % 2 == 1 { bw * 0.5 } else { 0.0 };
        let mut x = -w * 0.5;
        let mut first = true;
        while x < w * 0.5 - 1e-3 {
            let x1 = if first && shift > 0.0 { x + shift } else { (x + bw).min(w * 0.5) };
            kit.block(Vec3::new(x + gap * 0.5, -m.depth, z + gap * 0.5), Vec3::new(x1 - gap * 0.5, 0.0, z + ch - gap * 0.5));
            count += 1;
            x = x1;
            first = false;
        }
    }
    count
}

/// Merlons along local X (length `len`) on top of z, `t` thick.
pub fn crenellate(kit: &mut Kit, len: f32, z: f32, t: f32, m: &Masonry) {
    let n = merlon_count(len, m.merlon, m.ratio);
    let period = len / n as f32;
    let mw = period * m.ratio / (1.0 + m.ratio);
    kit.mat(ids::FACADE);
    for i in 0..n {
        let x = -len * 0.5 + period * (i as f32 + 0.5);
        kit.block(Vec3::new(x - mw * 0.5, -t * 0.5, z), Vec3::new(x + mw * 0.5, t * 0.5, z + m.merlon_h));
    }
}

/// Corbel brackets and a projecting parapet along local X at height `z`,
/// overhanging the face at y = -t/2 by `out`.
pub fn machicolations(kit: &mut Kit, len: f32, z: f32, t: f32, out: f32) {
    let n = (len / 1.2).floor().max(2.0) as usize;
    kit.mat(ids::FACADE);
    for i in 0..=n {
        let x = -len * 0.5 + len * i as f32 / n as f32;
        for (k, d) in [0.35, 0.7, 1.0].into_iter().enumerate() {
            let zz = z - 0.9 + k as f32 * 0.3;
            kit.block(Vec3::new(x - 0.18, -t * 0.5 - out * d, zz), Vec3::new(x + 0.18, -t * 0.5, zz + 0.3));
        }
    }
    kit.block(Vec3::new(-len * 0.5 - out, -t * 0.5 - out, z), Vec3::new(len * 0.5 + out, -t * 0.5 + 0.4, z + 0.9));
}

/// A vertical arrow slit (with the cross-shaped sighting slot) at x on a
/// face frame.
pub fn arrow_slit(kit: &mut Kit, x: f32, z: f32, h: f32) {
    kit.mat(ids::GLASS);
    kit.quad(Vec3::new(x - 0.07, -0.02, z), Vec3::new(x + 0.07, -0.02, z), Vec3::new(x + 0.07, -0.02, z + h), Vec3::new(x - 0.07, -0.02, z + h));
    let zc = z + h * 0.55;
    kit.quad(Vec3::new(x - 0.3, -0.03, zc - 0.06), Vec3::new(x + 0.3, -0.03, zc - 0.06), Vec3::new(x + 0.3, -0.03, zc + 0.06), Vec3::new(x - 0.3, -0.03, zc + 0.06));
}

/// Iron portcullis grate in the local XZ plane (`w` x `h`, bottom at z = 0).
pub fn portcullis(kit: &mut Kit, w: f32, h: f32) {
    kit.mat(ids::IRON);
    let nx = ((w / 0.35).round() as usize).max(2);
    for i in 0..=nx {
        let x = -w * 0.5 + w * i as f32 / nx as f32;
        kit.block(Vec3::new(x - 0.05, -0.05, -0.25), Vec3::new(x + 0.05, 0.05, h));
    }
    let nz = ((h / 0.5).round() as usize).max(2);
    for k in 1..=nz {
        let z = h * k as f32 / nz as f32 - 0.05;
        kit.block(Vec3::new(-w * 0.5, -0.07, z), Vec3::new(w * 0.5, 0.07, z + 0.08));
    }
}

/// Straight wall from `a` to `b` (its outer face on the right of a -> b),
/// `h` tall and `t` thick, with bricks, and either a recessed wall-walk
/// with front / rear parapets (`walk`) or plain crenellations.
#[allow(clippy::too_many_arguments)]
pub fn wall_segment(kit: &mut Kit, a: Vec2, b: Vec2, z0: f32, h: f32, t: f32, m: &Masonry, walk: bool) {
    let d = b - a;
    let len = d.length();
    if len < 0.05 {
        return;
    }
    let f = Mat4::from_rotation_translation(Quat::from_rotation_z(d.y.atan2(d.x)), ((a + b) * 0.5).extend(z0));
    kit.at(f, |k| {
        k.mat(if m.bricks { ids::PLASTER } else { ids::FACADE }).block(Vec3::new(-len * 0.5, -t * 0.5, 0.0), Vec3::new(len * 0.5, t * 0.5, h));
        if m.bricks {
            for side in [0.0, PI] {
                k.at(Mat4::from_rotation_z(side) * Mat4::from_translation(Vec3::new(0.0, -t * 0.5, 0.0)), |k| {
                    brick_face(k, len, 0.0, h, m);
                });
            }
        }
        if walk {
            let pt = (t * 0.22).clamp(0.3, 0.7);
            k.mat(ids::FACADE).block(Vec3::new(-len * 0.5, -t * 0.5, h), Vec3::new(len * 0.5, -t * 0.5 + pt, h + 0.8));
            k.block(Vec3::new(-len * 0.5, t * 0.5 - pt, h), Vec3::new(len * 0.5, t * 0.5, h + 0.8));
            k.mat(ids::LOT).quad(Vec3::new(-len * 0.5, -t * 0.5 + pt, h + 0.01), Vec3::new(len * 0.5, -t * 0.5 + pt, h + 0.01), Vec3::new(len * 0.5, t * 0.5 - pt, h + 0.01), Vec3::new(-len * 0.5, t * 0.5 - pt, h + 0.01));
            if m.crenels {
                for y in [-t * 0.5 + pt * 0.5, t * 0.5 - pt * 0.5] {
                    k.at(Mat4::from_translation(Vec3::Y * y), |k| crenellate(k, len, h + 0.8, pt, m));
                }
            }
        } else if m.crenels {
            crenellate(k, len, h, t, m);
        }
    });
}

/// Round tower with a masonry ring, crenellated parapet and conical roof.
pub fn round_tower(kit: &mut Kit, c: Vec3, r: f32, h: f32, m: &Masonry, roof: bool) {
    let circ = TAU * r;
    let sides = (circ / m.brick.x).round().clamp(12.0, 64.0) as u32;
    kit.mat(if m.bricks { ids::PLASTER } else { ids::FACADE }).frustum(c, r, r, h, sides, true);
    if m.bricks {
        brick_ring(kit, c, r, h, m);
    }
    if m.crenels {
        merlon_ring(kit, c + Vec3::Z * h, r, m);
    }
    if roof {
        kit.mat(ids::ROOF).frustum(c + Vec3::Z * (h + 0.2), r * 0.92, 0.0, r * 2.2, sides.min(24), true);
    }
}

/// Merlons around a circle of radius `r` standing on `c`.
pub fn merlon_ring(kit: &mut Kit, c: Vec3, r: f32, m: &Masonry) {
    let circ = TAU * r;
    let n = merlon_count(circ, m.merlon, m.ratio);
    let w = circ / n as f32 * m.ratio / (1.0 + m.ratio);
    kit.mat(ids::FACADE);
    for i in 0..n {
        let a = TAU * i as f32 / n as f32;
        let p = c + Vec3::new(a.cos() * (r - 0.3), a.sin() * (r - 0.3), m.merlon_h * 0.5);
        kit.cuboid(p, Vec3::new(0.3, w * 0.5, m.merlon_h * 0.5), Quat::from_rotation_z(a));
    }
}

/// Staggered blocks around a cylinder.
pub fn brick_ring(kit: &mut Kit, c: Vec3, r: f32, h: f32, m: &Masonry) -> usize {
    let courses = course_count(h, m.brick.y);
    let ch = h / courses as f32;
    let n = bricks_per_course(TAU * r, m.brick.x);
    let da = TAU / n as f32;
    let bw = TAU * r / n as f32;
    kit.mat(ids::FACADE);
    for k in 0..courses {
        let shift = if k % 2 == 1 { da * 0.5 } else { 0.0 };
        for i in 0..n {
            let a = i as f32 * da + shift;
            let p = c + Vec3::new(a.cos() * (r + m.depth * 0.5), a.sin() * (r + m.depth * 0.5), k as f32 * ch + ch * 0.5);
            kit.cuboid(p, Vec3::new(m.depth * 0.5, bw * 0.46, ch * 0.45), Quat::from_rotation_z(a));
        }
    }
    courses * n
}

/// A gatehouse spanning `a` -> `b`: two piers, an arch over the passage
/// with an iron portcullis, and a crenellated top.
pub fn gatehouse(kit: &mut Kit, a: Vec2, b: Vec2, h: f32, t: f32, m: &Masonry) {
    let d = b - a;
    let len = d.length();
    let gw = (len * 0.45).clamp(2.0, 5.0);
    let spring = (h * 0.4).min(3.5);
    let f = Mat4::from_rotation_translation(Quat::from_rotation_z(d.y.atan2(d.x)), ((a + b) * 0.5).extend(0.0));
    let deep = t * 1.6;
    kit.at(f, |k| {
        k.mat(ids::FACADE);
        for s in [-1.0f32, 1.0] {
            let (x0, x1) = (s * gw * 0.5, s * len * 0.5);
            k.block(Vec3::new(x0.min(x1), -deep * 0.5, 0.0), Vec3::new(x0.max(x1), deep * 0.5, h + 1.5));
        }
        k.at(Mat4::from_translation(Vec3::Z * spring), |k| k.arch_wall(gw, gw * 0.5, h + 1.5 - spring, deep, false));
        k.mat(ids::FACADE).block(Vec3::new(-len * 0.5, -deep * 0.5, h + 1.5), Vec3::new(len * 0.5, deep * 0.5, h + 1.8));
        if m.crenels {
            crenellate(k, len, h + 1.8, deep, m);
        }
        k.at(Mat4::from_translation(Vec3::new(0.0, -deep * 0.3, 0.6)), |k| portcullis(k, gw - 0.1, spring + gw * 0.5 - 0.7));
    });
}
