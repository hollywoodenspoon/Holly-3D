//! Big commercial buildings: supermarket, mall, hotel, motel, bank and
//! cinema. Ground at z = 0, entrance facing -Y; car parks sit in front.

use super::shops::{canopy, flat_box, parking, planters, pylon, sign, storefront};
use super::{BuildingKind as K, Ctx, massing, parts};
use crate::generators::kit::{ids, sides};
use crate::generators::settlement::window;
use bevy::math::{Mat4, Vec2, Vec3};
use rand::Rng;

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    match b.p.kind {
        K::Supermarket => supermarket(b, hx, hy),
        K::Mall => mall(b, hx, hy, floors),
        K::Hotel => hotel(b, hx, hy, floors),
        K::Motel => motel(b, hx, hy, floors),
        K::Bank => bank(b, hx, hy),
        K::Cinema => cinema(b, hx, hy),
        _ => {}
    }
}

fn supermarket(b: &mut Ctx, hx: f32, hy: f32) {
    // A big box at the back of its car park, glazed entrance, trolley bay.
    let (h, y0) = (7.0, -hy + hy * 0.95);
    parking(b, Vec2::new(-hx, -hy), Vec2::new(hx, y0 - 2.5), 2.6, 0.45);
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx, y0 - 2.5, 0.0), Vec3::new(hx, y0, 0.15));
    flat_box(&mut b.kit, -hx, y0, hx, hy, 0.0, h, ids::PLASTER, 0.8);
    b.kit.mat(ids::TRIM).block(Vec3::new(-hx - 0.05, y0 - 0.06, h - 1.6), Vec3::new(hx + 0.05, y0, h - 1.2));
    storefront(b, -hx * 0.35, hx * 0.6, y0, 0.15, 3.6, true);
    canopy(&mut b.kit, -hx * 0.35, y0, hx * 0.7, 2.4, 4.2);
    sign(b, -hx * 0.35, y0, h - 1.1, hx * 0.5);
    // Trolley shelter and loading dock round the side.
    let t = Vec3::new(hx * 0.4, y0 - 1.8, 0.15);
    b.kit.mat(ids::IRON).block(t - Vec3::new(2.0, 0.6, 0.0), t + Vec3::new(2.0, 0.6, 0.05));
    b.kit.mat(ids::GLASS).block(t + Vec3::new(-2.0, -0.6, 2.2), t + Vec3::new(2.0, 0.6, 2.3));
    for x in [-1.9, 1.9] {
        b.kit.mat(ids::IRON).block(t + Vec3::new(x - 0.05, -0.55, 0.0), t + Vec3::new(x + 0.05, -0.45, 2.2));
    }
    b.kit.mat(ids::CURB).block(Vec3::new(hx, hy - 8.0, 0.0), Vec3::new(hx + 2.5, hy - 1.0, 1.2));
    for i in 0..2 {
        let y = hy - 6.5 + i as f32 * 3.5;
        b.kit.mat(ids::IRON).quad(Vec3::new(hx + 0.02, y - 1.4, 1.2), Vec3::new(hx + 0.02, y + 1.4, 1.2), Vec3::new(hx + 0.02, y + 1.4, 4.0), Vec3::new(hx + 0.02, y - 1.4, 4.0));
    }
    pylon(b, Vec3::new(hx - 1.5, -hy + 1.0, 0.0), 7.0, 4.0);
}

fn mall(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // Two anchor blocks joined by a glazed central atrium, car park in front.
    let levels = floors.clamp(1, 4);
    let (fh, y0) = (5.0, -hy + hy * 0.7);
    let h = fh * levels as f32;
    parking(b, Vec2::new(-hx, -hy), Vec2::new(hx, y0 - 3.0), 2.6, 0.5);
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx, y0 - 3.0, 0.0), Vec3::new(hx, y0, 0.15));
    let ax = hx * 0.36;
    for s in [-1.0f32, 1.0] {
        let (x0, x1) = if s < 0.0 { (-hx, -hx + 2.0 * ax) } else { (hx - 2.0 * ax, hx) };
        let wall = if s < 0.0 { ids::STUCCO } else { ids::PLASTER };
        flat_box(&mut b.kit, x0, y0, x1, hy, 0.0, h + 1.5, wall, 1.0);
        let cx = (x0 + x1) * 0.5;
        storefront(b, cx, ax * 0.8, y0, 0.15, 3.4, true);
        sign(b, cx, y0, h - 0.4, ax * 0.9);
    }
    // The mall between them.
    let (mx, my) = (hx - 2.0 * ax, y0 + 4.0);
    flat_box(&mut b.kit, -mx, my, mx, hy, 0.0, h, ids::FACADE, 0.8);
    for l in 0..levels {
        let n = ((mx * 2.0 / 8.0) as usize).max(2);
        for i in 0..n {
            let x = -mx + 2.0 * mx * (i as f32 + 0.5) / n as f32;
            if x.abs() < 6.0 {
                continue;
            }
            storefront(b, x, 6.0, my, l as f32 * fh + 0.3, 3.2, l == 0);
        }
    }
    // Glazed atrium: a glass box with a pyramid-ish roof.
    let pane = b.pane(0.6);
    b.kit.mat(pane).block(Vec3::new(-6.0, y0 - 1.0, 0.15), Vec3::new(6.0, my + 1.0, h + 2.0));
    b.kit.mat(ids::IRON);
    for i in 0..=6 {
        let x = -6.0 + 2.0 * i as f32;
        b.kit.block(Vec3::new(x - 0.08, y0 - 1.06, 0.15), Vec3::new(x + 0.08, y0 - 0.94, h + 2.0));
    }
    b.kit.mat(pane).frustum(Vec3::new(0.0, (y0 + my) * 0.5, h + 2.0), 6.5, 1.0, 3.0, 4, true);
    canopy(&mut b.kit, 0.0, y0 - 1.0, 10.0, 3.0, 4.0);
    if b.p.parts.rooftop {
        let r = &mut b.r;
        parts::rooftop(&mut b.kit, mx * 0.8, (hy - my) * 0.4, h + 0.12, r);
    }
    pylon(b, Vec3::new(-hx + 2.0, -hy + 1.5, 0.0), 10.0, 5.0);
}

fn hotel(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // Two-storey podium with a porte-cochère, a slab tower and a crown sign.
    let floors = floors.clamp(4, 60);
    let (pod, fh) = (8.0, 3.1);
    flat_box(&mut b.kit, -hx, -hy, hx, hy, 0.0, pod, ids::FACADE, 0.6);
    storefront(b, 0.0, hx * 1.2, -hy, 0.2, 4.6, true);
    for s in [-1.0, 1.0] {
        storefront(b, s * hx * 0.8, hx * 0.3, -hy, 0.2, 4.6, false);
    }
    canopy(&mut b.kit, 0.0, -hy, hx * 0.9, 5.0, 4.8);
    let (tx, ty) = (hx * 0.85, hy * 0.42);
    let wall = if b.r.gen_bool(0.5) { ids::PLASTER } else { ids::STUCCO };
    super::houses::framed(b, Mat4::from_translation(Vec3::new(0.0, hy * 0.25, 0.0)), |b| {
        massing(b, tx, ty, pod, floors - 2, fh, wall, 2.0, 1.4, 1.7, 0.55);
        let top = pod + fh * (floors - 2) as f32;
        if b.p.parts.balconies {
            sides(&mut b.kit, tx, ty, |k, side, w| {
                if side % 2 == 0 {
                    let n = ((w / 4.0) as usize).max(1);
                    for f in 0..floors - 2 {
                        for i in 0..n {
                            parts::balcony(k, -w * 0.5 + w * (i as f32 + 0.5) / n as f32, pod + f as f32 * fh + 0.12, 2.6, 0.9);
                        }
                    }
                }
            });
        }
        b.kit.mat(ids::TRIM).block(Vec3::new(-tx - 0.2, -ty - 0.2, top), Vec3::new(tx + 0.2, ty + 0.2, top + 1.6));
        if b.p.parts.signs {
            let g = b.glow(true);
            b.kit.mat(g).block(Vec3::new(-tx * 0.6, -ty - 0.25, top + 0.3), Vec3::new(tx * 0.6, -ty - 0.2, top + 1.3));
        }
    });
}

fn motel(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    // A strip of rooms with outside walkways, a lobby, parking and a sign.
    let levels = floors.clamp(1, 3);
    let (fh, y0, y1) = (3.0, -hy + 7.0, hy);
    parking(b, Vec2::new(-hx, -hy), Vec2::new(hx, y0 - 2.2), 2.8, 0.4);
    let wall = [ids::STUCCO, ids::PLASTER, ids::BRICK][b.r.gen_range(0..3)];
    flat_box(&mut b.kit, -hx + 6.0, y0, hx, y1, 0.0, fh * levels as f32, wall, 0.4);
    let n = (((2.0 * hx - 6.0) / 4.0) as usize).max(2);
    for l in 0..levels {
        let z = l as f32 * fh;
        // Walkway slab with a railing (ground floor: a pavement).
        b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx + 6.0, y0 - 2.0, z), Vec3::new(hx, y0, z + 0.15));
        if l > 0 {
            b.kit.mat(ids::IRON).block(Vec3::new(-hx + 6.0, y0 - 2.0, z + 0.95), Vec3::new(hx, y0 - 1.95, z + 1.0));
        }
        for i in 0..n {
            let x = -hx + 6.0 + (2.0 * hx - 6.0) * (i as f32 + 0.5) / n as f32;
            super::houses::door(&mut b.kit, x - 0.8, y0 - 0.02, z + 0.15, 0.9, 2.1);
            let pane = b.pane(0.4);
            window(&mut b.kit, x + 0.7, y0 - 0.02, z + 1.0, 1.2, 1.1, pane);
        }
    }
    let top = fh * levels as f32;
    b.kit.mat(ids::ROOF).block(Vec3::new(-hx + 6.0, y0 - 2.2, top - 0.05), Vec3::new(hx, y0, top + 0.2));
    b.kit.mat(ids::IRON);
    for i in 0..=n {
        let x = -hx + 6.0 + (2.0 * hx - 6.0) * i as f32 / n as f32;
        b.kit.block(Vec3::new(x - 0.08, y0 - 2.0, 0.0), Vec3::new(x + 0.08, y0 - 1.84, top));
    }
    // Lobby with an office window.
    flat_box(&mut b.kit, -hx, y0 - 1.0, -hx + 5.8, y1 - 4.0, 0.0, 3.6, ids::PLASTER, 0.4);
    storefront(b, -hx + 2.9, 4.0, y0 - 1.0, 0.1, 2.5, true);
    pylon(b, Vec3::new(-hx + 1.2, -hy + 1.0, 0.0), 7.5, 3.6);
}

fn bank(b: &mut Ctx, hx: f32, hy: f32) {
    // Classical bank: steps, a portico of columns, a pediment, tall windows.
    let (base, h) = (1.4, 8.0);
    b.kit.mat(ids::CURB).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, hy, base));
    let y0 = -hy + 4.0;
    flat_box(&mut b.kit, -hx, y0, hx, hy, base, h, ids::PLASTER, 1.0);
    // Steps climb from the pavement up to the plinth at the front edge.
    let kind = if b.p.stairs == super::StairKind::Spiral { super::StairKind::Straight } else { b.p.stairs };
    b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hy - 0.28 * (base / 0.18).round(), 0.0)), |k| {
        super::arch::stairs(k, kind, hx * 1.2, base);
    });
    b.kit.mat(ids::PLASTER);
    let n = ((hx * 2.0 / 2.4) as usize).clamp(4, 8);
    for i in 0..n {
        let x = -hx + 0.8 + (2.0 * hx - 1.6) * i as f32 / (n - 1) as f32;
        b.kit.frustum(Vec3::new(x, -hy + 1.2, base), 0.42, 0.36, h - 1.2, 12, true);
        b.kit.block(Vec3::new(x - 0.55, -hy + 0.65, base + h - 1.2), Vec3::new(x + 0.55, -hy + 1.75, base + h - 0.9));
    }
    b.kit.mat(ids::TRIM).block(Vec3::new(-hx - 0.2, -hy + 0.4, base + h - 0.9), Vec3::new(hx + 0.2, y0, base + h));
    // Pediment over the portico.
    b.kit.at(Mat4::from_translation(Vec3::new(0.0, (-hy + 0.4 + y0) * 0.5, base + h)), |k| {
        k.at(Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2), |k| k.mat(ids::ROOF).gable((y0 + hy - 0.4) * 0.5, hx + 0.2, 2.2, 0.0, ids::PLASTER))
    });
    for i in 0..3 {
        let x = (i as f32 - 1.0) * hx * 0.5;
        let pane = b.pane(0.5);
        window(&mut b.kit, x, y0 - 0.02, base + 0.4, if i == 1 { 2.0 } else { 1.6 }, 5.2, pane);
    }
    super::houses::door(&mut b.kit, 0.0, y0 - 0.03, base, 2.0, 3.6);
}

fn cinema(b: &mut Ctx, hx: f32, hy: f32) {
    // A tall auditorium box behind a lobby with a glowing marquee and a
    // vertical blade sign.
    let (lobby_h, aud_h, y0) = (5.5, 13.0, -hy + 2.5);
    b.kit.mat(ids::SIDEWALK).block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, y0, 0.12));
    flat_box(&mut b.kit, -hx, y0, hx, y0 + 8.0, 0.0, lobby_h, ids::STUCCO, 0.6);
    flat_box(&mut b.kit, -hx + 1.0, y0 + 8.0, hx - 1.0, hy, 0.0, aud_h, ids::FACADE, 0.5);
    storefront(b, 0.0, hx * 1.2, y0, 0.12, 3.2, true);
    let g = b.glow(true);
    // Marquee: a deep canopy with glowing edges.
    b.kit.mat(ids::TRIM).block(Vec3::new(-hx * 0.75, y0 - 2.2, 3.6), Vec3::new(hx * 0.75, y0, 4.6));
    if b.p.parts.signs {
        b.kit.mat(g).block(Vec3::new(-hx * 0.73, y0 - 2.22, 3.75), Vec3::new(hx * 0.73, y0 - 2.2, 4.45));
        for s in [-1.0, 1.0] {
            b.kit.mat(g).block(Vec3::new(s * hx * 0.75 - 0.02, y0 - 2.2, 3.75), Vec3::new(s * hx * 0.75 + 0.02, y0, 4.45));
        }
        let x = hx * 0.55;
        b.kit.mat(ids::TRIM).block(Vec3::new(x - 0.3, y0 - 1.4, lobby_h - 0.5), Vec3::new(x + 0.3, y0, aud_h + 2.0));
        b.kit.mat(g).block(Vec3::new(x - 0.32, y0 - 1.3, lobby_h), Vec3::new(x + 0.32, y0 - 0.1, aud_h + 1.6));
        // Poster cases either side of the doors.
        for s in [-1.0, 1.0] {
            b.kit.mat(g).block(Vec3::new(s * hx * 0.85 - 0.6, y0 - 0.1, 0.8), Vec3::new(s * hx * 0.85 + 0.6, y0 - 0.02, 2.6));
        }
    }
    planters(&mut b.kit, -hx + 0.6, hx - 0.6, -hy + 0.5, 3);
}
