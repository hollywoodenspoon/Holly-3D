//! LightMix: groups are assigned per light, the identity mix reproduces the
//! beauty render, and faders relight each group independently.

use super::*;
use crate::materials::Material;
use crate::primitives::{Primitive, PrimitiveKind};
use crate::render::scene_build::RenderScene;
use crate::render::{ComputeDevice, Light, LightKind};
use crate::scene::{BaseObject, Scene};
use crate::viewport::camera::{ViewCamera, ViewKind};
use std::sync::atomic::AtomicBool;

/// Floor + glowing panel lit by a warm key, a blue rim and a sun.
fn scene() -> RenderScene {
    let mut s = Scene::default();
    (s.environment.ambient_level, s.environment.background) = (0.0, [0.1, 0.1, 0.12]);
    let mut add = |kind, at: Vec3, sc: f32, m: Material| {
        let id = s.add(BaseObject::Primitive(Primitive::default_of(kind)), at);
        s.objects[id].transform.scale = Vec3::splat(sc);
        s.objects[id].material = Some(s.materials.mats.insert(m));
        s.objects[id].evaluate();
    };
    add(PrimitiveKind::Plane, Vec3::ZERO, 3.0, Material { base_color: [0.8; 3], roughness: 0.8, ..Material::named("Floor") });
    add(PrimitiveKind::Plane, Vec3::new(0.5, 0.5, 0.4), 0.3, Material { emissive: [1.0, 0.5, 0.2], emissive_strength: 3.0, ..Material::named("Glow") });
    let mut key = Light::new(LightKind::Omni);
    key.color = [1.0, 0.8, 0.6];
    s.add_light(key, Some("Key"), Vec3::new(-1.0, -1.0, 2.0), Vec3::ZERO);
    s.add_light(Light::new(LightKind::Omni), Some("Rim"), Vec3::new(1.5, 1.0, 1.5), Vec3::ZERO);
    s.add_light(Light::new(LightKind::TargetDirect), Some("Sun"), Vec3::new(2.0, -2.0, 5.0), Vec3::ZERO);
    let mut c = ViewCamera::new(ViewKind::Top);
    (c.target, c.ortho_height) = (Vec3::ZERO, 4.0);
    RenderScene::build(&s, c, false)
}

fn settings() -> RenderSettings {
    let mut r = RenderSettings { width: 16, height: 16, ..Default::default() };
    (r.path.device, r.path.samples, r.path.adaptive) = (ComputeDevice::Cpu, 16, false);
    r
}

fn render(rs: &RenderScene) -> (Vec<Pixel>, Buffers) {
    let (px, mix) = crate::render::pathtrace::progressive::render_mix(rs, &settings(), [0, 0, 16, 16], &AtomicBool::new(false), |_, _| {});
    (px, mix.expect("CPU path tracer keeps light groups"))
}

#[test]
fn lights_get_their_own_groups_and_the_sun_joins_the_sky() {
    let rs = scene();
    let name = |i: usize| rs.groups[rs.lights[i].group as usize].name.clone();
    assert_eq!(rs.groups.len(), 4 + 2, "four fixed groups + Key + Rim");
    assert!(name(0).starts_with("Key") && name(1).starts_with("Rim"));
    assert_eq!(rs.lights[2].group, SUN_SKY);
    let mut groups = fixed_groups();
    let ids: Vec<u8> = (0..11).map(|i| light_group(&mut groups, None, &format!("L{i}"))).collect();
    assert_eq!(groups.len(), GROUPS);
    assert!(ids[7..].iter().all(|g| *g as usize == GROUPS - 1), "lights past the eighth share the last group: {ids:?}");
}

#[test]
fn identity_mix_reproduces_the_beauty_render() {
    let rs = scene();
    let (px, mix) = render(&rs);
    let again = mix.composite();
    let diff = px.iter().zip(&again).map(|(a, b)| (a.display - b.display).abs().max_element()).fold(0.0, f32::max);
    assert!(diff < 2e-3, "identity LightMix matches F9: max diff {diff}");
    for g in [2usize, 4, 5] {
        assert!(mix.energy(g) > 0.0, "group {} ({}) received light", g, mix.groups[g].name);
    }
}

#[test]
fn faders_relight_each_group() {
    let rs = scene();
    let (_, mut mix) = render(&rs);
    let (key, rim) = (rs.lights[0].group as usize, rs.lights[1].group as usize);
    let geom = |m: &Buffers| m.composite().iter().map(|p| p.layers.geom.truncate()).sum::<Vec3>();
    let base = geom(&mix);
    // Solo the key: only its buffer remains (before post).
    mix.cfg.gi.denoise = false;
    mix.faders[key].solo = true;
    let solo: Vec3 = mix.composite().iter().map(|p| p.layers.geom.truncate()).sum();
    let want: Vec3 = mix.data.chunks(mix.groups.len()).map(|c| c[key]).sum();
    assert!((solo - want).abs().max_element() < want.max_element() * 1e-3 + 1e-4, "{solo} vs {want}");
    // Rim off + key doubled and tinted blue: green/red drop relative to blue.
    mix.reset();
    mix.faders[rim].on = false;
    (mix.faders[key].intensity, mix.faders[key].tint) = (2.0, [0.2, 0.2, 1.0]);
    let mixed = geom(&mix);
    assert!(mixed.z / mixed.x > base.z / base.x * 1.5, "tint shifts the balance: {base} -> {mixed}");
    mix.faders.iter_mut().for_each(|f| f.on = false);
    assert_eq!(geom(&mix), Vec3::ZERO, "every group off -> black");
}
