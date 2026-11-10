//! What the destruction passes share: the input's elements (classified as
//! ground or structure), a coarse ground-height sampler, the blast zones
//! (user-placed or seeded on the structures) and, per mode, the influence
//! and cut-height fields (material above the cut is knocked down).

use super::{BattleParams, BlastZone, DestructionMode};
use crate::generators::kit::rng;
use crate::mesh::adjacency::Adjacency;
use crate::mesh::{Aabb, EditablePoly, PolyId};
use bevy::math::{Vec2, Vec3};
use rand::Rng;
use std::collections::HashMap;

/// A connected island of the input.
pub struct Element {
    pub polys: Vec<PolyId>,
    pub min: Vec3,
    pub max: Vec3,
    pub ground: bool,
    /// Material ID of its first polygon.
    pub mat: u16,
}

impl Element {
    pub fn size(&self) -> Vec3 {
        self.max - self.min
    }
    pub fn center(&self) -> Vec3 {
        (self.min + self.max) * 0.5
    }
    /// Columns, lamp posts, chimneys: much taller than wide.
    pub fn tall_thin(&self) -> bool {
        let s = self.size();
        let w = s.x.max(s.y);
        s.z > 3.0 * w && w < 1.6 && s.z > 1.5
    }
}

/// A resolved blast zone: centre on the ground, horizontal radius, and a
/// per-zone shear direction.
#[derive(Clone, Debug)]
pub struct Zone {
    pub c: Vec3,
    pub r: f32,
    pub tilt: Vec2,
}

pub struct Field {
    pub mode: DestructionMode,
    pub p: BattleParams,
    pub s: f32,
    pub bounds: Aabb,
    pub elements: Vec<Element>,
    pub zones: Vec<Zone>,
    cell: f32,
    heights: HashMap<(i32, i32), f32>,
}

impl Field {
    pub fn new(m: &EditablePoly, p: &BattleParams, user: &[BlastZone]) -> Option<Self> {
        let bounds = m.bounds()?;
        let adj = Adjacency::build(m);
        let span = (bounds.max - bounds.min).max(Vec3::splat(1e-3));
        let mut elements: Vec<Element> = adj
            .elements(m)
            .into_iter()
            .map(|polys| {
                let b = Aabb::from_points(polys.iter().flat_map(|&p| m.polygons[p].vertex_ids().map(|v| m.pos(v)))).expect("non-empty");
                let up = polys.iter().filter(|&&p| m.poly_normal(p).z > 0.7).count();
                let flat = b.max.z - b.min.z < 0.6 && b.min.z - bounds.min.z < span.z * 0.25 + 0.5;
                let ground = up * 20 >= polys.len() * 19 && (polys.len() > 30 || flat);
                Element { mat: m.polygons[polys[0]].material_id, polys, min: b.min, max: b.max, ground }
            })
            .collect();
        elements.sort_by(|a, b| a.min.x.total_cmp(&b.min.x).then(a.min.y.total_cmp(&b.min.y)));
        let cell = (span.x.max(span.y) / 48.0).clamp(1.0, 8.0);
        let mut heights: HashMap<(i32, i32), f32> = HashMap::new();
        for e in elements.iter().filter(|e| e.ground) {
            for v in e.polys.iter().flat_map(|&p| m.polygons[p].vertex_ids()) {
                let q = m.pos(v);
                let k = ((q.x / cell).floor() as i32, (q.y / cell).floor() as i32);
                let h = heights.entry(k).or_insert(f32::MIN);
                *h = h.max(q.z);
            }
        }
        let mut f = Self { mode: p.mode, p: p.clone(), s: p.strength(), bounds, elements, zones: vec![], cell, heights };
        f.zones = f.resolve_zones(user);
        Some(f)
    }

    /// Ground height under `q` (the input's bottom where there is no ground).
    pub fn ground_at(&self, q: Vec2) -> f32 {
        let k = ((q.x / self.cell).floor() as i32, (q.y / self.cell).floor() as i32);
        self.heights.get(&k).copied().unwrap_or(self.bounds.min.z)
    }

    fn resolve_zones(&self, user: &[BlastZone]) -> Vec<Zone> {
        let mut r = rng(self.p.seed, 0xB1A5);
        let tilt = |r: &mut rand_chacha::ChaCha8Rng| {
            let a = r.gen_range(0.0..std::f32::consts::TAU);
            Vec2::new(a.cos(), a.sin())
        };
        if !user.is_empty() {
            return user.iter().map(|z| Zone { c: z.center.truncate().extend(self.ground_at(z.center.truncate())), r: z.radius.max(0.1), tilt: tilt(&mut r) }).collect();
        }
        // Seeded zones centred on randomly chosen structures.
        let targets: Vec<Vec3> = self.elements.iter().filter(|e| !e.ground && e.size().max_element() > 0.5).map(|e| e.center()).collect();
        let span = self.bounds.max - self.bounds.min;
        let radius = (self.p.crater_radius * 2.5).max(1.0).min(span.x.max(span.y) * 0.6 + 1.0);
        (0..self.p.crater_count.clamp(1, 24))
            .map(|_| {
                let c = if targets.is_empty() {
                    Vec2::new(r.gen_range(self.bounds.min.x..=self.bounds.max.x), r.gen_range(self.bounds.min.y..=self.bounds.max.y))
                } else {
                    targets[r.gen_range(0..targets.len())].truncate()
                };
                Zone { c: c.extend(self.ground_at(c)), r: radius * r.gen_range(0.8..1.2), tilt: tilt(&mut r) }
            })
            .collect()
    }

    /// 0..=1: how hard the blast hits at `q` (scaled by the intensity).
    pub fn influence(&self, q: Vec3) -> f32 {
        let zone = self.zones.iter().map(|z| (1.0 - q.truncate().distance(z.c.truncate()) / z.r).clamp(0.0, 1.0)).fold(0.0f32, f32::max);
        let zone = zone * zone * (3.0 - 2.0 * zone);
        let ruin = if self.mode == DestructionMode::AncientRuins { 0.35 } else { 0.0 };
        (zone.max(ruin) * self.s).clamp(0.0, 1.0)
    }

    /// Height above which material at `q` is knocked down (infinite outside
    /// every zone).
    pub fn cut(&self, q: Vec3) -> f32 {
        let s = self.s.max(0.02);
        let n = wobble(self.p.seed, q.truncate());
        let mut best = f32::INFINITY;
        for z in &self.zones {
            let d = q.truncate() - z.c.truncate();
            let u = d.length() / z.r;
            if u >= 1.0 {
                continue;
            }
            let h = match self.mode {
                DestructionMode::Modern => z.r * (0.1 + 2.0 * u * u) / s + d.dot(z.tilt) * 0.5,
                DestructionMode::MedievalSiege => z.r * (0.05 + 2.5 * u) / s,
                DestructionMode::AncientRuins => z.r * (0.4 + 1.5 * u) / s,
            };
            best = best.min(z.c.z + h + n * z.r * 0.12);
        }
        if self.mode == DestructionMode::AncientRuins {
            // Time erodes every wall top, not just the zones.
            let top = self.bounds.max.z;
            let g = self.ground_at(q.truncate());
            best = best.min(g + (top - g) * (1.0 - 0.5 * self.s) + n * 0.6 + 0.3);
        }
        best
    }
}

/// Smooth seeded value in -1..=1 over the XY plane (a sum of sines).
pub fn wobble(seed: u32, q: Vec2) -> f32 {
    let s = seed as f32 * 0.618_034;
    let a = (q.x * 0.71 + s * 3.1).sin() * (q.y * 0.53 - s * 1.7).cos();
    let b = ((q.x + q.y) * 1.37 + s * 5.3).sin() * 0.5;
    let c = ((q.x - q.y * 0.8) * 2.9 + s).cos() * 0.25;
    ((a + b + c) / 1.75).clamp(-1.0, 1.0)
}
