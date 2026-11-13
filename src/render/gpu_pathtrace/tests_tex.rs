//! PR 1b: texture and procedural maps on the GPU. Maps are baked over a
//! UV grid (cached), looked up per hit, tilt normals, project from three
//! sides for World / Triplanar materials, and 3D-only maps stay on the CPU.

use super::bake::{self, Space};
use super::features::{CPU_ONLY, Features, reasons};
use super::tests::{add, parity};
use super::textures;
use crate::materials::maps::{MapChannel, MapNode, MapSlot};
use crate::materials::patterns::BrickKind;
use crate::materials::texture::{TexHandle, Texture};
use crate::materials::triplanar::Projection;
use crate::materials::{Material, proc_engine::Pattern};
use crate::primitives::PrimitiveKind;
use crate::render::scene_build::RenderScene;
use crate::render::{Light, LightKind};
use crate::scene::Scene;
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::Vec3;
use std::sync::Arc;

fn with(mut m: Material, ch: MapChannel, node: MapNode) -> Material {
    m.maps[ch as usize] = MapSlot { node: Some(node), ..Default::default() };
    m
}

fn gradient() -> MapNode {
    MapNode::Gradient { color_a: [0.9, 0.2, 0.1], color_b: [0.2, 0.8, 0.3], color_c: [0.1, 0.3, 0.9], mid: 0.5, radial: false }
}

fn checker(tiles: f32) -> MapNode {
    MapNode::Checker { tiles, color_a: [0.95; 3], color_b: [0.05; 3] }
}

fn bricks() -> MapNode {
    MapNode::Bricks { kind: BrickKind::Running, cols: 4.0, rows: 8.0, gap: 0.06, vary: 0.3, seed: 3, color_a: [0.7, 0.3, 0.2], color_b: [0.8; 3] }
}

fn lit(build: impl FnOnce(&mut Scene)) -> RenderScene {
    let mut s = Scene::default();
    (s.environment.ambient_level, s.environment.background) = (0.0, [0.2, 0.25, 0.3]);
    build(&mut s);
    s.add_light(Light::new(LightKind::Omni), None, Vec3::new(-1.0, -1.0, 2.5), Vec3::ZERO);
    let mut c = ViewCamera::new(ViewKind::Top);
    (c.target, c.ortho_height) = (Vec3::ZERO, 4.0);
    RenderScene::build(&s, c, false)
}

/// A gradient floor (clamped, not repeating), a brick box whose roughness
/// and metalness come from a checker, and a glowing checker panel.
fn uv_scene() -> RenderScene {
    lit(|s| {
        add(s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(3.0), with(Material::named("Floor"), MapChannel::BaseColor, gradient()));
        let rough = with(with(Material::named("Box"), MapChannel::BaseColor, bricks()), MapChannel::Roughness, checker(3.0));
        add(s, PrimitiveKind::Box, Vec3::new(-0.6, 0.3, 0.0), Vec3::splat(0.6), with(rough, MapChannel::Metallic, checker(3.0)));
        let glow = Material { emissive: [1.0, 0.9, 0.7], emissive_strength: 3.0, ..Material::named("Glow") };
        add(s, PrimitiveKind::Plane, Vec3::new(0.7, 0.8, 0.8), Vec3::splat(0.4), with(glow, MapChannel::Emission, checker(2.0)));
    })
}

#[test]
fn uv_maps_render_on_the_gpu_like_the_cpu() {
    let rs = uv_scene();
    assert_eq!(reasons(&rs), Vec::<&str>::new());
    assert_ne!(Features::of(&rs).0 & Features::TEXTURES, 0);
    let store = textures::build(&rs);
    assert_eq!(store.count, 3, "floor, box and panel each get a texture");
    assert_eq!(store.table as usize, rs.bvh.tris.len() * textures::TRI_WORDS);
    parity("UV maps", &rs);
}

#[test]
fn triplanar_procedural_bricks_with_relief_render_on_the_gpu() {
    let rs = lit(|s| {
        add(s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(3.0), Material { base_color: [0.6; 3], ..Material::named("Floor") });
        let mut m = Material::named("Wall");
        (m.proc.pattern, m.proc.auto_pbr, m.proc.relief) = (Pattern::Bricks, true, 0.8);
        m.projection = Projection::triplanar(0.5);
        add(s, PrimitiveKind::Box, Vec3::new(0.2, 0.1, 0.0), Vec3::new(1.2, 0.8, 0.9), m);
    });
    let wall = rs.materials.iter().find(|m| m.projection.on()).expect("the wall");
    assert_eq!(bake::why_not(wall), None);
    assert!(bake::bake(wall, Space::UNIT).normal, "the procedural relief tilts normals");
    parity("triplanar bricks", &rs);
}

#[test]
fn procedural_surfaces_render_on_the_gpu() {
    let rs = lit(|s| {
        let mut m = Material::named("Planks");
        (m.proc.pattern, m.proc.auto_pbr, m.proc.relief) = (Pattern::Wood, true, 0.6);
        add(s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(3.0), m);
    });
    assert_eq!(reasons(&rs), Vec::<&str>::new());
    parity("procedural wood", &rs);
}

#[test]
fn position_based_maps_stay_on_the_cpu() {
    let cracks = MapNode::Cracks { scale: 2.0, width: 0.05, vary: 0.5, seed: 1, color_a: [0.8; 3], color_b: [0.1; 3] };
    let m = with(Material::named("Cracked"), MapChannel::BaseColor, cracks);
    assert!(bake::position_based(&m));
    assert_eq!(bake::why_not(&m), Some("3D (position-based) procedural maps"));
    let rs = lit(|s| add(s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(3.0), m));
    assert_eq!(reasons(&rs), [CPU_ONLY[6]]);
    // A UV map is fine, and so is a triplanar one that repeats.
    assert_eq!(bake::why_not(&with(Material::named("Uv"), MapChannel::BaseColor, gradient())), None);
    let mut tri = with(Material::named("Tri"), MapChannel::BaseColor, gradient());
    tri.projection = Projection::triplanar(1.0);
    assert!(!bake::periodic(&tri));
    assert_eq!(bake::why_not(&tri), Some("non-repeating maps on World / Triplanar materials"));
}

#[test]
fn bakes_follow_bitmap_sizes_and_are_cached() {
    let tex = TexHandle { tex: Arc::new(Texture { width: 32, height: 16, data: vec![[200, 100, 50, 255]; 32 * 16] }), rev: 0 };
    let m = with(Material::named("Bitmap"), MapChannel::BaseColor, MapNode::Bitmap { path: "a.png".into(), tex: Some(tex) });
    assert_eq!(bake::resolution(&m), (32, 16));
    assert_eq!(bake::bytes(&m), 32 * 16 * 24);
    let a = bake::bake(&m, Space::UNIT);
    let b = bake::bake(&m, Space::UNIT);
    assert!(Arc::ptr_eq(&a.words, &b.words), "the second bake is a cache hit");
    assert!(!a.normal);
    // Texel (0,0): sRGB (200,100,50) to linear, alpha 1.
    let lo = half::f16::from_bits(a.words[0] as u16).to_f32();
    let alpha = half::f16::from_bits((a.words[1] >> 16) as u16).to_f32();
    assert!((lo - crate::materials::shading::lin(Vec3::new(200.0, 100.0, 50.0) / 255.0).x).abs() < 0.01, "{lo}");
    assert!((alpha - 1.0).abs() < 1e-3);
    assert_eq!(bake::resolution(&with(Material::named("P"), MapChannel::BaseColor, checker(2.0))), (bake::PROC_RES, bake::PROC_RES));
}

#[test]
fn untextured_scenes_carry_an_empty_store() {
    let rs = lit(|s| add(s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(3.0), Material::named("Plain")));
    let st = textures::build(&rs);
    assert_eq!((st.count, st.words.len()), (0, 4));
    assert_eq!(Features::of(&rs).0 & Features::TEXTURES, 0);
}
