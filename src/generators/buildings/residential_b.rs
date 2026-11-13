//! Houses, part two, and mid / high density homes: villa, Victorian
//! house, modern box, row houses, duplex, walk-up and residential tower.

use super::houses::{Roof, Shell, door, framed, lot, rise, roof, walls};
use super::{BuildingKind as K, Ctx, massing, parts};
use crate::generators::kit::{ids, sides};
use crate::generators::settlement::window;
use bevy::math::{Mat4, Vec2, Vec3};
use rand::Rng;

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    match b.p.kind {
        K::Villa => villa(b, hx, hy, floors),
        K::Victorian => victorian(b, hx, hy, floors),
        K::ModernBox => modern_box(b, hx, hy, floors),
        K::RowHouses => row_houses(b, hx, hy, floors),
        K::Duplex => duplex(b, hx, hy, floors),
        K::WalkUp => walk_up(b, hx, hy, floors),
        K::TowerBlock => tower(b, hx, hy, floors),
        _ => {}
    }
}

fn stoop(b: &mut Ctx, x: f32, w: f32, y: f32, rise: f32) {
    if b.p.parts.porches {
        let kind = b.p.stairs;
        b.kit.at(Mat4::from_translation(Vec3::new(0.0, y - 0.02, 0.0)), |k| parts::porch(k, x, w, 1.4, rise, kind));
    }
}

fn villa(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // Main block plus a lower side wing, both under hipped tile roofs.
    let main_hx = hx * 0.68;
    let mut s = Shell::new(main_hx, hy, floors.clamp(1, 3), ids::STUCCO);
    (s.fh, s.bay, s.win) = (3.2, 2.6, (1.1, 1.8));
    let off = -hx + main_hx;
    let top = s.top();
    framed(b, Mat4::from_translation(Vec3::X * off), |b| {
        walls(b, &s);
        roof(&mut b.kit, main_hx, hy, top, Roof::Hip(24.0), 0.7, ids::STUCCO);
        if b.p.parts.balconies && s.floors > 1 {
            let (z, w) = (s.base + s.fh, main_hx * 0.9);
            b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hy, 0.0)), |k| parts::balcony(k, 0.0, z, w, 1.1));
        }
        // Columned entrance.
        b.kit.mat(ids::PLASTER);
        for x in [-0.9, 0.9] {
            b.kit.frustum(Vec3::new(x, -hy - 1.4, s.base), 0.16, 0.14, s.fh - 0.1, 10, true);
        }
        b.kit.mat(ids::TRIM).block(Vec3::new(-1.3, -hy - 1.7, s.base + s.fh - 0.1), Vec3::new(1.3, -hy, s.base + s.fh + 0.2));
        b.kit.mat(ids::TRIM).block(Vec3::new(-1.3, -hy - 1.7, 0.0), Vec3::new(1.3, -hy, s.base));
    });
    // The wing.
    let wing_hx = hx - main_hx;
    let wing_hy = hy * 0.75;
    let mut w = Shell::new(wing_hx, wing_hy, 1, ids::STUCCO);
    (w.fh, w.bay, w.win, w.door) = (3.2, 2.4, (1.6, 2.0), None);
    framed(b, Mat4::from_translation(Vec3::new(hx - wing_hx, hy - wing_hy, 0.0)), |b| {
        walls(b, &w);
        roof(&mut b.kit, wing_hx, wing_hy, w.top(), Roof::Hip(24.0), 0.6, ids::STUCCO);
    });
    if b.p.parts.chimneys {
        parts::chimney(&mut b.kit, Vec3::new(off + main_hx * 0.4, hy * 0.2, top), rise(main_hx, hy, 24.0) + 0.6);
    }
    lot(b, hx, hy, off);
}

fn victorian(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    let wall = [ids::PLASTER, ids::STUCCO, ids::FACADE][b.r.gen_range(0..3)];
    let mut s = Shell::new(hx, hy, floors.clamp(2, 4), wall);
    (s.fh, s.bay, s.win, s.base, s.door) = (3.1, 1.9, (0.85, 1.7), 0.8, Some(hx * 0.35));
    walls(b, &s);
    let top = s.top();
    roof(&mut b.kit, hx, hy, top, Roof::Gable(52.0), 0.4, wall);
    // Two-storey bay window on the front left.
    let (bx, bw, bd) = (-hx * 0.4, (hx * 0.7).min(2.6), 0.9);
    let bay = [Vec2::new(bx - bw * 0.5, -hy), Vec2::new(bx - bw * 0.3, -hy - bd), Vec2::new(bx + bw * 0.3, -hy - bd), Vec2::new(bx + bw * 0.5, -hy)];
    let bay_h = s.fh * 2.0_f32.min(s.floors as f32);
    b.kit.mat(wall).prism(&bay, s.base, bay_h, false, false);
    b.kit.mat(ids::ROOF).prism(&bay, s.base + bay_h, 0.35, true, false);
    let pane = b.pane(0.4);
    for f in 0..2.min(s.floors) {
        let z = s.base + f as f32 * s.fh + 0.6;
        window(&mut b.kit, bx, -hy - bd - 0.02, z, bw * 0.5, 1.6, pane);
    }
    // Corner turret with a conical cap.
    let (tr, tc) = (1.2, Vec3::new(hx - 0.3, -hy + 0.3, 0.0));
    let th = top + 1.2;
    b.kit.mat(wall).frustum(tc, tr, tr, th, 8, false);
    b.kit.mat(ids::TRIM).frustum(tc + Vec3::Z * th, tr + 0.15, tr + 0.15, 0.25, 8, true);
    b.kit.mat(ids::SLATE).frustum(tc + Vec3::Z * (th + 0.25), tr + 0.2, 0.0, 3.2, 8, false);
    for f in 0..s.floors {
        let z = s.base + f as f32 * s.fh + 0.7;
        b.kit.at(Mat4::from_translation(tc) * Mat4::from_rotation_z(-0.4), |k| window(k, 0.0, -tr - 0.02, z, 0.6, 1.4, pane));
    }
    // Wraparound porch with gingerbread posts.
    if b.p.parts.porches {
        let (d, z) = (1.9, s.base);
        b.kit.mat(ids::TRIM).block(Vec3::new(-hx * 0.05, -hy - d, 0.0), Vec3::new(hx - tr - 0.1, -hy, z));
        for x in [0.0, (hx - tr) * 0.5, hx - tr - 0.25] {
            b.kit.block(Vec3::new(x - 0.07, -hy - d + 0.05, z), Vec3::new(x + 0.07, -hy - d + 0.19, z + 2.5));
        }
        b.kit.mat(ids::ROOF).block(Vec3::new(-hx * 0.05 - 0.2, -hy - d - 0.2, z + 2.5), Vec3::new(hx - tr, -hy, z + 2.65));
        let kind = b.p.stairs;
        b.kit.at(Mat4::from_translation(Vec3::new(hx * 0.35, -hy - d, 0.0)) * Mat4::from_rotation_z(std::f32::consts::PI), |k| {
            super::arch::stairs(k, kind, 1.3, z);
        });
    }
    if b.p.parts.chimneys {
        parts::chimney(&mut b.kit, Vec3::new(-hx * 0.6, hy * 0.3, top), rise(hx, hy, 52.0) + 0.7);
    }
    lot(b, hx, hy, hx * 0.35);
}

fn modern_box(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // Ground floor box, an upper box shifted out over a glass front, and
    // ribbon windows; flat roofs with slim parapets.
    let f = floors.clamp(1, 4);
    let fh = 3.1;
    b.kit.mat(ids::PLASTER).block(Vec3::new(-hx, -hy * 0.6, 0.0), Vec3::new(hx * 0.3, hy, fh));
    let glass = b.pane(0.55);
    b.kit.mat(glass).block(Vec3::new(hx * 0.3, -hy * 0.6, 0.0), Vec3::new(hx, hy * 0.5, fh));
    b.kit.mat(ids::IRON);
    for i in 0..=4 {
        let x = hx * 0.3 + hx * 0.7 * i as f32 / 4.0;
        b.kit.block(Vec3::new(x - 0.05, -hy * 0.6 - 0.05, 0.0), Vec3::new(x + 0.05, -hy * 0.6, fh));
    }
    door(&mut b.kit, -hx * 0.4, -hy * 0.6, 0.0, 1.1, 2.4);
    for i in 1..f {
        let z = fh * i as f32;
        b.kit.mat(ids::STUCCO).block(Vec3::new(-hx * 0.8, -hy, z), Vec3::new(hx, hy * 0.8, z + fh));
        let pane = b.pane(0.5);
        b.kit.at(Mat4::from_translation(Vec3::new(0.1 * hx, -hy - 0.03, 0.0)), |k| window(k, 0.0, 0.0, z + 0.8, hx * 1.4, 1.5, pane));
        b.kit.at(Mat4::from_translation(Vec3::new(hx + 0.03, 0.0, 0.0)) * Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2), |k| {
            window(k, 0.0, 0.0, z + 0.8, hy * 1.2, 1.5, pane)
        });
    }
    let top = fh * f as f32;
    let (ux0, ux1, uy0, uy1) = if f > 1 { (-hx * 0.8, hx, -hy, hy * 0.8) } else { (-hx, hx, -hy * 0.6, hy) };
    b.kit.at(Mat4::from_translation(Vec3::new((ux0 + ux1) * 0.5, (uy0 + uy1) * 0.5, 0.0)), |k| {
        roof(k, (ux1 - ux0) * 0.5, (uy1 - uy0) * 0.5, top, Roof::Flat(0.5), 0.0, ids::TRIM)
    });
    if f > 1 {
        // Roof over the ground-floor part that sticks out.
        b.kit.mat(ids::ROOF).block(Vec3::new(-hx, -hy * 0.6, fh), Vec3::new(-hx * 0.8, hy, fh + 0.15));
    }
    if b.p.parts.rooftop {
        let r = &mut b.r;
        parts::rooftop(&mut b.kit, (ux1 - ux0) * 0.3, (uy1 - uy0) * 0.3, top + 0.12, r);
    }
    lot(b, hx, hy, -hx * 0.4);
}

fn row_houses(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // Narrow terraced homes side by side, each with its own colour, door,
    // stoop and chimney on the party wall.
    let n = ((hx * 2.0 / 5.5).round() as usize).max(2);
    let uw = hx * 2.0 / n as f32;
    let colours = [ids::BRICK, ids::STUCCO, ids::PLASTER, ids::FACADE];
    let start = b.r.gen_range(0..4);
    let floors = floors.clamp(1, 4);
    let mut top = 0.0;
    for i in 0..n {
        let cx = -hx + uw * (i as f32 + 0.5);
        let wall = colours[(start + i) % 4];
        let mut s = Shell::new(uw * 0.5, hy, floors, wall);
        (s.fh, s.bay, s.win, s.base, s.door) = (2.9, uw * 0.5, (0.9, 1.5), 0.6, Some(if i % 2 == 0 { -uw * 0.25 } else { uw * 0.25 }));
        framed(b, Mat4::from_translation(Vec3::X * cx), |b| {
            walls(b, &s);
            stoop(b, s.door.unwrap_or(0.0), 1.3, -hy, s.base);
            if b.p.parts.chimneys && i > 0 {
                parts::chimney(&mut b.kit, Vec3::new(-uw * 0.5, hy * 0.1, s.top()), rise(hx, hy, 38.0) + 0.6);
            }
        });
        top = s.top();
    }
    roof(&mut b.kit, hx, hy, top, Roof::Gable(38.0), 0.3, ids::BRICK);
    if b.p.lot.garden {
        b.kit.mat(ids::FOLIAGE).block(Vec3::new(-hx, -hy - 3.5, 0.0), Vec3::new(hx, -hy, 0.04));
    }
    if b.p.lot.fence {
        b.kit.mat(ids::IRON).block(Vec3::new(-hx, -hy - 3.55, 0.0), Vec3::new(hx, -hy - 3.45, 0.9));
    }
}

fn duplex(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    let wall = if b.r.gen_bool(0.5) { ids::BRICK } else { ids::PLASTER };
    let mut s = Shell::new(hx, hy, floors.clamp(1, 3), wall);
    (s.fh, s.bay, s.win, s.door) = (2.9, 2.3, (1.0, 1.4), None);
    walls(b, &s);
    let top = s.top();
    // Party-wall line, two doors with porches, two chimneys.
    b.kit.mat(ids::TRIM).block(Vec3::new(-0.08, -hy - 0.1, s.base), Vec3::new(0.08, hy + 0.1, top));
    for x in [-1.1, 1.1] {
        door(&mut b.kit, x, -hy - 0.02, s.base, 0.95, 2.1);
        stoop(b, x, 1.6, -hy, s.base);
        if b.p.parts.chimneys {
            parts::chimney(&mut b.kit, Vec3::new(x * hx * 0.6, hy * 0.3, top), rise(hx, hy, 34.0) + 0.6);
        }
    }
    roof(&mut b.kit, hx, hy, top, Roof::Gable(34.0), 0.4, wall);
    lot(b, hx, hy, 0.0);
}

fn walk_up(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // Brick walk-up: stacked bay windows, a stoop per stair and a cornice.
    let floors = floors.clamp(2, 7);
    let (fh, base) = (3.0, 0.9);
    b.kit.mat(ids::CURB).block(Vec3::new(-hx - 0.05, -hy - 0.05, 0.0), Vec3::new(hx + 0.05, hy + 0.05, base));
    massing(b, hx, hy, base, floors, fh, ids::BRICK, 2.3, 1.0, 1.6, 0.35);
    let top = base + fh * floors as f32;
    let bays = [-hx * 0.5, hx * 0.5];
    for bx in bays {
        let bw = (hx * 0.6).min(3.2);
        let bay = [Vec2::new(bx - bw * 0.5, -hy), Vec2::new(bx - bw * 0.35, -hy - 0.8), Vec2::new(bx + bw * 0.35, -hy - 0.8), Vec2::new(bx + bw * 0.5, -hy)];
        b.kit.mat(ids::BRICK).prism(&bay, base + fh, fh * (floors - 1) as f32, false, true);
        b.kit.mat(ids::TRIM).prism(&bay, top, 0.2, true, false);
        for f in 1..floors {
            let pane = b.pane(0.35);
            window(&mut b.kit, bx, -hy - 0.82, base + f as f32 * fh + 0.6, bw * 0.55, 1.6, pane);
        }
    }
    door(&mut b.kit, 0.0, -hy - 0.03, base, 1.4, 2.4);
    stoop(b, 0.0, 2.2, -hy, base);
    b.kit.mat(ids::TRIM).block(Vec3::new(-hx - 0.35, -hy - 0.35, top), Vec3::new(hx + 0.35, hy + 0.35, top + 0.5));
    if b.p.parts.fire_escapes {
        b.kit.at(Mat4::from_rotation_z(std::f32::consts::PI) * Mat4::from_translation(Vec3::new(0.0, -hy - 0.03, base)), |k| {
            parts::fire_escape(k, 0.0, hx.min(4.0), floors, fh)
        });
    }
    if b.p.parts.rooftop {
        let r = &mut b.r;
        parts::rooftop(&mut b.kit, hx, hy, top + 0.5, r);
    }
}

fn tower(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // Glazed lobby, a slab of flats with a balcony per bay on every floor,
    // and a plant room on top.
    let floors = floors.clamp(4, 80);
    let (fh, lobby) = (2.95, 4.2);
    let glass = b.pane(0.5);
    b.kit.mat(glass).block(Vec3::new(-hx + 0.6, -hy + 0.6, 0.0), Vec3::new(hx - 0.6, hy - 0.6, lobby));
    b.kit.mat(ids::PLASTER);
    for (x, y) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
        let c = Vec3::new(x * (hx - 0.4), y * (hy - 0.4), 0.0);
        b.kit.block(c - Vec3::new(0.4, 0.4, 0.0), c + Vec3::new(0.4, 0.4, lobby));
    }
    massing(b, hx, hy, lobby, floors - 1, fh, ids::PLASTER, 3.2, 1.6, 1.6, 0.4);
    let top = lobby + fh * (floors - 1) as f32;
    if b.p.parts.balconies {
        sides(&mut b.kit, hx, hy, |k, side, w| {
            if side % 2 == 1 {
                return;
            }
            let n = ((w / 6.4).floor() as usize).max(1);
            for f in 0..floors - 1 {
                for i in 0..n {
                    let x = -w * 0.5 + w * (i as f32 + 0.5) / n as f32;
                    parts::balcony(k, x, lobby + f as f32 * fh + 0.15, 3.0, 1.2);
                }
            }
        });
    }
    door(&mut b.kit, 0.0, -hy + 0.58, 0.0, 2.2, 2.8);
    b.kit.mat(ids::TRIM).block(Vec3::new(-hx - 0.2, -hy - 0.2, top), Vec3::new(hx + 0.2, hy + 0.2, top + 0.4));
    b.kit.mat(ids::PLASTER).block(Vec3::new(-hx * 0.35, -hy * 0.35, top + 0.4), Vec3::new(hx * 0.35, hy * 0.35, top + 3.4));
    if b.p.parts.rooftop {
        let r = &mut b.r;
        parts::rooftop(&mut b.kit, hx * 0.8, hy * 0.8, top + 0.4, r);
    }
}
