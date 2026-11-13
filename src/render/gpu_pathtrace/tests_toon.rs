//! Cel / toon materials on the GPU: the same banded direct light as the
//! CPU (every light, area lights at their shadow samples, glowing
//! triangles), rim and specular highlights, all in the Environment group.

use super::features::reasons;
use super::tests::{add, parity, scene_seen, scene_with};
use super::textures::REC_WORDS;
use crate::materials::presets::Preset;
use crate::materials::{Material, ShadingModel};
use crate::primitives::PrimitiveKind;
use crate::render::{Light, LightKind};
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::Vec3;

#[test]
fn the_kernel_steps_material_records_by_rec_words() {
    assert!(super::source().contains(&format!("return u.tex.z + r * {REC_WORDS}u;")));
}

/// A big three-step toon sphere (so its rim covers whole blocks) with rim and specular and a two-step toon box,
/// lit by the omni, a rect area light and the glowing panel.
#[test]
fn toon_materials_match_the_cpu() {
    let rs = scene_with(|s| {
        let mut cel = Preset::CelShaded.material();
        (cel.base_color, cel.toon.steps, cel.toon.rim, cel.toon.specular) = ([0.9, 0.5, 0.6], 3, 0.8, 0.6);
        add(s, PrimitiveKind::Sphere, Vec3::new(-0.2, 0.7, 0.4), Vec3::splat(1.2), cel);
        let flat = Material { model: ShadingModel::Toon, base_color: [0.3, 0.7, 0.4], ..Material::named("Flat Toon") };
        add(s, PrimitiveKind::Box, Vec3::new(0.9, 0.3, 0.0), Vec3::splat(0.4), flat);
        let mut panel = Light::new(LightKind::RectArea);
        (panel.multiplier, panel.area_size) = (1.5, [0.5, 0.5]);
        s.add_light(panel, Some("Panel"), Vec3::new(0.6, 0.6, 1.6), Vec3::ZERO);
    });
    assert!(reasons(&rs).is_empty(), "{:?}", reasons(&rs));
    assert!(rs.materials.iter().filter(|m| m.model == ShadingModel::Toon).count() == 2);
    parity("toon sphere and box", &rs);
}

/// A toon sphere filling a close view under a bright glowing panel, so the
/// rim ring and the glowing-triangle light each cover whole blocks.
#[test]
fn a_toon_close_up_matches_rim_and_glow() {
    let mut c = ViewCamera::new(ViewKind::Perspective);
    (c.target, c.distance) = (Vec3::new(0.0, 0.0, 0.7), 2.4);
    let rs = scene_seen(|s| {
        let mut cel = Preset::CelShaded.material();
        (cel.base_color, cel.toon.steps, cel.toon.rim, cel.toon.rim_color) = ([0.5, 0.6, 0.9], 3, 1.0, [1.0, 0.6, 0.2]);
        add(s, PrimitiveKind::Sphere, Vec3::new(0.0, 0.0, 0.7), Vec3::splat(1.3), cel);
        let glow = Material { emissive: [1.0, 0.95, 0.8], emissive_strength: 6.0, ..Material::named("Lamp") };
        add(s, PrimitiveKind::Plane, Vec3::new(0.9, -0.9, 1.9), Vec3::splat(0.5), glow);
    }, c);
    assert!(reasons(&rs).is_empty(), "{:?}", reasons(&rs));
    parity("toon close up", &rs);
}
