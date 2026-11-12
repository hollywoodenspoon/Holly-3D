use super::*;
use crate::materials::Material;
use crate::materials::shading::surface;
use bevy::math::{Vec2, Vec4};

fn bsdf(m: &Material) -> Bsdf {
    let s = surface(m, Vec2::ZERO, Vec3::ZERO, Vec4::ONE);
    Bsdf::new(&s, Vec3::Z, true).principled(&m.principled, s.roughness)
}

/// (importance-sampled albedo, uniform-hemisphere albedo, SSS share).
fn albedos(b: &Bsdf, v: Vec3, seed: u64) -> (Vec3, Vec3, f32) {
    let mut rng = Rng::new(seed);
    let k = 60_000;
    let (mut sampled, mut sss) = (Vec3::ZERO, 0.0);
    for _ in 0..k {
        if let Some(s) = b.sample(v, &mut rng) {
            if s.sss {
                sss += s.weight.x;
            } else {
                sampled += s.weight;
            }
        }
    }
    let uniform: Vec3 = (0..k)
        .map(|_| {
            let (z, phi) = (rng.f(), rng.f() * std::f32::consts::TAU);
            let r = (1.0 - z * z).sqrt();
            b.eval(v, Vec3::new(r * phi.cos(), r * phi.sin(), z)).0 * std::f32::consts::TAU
        })
        .sum::<Vec3>()
        / k as f32;
    (sampled / k as f32, uniform, sss / k as f32)
}

fn looks() -> Vec<(&'static str, Material)> {
    let base = |f: fn(&mut Material)| {
        let mut m = Material { base_color: [0.9, 0.7, 0.5], roughness: 0.45, ..Material::named("T") };
        f(&mut m);
        m
    };
    vec![
        ("aniso metal", base(|m| (m.metallic, m.principled.anisotropy, m.principled.aniso_rotation) = (1.0, 0.8, 30.0))),
        ("clearcoat", base(|m| (m.principled.clearcoat, m.principled.clearcoat_roughness) = (1.0, 0.1))),
        ("sheen", base(|m| (m.principled.sheen, m.principled.sheen_tint, m.roughness) = (1.0, 0.5, 0.9))),
        ("sss", base(|m| m.principled.sss_weight = 0.7)),
        ("everything", base(|m| {
            let p = &mut m.principled;
            (p.sss_weight, p.clearcoat, p.sheen, p.anisotropy) = (0.5, 0.6, 0.5, 0.5);
        })),
    ]
}

#[test]
fn principled_lobes_sample_their_own_density() {
    let v = Vec3::new(0.45, -0.2, 0.85).normalize();
    for (name, m) in looks() {
        let (sampled, uniform, _) = albedos(&bsdf(&m), v, 5);
        assert!((sampled - uniform).abs().max_element() < 0.04, "{name}: {sampled} vs {uniform}");
    }
}

#[test]
fn principled_lobes_conserve_energy() {
    // White base: the extra lobes never add energy over the plain
    // Lambert + GGX base (which itself reflects up to ~1.04: diffuse + F0).
    for (name, mut m) in looks() {
        m.base_color = [1.0; 3];
        let plain = Material { principled: Default::default(), ..m.clone() };
        for v in [Vec3::Z, Vec3::new(0.7, 0.0, 0.714).normalize(), Vec3::new(0.97, 0.0, 0.243).normalize()] {
            let (sampled, _, sss) = albedos(&bsdf(&m), v, 11);
            let (base, _, _) = albedos(&bsdf(&plain), v, 11);
            let total = sampled.max_element() + sss;
            assert!(total < base.max_element().max(1.0) + 0.02, "{name} at nv {:.2}: reflected {sampled} + sss {sss} vs plain {base}", v.z);
        }
    }
}

#[test]
fn sss_moves_diffuse_energy_into_the_walk() {
    let mut m = Material { base_color: [0.8; 3], roughness: 0.9, ..Material::named("Wax") };
    let (plain, _, s0) = albedos(&bsdf(&m), Vec3::Z, 2);
    m.principled.sss_weight = 1.0;
    let (refl, _, s1) = albedos(&bsdf(&m), Vec3::Z, 2);
    assert_eq!(s0, 0.0);
    assert!(refl.max_element() < 0.1 && s1 > 0.55, "full SSS: {refl} reflected, {s1} entering");
    assert!((refl.max_element() + s1 - plain.max_element()).abs() < 0.05, "same total energy");
}

#[test]
fn anisotropy_stretches_the_highlight_along_the_tangent() {
    let mut m = Material { base_color: [1.0; 3], metallic: 1.0, roughness: 0.4, ..Material::named("Brushed") };
    m.principled.anisotropy = 0.9;
    let b = bsdf(&m);
    let f = |l: Vec3| b.eval(Vec3::Z, l.normalize()).0.x;
    // Frame tangent for n = Z is world Y-ish: compare tilts of equal angle.
    let (t, bt) = (b.frame.t, b.frame.b);
    let along = f(Vec3::Z + t * 0.6);
    let across = f(Vec3::Z + bt * 0.6);
    assert!(along > across * 3.0, "highlight wider along the tangent: {along} vs {across}");
    let (ax, ay) = microfacet::alphas(0.4, 0.0);
    assert_eq!(ax, ay, "no anisotropy -> isotropic");
}

#[test]
fn dispersion_splits_channels_by_wavelength() {
    let mut m = crate::materials::presets::Preset::Crystal.material();
    m.principled.abbe = 20.0;
    let p = &m.principled;
    let (r, g, b) = (p.ior_at(1.55, CHANNEL_LAMBDA[0]), p.ior_at(1.55, CHANNEL_LAMBDA[1]), p.ior_at(1.55, CHANNEL_LAMBDA[2]));
    assert!(r < g && g < b, "blue bends more: {r} {g} {b}");
    assert!((p.ior_at(1.55, 0.5893) - 1.55).abs() < 1e-4, "base IOR at the sodium D line");
    let bs = bsdf(&m);
    let mut rng = Rng::new(4);
    let refracted: Vec<_> = (0..600).filter_map(|_| bs.sample(Vec3::new(0.5, 0.0, 0.866), &mut rng)).filter(|s| s.dir.z < 0.0).collect();
    let dirs: Vec<f32> = (0..3)
        .map(|k| {
            let c: Vec<f32> = refracted.iter().filter(|s| s.channel == Some(k)).map(|s| s.dir.x).collect();
            c.iter().sum::<f32>() / c.len().max(1) as f32
        })
        .collect();
    assert!(refracted.iter().all(|s| s.channel.is_some()), "each refraction picks a channel");
    assert!(dirs[0] < dirs[1] && dirs[1] < dirs[2], "red bends least toward the normal: {dirs:?}");
}

#[test]
fn beer_lambert_absorption_darkens_with_depth() {
    let mut p = crate::materials::principled::Principled::default();
    assert_eq!(crate::render::pathtrace::subsurface::absorption(&p, 5.0), Vec3::ONE);
    (p.absorption_color, p.absorption_density) = ([0.5, 0.8, 1.0], 1.0);
    let thin = crate::render::pathtrace::subsurface::absorption(&p, 0.2);
    let thick = crate::render::pathtrace::subsurface::absorption(&p, 2.0);
    assert!(thick.x < thin.x && thick.z > 0.99, "{thin} -> {thick}");
    let one = crate::render::pathtrace::subsurface::absorption(&p, 1.0);
    assert!((one.x - crate::materials::shading::srgb_to_linear(0.5)).abs() < 1e-4, "density 1 reaches the color after one unit");
}

