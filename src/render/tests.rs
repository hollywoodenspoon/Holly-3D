use super::environment::apply_preset;
use super::lighting::*;
use super::*;
use bevy::math::Vec3;

#[test]
fn kelvin_runs_warm_to_cool() {
    let candle = kelvin_to_rgb(1900.0);
    let sky = kelvin_to_rgb(10000.0);
    assert!(candle[0] > candle[2] + 0.5, "candle light is orange");
    assert!(sky[2] > sky[0], "blue sky is blue");
    let d = kelvin_to_rgb(6500.0);
    assert!(d.iter().all(|c| *c > 0.9), "6500K is near white: {d:?}");
    for (_, k) in KELVIN_PRESETS {
        assert!(kelvin_to_rgb(k).iter().cloned().fold(0.0, f32::max) > 0.999);
    }
}

#[test]
fn aim_points_negative_z_at_the_target() {
    for (from, to) in [
        (Vec3::new(3.0, -4.0, 5.0), Vec3::ZERO),
        (Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO), // straight down (up vector fallback)
        (Vec3::new(-1.0, 2.0, 0.0), Vec3::new(4.0, 2.0, 0.0)),
    ] {
        let q = aim(from, to);
        let d = emit_dir(q);
        assert!(d.distance((to - from).normalize()) < 1e-4, "{from} -> {to}: {d}");
        assert!((q.length() - 1.0).abs() < 1e-4);
    }
    assert_eq!(emit_dir(bevy::math::Quat::IDENTITY), Vec3::NEG_Z, "free lights default to shining down");
}

#[test]
fn cone_and_radiance_rules() {
    let mut l = Light::new(LightKind::FreeSpot);
    l.hotspot = 80.0;
    l.falloff = 60.0;
    l.clamp_cone();
    assert!(l.hotspot < l.falloff);
    l.color = [1.0, 1.0, 1.0];
    l.multiplier = 2.0;
    assert_eq!(l.radiance(), Vec3::splat(2.0));
    l.on = false;
    assert_eq!(l.radiance(), Vec3::ZERO);
    assert!(LightKind::TargetSpot.has_target() && !LightKind::FreeSpot.has_target());
    assert!(!LightKind::Skylight.has_range() && LightKind::Omni.has_range());
}

#[test]
fn headlight_rig_follows_the_camera() {
    let (f, r, u) = (Vec3::Y, Vec3::X, Vec3::Z);
    let [key, fill] = headlight_rig(f, r, u);
    assert!(key.dir.dot(f) > 0.7, "key shines along the view");
    assert!(key.dir.z < 0.0 && key.dir.x > 0.0, "key comes from above-left of the eye");
    assert!(fill.dir.x < 0.0 && fill.strength < key.strength, "fill comes from the right");
    assert!(fill.color.z > fill.color.x, "fill is cool");
    let [k2, _] = headlight_rig(-Vec3::X, Vec3::Y, Vec3::Z);
    assert!(k2.dir.dot(-Vec3::X) > 0.7, "rig turns with the camera");
}

#[test]
fn presets_place_lights_around_the_scene() {
    let mut env = Environment::default();
    let c = Vec3::new(1.0, 2.0, 0.5);
    let three = apply_preset(&mut env, LightingPreset::ThreePoint, c, 2.0);
    assert_eq!(three.len(), 3);
    assert!(three.iter().all(|p| p.light.from_preset && p.position.distance(c) > 2.0));
    assert!(three.iter().any(|p| p.light.kind.has_target()));
    let day = apply_preset(&mut env, LightingPreset::Daylight, c, 2.0);
    assert!(day.iter().any(|p| p.light.kind == LightKind::Skylight));
    assert!(env.background[2] > env.background[0], "daylight sky background");
    assert!(apply_preset(&mut env, LightingPreset::PitchBlack, c, 2.0).is_empty());
    assert!(!env.auto_default_lights && env.ambient_level == 0.0 && env.background == [0.0; 3]);
    apply_preset(&mut env, LightingPreset::DefaultHeadlight, c, 2.0);
    assert!(env.auto_default_lights);
}

#[test]
fn near_and_far_attenuation_ramps() {
    let mut l = Light::new(LightKind::Omni);
    assert_eq!(l.atten().factor(1000.0), 1.0, "no attenuation: full everywhere");
    (l.use_near, l.near_start, l.near_end) = (true, 1.0, 3.0);
    (l.use_atten, l.atten_start, l.atten_end) = (true, 10.0, 20.0);
    let a = l.atten();
    let f = |d: f32| a.factor(d);
    assert_eq!((f(0.5), f(2.0), f(5.0), f(15.0), f(25.0)), (0.0, 0.5, 1.0, 0.5, 0.0));
    // Directional lights and the Skylight have no range.
    let mut d = Light::new(LightKind::FreeDirect);
    d.use_near = true;
    assert_eq!(d.atten(), Default::default());
}
