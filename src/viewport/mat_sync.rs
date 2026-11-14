//! Scene materials -> Bevy `StandardMaterial`s: bakes map textures once per
//! material change and builds per-object PBR / transmission / unlit looks.

use crate::materials::bake::{BAKE_SIZE, bake_base, bake_emissive, bake_metal_rough, bake_normal, direct_base};
use crate::materials::shading::lin3;
use crate::materials::{MatId, Material, ShadingModel, Texture};
use bevy::asset::RenderAssetUsages;
use bevy::image::{ImageAddressMode, ImageFilterMode, ImageSampler, ImageSamplerDescriptor};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use std::collections::HashMap;

/// GPU textures for one material.
#[derive(Clone, Default)]
pub struct MatTextures {
    pub base: Option<Handle<Image>>,
    pub metal_rough: Option<Handle<Image>>,
    pub emissive: Option<(Handle<Image>, f32)>,
    /// Tangent-space normal map (Bump map or procedural relief).
    pub normal: Option<Handle<Image>>,
    /// Cavity AO: the metal-rough texture's red channel, when it carries one.
    pub occlusion: Option<Handle<Image>>,
}

struct Entry {
    mat: Material,
    rev: u64,
    tex: MatTextures,
    /// Multi/Sub-Object: textures of each sub-material (list order).
    subs: Vec<MatTextures>,
    /// Layer composite of a painted material.
    paint: Option<super::paint_sync::PaintGpu>,
}

/// Per-material bake cache, refreshed when a material's value changes.
#[derive(Resource, Default)]
pub struct MatCache {
    entries: HashMap<MatId, Entry>,
}

impl MatCache {
    /// (revision, material, textures) of a cached material.
    pub fn get(&self, id: MatId) -> Option<(u64, &Material, &MatTextures)> {
        self.entries.get(&id).map(|e| (e.rev, &e.mat, &e.tex))
    }

    /// Textures of sub-material `i` of a cached Multi/Sub-Object material.
    pub fn sub_textures(&self, id: MatId, i: usize) -> Option<&MatTextures> {
        self.entries.get(&id).and_then(|e| e.subs.get(i))
    }
}

pub fn to_image(t: &Texture, srgb: bool) -> Image {
    let format = if srgb { TextureFormat::Rgba8UnormSrgb } else { TextureFormat::Rgba8Unorm };
    let mut img = Image::new(
        Extent3d { width: t.width, height: t.height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        t.bytes(),
        format,
        RenderAssetUsages::default(),
    );
    img.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
        address_mode_u: ImageAddressMode::Repeat,
        address_mode_v: ImageAddressMode::Repeat,
        mag_filter: ImageFilterMode::Linear,
        min_filter: ImageFilterMode::Linear,
        ..default()
    });
    img
}

fn bake_all(m: &Material, images: &mut Assets<Image>) -> MatTextures {
    let base = direct_base(m).map(|h| to_image(&h.tex, true));
    // Procedural surfaces are cooked at a higher resolution than plain map bakes.
    let size = if m.proc.active() { crate::materials::proc_engine::surface::COOK_SIZE } else { BAKE_SIZE };
    let base = base.or_else(|| bake_base(m, size).map(|t| to_image(&t, true)));
    let metal_rough = bake_metal_rough(m, size).map(|t| images.add(to_image(&t, false)));
    MatTextures {
        base: base.map(|i| images.add(i)),
        occlusion: metal_rough.clone().filter(|_| m.proc.active() && m.proc.auto_pbr),
        metal_rough,
        emissive: bake_emissive(m, BAKE_SIZE).map(|(t, peak)| (images.add(to_image(&t, true)), peak)),
        normal: bake_normal(m, size).map(|t| images.add(to_image(&t, false))),
    }
}

/// Re-bakes materials used by visible objects whose value changed.
pub fn sync_material_cache(editor: Res<crate::editor::Editor>, mut cache: ResMut<MatCache>, mut images: ResMut<Assets<Image>>) {
    let lib = &editor.scene.materials;
    let used: Vec<MatId> = editor.scene.objects.values().filter_map(|o| o.material).filter(|id| lib.mats.contains_key(*id)).collect();
    cache.entries.retain(|k, _| used.contains(k));
    for id in used {
        let m = &lib.mats[id];
        if cache.entries.get(&id).is_some_and(|e| e.mat == *m) {
            continue;
        }
        // Procedural maps cook on a worker thread; keep the old look until they are ready.
        if !crate::materials::proc_engine::worker::ready(m) {
            continue;
        }
        // Painted materials: one composite of all layers (and of all
        // sub-materials), updated tile by tile while painting.
        if let Some(stack) = &m.paint {
            let mut gpu = cache.entries.remove(&id).and_then(|e| e.paint).unwrap_or_else(|| super::paint_sync::PaintGpu::new(stack.size));
            let owner = editor.scene.objects.values().find(|o| o.material == Some(id));
            let tex = gpu.sync(m, owner, &mut images);
            cache.entries.insert(id, Entry { mat: m.clone(), rev: crate::scene::next_version(), tex, subs: vec![], paint: Some(gpu) });
            continue;
        }
        let tex = bake_all(m, &mut images);
        let subs = m.multi.as_ref().map_or(vec![], |ms| ms.subs.iter().map(|s| bake_all(&s.mat, &mut images)).collect());
        cache.entries.insert(id, Entry { mat: m.clone(), rev: crate::scene::next_version(), tex, subs, paint: None });
    }
}

fn color(c: [f32; 3], a: f32) -> Color {
    Color::srgba(c[0], c[1], c[2], a)
}

/// Viewport look for a material (`see_through` = Alt+X x-ray).
pub fn standard(m: &Material, tex: &MatTextures, see_through: bool) -> StandardMaterial {
    let textured_base = tex.base.is_some();
    let opacity = m.opacity * if see_through { 0.35 } else { 1.0 };
    let base = if textured_base { color([1.0; 3], opacity) } else { color(m.base_color, opacity) };
    let (emissive, emissive_texture) = match &tex.emissive {
        // Preserve Texture Appearance: glow with the surface's own colours.
        _ if m.decoupled() => {
            let k = m.direct_camera_brightness.clamp(0.0, 2.0);
            let e = if textured_base { Vec3::splat(k) } else { lin3(m.base_color) * k };
            (LinearRgba::rgb(e.x, e.y, e.z).with_alpha(0.0), tex.base.clone())
        }
        Some((h, peak)) => (LinearRgba::rgb(*peak, *peak, *peak).with_alpha(0.0), Some(h.clone())),
        None => {
            let e = lin3(m.emissive) * m.emissive_strength;
            (LinearRgba::rgb(e.x, e.y, e.z).with_alpha(0.0), None)
        }
    };
    let blended = opacity < 1.0 || m.active_map(crate::materials::MapChannel::Opacity).is_some();
    let mut s = StandardMaterial {
        base_color: base,
        base_color_texture: tex.base.clone(),
        metallic: if tex.metal_rough.is_some() { 1.0 } else { m.metallic },
        perceptual_roughness: if tex.metal_rough.is_some() { 1.0 } else { m.roughness.max(0.02) },
        metallic_roughness_texture: tex.metal_rough.clone(),
        normal_map_texture: tex.normal.clone(),
        occlusion_texture: tex.occlusion.clone(),
        emissive,
        emissive_texture,
        double_sided: true,
        cull_mode: None,
        // Cut-out alpha: MSAA alpha-to-coverage (no sorting, soft edges).
        alpha_mode: match (blended, m.alpha_cutout) {
            (true, _) => AlphaMode::Blend,
            (false, true) => AlphaMode::AlphaToCoverage,
            _ => AlphaMode::Opaque,
        },
        ..default()
    };
    let pr = &m.principled;
    if !matches!(m.model, ShadingModel::Toon | ShadingModel::Unlit) {
        // Viewport preview of the Principled lobes (F9 path-traces them).
        s.clearcoat = pr.clearcoat.clamp(0.0, 1.0);
        s.clearcoat_perceptual_roughness = pr.clearcoat_roughness.clamp(0.0, 1.0);
        s.anisotropy_strength = pr.anisotropy.clamp(0.0, 1.0);
        s.anisotropy_rotation = pr.aniso_rotation.to_radians();
    }
    match m.model {
        ShadingModel::Physical => s.diffuse_transmission = pr.sss_weight.clamp(0.0, 1.0) * 0.5,
        // Viewport approximation of the cel ramp: flat diffuse, no specular
        // sheen (F9 renders the true quantised bands, rim and highlights).
        ShadingModel::Toon => {
            s.perceptual_roughness = 1.0;
            s.reflectance = 0.0;
            s.metallic = 0.0;
        }
        ShadingModel::Crystal => {
            s.specular_transmission = m.transmission.clamp(0.0, 1.0);
            s.ior = m.ior.max(1.0);
            s.thickness = m.thickness.max(0.0);
            s.attenuation_color = color(m.attenuation_color, 1.0);
            s.attenuation_distance = m.thickness.max(0.01) * 2.0;
            s.diffuse_transmission = 0.0;
        }
        ShadingModel::Unlit => s.unlit = true,
        // Viewport stand-in for a volume: a soft matte in its scatter colour.
        ShadingModel::Volume => {
            s.base_color = color(m.volume.scatter, 0.85);
            (s.perceptual_roughness, s.reflectance, s.metallic) = (1.0, 0.0, 0.0);
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::Preset;

    #[test]
    fn presets_map_to_matching_standard_materials() {
        let none = MatTextures::default();
        let gold = standard(&Preset::PolishedGold.material(), &none, false);
        assert_eq!((gold.metallic, gold.alpha_mode), (1.0, AlphaMode::Opaque));
        let crystal = standard(&Preset::Crystal.material(), &none, false);
        assert!((crystal.specular_transmission - 0.92).abs() < 1e-6 && (crystal.ior - 1.55).abs() < 1e-6);
        let xray = standard(&Preset::Clay.material(), &none, true);
        assert_eq!(xray.alpha_mode, AlphaMode::Blend);
        let mut unlit = Material { model: ShadingModel::Unlit, ..Default::default() };
        assert!(standard(&unlit, &none, false).unlit);
        unlit.emissive_strength = 2.0;
        assert!(standard(&unlit, &none, false).emissive.red > 1.9);
        let mut paint = Preset::Clay.material();
        (paint.principled.clearcoat, paint.principled.anisotropy) = (1.0, 0.7);
        let p = standard(&paint, &none, false);
        assert!(p.clearcoat == 1.0 && (p.anisotropy_strength - 0.7).abs() < 1e-6, "Principled lobes preview in the viewport");
        let hair = Material { alpha_cutout: true, ..Default::default() };
        assert_eq!(standard(&hair, &none, false).alpha_mode, AlphaMode::AlphaToCoverage, "cut-out alpha needs no sorting");
        assert_eq!(standard(&hair, &none, true).alpha_mode, AlphaMode::Blend, "X-Ray still blends");
    }
}
