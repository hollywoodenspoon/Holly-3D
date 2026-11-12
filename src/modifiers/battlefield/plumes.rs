//! PR 8 `[✓] Spawn Rising Smoke Plumes`: columns of smoke rising from the
//! tallest wrecks (and the user blast zones),
//! with a fire glow at the base. Seeded and deterministic.

use super::BattleParams;
use crate::generators::smoke::{self, SmokeParams};
use crate::mesh::EditablePoly;
use bevy::math::{Vec2, Vec3};
use rand::Rng;

/// Ground spots for the plumes: the tallest cells of a coarse height grid
/// over the input (each at least two cells from the others), then the
/// user zones. Robust to rubble that links the buildings together.
fn spots(m: &EditablePoly, zones: &[super::BlastZone]) -> Vec<Vec2> {
    let Some(b) = m.bounds() else { return vec![] };
    let cell = ((b.max - b.min).truncate().max_element() / 12.0).max(1.0);
    let mut top: std::collections::BTreeMap<(i32, i32), f32> = Default::default();
    for v in m.vertices.values() {
        let k = ((v.position.x / cell).floor() as i32, (v.position.y / cell).floor() as i32);
        let e = top.entry(k).or_insert(f32::MIN);
        *e = e.max(v.position.z);
    }
    let mut cells: Vec<((i32, i32), f32)> = top.into_iter().filter(|&(_, z)| z > b.min.z + 2.0).collect();
    cells.sort_by(|a, c| c.1.total_cmp(&a.1));
    let mut out: Vec<Vec2> = vec![];
    for ((x, y), _) in cells {
        let c = Vec2::new(x as f32 + 0.5, y as f32 + 0.5) * cell;
        if out.iter().all(|o| o.distance(c) > cell * 2.0) {
            out.push(c);
        }
    }
    out.extend(zones.iter().map(|z| z.center.truncate()));
    out
}

/// Height of the wreck around `c` (the top of the rubble, where it burns).
fn ground(m: &EditablePoly, c: Vec2, r: f32) -> f32 {
    let mut zs: Vec<f32> = m.vertices.values().filter(|v| v.position.truncate().distance(c) < r).map(|v| v.position.z).collect();
    if zs.is_empty() {
        return m.bounds().map_or(0.0, |b| b.min.z);
    }
    zs.sort_by(f32::total_cmp);
    zs[zs.len() * 3 / 5]
}

pub fn run(m: &mut EditablePoly, p: &BattleParams, zones: &[super::BlastZone]) {
    if !p.plumes || p.plume_count == 0 || m.polygons.is_empty() {
        return;
    }
    let mut r = crate::generators::kit::rng(p.seed, 0x50E7);
    let mut at = spots(m, zones);
    if at.is_empty() {
        at.extend(m.bounds().map(|b| ((b.min + b.max) * 0.5).truncate()));
    }
    let count = (p.plume_count as usize).min(at.len().max(1) * 2);
    let mut placed: Vec<(Vec3, SmokeParams)> = vec![];
    for i in 0..count {
        let jitter = Vec2::new(r.gen_range(-2.0..2.0), r.gen_range(-2.0..2.0)) * (i / at.len().max(1)) as f32;
        let c = at[i % at.len()] + jitter;
        let base = c.extend(ground(m, c, p.plume.base_radius.max(1.0) * 1.5));
        let sp = SmokeParams {
            seed: p.plume.seed.wrapping_add(i as u32 * 131),
            height: p.plume.height * r.gen_range(0.75..1.2),
            phase: (p.plume.phase + r.gen_range(0.0..1.0)).fract(),
            ..p.plume.clone()
        };
        placed.push((base, sp));
    }
    for (base, sp) in placed {
        smoke::spawn(m, base, &sp);
    }
}
