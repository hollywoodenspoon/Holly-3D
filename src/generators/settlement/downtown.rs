//! Modern Downtown: a street grid with sidewalk curbs, crosswalks and
//! alleys, glass-and-steel towers on podium bases with recessed setbacks,
//! rooftop AC units and helipads, streetlamps and night-lit windows (ID 7).

use super::layout::{Lot, Plan, sides};
use crate::generators::kit::{Kit, ids};
use bevy::math::{Vec2, Vec3};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

/// Street-grid lines: positions of street centrelines across `lo..hi`.
pub(super) fn grid_lines(lo: f32, hi: f32, pitch: f32) -> Vec<f32> {
    let n = ((hi - lo) / pitch).floor().max(1.0) as usize;
    let used = n as f32 * pitch;
    let start = lo + (hi - lo - used) * 0.5;
    (0..=n).map(|i| start + i as f32 * pitch).collect()
}

pub fn build(plan: &mut Plan) {
    let p = plan.p.clone();
    let (sw, side) = (p.street_width.max(2.0), p.sidewalk_width.max(0.0));
    let pitch = p.block_size.max(12.0) + sw;
    let (xs, ys) = (grid_lines(plan.min.x, plan.max.x, pitch), grid_lines(plan.min.y, plan.max.y, pitch));
    plan.ground(8.0);
    // Avenues (full length along Y) and streets between them (along X).
    for &x in &xs {
        plan.road(Vec2::new(x, plan.min.y), Vec2::new(x, plan.max.y), sw, 0.03, ids::ROAD);
    }
    for &y in &ys {
        for w in xs.windows(2) {
            plan.road(Vec2::new(w[0] + sw * 0.5, y), Vec2::new(w[1] - sw * 0.5, y), sw, 0.03, ids::ROAD);
        }
    }
    crosswalks(plan, &xs, &ys, sw);
    let centre = (plan.min + plan.max) * 0.5;
    let reach = (plan.max - plan.min).length() * 0.5;
    for wx in xs.windows(2) {
        for wy in ys.windows(2) {
            let (lo, hi) = (Vec2::new(wx[0], wy[0]) + sw * 0.5, Vec2::new(wx[1], wy[1]) - sw * 0.5);
            if !plan.contains((lo + hi) * 0.5) {
                continue;
            }
            // Raised sidewalk slab, then an alley down the middle of the block.
            plan.kit.mat(ids::LOT).block(lo.extend(0.0), hi.extend(0.15));
            let core = (lo + side, hi - side);
            let mid = (core.0.x + core.1.x) * 0.5;
            plan.kit.mat(ids::ROAD).block(Vec3::new(mid - 1.5, core.0.y, 0.15), Vec3::new(mid + 1.5, core.1.y, 0.17));
            lamps(plan, lo, hi);
            for (a, b) in [(core.0.x, mid - 1.5), (mid + 1.5, core.1.x)] {
                let n = (((core.1.y - core.0.y) / 22.0).round() as usize).clamp(1, 4);
                let step = (core.1.y - core.0.y) / n as f32;
                for k in 0..n {
                    let c = Vec2::new((a + b) * 0.5, core.0.y + step * (k as f32 + 0.5));
                    // Front faces the avenue on the outer side of the half-block.
                    let yaw = if a < mid { -std::f32::consts::FRAC_PI_2 } else { std::f32::consts::FRAC_PI_2 };
                    let lot = Lot::new(c, Vec2::new(step * 0.5 - 0.6, (b - a) * 0.5 - 0.6), yaw);
                    if !plan.rng.gen_bool(p.density.clamp(0.0, 1.0) as f64) || !plan.claim(lot) {
                        continue;
                    }
                    // Taller towers towards the centre of downtown.
                    let core_t = (1.0 - c.distance(centre) / reach.max(1.0)).clamp(0.0, 1.0);
                    let t = (core_t.powf(1.3) * plan.rng.gen_range(0.55..1.0)).clamp(0.0, 1.0);
                    let floors = p.min_floors.max(1) + ((p.max_floors.max(p.min_floors) - p.min_floors.max(1)) as f32 * t).round() as u32;
                    plan.wrecked(lot, 0.15, false, |k, r| tower(k, lot.half, floors, r));
                }
            }
        }
    }
}

fn crosswalks(plan: &mut Plan, xs: &[f32], ys: &[f32], sw: f32) {
    plan.kit.mat(ids::LOT);
    for &x in xs {
        for &y in ys {
            if !plan.contains(Vec2::new(x, y)) {
                continue;
            }
            for k in 0..6 {
                let o = -sw * 0.4 + k as f32 * sw * 0.16;
                for s in [-1.0f32, 1.0] {
                    let (cx, cy) = (x + o, y + s * (sw * 0.5 + 1.6));
                    plan.kit.block(Vec3::new(cx - 0.3, cy - 1.4, 0.03), Vec3::new(cx + 0.3, cy + 1.4, 0.045));
                }
            }
        }
    }
}

fn lamps(plan: &mut Plan, lo: Vec2, hi: Vec2) {
    let d = plan.p.prop_density.clamp(0.0, 1.0);
    let corners = [lo + 0.8, Vec2::new(hi.x - 0.8, lo.y + 0.8), hi - 0.8, Vec2::new(lo.x + 0.8, hi.y - 0.8)];
    for (i, c) in corners.into_iter().enumerate() {
        if plan.rng.gen_bool(d as f64) {
            let out = (c - (lo + hi) * 0.5).normalize_or_zero();
            crate::generators::settlement::props::streetlamp(&mut plan.kit, c.extend(0.15), out);
        }
        if plan.rng.gen_bool((d * 0.6) as f64) {
            let next = corners[(i + 1) % 4];
            let at = c.lerp(next, 0.5);
            let (kit, r) = (&mut plan.kit, &mut plan.rng);
            super::props::tree(kit, at.extend(0.15), r.gen_range(6.0..9.0), false);
        }
    }
}

/// A glass-and-steel tower in its lot frame (front faces -Y).
pub(super) fn tower(kit: &mut Kit, half: Vec2, floors: u32, r: &mut ChaCha8Rng) {
    let fh = super::Era::Downtown.floor_height();
    let podium = if floors > 8 { r.gen_range(2..=3).min(floors) } else { 0 };
    let mut z = 0.0;
    let mut h = half;
    if podium > 0 {
        section(kit, h, z, podium, fh, 0.2, r, true);
        z += podium as f32 * fh;
        h *= 0.78;
    }
    let mut left = floors - podium;
    // Recessed upper setbacks every ~12 floors on tall towers.
    while left > 0 {
        let n = if floors > 20 && left > 14 { r.gen_range(10..=14) } else { left };
        section(kit, h, z, n, fh, r.gen_range(0.08..0.4), r, false);
        z += n as f32 * fh;
        left -= n;
        if left > 0 {
            h = (h * 0.86).max(Vec2::splat(3.0));
        }
    }
    // Roof parapet slab, AC units and a helipad on the tallest towers.
    kit.mat(ids::ROOF).block((-h).extend(z), h.extend(z + 0.6));
    kit.mat(ids::IRON);
    for _ in 0..r.gen_range(1..=4) {
        let c = Vec2::new(r.gen_range(-0.6..0.6) * h.x, r.gen_range(-0.6..0.6) * h.y);
        kit.block((c - 1.0).extend(z + 0.6), (c + 1.0).extend(z + 2.0));
    }
    if floors >= 25 && h.min_element() > 7.0 && r.gen_bool(0.6) {
        kit.mat(ids::FACADE).frustum(Vec3::Z * (z + 0.6), 6.0, 6.0, 0.35, 16, true);
        kit.mat(ids::LIT);
        let top = z + 0.97;
        for (x0, x1, y0, y1) in [(-2.0, -1.4, -2.5, 2.5), (1.4, 2.0, -2.5, 2.5), (-1.4, 1.4, -0.3, 0.3)] {
            kit.quad(Vec3::new(x0, y0, top), Vec3::new(x1, y0, top), Vec3::new(x1, y1, top), Vec3::new(x0, y1, top));
        }
    }
}

/// A stack of `n` floors: concrete core box plus window ribbons on four
/// sides, lit bays (ID 7) run-length merged per floor.
#[allow(clippy::too_many_arguments)]
fn section(kit: &mut Kit, h: Vec2, z0: f32, n: u32, fh: f32, lit: f64, r: &mut ChaCha8Rng, storefront: bool) {
    kit.mat(ids::FACADE).block((-h).extend(z0), h.extend(z0 + n as f32 * fh));
    sides(kit, h.x, h.y, |kit, _, w| {
        let bays = (w / 3.0).floor().max(1.0) as usize;
        let bw = w / bays as f32;
        for f in 0..n {
            let z = z0 + f as f32 * fh + fh * 0.2;
            let wh = if storefront && f == 0 { fh * 0.7 } else { fh * 0.6 };
            let states: Vec<bool> = (0..bays).map(|_| r.gen_bool(lit)).collect();
            let mut i = 0;
            while i < bays {
                let mut j = i;
                while j + 1 < bays && states[j + 1] == states[i] {
                    j += 1;
                }
                let (x0, x1) = (-w * 0.5 + i as f32 * bw + 0.15, -w * 0.5 + (j + 1) as f32 * bw - 0.15);
                super::window(kit, (x0 + x1) * 0.5, 0.0, z, x1 - x0, wh, if states[i] { ids::LIT } else { ids::GLASS });
                i = j + 1;
            }
        }
    });
}
