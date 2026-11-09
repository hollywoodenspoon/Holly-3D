use super::maps::{MapChannel, MapNode, color_correct};
use super::shading::{LightIn, linear_to_srgb, refract, shade, srgb_to_linear, surface};
use super::*;
use bevy::math::{Vec2, Vec3, Vec4};

#[test]
fn library_has_24_slots_with_presets_first() {
    let lib = MaterialLib::default();
    assert_eq!(lib.slots.len(), SLOT_COUNT);
    assert_eq!(lib.mats.len(), SLOT_COUNT);
    assert!(lib.mats[lib.slot(0)].name.starts_with("01 - "));
    assert_eq!(lib.mats[lib.slot(5)].model, ShadingModel::Crystal);
    assert_eq!(lib.mats[lib.slot(6)].model, ShadingModel::Toon);
    assert!(lib.mats[lib.slot(6)].outline.enabled);
}

#[test]
fn reset_and_copy_slot_create_new_materials() {
    let mut lib = MaterialLib::default();
    let old = lib.slot(2);
    let copy = lib.copy_slot(2);
    assert_ne!(old, copy);
    assert_eq!(lib.mats[copy].metallic, lib.mats[old].metallic);
    let fresh = lib.reset_slot(2);
    assert_eq!(lib.mats[fresh].metallic, 0.0);
    assert!(lib.mats.contains_key(old), "scene objects keep their material");
    lib.put_in_slot(3, old);
    assert_eq!(lib.slot_of(old), Some(3));
}

#[test]
fn preset_values_match_spec() {
    let steel = Preset::BrushedSteel.material();
    assert_eq!((steel.metallic, steel.roughness), (0.90, 0.25));
    let gold = Preset::PolishedGold.material();
    assert_eq!((gold.metallic, gold.roughness), (1.0, 0.05));
    let rubber = Preset::MatteRubber.material();
    assert_eq!((rubber.metallic, rubber.roughness), (0.0, 0.85));
    let gem = Preset::Crystal.material();
    assert_eq!((gem.transmission, gem.ior, gem.roughness), (0.92, 1.55, 0.06));
    let mut m = Material::named("Keep");
    Preset::Wood.apply(&mut m);
    assert_eq!(m.name, "Keep");
    assert!(m.active_map(MapChannel::BaseColor).is_some());
}

#[test]
fn maps_sample_finite_and_in_range() {
    let p = Vec3::new(0.3, -0.7, 0.2);
    for i in 0..MapNode::KINDS.len() {
        let n = MapNode::new_kind(i, None);
        for k in 0..20 {
            let uv = Vec2::new(k as f32 * 0.13, k as f32 * 0.07);
            let c = n.sample(uv, p * k as f32);
            assert!(c.is_finite(), "{} at {uv}", n.kind_name());
            assert!(c.min_element() >= 0.0 && c.max_element() <= 1.0 + 1e-4, "{} -> {c}", n.kind_name());
        }
    }
    let checker = MapNode::new_kind(2, None);
    assert_ne!(checker.sample(Vec2::new(0.01, 0.01), p), checker.sample(Vec2::new(0.13, 0.01), p));
}

#[test]
fn bitmap_texture_sampling_wraps_and_flips_v() {
    let mut t = Texture::new(2, 2, [0, 0, 0, 255]);
    t.data[0] = [255, 0, 0, 255]; // top-left (v = 1)
    let n = MapNode::Bitmap { path: String::new(), tex: Some(TexHandle::new(t)) };
    let top_left = n.sample(Vec2::new(0.25, 0.75), Vec3::ZERO);
    assert!(top_left.x > 0.99 && top_left.y < 0.01);
    let wrapped = n.sample(Vec2::new(1.25, 1.75), Vec3::ZERO);
    assert!((wrapped - top_left).length() < 1e-5);
}

#[test]
fn tex_handle_copy_on_write_keeps_snapshot() {
    let a = TexHandle::new(Texture::new(4, 4, [10, 10, 10, 255]));
    let mut b = a.clone();
    assert_eq!(a, b);
    b.make_mut().data[0] = [200, 0, 0, 255];
    assert_ne!(a, b);
    assert_eq!(a.tex.data[0], [10, 10, 10, 255]);
}

#[test]
fn color_correction_identity_and_saturation() {
    let c = Vec3::new(0.2, 0.5, 0.8);
    assert!((color_correct(c, 0.0, 1.0, 1.0, 0.0) - c).length() < 1e-6);
    let g = color_correct(c, 0.0, 1.0, 0.0, 0.0);
    assert!((g.x - g.y).abs() < 1e-6 && (g.y - g.z).abs() < 1e-6);
    let h = color_correct(c, 0.0, 1.0, 1.0, 360.0);
    assert!((h - c).length() < 1e-4);
}

#[test]
fn srgb_roundtrip() {
    for i in 0..=10 {
        let x = i as f32 / 10.0;
        assert!((linear_to_srgb(srgb_to_linear(x)) - x).abs() < 1e-5);
    }
}

#[test]
fn shading_models_respond_to_light() {
    let n = Vec3::Z;
    let lit = [LightIn { dir: Vec3::Z, radiance: Vec3::ONE * 3.0 }];
    let dark = [LightIn { dir: -Vec3::Z, radiance: Vec3::ONE * 3.0 }];
    for model in ShadingModel::ALL {
        let m = Material { model, ..Material::default() };
        let s = surface(&m, Vec2::ZERO, Vec3::ZERO, Vec4::ONE);
        let a = shade(&m, &s, n, Vec3::Z, &lit, Vec3::splat(0.1));
        let b = shade(&m, &s, n, Vec3::Z, &dark, Vec3::splat(0.1));
        assert!(a.is_finite() && b.is_finite());
        if model != ShadingModel::Unlit {
            assert!(a.element_sum() > b.element_sum(), "{model:?} lit {a} vs unlit {b}");
        } else {
            assert_eq!(a, b);
        }
    }
    // Toon: two flat bands only.
    let m = Preset::CelShaded.material();
    let s = surface(&m, Vec2::ZERO, Vec3::ZERO, Vec4::ONE);
    let at = |deg: f32| {
        let l = Vec3::new(deg.to_radians().sin(), 0.0, deg.to_radians().cos());
        shade(&m, &s, n, Vec3::Z, &[LightIn { dir: l, radiance: Vec3::ONE }], Vec3::ZERO)
    };
    assert_eq!(at(10.0), at(40.0));
    assert_ne!(at(10.0), at(75.0));
}

#[test]
fn refraction_bends_toward_normal() {
    let d = Vec3::new(1.0, 0.0, -1.0).normalize();
    let t = refract(d, Vec3::Z, 1.0 / 1.5).unwrap();
    assert!(t.x < d.x && t.x > 0.0);
    assert!(refract(Vec3::new(1.0, 0.0, -0.1).normalize(), Vec3::Z, 1.5).is_none(), "total internal reflection");
}

#[test]
fn previews_render_every_preset() {
    for p in Preset::ALL {
        let img = preview::render_preview(&p.material(), 24);
        assert_eq!(img.len(), 24 * 24);
        let center = img[12 * 24 + 12];
        let corner = img[0];
        assert_ne!(center, corner, "{p:?} sphere visible");
    }
}

#[test]
fn cel_preview_draws_ink_ring() {
    let m = Preset::CelShaded.material();
    let px = preview::render_preview(&m, 64);
    let ink = px.iter().filter(|p| p[0] < 40 && p[1] < 40 && p[2] < 40).count();
    let mut plain = m.clone();
    plain.outline.enabled = false;
    let base = preview::render_preview(&plain, 64).iter().filter(|p| p[0] < 40 && p[1] < 40 && p[2] < 40).count();
    assert!(ink > base + 60, "ink ring pixels: {ink} vs {base}");
}
