//! Phase 8A raytracer checks: Multi/Sub-Object per-ID rendering, emissive
//! area lights, AO, 1-bounce GI, the softbox dome, denoiser, bloom, DOF and
//! live re-tonemapping.

use super::job::raytrace_region;
use super::post::Tone;
use super::scene_build::RenderScene;
use super::trace::Pixel;
use super::*;
use crate::materials::{Material, MultiSub, ShadingModel};
use crate::primitives::{Primitive, PrimitiveKind};
use crate::scene::{BaseObject, ObjId, Scene};
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::{Quat, Vec3};
use std::sync::atomic::AtomicBool;

fn cam() -> ViewCamera {
    let mut c = ViewCamera::new(ViewKind::Top);
    (c.target, c.ortho_height) = (Vec3::ZERO, 4.0);
    c
}

fn cfg() -> RenderSettings {
    RenderSettings { width: 32, height: 32, aa: 1, outlines: false, ..Default::default() }
}

fn render(s: &Scene, cfg: &RenderSettings) -> Vec<Pixel> {
    let rs = RenderScene::build(s, cam(), false);
    raytrace_region(&rs, cfg, [0, 0, cfg.width, cfg.height], &AtomicBool::new(false), |_, _| {})
}

fn add(s: &mut Scene, kind: PrimitiveKind, at: Vec3, scale: Vec3, mat: Material) -> ObjId {
    let id = s.add(BaseObject::Primitive(Primitive::default_of(kind)), at);
    s.objects[id].transform.scale = scale;
    s.objects[id].material = Some(s.materials.mats.insert(mat));
    s.objects[id].evaluate();
    id
}

/// A dark scene (no lights, no ambient) with a matte white floor.
fn dark_floor() -> Scene {
    let mut s = Scene::default();
    (s.environment.ambient_level, s.environment.background) = (0.0, [0.0; 3]);
    add(
        &mut s,
        PrimitiveKind::Plane,
        Vec3::ZERO,
        Vec3::splat(3.0),
        Material { base_color: [0.8; 3], roughness: 0.9, ..Material::named("Floor") },
    );
    s
}

fn glow(rgb: [f32; 3], k: f32) -> Material {
    Material { emissive: rgb, emissive_strength: k, base_color: rgb, ..Material::named("Glow") }
}

#[test]
fn multi_sub_object_renders_each_polygon_by_its_id() {
    let mut s = Scene::default();
    let unlit = |rgb| Material { base_color: rgb, model: ShadingModel::Unlit, ..Material::named("u") };
    let mut ms = MultiSub::new(2);
    (ms.subs[0].mat, ms.subs[1].mat) = (unlit([1.0, 0.0, 0.0]), unlit([0.0, 0.0, 1.0]));
    let id = add(&mut s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::ONE, Material { multi: Some(ms), ..Material::named("Multi") });
    let mesh = &mut *s.objects[id].evaluated_mesh;
    let cx = |m: &crate::mesh::EditablePoly, p: &crate::mesh::Polygon| {
        p.corners.iter().map(|c| m.vertices[c.vertex_id].position.x).sum::<f32>() / p.corners.len() as f32
    };
    let right: Vec<_> = mesh.polygons.iter().filter(|(_, p)| cx(mesh, p) > 0.0).map(|(k, _)| k).collect();
    assert!(!right.is_empty());
    for p in right {
        mesh.polygons[p].material_id = 2;
    }
    let px = render(&s, &cfg());
    let (l, r) = (px[16 * 32 + 10].hdr, px[16 * 32 + 22].hdr);
    assert!(l.x > 0.9 && l.z < 0.1, "ID 1 is red: {l}");
    assert!(r.z > 0.9 && r.x < 0.1, "ID 2 is blue: {r}");
}

#[test]
fn emissive_polygons_light_nearby_geometry() {
    let mut s = dark_floor();
    let off = render(&s, &cfg());
    // A red glowing panel just outside the view, hovering over the floor.
    add(&mut s, PrimitiveKind::Plane, Vec3::new(2.6, 0.0, 0.6), Vec3::splat(0.5), glow([1.0, 0.1, 0.05], 20.0));
    let on = render(&s, &cfg());
    let (a, b) = (off[16 * 32 + 28].hdr, on[16 * 32 + 28].hdr);
    assert!(b.x > a.x + 0.05, "the floor near the panel is lit: {a} -> {b}");
    assert!(b.x > b.y * 3.0, "with the panel's color: {b}");
    let far = on[16 * 32 + 2].hdr.x;
    assert!(b.x > far * 2.0, "falls off with distance: {} vs {far}", b.x);
    let none = render(&s, &RenderSettings { gi: post::GiSettings { emitter_samples: 0, ..Default::default() }, ..cfg() });
    assert!(none[16 * 32 + 28].hdr.x < b.x * 0.5, "Emitter Samples 0 turns area lighting off");
}

fn floor_with_box(ambient: f32) -> Scene {
    let mut s = dark_floor();
    s.environment.ambient_level = ambient;
    let m = Material { base_color: [0.9, 0.05, 0.05], roughness: 0.9, ..Material::named("Red") };
    add(&mut s, PrimitiveKind::Box, Vec3::new(0.0, 0.0, 0.0), Vec3::new(0.6, 3.0, 1.5), m);
    s
}

#[test]
fn ambient_occlusion_darkens_contact_crevices() {
    let s = floor_with_box(1.0);
    let ao = |on| post::GiSettings { ao: on, ao_samples: 32, ao_radius: 1.0, ao_strength: 1.0, denoise: false, ..Default::default() };
    let off = render(&s, &RenderSettings { gi: ao(false), ..cfg() });
    let on = render(&s, &RenderSettings { gi: ao(true), ..cfg() });
    // Pixel on the floor right next to the box vs one far away.
    let (near, far) = (16 * 32 + 20, 16 * 32 + 30);
    assert!(on[near].hdr.y < off[near].hdr.y * 0.8, "crevice darkens: {} vs {}", on[near].hdr.y, off[near].hdr.y);
    assert!(on[far].hdr.y > on[near].hdr.y, "open floor stays brighter");
}

#[test]
fn one_bounce_gi_bleeds_color_onto_the_floor() {
    let mut s = floor_with_box(0.0);
    let mut l = Light::new(LightKind::FreeDirect);
    l.cast_shadows = false;
    s.add_light(l, None, Vec3::new(-3.0, 0.0, 3.0), Vec3::ZERO);
    let gi = |on| post::GiSettings { gi: on, gi_samples: 64, denoise: false, ..Default::default() };
    let off = render(&s, &RenderSettings { gi: gi(false), ..cfg() });
    let on = render(&s, &RenderSettings { gi: gi(true), ..cfg() });
    let i = 16 * 32 + 11; // floor on the lit side, next to the red box
    let (a, b) = (off[i].hdr, on[i].hdr);
    assert!(b.x - a.x > (b.y - a.y) * 2.0 && b.x > a.x + 0.01, "red bounce light: {a} -> {b}");
}

#[test]
fn softbox_dome_shows_in_mirror_reflections() {
    let mut s = Scene::default();
    (s.environment.ambient_level, s.environment.background) = (0.0, [0.0; 3]);
    let chrome = Material { base_color: [0.95; 3], metallic: 1.0, roughness: 0.0, ..Material::named("Chrome") };
    let id = add(&mut s, PrimitiveKind::Sphere, Vec3::ZERO, Vec3::splat(1.5), chrome);
    s.objects[id].transform.rotation = Quat::IDENTITY;
    let sum = |px: &[Pixel]| px.iter().map(|p| p.hdr.x).sum::<f32>();
    let with = render(&s, &cfg());
    let without = render(&s, &RenderSettings { gi: post::GiSettings { softbox: false, ..Default::default() }, ..cfg() });
    assert!(sum(&with) > sum(&without) + 5.0, "{} vs {}", sum(&with), sum(&without));
}

#[test]
fn denoiser_smooths_low_sample_noise() {
    let s = floor_with_box(1.0);
    let gi = |dn| post::GiSettings { ao: true, ao_samples: 4, ao_radius: 2.0, denoise: dn, ..Default::default() };
    let var = |px: &[Pixel]| {
        let v: Vec<f32> = (24..31).map(|x| px[16 * 32 + x].hdr.y).collect();
        let m = v.iter().sum::<f32>() / v.len() as f32;
        v.iter().map(|y| (y - m).powi(2)).sum::<f32>()
    };
    let noisy = render(&s, &RenderSettings { gi: gi(false), ..cfg() });
    let clean = render(&s, &RenderSettings { gi: gi(true), ..cfg() });
    assert!(var(&clean) < var(&noisy), "{} vs {}", var(&clean), var(&noisy));
}

#[test]
fn bloom_glows_around_bright_emitters_and_dof_blurs() {
    let mut s = dark_floor();
    add(&mut s, PrimitiveKind::Plane, Vec3::new(0.0, 0.0, 0.1), Vec3::splat(0.2), glow([1.0; 3], 40.0));
    let mut c = cfg();
    let plain = render(&s, &c);
    (c.post.bloom, c.post.bloom_radius) = (true, 200.0);
    let bloomed = render(&s, &c);
    assert!(bloomed[16 * 32 + 22].layers.glow.x > 0.01, "glow spreads past the panel");
    assert!(bloomed[16 * 32 + 22].display.x > plain[16 * 32 + 22].display.x);
    assert_eq!(plain[0].layers.glow, Vec3::ZERO);
    // Thin-lens circle of confusion: sharp at the focus distance, capped.
    assert_eq!(postfx::coc(500.0, 5.0, 5.0), 0.0);
    assert!(postfx::coc(50.0, 5.0, 2.0) > postfx::coc(50.0, 5.0, 4.0));
    assert_eq!(postfx::coc(1e6, 5.0, f32::INFINITY), 24.0);
}

#[test]
fn frame_buffer_retones_without_rerendering() {
    let s = floor_with_box(1.0);
    let px = render(&s, &cfg());
    let mut fb = FrameBuffer::new(32, 32);
    let (d, h): (Vec<_>, Vec<_>) = px.iter().map(|p| (p.display, p.hdr)).unzip();
    fb.put_block(0, 0, 32, &d, &h);
    fb.put_layers(0, 0, 32, &px.iter().map(|p| p.layers).collect::<Vec<_>>());
    assert!(fb.can_retone());
    let before = fb.display[16 * 32 + 28];
    fb.retone(&Tone { exposure: 1.5, ..Default::default() });
    assert!(fb.display[16 * 32 + 28].x > before.x + 0.05, "brighter with +1.5 EV");
    fb.retone(&Tone::default());
    assert!((fb.display[16 * 32 + 28] - before).length() < 1e-5, "back to the render");
}
