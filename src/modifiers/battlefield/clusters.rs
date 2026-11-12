//! PR 7 `Building Destruction` level of the Battlefield modifier: the
//! input's elements are grouped into buildings (elements whose XY bounds
//! overlap, skipping flat ground, roads and lots), and each building is
//! wrecked on its own by `generators::ruin` with the mode's era.

use super::{BattleParams, DestructionMode};
use crate::generators::ruin::{self, DestructionLevel, RuinEra, RuinParams};
use crate::mesh::adjacency::Adjacency;
use crate::mesh::{Aabb, EditablePoly, PolyId};
use bevy::math::Mat4;

fn find(p: &mut [usize], i: usize) -> usize {
    let mut i = i;
    while p[i] != i {
        p[i] = p[p[i]];
        i = p[i];
    }
    i
}

/// Buildings of `m`: unions of non-flat elements with overlapping footprints
/// (an element covering a large share of the whole input, such as a cratered
/// ground, is not a building).
pub fn buildings(m: &EditablePoly) -> Vec<Vec<PolyId>> {
    let Some(all) = m.bounds() else { return vec![] };
    let span = (all.max - all.min).truncate().max(bevy::math::Vec2::splat(1e-3));
    let adj = Adjacency::build(m);
    let els: Vec<(Vec<PolyId>, Aabb)> = adj
        .elements(m)
        .into_iter()
        .filter_map(|e| {
            let b = Aabb::from_points(e.iter().flat_map(|&p| m.polygons[p].vertex_ids()).map(|v| m.pos(v)))?;
            let s = (b.max - b.min).truncate() / span;
            (b.max.z - b.min.z > 0.5 && s.x * s.y < 0.4).then_some((e, b))
        })
        .collect();
    let mut parent: Vec<usize> = (0..els.len()).collect();
    for i in 0..els.len() {
        for j in i + 1..els.len() {
            let (a, b) = (&els[i].1, &els[j].1);
            let pad = 0.2;
            if a.min.x < b.max.x + pad && b.min.x < a.max.x + pad && a.min.y < b.max.y + pad && b.min.y < a.max.y + pad {
                let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                parent[ri] = rj;
            }
        }
    }
    let mut groups: std::collections::BTreeMap<usize, Vec<PolyId>> = Default::default();
    for (i, (e, _)) in els.into_iter().enumerate() {
        let r = find(&mut parent, i);
        groups.entry(r).or_default().extend(e);
    }
    groups.into_values().collect()
}

/// Wrecks every building of `m` to `p.buildings` (one step either side).
pub fn run(m: &mut EditablePoly, p: &BattleParams) {
    let base = &p.buildings;
    if base.level == DestructionLevel::Intact {
        return;
    }
    let era = match (base.era, p.mode) {
        (RuinEra::Auto, DestructionMode::Modern) => RuinEra::Modern,
        (RuinEra::Auto, DestructionMode::MedievalSiege) => RuinEra::Medieval,
        (RuinEra::Auto, DestructionMode::AncientRuins) => RuinEra::Ancient,
        (e, _) => e,
    };
    for (i, polys) in buildings(m).into_iter().enumerate() {
        let Some(bb) = Aabb::from_points(polys.iter().flat_map(|&q| m.polygons[q].vertex_ids()).map(|v| m.pos(v))) else { continue };
        let s = bb.max - bb.min;
        // Poles, lamps and props stay for the regular battlefield passes.
        if s.x * s.y < 4.0 || s.z < 2.5 {
            continue;
        }
        m.clear_selection();
        polys.iter().for_each(|&q| m.polygons[q].selected = true);
        let mut b = crate::mesh::ops::geometry::detach(m, false);
        let level = base.level.step(((i as u32).wrapping_mul(2_654_435_761) >> 30) as i32 % 3 - 1).max(DestructionLevel::Light);
        let rp = RuinParams { level, era, seed: base.seed.wrapping_add(i as u32 * 7919), ..base.clone() };
        ruin::apply(&mut b, &rp, false);
        m.append(&b, Mat4::IDENTITY);
    }
    m.clear_selection();
}
