//! PR 2: Principled lobes on the GPU. Anisotropic GGX, clearcoat, sheen,
//! random-walk SSS, Beer absorption, dispersion and Preserve Texture
//! Appearance glow render on the GPU like on the CPU.

use super::features::{Features, reasons};
use super::tests::{add, parity};
use super::textures::{self, NONE, REC_WORDS, TRI_WORDS};
use crate::materials::Material;
use crate::materials::principled::Principled;
use crate::primitives::PrimitiveKind;
use crate::render::scene_build::RenderScene;
use crate::render::{Light, LightKind};
use crate::scene::Scene;
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::Vec3;

fn lit(build: impl FnOnce(&mut Scene)) -> RenderScene {
    let mut s = Scene::default();
    (s.environment.ambient_level, s.environment.background) = (0.0, [0.2, 0.25, 0.3]);
    add(&mut s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(3.0), Material { base_color: [0.7; 3], roughness: 0.8, ..Material::named("Floor") });
    build(&mut s);
    s.add_light(Light::new(LightKind::Omni), None, Vec3::new(-1.0, -1.0, 2.5), Vec3::ZERO);
    let mut c = ViewCamera::new(ViewKind::Top);
    (c.target, c.ortho_height) = (Vec3::ZERO, 4.0);
    RenderScene::build(&s, c, false)
}

fn look(name: &str, base: [f32; 3], metallic: f32, roughness: f32, p: Principled) -> Material {
    Material { base_color: base, metallic, roughness, principled: p, ..Material::named(name) }
}

#[test]
fn clearcoat_sheen_and_anisotropy_render_on_the_gpu() {
    let rs = lit(|s| {
        let brushed = Principled { anisotropy: 0.8, aniso_rotation: 30.0, ..Default::default() };
        add(s, PrimitiveKind::Sphere, Vec3::new(-0.7, -0.5, 0.4), Vec3::splat(0.7), look("Brushed", [0.9, 0.8, 0.6], 1.0, 0.4, brushed));
        let paint = Principled { clearcoat: 1.0, clearcoat_roughness: 0.05, ..Default::default() };
        add(s, PrimitiveKind::Box, Vec3::new(0.6, -0.4, 0.3), Vec3::splat(0.6), look("Car Paint", [0.7, 0.05, 0.05], 0.3, 0.5, paint));
        let velvet = Principled { sheen: 1.0, sheen_tint: 0.8, ..Default::default() };
        add(s, PrimitiveKind::Sphere, Vec3::new(0.0, 0.7, 0.4), Vec3::splat(0.7), look("Velvet", [0.3, 0.1, 0.5], 0.0, 0.9, velvet));
    });
    assert_eq!(reasons(&rs), Vec::<&str>::new(), "Principled lobes no longer need the CPU");
    assert_ne!(Features::of(&rs).0 & Features::PRINCIPLED, 0);
    let st = textures::build(&rs);
    assert_eq!((st.count, st.rec_count), (0, 3), "no maps, three records");
    parity("clearcoat, sheen and anisotropy", &rs);
}

#[test]
fn subsurface_scattering_renders_on_the_gpu() {
    let rs = lit(|s| {
        let wax = Principled { sss_weight: 1.0, sss_color: [0.95, 0.7, 0.5], sss_radius: [1.0, 0.5, 0.3], sss_scale: 0.15, ..Default::default() };
        add(s, PrimitiveKind::Sphere, Vec3::new(-0.4, 0.0, 0.5), Vec3::splat(0.9), look("Wax", [0.9, 0.8, 0.7], 0.0, 0.5, wax));
        // A thin slab light shines through.
        let leaf = Principled { sss_weight: 0.7, sss_color: [0.5, 0.9, 0.3], sss_scale: 0.05, ..Default::default() };
        add(s, PrimitiveKind::Box, Vec3::new(0.7, 0.3, 0.6), Vec3::new(0.8, 0.8, 0.03), look("Leaf", [0.4, 0.8, 0.3], 0.0, 0.6, leaf));
    });
    assert_ne!(Features::of(&rs).0 & Features::SSS, 0);
    assert_eq!(reasons(&rs), Vec::<&str>::new());
    parity("subsurface scattering", &rs);
}

#[test]
fn dispersion_and_absorption_render_on_the_gpu() {
    // A thick slab over a glowing floor: every refracted path splits into
    // one channel and fades with the distance it travels inside.
    let rs = lit(|s| {
        let glow = Material { emissive: [0.9, 0.9, 0.8], emissive_strength: 1.5, ..Material::named("Glow") };
        add(s, PrimitiveKind::Plane, Vec3::new(0.0, 0.0, 0.02), Vec3::splat(2.5), glow);
        let mut prism = crate::materials::presets::Preset::Crystal.material();
        prism.principled = Principled { abbe: 25.0, absorption_density: 1.5, absorption_color: [0.4, 0.8, 1.0], ..Default::default() };
        add(s, PrimitiveKind::Box, Vec3::new(0.0, 0.0, 0.3), Vec3::new(1.6, 1.6, 0.8), prism);
    });
    assert_eq!(reasons(&rs), Vec::<&str>::new());
    parity("dispersion and absorption", &rs);
}

#[test]
fn preserve_texture_appearance_glows_on_the_gpu() {
    let rs = lit(|s| {
        let sign = Material { base_color: [0.9, 0.4, 0.1], emissive: [1.0; 3], emissive_strength: 6.0, preserve_emission_texture: true, direct_camera_brightness: 0.8, ..Material::named("Sign") };
        add(s, PrimitiveKind::Box, Vec3::new(0.0, 0.0, 0.3), Vec3::splat(0.6), sign);
    });
    assert!(rs.materials.iter().any(Material::decoupled));
    assert_eq!(reasons(&rs), Vec::<&str>::new());
    assert_eq!(textures::build(&rs).rec_count, 1, "the glow brightness rides in a record");
    parity("preserve texture appearance", &rs);
}

#[test]
fn records_pack_the_principled_parameters() {
    let p = Principled { anisotropy: 0.5, aniso_rotation: 45.0, clearcoat: 0.7, sheen: 0.4, sss_weight: 0.3, abbe: 30.0, absorption_density: 2.0, absorption_color: [1.0, 0.5, 0.25], ..Default::default() };
    let m = look("All", [0.5; 3], 0.0, 0.5, p.clone());
    assert!(textures::has_record(&m) && !textures::has_record(&Material::named("Plain")));
    let r = textures::record(&m);
    assert_eq!(r.len(), REC_WORDS);
    assert_eq!(r[..8], [0.5, 45.0, 0.7, p.clearcoat_roughness, 0.4, p.sheen_tint, 0.3, 30.0]);
    let lin = crate::materials::shading::lin3(p.absorption_color);
    assert!((r[9] - lin.y.ln() * 2.0).abs() < 1e-5 && r[8].abs() < 1e-5, "Beer exponent per unit length");
    assert!(r[11] < 0.0, "no camera glow");
    assert!((r[12] - 1.0 / p.mean_free_path()).abs() < 1e-4);
    // The record index rides in each triangle's word 7; plain materials get none.
    let rs = lit(|s| add(s, PrimitiveKind::Box, Vec3::ZERO, Vec3::splat(0.5), m));
    let st = textures::build(&rs);
    let ids: Vec<u32> = (0..rs.bvh.tris.len()).map(|i| st.words[i * TRI_WORDS + 7]).collect();
    assert!(ids.contains(&0) && ids.contains(&NONE));
    assert_eq!(st.words.len() as u32, st.records + REC_WORDS as u32);
}
