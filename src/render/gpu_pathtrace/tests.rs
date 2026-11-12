//! GPU / CPU parity: the WGSL kernel and the CPU path tracer converge to
//! the same image (skipped, with the reason printed, without a GPU adapter;
//! `MINIMAX_REQUIRE_GPU=1` makes that a failure), plus the CPU fallback.

use super::{gpu, pack};
use crate::render::{Light, LightKind};
use crate::materials::Material;
use crate::primitives::{Primitive, PrimitiveKind};
use crate::render::scene_build::RenderScene;
use crate::render::settings::RenderSettings;
use crate::render::trace::Pixel;
use crate::render::{ComputeDevice, pathtrace};
use crate::scene::{BaseObject, Scene};
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::Vec3;
use std::sync::atomic::AtomicBool;

fn add(s: &mut Scene, kind: PrimitiveKind, at: Vec3, scale: Vec3, mat: Material) {
    let id = s.add(BaseObject::Primitive(Primitive::default_of(kind)), at);
    s.objects[id].transform.scale = scale;
    s.objects[id].material = Some(s.materials.mats.insert(mat));
    s.objects[id].evaluate();
}

/// Floor, a glossy box, a glass sphere, an omni light and a glowing panel.
fn scene() -> RenderScene {
    let mut s = Scene::default();
    (s.environment.ambient_level, s.environment.background) = (0.0, [0.2, 0.25, 0.3]);
    add(&mut s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(3.0), Material { base_color: [0.8; 3], roughness: 0.9, ..Material::named("Floor") });
    add(&mut s, PrimitiveKind::Box, Vec3::new(-0.6, 0.3, 0.0), Vec3::splat(0.6), Material { base_color: [0.9, 0.3, 0.2], metallic: 0.6, roughness: 0.35, ..Material::named("Box") });
    let glass = crate::materials::presets::Preset::Crystal.material();
    add(&mut s, PrimitiveKind::Sphere, Vec3::new(0.7, -0.4, 0.3), Vec3::splat(0.6), glass);
    let glow = Material { emissive: [1.0, 0.9, 0.7], emissive_strength: 3.0, ..Material::named("Glow") };
    add(&mut s, PrimitiveKind::Plane, Vec3::new(0.6, 0.8, 0.8), Vec3::splat(0.3), glow);
    s.add_light(Light::new(LightKind::Omni), None, Vec3::new(-1.0, -1.0, 2.5), Vec3::ZERO);
    let mut c = ViewCamera::new(ViewKind::Top);
    (c.target, c.ortho_height) = (Vec3::ZERO, 4.0);
    RenderScene::build(&s, c, false)
}

fn settings(device: ComputeDevice) -> RenderSettings {
    let mut r = RenderSettings { width: 24, height: 24, outlines: false, ..Default::default() };
    (r.path.device, r.path.samples, r.path.adaptive) = (device, 256, false);
    (r.gi.denoise, r.post.bloom) = (false, false);
    r
}

fn block_means(px: &[Pixel]) -> Vec<Vec3> {
    (0..16).map(|b| {
        let (bx, by) = (b % 4 * 6, b / 4 * 6);
        (0..36).map(|i| px[(by + i / 6) * 24 + bx + i % 6].layers.geom.truncate()).sum::<Vec3>() / 36.0
    }).collect()
}

#[test]
fn gpu_and_cpu_path_tracers_agree() {
    let g = match gpu() {
        Ok(g) => g,
        Err(e) if std::env::var_os("MINIMAX_REQUIRE_GPU").is_none() => return eprintln!("skipping GPU parity: {e}"),
        Err(e) => panic!("{e}"),
    };
    eprintln!("{}", g.name);
    let rs = scene();
    assert_eq!(pack::unsupported(&rs), None);
    let r = settings(ComputeDevice::Gpu);
    let mut passes = 0;
    let gpu_px = super::render(g, &rs, &r, [0, 0, 24, 24], &AtomicBool::new(false), |_, _| passes += 1).expect("GPU render");
    assert_eq!(passes, r.path.schedule().len(), "progressive passes stream from the GPU too");
    let (cpu_px, label) = pathtrace::run(&rs, &settings(ComputeDevice::Cpu), [0, 0, 24, 24], &AtomicBool::new(false), |_, _| {});
    assert_eq!(label, "CPU");
    let (a, b) = (block_means(&gpu_px), block_means(&cpu_px));
    let total = |v: &[Vec3]| v.iter().map(|c| c.x + c.y + c.z).sum::<f32>();
    eprintln!("energy GPU {} CPU {}", total(&a), total(&b));
    assert!(total(&b) > 1.0, "the scene is lit");
    assert!((total(&a) - total(&b)).abs() < 0.05 * total(&b), "image energy: GPU {} vs CPU {}", total(&a), total(&b));
    for (i, (x, y)) in a.iter().zip(&b).enumerate() {
        assert!((*x - *y).abs().max_element() < 0.08 + 0.12 * y.max_element(), "block {i}: GPU {x} vs CPU {y}");
    }
    let cov = |px: &[Pixel]| px.iter().map(|p| p.layers.geom.w).sum::<f32>();
    assert!((cov(&gpu_px) - cov(&cpu_px)).abs() < 1.0, "same coverage");
}

#[test]
fn scenes_the_gpu_cannot_draw_fall_back_to_the_cpu() {
    let mut rs = scene();
    rs.translucent = true;
    assert_eq!(pack::unsupported(&rs), Some("see-through (opacity) surfaces"));
    let mut r = settings(ComputeDevice::Gpu);
    r.path.samples = 2;
    let (px, label) = pathtrace::run(&rs, &r, [0, 0, 24, 24], &AtomicBool::new(false), |_, _| {});
    assert_eq!(px.len(), 24 * 24);
    assert!(label.starts_with("CPU (see-through"), "{label}");
}
