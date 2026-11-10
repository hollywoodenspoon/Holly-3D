//! The Cycles-through-Blender bridge: the scene JSON, the loader script and
//! the process round trip (with a stand-in `blender`).

use super::scene_build::RenderScene;
use super::*;
use crate::primitives::{Primitive, PrimitiveKind};
use crate::scene::{BaseObject, Scene};
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::Vec3;
use std::sync::atomic::AtomicBool;

fn scene() -> (Scene, RenderScene) {
    let mut s = Scene::default();
    let id = s.add(BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Box)), Vec3::ZERO);
    s.objects[id].evaluate();
    s.add_light(Light::new(LightKind::TargetSpot), None, Vec3::new(2.0, 2.0, 3.0), Vec3::ZERO);
    let rs = RenderScene::build(&s, ViewCamera::new(ViewKind::Perspective), false);
    (s, rs)
}

#[test]
fn defaults_are_blender_hip_hiprt_and_oidn() {
    let r = RenderSettings::default();
    assert_eq!((r.cycles_via, r.cycles_device, r.hiprt, r.denoise), (CyclesVia::Blender, CyclesDevice::Hip, true, true));
    // Older scene files without the new fields still load with these defaults.
    let old: RenderSettings = ron::from_str("(width: 640, height: 480)").unwrap();
    assert_eq!((old.width, old.cycles_device, old.denoise), (640, CyclesDevice::Hip, true));
}

#[test]
fn blender_scene_json_describes_the_scene() {
    let (_, rs) = scene();
    let cfg = RenderSettings { width: 64, height: 48, ..Default::default() };
    let j = blender::scene_json(&rs, &cfg, std::path::Path::new("/tmp/out.png"));
    assert_eq!((j["device"].as_str(), j["hiprt"].as_bool(), j["denoise"].as_bool()), (Some("HIP"), Some(true), Some(true)));
    assert_eq!(j["camera"]["ortho"], false);
    let tris: usize = j["meshes"].as_array().unwrap().iter().map(|m| m["verts"].as_array().unwrap().len() / 9).sum();
    assert_eq!(tris, rs.bvh.tris.len(), "every triangle exported");
    let m0 = &j["meshes"][0];
    assert_eq!(m0["verts"].as_array().unwrap().len(), m0["normals"].as_array().unwrap().len());
    let l = &j["lights"][0];
    assert_eq!(l["type"], "SPOT");
    assert!(l["energy"].as_f64().unwrap() > 0.0 && l["spot"].as_array().unwrap().len() == 2);
    // The camera matrix maps Blender's -Z view axis onto our forward.
    let m = &j["camera"]["matrix"];
    let col = |c: usize| Vec3::new(m[0][c].as_f64().unwrap() as f32, m[1][c].as_f64().unwrap() as f32, m[2][c].as_f64().unwrap() as f32);
    assert!((-col(2)).distance(rs.camera.forward()) < 1e-5 && col(3).distance(rs.camera.eye()) < 1e-4);
    // HIP-RT only applies to the HIP device.
    let cpu = RenderSettings { cycles_device: CyclesDevice::Cpu, ..cfg };
    assert_eq!(blender::scene_json(&rs, &cpu, std::path::Path::new("x.png"))["hiprt"], false);
    // The loader reads every key the exporter writes and sets up HIP-RT / OIDN.
    for key in ["\"device\"", "\"hiprt\"", "\"denoise\"", "\"meshes\"", "\"lights\"", "\"world\"", "\"output\""] {
        assert!(blender::LOADER.contains(key), "loader reads {key}");
    }
    assert!(blender::LOADER.contains("use_hiprt") && blender::LOADER.contains("OPENIMAGEDENOISE"));
}

#[cfg(unix)]
#[test]
fn blender_round_trip_with_a_stand_in_executable() {
    use std::os::unix::fs::PermissionsExt;
    let (_, rs) = scene();
    let dir = std::env::temp_dir().join("minimax_fake_blender");
    std::fs::create_dir_all(&dir).unwrap();
    let png = dir.join("result.png");
    image::RgbaImage::from_pixel(8, 6, image::Rgba([10, 200, 30, 255])).save(&png).unwrap();
    // Checks the arguments, then "renders" by copying the PNG next to the scene file.
    let exe = dir.join("blender");
    let script = format!(
        "#!/bin/sh\ncase \"$*\" in *'-E CYCLES'*'--python '*loader.py*' -- '*scene.json) ;; *) echo bad args >&2; exit 3;; esac\n\
         for a in \"$@\"; do last=\"$a\"; done\ncp '{}' \"$(dirname \"$last\")/render.png\"\n",
        png.display()
    );
    std::fs::write(&exe, script).unwrap();
    std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
    let cfg = RenderSettings { blender_exe: exe.display().to_string(), width: 8, height: 6, ..Default::default() };
    let fb = cycles::run(&rs, &cfg, &AtomicBool::new(false)).unwrap();
    assert_eq!((fb.width, fb.height), (8, 6));
    // A missing executable is a readable error.
    let bad = RenderSettings { blender_exe: "/nonexistent/blender".into(), ..cfg };
    let err = cycles::run(&rs, &bad, &AtomicBool::new(false)).unwrap_err();
    assert!(err.contains("Could not start") && err.contains("F10"), "{err}");
}

/// Real Blender (`cargo test --lib real_blender -- --ignored`): renders the
/// box + spot scene through the loader script; on a machine without the
/// GPU device it falls back to the CPU.
#[test]
#[ignore]
fn real_blender_renders_the_scene() {
    let (_, mut rs) = scene();
    // An HDRI world when one is lying around (written by the save / open test).
    let sky = std::env::temp_dir().join("minimax_hdri_test/sky.hdr");
    rs.hdri = crate::render::hdri::Hdri::open(&sky).ok();
    let cfg = RenderSettings { width: 96, height: 72, cycles_samples: 8, ..Default::default() };
    let fb = cycles::run(&rs, &cfg, &AtomicBool::new(false)).unwrap();
    assert_eq!((fb.width, fb.height), (96, 72));
    let out = std::env::temp_dir().join("minimax_real_blender.png");
    fb.save(&out.display().to_string()).unwrap();
}
