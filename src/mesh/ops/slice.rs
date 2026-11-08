//! Slice Plane / QuickSlice.

use super::connect::insert_edge_points;
use super::*;
use std::collections::HashSet;

/// Cuts edges across the plane (`point`, `normal`). If `only_selected` and some
/// polygons are selected, only those are sliced. With `split` the mesh is also
/// separated along the cut. Returns the number of new edges.
pub fn slice(m: &mut EditablePoly, point: Vec3, normal: Vec3, only_selected: bool, split: bool) -> usize {
    let n = normal.normalize_or(Vec3::Z);
    let size = m.bounds().map(|b| b.size().length()).unwrap_or(1.0);
    let eps = 1e-5 * (1.0 + size);
    let sel = m.selected_polygons();
    let targets: Vec<PolyId> = if only_selected && !sel.is_empty() { sel } else { m.polygons.keys().collect() };
    let d: HashMap<VertexId, f32> = m.vertices.iter().map(|(k, v)| (k, (v.position - point).dot(n))).collect();
    let mut pts: HashMap<(VertexId, VertexId), Vec<(VertexId, f32)>> = HashMap::new();
    for &pid in &targets {
        let hes: Vec<_> = m.polygons[pid].half_edges().collect();
        for (a, b) in hes {
            let key = edge_key(a, b);
            if pts.contains_key(&key) {
                continue;
            }
            let (da, db) = (d[&key.0], d[&key.1]);
            if da.abs() > eps && db.abs() > eps && da * db < 0.0 {
                let t = da / (da - db);
                let p = m.pos(key.0).lerp(m.pos(key.1), t);
                let v = m.add_vertex(p);
                pts.insert(key, vec![(v, t)]);
            }
        }
    }
    insert_edge_points(m, &pts);
    let on_plane: HashSet<VertexId> = m
        .vertices
        .iter()
        .filter(|(k, v)| d.get(k).map(|x| x.abs() <= eps).unwrap_or(true) || (v.position - point).dot(n).abs() <= eps)
        .map(|(k, _)| k)
        .collect();
    let mut new_edges = vec![];
    for pid in targets {
        let fnrm = m.poly_normal(pid);
        let line = n.cross(fnrm);
        let mut hits: Vec<VertexId> = m.polygons[pid].vertex_ids().filter(|v| on_plane.contains(v)).collect();
        if hits.len() < 2 {
            continue;
        }
        hits.sort_by(|x, y| m.pos(*x).dot(line).total_cmp(&m.pos(*y).dot(line)));
        let mut pieces = vec![pid];
        for pair in hits.chunks(2) {
            if pair.len() < 2 {
                break;
            }
            let (a, b) = (pair[0], pair[1]);
            let Some(&holder) =
                pieces.iter().find(|&&p| m.polygons.get(p).is_some_and(|q| q.position_of(a).is_some() && q.position_of(b).is_some()))
            else {
                continue;
            };
            if let Some((_, q)) = split_polygon(m, holder, a, b) {
                pieces.push(q);
                new_edges.push((a, b));
            }
        }
    }
    m.finalize();
    if split && !new_edges.is_empty() {
        let keep_sel = m.selection_sets();
        m.clear_selection();
        for &(a, b) in &new_edges {
            if let Some(e) = m.edge_between(a, b) {
                m.edges[e].selected = true;
            }
        }
        super::remove::split_edges(m);
        m.apply_selection_sets(&keep_sel);
    }
    new_edges.len()
}
