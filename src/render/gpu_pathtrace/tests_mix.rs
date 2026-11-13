//! LightMix on the GPU: the kernel splits light into the same groups as the
//! CPU tracer (each scene light, sun & sky, environment and fog, emissive,
//! volume), so the Frame Window can relight a GPU render too.

use super::tests::add;
use super::{features, gpu};
use crate::materials::Material;
use crate::primitives::PrimitiveKind;
use crate::render::lightmix::{Buffers, EMISSIVE, ENV, FIRST_LIGHT, SUN_SKY, VOLUME};
use crate::render::scene_build::RenderScene;
use crate::render::settings::RenderSettings;
use crate::render::sky::SkyMode;
use crate::render::volumetrics::VolumeSettings;
use crate::render::{ComputeDevice, Light, LightKind, pathtrace};
use crate::scene::Scene;
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::Vec3;
use std::sync::atomic::AtomicBool;

const SIZE: u32 = 24;

fn settings(device: ComputeDevice) -> RenderSettings {
    let mut r = RenderSettings { width: SIZE, height: SIZE, outlines: false, ..Default::default() };
    (r.path.device, r.path.samples, r.path.adaptive, r.path.lightmix) = (device, 256, false, true);
    (r.gi.denoise, r.post.bloom) = (false, false);
    r
}

/// Image mean of every group.
fn means(m: &Buffers) -> Vec<Vec3> {
    let k = m.groups.len();
    let n = (m.data.len() / k) as f32;
    (0..k).map(|g| m.data.iter().skip(g).step_by(k).sum::<Vec3>() / n).collect()
}

/// Renders `rs` on both tracers and checks every group matches.
fn mix_parity(name: &str, rs: &RenderScene) -> Option<Vec<Vec3>> {
    let g = match gpu() {
        Ok(g) => g,
        Err(e) if std::env::var_os("MINIMAX_REQUIRE_GPU").is_none() => {
            eprintln!("skipping GPU LightMix ({name}): {e}");
            return None;
        }
        Err(e) => panic!("{e}"),
    };
    assert!(features::reasons(rs).is_empty(), "{name} renders on the GPU");
    let region = [0, 0, SIZE, SIZE];
    let (px, label, extras) = pathtrace::run_mix(rs, &settings(ComputeDevice::Gpu), region, &AtomicBool::new(false), |_, _| {});
    assert_eq!(label, g.name, "{name}: rendered on the GPU");
    assert_eq!(px.len(), (SIZE * SIZE) as usize);
    let gm = extras.lightmix.expect("the GPU keeps light groups");
    let (_, _, cpu) = pathtrace::run_mix(rs, &settings(ComputeDevice::Cpu), region, &AtomicBool::new(false), |_, _| {});
    let cm = cpu.lightmix.expect("CPU light groups");
    assert_eq!(gm.groups, cm.groups, "{name}: same groups");
    assert_eq!((gm.region, gm.data.len()), (cm.region, cm.data.len()));
    let (a, b) = (means(&gm), means(&cm));
    let total: f32 = b.iter().map(|c| c.element_sum()).sum();
    assert!(total > 0.05, "{name}: the scene is lit");
    for (i, (x, y)) in a.iter().zip(&b).enumerate() {
        eprintln!("{name} group {i} ({}): GPU {x} CPU {y}", gm.groups[i].name);
        let tol = 0.04 * y.max_element().max(0.02 * total) + 1e-3;
        assert!((*x - *y).abs().max_element() < tol, "{name} group {i} ({}): GPU {x} vs CPU {y}", gm.groups[i].name);
    }
    // The G-buffer and coverage the relight's post chain needs come along too.
    // Edge pixels are a 256-sample coverage estimate: compare them loosely, the sum tightly.
    assert!(gm.alpha.iter().zip(&cm.alpha).all(|(x, y)| (x - y).abs() < 0.15), "{name}: same coverage per pixel");
    let sum = |a: &[f32]| a.iter().sum::<f32>();
    assert!((sum(&gm.alpha) - sum(&cm.alpha)).abs() < 1.0, "{name}: same coverage");
    assert!(gm.g.iter().zip(&cm.g).all(|(x, y)| x.obj == y.obj), "{name}: same G-buffer");
    Some(a)
}

fn lit(v: Vec3) -> bool {
    v.max_element() > 1e-3
}

/// Two omnis of different colours, a sun, a glowing panel and fog.
#[test]
fn light_groups_match_the_cpu_on_the_gpu() {
    let mut s = Scene::default();
    (s.environment.ambient_level, s.environment.background) = (0.0, [0.1, 0.1, 0.12]);
    add(&mut s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(3.0), Material { base_color: [0.8; 3], roughness: 0.8, ..Material::named("Floor") });
    add(&mut s, PrimitiveKind::Box, Vec3::new(-0.5, 0.3, 0.0), Vec3::splat(0.6), Material { base_color: [0.9, 0.4, 0.2], metallic: 0.5, roughness: 0.3, ..Material::named("Box") });
    let glow = Material { emissive: [1.0, 0.5, 0.2], emissive_strength: 3.0, ..Material::named("Glow") };
    add(&mut s, PrimitiveKind::Plane, Vec3::new(0.6, 0.6, 0.5), Vec3::splat(0.3), glow);
    let mut key = Light::new(LightKind::Omni);
    key.color = [1.0, 0.2, 0.1];
    s.add_light(key, Some("Key"), Vec3::new(-1.0, -1.0, 2.0), Vec3::ZERO);
    let mut rim = Light::new(LightKind::Omni);
    rim.color = [0.1, 0.3, 1.0];
    s.add_light(rim, Some("Rim"), Vec3::new(1.5, 1.0, 1.5), Vec3::ZERO);
    s.add_light(Light::new(LightKind::TargetDirect), Some("Sun"), Vec3::new(2.0, -2.0, 5.0), Vec3::ZERO);
    (s.environment.fog, s.environment.fog_color, s.environment.fog_density) = (true, [0.6, 0.6, 0.7], 0.05);
    let mut c = ViewCamera::new(ViewKind::Perspective);
    (c.target, c.distance) = (Vec3::ZERO, 6.0);
    let rs = RenderScene::build(&s, c, false);
    let Some(m) = mix_parity("omnis, sun, glow and fog", &rs) else { return };
    let (key, rim) = (m[FIRST_LIGHT as usize], m[FIRST_LIGHT as usize + 1]);
    assert!(key.x > 3.0 * key.z && rim.z > 2.0 * rim.x, "each light keeps its own colour: key {key}, rim {rim}");
    assert!(lit(m[SUN_SKY as usize]) && lit(m[ENV as usize]) && lit(m[EMISSIVE as usize]), "sun, fog and glow groups: {m:?}");
    assert!(!lit(m[VOLUME as usize]), "no mist, no volume light");
}

/// Mist under the procedural sky, an area panel, a spot and a glowing
/// sphere light the camera sees (a panel facing down shows the camera its back).
#[test]
fn area_mist_and_sky_groups_match_the_cpu_on_the_gpu() {
    let mut s = Scene::default();
    s.environment.ambient_level = 0.0;
    s.environment.sky.mode = SkyMode::Earth;
    add(&mut s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(3.0), Material { base_color: [0.6; 3], roughness: 0.8, ..Material::named("Floor") });
    let mut panel = Light::new(LightKind::RectArea);
    (panel.multiplier, panel.area_size, panel.visible_to_camera) = (2.0, [0.6, 0.4], true);
    s.add_light(panel, Some("Panel"), Vec3::new(-0.4, 0.2, 1.2), Vec3::ZERO);
    let mut spot = Light::new(LightKind::FreeSpot);
    spot.multiplier = 2.0;
    s.add_light(spot, Some("Spot"), Vec3::new(0.6, -0.4, 2.2), Vec3::ZERO);
    let mut bulb = Light::new(LightKind::SphereArea);
    (bulb.multiplier, bulb.area_size, bulb.visible_to_camera) = (2.0, [0.2, 0.2], true);
    s.add_light(bulb, Some("Bulb"), Vec3::new(0.5, 0.5, 0.8), Vec3::ZERO);
    s.environment.volume = VolumeSettings { enabled: true, density: 0.3, intensity: 2.0, ..Default::default() };
    let mut c = ViewCamera::new(ViewKind::Perspective);
    (c.target, c.distance) = (Vec3::new(0.0, 0.0, 0.6), 6.0);
    let rs = RenderScene::build(&s, c, false);
    assert!(rs.procedural.is_some() && rs.volume.is_some());
    let Some(m) = mix_parity("area panel, spot and bulb in mist under the sky", &rs) else { return };
    let light = |i: usize| m[FIRST_LIGHT as usize + i];
    assert!(lit(m[VOLUME as usize]) && lit(light(0)) && lit(light(1)) && lit(light(2)), "mist, panel, spot and bulb groups: {m:?}");
}

#[test]
fn lightmix_off_keeps_no_groups_and_a_one_word_buffer() {
    assert_eq!(super::mix::bytes(100, 0), 4);
    assert_eq!(super::mix::bytes(100, 6), 100 * 6 * 3 * 4);
    let Ok(_) = gpu() else { return };
    let mut s = Scene::default();
    add(&mut s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(3.0), Material::named("Floor"));
    s.add_light(Light::new(LightKind::Omni), None, Vec3::new(0.0, 0.0, 2.0), Vec3::ZERO);
    let rs = RenderScene::build(&s, ViewCamera::new(ViewKind::Top), false);
    let mut r = settings(ComputeDevice::Gpu);
    (r.path.samples, r.path.lightmix) = (2, false);
    let (_, _, extras) = pathtrace::run_mix(&rs, &r, [0, 0, SIZE, SIZE], &AtomicBool::new(false), |_, _| {});
    assert!(extras.lightmix.is_none());
}
