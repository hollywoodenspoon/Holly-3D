//! Ground damage: bomb craters with raised rims and giant mech footprints
//! (Modern), shallow impact pits, siege ruts and clutter (Medieval Siege),
//! steep sinkholes (Ancient). Each crater replaces the ground cells it
//! covers with a finer patch following its height profile, and the ground
//! around it (and buckled roads, in Modern mode) is displaced to match.

use super::DestructionMode as Mode;
use super::debris;
use super::field::{Field, wobble};
use crate::generators::kit::{Kit, ids, rng};
use crate::mesh::PolyId;
use bevy::math::Vec2;
use rand::Rng;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Bomb,
    /// Three-toed mech foot pointing along the angle.
    Footprint(f32),
    Pit,
    Sinkhole,
}

#[derive(Clone, Copy, Debug)]
pub struct Crater {
    pub c: Vec2,
    pub r: f32,
    pub depth: f32,
    pub kind: Kind,
}

/// Crater shape: below 0 in the bowl, a raised rim around it, 0 beyond
/// `1.6` radii.
fn bowl(u: f32, depth: f32, rim: f32) -> f32 {
    if u < 1.0 {
        -depth * (1.0 - u * u) + rim * u * u * u
    } else if u < 1.6 {
        rim * (1.0 - (u - 1.0) / 0.6).powi(2)
    } else {
        0.0
    }
}

impl Crater {
    /// Height offset of the ground at `q`.
    pub fn profile(&self, q: Vec2) -> f32 {
        let d = q - self.c;
        match self.kind {
            Kind::Bomb => bowl(d.length() / self.r, self.depth, self.depth * 0.35),
            Kind::Pit => bowl(d.length() / self.r, self.depth, self.depth * 0.12),
            Kind::Sinkhole => {
                let u = d.length() / self.r;
                if u < 1.0 { -self.depth * 2.0 * (1.0 - u.powi(6)) } else { 0.0 }
            }
            Kind::Footprint(a) => {
                let (fw, side) = (Vec2::new(a.cos(), a.sin()), Vec2::new(-a.sin(), a.cos()));
                let l = Vec2::new(d.dot(fw), d.dot(side)) / self.r;
                // Heel plus three toes: the nearest (normalised) ellipse wins.
                let heel = ((l - Vec2::new(-0.45, 0.0)) / Vec2::new(0.45, 0.38)).length();
                let toes = [-0.5f32, 0.0, 0.5].map(|t| {
                    let dir = Vec2::new(t.cos(), t.sin());
                    let q = l - dir * 0.55;
                    (Vec2::new(q.dot(dir), q.dot(dir.perp())) / Vec2::new(0.42, 0.15)).length()
                });
                let e = toes.into_iter().fold(heel, f32::min);
                bowl(e, self.depth * 0.8, self.depth * 0.3)
            }
        }
    }
    /// Radius beyond which the profile is flat.
    pub fn reach(&self) -> f32 {
        self.r * if matches!(self.kind, Kind::Footprint(_)) { 2.0 } else { 1.6 }
    }
}

pub fn craters(f: &Field) -> Vec<Crater> {
    let mut r = rng(f.p.seed, 0xC4A7);
    let s = f.s;
    let (min, max) = (f.bounds.min.truncate(), f.bounds.max.truncate());
    (0..f.p.crater_count as usize)
        .map(|i| {
            let c = match f.zones.get(i) {
                Some(z) => z.c.truncate() + Vec2::new(r.gen_range(-0.4..0.4), r.gen_range(-0.4..0.4)) * z.r,
                None => Vec2::new(r.gen_range(min.x..=max.x.max(min.x + 1e-3)), r.gen_range(min.y..=max.y.max(min.y + 1e-3))),
            };
            let rad = f.p.crater_radius.max(0.3) * r.gen_range(0.7..1.3);
            let depth = f.p.crater_depth.max(0.05) * (0.3 + 0.7 * s);
            let kind = match f.mode {
                Mode::Modern if i % 2 == 1 => Kind::Footprint(r.gen_range(0.0..std::f32::consts::TAU)),
                Mode::Modern => Kind::Bomb,
                Mode::MedievalSiege => Kind::Pit,
                Mode::AncientRuins => Kind::Sinkhole,
            };
            let rad = if kind == Kind::Pit { rad * 0.6 } else if kind == Kind::Sinkhole { rad * 0.7 } else { rad };
            Crater { c, r: rad, depth, kind }
        })
        .collect()
}

pub fn run(kit: &mut Kit, f: &Field) {
    let list = craters(f);
    let ground: Vec<PolyId> = f.elements.iter().filter(|e| e.ground).flat_map(|e| e.polys.iter().copied()).filter(|p| kit.m.polygons.contains_key(*p)).collect();
    let mut gverts = HashSet::new();
    for &p in &ground {
        gverts.extend(kit.m.polygons[p].vertex_ids());
    }
    let road: HashSet<_> = ground.iter().filter(|p| kit.m.polygons[**p].material_id == ids::ROAD).flat_map(|p| kit.m.polygons[*p].vertex_ids()).collect();
    for cr in &list {
        // Cells wholly inside the crater go; the patch replaces them.
        let reach = cr.reach();
        for &p in &ground {
            if kit.m.polygons.get(p).is_some_and(|poly| poly.vertex_ids().all(|v| kit.m.pos(v).truncate().distance(cr.c) < reach)) {
                kit.m.polygons.remove(p);
            }
        }
        let g = f.ground_at(cr.c);
        let cell = (cr.r / 6.0).clamp(0.15, 2.0);
        let ext = Vec2::splat(reach + cell);
        let seed = f.p.seed;
        kit.mat(ids::LOT);
        kit.ground(
            cr.c - ext,
            cr.c + ext,
            cell,
            |q| g + cr.profile(q) + wobble(seed, q * 3.0) * cr.depth * 0.06 + 0.02,
            |q| q.distance(cr.c) < reach + cell * 0.5,
        );
    }
    // Displace the remaining ground; buckle roads near the zones.
    for &v in &gverts {
        let Some(vx) = kit.m.vertices.get_mut(v) else { continue };
        let q = vx.position.truncate();
        let mut dz: f32 = list.iter().map(|c| c.profile(q)).sum();
        if f.mode == Mode::Modern && road.contains(&v) {
            dz += f.influence(vx.position) * 0.45 * wobble(f.p.seed ^ 0x55, q * 0.6);
        }
        vx.position.z += dz;
    }
    let mut r = rng(f.p.seed, 0x5E1);
    if f.mode == Mode::MedievalSiege {
        for z in &f.zones {
            ruts(kit, f, z.c.truncate(), z.r, &mut r);
            debris::siege_clutter(kit, &mut r, z.c, z.r * 0.6);
        }
    }
    if f.mode != Mode::AncientRuins {
        for cr in &list {
            let n = (f.p.rubble * 6.0) as usize;
            debris::rubble(kit, &mut r, cr.c.extend(f.ground_at(cr.c)), cr.r * 1.4, n, if f.mode == Mode::Modern { ids::FACADE } else { ids::LOT }, f.mode == Mode::Modern);
        }
    }
}

/// Two parallel wheel ruts across a siege zone.
fn ruts(kit: &mut Kit, f: &Field, c: Vec2, r: f32, rng: &mut rand_chacha::ChaCha8Rng) {
    let a = rng.gen_range(0.0..std::f32::consts::TAU);
    let dir = Vec2::new(a.cos(), a.sin());
    let side = dir.perp() * 0.8;
    kit.mat(ids::LOT);
    for s in [-1.0f32, 1.0] {
        let (p, q) = (c - dir * r * 1.5 + side * s, c + dir * r * 1.5 + side * s);
        kit.strip(p, q, 0.3, 1.5, |x| f.ground_at(x) + 0.03);
    }
}
