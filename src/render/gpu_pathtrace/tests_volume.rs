//! PR 3: godrays and height mist on the GPU. The kernel's wisp noise and
//! phase function match the CPU to float precision, and lit mist scenes
//! (shafts cut by geometry, spot cones, height falloff, wisps) converge to
//! the CPU image.

use super::features::{Features, reasons};
use super::tests::{add, parity};
use super::{gpu, read_back, storage};
use crate::materials::Material;
use crate::primitives::PrimitiveKind;
use crate::render::scene_build::RenderScene;
use crate::render::volumetrics::{VolumeSettings, hg_phase};
use crate::render::{Light, LightKind};
use crate::scene::Scene;
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::{Vec3, Vec4};

const ENTRY: &str = r"
@group(0) @binding(10) var<storage, read> vol_in: array<vec4<f32>>;
@group(0) @binding(11) var<storage, read_write> vol_out: array<vec4<f32>>;

@compute @workgroup_size(16)
fn vol(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i >= arrayLength(&vol_out)) { return; }
    let q = vol_in[i];
    vol_out[i] = vec4<f32>(fbm3(q.xyz), hg_phase(q.w, 0.6), hg_phase(q.w, -0.3), 0.0);
}
";

#[test]
fn wisp_noise_and_phase_match_the_cpu() {
    let g = match gpu() {
        Ok(g) => g,
        Err(e) if std::env::var_os("MINIMAX_REQUIRE_GPU").is_none() => return eprintln!("skipping volume parity: {e}"),
        Err(e) => panic!("{e}"),
    };
    // Points on both sides of zero (negative cells hash through the u32 cast).
    let pts: Vec<Vec4> = (0..256).map(|i| {
        let f = i as f32;
        Vec4::new((f * 0.731).sin() * 9.0, (f * 1.37).cos() * 7.0 - 2.0, f * 0.113 - 10.0, (f * 0.29).sin())
    }).collect();
    let src = format!("{}\n{ENTRY}", super::source());
    let d = &g.device;
    let module = d.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("vol"), source: wgpu::ShaderSource::Wgsl(src.into()) });
    let pipe = d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor { label: Some("vol"), layout: None, module: &module, entry_point: Some("vol"), compilation_options: Default::default(), cache: None });
    let bytes: Vec<u8> = pts.iter().flat_map(|v| v.to_array()).flat_map(f32::to_le_bytes).collect();
    let size = (pts.len() * 16) as u64;
    let inp = storage(d, "vol in", &bytes, false);
    let out = storage(d, "vol out", &vec![0; size as usize], true);
    let entries = [(10, &inp), (11, &out)].map(|(b, buf)| wgpu::BindGroupEntry { binding: b, resource: buf.as_entire_binding() });
    let bind = d.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("vol"), layout: &pipe.get_bind_group_layout(0), entries: &entries });
    let mut enc = d.create_command_encoder(&Default::default());
    {
        let mut cp = enc.begin_compute_pass(&Default::default());
        cp.set_pipeline(&pipe);
        cp.set_bind_group(0, &bind, &[]);
        cp.dispatch_workgroups((pts.len() as u32).div_ceil(16), 1, 1);
    }
    g.queue.submit([enc.finish()]);
    let res = read_back(g, &out, size).expect("volume results");
    for (p, r) in pts.iter().zip(&res) {
        let n = crate::render::sky_noise::fbm(p.truncate(), 3);
        assert!((r.x - n).abs() < 1e-4, "fbm at {p}: GPU {} vs CPU {n}", r.x);
        for (gv, g) in [(r.y, 0.6), (r.z, -0.3)] {
            let c = hg_phase(p.w, g);
            assert!((gv - c).abs() <= 1e-4 * c.max(1.0), "phase {g} at {}: GPU {gv} vs CPU {c}", p.w);
        }
    }
}

fn misty(v: VolumeSettings, build: impl FnOnce(&mut Scene)) -> RenderScene {
    let mut s = Scene::default();
    (s.environment.ambient_level, s.environment.background) = (0.0, [0.05, 0.06, 0.08]);
    add(&mut s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(3.0), Material { base_color: [0.6; 3], roughness: 0.8, ..Material::named("Floor") });
    build(&mut s);
    s.environment.volume = VolumeSettings { enabled: true, ..v };
    let mut c = ViewCamera::new(ViewKind::Perspective);
    (c.target, c.distance) = (Vec3::new(0.0, 0.0, 0.6), 6.0);
    RenderScene::build(&s, c, false)
}

/// The camera looks at a wall with a low sun behind it: one shaft through a gap
/// cuts through mist that is otherwise in shadow; forward scattering, height falloff.
#[test]
fn sun_shafts_through_mist_render_on_the_gpu() {
    let v = VolumeSettings { density: 0.35, anisotropy: 0.7, falloff: 0.4, intensity: 4.0, ..Default::default() };
    let rs = misty(v, |s| {
        let wall = Material { base_color: [0.3, 0.4, 0.8], ..Material::named("Wall") };
        add(s, PrimitiveKind::Box, Vec3::new(1.2, -1.7, 0.0), Vec3::new(0.15, 3.0, 3.0), wall.clone());
        add(s, PrimitiveKind::Box, Vec3::new(1.2, 1.9, 0.0), Vec3::new(0.15, 3.0, 3.0), wall);
        s.add_light(Light::new(LightKind::FreeDirect), None, Vec3::new(-8.0, 0.5, 3.0), Vec3::ZERO);
    });
    assert_eq!(reasons(&rs), Vec::<&str>::new(), "volumes no longer need the CPU");
    assert_ne!(Features::of(&rs).0 & Features::VOLUME, 0);
    parity("sun shafts in mist", &rs);
}

/// A spot cone and an omni in patchy, drifting wisps with backward scattering.
#[test]
fn spot_cone_in_wisps_renders_on_the_gpu() {
    let v = VolumeSettings { density: 0.5, anisotropy: -0.3, wisps: 0.9, wisp_scale: 0.7, wind: [0.4, -1.1], steps: 32, color: [0.9, 0.8, 1.0], ..Default::default() };
    let rs = misty(v, |s| {
        add(s, PrimitiveKind::Cylinder, Vec3::new(0.3, -0.2, 0.0), Vec3::splat(0.4), Material { base_color: [0.9, 0.5, 0.2], ..Material::named("Post") });
        let mut spot = Light::new(LightKind::FreeSpot);
        spot.multiplier = 3.0;
        s.add_light(spot, None, Vec3::new(-0.4, 0.3, 2.4), Vec3::ZERO);
        s.add_light(Light::new(LightKind::Omni), None, Vec3::new(1.0, 1.0, 1.0), Vec3::ZERO);
    });
    parity("spot cone in wisps", &rs);
}
