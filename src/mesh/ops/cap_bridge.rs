//! Cap (border) and Bridge (borders / polygons).

use super::*;
use crate::mesh::adjacency::Adjacency;
use std::collections::HashSet;

fn loops_with_selected_edges(m: &EditablePoly) -> Vec<Vec<VertexId>> {
    let adj = Adjacency::build(m);
    adj.boundary_loops()
        .into_iter()
        .filter(|lp| {
            let n = lp.len();
            (0..n).any(|i| m.edge_between(lp[i], lp[(i + 1) % n]).is_some_and(|e| m.edges[e].selected))
        })
        .collect()
}

/// Cap: closes each selected border loop with a new polygon (selected afterwards).
pub fn cap_borders(m: &mut EditablePoly) -> usize {
    let loops = loops_with_selected_edges(m);
    let mut caps = vec![];
    for lp in &loops {
        let rev: Vec<VertexId> = lp.iter().rev().copied().collect();
        let c = m.polygons.values().next().cloned();
        let uvs = planar_loop_uvs(m, &rev);
        let pid = m.add_polygon_uv(&rev, &uvs);
        if let Some(t) = c {
            m.polygons[pid].smoothing_group = 0;
            m.polygons[pid].material_id = t.material_id;
        }
        caps.push(pid);
    }
    m.finalize();
    for p in &caps {
        m.polygons[*p].selected = true;
    }
    caps.len()
}

fn planar_loop_uvs(m: &EditablePoly, vs: &[VertexId]) -> Vec<Vec2> {
    let pts: Vec<Vec3> = vs.iter().map(|&v| m.pos(v)).collect();
    let c = pts.iter().sum::<Vec3>() / pts.len().max(1) as f32;
    let mut n = Vec3::ZERO;
    for i in 0..pts.len() {
        n += (pts[i] - c).cross(pts[(i + 1) % pts.len()] - c);
    }
    let n = n.normalize_or(Vec3::Z);
    let u = n.any_orthonormal_vector();
    let v = n.cross(u);
    let r = pts.iter().map(|p| (*p - c).length()).fold(1e-6, f32::max);
    pts.iter().map(|p| Vec2::new((*p - c).dot(u) / (2. * r) + 0.5, (*p - c).dot(v) / (2. * r) + 0.5)).collect()
}

/// Builds the tube between two border loops (given in border half-edge order).
pub fn bridge_loops(m: &mut EditablePoly, a: &[VertexId], b: &[VertexId], segments: u32, taper: f32, twist: i32) -> bool {
    let n = a.len();
    if n < 3 || b.len() != n {
        return false;
    }
    let partner = |s: usize, i: usize| b[(s + n * 4 - i) % n];
    let s_best = (0..n)
        .min_by(|&s1, &s2| {
            let cost = |s: usize| (0..n).map(|i| m.pos(a[i]).distance(m.pos(partner(s, i)))).sum::<f32>();
            cost(s1).total_cmp(&cost(s2))
        })
        .unwrap();
    let s = (s_best as i32 + twist).rem_euclid(n as i32) as usize;
    let segs = segments.max(1) as usize;
    let mut rings: Vec<Vec<VertexId>> = vec![a.to_vec()];
    for k in 1..segs {
        let t = k as f32 / segs as f32;
        let pts: Vec<Vec3> = (0..n).map(|i| m.pos(a[i]).lerp(m.pos(partner(s, i)), t)).collect();
        let c = pts.iter().sum::<Vec3>() / n as f32;
        let scale = 1.0 - taper * (std::f32::consts::PI * t).sin();
        rings.push(pts.iter().map(|&p| m.add_vertex(c + (p - c) * scale)).collect());
    }
    rings.push((0..n).map(|i| partner(s, i)).collect());
    let tmpl = m.polygons.values().next().cloned();
    for k in 0..segs {
        for i in 0..n {
            let i1 = (i + 1) % n;
            let (r0, r1) = (&rings[k], &rings[k + 1]);
            let vs = [r0[i1], r0[i], r1[i], r1[i1]];
            let u0 = i as f32 / n as f32;
            let u1 = (i + 1) as f32 / n as f32;
            let (v0, v1) = (k as f32 / segs as f32, (k + 1) as f32 / segs as f32);
            let pid = m.add_polygon_uv(&vs, &[Vec2::new(u1, v0), Vec2::new(u0, v0), Vec2::new(u0, v1), Vec2::new(u1, v1)]);
            if let Some(t) = &tmpl {
                m.polygons[pid].smoothing_group = t.smoothing_group;
                m.polygons[pid].material_id = t.material_id;
            }
            m.polygons[pid].selected = true;
        }
    }
    m.finalize();
    true
}

/// Bridge in Border/Edge mode: connects the two border loops containing selected edges.
pub fn bridge_borders(m: &mut EditablePoly, segments: u32, taper: f32, twist: i32) -> bool {
    let loops = loops_with_selected_edges(m);
    if loops.len() != 2 {
        return false;
    }
    m.clear_selection();
    bridge_loops(m, &loops[0], &loops[1], segments, taper, twist)
}

/// Bridge in Polygon mode: removes two selected polygons/regions and tunnels
/// between the resulting holes.
pub fn bridge_polygons(m: &mut EditablePoly, segments: u32, taper: f32, twist: i32) -> bool {
    let adj = Adjacency::build(m);
    let sel: HashSet<PolyId> = m.selected_polygons().into_iter().collect();
    let mut seen = HashSet::new();
    let regions: Vec<Vec<PolyId>> = sel
        .iter()
        .filter_map(|&p| if seen.contains(&p) { None } else { Some(adj.flood(m, p, &mut seen, |g| sel.contains(&g))) })
        .collect();
    if regions.len() != 2 {
        return false;
    }
    let vsets: Vec<HashSet<VertexId>> =
        regions.iter().map(|r| r.iter().flat_map(|&p| m.polygons[p].vertex_ids().collect::<Vec<_>>()).collect()).collect();
    for r in &regions {
        for &p in r {
            m.polygons.remove(p);
        }
    }
    m.remove_isolated_vertices();
    m.finalize();
    let adj = Adjacency::build(m);
    let loops = adj.boundary_loops();
    let find = |set: &HashSet<VertexId>| loops.iter().find(|lp| lp.iter().all(|v| set.contains(v))).cloned();
    let (Some(a), Some(b)) = (find(&vsets[0]), find(&vsets[1])) else { return false };
    m.clear_selection();
    bridge_loops(m, &a, &b, segments, taper, twist)
}
