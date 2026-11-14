//! Brush Strokes wiring: the painted normals reach every renderer's path.

use super::brush::{BrushStrokes, PRESETS};
use super::*;
use bevy::math::{Vec2, Vec3};

#[test]
fn brush_strokes_reach_relief_bakes_and_the_gpu() {
    let mut m = Material::default();
    assert!(!m.brush.enabled, "off by default");
    assert!(!bake::has_relief(&m));
    m.brush = PRESETS[2].1.clone();
    assert!(bake::has_relief(&m), "the viewport bakes a normal map");
    let uv = Vec2::new(0.31, 0.47);
    let n = relief::base_tangent_normal(&m, uv, Vec3::ZERO, Vec3::X, Vec3::Y).expect("painted normals give a tangent normal");
    assert!((n.length() - 1.0).abs() < 1e-4 && n.z > 0.5);
    assert!((n - Vec3::Z).length() > 1e-3, "the surface is tilted");
    assert!(crate::render::gpu_pathtrace::bake::why_not(&m).is_none(), "the GPU path tracer can bake it");
    m.projection.mapping = Mapping::Triplanar;
    assert!(crate::render::gpu_pathtrace::bake::why_not(&m).is_none(), "and on Triplanar (strokes repeat every tile)");
}

/// Check renders (not part of `cargo test`):
/// `BRUSH_SHOTS=/some/dir cargo test --lib brush_check_renders -- --ignored`
/// renders a sphere, torus and box per look and times the normal bake.
#[test]
#[ignore]
fn brush_check_renders() {
    use crate::editor::Editor;
    use crate::primitives::PrimitiveKind;
    use crate::render::RenderSettings;
    use crate::render::job::raytrace_region;
    use crate::render::scene_build::RenderScene;
    use crate::viewport::camera::{ViewCamera, ViewKind};
    use std::sync::atomic::AtomicBool;
    let Ok(dir) = std::env::var("BRUSH_SHOTS") else { return };
    let looks = [("Off", BrushStrokes::default())].into_iter().chain(PRESETS.iter().cloned());
    for (name, look) in looks {
        let mut ed = Editor::default();
        let mut size = 0.0f32;
        for (i, k) in [PrimitiveKind::Sphere, PrimitiveKind::Torus, PrimitiveKind::Box].into_iter().enumerate() {
            ed.create_primitive(k, Vec3::ZERO);
            let id = ed.current().unwrap();
            let b = ed.scene.objects[id].evaluated_mesh.bounds().unwrap();
            size = size.max((b.max - b.min).max_element());
            ed.scene.objects[id].transform.translation.x = (i as f32 - 1.0) * 1.3;
        }
        for o in ed.scene.objects.values_mut() {
            o.transform.translation.x *= size;
        }
        let mut m = Preset::Clay.material();
        m.base_color = [0.75, 0.42, 0.28];
        m.brush = look.clone();
        let t = std::time::Instant::now();
        let _ = bake::bake_normal(&m, 1024);
        println!("{name}: 1024² normal bake {:?}", t.elapsed());
        let id = ed.scene.materials.mats.insert(m);
        for o in ed.scene.objects.values_mut() {
            o.material = Some(id);
        }
        let mut cam = ViewCamera::new(ViewKind::Perspective);
        (cam.target, cam.distance, cam.yaw, cam.pitch) = (Vec3::new(0.0, 0.0, size * 0.3), size * 3.0, (-60f32).to_radians(), (-20f32).to_radians());
        let cfg = RenderSettings { width: 900, height: 360, aa: 3, ..Default::default() };
        let rs = RenderScene::build(&ed.scene, cam, true);
        let px = raytrace_region(&rs, &cfg, [0, 0, cfg.width, cfg.height], &AtomicBool::new(false), |_, _| {});
        let mut img = image::RgbImage::new(cfg.width, cfg.height);
        for (i, p) in px.iter().enumerate() {
            let c = p.display.truncate().clamp(Vec3::ZERO, Vec3::ONE) * 255.0;
            img.put_pixel(i as u32 % cfg.width, i as u32 / cfg.width, image::Rgb([c.x as u8, c.y as u8, c.z as u8]));
        }
        img.save(format!("{dir}/brush_{}.png", name.replace(' ', "_"))).expect("write the check render");
    }
}
