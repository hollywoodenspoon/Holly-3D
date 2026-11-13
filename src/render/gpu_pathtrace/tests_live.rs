//! PR 4c: live maps. Prefab Library materials whose maps are 3D (noise,
//! cracks, grunge, wood) or never repeat under triplanar projection
//! (bricks) are evaluated by the kernel per hit: the evaluator matches the
//! CPU map by map, and Settlement scenes render on the GPU.

use super::features::{CPU_ONLY, reasons};
use super::live;
use super::tests::{add, parity};
use super::{gpu, read_back, storage};
use crate::materials::maps::{MapChannel, MapNode, MapSlot, NoiseKind};
use crate::materials::patterns::{BrickKind, GrungeKind};
use crate::materials::relief::base_tangent_normal;
use crate::materials::shading::base_surface;
use crate::materials::triplanar::Projection;
use crate::materials::{Material, MultiPreset};
use crate::primitives::PrimitiveKind;
use crate::render::scene_build::RenderScene;
use crate::render::{Light, LightKind};
use crate::scene::Scene;
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::{Vec2, Vec3, Vec4};

const ENTRY: &str = r"
@group(0) @binding(10) var<storage, read> live_in: array<vec4<f32>>;
@group(0) @binding(11) var<storage, read_write> live_out: array<vec4<f32>>;

@compute @workgroup_size(16)
fn live_eval(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i * 3u >= arrayLength(&live_out)) { return; }
    let a = live_in[i * 4u];
    let s = live_shade(u32(a.x), a.yz, live_in[i * 4u + 1u].xyz, live_in[i * 4u + 2u].xyz, live_in[i * 4u + 3u].xyz);
    live_out[i * 3u] = s.a;
    live_out[i * 3u + 1u] = s.b;
    live_out[i * 3u + 2u] = s.c;
}
";

fn with(mut m: Material, ch: MapChannel, node: MapNode) -> Material {
    m.maps[ch as usize] = MapSlot { node: Some(node), ..Default::default() };
    m
}

/// Every kind and sub-kind the evaluator knows, one material each.
fn every_kind() -> Vec<Material> {
    let (a, b) = ([0.8, 0.5, 0.3], [0.1, 0.2, 0.4]);
    let mut nodes: Vec<MapNode> = [NoiseKind::Perlin, NoiseKind::Fractal, NoiseKind::Cellular].map(|kind| MapNode::Noise { kind, scale: 1.7, seed: 4, color_a: a, color_b: b }).into();
    nodes.extend(BrickKind::ALL.map(|kind| MapNode::Bricks { kind, cols: 4.0, rows: 8.0, gap: 0.08, vary: 0.4, seed: 2, color_a: a, color_b: b }));
    nodes.extend(GrungeKind::ALL.map(|kind| MapNode::Grunge { kind, scale: 1.3, amount: 0.6, seed: 5, color_a: a, color_b: b }));
    nodes.push(MapNode::Cracks { scale: 2.0, width: 0.1, vary: 0.3, seed: 6, color_a: a, color_b: b });
    nodes.push(MapNode::Wood { scale: 1.5, rings: 4.0, grain: 0.6, light: a, dark: b });
    nodes.push(MapNode::Checker { tiles: 3.0, color_a: a, color_b: b });
    nodes.extend([false, true].map(|radial| MapNode::Gradient { color_a: a, color_b: b, color_c: [0.9, 0.9, 0.1], mid: 0.4, radial }));
    nodes.extend([(false, false, 0.0), (false, true, 3.0), (true, false, 0.0)].map(|(soft, along_u, dash)| MapNode::Stripes { count: 5.0, width: 0.3, dash, soft, along_u, color_a: a, color_b: b }));
    nodes
        .into_iter()
        .map(|n| {
            // A 3D map on the colour, the same map as relief, roughness and glow.
            let m = Material { emissive: [0.2, 0.1, 0.0], emissive_strength: 2.0, ..Material::named("Kind") };
            let m = with(with(m, MapChannel::BaseColor, n.clone()), MapChannel::Bump, n.clone());
            with(with(m, MapChannel::Roughness, n.clone()), MapChannel::Emission, n)
        })
        .collect()
}

/// The prefab materials that cannot be baked (all of them live now).
fn prefab_live() -> Vec<(String, Material)> {
    let mut out = Vec::new();
    for p in [MultiPreset::Settlement, MultiPreset::MedievalCastle, MultiPreset::ModernCity, MultiPreset::AlpineTerrain] {
        for s in p.build().subs {
            if super::bake::why_not(&s.mat).is_some() && s.mat.active_map(MapChannel::Opacity).is_none() {
                out.push((format!("{p:?} {} {}", s.id, s.name), s.mat));
            }
        }
    }
    out
}

#[test]
fn prefab_materials_no_longer_need_the_cpu() {
    let mats = prefab_live();
    assert!(mats.len() >= 14, "the Settlement pack alone has 13 position-based or non-repeating materials: {}", mats.len());
    for (name, m) in &mats {
        assert!(live::supported(m), "{name} evaluates on the GPU");
    }
    let bitmap = with(Material::named("Bitmap"), MapChannel::Roughness, MapNode::Bitmap { path: String::new(), tex: None });
    let mixed = with(bitmap, MapChannel::BaseColor, MapNode::Cracks { scale: 2.0, width: 0.1, vary: 0.3, seed: 6, color_a: [1.0; 3], color_b: [0.0; 3] });
    assert!(!live::supported(&mixed), "a bitmap mixed with a 3D map stays on the CPU");
    assert_eq!(live::tables().len(), live::SEEDS.len() * 256);
}

/// Evaluates `mats` at scattered points on the GPU and the CPU.
fn compare(mats: &[(String, Material)]) {
    let g = match gpu() {
        Ok(g) => g,
        Err(e) if std::env::var_os("MINIMAX_REQUIRE_GPU").is_none() => return eprintln!("skipping live-map parity: {e}"),
        Err(e) => panic!("{e}"),
    };
    let tables = (mats.len() * live::WORDS) as u32;
    let mut aux: Vec<u32> = mats.iter().flat_map(|(_, m)| live::record(m, tables)).collect();
    aux.extend(live::tables());
    let mut pts = Vec::new();
    for k in 0..mats.len() {
        for i in 0..96 {
            let f = i as f32;
            let uv = Vec2::new((f * 0.618_034).fract() * 4.0 - 2.0, (f * 0.414_214 + 0.3).fract() * 4.0 - 2.0);
            let p = Vec3::new((f * 0.754_878).fract() * 12.0 - 6.0, (f * 0.569_840 + 0.1).fract() * 12.0 - 6.0, (f * 0.311_7 + 0.7).fract() * 8.0 - 4.0);
            let (du, dv) = if i % 2 == 0 { (Vec3::Y * 0.5, Vec3::Z * 0.5) } else { (Vec3::new(0.3, 0.1, 0.0), Vec3::new(0.0, 0.2, 0.4)) };
            pts.push((k, uv, p, du, dv));
        }
    }
    let input: Vec<f32> = pts.iter().flat_map(|&(k, uv, p, du, dv)| [(k * live::WORDS) as f32, uv.x, uv.y, 0.0, p.x, p.y, p.z, 0.0, du.x, du.y, du.z, 0.0, dv.x, dv.y, dv.z, 0.0]).collect();
    let src = format!("{}\n{ENTRY}", super::source());
    let d = &g.device;
    let module = d.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("live"), source: wgpu::ShaderSource::Wgsl(src.into()) });
    let pipe = d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor { label: Some("live"), layout: None, module: &module, entry_point: Some("live_eval"), compilation_options: Default::default(), cache: None });
    let size = (pts.len() * 48) as u64;
    let aux_b = storage(d, "aux", &aux.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<_>>(), false);
    let inp = storage(d, "live in", &input.iter().flat_map(|f| f.to_le_bytes()).collect::<Vec<_>>(), false);
    let out = storage(d, "live out", &vec![0; size as usize], true);
    let entries = [(9, &aux_b), (10, &inp), (11, &out)].map(|(b, buf)| wgpu::BindGroupEntry { binding: b, resource: buf.as_entire_binding() });
    let bind = d.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("live"), layout: &pipe.get_bind_group_layout(0), entries: &entries });
    let mut enc = d.create_command_encoder(&Default::default());
    {
        let mut cp = enc.begin_compute_pass(&Default::default());
        cp.set_pipeline(&pipe);
        cp.set_bind_group(0, &bind, &[]);
        cp.dispatch_workgroups((pts.len() as u32).div_ceil(16), 1, 1);
    }
    g.queue.submit([enc.finish()]);
    let res = read_back(g, &out, size).expect("live results");
    let mut bad = vec![0usize; mats.len()];
    let mut worst = vec![0f32; mats.len()];
    for (j, &(k, uv, p, du, dv)) in pts.iter().enumerate() {
        let m = &mats[k].1;
        let s = base_surface(m, uv, p, Vec4::ONE);
        let n = base_tangent_normal(m, uv, p, du, dv).unwrap_or(Vec3::Z);
        let want = [s.base.extend(1.0), s.emissive.extend(s.metallic), Vec4::new(s.roughness, n.x, n.y, n.z)];
        let err = (0..3).map(|r| (res[j * 3 + r] - want[r]).abs().max_element()).fold(0.0, f32::max);
        worst[k] = worst[k].max(err);
        bad[k] += usize::from(err > 0.01);
    }
    for (k, (name, _)) in mats.iter().enumerate() {
        // A point on a cell border may round to the other cell in f32.
        assert!(bad[k] <= 2, "{name}: {} of 96 points differ (worst {})", bad[k], worst[k]);
    }
}

#[test]
fn every_live_map_kind_matches_the_cpu() {
    let mats: Vec<_> = every_kind().into_iter().enumerate().map(|(i, m)| (format!("kind {i} {:?}", m.maps[0].node.as_ref().map(MapNode::kind_name)), m)).collect();
    compare(&mats);
}

#[test]
fn prefab_live_materials_match_the_cpu() {
    compare(&prefab_live());
}

#[test]
fn a_settlement_street_renders_on_the_gpu_like_the_cpu() {
    let subs = MultiPreset::Settlement.build().subs;
    let pick = |id: u16| subs.iter().find(|s| s.id == id).map(|s| s.mat.clone()).expect("pack material");
    let mut s = Scene::default();
    (s.environment.ambient_level, s.environment.background) = (0.05, [0.2, 0.25, 0.3]);
    add(&mut s, PrimitiveKind::Plane, Vec3::ZERO, Vec3::splat(3.0), pick(1));
    add(&mut s, PrimitiveKind::Box, Vec3::new(-0.6, 0.3, 0.0), Vec3::new(0.8, 0.6, 0.9), pick(3));
    add(&mut s, PrimitiveKind::Box, Vec3::new(0.7, -0.4, 0.0), Vec3::new(0.6, 0.6, 0.5), pick(22));
    add(&mut s, PrimitiveKind::Box, Vec3::new(0.6, 0.8, 0.0), Vec3::new(0.5, 0.4, 0.3), pick(16));
    let mut m = with(Material::named("Cobble wall"), MapChannel::Bump, MapNode::Cracks { scale: 3.0, width: 0.1, vary: 0.3, seed: 6, color_a: [1.0; 3], color_b: [0.0; 3] });
    m.projection = Projection::triplanar(0.5);
    add(&mut s, PrimitiveKind::Box, Vec3::new(-0.6, -0.8, 0.0), Vec3::new(0.5, 0.4, 0.6), m);
    s.add_light(Light::new(LightKind::Omni), None, Vec3::new(-1.0, -1.5, 2.5), Vec3::ZERO);
    let mut c = ViewCamera::new(ViewKind::Perspective);
    (c.target, c.distance) = (Vec3::new(0.0, 0.0, 0.3), 5.0);
    let rs = RenderScene::build(&s, c, false);
    assert!(!reasons(&rs).contains(&CPU_ONLY[3]), "{:?}", reasons(&rs));
    assert_eq!(super::textures::planned_bytes(&rs), 0, "live maps bake nothing");
    parity("Settlement street", &rs);
}
