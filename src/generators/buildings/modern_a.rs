//! Modern multi-storey buildings: glass skyscraper, office block, brick
//! tenement, apartment block, parking garage, school and hospital.

use super::arch::{Opening, wall_with_openings, window_row};
use super::parts::{self, balcony, fire_escape};
use super::{BuildingKind as K, Ctx, massing};
use crate::generators::kit::{ids, sides};
use bevy::math::{Mat4, Vec3};
use rand::Rng;

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32, floors: u32) {
    let fh = 3.6;
    match b.p.kind {
        K::GlassSkyscraper => skyscraper(b, hx, hy, floors, fh),
        K::OfficeBlock => {
            podium(b, hx, hy, fh);
            massing(b, hx, hy, fh, floors.saturating_sub(1), fh, ids::FACADE, 2.4, 1.8, 2.0, 0.4);
            roof_clutter(b, hx, hy, fh * floors as f32);
        }
        K::BrickTenement => tenement(b, hx, hy, floors, 3.2),
        K::ApartmentBlock => apartments(b, hx, hy, floors, 3.0),
        K::ParkingGarage => garage(b, hx, hy, floors, 3.0),
        K::School => {
            low_block(b, hx, hy, floors, 3.8, ids::BRICK, 3.2);
            if b.p.parts.signs {
                let g = b.glow(false);
                b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hy - 0.03, 0.0)), |k| parts::sign(k, 0.0, 3.9 * floors as f32 - 1.0, hx.min(6.0), g));
            }
        }
        K::Hospital => {
            low_block(b, hx, hy, 1, 4.5, ids::PLASTER, 3.0);
            let (tx, ty) = (hx * 0.6, hy * 0.55);
            massing(b, tx, ty, 4.5, floors.saturating_sub(1), 3.6, ids::PLASTER, 2.2, 1.6, 1.6, 0.5);
            let top = 4.5 + 3.6 * floors.saturating_sub(1) as f32;
            // Helipad H on the roof, and a red cross sign.
            b.kit.mat(ids::LANE).block(Vec3::new(-0.3, -2.0, top), Vec3::new(0.3, 2.0, top + 0.03));
            b.kit.block(Vec3::new(-1.5, -0.25, top), Vec3::new(1.5, 0.25, top + 0.03));
            if b.p.parts.signs {
                let g = b.glow(true);
                b.kit.at(Mat4::from_translation(Vec3::new(0.0, -ty - 0.03, 0.0)), |k| parts::sign(k, 0.0, top - 1.5, 3.0, g));
            }
        }
        _ => {}
    }
}

fn skyscraper(b: &mut Ctx, hx: f32, hy: f32, floors: u32, fh: f32) {
    podium(b, hx, hy, fh * 1.5);
    let mut z = fh * 1.5;
    let (mut x, mut y) = (hx * 0.92, hy * 0.92);
    // Up to three setbacks, each a curtain wall of glass with steel mullions.
    let tiers = (floors / 12).clamp(1, 3);
    let per = floors.saturating_sub(1) / tiers;
    for t in 0..tiers {
        let n = if t + 1 == tiers { floors.saturating_sub(1) - per * t } else { per };
        let h = n as f32 * fh;
        b.kit.mat(ids::GLASS).block(Vec3::new(-x, -y, z), Vec3::new(x, y, z + h));
        let (r, em) = (&mut b.r, b.p.emissive);
        sides(&mut b.kit, x, y, |k, _, w| {
            let m = ((w / 1.6).round() as usize).max(2);
            k.mat(ids::IRON);
            for i in 0..=m {
                let px = -w * 0.5 + w * i as f32 / m as f32;
                k.block(Vec3::new(px - 0.05, -0.08, z), Vec3::new(px + 0.05, 0.0, z + h));
            }
            for f in 0..n {
                let fz = z + f as f32 * fh;
                k.mat(ids::IRON).block(Vec3::new(-w * 0.5, -0.1, fz), Vec3::new(w * 0.5, 0.0, fz + 0.25));
                if em && r.gen_bool(0.3) {
                    let i = r.gen_range(0..m);
                    let px = -w * 0.5 + w * (i as f32 + 0.5) / m as f32;
                    crate::generators::settlement::window(k, px, -0.11, fz + 0.4, w / m as f32 - 0.2, fh - 0.8, ids::LIT);
                }
            }
        });
        z += h;
        (x, y) = (x * 0.8, y * 0.8);
    }
    b.kit.mat(ids::IRON).block(Vec3::new(-x, -y, z), Vec3::new(x, y, z + 2.5));
    b.kit.tube(Vec3::new(0.0, 0.0, z + 2.5), Vec3::new(0.0, 0.0, z + 2.5 + fh * 3.0), 0.25, 6);
    if b.p.emissive {
        b.kit.mat(ids::NEON).cuboid(Vec3::new(0.0, 0.0, z + 2.6 + fh * 3.0), Vec3::splat(0.3), bevy::math::Quat::IDENTITY);
    }
    roof_clutter(b, x, y, z + 2.5);
}

/// Ground-floor podium with a glazed lobby, door and optional sign.
fn podium(b: &mut Ctx, hx: f32, hy: f32, h: f32) {
    let w = 2.0 * hx;
    let lit = b.glow(false);
    let mut ops = window_row(w, 3.0, 2.4, 0.4, h - 1.0, |_| ids::GLASS);
    let mid = ops.len() / 2;
    ops[mid] = Opening { at: ops[mid].at, width: 2.2, sill: 0.0, height: 2.6, fill: ids::GLASS };
    b.kit.at(Mat4::from_translation(Vec3::new(-hx, -hy, 0.0)), |k| {
        k.mat(ids::FACADE);
        wall_with_openings(k, w, 0.3, h, &ops);
    });
    b.kit.mat(ids::FACADE).block(Vec3::new(-hx, -hy + 0.3, 0.0), Vec3::new(hx, hy, h));
    if b.p.parts.awnings {
        b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hy, 0.0)), |k| k.mat(ids::IRON).block(Vec3::new(-2.0, -1.8, 3.0), Vec3::new(2.0, 0.0, 3.2)));
    }
    if b.p.parts.signs {
        b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hy - 0.03, 0.0)), |k| parts::sign(k, hx * 0.5, h - 0.9, hx * 0.6, lit));
    }
}

fn roof_clutter(b: &mut Ctx, hx: f32, hy: f32, z: f32) {
    b.kit.mat(ids::FACADE).block(Vec3::new(-hx, -hy, z), Vec3::new(hx, hy, z + 0.4));
    if b.p.parts.rooftop {
        parts::rooftop(&mut b.kit, hx, hy, z + 0.4, &mut b.r);
    }
}

fn tenement(b: &mut Ctx, hx: f32, hy: f32, floors: u32, fh: f32) {
    massing(b, hx, hy, 0.0, floors, fh, ids::BRICK, 2.2, 1.0, 1.6, 0.35);
    let top = fh * floors as f32;
    // Cornice, stoop, fire escape on the front.
    b.kit.mat(ids::TRIM).block(Vec3::new(-hx - 0.3, -hy - 0.3, top), Vec3::new(hx + 0.3, hy + 0.3, top + 0.5));
    let (p, st) = (b.p.parts, b.p.stairs);
    b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hy - 0.03, 0.0)), |k| {
        if p.fire_escapes {
            fire_escape(k, hx * 0.4, 2.4, floors, fh);
        }
        if p.porches {
            parts::porch(k, -hx * 0.5, 2.0, 1.4, 1.1, st);
        }
    });
    if p.rooftop {
        parts::rooftop(&mut b.kit, hx, hy, top + 0.5, &mut b.r);
    }
    if p.chimneys {
        parts::chimney(&mut b.kit, Vec3::new(hx * 0.7, hy * 0.6, top), 2.0);
    }
}

fn apartments(b: &mut Ctx, hx: f32, hy: f32, floors: u32, fh: f32) {
    massing(b, hx, hy, 0.0, floors, fh, ids::STUCCO, 3.0, 1.4, 1.5, 0.4);
    let top = fh * floors as f32;
    b.kit.mat(ids::FACADE).block(Vec3::new(-hx, -hy, top), Vec3::new(hx, hy, top + 0.3));
    let p = b.p.parts;
    if p.balconies {
        sides(&mut b.kit, hx, hy, |k, side, w| {
            if side % 2 == 1 {
                return;
            }
            let n = ((w / 6.0).floor() as usize).max(1);
            for f in 1..floors {
                for i in 0..n {
                    balcony(k, -w * 0.5 + w * (i as f32 + 0.5) / n as f32, f as f32 * fh, 2.8, 1.3);
                }
            }
        });
    }
    if p.porches {
        let st = b.p.stairs;
        b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hy - 0.03, 0.0)), |k| parts::porch(k, 0.0, 3.0, 2.0, 0.6, st));
    }
    if p.rooftop {
        parts::rooftop(&mut b.kit, hx, hy, top + 0.3, &mut b.r);
    }
}

fn garage(b: &mut Ctx, hx: f32, hy: f32, floors: u32, fh: f32) {
    // Open decks on columns with parapets, a ramp, and a stair core.
    for f in 0..=floors {
        let z = f as f32 * fh;
        b.kit.mat(ids::FACADE).block(Vec3::new(-hx, -hy, z), Vec3::new(hx, hy, z + 0.3));
        if f > 0 {
            b.kit.block(Vec3::new(-hx, -hy, z + 0.3), Vec3::new(hx, -hy + 0.2, z + 1.1));
            b.kit.block(Vec3::new(-hx, hy - 0.2, z + 0.3), Vec3::new(hx, hy, z + 1.1));
        }
        if f < floors {
            b.kit.mat(ids::FACADE);
            let n = ((2.0 * hx / 8.0).round() as usize).max(1);
            for i in 0..=n {
                for y in [-hy + 0.4, hy - 0.4] {
                    let x = -hx + 0.3 + (2.0 * hx - 0.6) * i as f32 / n as f32;
                    b.kit.block(Vec3::new(x - 0.3, y - 0.3, z + 0.3), Vec3::new(x + 0.3, y + 0.3, z + fh));
                }
            }
            b.kit.mat(ids::ASPHALT).beam(Vec3::new(-hx * 0.6, 0.0, z + 0.3), Vec3::new(hx * 0.2, 0.0, z + fh + 0.3), 3.0, 0.2);
        }
    }
    b.kit.mat(ids::LANE);
    for f in 1..=floors {
        let z = f as f32 * fh + 0.31;
        let n = ((2.0 * hx / 2.6) as usize).max(1);
        for i in 0..n {
            let x = -hx + 1.3 + 2.6 * i as f32;
            b.kit.block(Vec3::new(x - 0.05, hy * 0.3, z), Vec3::new(x + 0.05, hy - 0.6, z + 0.01));
        }
    }
    b.kit.mat(ids::FACADE).block(Vec3::new(hx - 4.0, -hy - 4.0, 0.0), Vec3::new(hx, -hy, fh * floors as f32 + 2.5));
}

/// A low institutional block with framed windows on its long faces.
fn low_block(b: &mut Ctx, hx: f32, hy: f32, floors: u32, fh: f32, wall: u16, bay: f32) {
    massing(b, hx, hy, 0.0, floors, fh, wall, bay, 2.0, 1.8, 0.3);
    let top = fh * floors as f32;
    b.kit.mat(ids::TRIM).block(Vec3::new(-hx - 0.2, -hy - 0.2, top), Vec3::new(hx + 0.2, hy + 0.2, top + 0.5));
    if b.p.parts.porches {
        let st = b.p.stairs;
        b.kit.at(Mat4::from_translation(Vec3::new(0.0, -hy - 0.03, 0.0)), |k| parts::porch(k, 0.0, 4.0, 2.4, 0.5, st));
    }
    if b.p.parts.rooftop {
        parts::rooftop(&mut b.kit, hx, hy, top + 0.5, &mut b.r);
    }
}
