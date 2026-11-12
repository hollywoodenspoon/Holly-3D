use super::bvh::{Bvh, Tri};
use super::job::raytrace_region;
use super::scene_build::RenderScene;
use super::trace::Pixel;
use super::*;
use crate::materials::Material;
use crate::primitives::{Primitive, PrimitiveKind};
use crate::scene::{BaseObject, Scene};
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::{Vec2, Vec3, Vec4};
use std::sync::atomic::AtomicBool;

fn tri(a: Vec3, b: Vec3, c: Vec3) -> Tri {
    Tri { p: [a, b, c], n: [Vec3::Z; 3], uv: [Vec2::ZERO; 3], col: [Vec4::ONE; 3], mat: 0, obj: 0 }
}

#[test]
fn bvh_matches_brute_force() {
    let mut rng = trace::Rng::new(7);
    let tris: Vec<Tri> = (0..300)
        .map(|_| {
            let c = rng.ball() * 5.0;
            tri(c + rng.ball(), c + rng.ball(), c + rng.ball())
        })
        .collect();
    let bvh = Bvh::build(tris.clone());
    for _ in 0..200 {
        let o = rng.ball() * 8.0;
        let d = rng.ball().normalize_or(Vec3::X);
        let brute = Bvh::build(vec![]);
        assert!(brute.closest(o, d, f32::MAX).is_none());
        let best =
            tris.iter().filter_map(|t| Bvh::build(vec![t.clone()]).closest(o, d, f32::MAX).map(|h| h.t)).fold(f32::INFINITY, f32::min);
        let got = bvh.closest(o, d, f32::MAX).map_or(f32::INFINITY, |h| h.t);
        assert!((best - got).abs() < 1e-3 || (best.is_infinite() && got.is_infinite()), "{best} vs {got}");
        assert_eq!(bvh.occluded(o, d, f32::MAX), best.is_finite());
    }
}

fn scene_with(kind: PrimitiveKind, mat: Option<Material>) -> Scene {
    let mut s = Scene::default();
    let id = s.add(BaseObject::Primitive(Primitive::default_of(kind)), Vec3::ZERO);
    if let Some(m) = mat {
        let mid = s.materials.mats.insert(m);
        s.objects[id].material = Some(mid);
    }
    s.objects[id].evaluate();
    s
}

fn top_cam() -> ViewCamera {
    let mut c = ViewCamera::new(ViewKind::Top);
    c.target = Vec3::ZERO;
    c.ortho_height = 4.0;
    c
}

fn render(s: &Scene, cfg: &RenderSettings, rig: bool) -> Vec<Pixel> {
    let rs = RenderScene::build(s, top_cam(), rig);
    raytrace_region(&rs, cfg, [0, 0, cfg.width, cfg.height], &AtomicBool::new(false), |_, _| {})
}

fn small() -> RenderSettings {
    RenderSettings { width: 32, height: 32, aa: 1, outlines: false, ..Default::default() }
}

#[test]
fn lit_plane_and_background() {
    let s = scene_with(PrimitiveKind::Plane, None);
    let cfg = small();
    let px = render(&s, &cfg, true);
    let center = px[16 * 32 + 16];
    assert!(center.display.truncate().min_element() > 0.1, "plane is lit by the rig: {:?}", center.display);
    assert_eq!(center.g.obj, 0);
    let corner = px[0];
    assert_eq!(corner.g.obj, u32::MAX, "4-unit view shows background around the 2-unit plane");
    let bg = Vec3::from_array(s.environment.background);
    assert!(corner.display.truncate().distance(bg) < 0.01, "background shows the Environment color exactly");
    let mut t = small();
    t.transparent = true;
    let px = render(&s, &t, true);
    assert_eq!(px[0].display.w, 0.0, "transparent background has zero alpha");
    assert!(px[16 * 32 + 16].display.w > 0.99);
}

fn brightness(px: &[Pixel]) -> f32 {
    px.iter().map(|p| p.hdr.truncate().element_sum()).sum()
}

#[test]
fn shadows_darken_the_ground() {
    let mut s = scene_with(PrimitiveKind::Plane, None);
    let id = s.objects.keys().next().unwrap();
    s.objects[id].transform.scale = Vec3::splat(3.0);
    s.objects[id].evaluate();
    let b = s.add(BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Box)), Vec3::new(0.0, 0.0, 0.5));
    s.objects[b].evaluate();
    let mut l = Light::new(LightKind::FreeDirect);
    l.cast_shadows = true;
    s.add_light(l, None, Vec3::new(3.0, 0.0, 3.0), Vec3::ZERO);
    let on = render(&s, &small(), false);
    let off = render(&s, &RenderSettings { shadows: false, ..small() }, false);
    assert!(brightness(&on) < brightness(&off) * 0.97, "{} vs {}", brightness(&on), brightness(&off));
}

#[test]
fn ink_outline_on_any_material() {
    let mut m = Material::named("Inked");
    m.outline.enabled = true;
    m.outline.width = 3.0;
    m.outline.color = [1.0, 0.0, 1.0];
    let s = scene_with(PrimitiveKind::Sphere, Some(m));
    let ink = |px: &[Pixel]| px.iter().filter(|p| p.display.x > 0.9 && p.display.y < 0.1 && p.display.z > 0.9).count();
    let with = render(&s, &RenderSettings { outlines: true, ..small() }, true);
    let without = render(&s, &small(), true);
    assert!(ink(&with) > 20, "silhouette ring drawn: {}", ink(&with));
    assert_eq!(ink(&without), 0);
}

#[test]
fn scenes_saved_with_the_cycles_bridge_load_as_the_path_tracer() {
    // Render Setup as written before Phase 11.5 (Cycles via Blender, HIP).
    let legacy = r#"(width: 640, height: 480, engine: Cycles, cycles_exe: "cycles", cycles_samples: 128,
        cycles_via: Blender, blender_exe: "/opt/blender/blender", cycles_device: Hip, hiprt: true, denoise: true, aa: 3)"#;
    let r: RenderSettings = ron::from_str(legacy).expect("old Render Setup still loads");
    assert_eq!((r.width, r.height, r.aa), (640, 480, 3));
    assert_eq!(r.path.integrator, crate::render::Integrator::PathTracer, "native path tracer replaces Cycles");
    assert_eq!(r.path.device, crate::render::ComputeDevice::Auto);
    let again: RenderSettings = ron::from_str(&ron::to_string(&r).unwrap()).unwrap();
    assert_eq!(again, r);
}

#[test]
fn frame_buffer_blocks_and_files() {
    let mut fb = FrameBuffer::new(4, 3);
    let row = vec![Vec4::new(1.0, 0.5, 0.0, 1.0); 2];
    fb.put_block(1, 1, 2, &row, &row);
    assert_eq!(fb.display[5], row[0]);
    assert_eq!(fb.display[4], Vec4::ZERO);
    assert_eq!(fb.rev, 1);
    let v = fb.view_rgba8(ChannelView { g: false, b: false, ..Default::default() });
    assert_eq!(&v[20..24], &[255, 0, 0, 255], "R-only view");
    let dir = std::env::temp_dir().join(format!("minimax_fb_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    for ext in ["png", "jpg", "exr"] {
        let p = dir.join(format!("t.{ext}"));
        fb.save(p.to_str().unwrap()).unwrap();
        let img = image::open(&p).unwrap();
        assert_eq!((img.width(), img.height()), (4, 3), "{ext}");
    }
    assert!(fb.save("x.bmpx").is_err());
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn hdri_background_and_image_based_ambient() {
    use crate::render::hdri::{HdrImage, Hdri};
    let mut s = scene_with(PrimitiveKind::Plane, None);
    let cfg = small();
    let plain = render(&s, &cfg, false);
    // Sky blue above, green ground below: the Top camera's misses see the ground.
    let px = (0..32).map(|i| if i < 16 { Vec3::new(0.2, 0.3, 1.0) } else { Vec3::new(0.1, 0.8, 0.1) }).collect();
    let mut h = Hdri::with_image("t.hdr".into(), HdrImage { w: 4, h: 8, px });
    h.lighting = false;
    s.environment.hdri = Some(h);
    let px = render(&s, &cfg, false);
    let c = px[0].hdr.truncate();
    assert!(c.y > 0.7 && c.x < 0.2, "background shows the HDRI ground: {c}");
    assert_eq!(px[16 * 32 + 16].hdr, plain[16 * 32 + 16].hdr, "lighting off: the plane is unchanged");
    s.environment.hdri.as_mut().unwrap().lighting = true;
    let lit = render(&s, &cfg, false);
    assert!(lit[16 * 32 + 16].hdr.y > plain[16 * 32 + 16].hdr.y + 0.05, "image-based ambient brightens the plane");
}
