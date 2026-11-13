//! Shadow & Reflection Catcher on the GPU: the floor turns clear except
//! where shadows, contact AO and reflections darken or cover it, and what
//! it reflects is shaded as a normal camera path, as on the CPU tracer.

use super::features::reasons;
use super::tests::{parity_with, scene_seen};
use crate::viewport::camera::{ViewCamera, ViewKind};
use crate::render::scene_build::RenderScene;
use bevy::math::Vec3;

/// The test scene with its floor made a catcher of the given roughness.
fn caught(rough: f32) -> RenderScene {
    let mut c = ViewCamera::new(ViewKind::Top);
    (c.target, c.ortho_height) = (Vec3::ZERO, 4.0);
    caught_from(rough, c)
}

/// [`caught`] seen from camera `c`.
fn caught_from(rough: f32, c: ViewCamera) -> RenderScene {
    let mut rs = scene_seen(|s| {
        s.add_light(crate::render::Light::new(crate::render::LightKind::FreeDirect), None, Vec3::new(2.0, 1.0, 3.0), Vec3::ZERO);
    }, c);
    let floor = rs.materials.iter_mut().find(|m| m.name == "Floor").expect("the floor");
    (floor.shadow_catcher, floor.roughness) = (true, rough);
    rs
}

#[test]
fn a_matte_catcher_with_contact_ao_matches_the_cpu() {
    let rs = caught(0.9);
    assert!(reasons(&rs).is_empty(), "{:?}", reasons(&rs));
    parity_with("matte catcher with AO", &rs, |r| (r.gi.ao, r.gi.ao_strength, r.gi.ao_radius) = (true, 0.8, 0.6));
}

#[test]
fn a_glossy_catcher_reflects_like_the_cpu() {
    parity_with("glossy catcher", &caught(0.1), |r| r.gi.ao = false);
}

/// The glossy catcher at a grazing angle: Fresnel makes the reflection of
/// the box and the glass sphere a large part of the floor's alpha.
#[test]
fn a_glossy_catcher_at_a_grazing_angle_matches_the_cpu() {
    let mut c = ViewCamera::new(ViewKind::Perspective);
    (c.target, c.distance, c.pitch) = (Vec3::new(0.0, 0.0, 0.3), 4.5, -12f32.to_radians());
    parity_with("grazing glossy catcher", &caught_from(0.05, c), |r| r.gi.ao = false);
}
