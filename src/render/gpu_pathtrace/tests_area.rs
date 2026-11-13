//! PR 4: area lights (Rect, Disc, Sphere, Tube) and IES profiles on the
//! GPU. The kernel's photometric webs match the CPU to float precision, and
//! lit scenes (soft shadows, lights seen by the camera, IES-shaped spots)
//! converge to the CPU image.

use super::features::{Features, reasons};
use super::tests::{add, parity};
use super::{gpu, read_back, storage};
use crate::materials::Material;
use crate::primitives::PrimitiveKind;
use crate::render::scene_build::RenderScene;
use crate::render::{IesProfile, Light, LightKind};
use crate::scene::Scene;
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::Vec3;

const ENTRY: &str = r"
@group(0) @binding(10) var<storage, read> ies_in: array<vec4<f32>>;
@group(0) @binding(11) var<storage, read_write> ies_out: array<vec4<f32>>;

@compute @workgroup_size(16)
fn ies_web(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= arrayLength(&ies_out)) { return; }
    let q = ies_in[i];
    ies_out[i] = vec4<f32>(ies(u32(q.x), q.y), 0.0, 0.0, 0.0);
}
";

#[test]
fn ies_webs_match_the_cpu() {
    let g = match gpu() {
        Ok(g) => g,
        Err(e) if std::env::var_os("MINIMAX_REQUIRE_GPU").is_none() => return eprintln!("skipping IES parity: {e}"),
        Err(e) => panic!("{e}"),
    };
    let pts: Vec<[f32; 4]> = (0..IesProfile::ALL.len()).flat_map(|p| (0..=200).map(move |i| [p as f32, 1.0 - i as f32 / 100.0, 0.0, 0.0])).collect();
    let src = format!("{}\n{ENTRY}", super::source());
    let d = &g.device;
    let module = d.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("ies"), source: wgpu::ShaderSource::Wgsl(src.into()) });
    let pipe = d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor { label: Some("ies"), layout: None, module: &module, entry_point: Some("ies_web"), compilation_options: Default::default(), cache: None });
    let bytes: Vec<u8> = pts.iter().flatten().flat_map(|f| f.to_le_bytes()).collect();
    let size = (pts.len() * 16) as u64;
    let inp = storage(d, "ies in", &bytes, false);
    let out = storage(d, "ies out", &vec![0; size as usize], true);
    let entries = [(10, &inp), (11, &out)].map(|(b, buf)| wgpu::BindGroupEntry { binding: b, resource: buf.as_entire_binding() });
    let bind = d.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("ies"), layout: &pipe.get_bind_group_layout(0), entries: &entries });
    let mut enc = d.create_command_encoder(&Default::default());
    {
        let mut cp = enc.begin_compute_pass(&Default::default());
        cp.set_pipeline(&pipe);
        cp.set_bind_group(0, &bind, &[]);
        cp.dispatch_workgroups((pts.len() as u32).div_ceil(16), 1, 1);
    }
    g.queue.submit([enc.finish()]);
    let res = read_back(g, &out, size).expect("IES results");
    for (p, r) in pts.iter().zip(&res) {
        let prof = IesProfile::ALL[p[0] as usize];
        assert_eq!(super::pack::ies_id(prof), p[0], "packed id is the ALL index");
        let c = prof.factor(p[1]);
        assert!((r.x - c).abs() < 2e-3, "{prof:?} at cos {}: GPU {} vs CPU {c}", p[1], r.x);
    }
}

/// A floor, a box casting a shadow and a dim ambient, lit by `build`'s lights.
fn lit(build: impl FnOnce(&mut Scene)) -> RenderScene {
    let mut s = Scene::default();
    (s.environment.ambient_level, s.environment.background) = (0.02, [0.05, 0.06, 0.08]);
    add(&mut s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(3.0), Material { base_color: [0.7; 3], roughness: 0.7, ..Material::named("Floor") });
    add(&mut s, PrimitiveKind::Box, Vec3::new(0.2, 0.1, 0.0), Vec3::splat(0.4), Material { base_color: [0.8, 0.4, 0.2], roughness: 0.3, ..Material::named("Block") });
    build(&mut s);
    let mut c = ViewCamera::new(ViewKind::Perspective);
    (c.target, c.distance) = (Vec3::new(0.0, 0.0, 0.5), 6.0);
    let rs = RenderScene::build(&s, c, false);
    assert_eq!(reasons(&rs), Vec::<&str>::new(), "area lights and IES no longer need the CPU");
    rs
}

fn area(kind: LightKind, mult: f32, size: [f32; 2], visible: bool) -> Light {
    let mut l = Light::new(kind);
    (l.multiplier, l.area_size, l.visible_to_camera) = (mult, size, visible);
    l
}

/// A large softbox panel and a disc, both hidden from the camera.
#[test]
fn rect_and_disc_lights_render_on_the_gpu() {
    let rs = lit(|s| {
        s.add_light(area(LightKind::RectArea, 3.0, [1.6, 1.0], false), None, Vec3::new(-0.5, 0.0, 2.2), Vec3::ZERO);
        s.add_light(area(LightKind::DiscArea, 1.5, [0.9, 0.9], false), None, Vec3::new(1.5, -1.2, 0.9), Vec3::ZERO);
    });
    assert_ne!(Features::of(&rs).0 & Features::AREA, 0);
    // Smooth, well-lit floor: hold the image to a tighter error than the
    // generic check, so a skewed disc sample (centre-heavy) shows up.
    if let Some(d) = parity("rect and disc lights", &rs) {
        assert!(d.mean_err < 0.025, "rect and disc lights: mean block error {}", d.mean_err);
    }
}

/// A glowing sphere and a neon tube in view of the camera.
#[test]
fn sphere_and_tube_lights_seen_by_the_camera_render_on_the_gpu() {
    let rs = lit(|s| {
        s.add_light(area(LightKind::SphereArea, 2.0, [0.25, 0.25], true), None, Vec3::new(-0.6, -0.4, 1.0), Vec3::ZERO);
        s.add_light(area(LightKind::TubeArea, 2.0, [0.08, 1.4], true), None, Vec3::new(0.4, 0.6, 1.3), Vec3::ZERO);
    });
    parity("sphere and tube lights", &rs);
}

/// IES webs on a spot and on a downward panel.
#[test]
fn ies_shaped_lights_render_on_the_gpu() {
    let rs = lit(|s| {
        let mut spot = Light::new(LightKind::FreeSpot);
        (spot.multiplier, spot.ies) = (3.0, IesProfile::TacticalFlashlight);
        s.add_light(spot, None, Vec3::new(-0.3, -0.2, 2.6), Vec3::ZERO);
        let mut panel = area(LightKind::RectArea, 3.0, [0.5, 0.5], true);
        panel.ies = IesProfile::WallScallop;
        s.add_light(panel, None, Vec3::new(0.8, 0.5, 2.0), Vec3::ZERO);
    });
    parity("IES spot and panel", &rs);
}
