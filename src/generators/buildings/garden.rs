//! Garden & lot details, part one: mailbox, garden beds, hedge, picket
//! fence, patio, barbecue, swing set and trampoline. Ground at z = 0,
//! front facing -Y; the width runs along X.

use super::parks::bench;
use super::{BuildingKind as K, Ctx};
use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Vec2, Vec3};
use rand::Rng;
use std::f32::consts::{FRAC_PI_2, TAU};

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32) {
    match b.p.kind {
        K::Mailbox => mailbox(b),
        K::GardenBeds => beds(b, hx, hy),
        K::Hedge => hedge(b, hx, hy),
        K::PicketFence => picket(b, hx),
        K::Patio => patio(b, hx, hy),
        K::Barbecue => barbecue(b, hx, hy),
        K::SwingSet => swings(b, hx, hy),
        K::Trampoline => trampoline(b, hx),
        _ => {}
    }
}

/// A small round shrub at `p`.
pub(super) fn shrub(k: &mut Kit, p: Vec3, r: f32) {
    k.mat(ids::FOLIAGE).frustum(p, r * 0.7, r, r * 0.6, 8, false);
    k.frustum(p + Vec3::Z * r * 0.6, r, r * 0.3, r * 0.7, 8, true);
}

fn mailbox(b: &mut Ctx) {
    // A post-mounted box with a rounded top, a door and a flag.
    let paint = [ids::ROOF, ids::SLATE, ids::IRON][b.r.gen_range(0..3)];
    let k = &mut b.kit;
    k.mat(ids::TRIM).block(Vec3::new(-0.05, -0.05, 0.0), Vec3::new(0.05, 0.05, 1.05));
    k.block(Vec3::new(-0.12, -0.3, 0.95), Vec3::new(0.12, 0.3, 1.05));
    k.mat(paint).block(Vec3::new(-0.13, -0.25, 1.05), Vec3::new(0.13, 0.25, 1.25));
    k.at(Mat4::from_translation(Vec3::new(0.0, -0.25, 1.25)) * Mat4::from_rotation_x(-FRAC_PI_2), |k| k.frustum(Vec3::ZERO, 0.13, 0.13, 0.5, 10, false));
    k.mat(ids::TRIM).quad(Vec3::new(-0.13, -0.26, 1.05), Vec3::new(0.13, -0.26, 1.05), Vec3::new(0.13, -0.26, 1.3), Vec3::new(-0.13, -0.26, 1.3));
    k.mat(ids::ROOF).block(Vec3::new(0.14, 0.05, 1.15), Vec3::new(0.16, 0.1, 1.5));
    k.block(Vec3::new(0.14, 0.1, 1.38), Vec3::new(0.16, 0.25, 1.5));
}

fn beds(b: &mut Ctx, hx: f32, hy: f32) {
    // Raised timber vegetable beds on a lawn with gravel paths between,
    // rows of crops, a few canes and a water butt.
    b.kit.mat(ids::FOLIAGE).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.03));
    let n = ((hx * 2.0 / 1.8) as usize).clamp(1, 6);
    let bw = (hx * 2.0 - 0.6 * (n + 1) as f32) / n as f32;
    for i in 0..n {
        let x0 = -hx + 0.6 + i as f32 * (bw + 0.6);
        b.kit.mat(ids::DUST).block(Vec3::new(x0 - 0.6, -hy, 0.03), Vec3::new(x0, hy, 0.05));
        let (min, max) = (Vec3::new(x0, -hy + 0.4, 0.03), Vec3::new(x0 + bw, hy - 0.4, 0.35));
        b.kit.mat(ids::TRIM).block(min, max);
        b.kit.mat(ids::LOT).block(min + Vec3::new(0.06, 0.06, 0.0), max + Vec3::new(-0.06, -0.06, 0.02));
        let rows = (((max.y - min.y) / 0.5) as usize).max(1);
        let crop = b.r.gen_range(0..3);
        for j in 0..rows {
            let y = min.y + 0.3 + j as f32 * 0.5;
            if y > max.y - 0.2 {
                break;
            }
            let k = &mut b.kit;
            match crop {
                0 => k.mat(ids::FOLIAGE).block(Vec3::new(x0 + 0.15, y - 0.1, 0.37), Vec3::new(x0 + bw - 0.15, y + 0.1, 0.55)),
                1 => {
                    for x in 0..((bw / 0.4) as usize).max(1) {
                        shrub(k, Vec3::new(x0 + 0.25 + x as f32 * 0.4, y, 0.37), 0.15);
                    }
                }
                _ => {
                    for x in [x0 + 0.2, x0 + bw - 0.2] {
                        k.mat(ids::TRIM).beam(Vec3::new(x, y, 0.37), Vec3::new(x0 + bw * 0.5, y, 1.6), 0.02, 0.02);
                    }
                    k.mat(ids::FOLIAGE).block(Vec3::new(x0 + bw * 0.3, y - 0.05, 0.37), Vec3::new(x0 + bw * 0.7, y + 0.05, 1.3));
                }
            }
        }
    }
    let k = &mut b.kit;
    k.mat(ids::SLATE).frustum(Vec3::new(hx - 0.35, hy - 0.2, 0.03), 0.3, 0.3, 0.9, 12, true);
}

fn hedge(b: &mut Ctx, hx: f32, hy: f32) {
    // A clipped hedge with a slightly lumpy top, along X.
    let w = hy.min(0.6);
    let h = 1.2 + b.r.gen_range(0.0..0.6);
    let k = &mut b.kit;
    k.mat(ids::FOLIAGE).block(Vec3::new(-hx, -w, 0.0), Vec3::new(hx, w, h));
    let n = ((hx * 2.0 / 0.9) as usize).max(1);
    for i in 0..n {
        let x = -hx + 0.45 + i as f32 * 0.9;
        let r = b.r.gen_range(0.3..0.5);
        b.kit.mat(ids::FOLIAGE).frustum(Vec3::new(x, 0.0, h - 0.05), w * 0.9, r * 0.6, r * 0.5, 7, true);
    }
}

fn picket(b: &mut Ctx, hx: f32) {
    // A white picket fence with pointed pickets on two rails, a gate in
    // the middle when long enough.
    let paint = if b.r.gen_bool(0.7) { ids::PLASTER } else { ids::TRIM };
    let k = &mut b.kit;
    let gate = hx > 2.5;
    let n = ((hx * 2.0 / 0.15) as usize).max(4);
    for i in 0..n {
        let x = -hx + (i as f32 + 0.5) * 2.0 * hx / n as f32;
        if gate && x.abs() < 0.5 {
            continue;
        }
        k.mat(paint).block(Vec3::new(x - 0.04, -0.02, 0.05), Vec3::new(x + 0.04, 0.02, 0.9));
        k.poly(&[Vec3::new(x - 0.04, -0.021, 0.9), Vec3::new(x + 0.04, -0.021, 0.9), Vec3::new(x, -0.021, 1.0)]);
        k.poly(&[Vec3::new(x + 0.04, 0.021, 0.9), Vec3::new(x - 0.04, 0.021, 0.9), Vec3::new(x, 0.021, 1.0)]);
    }
    for z in [0.25, 0.7] {
        for (a, c) in if gate { vec![(-hx, -0.55), (0.55, hx)] } else { vec![(-hx, hx)] } {
            k.block(Vec3::new(a, 0.02, z), Vec3::new(c, 0.06, z + 0.08));
        }
    }
    let posts = ((hx * 2.0 / 2.4) as usize).max(1);
    for i in 0..=posts {
        let x = -hx + 2.0 * hx * i as f32 / posts as f32;
        k.block(Vec3::new(x - 0.05, -0.05, 0.0), Vec3::new(x + 0.05, 0.05, 1.05));
    }
    if gate {
        k.at(Mat4::from_translation(Vec3::new(-0.5, -0.03, 0.0)) * Mat4::from_rotation_z(-0.5), |k| {
            for i in 0..6 {
                let x = 0.08 + i as f32 * 0.165;
                k.block(Vec3::new(x - 0.04, -0.02, 0.1), Vec3::new(x + 0.04, 0.02, 0.95));
            }
            k.block(Vec3::new(0.0, -0.06, 0.3), Vec3::new(1.0, -0.02, 0.38));
            k.block(Vec3::new(0.0, -0.06, 0.7), Vec3::new(1.0, -0.02, 0.78));
        });
    }
}

fn patio(b: &mut Ctx, hx: f32, hy: f32) {
    // Paving slabs with a table and chairs under a parasol, a bench and
    // potted plants at the corners.
    let k = &mut b.kit;
    let (nx, ny) = (((hx * 2.0 / 0.6) as usize).max(1), ((hy * 2.0 / 0.6) as usize).max(1));
    for i in 0..nx {
        for j in 0..ny {
            let (x, y) = (-hx + i as f32 * 2.0 * hx / nx as f32, -hy + j as f32 * 2.0 * hy / ny as f32);
            k.mat(if (i + j) % 2 == 0 { ids::SIDEWALK } else { ids::CURB });
            k.block(Vec3::new(x + 0.01, y + 0.01, 0.0), Vec3::new(x + 2.0 * hx / nx as f32 - 0.01, y + 2.0 * hy / ny as f32 - 0.01, 0.06));
        }
    }
    let c = Vec3::new(-hx * 0.2, 0.0, 0.06);
    k.mat(ids::PLASTER).frustum(c + Vec3::Z * 0.7, 0.55, 0.55, 0.04, 14, true);
    k.mat(ids::IRON).frustum(c, 0.25, 0.04, 0.7, 8, true);
    k.frustum(c + Vec3::Z * 0.74, 0.025, 0.025, 1.6, 6, true);
    k.mat(ids::STUCCO).frustum(c + Vec3::Z * 2.0, 1.4, 0.05, 0.5, 8, true);
    for i in 0..4 {
        let a = i as f32 * TAU / 4.0 + 0.4;
        let p = (Vec2::from_angle(a) * 0.9).extend(0.0) + c;
        k.at(Mat4::from_translation(p) * Mat4::from_rotation_z(a + FRAC_PI_2), |k| {
            k.mat(ids::TRIM).block(Vec3::new(-0.22, -0.2, 0.42), Vec3::new(0.22, 0.2, 0.47));
            k.block(Vec3::new(-0.22, 0.18, 0.47), Vec3::new(0.22, 0.22, 0.9));
            for (x, y) in [(-0.2, -0.18), (0.2, -0.18), (-0.2, 0.18), (0.2, 0.18)] {
                k.block(Vec3::new(x - 0.02, y - 0.02, 0.0), Vec3::new(x + 0.02, y + 0.02, 0.42));
            }
        });
    }
    bench(k, Vec3::new(hx * 0.55, hy - 0.5, 0.06), std::f32::consts::PI);
    for (x, y) in [(-hx + 0.3, -hy + 0.3), (hx - 0.3, -hy + 0.3), (-hx + 0.3, hy - 0.3)] {
        k.mat(ids::RUST).frustum(Vec3::new(x, y, 0.06), 0.16, 0.22, 0.4, 10, true);
        shrub(k, Vec3::new(x, y, 0.46), 0.25);
    }
}

fn barbecue(b: &mut Ctx, hx: f32, hy: f32) {
    // A kettle grill on legs or a brick barbecue with a hood (by seed).
    let k = &mut b.kit;
    if b.r.gen_bool(0.5) {
        let c = Vec3::Z * 0.75;
        k.mat(ids::IRON);
        for i in 0..3 {
            let a = i as f32 * TAU / 3.0;
            k.beam(Vec2::from_angle(a).extend(0.03) * 0.32 + Vec3::Z * 0.02, c, 0.03, 0.03);
        }
        k.mat(ids::ASPHALT).frustum(c, 0.18, 0.32, 0.25, 14, false);
        k.mat(ids::IRON).frustum(c + Vec3::Z * 0.25, 0.32, 0.32, 0.02, 14, true);
        k.mat(ids::ASPHALT).frustum(c + Vec3::Z * 0.27, 0.32, 0.12, 0.25, 14, true);
        k.mat(ids::TRIM).block(Vec3::new(-0.05, -0.02, 1.27), Vec3::new(0.05, 0.02, 1.33));
    } else {
        let (w, d) = (hx.min(1.2), hy.min(0.5));
        k.mat(ids::BRICK).block(Vec3::new(-w, -d, 0.0), Vec3::new(w, d, 0.85));
        k.mat(ids::CURB).block(Vec3::new(-w - 0.05, -d - 0.05, 0.85), Vec3::new(w + 0.05, d + 0.05, 0.9));
        k.mat(ids::IRON).block(Vec3::new(-w * 0.6, -d + 0.05, 0.9), Vec3::new(w * 0.6, d - 0.05, 0.95));
        k.mat(ids::ASPHALT).block(Vec3::new(-w * 0.62, -d * 0.2, 0.95), Vec3::new(w * 0.62, d, 1.3));
        k.mat(ids::BRICK).block(Vec3::new(-0.2, d - 0.3, 0.9), Vec3::new(0.2, d, 1.6));
    }
}

fn swings(b: &mut Ctx, hx: f32, hy: f32) {
    // A garden swing set: A-frame legs, a top bar, two swings and a
    // small slide off one end.
    let (w, d) = (hx.min(2.0) - 0.2, hy.min(1.4));
    let paint = [ids::ROOF, ids::SLATE, ids::STUCCO][b.r.gen_range(0..3)];
    let k = &mut b.kit;
    k.mat(paint);
    for x in [-w, w] {
        for y in [-d, d] {
            k.beam(Vec3::new(x, y, 0.04), Vec3::new(x, 0.0, 2.1), 0.06, 0.06);
        }
    }
    k.tube(Vec3::new(-w - 0.1, 0.0, 2.1), Vec3::new(w + 0.1, 0.0, 2.1), 0.05, 8);
    for x in [-w * 0.45, w * 0.45] {
        for o in [-0.22, 0.22] {
            k.mat(ids::IRON).beam(Vec3::new(x + o, 0.0, 2.08), Vec3::new(x + o, 0.0, 0.45), 0.015, 0.015);
        }
        k.mat(ids::ASPHALT).block(Vec3::new(x - 0.24, -0.1, 0.4), Vec3::new(x + 0.24, 0.1, 0.45));
    }
    k.mat(ids::PLASTER).quad(Vec3::new(w, -0.25, 1.2), Vec3::new(w, 0.25, 1.2), Vec3::new(w + 1.6, 0.25, 0.1), Vec3::new(w + 1.6, -0.25, 0.1));
    k.quad(Vec3::new(w + 1.6, -0.25, 0.1), Vec3::new(w + 1.6, 0.25, 0.1), Vec3::new(w, 0.25, 1.2), Vec3::new(w, -0.25, 1.2));
}

fn trampoline(b: &mut Ctx, hx: f32) {
    // A round trampoline: legs, a padded frame, the mat and a safety net
    // on poles.
    let r = hx.min(2.5) - 0.1;
    let k = &mut b.kit;
    let z = 0.8;
    k.mat(ids::IRON);
    for i in 0..6 {
        let p = Vec2::from_angle(i as f32 * TAU / 6.0) * r;
        k.block((p - Vec2::splat(0.03)).extend(0.0), (p + Vec2::splat(0.03)).extend(z + 1.8));
    }
    k.mat(ids::SLATE).frustum(Vec3::Z * z, r + 0.08, r + 0.08, 0.1, 20, false);
    k.mat(ids::ASPHALT).frustum(Vec3::Z * (z + 0.05), r - 0.25, r - 0.25, 0.02, 20, true);
    k.mat(ids::SLATE).frustum(Vec3::Z * (z + 0.05), r, r - 0.25, 0.04, 20, false);
    k.mat(ids::REBAR);
    for zz in [z + 0.9, z + 1.8] {
        k.frustum(Vec3::Z * zz, r + 0.02, r + 0.02, 0.03, 20, false);
    }
}
