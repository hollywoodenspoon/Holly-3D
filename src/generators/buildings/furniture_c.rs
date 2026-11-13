//! Furniture, part three: bedroom and storage. Single, double and bunk
//! beds, wardrobe, dresser, nightstand, bookshelf, TV stand and a glass
//! display cabinet. Floor at z = 0, front facing -Y, backs against +Y.

use super::furniture::{cushion, fabric, legs};
use super::{BuildingKind as K, Ctx};
use crate::generators::kit::{Kit, ids};
use bevy::math::Vec3;
use rand::Rng;

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32) {
    match b.p.kind {
        K::SingleBed | K::DoubleBed => bed(b, hx, hy),
        K::BunkBed => bunk(b, hx, hy),
        K::Wardrobe => wardrobe(b, hx, hy),
        K::Dresser => drawers(b, hx, hy, 0.85, 3),
        K::Nightstand => drawers(b, hx, hy, 0.55, 2),
        K::Bookshelf => bookshelf(b, hx, hy),
        K::TvStand => tv_stand(b, hx, hy),
        K::DisplayCabinet => display(b, hx, hy),
        _ => {}
    }
}

/// A seeded carcass finish: wood, white or dark.
fn carcass(b: &mut Ctx) -> u16 {
    [ids::TRIM, ids::TRIM, ids::PLASTER, ids::SLATE][b.r.gen_range(0..4)]
}

/// A drawer front with a handle, on the -Y face at `y`.
fn drawer(k: &mut Kit, x0: f32, x1: f32, y: f32, z0: f32, z1: f32, front: u16) {
    k.mat(front);
    cushion(k, Vec3::new(x0, y - 0.015, z0), Vec3::new(x1, y, z1));
    let c = (x0 + x1) * 0.5;
    let w = ((x1 - x0) * 0.15).min(0.08);
    k.mat(ids::IRON).block(Vec3::new(c - w, y - 0.03, (z0 + z1) * 0.5 - 0.01), Vec3::new(c + w, y - 0.015, (z0 + z1) * 0.5 + 0.01));
}

/// A mattress, pillows and a turned-down duvet on a frame top at `z`.
fn bedding(b: &mut Ctx, x0: f32, x1: f32, y0: f32, y1: f32, z: f32) {
    let duvet = fabric(b);
    let k = &mut b.kit;
    k.mat(ids::PLASTER);
    cushion(k, Vec3::new(x0, y0, z), Vec3::new(x1, y1, z + 0.2));
    let pillows = if x1 - x0 > 1.2 { 2 } else { 1 };
    let pw = (x1 - x0 - 0.1) / pillows as f32;
    for i in 0..pillows {
        let px = x0 + 0.05 + i as f32 * pw;
        cushion(k, Vec3::new(px + 0.03, y1 - 0.45, z + 0.2), Vec3::new(px + pw - 0.03, y1 - 0.08, z + 0.32));
    }
    k.mat(duvet);
    cushion(k, Vec3::new(x0 - 0.02, y0 - 0.02, z + 0.12), Vec3::new(x1 + 0.02, y1 - 0.6, z + 0.27));
    k.mat(ids::PLASTER).block(Vec3::new(x0 - 0.01, y1 - 0.75, z + 0.2), Vec3::new(x1 + 0.01, y1 - 0.6, z + 0.28));
}

fn bed(b: &mut Ctx, hx: f32, hy: f32) {
    // A bed frame with a headboard at +Y, a low footboard, mattress,
    // pillows and a duvet.
    let wood = carcass(b);
    let k = &mut b.kit;
    k.mat(wood);
    legs(k, hx, hy, 0.04, 0.3, 0.035);
    k.block(Vec3::new(-hx, -hy, 0.15), Vec3::new(hx, hy, 0.32));
    k.mat(wood);
    cushion(k, Vec3::new(-hx, hy - 0.08, 0.0), Vec3::new(hx, hy, 1.0));
    k.block(Vec3::new(-hx, -hy, 0.0), Vec3::new(hx, -hy + 0.05, 0.45));
    bedding(b, -hx + 0.04, hx - 0.04, -hy + 0.06, hy - 0.08, 0.32);
}

fn bunk(b: &mut Ctx, hx: f32, hy: f32) {
    // Two stacked single beds on corner posts, a guard rail on the top
    // bunk and a ladder at the foot.
    let wood = carcass(b);
    let k = &mut b.kit;
    k.mat(wood);
    legs(k, hx, hy, 0.04, 1.75, 0.04);
    for z in [0.2, 1.1] {
        k.block(Vec3::new(-hx, -hy, z), Vec3::new(hx, hy, z + 0.12));
    }
    for y in [-hy + 0.04, hy - 0.04] {
        for z in [0.6, 1.5, 1.72] {
            k.block(Vec3::new(-hx + 0.04, y - 0.02, z - 0.04), Vec3::new(hx - 0.04, y + 0.02, z));
        }
    }
    k.block(Vec3::new(-hx + 0.04, -hy + 0.04, 1.5), Vec3::new(-hx + 0.08, hy * 0.4, 1.56));
    k.block(Vec3::new(-hx + 0.04, -hy + 0.04, 1.66), Vec3::new(-hx + 0.08, hy * 0.4, 1.72));
    // Ladder on the right side at the foot.
    let (x, y0, y1) = (hx + 0.02, -hy + 0.1, -hy + 0.5);
    for y in [y0, y1] {
        k.block(Vec3::new(x, y - 0.02, 0.0), Vec3::new(x + 0.04, y + 0.02, 1.7));
    }
    for i in 1..6 {
        let z = i as f32 * 0.28;
        k.block(Vec3::new(x, y0, z - 0.015), Vec3::new(x + 0.04, y1, z + 0.015));
    }
    bedding(b, -hx + 0.05, hx - 0.05, -hy + 0.08, hy - 0.08, 0.32);
    bedding(b, -hx + 0.05, hx - 0.05, -hy + 0.08, hy - 0.08, 1.22);
}

fn wardrobe(b: &mut Ctx, hx: f32, hy: f32) {
    // A tall two- or three-door wardrobe on a plinth with a cornice, a
    // mirror on one door, and a drawer under the doors.
    let wood = carcass(b);
    let doors = if hx > 0.6 { 3 } else { 2 };
    let mirror = b.r.gen_range(0..doors);
    let k = &mut b.kit;
    let h = 2.0;
    k.mat(wood).block(Vec3::new(-hx + 0.03, -hy + 0.03, 0.0), Vec3::new(hx - 0.03, hy, 0.08));
    k.block(Vec3::new(-hx, -hy + 0.02, 0.08), Vec3::new(hx, hy, h));
    k.block(Vec3::new(-hx - 0.02, -hy - 0.01, h), Vec3::new(hx + 0.02, hy, h + 0.05));
    let fy = -hy + 0.02;
    drawer(k, -hx + 0.03, hx - 0.03, fy, 0.1, 0.3, wood);
    let dw = (hx * 2.0 - 0.06) / doors as f32;
    for i in 0..doors {
        let x0 = -hx + 0.03 + i as f32 * dw;
        k.mat(wood);
        cushion(k, Vec3::new(x0 + 0.005, fy - 0.02, 0.33), Vec3::new(x0 + dw - 0.005, fy, h - 0.03));
        if i == mirror {
            k.mat(ids::GLASS).block(Vec3::new(x0 + 0.08, fy - 0.025, 0.5), Vec3::new(x0 + dw - 0.08, fy - 0.02, h - 0.2));
        }
        let hxp = if i % 2 == 0 { x0 + dw - 0.05 } else { x0 + 0.05 };
        k.mat(ids::IRON).block(Vec3::new(hxp - 0.01, fy - 0.045, 1.0), Vec3::new(hxp + 0.01, fy - 0.02, 1.25));
    }
}

fn drawers(b: &mut Ctx, hx: f32, hy: f32, h: f32, rows: u32) {
    // A chest of drawers (dresser or nightstand) on short legs with a
    // few things on top: a mirror or a lamp and a book.
    let wood = carcass(b);
    let front = if b.r.gen_bool(0.3) { ids::STUCCO } else { wood };
    let shade = fabric(b);
    let k = &mut b.kit;
    k.mat(wood);
    legs(k, hx, hy, 0.03, 0.1, 0.02);
    k.block(Vec3::new(-hx, -hy + 0.02, 0.08), Vec3::new(hx, hy, h - 0.03));
    k.block(Vec3::new(-hx - 0.01, -hy, h - 0.03), Vec3::new(hx + 0.01, hy + 0.01, h));
    let cols = if hx > 0.5 { 2 } else { 1 };
    let (cw, rh) = ((hx * 2.0 - 0.06) / cols as f32, (h - 0.15) / rows as f32);
    for c in 0..cols {
        for r in 0..rows {
            let (x0, z0) = (-hx + 0.03 + c as f32 * cw, 0.1 + r as f32 * rh);
            drawer(k, x0 + 0.01, x0 + cw - 0.01, -hy + 0.02, z0 + 0.01, z0 + rh - 0.01, front);
        }
    }
    if b.p.kind == K::Dresser {
        k.mat(wood).block(Vec3::new(-hx * 0.6, hy - 0.06, h), Vec3::new(hx * 0.6, hy - 0.02, h + 0.7));
        k.mat(ids::GLASS).block(Vec3::new(-hx * 0.55, hy - 0.065, h + 0.05), Vec3::new(hx * 0.55, hy - 0.06, h + 0.65));
    } else {
        let l = Vec3::new(hx * 0.3, hy * 0.2, h);
        k.mat(ids::IRON).frustum(l, 0.06, 0.05, 0.02, 10, true);
        k.frustum(l + Vec3::Z * 0.02, 0.01, 0.01, 0.22, 6, true);
        k.mat(shade).frustum(l + Vec3::Z * 0.16, 0.12, 0.08, 0.15, 14, false);
        k.mat(ids::ROOF).block(Vec3::new(-hx * 0.7, -hy * 0.5, h), Vec3::new(-hx * 0.1, hy * 0.1, h + 0.03));
    }
}

/// A row of books standing on a shelf from `x0` to `x1` at height `z`.
fn books(b: &mut Ctx, x0: f32, x1: f32, y0: f32, y1: f32, z: f32, max_h: f32) {
    let colours = [ids::ROOF, ids::SLATE, ids::STUCCO, ids::PLASTER, ids::TRIM];
    let mut x = x0 + b.r.gen_range(0.0..0.1);
    while x < x1 - 0.04 {
        let t = b.r.gen_range(0.025..0.05f32).min(x1 - x);
        let h = b.r.gen_range(0.6..0.95) * max_h;
        let d = b.r.gen_range(0.75..1.0) * (y1 - y0);
        let c = colours[b.r.gen_range(0..colours.len())];
        b.kit.mat(c).block(Vec3::new(x, y0, z), Vec3::new(x + t, y0 + d, z + h));
        x += t + 0.004;
        if b.r.gen_bool(0.08) {
            x += b.r.gen_range(0.08..0.2);
        }
    }
}

fn bookshelf(b: &mut Ctx, hx: f32, hy: f32) {
    // An open bookcase: sides, top, back, shelves and seeded books.
    let wood = carcass(b);
    let h = 1.9;
    let shelves = 5;
    let k = &mut b.kit;
    k.mat(wood);
    for x in [-hx, hx - 0.025] {
        k.block(Vec3::new(x, -hy, 0.0), Vec3::new(x + 0.025, hy, h));
    }
    k.block(Vec3::new(-hx, hy - 0.01, 0.0), Vec3::new(hx, hy, h));
    let gap = (h - 0.1) / shelves as f32;
    for i in 0..=shelves {
        let z = 0.06 + i as f32 * gap;
        k.mat(wood).block(Vec3::new(-hx + 0.025, -hy, z - 0.025), Vec3::new(hx - 0.025, hy - 0.01, z));
    }
    k.block(Vec3::new(-hx + 0.025, -hy, 0.0), Vec3::new(hx - 0.025, -hy + 0.02, 0.06));
    for i in 0..shelves {
        let z = 0.06 + i as f32 * gap;
        books(b, -hx + 0.03, hx - 0.03, -hy + 0.02, hy - 0.02, z, gap - 0.06);
    }
}

fn tv_stand(b: &mut Ctx, hx: f32, hy: f32) {
    // A low media unit with open shelves and doors, a flat TV on a stand
    // and a sound bar.
    let wood = carcass(b);
    let k = &mut b.kit;
    let h = 0.5;
    k.mat(wood);
    legs(k, hx, hy, 0.04, 0.08, 0.02);
    k.block(Vec3::new(-hx, -hy, 0.08), Vec3::new(hx, hy, h));
    let third = hx * 2.0 / 3.0;
    drawer(k, -hx + 0.02, -hx + third - 0.01, -hy, 0.1, h - 0.02, wood);
    drawer(k, hx - third + 0.01, hx - 0.02, -hy, 0.1, h - 0.02, wood);
    k.mat(ids::ASPHALT).block(Vec3::new(-third * 0.5 + 0.02, -hy - 0.001, 0.12), Vec3::new(third * 0.5 - 0.02, -hy + 0.02, h - 0.04));
    k.mat(ids::IRON).block(Vec3::new(-third * 0.4, -hy - 0.005, 0.15), Vec3::new(third * 0.4, -hy + 0.2, 0.22));
    let tw = (hx * 0.8).min(0.75);
    let th = tw * 0.58;
    k.mat(ids::ASPHALT).block(Vec3::new(-0.15, -0.08, h), Vec3::new(0.15, 0.08, h + 0.02));
    k.block(Vec3::new(-0.03, 0.0, h + 0.02), Vec3::new(0.03, 0.03, h + 0.1));
    k.block(Vec3::new(-tw, -0.02, h + 0.08), Vec3::new(tw, 0.02, h + 0.08 + th * 2.0));
    let glow = b.glow(false);
    let k = &mut b.kit;
    k.mat(glow).block(Vec3::new(-tw + 0.02, -0.021, h + 0.1), Vec3::new(tw - 0.02, -0.02, h + 0.06 + th * 2.0));
    k.mat(ids::ASPHALT).block(Vec3::new(-tw * 0.6, -hy + 0.05, h), Vec3::new(tw * 0.6, -hy + 0.13, h + 0.06));
}

fn display(b: &mut Ctx, hx: f32, hy: f32) {
    // A glass-fronted cabinet with glass shelves holding plates and
    // vases, on a cupboard base.
    let wood = carcass(b);
    let k = &mut b.kit;
    let h = 1.8;
    k.mat(wood);
    legs(k, hx, hy, 0.03, 0.08, 0.02);
    k.block(Vec3::new(-hx, -hy, 0.08), Vec3::new(hx, hy, 0.7));
    drawer(k, -hx + 0.02, -0.01, -hy, 0.12, 0.66, wood);
    drawer(k, 0.01, hx - 0.02, -hy, 0.12, 0.66, wood);
    for x in [-hx, hx - 0.03] {
        k.mat(wood).block(Vec3::new(x, -hy, 0.7), Vec3::new(x + 0.03, hy, h));
    }
    k.block(Vec3::new(-hx, hy - 0.02, 0.7), Vec3::new(hx, hy, h));
    k.block(Vec3::new(-hx - 0.01, -hy - 0.01, h), Vec3::new(hx + 0.01, hy, h + 0.04));
    for z in [1.05, 1.42] {
        k.mat(ids::GLASS).block(Vec3::new(-hx + 0.03, -hy + 0.03, z), Vec3::new(hx - 0.03, hy - 0.02, z + 0.01));
    }
    for (i, z) in [0.7f32, 1.06, 1.43].into_iter().enumerate() {
        let n = ((hx * 2.0 / 0.22) as usize).max(1);
        for j in 0..n {
            let x = -hx + 0.14 + j as f32 * (hx * 2.0 - 0.28) / (n.max(2) - 1) as f32;
            if (i + j) % 2 == 0 {
                k.mat(ids::PLASTER).frustum(Vec3::new(x, hy - 0.1, z), 0.09, 0.09, 0.01, 14, true);
                k.block(Vec3::new(x - 0.09, hy - 0.08, z), Vec3::new(x + 0.09, hy - 0.06, z + 0.18));
            } else {
                k.mat(ids::SLATE).frustum(Vec3::new(x, 0.0, z), 0.04, 0.06, 0.2, 10, true);
            }
        }
    }
    k.mat(ids::GLASS).block(Vec3::new(-hx + 0.03, -hy, 0.72), Vec3::new(hx - 0.03, -hy + 0.01, h - 0.02));
    k.mat(wood).block(Vec3::new(-0.01, -hy - 0.005, 0.72), Vec3::new(0.01, -hy + 0.01, h - 0.02));
}
