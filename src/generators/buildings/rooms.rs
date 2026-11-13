//! Room shells: the floor to build interiors on. A floor slab with a
//! finish, walls with real door and window openings, frames, sills,
//! glass and skirting. The inside of the room is the lot (width x
//! length), walls stand outside it, and the floor top is z = 0, so
//! furniture dropped in sits on the boards. Cutaway: no ceiling, so the
//! room can be seen into from above.

use super::{BuildingKind as K, Ctx};
use crate::generators::kit::{Kit, ids};
use bevy::math::{Quat, Vec2, Vec3};
use rand::Rng;

const H: f32 = 2.6;
const T: f32 = 0.15;

/// An opening in a wall: centre along the wall, width, sill and head
/// height. A sill of 0 is a door.
#[derive(Clone, Copy)]
struct Hole {
    at: f32,
    w: f32,
    sill: f32,
    head: f32,
}

const fn door(at: f32, w: f32) -> Hole {
    Hole { at, w, sill: 0.0, head: 2.05 }
}

const fn window(at: f32, w: f32) -> Hole {
    Hole { at, w, sill: 0.9, head: 2.1 }
}

/// Which wall: 0 front (-Y), 1 right (+X), 2 back (+Y), 3 left (-X).
#[derive(Clone, Copy)]
struct Side {
    i: usize,
    hx: f32,
    hy: f32,
}

impl Side {
    /// A point `u` along the wall, `d` out from its inner face, at `z`.
    fn pt(self, u: f32, d: f32, z: f32) -> Vec3 {
        let (hx, hy) = (self.hx, self.hy);
        match self.i {
            0 => Vec3::new(u, -hy - d, z),
            1 => Vec3::new(hx + d, u, z),
            2 => Vec3::new(u, hy + d, z),
            _ => Vec3::new(-hx - d, u, z),
        }
    }
    /// The wall's extent along `u` (front and back walls cover corners).
    fn span(self) -> (f32, f32) {
        if self.i.is_multiple_of(2) { (-self.hx - T, self.hx + T) } else { (-self.hy, self.hy) }
    }
    fn dir(self) -> Vec2 {
        if self.i.is_multiple_of(2) { Vec2::X } else { Vec2::Y }
    }
    fn inward(self) -> Vec2 {
        [Vec2::Y, -Vec2::X, -Vec2::Y, Vec2::X][self.i]
    }
    fn bx(self, k: &mut Kit, u: (f32, f32), d: (f32, f32), z: (f32, f32)) {
        if u.1 - u.0 < 1e-3 || z.1 - z.0 < 1e-3 {
            return;
        }
        let (a, b) = (self.pt(u.0, d.0, z.0), self.pt(u.1, d.1, z.1));
        k.block(a.min(b), a.max(b));
    }
}

/// A wall layer from `d.0` to `d.1` out of the inner face, `top` high,
/// with `holes` cut through it.
fn layer(k: &mut Kit, s: Side, d: (f32, f32), top: f32, holes: &[Hole]) {
    let (mut cur, end) = s.span();
    for h in holes {
        let (a, b) = (h.at - h.w * 0.5, h.at + h.w * 0.5);
        s.bx(k, (cur, a), d, (0.0, top));
        s.bx(k, (a, b), d, (0.0, h.sill.min(top)));
        s.bx(k, (a, b), d, (h.head.min(top), top));
        cur = b;
    }
    s.bx(k, (cur, end), d, (0.0, top));
}

/// A full wall: the structure, frames, glass, sills and skirting.
fn wall(k: &mut Kit, s: Side, holes: &[Hole], frosted: bool) {
    k.mat(ids::PLASTER);
    layer(k, s, (0.0, T), H, holes);
    k.mat(ids::TRIM);
    // Skirting between the doors.
    let doors: Vec<Hole> = holes.iter().copied().filter(|h| h.sill == 0.0).map(|h| Hole { sill: 0.0, head: 0.0, ..h }).collect();
    let (u0, u1) = s.span();
    let inner = if s.i.is_multiple_of(2) { (u0 + T, u1 - T) } else { (u0, u1) };
    let mut cur = inner.0;
    for h in &doors {
        s.bx(k, (cur, h.at - h.w * 0.5 - 0.07), (-0.015, 0.0), (0.0, 0.1));
        cur = h.at + h.w * 0.5 + 0.07;
    }
    s.bx(k, (cur, inner.1), (-0.015, 0.0), (0.0, 0.1));
    for h in holes {
        let (a, b) = (h.at - h.w * 0.5, h.at + h.w * 0.5);
        // Architrave / window frame on the inner face.
        s.bx(k, (a - 0.07, a), (-0.02, 0.0), (0.0f32.max(h.sill - 0.07), h.head + 0.07));
        s.bx(k, (b, b + 0.07), (-0.02, 0.0), (0.0f32.max(h.sill - 0.07), h.head + 0.07));
        s.bx(k, (a, b), (-0.02, 0.0), (h.head, h.head + 0.07));
        if h.sill > 0.0 {
            s.bx(k, (a - 0.05, b + 0.05), (-0.06, T * 0.5), (h.sill - 0.03, h.sill));
            for x in [a, b - 0.04] {
                s.bx(k, (x, x + 0.04), (T * 0.4, T * 0.6), (h.sill, h.head));
            }
            s.bx(k, (a, b), (T * 0.4, T * 0.6), (h.head - 0.04, h.head));
            if h.w > 1.0 {
                s.bx(k, (h.at - 0.02, h.at + 0.02), (T * 0.4, T * 0.6), (h.sill, h.head));
            }
            k.mat(if frosted { ids::STUCCO } else { ids::GLASS });
            s.bx(k, (a + 0.04, b - 0.04), (T * 0.48, T * 0.52), (h.sill, h.head - 0.04));
            k.mat(ids::TRIM);
        }
    }
}

/// A door leaf hung in `h`, opened into the room by `open` radians.
fn leaf(k: &mut Kit, s: Side, h: Hole, open: f32) {
    let hinge = s.pt(h.at - h.w * 0.5 + 0.02, -0.02, 0.0);
    let lw = h.w - 0.04;
    let dir = s.dir() * open.cos() + s.inward() * open.sin();
    let c = hinge + (dir * lw * 0.5).extend(1.01);
    let q = Quat::from_rotation_z(dir.y.atan2(dir.x));
    k.mat(ids::TRIM).cuboid(c, Vec3::new(lw * 0.5, 0.02, 1.01), q);
    let knob = hinge + (dir * (lw - 0.07)).extend(1.0) + (s.inward() * 0.035).extend(0.0);
    k.mat(ids::IRON).frustum(knob - Vec3::Z * 0.02, 0.025, 0.025, 0.04, 8, true);
}

fn floor(k: &mut Kit, hx: f32, hy: f32, finish: u16) {
    k.mat(ids::CURB).block(Vec3::new(-hx - T, -hy - T, -0.12), Vec3::new(hx + T, hy + T, -0.01));
    k.mat(finish).block(Vec3::new(-hx, -hy, -0.01), Vec3::new(hx, hy, 0.0));
}

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32) {
    let open = b.r.gen_range(0.6..1.5);
    let flip = b.r.gen_bool(0.5);
    let wood = [ids::TRIM, ids::TRIM, ids::SLATE][b.r.gen_range(0..3)];
    let k = &mut b.kit;
    let s = |i| Side { i, hx, hy };
    let sx = if flip { -1.0 } else { 1.0 };
    match b.p.kind {
        K::RoomShell => {
            floor(k, hx, hy, wood);
            let d = door(sx * -(hx - 0.7), 0.85);
            wall(k, s(0), &[d], false);
            wall(k, s(2), &[window(0.0, (hx * 0.6).min(1.6))], false);
            wall(k, s(1), &[window(0.0, (hy * 0.5).min(1.2))], false);
            wall(k, s(3), &[], false);
            leaf(k, s(0), d, open);
        }
        K::OpenPlanRoom => {
            floor(k, hx, hy, wood);
            let d = door(-hx + 0.8, 0.9);
            wall(k, s(0), &[d, window(hx * 0.45, 1.4)], false);
            wall(k, s(2), &[window(-hx * 0.5, 1.4), window(hx * 0.5, 1.4)], false);
            // Glazed patio doors on the right.
            let patio = Hole { at: 0.0, w: (hy * 0.9).min(2.4), sill: 0.02, head: 2.1 };
            wall(k, s(1), &[patio], false);
            wall(k, s(3), &[window(0.0, 1.0)], false);
            leaf(k, s(0), d, open);
            // A partition stub with a breakfast bar marks the kitchen end.
            let x = -hx * 0.3;
            k.mat(ids::PLASTER).block(Vec3::new(x - 0.05, hy * 0.25, 0.0), Vec3::new(x + 0.05, hy, H));
            k.block(Vec3::new(x - 0.05, -hy * 0.15, 0.0), Vec3::new(x + 0.05, hy * 0.25, 1.0));
            k.mat(ids::TRIM).block(Vec3::new(x - 0.2, -hy * 0.15 - 0.05, 1.0), Vec3::new(x + 0.2, hy * 0.25, 1.04));
            k.mat(ids::SLATE).block(Vec3::new(-hx, -hy * 0.15, -0.005), Vec3::new(x, hy, 0.001));
        }
        K::BathroomShell => {
            floor(k, hx, hy, ids::SLATE);
            let d = door(sx * (hx - 0.55), 0.75);
            let w = Hole { at: 0.0, w: 0.6, sill: 1.5, head: 2.05 };
            wall(k, s(0), &[d], false);
            wall(k, s(2), &[w], true);
            wall(k, s(1), &[], false);
            wall(k, s(3), &[], false);
            // Tiled walls to 1.2 m.
            k.mat(ids::SIDEWALK);
            layer(k, s(0), (-0.012, 0.0), 1.2, &[d]);
            for i in 1..4 {
                layer(k, s(i), (-0.012, 0.0), 1.2, &[]);
            }
            leaf(k, s(0), d, open);
        }
        _ => {
            // Hallway: doors at both ends and off each side.
            floor(k, hx, hy, wood);
            let (f, bk) = (door(0.0, 0.9), door(0.0, 0.85));
            let (l, r) = (door(-hy * 0.4, 0.8), door(hy * 0.3, 0.8));
            wall(k, s(0), &[f], false);
            wall(k, s(2), &[bk], false);
            wall(k, s(1), &[r], false);
            wall(k, s(3), &[l, window(hy * 0.55, 0.6)], false);
            leaf(k, s(0), f, open * 0.5);
            k.mat(ids::ROOF).block(Vec3::new(-hx + 0.25, -hy + 0.5, 0.0), Vec3::new(hx - 0.25, hy - 0.5, 0.008));
        }
    }
}
