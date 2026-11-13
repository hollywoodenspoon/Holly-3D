//! Road cars: sedan, hatchback, SUV, pickup, van, sports car, taxi and
//! police car, each with three designs (the Design selector; stored in
//! `floors`). Generic shapes inspired by real body styles, no brands.

use super::vehicles::{Body, SEDAN, car};
use super::{BuildingKind as K, Ctx};
use crate::generators::kit::{Kit, ids};
use bevy::math::Vec3;
use rand::Rng;

impl super::BuildingKind {
    /// Named designs for kinds that have them (vehicles); empty otherwise.
    pub fn designs(self) -> &'static [&'static str] {
        match self {
            K::Sedan => &["Family Sedan", "Executive Saloon", "Compact Coupe"],
            K::Hatchback => &["City Hatch", "Hot Hatch", "Estate"],
            K::Suv => &["Compact Crossover", "Boxy Off-Roader", "Full-Size SUV"],
            K::Pickup => &["Single Cab", "Double Cab", "Flatbed Work Truck"],
            K::Van => &["Panel Van", "Minivan", "Camper Van"],
            K::SportsCar => &["Mid-Engine", "Grand Tourer", "Roadster"],
            K::Taxi => &["City Cab", "Tall Cab", "Minivan Cab"],
            K::PoliceCar => &["Patrol Sedan", "Patrol SUV", "Interceptor"],
            _ => &[],
        }
    }
}

const HATCH: Body = Body { wr: 0.31, axles: (-0.66, 0.68), ws: -0.4, roof: (-0.12, 0.66), back: 0.9, tail: 0.98, height: 1.48, ..SEDAN };
const CROSSOVER: Body = Body { wr: 0.36, sill: 0.3, nose: 0.85, belt: 1.05, tail: 1.08, ws: -0.36, roof: (-0.1, 0.8), back: 0.9, height: 1.62, ..SEDAN };
const BIG_SUV: Body = Body { wr: 0.4, sill: 0.34, nose: 0.98, belt: 1.12, tail: 1.14, ws: -0.4, roof: (-0.16, 0.9), back: 0.94, height: 1.86, ..SEDAN };
const OFFROAD: Body = Body { tumble: 0.05, ws: -0.46, roof: (-0.36, 0.94), back: 0.95, height: 1.88, ..BIG_SUV };
const CAB: Body = Body { wr: 0.4, sill: 0.36, nose: 1.0, belt: 1.12, tail: 1.12, ws: -0.32, roof: (-0.12, -0.02), back: 0.0, height: 1.85, doors: 2, ..SEDAN };
const VAN: Body = Body { wr: 0.36, sill: 0.3, nose: 0.95, belt: 1.05, tail: 1.05, ws: -0.62, roof: (-0.42, 0.97), back: 0.98, height: 2.3, tumble: 0.04, ..SEDAN };
const SPORT: Body = Body { wr: 0.34, axles: (-0.62, 0.64), sill: 0.12, nose: 0.5, belt: 0.78, tail: 0.86, ws: -0.22, roof: (0.0, 0.2), back: 0.58, height: 1.15, doors: 2, tumble: 0.16, ..SEDAN };

pub(super) fn build(b: &mut Ctx, hx: f32, hy: f32) {
    let d = b.p.floors.clamp(1, 3) as usize - 1;
    let paint = [ids::ROOF, ids::STUCCO, ids::IRON, ids::PLASTER, ids::SLATE, ids::ASPHALT][(b.r.gen_range(0..6) + d * 2) % 6];
    let (glow, neon) = (b.glow(false), b.glow(true));
    let k = &mut b.kit;
    match b.p.kind {
        K::Sedan => {
            let body = [SEDAN, Body { ws: -0.26, roof: (0.0, 0.42), back: 0.66, height: 1.47, ..SEDAN }, Body { doors: 2, ws: -0.3, roof: (-0.04, 0.3), back: 0.7, height: 1.36, ..SEDAN }][d];
            car(k, &body, hx, hy, paint, glow);
            if d == 1 {
                chrome_strip(k, &body, hx, hy);
            }
        }
        K::Hatchback => {
            let body = [HATCH, Body { doors: 2, height: 1.42, wr: 0.32, ..HATCH }, Body { roof: (-0.06, 0.86), back: 0.93, height: 1.47, ..SEDAN }][d];
            car(k, &body, hx, hy, paint, glow);
            match d {
                1 => spoiler(k, &body, hx, hy, ids::ASPHALT),
                2 => rails(k, &body, hx, hy),
                _ => {}
            }
        }
        K::Suv => {
            let body = [CROSSOVER, OFFROAD, BIG_SUV][d];
            car(k, &body, hx, hy, paint, glow);
            rails(k, &body, hx, hy);
            if d == 1 {
                wheel_on_back(k, &body, hy);
                k.mat(ids::ASPHALT).block(Vec3::new(-hx + 0.15, -hy - 0.12, body.sill), Vec3::new(hx - 0.15, -hy - 0.04, body.nose - 0.25));
            }
        }
        K::Pickup => {
            let body = [CAB, Body { doors: 4, roof: (-0.12, 0.16), back: 0.18, ..CAB }, CAB][d];
            car(k, &body, hx, hy, paint, glow);
            bed(k, &body, hx, hy, paint, d == 2);
            if d == 2 {
                k.mat(neon).block(Vec3::new(-0.15, body.roof.0 * hy + 0.1, body.height + 0.02), Vec3::new(0.15, body.roof.0 * hy + 0.25, body.height + 0.12));
            }
        }
        K::Van => {
            let body = [VAN, Body { height: 1.85, ws: -0.5, roof: (-0.25, 0.92), back: 0.95, nose: 0.85, belt: 1.0, tail: 1.0, doors: 3, ..VAN }, VAN][d];
            car(k, &body, hx, hy, paint, glow);
            if d != 1 {
                panel_sides(k, &body, hx, hy, paint);
            }
            if d == 2 {
                camper(k, &body, hx, hy);
            }
        }
        K::SportsCar => {
            let body = [SPORT, Body { ws: -0.12, nose: 0.6, belt: 0.82, tail: 0.88, roof: (0.1, 0.38), back: 0.72, height: 1.26, ..SPORT }, Body { open: true, ws: -0.05, belt: 0.8, ..SPORT }][d];
            car(k, &body, hx, hy, paint, glow);
            if d == 0 {
                spoiler(k, &body, hx, hy, paint);
                for s in [-1.0f32, 1.0] {
                    k.mat(ids::ASPHALT).block(Vec3::new(s * hx - 0.006, 0.2 * hy, body.belt - 0.24), Vec3::new(s * hx + 0.006, 0.4 * hy, body.belt - 0.1));
                }
            }
        }
        K::Taxi => {
            let body = [SEDAN, Body { height: 1.75, ws: -0.36, roof: (-0.15, 0.85), back: 0.92, nose: 0.82, belt: 1.0, tail: 1.02, wr: 0.34, ..SEDAN }, Body { height: 1.85, ws: -0.5, roof: (-0.25, 0.92), back: 0.95, nose: 0.85, belt: 1.0, tail: 1.0, doors: 3, ..VAN }][d];
            car(k, &body, hx, hy, ids::STUCCO, glow);
            chequer(k, &body, hx, hy);
            let y = (body.roof.0 + body.roof.1) * 0.5 * hy;
            k.mat(glow).block(Vec3::new(-0.3, y - 0.1, body.height + 0.025), Vec3::new(0.3, y + 0.1, body.height + 0.2));
        }
        _ => {
            // Police car: white body, dark doors band, light bar, push bar.
            let body = [SEDAN, CROSSOVER, Body { height: 1.38, nose: 0.66, belt: 0.88, sill: 0.16, ..SEDAN }][d];
            car(k, &body, hx, hy, ids::PLASTER, glow);
            band(k, &body, hx, hy);
            let y = (body.roof.0 + body.roof.1) * 0.5 * hy;
            k.mat(ids::ASPHALT).block(Vec3::new(-0.6, y - 0.12, body.height + 0.025), Vec3::new(0.6, y + 0.12, body.height + 0.06));
            k.mat(neon).block(Vec3::new(-0.58, y - 0.1, body.height + 0.06), Vec3::new(-0.05, y + 0.1, body.height + 0.14));
            k.mat(glow).block(Vec3::new(0.05, y - 0.1, body.height + 0.06), Vec3::new(0.58, y + 0.1, body.height + 0.14));
            if d == 1 {
                k.mat(ids::ASPHALT);
                for x in [-0.4, 0.4] {
                    k.block(Vec3::new(x - 0.04, -hy - 0.2, body.sill + 0.05), Vec3::new(x + 0.04, -hy - 0.04, body.nose - 0.05));
                }
                k.block(Vec3::new(-0.5, -hy - 0.22, body.nose - 0.15), Vec3::new(0.5, -hy - 0.16, body.nose - 0.08));
            }
        }
    }
}

fn chrome_strip(k: &mut Kit, b: &Body, hx: f32, hy: f32) {
    k.mat(ids::IRON);
    for s in [-1.0f32, 1.0] {
        k.block(Vec3::new(s * hx - 0.006, -hy + 0.3, b.belt - 0.04), Vec3::new(s * hx + 0.006, hy - 0.2, b.belt - 0.02));
    }
}

fn spoiler(k: &mut Kit, b: &Body, hx: f32, hy: f32, m: u16) {
    let (y, z) = if b.back > 0.8 { (b.roof.1 * hy, b.height) } else { (hy - 0.2, b.tail) };
    k.mat(m);
    for x in [-hx + 0.3, hx - 0.3] {
        k.block(Vec3::new(x - 0.03, y - 0.1, z), Vec3::new(x + 0.03, y, z + 0.12));
    }
    k.block(Vec3::new(-hx + 0.1, y - 0.2, z + 0.12), Vec3::new(hx - 0.1, y + 0.05, z + 0.15));
}

fn rails(k: &mut Kit, b: &Body, hx: f32, hy: f32) {
    k.mat(ids::ASPHALT);
    let x = hx - b.tumble - 0.08;
    for s in [-1.0f32, 1.0] {
        k.block(Vec3::new(s * x - 0.025, b.roof.0 * hy + 0.1, b.height + 0.025), Vec3::new(s * x + 0.025, b.roof.1 * hy - 0.05, b.height + 0.07));
    }
}

const fn paint_cover() -> u16 {
    ids::SLATE
}

fn wheel_on_back(k: &mut Kit, b: &Body, hy: f32) {
    // A spare on the tailgate, axle along Y.
    let m = bevy::math::Mat4::from_translation(Vec3::new(0.0, hy, b.belt + 0.05)) * bevy::math::Mat4::from_rotation_x(-std::f32::consts::FRAC_PI_2);
    k.at(m, |k| {
        k.mat(ids::ASPHALT).frustum(Vec3::ZERO, b.wr, b.wr, 0.22, 18, true);
        k.mat(paint_cover()).frustum(Vec3::Z * 0.22, b.wr * 0.9, b.wr * 0.85, 0.02, 18, true);
    });
}

fn bed(k: &mut Kit, b: &Body, hx: f32, hy: f32, paint: u16, flat: bool) {
    let y0 = b.back * hy + 0.06;
    if flat {
        k.mat(ids::TRIM).block(Vec3::new(-hx - 0.05, y0, b.belt), Vec3::new(hx + 0.05, hy + 0.05, b.belt + 0.08));
        k.mat(ids::IRON);
        k.block(Vec3::new(-hx, y0, b.belt + 0.08), Vec3::new(hx, y0 + 0.05, b.height - 0.1));
        for x in [-hx + 0.05, hx - 0.05] {
            k.block(Vec3::new(x - 0.025, y0 + 0.4, b.belt + 0.08), Vec3::new(x + 0.025, hy, b.belt + 0.14));
        }
        return;
    }
    k.mat(paint);
    let top = b.belt + 0.45;
    for s in [-1.0f32, 1.0] {
        k.block(Vec3::new(s * hx - 0.05 * (1.0 + s), y0, b.belt), Vec3::new(s * hx + 0.05 * (1.0 - s), hy, top));
    }
    k.block(Vec3::new(-hx, y0, b.belt), Vec3::new(hx, y0 + 0.06, top));
    k.block(Vec3::new(-hx, hy - 0.06, b.belt), Vec3::new(hx, hy, top));
    k.mat(ids::ASPHALT).block(Vec3::new(-hx + 0.1, y0 + 0.06, b.belt), Vec3::new(hx - 0.1, hy - 0.06, b.belt + 0.02));
}

fn panel_sides(k: &mut Kit, b: &Body, hx: f32, hy: f32, paint: u16) {
    let x = hx - b.tumble + 0.006;
    let y0 = b.ws * hy + (b.back * hy - b.ws * hy) * 0.45 + 0.08;
    k.mat(paint);
    for s in [-1.0f32, 1.0] {
        k.block(Vec3::new(s * x - 0.006, y0, b.belt), Vec3::new(s * x + 0.006, b.back * hy, b.height - 0.02));
    }
    k.block(Vec3::new(-x, hy * 0.98 - 0.01, b.belt), Vec3::new(x, hy * 0.98 + 0.012, b.height - 0.02));
}

fn camper(k: &mut Kit, b: &Body, hx: f32, hy: f32) {
    let x = hx - b.tumble;
    k.mat(ids::PLASTER).block(Vec3::new(-x + 0.05, b.roof.0 * hy + 0.3, b.height + 0.025), Vec3::new(x - 0.05, b.roof.1 * hy - 0.1, b.height + 0.3));
    k.mat(ids::GLASS).block(Vec3::new(x + 0.005, 0.1 * hy, b.belt + 0.3), Vec3::new(x + 0.015, 0.45 * hy, b.height - 0.4));
    k.mat(ids::SLATE).block(Vec3::new(x + 0.02, -0.1 * hy, b.height - 0.15), Vec3::new(x + 0.12, 0.6 * hy, b.height - 0.05));
}

fn chequer(k: &mut Kit, b: &Body, hx: f32, hy: f32) {
    let n = ((hy * 1.4) / 0.12) as usize;
    for s in [-1.0f32, 1.0] {
        for i in 0..n {
            let y = -hy * 0.7 + i as f32 * 0.12;
            k.mat(if i % 2 == 0 { ids::ASPHALT } else { ids::PLASTER });
            k.block(Vec3::new(s * hx - 0.007, y, b.belt - 0.14), Vec3::new(s * hx + 0.007, y + 0.12, b.belt - 0.06));
        }
    }
}

fn band(k: &mut Kit, b: &Body, hx: f32, hy: f32) {
    k.mat(ids::SLATE);
    for s in [-1.0f32, 1.0] {
        k.block(Vec3::new(s * hx - 0.007, b.ws * hy, b.sill + 0.25), Vec3::new(s * hx + 0.007, (b.back * hy).min(hy - 0.15), b.belt - 0.05));
    }
}
