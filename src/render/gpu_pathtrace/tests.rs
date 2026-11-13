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

pub(super) fn add(s: &mut Scene, kind: PrimitiveKind, at: Vec3, scale: Vec3, mat: Material) {
    let id = s.add(BaseObject::Primitive(Primitive::default_of(kind)), at);
    s.objects[id].transform.scale = scale;
    s.objects[id].material = Some(s.materials.mats.insert(mat));
    s.objects[id].evaluate();
}

/// Floor, a glossy box, a glass sphere, an omni light and a glowing panel.
fn scene() -> RenderScene {
    scene_with(|_| {})
}

pub(super) fn scene_with(extra: impl FnOnce(&mut Scene)) -> RenderScene {
    let mut s = Scene::default();
    (s.environment.ambient_level, s.environment.background) = (0.0, [0.2, 0.25, 0.3]);
    add(&mut s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(3.0), Material { base_color: [0.8; 3], roughness: 0.9, ..Material::named("Floor") });
    add(&mut s, PrimitiveKind::Box, Vec3::new(-0.6, 0.3, 0.0), Vec3::splat(0.6), Material { base_color: [0.9, 0.3, 0.2], metallic: 0.6, roughness: 0.35, ..Material::named("Box") });
    let glass = crate::materials::presets::Preset::Crystal.material();
    add(&mut s, PrimitiveKind::Sphere, Vec3::new(0.7, -0.4, 0.3), Vec3::splat(0.6), glass);
    let glow = Material { emissive: [1.0, 0.9, 0.7], emissive_strength: 3.0, ..Material::named("Glow") };
    add(&mut s, PrimitiveKind::Plane, Vec3::new(0.6, 0.8, 0.8), Vec3::splat(0.3), glow);
    s.add_light(Light::new(LightKind::Omni), None, Vec3::new(-1.0, -1.0, 2.5), Vec3::ZERO);
    extra(&mut s);
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

/// How far the GPU image is from the CPU one: mean absolute error of the
/// 6x6 block means, and the mean signed difference per channel (a bias
/// shows up there even when noise hides it in single pixels).
pub(super) struct Diff {
    pub mean_err: f32,
    bias: Vec3,
    energy: (f32, f32),
}

fn diff(gpu: &[Pixel], cpu: &[Pixel]) -> Diff {
    let (a, b) = (block_means(gpu), block_means(cpu));
    let mean_err = a.iter().zip(&b).map(|(x, y)| (*x - *y).abs().element_sum() / 3.0).sum::<f32>() / a.len() as f32;
    let bias = a.iter().zip(&b).map(|(x, y)| *x - *y).sum::<Vec3>() / a.len() as f32;
    let total = |v: &[Vec3]| v.iter().map(|c| c.element_sum()).sum::<f32>();
    Diff { mean_err, bias, energy: (total(&a), total(&b)) }
}

/// Renders `rs` on both tracers and checks they match (`None` = no adapter).
pub(super) fn parity(name: &str, rs: &RenderScene) -> Option<Diff> {
    let g = match gpu() {
        Ok(g) => g,
        Err(e) if std::env::var_os("MINIMAX_REQUIRE_GPU").is_none() => {
            eprintln!("skipping GPU parity ({name}): {e}");
            return None;
        }
        Err(e) => panic!("{e}"),
    };
    eprintln!("{}", g.name);
    assert_eq!(pack::unsupported(rs), None, "{name} renders on the GPU");
    let r = settings(ComputeDevice::Gpu);
    let mut passes = 0;
    let gpu_px = super::render(g, rs, &r, [0, 0, 24, 24], &AtomicBool::new(false), |_, _| passes += 1).expect("GPU render");
    assert_eq!(passes, r.path.schedule().len(), "progressive passes stream from the GPU too");
    let (cpu_px, label) = pathtrace::run(rs, &settings(ComputeDevice::Cpu), [0, 0, 24, 24], &AtomicBool::new(false), |_, _| {});
    assert_eq!(label, "CPU");
    let d = diff(&gpu_px, &cpu_px);
    let (eg, ec) = d.energy;
    eprintln!("{name}: energy GPU {eg} CPU {ec}, mean error {}, bias {}", d.mean_err, d.bias);
    assert!(ec > 1.0, "{name}: the scene is lit");
    assert!((eg - ec).abs() < 0.05 * ec, "{name}: image energy GPU {eg} vs CPU {ec}");
    let level = ec / 48.0;
    assert!(d.mean_err < 0.02 + 0.1 * level, "{name}: mean block error {}", d.mean_err);
    assert!(d.bias.abs().max_element() < 0.01 + 0.04 * level, "{name}: per-channel bias {}", d.bias);
    let (a, b) = (block_means(&gpu_px), block_means(&cpu_px));
    for (i, (x, y)) in a.iter().zip(&b).enumerate() {
        assert!((*x - *y).abs().max_element() < 0.08 + 0.12 * y.max_element(), "{name} block {i}: GPU {x} vs CPU {y}");
    }
    let cov = |px: &[Pixel]| px.iter().map(|p| p.layers.geom.w).sum::<f32>();
    assert!((cov(&gpu_px) - cov(&cpu_px)).abs() < 1.0, "{name}: same coverage");
    Some(d)
}

#[test]
fn gpu_and_cpu_path_tracers_agree() {
    parity("glass, metal and emitter", &scene());
}

/// Spot and sun lights through exponential fog, with a dim ambient.
#[test]
fn gpu_and_cpu_agree_on_spot_sun_and_fog() {
    let mut s = Scene::default();
    (s.environment.ambient_level, s.environment.background) = (0.05, [0.1, 0.12, 0.15]);
    add(&mut s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(3.0), Material { base_color: [0.7, 0.75, 0.8], roughness: 0.6, ..Material::named("Floor") });
    add(&mut s, PrimitiveKind::Cylinder, Vec3::new(0.4, 0.2, 0.0), Vec3::splat(0.5), Material { base_color: [0.2, 0.6, 0.9], roughness: 0.2, ..Material::named("Can") });
    let mut spot = Light::new(LightKind::FreeSpot);
    spot.multiplier = 2.0;
    s.add_light(spot, None, Vec3::new(-0.5, -0.5, 2.5), Vec3::ZERO);
    s.add_light(Light::new(LightKind::FreeDirect), None, Vec3::new(1.0, -1.0, 3.0), Vec3::ZERO);
    (s.environment.fog, s.environment.fog_color, s.environment.fog_density) = (true, [0.6, 0.6, 0.7], 0.06);
    // Perspective and close in: the far ortho eye would see only fog.
    let mut c = ViewCamera::new(ViewKind::Perspective);
    (c.target, c.distance) = (Vec3::ZERO, 6.0);
    parity("spot, sun and fog", &RenderScene::build(&s, c, false));
}

#[test]
fn scenes_the_gpu_cannot_draw_fall_back_to_the_cpu() {
    let mut rs = scene_with(|s| {
        s.add_light(Light::new(LightKind::DiscArea), None, Vec3::new(0.0, 0.0, 2.0), Vec3::ZERO);
    });
    rs.translucent = true;
    assert_eq!(super::features::reasons(&rs), ["area lights", "see-through (opacity) surfaces"]);
    let mut r = settings(ComputeDevice::Gpu);
    r.path.samples = 2;
    let (px, label) = pathtrace::run(&rs, &r, [0, 0, 24, 24], &AtomicBool::new(false), |_, _| {});
    assert_eq!(px.len(), 24 * 24);
    assert_eq!(label, "CPU (area lights and see-through (opacity) surfaces need the CPU tracer)");
}
