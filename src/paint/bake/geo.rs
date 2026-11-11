//! Per-texel geometry of a painted object, for the viewport composite:
//! which Material ID owns each texel (Multi/Sub-Object materials) and,
//! when asked, where the texel sits in projection space (World /
//! Triplanar materials).

use crate::paint::raster::{PaintTri, raster_tri};
use bevy::math::Vec3;

#[derive(Clone, Debug)]
pub struct GeoMap {
    pub size: u32,
    /// Polygon material ID per texel (0 = outside every UV island).
    pub ids: Vec<u16>,
    /// Position and normal per texel; empty unless baked with `space`.
    pub pos: Vec<[f32; 3]>,
    pub nrm: Vec<[i8; 3]>,
}

impl GeoMap {
    /// Rasterizes `tris` (already in projection space) into UV space.
    pub fn build(tris: &[PaintTri], size: u32, space: bool) -> GeoMap {
        let n = (size * size) as usize;
        let mut g = GeoMap { size, ids: vec![0; n], pos: vec![], nrm: vec![] };
        if space {
            g.pos = vec![[0.0; 3]; n];
            g.nrm = vec![[0, 0, 127]; n];
        }
        let mut rank = vec![0u8; n];
        for t in tris {
            raster_tri(t, size, None, |i, w, inside| {
                let r = if inside { 2 } else { 1 };
                if r <= rank[i] {
                    return;
                }
                rank[i] = r;
                g.ids[i] = t.id.max(1);
                if space {
                    let p = t.p[0] * w[0] + t.p[1] * w[1] + t.p[2] * w[2];
                    let nn = (t.n[0] * w[0] + t.n[1] * w[1] + t.n[2] * w[2]).normalize_or(t.normal);
                    g.pos[i] = p.to_array();
                    g.nrm[i] = [(nn.x * 127.0) as i8, (nn.y * 127.0) as i8, (nn.z * 127.0) as i8];
                }
            });
        }
        g
    }

    /// Position and normal of texel `i`, when baked with `space`.
    pub fn space(&self, i: usize) -> Option<(Vec3, Vec3)> {
        let (p, n) = (self.pos.get(i)?, self.nrm.get(i)?);
        Some((Vec3::from_array(*p), Vec3::new(n[0] as f32, n[1] as f32, n[2] as f32).normalize_or(Vec3::Z)))
    }
}
