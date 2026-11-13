//! Check renders for the prefab thumbnail sheets (not part of `cargo test`):
//! `PREFAB_SHOTS=/some/dir PREFAB_CAT=Residential cargo test --lib prefab_check_renders -- --ignored`
//! writes one 3/4 daylight render per kind of that category.

use super::*;
use crate::editor::Editor;
use crate::generators::WorldKind;
use crate::render::RenderSettings;
use crate::render::job::raytrace_region;
use crate::render::scene_build::RenderScene;
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::Vec3;
use std::sync::atomic::AtomicBool;

#[test]
#[ignore]
fn prefab_check_renders() {
    let Ok(dir) = std::env::var("PREFAB_SHOTS") else { return };
    let cat = std::env::var("PREFAB_CAT").unwrap_or_default();
    for k in Category::ALL.into_iter().filter(|c| format!("{c:?}") == cat).flat_map(|c| c.kinds()) {
        let mut ed = Editor::default();
        ed.create_world(WorldKind::Building(k), Vec3::ZERO);
        let Some(id) = ed.current() else { continue };
        let b = ed.scene.objects[id].evaluated_mesh.bounds().unwrap();
        let size = (b.max - b.min).length();
        let mut cam = ViewCamera::new(ViewKind::Perspective);
        let target = (b.min + b.max) * 0.5;
        let pitch: f32 = std::env::var("PREFAB_PITCH").ok().and_then(|s| s.parse().ok()).unwrap_or(-22.0);
        (cam.target, cam.distance, cam.yaw, cam.pitch) = (target, size * 1.15, (62f32).to_radians(), pitch.to_radians());
        let cfg = RenderSettings { width: 480, height: 360, aa: 2, ..Default::default() };
        let rs = RenderScene::build(&ed.scene, cam, true);
        let px = raytrace_region(&rs, &cfg, [0, 0, cfg.width, cfg.height], &AtomicBool::new(false), |_, _| {});
        let mut img = image::RgbImage::new(cfg.width, cfg.height);
        for (i, p) in px.iter().enumerate() {
            let c = p.display.truncate().clamp(Vec3::ZERO, Vec3::ONE) * 255.0;
            img.put_pixel(i as u32 % cfg.width, i as u32 / cfg.width, image::Rgb([c.x as u8, c.y as u8, c.z as u8]));
        }
        img.save(format!("{dir}/{:02}_{k:?}.png", k as usize)).expect("write the check render");
    }
}
