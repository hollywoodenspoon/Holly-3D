//! Inset, Outline and Bevel for polygons.

use super::dispatch::ExtrudeMode;
use super::extrude::extrude_polygons;
use super::*;
use crate::mesh::adjacency::Adjacency;
use std::collections::HashSet;

/// Outline: moves the border of each selected region in its plane.
/// Positive `amount` grows the region outward, negative shrinks it.
pub fn outline_region(m: &mut EditablePoly, polys: &[PolyId], amount: f32) {
    let sset: HashSet<PolyId> = polys.iter().copied().collect();
    let adj = Adjacency::build(m);
    let mut inward: HashMap<VertexId, Vec<Vec3>> = HashMap::new();
    for &pid in polys {
        let n = m.poly_normal(pid);
        for (a, b) in m.polygons[pid].half_edges() {
            let twin_in = adj.half_edge.get(&(b, a)).is_some_and(|g| sset.contains(g));
            if twin_in {
                continue;
            }
            let d = n.cross(m.pos(b) - m.pos(a)).normalize_or_zero();
            inward.entry(a).or_default().push(d);
            inward.entry(b).or_default().push(d);
        }
    }
    let moves: Vec<(VertexId, Vec3)> = inward
        .into_iter()
        .map(|(v, ns)| {
            let off = if ns.len() == 2 {
                miter(ns[0], ns[1])
            } else {
                ns.iter().sum::<Vec3>().normalize_or_zero()
            };
            (v, -off * amount)
        })
        .collect();
    for (v, d) in moves {
        m.vertices[v].position += d;
    }
}

fn offset_each_polygon(m: &mut EditablePoly, polys: &[PolyId], inward: f32) {
    for &pid in polys {
        let offs = corner_inset_offsets(m, pid);
        let vs: Vec<VertexId> = m.polygons[pid].vertex_ids().collect();
        for (v, o) in vs.into_iter().zip(offs) {
            m.vertices[v].position += o * inward;
        }
    }
}

/// Inset: creates a ring of new polygons inside the selection.
pub fn inset(m: &mut EditablePoly, by_polygon: bool, amount: f32) {
    let sel = m.selected_polygons();
    if sel.is_empty() {
        return;
    }
    if by_polygon {
        extrude_polygons(m, ExtrudeMode::ByPolygon, 0.0);
        offset_each_polygon(m, &sel, amount);
    } else {
        extrude_polygons(m, ExtrudeMode::Group, 0.0);
        outline_region(m, &sel, -amount);
    }
}

/// Outline op on the current polygon selection.
pub fn outline(m: &mut EditablePoly, amount: f32) {
    let sel = m.selected_polygons();
    outline_region(m, &sel, amount);
}

/// Bevel: extrude by `height` then outline the caps by `outline_amount`.
pub fn bevel(m: &mut EditablePoly, mode: ExtrudeMode, height: f32, outline_amount: f32) {
    let sel = m.selected_polygons();
    if sel.is_empty() {
        return;
    }
    extrude_polygons(m, mode, height);
    match mode {
        ExtrudeMode::ByPolygon => offset_each_polygon(m, &sel, -outline_amount),
        _ => outline_region(m, &sel, outline_amount),
    }
}
