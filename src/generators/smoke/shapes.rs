//! Plume geometry: noise-displaced lobe shells up the column, the
//! mushroom-cap ring vortex, and soft alpha-eroded turbulence cards.
//! Colour and density go in the vertex colours (alpha = density).

use super::column::Column;
use super::{SmokeIds, SmokeParams};
use crate::generators::rgba;
use crate::materials::patterns::fbm;
use crate::mesh::EditablePoly;
use bevy::math::{Quat, Vec2, Vec3};
use rand::Rng;
use std::f32::consts::{PI, TAU};

const LON: usize = 10;
const LAT: usize = 6;

/// Fire glow colour at the base of a plume.
const FIRE: Vec3 = Vec3::new(1.0, 0.42, 0.08);

struct Ctx<'a> {
    p: &'a SmokeParams,
    ids: SmokeIds,
    /// Share of the height that burns (0 = no fire).
    fire_zone: f32,
    noise_off: Vec3,
}

impl Ctx<'_> {
    /// Vertex colour of smoke at normalised height `s` with lobe shading
    /// `shade` (0 underside .. 1 top) and noise `n`.
    fn smoke_color(&self, s: f32, shade: f32, n: f32) -> bevy::math::Vec4 {
        let base = self.p.style.color();
        let mut c = base * (0.65 + 0.45 * shade) * (0.85 + 0.3 * n);
        // Orange under-glow from the fire just below.
        if self.fire_zone > 0.0 {
            let glow = 1.0 - crate::generators::smooth(self.fire_zone, self.fire_zone * 2.5, s);
            c = c.lerp(FIRE * 0.7, glow * (1.0 - shade) * 0.8);
        }
        // Thick near the source, thinning as it spreads.
        let a = self.p.style.density() * (1.0 - 0.6 * s) * (0.7 + 0.3 * n);
        rgba(c, a)
    }
}

/// One displaced lobe (lat-long shell) centred at `c`, radius `r`, flattened
/// vertically by `squash`, at plume height `s`.
fn lobe(m: &mut EditablePoly, cx: &Ctx, c: Vec3, r: f32, squash: f32, s: f32, rot: Quat) {
    let turb = cx.p.turbulence.clamp(0.0, 1.0);
    let burning = s < cx.fire_zone;
    let id = if burning { cx.ids.fire } else { cx.ids.smoke };
    let mut put = |dir: Vec3| {
        let d = rot * dir;
        let q = c + d * r;
        let n = fbm(q * (1.6 / r.max(0.2)) + cx.noise_off);
        let k = 1.0 + turb * 0.55 * (n - 0.5) * 2.0;
        let v = m.add_vertex(c + Vec3::new(d.x, d.y, d.z * squash) * r * k);
        let shade = d.z * 0.5 + 0.5;
        m.vertices[v].color = if burning {
            let heat = 1.0 - s / cx.fire_zone.max(1e-3);
            rgba(FIRE.lerp(Vec3::new(1.0, 0.85, 0.4), heat * 0.6) * (0.8 + 0.2 * n), 1.0)
        } else {
            cx.smoke_color(s, shade, n)
        };
        v
    };
    let bottom = put(-Vec3::Z);
    let rings: Vec<Vec<_>> = (1..LAT)
        .map(|i| {
            let th = PI * i as f32 / LAT as f32;
            (0..LON)
                .map(|j| {
                    let ph = TAU * j as f32 / LON as f32;
                    put(Vec3::new(th.sin() * ph.cos(), th.sin() * ph.sin(), -th.cos()))
                })
                .collect()
        })
        .collect();
    let top = put(Vec3::Z);
    let uv = |i: usize, j: usize| Vec2::new(j as f32 / LON as f32, i as f32 / LAT as f32);
    let add = |m: &mut EditablePoly, vs: &[_], uvs: &[Vec2]| {
        let p = m.add_polygon_uv(vs, uvs);
        m.polygons[p].material_id = id;
    };
    for j in 0..LON {
        let jn = (j + 1) % LON;
        add(m, &[bottom, rings[0][jn], rings[0][j]], &[uv(0, j), uv(1, j + 1), uv(1, j)]);
        let t = rings.len() - 1;
        add(m, &[top, rings[t][j], rings[t][jn]], &[uv(LAT, j), uv(LAT - 1, j), uv(LAT - 1, j + 1)]);
        for i in 0..t {
            add(m, &[rings[i][j], rings[i][jn], rings[i + 1][jn], rings[i + 1][j]], &[uv(i + 1, j), uv(i + 1, j + 1), uv(i + 2, j + 1), uv(i + 2, j)]);
        }
    }
}

/// A soft card: a fan whose rim fades to zero alpha (alpha-eroded edge).
fn card(m: &mut EditablePoly, cx: &Ctx, c: Vec3, r: f32, s: f32, rot: Quat) {
    let k = 10;
    let n = fbm(c * 0.3 + cx.noise_off);
    let mut col = cx.smoke_color(s, 0.6, n);
    col.w *= 0.45;
    let centre = m.add_vertex(c);
    m.vertices[centre].color = col;
    let rim: Vec<_> = (0..k)
        .map(|i| {
            let a = TAU * i as f32 / k as f32;
            let wobble = 0.75 + 0.5 * fbm(c + Vec3::new(a.cos(), a.sin(), 0.0) * 2.0 + cx.noise_off);
            let v = m.add_vertex(c + rot * Vec3::new(a.cos(), 0.0, a.sin()) * r * wobble);
            m.vertices[v].color = col.truncate().extend(0.0);
            v
        })
        .collect();
    for i in 0..k {
        let (a, b) = (rim[i], rim[(i + 1) % k]);
        let ua = Vec2::new(0.5 + 0.5 * (TAU * i as f32 / k as f32).cos(), 0.5 + 0.5 * (TAU * i as f32 / k as f32).sin());
        let ub = Vec2::new(0.5 + 0.5 * (TAU * (i + 1) as f32 / k as f32).cos(), 0.5 + 0.5 * (TAU * (i + 1) as f32 / k as f32).sin());
        let p = m.add_polygon_uv(&[centre, a, b], &[Vec2::splat(0.5), ua, ub]);
        m.polygons[p].material_id = cx.ids.wisp;
    }
}

/// The whole plume on the origin.
pub fn plume(p: &SmokeParams, ids: SmokeIds) -> EditablePoly {
    let col = Column::new(p);
    let mut r = crate::generators::kit::rng(p.seed, 0x5307E);
    let cx = Ctx {
        p,
        ids,
        fire_zone: 0.16 * (p.fire / 100.0).clamp(0.0, 1.0),
        noise_off: Vec3::new(r.gen_range(0.0..100.0), r.gen_range(0.0..100.0), r.gen_range(0.0..100.0)),
    };
    let mut m = EditablePoly::new();
    let n = p.lobes.clamp(3, 64) as usize;
    let mut puffs = Vec::with_capacity(n);
    for i in 0..n {
        // Puffs rise with the phase and slow toward the top (z ~ t^(3/4)).
        let t = ((i as f32 + 0.35 + r.gen_range(-0.25..0.25)) / n as f32 + col.phase).rem_euclid(1.0);
        puffs.push(Column::height_at(t).min(0.97));
    }
    puffs.sort_by(f32::total_cmp);
    for &s in &puffs {
        let jitter = Vec3::new(r.gen_range(-1.0..1.0), r.gen_range(-1.0..1.0), r.gen_range(-0.5..0.5)) * col.radius(s) * 0.25 * col.turbulence;
        let rad = col.radius(s) * r.gen_range(0.75..1.1) * if s < 0.06 { 0.8 } else { 1.0 };
        let rot = Quat::from_rotation_arc(Vec3::Z, col.tangent(s)) * Quat::from_rotation_z(r.gen_range(0.0..TAU));
        lobe(&mut m, &cx, col.centre(s) + jitter, rad, r.gen_range(0.75..0.95), s, rot);
        if p.cards && r.gen_bool((0.3 + 0.6 * col.turbulence) as f64) {
            let yaw = Quat::from_rotation_z(r.gen_range(0.0..TAU));
            card(&mut m, &cx, col.centre(s) + jitter * 1.5, rad * 1.7, s, yaw);
            card(&mut m, &cx, col.centre(s) + jitter * 1.5, rad * 1.5, s, yaw * Quat::from_rotation_z(PI / 2.0));
        }
    }
    // Flame tongues licking up around the source (Fire Base Intensity).
    let flames = (p.fire.clamp(0.0, 100.0) / 100.0 * 9.0).ceil() as usize;
    let fz = (cx.fire_zone * 0.5).max(1e-3);
    for _ in 0..flames {
        let off = Vec2::from_angle(r.gen_range(0.0..TAU)) * col.r0 * r.gen_range(0.0..0.8);
        let size = col.r0 * r.gen_range(0.35..0.65);
        let c = off.extend(size * r.gen_range(0.6..1.6));
        lobe(&mut m, &cx, c, size, r.gen_range(1.6..2.4), fz * r.gen_range(0.0..0.9), Quat::from_rotation_z(r.gen_range(0.0..TAU)));
    }
    if p.mushroom {
        // Ring vortex: a cap puff with a torus of lobes curling down around it.
        let (top, r1) = (col.centre(1.0), col.radius(1.0));
        lobe(&mut m, &cx, top, r1 * 1.05, 0.6, 1.0, Quat::IDENTITY);
        let k = 7;
        for ring in 0..2 {
            let (rad, drop, size) = if ring == 0 { (1.05, 0.25, 0.55) } else { (1.3, 0.7, 0.35) };
            for i in 0..k {
                let a = TAU * (i as f32 + 0.5 * ring as f32) / k as f32 + r.gen_range(-0.2..0.2);
                let c = top + Vec3::new(a.cos(), a.sin(), 0.0) * r1 * rad - Vec3::Z * r1 * drop;
                lobe(&mut m, &cx, c, r1 * size * r.gen_range(0.85..1.15), 0.8, 0.98, Quat::from_rotation_z(a));
            }
        }
    }
    m.finalize();
    m
}
