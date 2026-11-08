//! Edit Geometry tools: Make Planar (+X/Y/Z), Flip, smoothing groups,
//! material IDs, Detach, Tessellate, MSmooth and vertex transforms.

use super::connect::insert_edge_points;
use super::*;
use crate::mesh::adjacency::Adjacency;
use bevy::math::Mat4;
use std::collections::HashSet;

/// Make Planar: projects the selection onto its average plane.
pub fn make_planar(m: &mut EditablePoly, level: SubObject) {
    let verts = m.selection_vertices(level);
    if verts.len() < 3 {
        return;
    }
    let c = verts.iter().map(|&v| m.pos(v)).sum::<Vec3>() / verts.len() as f32;
    let n = if level.is_face_like() {
        m.selected_polygons().iter().map(|&p| m.poly_normal_raw(p)).sum::<Vec3>()
    } else {
        let vn = m.vertex_normals();
        verts.iter().map(|v| vn[v]).sum::<Vec3>()
    }
    .normalize_or(Vec3::Z);
    for v in verts {
        let p = m.pos(v);
        m.vertices[v].position = p - n * (p - c).dot(n);
    }
}

/// Make Planar X / Y / Z: flattens the selection to its average coordinate on `axis`.
pub fn flatten_axis(m: &mut EditablePoly, level: SubObject, axis: usize) {
    let verts = m.selection_vertices(level);
    if verts.is_empty() {
        return;
    }
    let avg = verts.iter().map(|&v| m.pos(v)[axis]).sum::<f32>() / verts.len() as f32;
    for v in verts {
        m.vertices[v].position[axis] = avg;
    }
}

/// Flip: reverses winding (normals) of the selected polygons.
pub fn flip(m: &mut EditablePoly) {
    for p in m.polygons.values_mut().filter(|p| p.selected) {
        p.corners.reverse();
    }
}

pub fn set_smoothing_group(m: &mut EditablePoly, bit: u32, on: bool) {
    for p in m.polygons.values_mut().filter(|p| p.selected) {
        if on {
            p.smoothing_group |= 1 << bit;
        } else {
            p.smoothing_group &= !(1 << bit);
        }
    }
}

pub fn clear_smoothing(m: &mut EditablePoly) {
    m.polygons.values_mut().filter(|p| p.selected).for_each(|p| p.smoothing_group = 0);
}

pub fn set_material_id(m: &mut EditablePoly, id: u16) {
    m.polygons.values_mut().filter(|p| p.selected).for_each(|p| p.material_id = id.clamp(1, 64));
}

/// Detach: extracts the selected polygons into a new mesh. Unless `keep`
/// (Detach As Clone) they are removed from `m`.
pub fn detach(m: &mut EditablePoly, keep: bool) -> EditablePoly {
    let sel = m.selected_polygons();
    let mut out = EditablePoly::new();
    let mut map = HashMap::new();
    for &pid in &sel {
        let mut p = m.polygons[pid].clone();
        for c in &mut p.corners {
            c.vertex_id = *map.entry(c.vertex_id).or_insert_with(|| {
                let mut v = m.vertices[c.vertex_id].clone();
                v.selected = false;
                out.vertices.insert(v)
            });
        }
        p.selected = false;
        out.polygons.insert(p);
    }
    out.rebuild_edges();
    if !keep {
        for pid in sel {
            m.polygons.remove(pid);
        }
        m.remove_isolated_vertices();
        m.finalize();
    }
    out
}

/// Tessellate selected polygons: `face_center` pokes each face; otherwise each
/// face is split into quads through its edge midpoints and center.
pub fn tessellate(m: &mut EditablePoly, face_center: bool) {
    let sel = m.selected_polygons();
    if face_center {
        for pid in sel {
            let c = m.poly_centroid(pid);
            super::insert::poke_polygon(m, pid, c);
        }
        return;
    }
    let mut mids: HashMap<(VertexId, VertexId), Vec<(VertexId, f32)>> = HashMap::new();
    let mut orig: HashMap<PolyId, Vec<VertexId>> = HashMap::new();
    for &pid in &sel {
        orig.insert(pid, m.polygons[pid].vertex_ids().collect());
        let hes: Vec<_> = m.polygons[pid].half_edges().collect();
        for (a, b) in hes {
            let k = edge_key(a, b);
            if !mids.contains_key(&k) {
                let mid = m.pos(a).lerp(m.pos(b), 0.5);
                let v = m.add_vertex(mid);
                mids.insert(k, vec![(v, 0.5)]);
            }
        }
    }
    insert_edge_points(m, &mids);
    for pid in sel {
        let p = m.polygons[pid].clone();
        let center = m.add_vertex(m.poly_centroid(pid));
        let cuv = p.corners.iter().map(|c| c.uv).sum::<Vec2>() / p.corners.len() as f32;
        let ov: HashSet<VertexId> = orig[&pid].iter().copied().collect();
        let n = p.corners.len();
        m.polygons.remove(pid);
        for i in 0..n {
            if !ov.contains(&p.corners[i].vertex_id) {
                continue;
            }
            let prev = p.corners[(i + n - 1) % n].clone();
            let next = p.corners[(i + 1) % n].clone();
            let id = m.add_polygon_corners(vec![prev, p.corners[i].clone(), next, FaceCorner::new(center, cuv)], &p);
            m.polygons[id].selected = true;
        }
    }
    m.finalize();
}

/// MSmooth: tessellate the selection, then relax its interior vertices.
pub fn msmooth(m: &mut EditablePoly, strength: f32) {
    if m.selected_polygons().is_empty() {
        return;
    }
    tessellate(m, false);
    let adj = Adjacency::build(m);
    let verts = m.selection_vertices(SubObject::Polygon);
    let moves: Vec<(VertexId, Vec3)> = verts
        .iter()
        .filter(|&&v| !adj.is_border_vertex(v))
        .map(|&v| {
            let ns = adj.neighbors(v);
            let avg = ns.iter().map(|&n| m.pos(n)).sum::<Vec3>() / ns.len().max(1) as f32;
            (v, m.pos(v).lerp(avg, strength))
        })
        .collect();
    for (v, p) in moves {
        m.vertices[v].position = p;
    }
}

/// Applies `mat` (object-local space) to the given vertices.
pub fn transform_vertices(m: &mut EditablePoly, verts: &[VertexId], mat: Mat4) {
    for &v in verts {
        if let Some(v) = m.vertices.get_mut(v) {
            v.position = mat.transform_point3(v.position);
        }
    }
}

/// Soft Selection weights: 1 for selected vertices, falling off to 0 at
/// `falloff` distance (shaped by `pinch` / `bubble`, like 3ds Max).
pub fn soft_weights(m: &EditablePoly, selected: &[VertexId], falloff: f32, pinch: f32, bubble: f32) -> HashMap<VertexId, f32> {
    let sel: HashSet<VertexId> = selected.iter().copied().collect();
    let pts: Vec<Vec3> = selected.iter().map(|&v| m.pos(v)).collect();
    let mut out = HashMap::new();
    for (id, v) in &m.vertices {
        if sel.contains(&id) {
            out.insert(id, 1.0);
            continue;
        }
        if falloff <= 0.0 || pts.is_empty() {
            continue;
        }
        let d = pts.iter().map(|p| p.distance(v.position)).fold(f32::MAX, f32::min);
        if d >= falloff {
            continue;
        }
        let x = d / falloff;
        let smooth = (1.0 - x) * (1.0 - x) * (1.0 + 2.0 * x);
        let w = smooth + bubble * (std::f32::consts::PI * x).sin() * 0.5 - pinch * x * (1.0 - x);
        out.insert(id, w.clamp(0.0, 1.0));
    }
    out
}

/// Weighted transform used by Soft Selection drags.
pub fn soft_transform(m: &mut EditablePoly, weights: &[(VertexId, f32)], mat: Mat4) {
    for &(v, w) in weights {
        if let Some(v) = m.vertices.get_mut(v) {
            v.position = v.position.lerp(mat.transform_point3(v.position), w);
        }
    }
}
