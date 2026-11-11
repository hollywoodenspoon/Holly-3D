//! Per-vertex tangents for normal maps: each triangle's dP/dU is summed
//! onto its corners, then made perpendicular to the vertex normal. The
//! sign (w) says which way V runs, so mirrored UVs light correctly.

use crate::materials::relief::uv_frame;
use crate::mesh::triangulate::TriMesh;
use bevy::math::{Vec2, Vec3};

/// `[x, y, z, w]` tangents for a triangle mesh (Bevy's `ATTRIBUTE_TANGENT`).
pub fn tangents(t: &TriMesh) -> Vec<[f32; 4]> {
    let n = t.positions.len();
    let (mut tan, mut bit) = (vec![Vec3::ZERO; n], vec![Vec3::ZERO; n]);
    // Mesh UVs are stored V-down for the GPU; maps are authored V-up.
    let uv = |i: u32| Vec2::new(t.uvs[i as usize][0], 1.0 - t.uvs[i as usize][1]);
    for tri in t.indices.as_chunks::<3>().0 {
        let p = tri.map(|i| Vec3::from_array(t.positions[i as usize]));
        if let Some((dpdu, dpdv)) = uv_frame(p, [uv(tri[0]), uv(tri[1]), uv(tri[2])]) {
            // Weight by direction only, so tiny UV islands do not swamp their neighbours.
            let (du, dv) = (dpdu.normalize_or_zero(), dpdv.normalize_or_zero());
            for &i in tri {
                tan[i as usize] += du;
                bit[i as usize] += dv;
            }
        }
    }
    (0..n)
        .map(|i| {
            let nrm = Vec3::from_array(t.normals[i]);
            let x = (tan[i] - nrm * nrm.dot(tan[i])).normalize_or(nrm.any_orthonormal_vector());
            let w = if nrm.cross(x).dot(bit[i]) < 0.0 { -1.0 } else { 1.0 };
            [x.x, x.y, x.z, w]
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad(flip_u: bool) -> TriMesh {
        // A unit quad in XY facing +Z; U along +X (or mirrored), V up (+Y), stored V-down.
        let u = |x: f32| if flip_u { 1.0 - x } else { x };
        TriMesh {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 4],
            uvs: vec![[u(0.0), 1.0], [u(1.0), 1.0], [u(1.0), 0.0], [u(0.0), 0.0]],
            colors: vec![[1.0; 4]; 4],
            indices: vec![0, 1, 2, 0, 2, 3],
            tri_poly: vec![],
        }
    }

    #[test]
    fn tangents_follow_u_and_the_sign_follows_v() {
        let t = tangents(&quad(false));
        assert_eq!(t.len(), 4);
        assert!(t.iter().all(|v| (v[0] - 1.0).abs() < 1e-5 && v[3] == 1.0), "{t:?}");
        // Mirrored U: the tangent turns round and the handedness flips.
        let m = tangents(&quad(true));
        assert!(m.iter().all(|v| (v[0] + 1.0).abs() < 1e-5 && v[3] == -1.0), "{m:?}");
        // No UVs at all still gives unit tangents perpendicular to the normal.
        let mut flat = quad(false);
        flat.uvs = vec![[0.0, 0.0]; 4];
        assert!(tangents(&flat).iter().all(|v| v[2].abs() < 1e-5 && (v[0] * v[0] + v[1] * v[1] - 1.0).abs() < 1e-4));
    }
}
