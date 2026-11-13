//! The GPU path tracer's texture store (binding 9, one `u32` array):
//!
//! * per triangle, 16 words: UVs of the three corners (6 f32), the
//!   triangle's texture (`u32`, [`NONE`] = untextured), its material
//!   record (`u32`, [`NONE`] = none), the object's rotation quaternion and
//!   origin (World / Triplanar projection space);
//! * at `table`, 12 words per texture: texel offset, width, height, flags
//!   (1 wrap, 2 triplanar, 4 tilts normals, 8 live maps: the offset is
//!   then a [`live::record`]), UV origin and span, triplanar scale and
//!   blend exponent;
//! * then the baked texels ([`super::bake`]), the live-map records and,
//!   when there are any, the noise tables ([`live::tables`]);
//! * at `records`, [`REC_WORDS`] per material with Principled lobes or
//!   Preserve Texture Appearance ([`record`]).

use super::bake::{self, Baked, Space};
use super::live;
use crate::materials::Material;
use crate::render::scene_build::RenderScene;
use bevy::math::Vec2;

pub const NONE: u32 = u32::MAX;
pub const TRI_WORDS: usize = 16;
pub const TEX_WORDS: usize = 12;
pub const REC_WORDS: usize = 16;

pub struct Store {
    pub words: Vec<u32>,
    /// Word index of the texture table.
    pub table: u32,
    pub count: u32,
    /// Word index of the material records, and how many there are.
    pub records: u32,
    pub rec_count: u32,
}

impl Store {
    pub fn empty() -> Self {
        Self { words: vec![0; 4], table: 0, count: 0, records: 0, rec_count: 0 }
    }
    pub fn bytes(&self) -> Vec<u8> {
        self.words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }
}

/// Materials the GPU reads from the store: every mapped one.
pub fn textured(m: &Material) -> bool {
    super::features::mapped(m)
}

/// Materials with a record: Principled lobes, or a camera glow of their own.
pub fn has_record(m: &Material) -> bool {
    m.principled.active() || m.decoupled()
}

/// A material's record: anisotropy, rotation (deg), clearcoat, clearcoat
/// roughness | sheen, sheen tint, SSS weight, Abbe number | Beer
/// absorption exponent per unit length (rgb), camera glow brightness (< 0
/// = off) | random-walk extinction and single-scatter albedo (rgb).
pub fn record(m: &Material) -> [f32; REC_WORDS] {
    use crate::materials::principled::Principled;
    let p = &m.principled;
    let ab = if p.absorption_density > 0.0 {
        let c = crate::materials::shading::lin3(p.absorption_color).max(bevy::math::Vec3::splat(1e-3));
        bevy::math::Vec3::new(c.x.ln(), c.y.ln(), c.z.ln()) * p.absorption_density
    } else {
        bevy::math::Vec3::ZERO
    };
    let glow = if m.decoupled() { m.direct_camera_brightness.clamp(0.0, 2.0) } else { -1.0 };
    let col = crate::materials::shading::lin3(p.sss_color);
    let alb = col.to_array().map(Principled::walk_albedo);
    let (aniso, cc, sheen, sss) = (p.anisotropy.clamp(0.0, 1.0), p.clearcoat.clamp(0.0, 1.0), p.sheen.clamp(0.0, 1.0), p.sss_weight.clamp(0.0, 1.0));
    #[rustfmt::skip]
    let r = [aniso, p.aniso_rotation, cc, p.clearcoat_roughness.clamp(0.0, 1.0),
        sheen, p.sheen_tint.clamp(0.0, 1.0), sss, p.abbe.max(0.0),
        ab.x, ab.y, ab.z, glow,
        1.0 / p.mean_free_path(), alb[0], alb[1], alb[2]];
    r
}

/// Total store size for the scene's mapped materials (bytes).
pub fn planned_bytes(rs: &RenderScene) -> u64 {
    used(rs).iter().filter(|&&i| !live::live(&rs.materials[i])).map(|&i| bake::bytes(&rs.materials[i])).sum()
}

/// Indices of the mapped materials some triangle uses.
fn used(rs: &RenderScene) -> Vec<usize> {
    let mut on = vec![false; rs.materials.len()];
    for t in &rs.bvh.tris {
        on[t.mat] = true;
    }
    (0..on.len()).filter(|&i| on[i] && textured(&rs.materials[i])).collect()
}

/// Where material `mi`'s grid goes: one wrapping repeat when its maps
/// repeat (or a projection), else the bounds of the UVs that use it.
fn space(rs: &RenderScene, mi: usize) -> Space {
    let m = &rs.materials[mi];
    if m.projection.on() || bake::periodic(m) {
        return Space::UNIT;
    }
    let (mut lo, mut hi) = (Vec2::splat(f32::MAX), Vec2::splat(f32::MIN));
    for t in rs.bvh.tris.iter().filter(|t| t.mat == mi) {
        for uv in t.uv {
            (lo, hi) = (lo.min(uv), hi.max(uv));
        }
    }
    if lo.x > hi.x {
        return Space { wrap: false, ..Space::UNIT };
    }
    // A sliver of margin so the edge texels are not stretched by the clamp.
    let span = (hi - lo).max(Vec2::splat(1e-3));
    let pad = span * 0.002;
    Space { lo: lo - pad, span: span + pad * 2.0, wrap: false }
}

pub fn build(rs: &RenderScene) -> Store {
    let mats = used(rs);
    let mut rec = vec![NONE; rs.materials.len()];
    let mut recs = Vec::new();
    for t in &rs.bvh.tris {
        if rec[t.mat] == NONE && has_record(&rs.materials[t.mat]) {
            rec[t.mat] = recs.len() as u32;
            recs.push(t.mat);
        }
    }
    if mats.is_empty() && recs.is_empty() {
        return Store::empty();
    }
    let mut id = vec![NONE; rs.materials.len()];
    // Live-map materials carry an empty grid; their record goes after the texels.
    let baked: Vec<Option<Baked>> = mats
        .iter()
        .enumerate()
        .map(|(k, &mi)| {
            id[mi] = k as u32;
            let m = &rs.materials[mi];
            (!live::live(m)).then(|| bake::bake(m, space(rs, mi)))
        })
        .collect();
    let n = rs.bvh.tris.len();
    let table = n * TRI_WORDS;
    let texels: usize = baked.iter().flatten().map(|b| b.words.len()).sum();
    let lives = baked.iter().filter(|b| b.is_none()).count();
    let mut w = Vec::with_capacity(table + baked.len() * TEX_WORDS + texels + lives * live::WORDS);
    let f = |x: f32| x.to_bits();
    for t in &rs.bvh.tris {
        let (rot, origin) = rs.frames.get(t.obj as usize).copied().unwrap_or_default();
        w.extend(t.uv.iter().flat_map(|uv| [f(uv.x), f(uv.y)]));
        w.extend([id[t.mat], rec[t.mat]]);
        w.extend(rot.to_array().map(f));
        w.extend([f(origin.x), f(origin.y), f(origin.z), 0]);
    }
    let mut off = (table + baked.len() * TEX_WORDS) as u32;
    let mut live_off = off + texels as u32;
    let tables = live_off + (lives * live::WORDS) as u32;
    for (b, &mi) in baked.iter().zip(&mats) {
        let m = &rs.materials[mi];
        let pr = &m.projection;
        let k = 1.0 + (1.0 - pr.blend.clamp(0.0, 1.0)) * 15.0;
        let scale = f(1.0 / pr.size.max(1e-4));
        match b {
            Some(b) => {
                let flags = u32::from(b.space.wrap) | u32::from(pr.on()) << 1 | u32::from(b.normal) << 2;
                let (lo, span) = (b.space.lo, b.space.span);
                w.extend([off, b.w, b.h, flags, f(lo.x), f(lo.y), f(span.x), f(span.y), scale, f(k), 0, 0]);
                off += b.words.len() as u32;
            }
            None => {
                let flags = 8 | u32::from(pr.on()) << 1 | u32::from(live::tilts(m)) << 2;
                w.extend([live_off, 0, 0, flags, 0, 0, f(1.0), f(1.0), scale, f(k), 0, 0]);
                live_off += live::WORDS as u32;
            }
        }
    }
    for b in baked.iter().flatten() {
        w.extend_from_slice(&b.words);
    }
    for (b, &mi) in baked.iter().zip(&mats) {
        if b.is_none() {
            w.extend(live::record(&rs.materials[mi], tables));
        }
    }
    if lives > 0 {
        w.extend(live::tables());
    }
    let records = w.len() as u32;
    for &mi in &recs {
        w.extend(record(&rs.materials[mi]).map(f));
    }
    Store { words: w, table: table as u32, count: baked.len() as u32, records, rec_count: recs.len() as u32 }
}
