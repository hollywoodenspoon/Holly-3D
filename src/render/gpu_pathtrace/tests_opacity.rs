//! See-through surfaces on the GPU: paths pass through with probability
//! 1 - alpha, and shadow rays (lights, glowing surfaces, mist) are dimmed
//! by every see-through surface they cross, as on the CPU tracer.

use super::features::{Features, reasons};
use super::tests::{add, parity, scene_seen, scene_with};
use crate::viewport::camera::{ViewCamera, ViewKind};
use crate::materials::Material;
use crate::materials::maps::{MapChannel, MapNode, MapSlot, NoiseKind};
use crate::primitives::PrimitiveKind;
use crate::render::volumetrics::VolumeSettings;
use bevy::math::Vec3;

fn with(mut m: Material, ch: MapChannel, node: MapNode) -> Material {
    m.maps[ch as usize] = MapSlot { node: Some(node), ..Default::default() };
    m
}

/// A half-clear veil over the floor, the box and under the glowing panel:
/// the camera sees through it and the omni and the panel shine through it.
#[test]
fn a_see_through_veil_matches_the_cpu() {
    let rs = scene_with(|s| {
        let veil = Material { base_color: [0.3, 0.6, 0.9], opacity: 0.4, roughness: 0.5, ..Material::named("Veil") };
        add(s, PrimitiveKind::Plane, Vec3::new(0.3, 0.3, 0.5), Vec3::splat(2.0), veil);
    });
    assert!(rs.translucent && reasons(&rs).is_empty(), "{:?}", reasons(&rs));
    assert_ne!(Features::of(&rs).0 & Features::OPACITY, 0);
    parity("a see-through veil", &rs);
}

/// A cut-out checker card (filtered texels must snap back to holes), a
/// cut-out card below the threshold (invisible), a live-map card at 60 %
/// opacity, all in mist so the volume's shadow rays pass them too (seen
/// close up: the far top view would see only mist).
#[test]
fn cut_out_mapped_and_live_opacity_in_mist_match_the_cpu() {
    let mut c = ViewCamera::new(ViewKind::Perspective);
    (c.target, c.distance) = (Vec3::new(0.0, 0.0, 0.4), 6.0);
    let rs = scene_seen(|s| {
        let card = Material { alpha_cutout: true, base_color: [0.9, 0.8, 0.3], ..Material::named("Card") };
        add(s, PrimitiveKind::Plane, Vec3::new(-0.5, 0.0, 0.6), Vec3::splat(1.4), with(card, MapChannel::Opacity, MapNode::Checker { tiles: 4.0, color_a: [0.95; 3], color_b: [0.05; 3] }));
        let ghost = Material { alpha_cutout: true, opacity: 0.3, base_color: [1.0, 0.0, 0.0], ..Material::named("Ghost") };
        add(s, PrimitiveKind::Plane, Vec3::new(0.2, -0.6, 0.4), Vec3::splat(1.2), ghost);
        let noise = MapNode::Noise { kind: NoiseKind::Fractal, scale: 1.7, seed: 4, color_a: [0.9, 0.5, 0.2], color_b: [0.2, 0.4, 0.8] };
        let haze = with(Material { opacity: 0.6, ..Material::named("Haze") }, MapChannel::BaseColor, noise);
        add(s, PrimitiveKind::Plane, Vec3::new(0.6, 0.6, 0.45), Vec3::splat(1.0), haze);
        s.environment.volume = VolumeSettings { enabled: true, density: 0.2, intensity: 1.5, ..Default::default() };
    }, c);
    assert!(rs.translucent && rs.volume.is_some() && reasons(&rs).is_empty(), "{:?}", reasons(&rs));
    assert!(rs.materials.iter().any(super::live::live), "the haze card uses live maps");
    parity("cut-out, mapped and live opacity in mist", &rs);
}
