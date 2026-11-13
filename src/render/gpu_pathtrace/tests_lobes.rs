//! Per-lobe parity: the kernel's Principled BSDF (`pr_eval` / `pr_sample`)
//! run on the GPU for random materials and directions against the CPU
//! `Bsdf`. Evaluation must match to float precision; the mean sample
//! weight (the directional albedo, a white-furnace check of sampling
//! against its own density) must match statistically.

use super::{gpu, read_back, storage, textures};
use crate::materials::principled::Principled;
use crate::materials::shading::Surface;
use crate::render::bsdf::Bsdf;
use crate::render::trace::Rng;
use bevy::math::{Vec3, Vec4};

const ENTRY: &str = r"
@group(0) @binding(10) var<storage, read> lobe_in: array<vec4<f32>>;
@group(0) @binding(11) var<storage, read_write> lobe_out: array<vec4<f32>>;

@compute @workgroup_size(16)
fn lobes(@builtin(global_invocation_id) id: vec3<u32>) {
    let i = id.x;
    if (i * 2u >= arrayLength(&lobe_out)) { return; }
    let n = lobe_in[i * 5u].xyz;
    let v = lobe_in[i * 5u + 1u].xyz;
    let l = lobe_in[i * 5u + 2u].xyz;
    let bm = lobe_in[i * 5u + 3u];
    let r = lobe_in[i * 5u + 4u].x;
    var s = surf(n, bm.xyz, bm.w, max(r * r, 2e-3), 0.0, 1.5, true);
    s = principled(s, i * RECu, r);
    let e = pr_eval(s, v, l);
    rng_state = pcg(i * 7919u + 17u);
    var sum = vec3<f32>(0.0);
    var sss = 0.0;
    for (var k = 0u; k < SAMPLES; k++) {
        let b = pr_sample(s, v);
        if (b.ok) { sum += b.weight; }
        if (b.sss) { sss += 1.0; }
    }
    lobe_out[i * 2u] = vec4<f32>(e.f, e.pdf);
    lobe_out[i * 2u + 1u] = vec4<f32>(sum / f32(SAMPLES), sss / f32(SAMPLES));
}
";
const SAMPLES: u32 = 1 << 16;

struct Case {
    n: Vec3,
    v: Vec3,
    l: Vec3,
    base: Vec3,
    metallic: f32,
    rough: f32,
    p: Principled,
}

fn unit(rng: &mut Rng) -> Vec3 {
    let z = 1.0 - 2.0 * rng.f();
    let r = (1.0 - z * z).max(0.0).sqrt();
    let a = std::f32::consts::TAU * rng.f();
    Vec3::new(r * a.cos(), r * a.sin(), z)
}

/// Every lobe combination, with random normals and directions (some below
/// the horizon), roughness and colours.
fn cases() -> Vec<Case> {
    let mut rng = Rng::new(41);
    (0..48)
        .map(|i| {
            let n = if i % 7 == 0 { Vec3::Z } else { unit(&mut rng) };
            let hemi = |rng: &mut Rng| {
                let d = unit(rng);
                if d.dot(n) < 0.1 { (d - n * d.dot(n) * 1.5).normalize() } else { d }
            };
            let (v, l) = (hemi(&mut rng), if i % 5 == 4 { -hemi(&mut rng) } else { hemi(&mut rng) });
            let on = |bit: usize| i >> bit & 1 == 1;
            let p = Principled {
                anisotropy: if on(0) { 0.3 + 0.7 * rng.f() } else { 0.0 },
                aniso_rotation: 360.0 * rng.f(),
                clearcoat: if on(1) { 0.5 + 0.5 * rng.f() } else { 0.0 },
                clearcoat_roughness: 0.02 + 0.4 * rng.f(),
                sheen: if on(2) { rng.f() } else { 0.0 },
                sheen_tint: rng.f(),
                sss_weight: if on(3) { 0.2 + 0.8 * rng.f() } else { 0.0 },
                ..Default::default()
            };
            let base = Vec3::new(rng.f(), rng.f(), rng.f()) * 0.9 + 0.05;
            Case { n, v, l, base, metallic: if on(4) { rng.f() } else { 0.0 }, rough: 0.05 + 0.9 * rng.f(), p }
        })
        .collect()
}

fn cpu_bsdf(c: &Case) -> Bsdf {
    let s = Surface { base: c.base, alpha: 1.0, metallic: c.metallic, roughness: c.rough, transmission: 0.0, ior: 1.5, emissive: Vec3::ZERO, ao: 1.0 };
    Bsdf::new(&s, c.n, true).principled(&c.p, c.rough)
}

#[test]
fn principled_lobes_match_the_cpu_bsdf() {
    let g = match gpu() {
        Ok(g) => g,
        Err(e) if std::env::var_os("MINIMAX_REQUIRE_GPU").is_none() => return eprintln!("skipping lobe parity: {e}"),
        Err(e) => panic!("{e}"),
    };
    let cs = cases();
    let src = format!("{}\n{}", super::source(), ENTRY.replace("SAMPLES", &format!("{SAMPLES}u")).replace("RECu", &format!("{}u", textures::REC_WORDS)));
    let d = &g.device;
    let module = d.create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("lobes"), source: wgpu::ShaderSource::Wgsl(src.into()) });
    let consts = super::features::Features(super::features::Features::PRINCIPLED | super::features::Features::SSS).constants();
    let opts = wgpu::PipelineCompilationOptions { constants: &consts, ..Default::default() };
    let pipe = d.create_compute_pipeline(&wgpu::ComputePipelineDescriptor { label: Some("lobes"), layout: None, module: &module, entry_point: Some("lobes"), compilation_options: opts, cache: None });
    let mut recs = Vec::new();
    let mut input = Vec::new();
    for c in &cs {
        let m = crate::materials::Material { principled: c.p.clone(), ..Default::default() };
        recs.extend(textures::record(&m).map(f32::to_bits));
        for v in [c.n.extend(0.0), c.v.extend(0.0), c.l.extend(0.0), c.base.extend(c.metallic), Vec4::new(c.rough, 0.0, 0.0, 0.0)] {
            input.extend(v.to_array().iter().flat_map(|f| f.to_le_bytes()));
        }
    }
    let size = (cs.len() * 32) as u64;
    let aux = storage(d, "records", &recs.iter().flat_map(|w| w.to_le_bytes()).collect::<Vec<_>>(), false);
    let inp = storage(d, "lobe in", &input, false);
    let out = storage(d, "lobe out", &vec![0; size as usize], true);
    let layout = pipe.get_bind_group_layout(0);
    let entries = [(9, &aux), (10, &inp), (11, &out)].map(|(b, buf)| wgpu::BindGroupEntry { binding: b, resource: buf.as_entire_binding() });
    let bind = d.create_bind_group(&wgpu::BindGroupDescriptor { label: Some("lobes"), layout: &layout, entries: &entries });
    let mut enc = d.create_command_encoder(&Default::default());
    {
        let mut cp = enc.begin_compute_pass(&Default::default());
        cp.set_pipeline(&pipe);
        cp.set_bind_group(0, &bind, &[]);
        cp.dispatch_workgroups((cs.len() as u32).div_ceil(16), 1, 1);
    }
    g.queue.submit([enc.finish()]);
    let res = read_back(g, &out, size).expect("lobe results");
    let mut rng = Rng::new(5);
    for (i, c) in cs.iter().enumerate() {
        let b = cpu_bsdf(c);
        let (f, pdf) = b.eval(c.v, c.l);
        let (gf, gpdf) = (res[2 * i].truncate(), res[2 * i].w);
        let tol = |x: f32| 2e-3 * x.abs().max(1.0);
        assert!((gf - f).abs().max_element() <= tol(f.max_element()), "case {i}: f GPU {gf} vs CPU {f}");
        assert!((gpdf - pdf).abs() <= tol(pdf), "case {i}: pdf GPU {gpdf} vs CPU {pdf}");
        let k = 1 << 17;
        let (mut sum, mut sss) = (Vec3::ZERO, 0);
        for _ in 0..k {
            if let Some(s) = b.sample(c.v, &mut rng) {
                sum += s.weight;
                sss += usize::from(s.sss);
            }
        }
        let (albedo, gpu_albedo) = (sum / k as f32, res[2 * i + 1].truncate());
        assert!((gpu_albedo - albedo).abs().max_element() < 0.02 + 0.03 * albedo.max_element(), "case {i}: albedo GPU {gpu_albedo} vs CPU {albedo}");
        assert!((res[2 * i + 1].w - sss as f32 / k as f32).abs() < 0.01, "case {i}: SSS entry share");
    }
}
