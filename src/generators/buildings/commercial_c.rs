//! Car dealership, fast-food drive-through and offices (low-rise,
//! mid-rise, campus). Ground at z = 0, entrance facing -Y.

use super::houses::framed;
use super::shops::{canopy, flat_box, parking, planters, pylon, sign, storefront};
use super::{BuildingKind as K, Ctx, doodads, parts};
use crate::generators::kit::ids;
use bevy::math::{Mat4, Vec2, Vec3};
use rand::Rng;

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    match b.p.kind {
        K::CarDealer => car_dealer(b, hx, hy),
        K::DriveThru => drive_thru(b, hx, hy),
        K::LowRiseOffice => low_rise(b, hx, hy, floors),
        K::MidRiseOffice => mid_rise(b, hx, hy, floors),
        K::OfficeCampus => campus(b, hx, hy, floors),
        _ => {}
    }
}

fn car_dealer(b: &mut Ctx, hx: f32, hy: f32) {
    // A tall glass showroom with cars inside, a forecourt lot and flags.
    let (h, y0) = (6.5, hy - 14.0);
    b.kit.mat(ids::ASPHALT).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, y0, 0.05));
    let n = ((hx * 2.0 / 3.2) as usize).max(2);
    for row in 0..2 {
        let y = -hy + 4.0 + row as f32 * 7.0;
        for i in 0..n {
            if b.r.gen_bool(0.2) {
                continue;
            }
            let x = -hx + 2.0 * hx * (i as f32 + 0.5) / n as f32;
            let m = Mat4::from_translation(Vec3::new(x, y, 0.05)) * Mat4::from_rotation_z(0.35);
            framed(b, m, |b| doodads::car(b, 0.92, 2.2));
        }
    }
    // Showroom: glazed front and sides on a low plinth.
    b.kit.mat(ids::CURB).block(Vec3::new(-hx * 0.7, y0, 0.0), Vec3::new(hx * 0.7, hy, 0.2));
    flat_box(&mut b.kit, -hx * 0.7, y0 + 1.0, hx * 0.7, hy, 0.2, h, ids::PLASTER, 0.6);
    storefront(b, 0.0, hx * 1.36, y0 + 0.98, 0.2, h - 1.0, true);
    for i in 0..2 {
        let m = Mat4::from_translation(Vec3::new(-hx * 0.35 + i as f32 * hx * 0.7, y0 + 5.0, 0.2));
        framed(b, m, |b| doodads::car(b, 0.92, 2.2));
    }
    b.kit.mat(ids::TRIM).block(Vec3::new(-hx * 0.72, y0, h - 0.6), Vec3::new(hx * 0.72, y0 + 1.0, h + 0.2));
    sign(b, 0.0, y0, h - 0.5, hx * 0.8);
    // Flag poles along the road edge.
    for i in 0..5 {
        let x = -hx + 1.0 + (2.0 * hx - 2.0) * i as f32 / 4.0;
        let p = Vec3::new(x, -hy + 0.5, 0.0);
        let flag = [ids::STUCCO, ids::ROOF, ids::TRIM][i % 3];
        b.kit.mat(ids::IRON).frustum(p, 0.06, 0.04, 7.0, 6, true);
        b.kit.mat(flag).block(p + Vec3::new(0.05, -0.02, 5.6), p + Vec3::new(1.4, 0.02, 6.8));
    }
    pylon(b, Vec3::new(hx - 1.5, -hy + 1.5, 0.0), 8.0, 4.0);
}

fn drive_thru(b: &mut Ctx, hx: f32, hy: f32) {
    // A small fast-food box ringed by a drive lane, a menu board, a pay
    // window canopy, parking and a tall pylon.
    let (h, bx, by) = (4.2, hx * 0.42, hy * 0.3);
    b.kit.mat(ids::ASPHALT).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.05));
    // Lane arrows / edge stripes around the building.
    b.kit.mat(ids::LANE);
    let (lx, ly) = (bx + 4.0, by + 4.0);
    for (a, c) in [(Vec2::new(-lx, -ly), Vec2::new(lx, -ly)), (Vec2::new(lx, -ly), Vec2::new(lx, ly)), (Vec2::new(lx, ly), Vec2::new(-lx, ly))] {
        let d = (c - a).normalize().perp() * 0.06;
        b.kit.quad((a - d).extend(0.06), (c - d).extend(0.06), (c + d).extend(0.06), (a + d).extend(0.06));
    }
    b.kit.mat(ids::CURB).block(Vec3::new(-bx - 1.2, -by - 1.2, 0.0), Vec3::new(bx + 1.2, by + 1.2, 0.15));
    let wall = [ids::BRICK, ids::STUCCO, ids::PLASTER][b.r.gen_range(0..3)];
    flat_box(&mut b.kit, -bx, -by, bx, by, 0.15, h, wall, 0.9);
    storefront(b, -bx * 0.3, bx * 1.1, -by, 0.15, 2.8, true);
    // A coloured band and a roof sign.
    let g = b.glow(true);
    b.kit.mat(ids::ROOF).block(Vec3::new(-bx - 0.1, -by - 0.1, h - 0.6), Vec3::new(bx + 0.1, by + 0.1, h + 0.15));
    sign(b, bx * 0.4, -by - 0.1, h + 0.2, bx * 0.9);
    // Pay window canopy on the +X side, menu board ahead of it.
    b.kit.mat(ids::TRIM).block(Vec3::new(bx, -by * 0.6, 2.8), Vec3::new(bx + 2.6, by * 0.6, 3.1));
    let mb = Vec3::new(lx + 1.4, ly - 2.0, 0.0);
    b.kit.mat(ids::IRON).block(mb - Vec3::new(0.1, 0.1, 0.0), mb + Vec3::new(0.1, 0.1, 1.2));
    b.kit.mat(ids::TRIM).block(mb + Vec3::new(-0.15, -1.0, 1.0), mb + Vec3::new(0.15, 1.0, 2.4));
    b.kit.mat(g).block(mb + Vec3::new(-0.17, -0.9, 1.1), mb + Vec3::new(-0.15, 0.9, 2.3));
    parking(b, Vec2::new(-hx, -hy), Vec2::new(hx * 0.4, -ly - 1.0), 2.6, 0.5);
    planters(&mut b.kit, -bx, bx, by + 1.6, 3);
    pylon(b, Vec3::new(hx - 1.5, -hy + 1.5, 0.0), 11.0, 3.6);
}

/// A curtain-wall band: dark glass with spandrel strips at each floor.
fn ribbons(b: &mut Ctx, hx: f32, hy: f32, z0: f32, floors: u32, fh: f32, glass_h: f32) {
    let pane = b.pane(0.5);
    for f in 0..floors {
        let z = z0 + f as f32 * fh + (fh - glass_h) * 0.6;
        let k = &mut b.kit;
        k.mat(pane).block(Vec3::new(-hx - 0.04, -hy - 0.04, z), Vec3::new(hx + 0.04, hy + 0.04, z + glass_h));
        k.mat(ids::IRON);
        let n = ((hx * 2.0 / 1.5) as usize).max(2);
        for i in 0..=n {
            let x = -hx + 2.0 * hx * i as f32 / n as f32;
            k.block(Vec3::new(x - 0.04, -hy - 0.1, z), Vec3::new(x + 0.04, -hy - 0.02, z + glass_h));
            k.block(Vec3::new(x - 0.04, hy + 0.02, z), Vec3::new(x + 0.04, hy + 0.1, z + glass_h));
        }
    }
}

fn low_rise(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // Ribbon-glazed office over a car park, entrance canopy, planters.
    let floors = floors.clamp(1, 6);
    let (fh, y0) = (3.6, -hy + hy * 0.55);
    parking(b, Vec2::new(-hx, -hy), Vec2::new(hx, y0 - 3.0), 2.6, 0.55);
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx, y0 - 3.0, 0.0), Vec3::new(hx, y0, 0.12));
    let (ox, oy) = (hx * 0.9, (hy - y0) * 0.5);
    let wall = if b.r.gen_bool(0.5) { ids::PLASTER } else { ids::FACADE };
    framed(b, Mat4::from_translation(Vec3::new(0.0, y0 + oy, 0.0)), |b| {
        let top = fh * floors as f32;
        b.kit.mat(wall).block(Vec3::new(-ox, -oy, 0.0), Vec3::new(ox, oy, top));
        ribbons(b, ox, oy, 0.0, floors, fh, 2.0);
        flat_box(&mut b.kit, -ox, -oy, ox, oy, top, 0.0, wall, 0.9);
        storefront(b, 0.0, 6.0, -oy - 0.06, 0.1, 3.0, true);
        canopy(&mut b.kit, 0.0, -oy, 8.0, 2.6, 3.4);
        sign(b, 0.0, -oy, top - 1.0, 6.0);
        if b.p.parts.rooftop {
            let r = &mut b.r;
            parts::rooftop(&mut b.kit, ox * 0.7, oy * 0.6, top + 0.12, r);
        }
    });
    planters(&mut b.kit, -hx + 1.0, hx - 1.0, y0 - 1.0, 4);
}

fn mid_rise(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // A glazed lobby, a gridded curtain-wall shaft and a set-back crown.
    let floors = floors.clamp(4, 40);
    let (fh, lobby) = (3.5, 5.0);
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, 0.12));
    let (sx, sy) = (hx - 2.0, hy - 3.0);
    flat_box(&mut b.kit, -sx, -sy, sx, sy, 0.12, lobby, ids::FACADE, 0.0);
    storefront(b, 0.0, sx * 1.6, -sy, 0.12, lobby - 0.6, true);
    canopy(&mut b.kit, 0.0, -sy, sx, 2.5, lobby - 0.4);
    let upper = floors - 2;
    let wall = [ids::PLASTER, ids::FACADE, ids::STUCCO][b.r.gen_range(0..3)];
    let z0 = lobby + 0.12;
    b.kit.mat(wall).block(Vec3::new(-sx, -sy, z0), Vec3::new(sx, sy, z0 + fh * upper as f32));
    ribbons(b, sx, sy, z0, upper, fh, 2.3);
    let top = z0 + fh * upper as f32;
    let (cx, cy) = (sx * 0.75, sy * 0.75);
    b.kit.mat(wall).block(Vec3::new(-cx, -cy, top), Vec3::new(cx, cy, top + fh * 2.0));
    ribbons(b, cx, cy, top, 2, fh, 2.3);
    flat_box(&mut b.kit, -cx, -cy, cx, cy, top + fh * 2.0, 0.0, ids::TRIM, 1.2);
    if b.p.parts.signs {
        let g = b.glow(false);
        b.kit.mat(g).block(Vec3::new(-cx * 0.5, -cy - 0.06, top + fh * 2.0 - 1.0), Vec3::new(cx * 0.5, -cy - 0.02, top + fh * 2.0 - 0.3));
    }
    planters(&mut b.kit, -hx + 1.0, hx - 1.0, -hy + 1.0, 4);
}

fn campus(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // Three ribbon-glazed blocks round a lawn courtyard, joined by glazed
    // bridges, with parking out front.
    let floors = floors.clamp(1, 8);
    let fh = 3.6;
    let top = fh * floors as f32;
    let y0 = -hy + hy * 0.4;
    parking(b, Vec2::new(-hx, -hy), Vec2::new(hx, y0 - 2.0), 2.6, 0.55);
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx, y0 - 2.0, 0.0), Vec3::new(hx, hy, 0.1));
    let (w, d) = (hx * 0.3, (hy - y0) * 0.5);
    // Lawn in the middle.
    b.kit.mat(ids::FOLIAGE).block(Vec3::new(-hx + 2.0 * w + 1.0, y0 + 1.0, 0.1), Vec3::new(hx - 2.0 * w - 1.0, hy - 2.0 * w * 0.6, 0.18));
    let wall = if b.r.gen_bool(0.5) { ids::PLASTER } else { ids::STUCCO };
    let blocks = [
        (Vec3::new(-hx + w, y0 + d, 0.0), w, d * 0.95),
        (Vec3::new(hx - w, y0 + d, 0.0), w, d * 0.95),
        (Vec3::new(0.0, hy - w * 0.6, 0.0), hx - 2.0 * w - 1.0, w * 0.6),
    ];
    for (i, (c, bx, by)) in blocks.into_iter().enumerate() {
        let f = if i == 2 { floors } else { (floors + 1).min(8) };
        let t = fh * f as f32;
        framed(b, Mat4::from_translation(c), |b| {
            b.kit.mat(wall).block(Vec3::new(-bx, -by, 0.0), Vec3::new(bx, by, t));
            ribbons(b, bx, by, 0.0, f, fh, 2.1);
            flat_box(&mut b.kit, -bx, -by, bx, by, t, 0.0, wall, 0.9);
            if i < 2 {
                storefront(b, 0.0, bx * 1.2, -by - 0.06, 0.1, 3.0, true);
            } else if b.p.parts.rooftop {
                let r = &mut b.r;
                parts::rooftop(&mut b.kit, bx * 0.6, by * 0.6, t + 0.12, r);
            }
        });
    }
    // Glazed bridges at the top floor from the side blocks to the back.
    let pane = b.pane(0.6);
    for s in [-1.0f32, 1.0] {
        let x0 = s * (hx - w);
        let k = &mut b.kit;
        let (a, c) = (Vec3::new(x0 - w * 0.4, hy - 2.0 * w * 0.6 - 4.0, top - fh), Vec3::new(x0 + w * 0.4, hy - 2.0 * w * 0.6, top - 0.4));
        k.mat(pane).block(a, c);
        k.mat(ids::TRIM).block(a - Vec3::Z * 0.3, Vec3::new(c.x, c.y, a.z));
    }
    canopy(&mut b.kit, 0.0, y0 + 1.0, 6.0, 2.4, 3.2);
    sign(b, -hx + w, y0 + d - d * 0.95, top - 1.0, w * 1.2);
    planters(&mut b.kit, -hx + 2.0 * w + 2.0, hx - 2.0 * w - 2.0, y0 + 0.5, 3);
}
