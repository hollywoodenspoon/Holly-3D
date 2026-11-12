//! Phase 11.5A path tracer checks: energy conservation (white furnace),
//! multi-bounce interreflection, unbiased Russian Roulette, MIS agreeing
//! with pure BSDF sampling, adaptive early stop and BSDF pdf consistency.

use crate::render::bsdf::{Bsdf, distribution, sample_vndf, smith_g1};
use super::progressive::{PassInfo, render};
use crate::materials::Material;
use crate::materials::shading::surface;
use crate::primitives::{Primitive, PrimitiveKind};
use crate::render::scene_build::RenderScene;
use crate::render::settings::RenderSettings;
use crate::render::trace::{Pixel, Rng};
use crate::scene::{BaseObject, Scene};
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::{Vec2, Vec3, Vec4};
use std::sync::atomic::AtomicBool;

fn cam() -> ViewCamera {
    let mut c = ViewCamera::new(ViewKind::Top);
    (c.target, c.ortho_height) = (Vec3::ZERO, 4.0);
    c
}

fn cfg(samples: u32) -> RenderSettings {
    let mut r = RenderSettings { width: 16, height: 16, outlines: false, ..Default::default() };
    (r.path.samples, r.path.adaptive, r.gi.softbox, r.gi.denoise, r.post.bloom) = (samples, false, false, false, false);
    r
}

fn add(s: &mut Scene, kind: PrimitiveKind, at: Vec3, scale: Vec3, mat: Material) {
    let id = s.add(BaseObject::Primitive(Primitive::default_of(kind)), at);
    s.objects[id].transform.scale = scale;
    s.objects[id].material = Some(s.materials.mats.insert(mat));
    s.objects[id].evaluate();
}

fn matte(c: f32) -> Material {
    Material { base_color: [c; 3], roughness: 1.0, metallic: 0.0, ..Material::named("Matte") }
}

/// Scene lit only by a uniform environment of radiance `env`.
fn build(s: &Scene, env: f32) -> RenderScene {
    let mut rs = RenderScene::build(s, cam(), false);
    (rs.ambient, rs.sky, rs.background, rs.fog) = (Vec3::ZERO, Vec3::ZERO, Vec3::splat(env), None);
    rs
}

fn run(rs: &RenderScene, c: &RenderSettings) -> (Vec<Pixel>, Vec<PassInfo>) {
    let mut passes = vec![];
    let px = render(rs, c, [0, 0, c.width, c.height], &AtomicBool::new(false), |p, _| passes.push(p));
    (px, passes)
}

fn mean(px: &[Pixel]) -> f32 {
    px.iter().map(|p| p.layers.geom.x).sum::<f32>() / px.len() as f32
}

#[test]
fn white_furnace_conserves_energy() {
    let mut s = Scene::default();
    add(&mut s, PrimitiveKind::Sphere, Vec3::ZERO, Vec3::ONE, matte(0.8));
    let (px, _) = run(&build(&s, 1.0), &cfg(128));
    let c = px[8 * 16 + 8].layers.geom;
    assert_eq!(c.w, 1.0, "the sphere covers the center");
    // sRGB 0.8 is linear albedo 0.604, plus a few percent of specular.
    assert!(c.x > 0.56 && c.x < 0.72, "a convex sphere in a unit furnace reflects its albedo: {c}");
    let mut s = Scene::default();
    add(&mut s, PrimitiveKind::Sphere, Vec3::ZERO, Vec3::ONE, Material { base_color: [1.0; 3], metallic: 1.0, roughness: 0.3, ..Material::named("Chrome") });
    let (px, _) = run(&build(&s, 1.0), &cfg(128));
    let m = px[8 * 16 + 8].layers.geom.x;
    assert!(m > 0.85 && m < 1.02, "white metal neither gains nor loses much: {m}");
}

/// A white floor with a white box standing on it: light bounces between them.
fn interreflection() -> Scene {
    let mut s = Scene::default();
    add(&mut s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(3.0), matte(0.9));
    add(&mut s, PrimitiveKind::Box, Vec3::new(0.6, 0.0, 0.0), Vec3::new(0.6, 3.0, 3.0), matte(0.9));
    s
}

#[test]
fn more_bounces_add_interreflected_light_and_roulette_stays_unbiased() {
    let rs = build(&interreflection(), 1.0);
    let at = |bounces: u32, rr: u32| {
        let mut c = cfg(96);
        (c.path.max_bounces, c.path.diffuse_bounces, c.path.rr_start) = (bounces, bounces, rr);
        mean(&run(&rs, &c).0)
    };
    let (one, four) = (at(1, 8), at(4, 8));
    assert!(four > one * 1.03, "bounce light between floor and box: 1 bounce {one}, 4 bounces {four}");
    let early = at(4, 1);
    assert!((early - four).abs() < 0.03 * four, "Russian Roulette after bounce 1 keeps the mean: {early} vs {four}");
}

#[test]
fn mis_matches_pure_bsdf_sampling_on_an_area_light() {
    let mut s = Scene::default();
    add(&mut s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(3.0), matte(0.8));
    let glow = Material { emissive: [1.0; 3], emissive_strength: 4.0, base_color: [1.0; 3], ..Material::named("Glow") };
    add(&mut s, PrimitiveKind::Plane, Vec3::new(0.0, 0.0, 0.6), Vec3::splat(0.4), glow);
    let rs = build(&s, 0.0);
    assert!(!rs.emitters.is_empty());
    let floor = |px: &[Pixel]| px.iter().take(16 * 4).map(|p| p.layers.geom.x).sum::<f32>() / 64.0;
    let mut c = cfg(256);
    c.path.max_bounces = 1;
    let mis = floor(&run(&rs, &c).0);
    c.gi.emitter_samples = 0;
    let bsdf_only = floor(&run(&rs, &c).0);
    assert!(mis > 0.01, "the panel lights the floor: {mis}");
    assert!((mis - bsdf_only).abs() < 0.08 * mis, "both estimators converge to the same light: MIS {mis} vs BSDF {bsdf_only}");
}

#[test]
fn adaptive_sampling_stops_flat_images_early() {
    let mut s = Scene::default();
    add(&mut s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(9.0), Material { model: crate::materials::ShadingModel::Unlit, ..matte(0.5) });
    let rs = build(&s, 0.0);
    let mut c = cfg(256);
    (c.path.adaptive, c.path.min_samples) = (true, 8);
    let (_, passes) = run(&rs, &c);
    let last = passes.last().unwrap();
    assert_eq!(last.active, 0.0);
    assert!(last.spp < 64, "a noiseless image converges right after Min Samples: {passes:?}");
    c.path.adaptive = false;
    let (_, all) = run(&rs, &c);
    assert_eq!(all.iter().map(|p| p.spp).collect::<Vec<_>>(), vec![1, 2, 4, 8, 16, 32, 64, 128, 256], "progressive 1 -> 2 -> 4 ... passes");
}

#[test]
fn bsdf_sampling_matches_its_evaluated_density() {
    let n = Vec3::Z;
    let v = Vec3::new(0.5, 0.1, 0.8).normalize();
    for (metallic, rough) in [(0.0, 1.0), (0.0, 0.4), (1.0, 0.25), (0.5, 0.6)] {
        let m = Material { base_color: [0.9, 0.6, 0.3], metallic, roughness: rough, ..Material::named("T") };
        let b = Bsdf::new(&surface(&m, Vec2::ZERO, Vec3::ZERO, Vec4::ONE), n, true);
        let mut rng = Rng::new(3);
        let k = 400_000;
        // E[f cos / pdf] by importance sampling ...
        let sampled: Vec3 = (0..k).filter_map(|_| b.sample(v, &mut rng)).map(|s| s.weight).sum::<Vec3>() / k as f32;
        // ... equals the albedo integrated by uniform hemisphere sampling.
        let uniform: Vec3 = (0..k)
            .map(|_| {
                let (z, phi) = (rng.f(), rng.f() * std::f32::consts::TAU);
                let r = (1.0 - z * z).sqrt();
                b.eval(v, Vec3::new(r * phi.cos(), r * phi.sin(), z)).0 * std::f32::consts::TAU
            })
            .sum::<Vec3>()
            / k as f32;
        assert!((sampled - uniform).abs().max_element() < 0.04, "metal {metallic} rough {rough}: {sampled} vs {uniform}");
    }
    // VNDF microfacet normals stay in the upper hemisphere; D and G are sane.
    let mut rng = Rng::new(9);
    assert!((0..500).all(|_| sample_vndf(n, v, 0.3, rng.f(), rng.f()).z > 0.0));
    assert!(distribution(1.0, 0.04) > distribution(0.8, 0.04) && smith_g1(1.0, 0.1) > 0.99);
}

#[test]
fn glass_refracts_and_tints_entering_light() {
    let m = Material { base_color: [0.2, 0.9, 0.4], ..crate::materials::presets::Preset::Crystal.material() };
    let s = surface(&m, Vec2::ZERO, Vec3::ZERO, Vec4::ONE);
    let b = Bsdf::new(&s, Vec3::Z, true);
    let mut rng = Rng::new(1);
    let samples: Vec<_> = (0..400).filter_map(|_| b.sample(Vec3::Z, &mut rng)).collect();
    let through = samples.iter().filter(|s| s.dir.z < 0.0).count();
    assert!(through > 250, "head-on light mostly refracts: {through}/400");
    assert!(samples.iter().filter(|s| s.dir.z < 0.0).all(|s| s.delta && s.pdf == 0.0 && s.weight.y > s.weight.x), "green tint");
}

