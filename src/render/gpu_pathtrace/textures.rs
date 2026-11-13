//! The GPU path tracer's texture store (binding 9, one `u32` array):
//!
//! * per triangle, 16 words: UVs of the three corners (6 f32), the
//!   triangle's texture (`u32`, [`NONE`] = untextured), the object's
//!   rotation quaternion and origin (World / Triplanar projection space);
//! * at `table`, 12 words per texture: texel offset, width, height, flags
//!   (1 wrap, 2 triplanar, 4 tilts normals), UV origin and span, triplanar
//!   scale and blend exponent;
//! * then the baked texels ([`super::bake`]).

use super::bake::{self, Space};
use crate::materials::Material;
use crate::render::scene_build::RenderScene;
use bevy::math::Vec2;

pub const NONE: u32 = u32::MAX;
pub const TRI_WORDS: usize = 16;
pub const TEX_WORDS: usize = 12;

pub struct Store {
    pub words: Vec<u32>,
    /// Word index of the texture table.
    pub table: u32,
    pub count: u32,
}

impl Store {
    pub fn empty() -> Self {
        Self { words: vec![0; 4], table: 0, count: 0 }
    }
    pub fn bytes(&self) -> Vec<u8> {
        self.words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }
}

/// Materials the GPU reads from the store: every mapped one.
pub fn textured(m: &Material) -> bool {
    super::features::mapped(m)
}

/// Total store size for the scene's mapped materials (bytes).
pub fn planned_bytes(rs: &RenderScene) -> u64 {
    used(rs).iter().map(|&i| bake::bytes(&rs.materials[i])).sum()
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
    if mats.is_empty() {
        return Store::empty();
    }
    let mut id = vec![NONE; rs.materials.len()];
    let baked: Vec<_> = mats
        .iter()
        .enumerate()
        .map(|(k, &mi)| {
            id[mi] = k as u32;
            bake::bake(&rs.materials[mi], space(rs, mi))
        })
        .collect();
    let n = rs.bvh.tris.len();
    let table = n * TRI_WORDS;
    let texels: usize = baked.iter().map(|b| b.words.len()).sum();
    let mut w = Vec::with_capacity(table + baked.len() * TEX_WORDS + texels);
    let f = |x: f32| x.to_bits();
    for t in &rs.bvh.tris {
        let (rot, origin) = rs.frames.get(t.obj as usize).copied().unwrap_or_default();
        w.extend(t.uv.iter().flat_map(|uv| [f(uv.x), f(uv.y)]));
        w.extend([id[t.mat], 0]);
        w.extend(rot.to_array().map(f));
        w.extend([f(origin.x), f(origin.y), f(origin.z), 0]);
    }
    let mut off = (table + baked.len() * TEX_WORDS) as u32;
    for (b, &mi) in baked.iter().zip(&mats) {
        let pr = &rs.materials[mi].projection;
        let flags = u32::from(b.space.wrap) | u32::from(pr.on()) << 1 | u32::from(b.normal) << 2;
        let k = 1.0 + (1.0 - pr.blend.clamp(0.0, 1.0)) * 15.0;
        let (lo, span) = (b.space.lo, b.space.span);
        w.extend([off, b.w, b.h, flags, f(lo.x), f(lo.y), f(span.x), f(span.y), f(1.0 / pr.size.max(1e-4)), f(k), 0, 0]);
        off += b.words.len() as u32;
    }
    for b in &baked {
        w.extend_from_slice(&b.words);
    }
    Store { words: w, table: table as u32, count: baked.len() as u32 }
}
