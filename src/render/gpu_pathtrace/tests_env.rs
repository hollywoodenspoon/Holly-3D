//! PR 1a: the sky and HDRI on the GPU. The bake uses the CPU's equirect
//! mapping, and lit scenes match the CPU tracer.

use super::envmap::{bake, env_map, texel_dir};
use super::features::{Features, reasons};
use super::tests::{parity, scene_with};
use crate::render::hdri::{HdrImage, Hdri};
use crate::render::sky::SkyMode;
use bevy::math::Vec3;

/// A test HDRI: a blue-to-orange gradient with a bright patch up high.
fn image() -> HdrImage {
    let (w, h) = (64, 32);
    let px = (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            let t = y as f32 / h as f32;
            let base = Vec3::new(0.2, 0.35, 0.8).lerp(Vec3::new(0.9, 0.5, 0.2), t);
            if (10..16).contains(&x) && (4..9).contains(&y) { base + Vec3::splat(6.0) } else { base }
        })
        .collect();
    HdrImage { w, h, px }
}

#[test]
fn texel_centres_map_back_to_their_own_pixel() {
    let img = image();
    for (x, y) in [(0, 0), (13, 6), (63, 31), (32, 16), (5, 30)] {
        let d = texel_dir(x, y, img.w, img.h);
        let want = img.px[(y * img.w + x) as usize];
        assert!((img.sample(d, 0.0) - want).abs().max_element() < 1e-3, "texel {x},{y}");
    }
}

#[test]
fn a_baked_sky_matches_the_sky_at_texel_centres() {
    let mut sky = crate::render::sky::SkySettings { mode: SkyMode::Earth, ..Default::default() };
    sky.clouds.enabled = false;
    let img = bake(&sky, 64);
    assert_eq!((img.w, img.h), (64, 32));
    for (x, y) in [(3, 2), (40, 10), (20, 25)] {
        let d = texel_dir(x, y, img.w, img.h);
        assert!((img.px[(y * img.w + x) as usize] - sky.radiance(d)).abs().max_element() < 1e-4);
    }
}

#[test]
fn sky_and_hdri_scenes_use_the_env_switch_and_no_fallback() {
    let rs = scene_with(|s| s.environment.sky.mode = SkyMode::Earth);
    assert!(reasons(&rs).is_empty(), "{:?}", reasons(&rs));
    assert_eq!(Features::of(&rs).0 & Features::ENV_MAP, Features::ENV_MAP);
    let m = env_map(&rs, 256).expect("a baked sky");
    assert_eq!((m.w, m.h, m.px.len()), (256, 128, 256 * 128 * 16));
    let again = env_map(&rs, 256).unwrap();
    assert!(std::sync::Arc::ptr_eq(&m.px, &again.px), "a repeated bake hits the cache");
    let rs = scene_with(|s| s.environment.hdri = Some(Hdri::with_image("test.hdr".into(), image())));
    let m = env_map(&rs, 256).expect("the HDRI");
    assert_eq!((m.w, m.h), (64, 32), "an HDRI uploads at its own size");
}

#[test]
fn gpu_and_cpu_agree_under_the_procedural_sky() {
    parity("procedural sky", &scene_with(|s| s.environment.sky.mode = SkyMode::Earth));
}

#[test]
fn gpu_and_cpu_agree_under_an_hdri() {
    parity(
        "HDRI",
        &scene_with(|s| {
            let mut h = Hdri::with_image("test.hdr".into(), image());
            (h.rotation, h.strength) = (40.0, 1.5);
            s.environment.hdri = Some(h);
        }),
    );
}
