//! Clouds V1b: volume objects on the GPU. The packed buffer mirrors the
//! CPU's grids, and smoke boxes, puff clouds and glowing fire converge to
//! the CPU path tracer's image.

use super::features::{Features, reasons};
use super::tests::{add, parity, parity_with};
use super::volobj::{GLOW_STEPS, REC, pack};
use crate::generators::WorldGen;
use crate::generators::clouds::{CloudParams, CloudType};
use crate::materials::volume::VolumePreset;
use crate::materials::{Material, ShadingModel};
use crate::primitives::PrimitiveKind;
use crate::render::scene_build::RenderScene;
use crate::render::settings::RenderSettings;
use crate::render::{Light, LightKind};
use crate::scene::{BaseObject, Scene};
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::Vec3;

fn vol(p: VolumePreset, density: f32) -> Material {
    let mut m = Material::named("Vol");
    m.model = ShadingModel::Volume;
    m.volume = p.params();
    m.volume.density = density;
    m
}

fn floor(s: &mut Scene) {
    (s.environment.ambient_level, s.environment.background) = (0.0, [0.15, 0.18, 0.24]);
    add(s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(3.0), Material { base_color: [0.7; 3], roughness: 0.8, ..Material::named("Floor") });
}

fn seen(s: &Scene) -> RenderScene {
    let mut c = ViewCamera::new(ViewKind::Perspective);
    (c.target, c.distance) = (Vec3::new(0.0, 0.0, 0.5), 5.0);
    RenderScene::build(s, c, false)
}

fn cloud_scene() -> RenderScene {
    let mut s = Scene::default();
    floor(&mut s);
    let p = CloudParams { size: 2.0, height: 1.0, detail: 2, ..CloudParams::of(CloudType::Cumulus) };
    let id = s.add(BaseObject::World(WorldGen::Cloud(Box::new(p))), Vec3::new(0.0, 0.0, 0.6));
    s.objects[id].material = Some(s.materials.mats.insert(vol(VolumePreset::CumulusCloud, 6.0)));
    s.objects[id].evaluate();
    s.add_light(Light::new(LightKind::FreeDirect), None, Vec3::new(-2.0, -1.0, 3.0), Vec3::ZERO);
    seen(&s)
}

#[test]
fn the_packed_buffer_mirrors_the_cpu_grids() {
    let rs = cloud_scene();
    let cfg = RenderSettings::default();
    let w: Vec<f32> = pack(&rs, &cfg).chunks(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
    let v = &rs.volumes.list[0];
    assert_eq!((w[0].to_bits(), w[1], w[2]), (1, cfg.volumes.max_bounces as f32, cfg.volumes.boost));
    let r = &w[4..4 + REC];
    assert_eq!(&r[..16], &v.inv.to_cols_array());
    assert_eq!((r[19], r[23].to_bits()), (v.sigma(), 1), "extinction, puff source");
    let dims = v.grid.dims.map(|d| d as u32);
    assert_eq!([r[24], r[25], r[26]].map(f32::to_bits), dims);
    let mj = r[27].to_bits() as usize;
    assert_eq!(&w[mj..mj + v.grid.max.len()], &v.grid.max[..], "cell majorants");
    let pairs = r[34].to_bits() as usize;
    for (c, list) in v.grid.lists.iter().enumerate() {
        let (start, n) = (w[pairs + 2 * c].to_bits() as usize, w[pairs + 2 * c + 1].to_bits() as usize);
        assert_eq!(w[start..start + n].iter().map(|f| f.to_bits()).collect::<Vec<_>>(), *list, "cell {c}");
    }
    assert_eq!(r[36], 0.0, "a cloud does not glow");
    // An empty scene still binds one word; a fire glows along its ramp.
    assert_eq!(pack(&RenderScene::build(&Scene::default(), ViewCamera::new(ViewKind::Top), false), &cfg).len(), 16);
    let mut s = Scene::default();
    add(&mut s, PrimitiveKind::Box, Vec3::ZERO, Vec3::ONE, vol(VolumePreset::Fire, 2.0));
    let fire = pack(&seen(&s), &cfg);
    let f = |i: usize| f32::from_le_bytes(fire[4 * i..4 * i + 4].try_into().unwrap());
    assert_eq!((f(4 + 23).to_bits(), f(4 + 36)), (0, 1.0), "a box is uniform, fire glows");
    let hot = 4 + 37 + 3 * (GLOW_STEPS - 1);
    assert!(f(hot) > f(4 + 37), "hotter where denser");
}

#[test]
fn puff_and_box_volumes_turn_on_the_gpu_switch() {
    let rs = cloud_scene();
    assert!(reasons(&rs).is_empty(), "{:?}", reasons(&rs));
    assert_ne!(Features::of(&rs).0 & Features::VOLOBJ, 0);
    assert_eq!(Features::of(&seen(&Scene::default())).0 & Features::VOLOBJ, 0);
}

/// A sunlit cumulus over a floor: puff densities, multiple scattering with
/// the boost, and the cloud's shadow on the ground.
#[test]
fn a_sunlit_cloud_renders_on_the_gpu() {
    parity("sunlit cumulus", &cloud_scene());
}

/// Absorbing grey smoke in a box (exact Beer-Lambert shadows) lit by an
/// omni, with single scattering only and Smooth shadows.
#[test]
fn smoke_box_renders_on_the_gpu() {
    let mut s = Scene::default();
    floor(&mut s);
    add(&mut s, PrimitiveKind::Box, Vec3::new(0.2, 0.0, 0.0), Vec3::splat(0.9), vol(VolumePreset::GreySmoke, 2.5));
    s.add_light(Light::new(LightKind::Omni), None, Vec3::new(-1.2, -1.0, 2.2), Vec3::ZERO);
    parity_with("grey smoke box", &seen(&s), |r| {
        r.volumes.max_bounces = 1;
        r.volumes.shadow_quality = crate::render::volumes::ShadowQuality::Smooth;
    });
}

/// A glowing fire volume with a dim spot: emission along the ramp.
#[test]
fn fire_glow_renders_on_the_gpu() {
    let mut s = Scene::default();
    floor(&mut s);
    add(&mut s, PrimitiveKind::Box, Vec3::ZERO, Vec3::splat(0.7), vol(VolumePreset::Fire, 3.0));
    let mut spot = Light::new(LightKind::FreeSpot);
    spot.multiplier = 0.5;
    s.add_light(spot, None, Vec3::new(1.0, -1.0, 2.5), Vec3::ZERO);
    parity("fire glow", &seen(&s));
}
