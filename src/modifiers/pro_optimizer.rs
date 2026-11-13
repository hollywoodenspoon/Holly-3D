//! ProOptimizer: quadric-error decimation (meshoptimizer). Attribute seams
//! (UVs, material IDs, hard normals) are kept by splitting render vertices
//! along them, so the simplifier only collapses along a seam, never across.

use crate::mesh::triangulate::triangulate_polygon;
use crate::mesh::{EditablePoly, FaceCorner, Polygon, VertexId};
use bevy::math::{Vec2, Vec3};
use meshopt::{SimplifyOptions, VertexDataAdapter, typed_to_bytes};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ProOptimizerMod {
    /// Vertex % of the calculated source (1..=100).
    pub percent: f32,
    /// Nothing is reduced until `[ Calculate ]` has been pressed.
    pub calculated: bool,
    pub source_verts: usize,
    pub keep_materials: bool,
    pub keep_uvs: bool,
    pub keep_colors: bool,
    pub keep_normals: bool,
    /// Never move open-border vertices.
    pub protect_borders: bool,
}

impl Default for ProOptimizerMod {
    fn default() -> Self {
        Self {
            percent: 100.0,
            calculated: false,
            source_verts: 0,
            keep_materials: true,
            keep_uvs: true,
            keep_colors: true,
            keep_normals: true,
            protect_borders: true,
        }
    }
}

/// One split vertex fed to the simplifier.
#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
struct RenderVert {
    vid: VertexId,
    uv: Vec2,
    mat: u16,
    sg: u32,
}

impl ProOptimizerMod {
    pub fn calculate(&mut self, input: &EditablePoly) {
        self.calculated = true;
        self.source_verts = input.vertices.len();
    }

    pub fn target_verts(&self) -> usize {
        (self.source_verts as f32 * self.percent.clamp(1.0, 100.0) / 100.0).round() as usize
    }

    /// Vertex Count spinner, linked to Vertex %.
    pub fn set_target_verts(&mut self, n: usize) {
        if self.source_verts > 0 {
            self.percent = (n as f32 / self.source_verts as f32 * 100.0).clamp(1.0, 100.0);
        }
    }

    pub fn apply(&self, m: &mut EditablePoly) {
        if !self.calculated || self.percent >= 100.0 || m.polygons.is_empty() {
            return;
        }
        *m = self.optimize(m);
    }

    fn optimize(&self, m: &EditablePoly) -> EditablePoly {
        let normals = if self.keep_normals { m.corner_normals() } else { HashMap::new() };
        let mut key_of: HashMap<(VertexId, [u32; 2], u16, [i32; 3]), u32> = HashMap::new();
        let mut verts: Vec<RenderVert> = vec![];
        let (mut positions, mut attrs, mut indices) = (vec![], vec![], vec![]);
        for (pid, p) in &m.polygons {
            let pts: Vec<Vec3> = p.vertex_ids().map(|v| m.pos(v)).collect();
            let ids: Vec<u32> = p
                .corners
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    let uv = if self.keep_uvs { c.uv.to_array().map(f32::to_bits) } else { [0; 2] };
                    let mat = if self.keep_materials { p.material_id } else { 0 };
                    let n = normals.get(&pid).map_or(Vec3::ZERO, |ns| ns[i]);
                    let nq = (n * 1000.0).round().as_ivec3().to_array();
                    *key_of.entry((c.vertex_id, uv, mat, nq)).or_insert_with(|| {
                        let v = &m.vertices[c.vertex_id];
                        positions.push(v.position.to_array());
                        if self.keep_colors {
                            attrs.extend_from_slice(&v.color.to_array());
                        }
                        if self.keep_normals {
                            attrs.extend_from_slice(&n.to_array());
                        }
                        verts.push(RenderVert { vid: c.vertex_id, uv: c.uv, mat: p.material_id, sg: p.smoothing_group });
                        verts.len() as u32 - 1
                    })
                })
                .collect();
            for tri in triangulate_polygon(&pts) {
                indices.extend(tri.map(|k| ids[k]));
            }
        }
        let tris = indices.len() / 3;
        let target = ((tris as f32 * self.percent / 100.0).round() as usize).max(1) * 3;
        let Ok(adapter) = VertexDataAdapter::new(typed_to_bytes(&positions), 12, 0) else { return m.clone() };
        let opts = if self.protect_borders { SimplifyOptions::LockBorder } else { SimplifyOptions::None };
        let stride = (self.keep_colors as usize) * 4 + (self.keep_normals as usize) * 3;
        let result = if stride == 0 {
            meshopt::simplify(&indices, &adapter, target, 1.0, opts, None)
        } else {
            let mut weights = vec![];
            weights.extend(std::iter::repeat_n(1.0, self.keep_colors as usize * 4));
            weights.extend(std::iter::repeat_n(0.5, self.keep_normals as usize * 3));
            let locks = vec![false; verts.len()];
            meshopt::simplify_with_attributes_and_locks(&indices, &adapter, &attrs, &weights, stride * 4, &locks, target, 1.0, opts, None)
        };
        self.rebuild(m, &verts, &result)
    }

    fn rebuild(&self, m: &EditablePoly, verts: &[RenderVert], tris: &[u32]) -> EditablePoly {
        let mut out = EditablePoly::new();
        let mut remap: HashMap<VertexId, VertexId> = HashMap::new();
        for t in tris.as_chunks::<3>().0 {
            let rv: Vec<RenderVert> = t.iter().map(|&i| verts[i as usize]).collect();
            let corners: Vec<FaceCorner> = rv
                .iter()
                .map(|r| {
                    let nv = *remap.entry(r.vid).or_insert_with(|| {
                        let src = &m.vertices[r.vid];
                        let id = out.add_vertex(src.position);
                        out.vertices[id].color = src.color;
                        id
                    });
                    FaceCorner::new(nv, r.uv)
                })
                .collect();
            // majority material when boundaries were not kept
            let mat = if rv[1].mat == rv[2].mat { rv[1].mat } else { rv[0].mat };
            let mut template = Polygon::new(vec![]);
            template.material_id = mat;
            template.smoothing_group = rv[0].sg;
            out.add_polygon_corners(corners, &template);
        }
        out.finalize();
        out
    }
}
