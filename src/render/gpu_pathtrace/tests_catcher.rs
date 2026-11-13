//! Shadow & Reflection Catcher on the GPU: the floor turns clear except
//! where shadows, contact AO and reflections darken or cover it, and what
//! it reflects is shaded as a normal camera path, as on the CPU tracer.

use super::features::reasons;
use super::tests::{parity_with, scene_with};
use crate::render::scene_build::RenderScene;
use bevy::math::Vec3;

/// The test scene with its floor made a catcher of the given roughness.
fn caught(rough: f32) -> RenderScene {
    let mut rs = scene_with(|s| {
        s.add_light(crate::render::Light::new(crate::render::LightKind::FreeDirect), None, Vec3::new(2.0, 1.0, 3.0), Vec3::ZERO);
    });
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
