//! Panel Line / Seam Groove: cuts a recessed channel along the selected
//! edges. The edges are chamfered into a strip `width + 2 × lip` wide; a
//! 45° bevel lip steps down and in by `lip`, then the channel floor sinks
//! to `depth`. The channel walls and floor take the inner material ID and
//! stay faceted (smoothing group 0). The floor ends up selected.

use super::chamfer::chamfer_edges_ex;
use super::dispatch::ExtrudeMode;
use super::extrude::extrude_polygons;
use super::inset::outline;
use super::*;
use std::collections::HashMap;
use std::collections::HashSet;

/// Polygons not in `before`.
fn added(m: &EditablePoly, before: &HashSet<PolyId>) -> Vec<PolyId> {
    m.polygons.keys().filter(|k| !before.contains(k)).collect()
}

/// Extrudes the selected polygons (Group, zero height, so walls appear)
/// and sinks them by `h` along mitered normals: every face drops by exactly
/// `h`, also across a hard edge where Local Normal would cut shallower.
fn sink(m: &mut EditablePoly, h: f32) {
    extrude_polygons(m, ExtrudeMode::Group, 0.0);
    let mut normals: HashMap<VertexId, Vec<Vec3>> = HashMap::new();
    for p in m.selected_polygons() {
        let n = m.poly_normal(p);
        for v in m.polygons[p].vertex_ids() {
            let ns = normals.entry(v).or_default();
            if !ns.iter().any(|q| q.dot(n) > 0.999) {
                ns.push(n);
            }
        }
    }
    for (v, ns) in normals {
        let d = ns.iter().sum::<Vec3>().normalize_or_zero();
        let cos = ns.iter().map(|n| n.dot(d)).fold(1.0f32, f32::min).max(0.25);
        m.vertices[v].position -= d * (h / cos);
    }
}

/// Returns false when there was nothing to groove.
pub fn panel_groove(m: &mut EditablePoly, width: f32, depth: f32, lip: f32, material_id: u16) -> bool {
    let (width, depth, lip) = (width.max(0.0), depth.max(0.0), lip.max(0.0));
    if m.selected_edges().is_empty() || width <= 1e-6 {
        return false;
    }
    let mut grooved: Vec<PolyId> = vec![];
    chamfer_edges_ex(m, width * 0.5 + lip, 1, 0.5, false);
    if m.selected_polygons().is_empty() {
        return false;
    }
    // Bevel lip: down and in by `lip` (at most the whole depth).
    let lip_h = lip.min(depth);
    if lip > 1e-6 {
        let keys: HashSet<PolyId> = m.polygons.keys().collect();
        sink(m, lip_h);
        outline(m, -lip);
        grooved.extend(added(m, &keys));
    }
    let lip_faces = grooved.len();
    // Channel walls and floor.
    if depth - lip_h > 1e-6 {
        let keys: HashSet<PolyId> = m.polygons.keys().collect();
        sink(m, depth - lip_h);
        grooved.extend(added(m, &keys));
    }
    let floor = m.selected_polygons();
    for (i, &p) in grooved.iter().chain(&floor).enumerate() {
        let Some(poly) = m.polygons.get_mut(p) else { continue };
        poly.smoothing_group = 0;
        if i >= lip_faces {
            poly.material_id = material_id.clamp(1, 64);
        }
    }
    m.edges.values_mut().for_each(|e| e.selected = false);
    m.finalize();
    true
}
