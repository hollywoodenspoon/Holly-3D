//! Dynamic sky checks: noon blue, golden-hour sunset, starlit night, clouds
//! (coverage + wind drift), deep space stars / nebula, bake layout, cache key
//! and the F9 render using the sky as its background.

use super::job::raytrace_region;
use super::scene_build::RenderScene;
use super::sky::{SkyMode, SkySettings, procedural_hdri};
use super::*;
use crate::scene::Scene;
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::Vec3;
use std::sync::atomic::AtomicBool;

fn earth(time: f32) -> SkySettings {
    let mut s = SkySettings { mode: SkyMode::Earth, time_of_day: time, ..Default::default() };
    s.clouds.enabled = false;
    s
}

/// A low direction on the sun's side of the sky.
fn toward_sun_low(s: &SkySettings) -> Vec3 {
    let sun = s.sun_dir();
    (Vec3::new(sun.x, sun.y, 0.0).normalize() + Vec3::Z * 0.05).normalize()
}

#[test]
fn noon_zenith_is_blue_and_bright() {
    let s = earth(12.0);
    assert!(s.sun_dir().z > 0.75, "high noon sun");
    let z = s.radiance(Vec3::Z);
    assert!(z.z > z.x * 2.0 && z.z > 0.5, "blue zenith: {z}");
    let h = s.radiance(Vec3::new(0.0, 1.0, 0.02).normalize());
    assert!(h.x > z.x, "paler horizon: {h} vs {z}");
}

#[test]
fn golden_hour_glows_orange_near_the_sun() {
    let s = earth(18.0);
    assert!(s.sun_dir().z.abs() < 0.15, "sun near the horizon at 18:00: {}", s.sun_dir());
    let c = s.radiance(toward_sun_low(&s));
    assert!(c.x > c.z * 1.3, "sunset horizon is warm: {c}");
    let away = s.radiance((-toward_sun_low(&s) + Vec3::Z * 0.1).normalize());
    assert!(c.x > away.x, "glow sits on the sun's side");
}

#[test]
fn midnight_is_dark_indigo_with_stars() {
    let s = earth(0.0);
    assert!(s.sun_dir().z < -0.5 && s.moon_dir().z > 0.5);
    let dirs: Vec<Vec3> = (0..4000)
        .map(|i| {
            let (a, b) = (i as f32 * 0.137, 0.2 + (i % 50) as f32 / 60.0);
            Vec3::new(a.cos() * (1.0 - b * b).sqrt(), a.sin() * (1.0 - b * b).sqrt(), b)
        })
        .collect();
    let sky: Vec<Vec3> = dirs.iter().map(|d| s.radiance(*d)).collect();
    let median = {
        let mut v: Vec<f32> = sky.iter().map(|c| c.length()).collect();
        v.sort_by(f32::total_cmp);
        v[v.len() / 2]
    };
    // Phase 8B lifted the night sky a little so moonlit scenes read.
    assert!(median < 0.12, "night sky is dark: {median}");
    assert!(s.radiance(Vec3::Z).z > s.radiance(Vec3::Z).x, "indigo tint");
    assert!(sky.iter().any(|c| c.length() > median * 4.0), "some stars twinkle");
    let (_, col, mult) = s.key_light().unwrap();
    assert!(mult < 0.3 && col.z > col.x, "moonlight key");
}

#[test]
fn clouds_cover_and_drift_with_wind() {
    let mut s = earth(12.0);
    let dirs: Vec<Vec3> = (0..400).map(|i| Vec3::new((i % 20) as f32 * 0.1 - 1.0, (i / 20) as f32 * 0.1 - 1.0, 1.0).normalize()).collect();
    let clear: Vec<Vec3> = dirs.iter().map(|d| s.radiance(*d)).collect();
    s.clouds = super::sky::Clouds { enabled: true, coverage: 0.7, density: 1.0, ..Default::default() };
    let cloudy: Vec<Vec3> = dirs.iter().map(|d| s.radiance(*d)).collect();
    let whiter = clear.iter().zip(&cloudy).filter(|(a, b)| b.x - a.x > 0.1).count();
    assert!(whiter > 40, "clouds whiten part of the sky: {whiter}");
    s.time_of_day = 12.5;
    let later: Vec<Vec3> = dirs.iter().map(|d| s.radiance(*d)).collect();
    let moved = cloudy.iter().zip(&later).filter(|(a, b)| (**a - **b).length() > 0.05).count();
    assert!(moved > 20, "wind drift moves the clouds: {moved}");
    s.clouds.wind_speed = 0.0;
    let a = s.radiance(dirs[210]);
    s.time_of_day = 12.6;
    assert!((a - s.radiance(dirs[210])).length() < 0.05, "no wind, (almost) no motion");
}

#[test]
fn deep_space_has_stars_nebula_and_a_bright_star() {
    let mut s = SkySettings { mode: SkyMode::Space, ..Default::default() };
    let img = s.bake(256, 128);
    let lum: Vec<f32> = img.px.iter().map(|c| c.length()).collect();
    let dark = lum.iter().filter(|l| **l < 0.05).count();
    assert!(dark > lum.len() / 3, "mostly black space");
    assert!(lum.iter().filter(|l| **l > 0.3).count() > 10, "plenty of stars");
    let (dir, _, mult) = s.key_light().unwrap();
    assert!(s.radiance(dir).length() > 20.0 && mult > 1.0, "the single star is bright");
    let with: f32 = img.average().length();
    s.space.nebula = false;
    assert!(s.bake(256, 128).average().length() < with, "nebula adds colored glow");
}

#[test]
fn bake_layout_key_and_hdri() {
    let s = earth(12.0);
    let img = s.bake(64, 32);
    assert_eq!((img.w, img.h, img.px.len()), (64, 32, 64 * 32));
    // Top row is the zenith, bottom row the ground.
    assert!(img.px[0].z > img.px[(31 * 64) as usize].z);
    let up = img.sample(Vec3::Z, 0.0);
    assert!((up - s.radiance(Vec3::Z)).length() < 0.1, "bake matches sample direction");
    let mut t = s.clone();
    t.time_of_day = 13.0;
    assert_ne!(s.key(), t.key(), "key follows changes");
    assert_eq!(s.key(), s.clone().key());
    assert!(procedural_hdri(&SkySettings::default(), 64).is_none(), "Off -> no sky");
    let h = procedural_hdri(&s, 64).unwrap();
    assert_eq!(h.image.as_ref().map(|i| (i.w, i.h)), Some((64, 32)));
}

#[test]
fn f9_render_uses_the_sky_background() {
    let mut scene = Scene::default();
    scene.environment.background = [0.0; 3];
    scene.environment.sky = earth(12.0);
    let mut cam = ViewCamera::new(ViewKind::Perspective);
    (cam.target, cam.pitch) = (Vec3::ZERO, -0.4);
    let rs = RenderScene::build(&scene, cam, false);
    assert!(rs.hdri.is_some(), "the sky becomes the environment");
    let cfg = RenderSettings { width: 24, height: 24, aa: 1, outlines: false, ..Default::default() };
    let px = raytrace_region(&rs, &cfg, [0, 0, 24, 24], &AtomicBool::new(false), |_, _| {});
    let top = px[0].hdr;
    assert!(top.z > 0.3 && top.z > top.x, "blue sky at the top of the frame: {top}");
}

#[test]
fn baked_sky_is_written_for_cycles() {
    let mut s = earth(9.0);
    s.time_of_day = 9.123; // unique key for this test
    let h = procedural_hdri(&s, 32).unwrap();
    let _ = std::fs::remove_file(&h.path);
    h.ensure_file().unwrap();
    let back = super::hdri::HdrImage::load(&h.path).unwrap();
    assert_eq!((back.w, back.h), (32, 16));
    let (a, b) = (back.px[40], h.image.as_ref().unwrap().px[40]);
    assert!((a - b).length() < 0.05 * b.length().max(0.1), "round trip: {a} vs {b}");
    let _ = std::fs::remove_file(&h.path);
}
