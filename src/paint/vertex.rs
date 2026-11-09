//! VertexPaint strokes: brush stamps on the per-vertex paint layer.
//! Opacity is per stroke (coverage takes the max over stamps), so
//! dragging back and forth never builds past the brush opacity.

use super::{Brush, BrushTool};
use crate::mesh::adjacency::Adjacency;
use crate::mesh::{EditablePoly, VertexId};
use crate::modifiers::vertex_paint::over;
use bevy::math::{Mat4, Vec3, Vec4};
use std::collections::{HashMap, HashSet};

pub struct VertexStroke {
    /// World position of every paintable input vertex.
    positions: Vec<(VertexId, Vec3)>,
    input: HashMap<VertexId, Vec4>,
    neighbors: HashMap<VertexId, Vec<VertexId>>,
    /// Layer at stroke start.
    pre: HashMap<VertexId, Vec4>,
    cov: HashMap<VertexId, f32>,
}

/// Vertices of the selected polygons (the "Selected Faces Only" mask).
pub fn selected_face_vertices(m: &EditablePoly) -> HashSet<VertexId> {
    m.selected_polygons().into_iter().flat_map(|p| m.polygons[p].vertex_ids().collect::<Vec<_>>()).collect()
}

impl VertexStroke {
    pub fn new(input: &EditablePoly, world: Mat4, layer: &HashMap<VertexId, Vec4>, selected_only: bool) -> Self {
        let mask = selected_only.then(|| selected_face_vertices(input));
        let positions = input
            .vertices
            .iter()
            .filter(|(k, _)| mask.as_ref().is_none_or(|m| m.contains(k)))
            .map(|(k, v)| (k, world.transform_point3(v.position)))
            .collect();
        let adj = Adjacency::build(input);
        Self {
            positions,
            input: input.vertices.iter().map(|(k, v)| (k, v.color)).collect(),
            neighbors: adj.vertex_neighbors,
            pre: layer.clone(),
            cov: HashMap::new(),
        }
    }

    /// Color of `v` with the layer at full opacity.
    fn painted(&self, layer: &HashMap<VertexId, Vec4>, v: VertexId) -> Vec3 {
        let base = self.input.get(&v).copied().unwrap_or(Vec4::ONE).truncate();
        layer.get(&v).map_or(base, |l| base.lerp(l.truncate(), l.w))
    }

    /// One brush stamp centered at `center` (world). Returns vertices touched.
    pub fn stamp(&mut self, layer: &mut HashMap<VertexId, Vec4>, center: Vec3, brush: &Brush) -> usize {
        let color = Vec3::from_array(brush.color);
        let hits: Vec<(VertexId, f32)> =
            self.positions.iter().map(|(k, p)| (*k, brush.weight(p.distance(center)))).filter(|(_, w)| *w > 0.0).collect();
        match brush.tool {
            BrushTool::Paint | BrushTool::Erase => {
                for &(k, w) in &hits {
                    let c = self.cov.entry(k).or_insert(0.0);
                    *c = c.max(w);
                    let pre = self.pre.get(&k).copied().unwrap_or(Vec4::ZERO);
                    let out = if brush.tool == BrushTool::Paint { over(pre, color, *c) } else { pre.truncate().extend(pre.w * (1.0 - *c)) };
                    if out.w <= 1e-4 {
                        layer.remove(&k);
                    } else {
                        layer.insert(k, out);
                    }
                }
            }
            BrushTool::Blur => {
                let targets: Vec<(VertexId, f32, Vec3)> = hits
                    .iter()
                    .filter_map(|&(k, w)| {
                        let n = self.neighbors.get(&k)?;
                        if n.is_empty() {
                            return None;
                        }
                        let avg = n.iter().map(|&v| self.painted(layer, v)).sum::<Vec3>() / n.len() as f32;
                        Some((k, w * 0.5, avg))
                    })
                    .collect();
                for (k, w, avg) in targets {
                    let cur = layer.get(&k).copied().unwrap_or_else(|| self.painted(layer, k).extend(0.0));
                    let out = over(cur, avg, w);
                    layer.insert(k, out);
                }
            }
        }
        hits.len()
    }
}

/// PaintBox "Fill Object": floods every (or every masked) vertex.
pub fn fill(input: &EditablePoly, layer: &mut HashMap<VertexId, Vec4>, color: [f32; 3], opacity: f32, selected_only: bool) -> usize {
    let mask = selected_only.then(|| selected_face_vertices(input));
    let mut n = 0;
    for k in input.vertices.keys().filter(|k| mask.as_ref().is_none_or(|m| m.contains(k))) {
        let pre = layer.get(&k).copied().unwrap_or(Vec4::ZERO);
        layer.insert(k, over(pre, Vec3::from_array(color), opacity));
        n += 1;
    }
    n
}
