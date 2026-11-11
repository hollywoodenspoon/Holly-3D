//! World / Triplanar mapping in the GPU viewport: the mesh gets box-mapped
//! UVs (each vertex projected along the axis its normal faces most), in
//! the object's own space at world scale. The F9 renderer blends the three
//! projections exactly; this is the viewport's seam-free stand-in.

use crate::materials::{Material, Projection};
use crate::mesh::triangulate::TriMesh;
use crate::scene::SceneObject;
use bevy::math::Vec3;

/// Scale of the object's world matrix (projection space = local x scale).
pub fn scale_of(o: &SceneObject) -> Vec3 {
    o.world_matrix().to_scale_rotation_translation().0
}

/// Changes whenever the box-mapped UVs of `o` would: 0 for materials that
/// use the mesh's own UVs (painted materials always do; their composite
/// holds the projected base).
pub fn key(o: &SceneObject, m: Option<&Material>) -> u64 {
    let Some(m) = m.filter(|m| m.projection.on() && m.paint.is_none()) else { return 0 };
    let s = scale_of(o);
    [m.projection.size, s.x, s.y, s.z].iter().fold(1u64, |h, v| h.wrapping_mul(0x100_0000_01b3) ^ v.to_bits() as u64)
}

/// Replaces the UVs of `tri` with the box projection of `p`.
pub fn apply(tri: &mut TriMesh, p: &Projection, scale: Vec3) {
    for i in 0..tri.positions.len() {
        let uv = p.box_uv(Vec3::from_array(tri.positions[i]) * scale, Vec3::from_array(tri.normals[i]));
        // Mesh UVs are stored V-down for the GPU.
        tri.uvs[i] = [uv.x, 1.0 - uv.y];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::{Primitive, PrimitiveKind};

    #[test]
    fn box_mapped_uvs_follow_world_size_and_keep_counts() {
        let mesh = Primitive::default_of(PrimitiveKind::Box).to_editable_poly();
        let mut tri = mesh.to_tri_mesh();
        let (verts, before) = (tri.positions.len(), tri.uvs.clone());
        apply(&mut tri, &Projection::triplanar(0.5), Vec3::ONE);
        assert_eq!((tri.positions.len(), tri.uvs.len()), (verts, verts), "only the UVs change");
        assert_ne!(tri.uvs, before);
        // Along any triangle edge the UV moves 1 / size as far as the point does.
        for t in tri.indices.chunks(3) {
            let (a, b) = (t[0] as usize, t[1] as usize);
            let d = (Vec3::from_array(tri.positions[a]) - Vec3::from_array(tri.positions[b])).length();
            let du = ((tri.uvs[a][0] - tri.uvs[b][0]).powi(2) + (tri.uvs[a][1] - tri.uvs[b][1]).powi(2)).sqrt();
            assert!((du - d / 0.5).abs() < 1e-4, "{du} vs {d}");
        }
    }
}
