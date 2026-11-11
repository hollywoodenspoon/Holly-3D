//! Mesh map baking (TextureCraft's `bake` design, MIT / Apache-2.0): the
//! object's shape written into UV space as Curvature, Cavity AO, World
//! Normal and Z-Height maps. Smart masks read them, so wear lands on real
//! edges and dirt in real crevices.

mod curvature;
pub mod geo;
pub mod smart;
#[cfg(test)]
mod tests;

pub use geo::GeoMap;
pub use smart::{SmartKind, SmartMask};

use super::raster::{PaintTri, paint_tris, raster_tri};
use crate::materials::Texture;
use crate::materials::proc_engine::grid::par_map;
use crate::mesh::EditablePoly;
use crate::render::bvh::{Bvh, Tri};
use bevy::math::{Mat4, Vec2, Vec3, Vec4};

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BakeSettings {
    /// Map size in texels (square).
    pub size: u32,
    /// Hemisphere rays per texel for Cavity AO.
    pub ao_rays: u32,
    /// How far wear and cavities reach from an edge, as a fraction of the
    /// object's size.
    pub edge_radius: f32,
}

impl Default for BakeSettings {
    fn default() -> Self {
        Self { size: 512, ao_rays: 24, edge_radius: 0.04 }
    }
}

/// Baked maps of one object, in its UV space (row 0 = top, v = 1).
#[derive(Clone, Debug)]
pub struct MeshMaps {
    pub size: u32,
    /// Texels inside a UV island (the rest are padding).
    pub valid: Vec<bool>,
    /// 0 = concave crease, 0.5 = flat, 1 = convex edge.
    pub curvature: Vec<f32>,
    /// 1 = open, 0 = fully occluded.
    pub ao: Vec<f32>,
    /// World-space surface normal.
    pub normal: Vec<Vec3>,
    /// World-space position.
    pub pos: Vec<Vec3>,
    /// Height along world Z, 0 at the object's lowest point, 1 at its top.
    pub height: Vec<f32>,
    /// Object version the maps were baked from.
    pub version: u64,
}

/// Uniform direction in the hemisphere around +Z, cosine weighted.
fn hemisphere(i: u32, n: u32, turn: f32) -> Vec3 {
    let (u, v) = ((i as f32 + 0.5) / n as f32, (i as f32 * 0.618_034 + turn).fract());
    let (r, phi) = (u.sqrt(), v * std::f32::consts::TAU);
    Vec3::new(r * phi.cos(), r * phi.sin(), (1.0 - u).max(0.0).sqrt())
}

impl MeshMaps {
    /// Bakes every map of `mesh` placed by `world`.
    pub fn bake(mesh: &EditablePoly, world: Mat4, cfg: &BakeSettings) -> MeshMaps {
        let size = cfg.size.clamp(16, 4096);
        let tris = paint_tris(mesh, world, false);
        let n = (size * size) as usize;
        // Which triangle owns each texel (inside beats seam padding).
        let mut owner: Vec<(u32, [f32; 3], u8)> = vec![(u32::MAX, [0.0; 3], 0); n];
        for (ti, t) in tris.iter().enumerate() {
            raster_tri(t, size, None, |i, w, inside| {
                let rank = if inside { 2 } else { 1 };
                if rank > owner[i].2 {
                    owner[i] = (ti as u32, w, rank);
                }
            });
        }
        let (lo, hi) = tris.iter().flat_map(|t| t.p).fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), p| (lo.min(p), hi.max(p)));
        let diag = if tris.is_empty() { 1.0 } else { (hi - lo).length().max(1e-6) };
        let edges = curvature::Edges::build(mesh, world, diag * cfg.edge_radius.max(1e-4));
        let bvh = Bvh::build(tris.iter().map(|t| Tri { p: t.p, n: t.n, uv: t.uv, col: [Vec4::ONE; 3], mat: 0, obj: 0 }).collect());
        let rays = cfg.ao_rays.clamp(1, 256);
        let reach = diag * 0.35;
        let texels = par_map(n, |i| {
            let (ti, w, rank) = owner[i];
            if rank == 0 {
                return (Vec3::ZERO, Vec3::Z, 0.5, 1.0);
            }
            let t: &PaintTri = &tris[ti as usize];
            // Clamp padding texels onto the triangle.
            let w = [w[0].max(0.0), w[1].max(0.0), w[2].max(0.0)];
            let s = (w[0] + w[1] + w[2]).max(1e-9);
            let w = [w[0] / s, w[1] / s, w[2] / s];
            let p = t.p[0] * w[0] + t.p[1] * w[1] + t.p[2] * w[2];
            let nrm = (t.n[0] * w[0] + t.n[1] * w[1] + t.n[2] * w[2]).normalize_or(t.normal);
            let (tx, ty) = nrm.any_orthonormal_pair();
            let o = p + t.normal * (diag * 1e-4);
            let turn = (i as f32 * 0.754_877).fract();
            let open = (0..rays)
                .filter(|&k| {
                    let h = hemisphere(k, rays, turn);
                    !bvh.occluded(o, (tx * h.x + ty * h.y + nrm * h.z).normalize(), reach)
                })
                .count();
            (p, nrm, edges.at(p, t.normal), open as f32 / rays as f32)
        });
        let span = (hi.z - lo.z).max(1e-6);
        MeshMaps {
            size,
            valid: owner.iter().map(|o| o.2 > 0).collect(),
            height: texels.iter().map(|t| ((t.0.z - lo.z) / span).clamp(0.0, 1.0)).collect(),
            pos: texels.iter().map(|t| t.0).collect(),
            normal: texels.iter().map(|t| t.1).collect(),
            curvature: texels.iter().map(|t| t.2).collect(),
            ao: texels.iter().map(|t| t.3).collect(),
            version: 0,
        }
    }

    /// Nearest texel index of a UV.
    pub fn index(&self, uv: Vec2) -> usize {
        let n = self.size as i64;
        let x = ((uv.x * n as f32).floor() as i64).rem_euclid(n);
        let y = (((1.0 - uv.y) * n as f32).floor() as i64).rem_euclid(n);
        (y * n + x) as usize
    }

    /// Fraction of the map inside UV islands.
    pub fn coverage(&self) -> f32 {
        self.valid.iter().filter(|v| **v).count() as f32 / self.valid.len().max(1) as f32
    }

    /// The maps as images, for `[ Export Textures ]`: curvature, AO and
    /// height in gray, the world normal packed like a normal map.
    pub fn images(&self) -> [(&'static str, Texture); 4] {
        let gray = |v: &[f32]| {
            let mut t = Texture::new(self.size, self.size, [0, 0, 0, 255]);
            for (d, x) in t.data.iter_mut().zip(v) {
                let b = (x.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
                *d = [b, b, b, 255];
            }
            t
        };
        let mut nrm = Texture::new(self.size, self.size, [128, 128, 255, 255]);
        for (d, n) in nrm.data.iter_mut().zip(&self.normal) {
            let b = |x: f32| ((x * 0.5 + 0.5).clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
            *d = [b(n.x), b(n.y), b(n.z), 255];
        }
        [("curvature", gray(&self.curvature)), ("ao", gray(&self.ao)), ("world_normal", nrm), ("height", gray(&self.height))]
    }
}
