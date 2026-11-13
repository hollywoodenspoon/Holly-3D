//! Small shops and places to eat: corner shop, café, diner, restaurant,
//! bakery and pharmacy. Ground at z = 0, shopfront facing -Y.

use super::houses::{Roof, Shell, door, roof, walls};
use super::shops::{canopy, flat_box, planters, pylon, sign, storefront, tables};
use super::{BuildingKind as K, Ctx, massing, parts};
use crate::generators::kit::ids;
use bevy::math::{Mat4, Vec2, Vec3};
use rand::Rng;

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    match b.p.kind {
        K::CornerShop => corner_shop(b, hx, hy, floors),
        K::Cafe => cafe(b, hx, hy),
        K::Diner => diner(b, hx, hy),
        K::Restaurant => restaurant(b, hx, hy),
        K::Bakery => bakery(b, hx, hy, floors),
        K::Pharmacy => pharmacy(b, hx, hy),
        _ => {}
    }
}

fn awning(b: &mut Ctx, x: f32, y: f32, z: f32, w: f32, stripe: bool) {
    if b.p.parts.awnings {
        b.kit.at(Mat4::from_translation(Vec3::new(0.0, y - 0.03, 0.0)), |k| parts::awning(k, x, z, w, 1.4, stripe));
    }
}

fn corner_shop(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // Shop on the ground floor wrapping the -X / -Y corner, flats above.
    let fh = 3.6;
    let wall = if b.r.gen_bool(0.5) { ids::BRICK } else { ids::STUCCO };
    b.kit.mat(wall).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, fh));
    storefront(b, -hx * 0.25, hx * 1.3, -hy, 0.4, 2.6, true);
    // Side shopfront on -X, built in a frame turned to face -X.
    let m = Mat4::from_rotation_z(-std::f32::consts::FRAC_PI_2);
    super::houses::framed(b, m, |b| storefront(b, hy * 0.2, hy * 1.2, -hx, 0.4, 2.6, false));
    let up = floors.clamp(1, 5) - 1;
    if up > 0 {
        massing(b, hx, hy, fh, up, 3.0, wall, 2.4, 1.0, 1.5, 0.35);
    }
    let top = fh + 3.0 * up as f32;
    b.kit.mat(ids::TRIM).block(Vec3::new(-hx - 0.1, -hy - 0.1, fh - 0.25), Vec3::new(hx + 0.1, hy + 0.1, fh));
    flat_box(&mut b.kit, -hx, -hy, hx, hy, top, 0.0, wall, 0.7);
    awning(b, -hx * 0.25, -hy, fh - 0.35, hx * 1.3, true);
    sign(b, -hx * 0.25, -hy, fh - 0.1, hx * 1.1);
    if b.p.parts.fire_escapes && up > 1 {
        b.kit.at(Mat4::from_rotation_z(std::f32::consts::PI) * Mat4::from_translation(Vec3::new(0.0, -hy - 0.03, fh - 3.0)), |k| {
            parts::fire_escape(k, 0.0, hx.min(3.5), up + 1, 3.0)
        });
    }
}

fn cafe(b: &mut Ctx, hx: f32, hy: f32) {
    // A one-storey café set back behind a terrace of tables.
    let (h, back) = (3.8, -hy + 3.2);
    let wall = [ids::PLASTER, ids::STUCCO, ids::BRICK][b.r.gen_range(0..3)];
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, back, 0.08));
    flat_box(&mut b.kit, -hx, back, hx, hy, 0.0, h, wall, 0.5);
    storefront(b, 0.0, hx * 1.6, back, 0.08, 2.7, true);
    awning(b, 0.0, back, h - 0.3, hx * 1.9, true);
    sign(b, 0.0, back, h - 0.05, hx * 1.0);
    tables(b, Vec2::new(0.0, -hy + 1.4), hx - 1.0, ((hx * 2.0 / 2.6) as usize).clamp(1, 6));
    planters(&mut b.kit, -hx + 0.5, hx - 0.5, -hy + 0.45, 2);
}

fn diner(b: &mut Ctx, hx: f32, hy: f32) {
    // Long low diner: steel band, ribbon of windows, a sign pole.
    let (h, y0, y1) = (3.6, -hy * 0.45, hy * 0.7);
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, y1 + 0.6, 0.08));
    b.kit.mat(ids::CURB).block(Vec3::new(-hx + 0.6, y0, 0.0), Vec3::new(hx - 0.6, y1, 0.6));
    flat_box(&mut b.kit, -hx + 0.6, y0, hx - 0.6, y1, 0.6, h - 0.6, ids::PLASTER, 0.0);
    b.kit.mat(ids::IRON).block(Vec3::new(-hx + 0.55, y0 - 0.06, 0.65), Vec3::new(hx - 0.55, y1 + 0.06, 1.15));
    b.kit.mat(ids::IRON).block(Vec3::new(-hx + 0.55, y0 - 0.06, h - 0.45), Vec3::new(hx - 0.55, y1 + 0.06, h - 0.2));
    let pane = b.pane(0.8);
    crate::generators::settlement::window(&mut b.kit, 0.0, y0 - 0.07, 1.25, (hx - 1.2) * 2.0, 1.4, pane);
    // Entrance vestibule on the front.
    flat_box(&mut b.kit, -1.4, y0 - 1.8, 1.4, y0, 0.0, 3.1, ids::PLASTER, 0.2);
    storefront(b, 0.0, 2.2, y0 - 1.8, 0.1, 2.4, true);
    sign(b, 0.0, y0, h, hx * 0.8);
    pylon(b, Vec3::new(hx - 1.2, -hy + 0.8, 0.0), 6.0, 3.0);
}

fn restaurant(b: &mut Ctx, hx: f32, hy: f32) {
    // Brick restaurant with tall windows, a canopy and planters.
    let h = 4.6;
    let (y0, top) = (-hy + 2.0, 4.6);
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, y0, 0.08));
    flat_box(&mut b.kit, -hx, y0, hx, hy, 0.0, h, ids::BRICK, 0.9);
    let n = ((hx * 2.0 / 3.0) as usize).max(2);
    for i in 0..n {
        let x = -hx + 2.0 * hx * (i as f32 + 0.5) / n as f32;
        if x.abs() < 1.6 {
            continue;
        }
        let pane = b.pane(0.8);
        crate::generators::settlement::window(&mut b.kit, x, y0 - 0.02, 0.7, 1.6, 2.9, pane);
        b.kit.mat(ids::TRIM).block(Vec3::new(x - 1.0, y0 - 0.15, 3.6), Vec3::new(x + 1.0, y0, 3.85));
    }
    door(&mut b.kit, 0.0, y0 - 0.02, 0.08, 1.6, 2.6);
    canopy(&mut b.kit, 0.0, y0, 3.0, 1.9, 3.0);
    sign(b, 0.0, y0, top - 0.6, hx);
    planters(&mut b.kit, -hx + 0.6, -1.8, -hy + 0.6, 2);
    planters(&mut b.kit, 1.8, hx - 0.6, -hy + 0.6, 2);
}

fn bakery(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // Narrow bakery with a bow display window, awning and a flat above.
    let wall = if b.r.gen_bool(0.5) { ids::STUCCO } else { ids::PLASTER };
    let mut s = Shell::new(hx, hy, floors.clamp(1, 3), wall);
    (s.fh, s.base, s.bay, s.door) = (3.2, 0.15, 2.0, Some(hx * 0.55));
    walls(b, &s);
    let top = s.top();
    roof(&mut b.kit, hx, hy, top, Roof::Gable(42.0), 0.3, wall);
    let (bx, bw) = (-hx * 0.25, hx * 1.0);
    let bow = [Vec2::new(bx - bw * 0.5, -hy), Vec2::new(bx - bw * 0.4, -hy - 0.6), Vec2::new(bx + bw * 0.4, -hy - 0.6), Vec2::new(bx + bw * 0.5, -hy)];
    b.kit.mat(ids::TRIM).prism(&bow, 0.0, 0.7, true, false);
    b.kit.mat(ids::TRIM).prism(&bow, 2.7, 0.25, true, false);
    let pane = b.pane(0.9);
    crate::generators::settlement::window(&mut b.kit, bx, -hy - 0.62, 0.7, bw * 0.8, 2.0, pane);
    awning(b, 0.0, -hy - 0.6, 3.3, hx * 1.8, true);
    sign(b, 0.0, -hy, 3.4, hx * 1.2);
    if b.p.parts.chimneys {
        parts::chimney(&mut b.kit, Vec3::new(hx * 0.5, hy * 0.4, top), 2.6);
    }
}

fn pharmacy(b: &mut Ctx, hx: f32, hy: f32) {
    // Flat box with a glazed corner, a glowing cross and a few bays.
    let h = 4.4;
    let y0 = -hy + 5.5;
    super::shops::parking(b, Vec2::new(-hx, -hy), Vec2::new(hx, y0 - 0.4), 2.6, 0.35);
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx, y0 - 0.4, 0.0), Vec3::new(hx, y0, 0.12));
    flat_box(&mut b.kit, -hx, y0, hx, hy, 0.0, h, ids::PLASTER, 0.6);
    storefront(b, -hx * 0.3, hx * 1.2, y0, 0.12, 3.0, true);
    canopy(&mut b.kit, -hx * 0.3, y0, 4.0, 1.6, 3.3);
    sign(b, -hx * 0.3, y0, h - 0.8, hx * 1.1);
    if b.p.parts.signs {
        let g = b.glow(true);
        let c = Vec3::new(hx - 1.2, y0 - 0.25, h - 1.0);
        b.kit.mat(g).block(c - Vec3::new(0.7, 0.12, 0.22), c + Vec3::new(0.7, 0.12, 0.22));
        b.kit.block(c - Vec3::new(0.22, 0.12, 0.7), c + Vec3::new(0.22, 0.12, 0.7));
    }
}
