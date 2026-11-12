//! Modern low-rise and special buildings: shop with awning, gas station,
//! warehouse, sawtooth factory, modern church, water tower, street kiosk.

use super::arch::{Opening, wall_with_openings, window_row};
use super::parts;
use super::{BuildingKind as K, Ctx, massing};
use crate::generators::kit::ids;
use bevy::math::{Mat4, Quat, Vec3};
use std::f32::consts::FRAC_PI_2;

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    match b.p.kind {
        K::Shop => shop(b, hx, hy, floors),
        K::GasStation => gas_station(b, hx, hy),
        K::Warehouse => shed(b, hx, hy, 7.0 + floors as f32, false),
        K::SawtoothFactory => shed(b, hx, hy, 6.0 + floors as f32, true),
        K::ModernChurch => church(b, hx, hy),
        K::WaterTower => water_tower(b, hx, floors),
        K::Kiosk => kiosk(b, hx, hy),
        _ => {}
    }
}

fn shop(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    let fh = 3.4;
    let w = 2.0 * hx;
    // Shopfront: wide display windows either side of the door.
    let ops = [
        Opening { at: w * 0.22, width: w * 0.32, sill: 0.5, height: 2.3, fill: ids::GLASS },
        Opening { at: w * 0.5, width: 1.2, sill: 0.0, height: 2.4, fill: ids::GLASS },
        Opening { at: w * 0.78, width: w * 0.32, sill: 0.5, height: 2.3, fill: ids::GLASS },
    ];
    b.kit.at(Mat4::from_translation(Vec3::new(-hx, -hy, 0.0)), |k| {
        k.mat(ids::BRICK);
        wall_with_openings(k, w, 0.3, fh, &ops);
    });
    b.kit.mat(ids::BRICK).block(Vec3::new(-hx, -hy + 0.3, 0.0), Vec3::new(hx, hy, fh));
    if floors > 1 {
        massing(b, hx, hy, fh, floors - 1, 3.0, ids::STUCCO, 2.4, 1.0, 1.4, 0.3);
    }
    let top = fh + 3.0 * floors.saturating_sub(1) as f32;
    b.kit.mat(ids::TRIM).block(Vec3::new(-hx - 0.2, -hy - 0.2, top), Vec3::new(hx + 0.2, hy + 0.2, top + 0.6));
    let (p, sh, g) = (b.p.parts, b.p.parts.shutters, b.glow(true));
    b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hy - 0.03, 0.0)), |k| {
        if p.awnings {
            parts::awning(k, 0.0, fh - 0.2, w - 0.6, 1.6, true);
        }
        if p.signs {
            parts::sign(k, 0.0, fh + 0.05, w * 0.6, g);
        }
        if sh && floors > 1 {
            parts::shutters(k, -hx * 0.5, fh + 1.0, 1.0, 1.4);
        }
    });
    if p.chimneys {
        parts::chimney(&mut b.kit, Vec3::new(hx * 0.6, hy * 0.5, top + 0.6), 1.4);
    }
}

fn gas_station(b: &mut Ctx, hx: f32, hy: f32) {
    // Forecourt slab, canopy on four columns, pumps, and a shop box behind.
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.12));
    let (cx, cy) = (hx * 0.7, hy * 0.35);
    let cz = 5.0;
    b.kit.mat(ids::IRON);
    for (sx, sy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 0.2), (1.0, 0.2)] {
        let p = Vec3::new(sx * cx * 0.8, -hy * 0.35 + sy * cy, 0.0);
        b.kit.block(p - Vec3::new(0.2, 0.2, 0.0), p + Vec3::new(0.2, 0.2, cz));
    }
    b.kit.mat(ids::PLASTER).block(Vec3::new(-cx, -hy * 0.35 - cy - 1.0, cz), Vec3::new(cx, -hy * 0.35 + cy * 0.4 + 1.0, cz + 0.9));
    let g = b.glow(true);
    b.kit.mat(g).block(Vec3::new(-cx, -hy * 0.35 - cy - 1.02, cz + 0.2), Vec3::new(cx, -hy * 0.35 - cy - 1.0, cz + 0.7));
    for i in 0..3 {
        let x = -cx * 0.5 + cx * 0.5 * i as f32;
        b.kit.mat(ids::CURB).block(Vec3::new(x - 0.6, -hy * 0.35 - 0.5, 0.12), Vec3::new(x + 0.6, -hy * 0.35 + 0.5, 0.3));
        b.kit.mat(ids::PLASTER).block(Vec3::new(x - 0.3, -hy * 0.35 - 0.25, 0.3), Vec3::new(x + 0.3, -hy * 0.35 + 0.25, 1.9));
    }
    b.kit.at(Mat4::from_translation(Vec3::new(0.0, hy * 0.55, 0.0)), |k| {
        k.mat(ids::PLASTER).block(Vec3::new(-hx * 0.6, -hy * 0.35, 0.0), Vec3::new(hx * 0.6, hy * 0.45, 3.6));
        crate::generators::settlement::window(k, 0.0, -hy * 0.35 - 0.02, 0.4, hx * 0.8, 2.4, ids::GLASS);
    });
    if b.p.parts.signs {
        let p = Vec3::new(hx - 1.0, -hy + 1.0, 0.0);
        b.kit.mat(ids::IRON).tube(p, p + Vec3::Z * 7.0, 0.15, 6);
        b.kit.mat(g).block(p + Vec3::new(-1.0, -0.2, 7.0), p + Vec3::new(1.0, 0.2, 9.0));
    }
}

/// A warehouse (gable roof, loading doors) or a sawtooth-roofed factory
/// with north-light glazing, chimney stack and a dock.
fn shed(b: &mut Ctx, hx: f32, hy: f32, h: f32, sawtooth: bool) {
    let w = 2.0 * hx;
    let doors = window_row(w, 7.0, 4.0, 0.0, 4.5, |_| ids::RUST);
    b.kit.at(Mat4::from_translation(Vec3::new(-hx, -hy, 0.0)), |k| {
        k.mat(ids::BRICK);
        wall_with_openings(k, w, 0.35, h, &doors);
    });
    b.kit.mat(ids::BRICK).block(Vec3::new(-hx, -hy + 0.35, 0.0), Vec3::new(hx, hy, h));
    if sawtooth {
        let n = ((2.0 * hy / 6.0).round() as usize).max(2);
        let d = 2.0 * hy / n as f32;
        for i in 0..n {
            let y0 = -hy + d * i as f32;
            let (a, bb, c) = (Vec3::new(0.0, y0, h), Vec3::new(0.0, y0 + d, h), Vec3::new(0.0, y0 + d, h + d * 0.55));
            b.kit.mat(ids::RUST).quad(a - Vec3::X * hx, a + Vec3::X * hx, c + Vec3::X * hx, c - Vec3::X * hx);
            b.kit.mat(ids::GLASS).quad(bb + Vec3::X * hx, bb - Vec3::X * hx, c - Vec3::X * hx, c + Vec3::X * hx);
            b.kit.mat(ids::BRICK).poly(&[a - Vec3::X * hx, c - Vec3::X * hx, bb - Vec3::X * hx]);
            b.kit.poly(&[a + Vec3::X * hx, bb + Vec3::X * hx, c + Vec3::X * hx]);
        }
        if b.p.parts.chimneys {
            b.kit.mat(ids::BRICK).frustum(Vec3::new(hx * 0.7, hy * 0.7, 0.0), 1.2, 0.8, h * 3.2, 10, true);
        }
    } else {
        b.kit.at(Mat4::from_translation(Vec3::Z * h) * Mat4::from_rotation_z(FRAC_PI_2), |k| k.mat(ids::RUST).gable(hy, hx, hx * 0.25, 0.4, ids::BRICK));
    }
    if b.p.parts.porches {
        b.kit.mat(ids::FACADE).block(Vec3::new(-hx, -hy - 3.0, 0.0), Vec3::new(hx, -hy, 1.2));
    }
    if b.p.parts.signs {
        let g = b.glow(false);
        b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hy - 0.03, 0.0)), |k| parts::sign(k, 0.0, h - 1.2, hx, g));
    }
}

fn church(b: &mut Ctx, hx: f32, hy: f32) {
    let h = 9.0;
    b.kit.mat(ids::PLASTER).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, h));
    // Long slit windows on the sides, a tall glazed front and a bell tower.
    let (r, em) = (&mut b.r, b.p.emissive);
    crate::generators::kit::sides(&mut b.kit, hx, hy, |k, side, w| {
        let n = if side % 2 == 1 { ((w / 3.0) as usize).max(1) } else { 1 };
        for i in 0..n {
            let x = -w * 0.5 + w * (i as f32 + 0.5) / n as f32;
            let id = super::pane_id(r, em, 0.5);
            crate::generators::settlement::window(k, x, 0.0, 1.5, if side == 0 { 2.0 } else { 0.5 }, h - 3.0, id);
        }
    });
    b.kit.at(Mat4::from_translation(Vec3::Z * h), |k| k.mat(ids::SLATE).gable(hx, hy, hx * 1.1, 0.5, ids::PLASTER));
    let t = Vec3::new(hx + 2.5, -hy + 2.5, 0.0);
    b.kit.mat(ids::FACADE).block(t - Vec3::new(1.8, 1.8, 0.0), t + Vec3::new(1.8, 1.8, h * 2.2));
    b.kit.mat(ids::IRON).cuboid(t + Vec3::Z * (h * 2.2 + 2.0), Vec3::new(0.1, 0.1, 2.0), Quat::IDENTITY);
    b.kit.cuboid(t + Vec3::Z * (h * 2.2 + 2.8), Vec3::new(0.8, 0.1, 0.1), Quat::IDENTITY);
}

fn water_tower(b: &mut Ctx, hx: f32, floors: u32) {
    let h = 10.0 + 4.0 * floors as f32;
    let r = hx * 0.9;
    b.kit.mat(ids::IRON);
    for k in 0..4 {
        let a = k as f32 * FRAC_PI_2 + FRAC_PI_2 * 0.5;
        let foot = Vec3::new(a.cos() * r, a.sin() * r, 0.0);
        let head = Vec3::new(a.cos() * r * 0.7, a.sin() * r * 0.7, h);
        b.kit.beam(foot, head, 0.3, 0.3);
        // Cross bracing between legs.
        let a2 = a + FRAC_PI_2;
        b.kit.beam(foot * 0.95 + Vec3::Z * h * 0.2, Vec3::new(a2.cos() * r * 0.8, a2.sin() * r * 0.8, h * 0.6), 0.08, 0.08);
    }
    b.kit.mat(ids::RUST).frustum(Vec3::Z * h, r * 0.8, r * 0.8, r * 1.2, 16, true);
    b.kit.frustum(Vec3::Z * (h + r * 1.2), r * 0.85, 0.0, r * 0.5, 16, false);
}

fn kiosk(b: &mut Ctx, hx: f32, hy: f32) {
    b.kit.mat(ids::IRON).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 2.4));
    crate::generators::settlement::window(&mut b.kit, 0.0, -hy - 0.02, 1.0, hx * 1.4, 0.9, ids::GLASS);
    b.kit.mat(ids::ROOF).block(Vec3::new(-hx - 0.4, -hy - 0.6, 2.4), Vec3::new(hx + 0.4, hy + 0.3, 2.6));
    if b.p.parts.signs {
        let g = b.glow(true);
        b.kit.mat(g).block(Vec3::new(-hx, -hy - 0.62, 2.6), Vec3::new(hx, -hy - 0.5, 3.1));
    }
    if b.p.parts.awnings {
        b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hy - 0.03, 0.0)), |k| parts::awning(k, 0.0, 2.3, hx * 2.0, 0.8, true));
    }
}
