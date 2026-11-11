//! Bakes procedural / mixed maps into UV-space textures for the GPU viewport
//! (3D maps are evaluated on a UV-derived point so they still read as grain).

use super::Material;
use super::maps::{MapChannel, MapNode};
use super::shading::{linear_to_srgb, surface};
use super::texture::{TexHandle, Texture};
use bevy::math::{Vec2, Vec3, Vec4};

pub const BAKE_SIZE: u32 = 256;

/// Point used for 3D maps when baking in UV space.
pub fn uv_point(uv: Vec2) -> Vec3 {
    Vec3::new(uv.x * 2.0 - 1.0, uv.y * 2.0 - 1.0, 0.0)
}

fn bake(size: u32, mut f: impl FnMut(Vec2) -> [u8; 4]) -> Texture {
    let mut t = Texture::new(size, size, [0; 4]);
    for y in 0..size {
        for x in 0..size {
            // Row 0 is the top (v = 1), matching `Texture::sample`.
            let uv = Vec2::new((x as f32 + 0.5) / size as f32, 1.0 - (y as f32 + 0.5) / size as f32);
            t.data[(y * size + x) as usize] = f(uv);
        }
    }
    t
}

fn byte(x: f32) -> u8 {
    (x.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

fn srgb_bytes(lin: Vec3, a: f32) -> [u8; 4] {
    [byte(linear_to_srgb(lin.x)), byte(linear_to_srgb(lin.y)), byte(linear_to_srgb(lin.z)), byte(a)]
}

/// A full-strength bitmap on Base Color (no opacity map) is used as-is,
/// keeping full resolution for painted canvases.
pub fn direct_base(m: &Material) -> Option<&TexHandle> {
    match m.active_map(MapChannel::BaseColor) {
        Some((MapNode::Bitmap { tex: Some(t), .. }, a)) if a >= 1.0 && m.active_map(MapChannel::Opacity).is_none() => Some(t),
        _ => None,
    }
}

/// Base color (sRGB) + alpha when a Base Color or Opacity map is active.
pub fn bake_base(m: &Material, size: u32) -> Option<Texture> {
    if m.active_map(MapChannel::BaseColor).is_none() && m.active_map(MapChannel::Opacity).is_none() && !m.proc.active() {
        return None;
    }
    Some(bake(size, |uv| {
        let s = surface(m, uv, uv_point(uv), Vec4::ONE);
        srgb_bytes(s.base, s.alpha)
    }))
}

/// glTF-style occlusion-roughness-metallic texture (R = cavity AO,
/// G = roughness, B = metallic, linear).
pub fn bake_metal_rough(m: &Material, size: u32) -> Option<Texture> {
    if m.active_map(MapChannel::Metallic).is_none() && m.active_map(MapChannel::Roughness).is_none() && !m.proc.active() {
        return None;
    }
    Some(bake(size, |uv| {
        let s = surface(m, uv, uv_point(uv), Vec4::ONE);
        [byte(s.ao), byte(s.roughness), byte(s.metallic), 255]
    }))
}

/// True when the material tilts its normals (a Bump / Normal map, or a
/// procedural surface with auto-derived relief).
pub fn has_relief(m: &Material) -> bool {
    m.active_map(MapChannel::Bump).is_some() || (m.proc.active() && m.proc.auto_pbr && m.proc.relief > 0.0)
}

/// Tangent-space normal map (linear, +Y up the V axis).
pub fn bake_normal(m: &Material, size: u32) -> Option<Texture> {
    has_relief(m).then(|| {
        bake(size, |uv| {
            let n = super::relief::tangent_normal(m, uv, uv_point(uv), Vec3::X * 2.0, Vec3::Y * 2.0).unwrap_or(Vec3::Z);
            [byte(n.x * 0.5 + 0.5), byte(n.y * 0.5 + 0.5), byte(n.z * 0.5 + 0.5), 255]
        })
    })
}

/// Emission (sRGB, normalised to 0..1; the material scales it back up).
pub fn bake_emissive(m: &Material, size: u32) -> Option<(Texture, f32)> {
    m.active_map(MapChannel::Emission)?;
    let mut peak = 1e-4f32;
    let samples: Vec<Vec3> = (0..size * size)
        .map(|i| {
            let uv = Vec2::new(((i % size) as f32 + 0.5) / size as f32, 1.0 - ((i / size) as f32 + 0.5) / size as f32);
            let e = surface(m, uv, uv_point(uv), Vec4::ONE).emissive;
            peak = peak.max(e.max_element());
            e
        })
        .collect();
    let mut t = Texture::new(size, size, [0; 4]);
    for (d, e) in t.data.iter_mut().zip(samples) {
        *d = srgb_bytes(e / peak, 1.0);
    }
    Some((t, peak))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::{Preset, Texture as Tex};

    #[test]
    fn plain_materials_need_no_textures() {
        let m = Preset::BrushedSteel.material();
        assert!(bake_base(&m, 16).is_none() && bake_metal_rough(&m, 16).is_none() && bake_emissive(&m, 16).is_none());
        assert!(direct_base(&m).is_none());
    }

    #[test]
    fn wood_preset_bakes_grain_and_roughness() {
        let m = Preset::Wood.material();
        let base = bake_base(&m, 32).unwrap();
        assert_eq!((base.width, base.height, base.data.len()), (32, 32, 1024));
        let lum: Vec<u32> = base.data.iter().map(|p| p[0] as u32 + p[1] as u32 + p[2] as u32).collect();
        assert!(lum.iter().max().unwrap() - lum.iter().min().unwrap() > 30, "wood rings should vary");
        let mr = bake_metal_rough(&m, 32).unwrap();
        assert!(mr.data.iter().all(|p| p[2] == 0), "wood is not metallic");
        assert!(mr.data.iter().any(|p| p[1] != mr.data[0][1]), "roughness map varies");
    }

    #[test]
    fn bitmap_base_is_used_directly_and_emission_normalises() {
        let mut m = Material::default();
        let h = TexHandle::new(Tex::new(8, 8, [10, 20, 30, 255]));
        m.map_mut(MapChannel::BaseColor).node = Some(MapNode::Bitmap { path: String::new(), tex: Some(h.clone()) });
        assert_eq!(direct_base(&m), Some(&h));
        m.map_mut(MapChannel::BaseColor).amount = 50.0;
        assert!(direct_base(&m).is_none());
        m.emissive_strength = 4.0;
        m.map_mut(MapChannel::Emission).node = Some(MapNode::new_kind(2, None));
        let (t, peak) = bake_emissive(&m, 16).unwrap();
        assert!(peak > 1.0);
        assert!(t.data.iter().any(|p| p[0] == 255));
    }
}
