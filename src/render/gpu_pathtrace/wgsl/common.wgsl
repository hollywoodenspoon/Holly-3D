// Ilex GPU path tracer, shared declarations: parameters, scene records,
// bindings, feature switches and sampling helpers. The host joins every
// file in this folder into one module (`super::source`); WGSL allows any
// declaration order, so the files only group the code.
//
// Design ported from the SolidCraft wgpu path tracer (MIT / Apache-2.0).
// One invocation per pixel traces `spp` paths: stack BVH traversal, Lambert
// + GGX (VNDF) + smooth dielectric BSDF, NEE on punctual lights and emissive
// triangles combined with BSDF sampling by the power heuristic, per-lobe
// bounce limits and Russian Roulette. Sums go to `accum` (rgb + coverage)
// and `stats` (luminance, luminance², n) for adaptive sampling on the host.

struct Params {
    inv_vp: mat4x4<f32>,
    dims: vec4<u32>,     // image w, h, region w, h
    origin: vec4<u32>,   // region x0, y0, spp, seed
    limits: vec4<u32>,   // max, diffuse, glossy, transmission bounces
    counts: vec4<u32>,   // lights, emitters, rr_start, flags
    background: vec4<f32>, // rgb, eps
    sky: vec4<f32>,
    ambient: vec4<f32>,
    fog: vec4<f32>,      // rgb, density (0 = none)
    env: vec4<f32>,      // environment image w, h, turn (0..1), strength
    tex: vec4<u32>,      // texture table word index, texture count, material records word index, count
    vol0: vec4<f32>,     // godrays / mist (layout in volume.wgsl)
    vol1: vec4<f32>,
    vol2: vec4<f32>,
    vol3: vec4<f32>,
    vol4: vec4<f32>,
}

struct Tri {
    p0: vec4<f32>, p1: vec4<f32>, p2: vec4<f32>,
    n0: vec4<f32>, n1: vec4<f32>, n2: vec4<f32>,
    c0: vec4<f32>, c1: vec4<f32>, c2: vec4<f32>,
    mat: vec4<f32>,   // metallic, roughness, transmission, ior
    emit: vec4<f32>,  // emitted radiance, model (0 standard, 1 unlit, 2 crystal)
    att: vec4<f32>,   // crystal attenuation rgb, thickness
    light: vec4<f32>, // emitter pick probability, area
}

struct Node { lo: vec4<f32>, hi: vec4<f32> }

struct Light {
    pos: vec4<f32>,   // position, kind (0 point, 1 spot, 2 directional, 3 area)
    dir: vec4<f32>,   // axis / travel direction (area: emission axis), cos inner
    rad: vec4<f32>,   // radiance, cos outer
    atten: vec4<f32>, // near a, b, far a, b (negative = off)
    misc: vec4<f32>,  // range, shadows, softness
    tint: vec4<f32>,  // shadow tint
    au: vec4<f32>,    // area: local X axis, half width (Rect) or radius
    av: vec4<f32>,    // area: local Y axis, half length (Rect, Tube) or radius
    area: vec4<f32>,  // area shape (0 rect, 1 disc, 2 sphere, 3 tube), visible to camera, IES profile, samples
}

@group(0) @binding(0) var<uniform> u: Params;
@group(0) @binding(1) var<storage, read> tris: array<Tri>;
@group(0) @binding(2) var<storage, read> nodes: array<Node>;
@group(0) @binding(3) var<storage, read> lights: array<Light>;
@group(0) @binding(4) var<storage, read> emitters: array<vec4<f32>>; // tri, prob, area, cdf
@group(0) @binding(5) var<storage, read_write> accum: array<vec4<f32>>;
@group(0) @binding(6) var<storage, read_write> stats: array<vec4<f32>>;
@group(0) @binding(7) var<storage, read> done: array<u32>;
@group(0) @binding(8) var<storage, read> envmap: array<vec4<f32>>; // equirect rgb rows

const PI: f32 = 3.14159265;
const F_ORTHO: u32 = 1u;
const F_SHADOWS: u32 = 2u;
const F_SOFTBOX: u32 = 4u;
const F_NEE: u32 = 8u;
// The sky is the procedural one (LightMix: Sun & Sky, else Environment).
const F_PROC_SKY: u32 = 16u;

// Feature switches, set per scene by the host as pipeline-overridable
// constants (`super::features`), so a scene only compiles what it uses and a
// plain scene keeps its register count low. A switched-off branch folds away
// at pipeline creation. Each parity PR fills in the hook its switch guards.
override FEAT_TEXTURES: bool = false;
override FEAT_PRINCIPLED: bool = false;
override FEAT_SSS: bool = false;
override FEAT_VOLUME: bool = false;
override FEAT_ENV_MAP: bool = false;
override FEAT_AREA: bool = false;
override FEAT_OPACITY: bool = false;
override FEAT_LIGHTMIX: bool = false;

var<private> rng_state: u32;

// LightMix (`FEAT_LIGHTMIX`): one path's light split by group, in the CPU's
// order (0 sun & sky, 1 environment, 2 emissive, 3 volume, 4.. scene lights),
// summed per pixel into `groups` (`u.vol4.y` groups, 3 floats each).
@group(0) @binding(12) var<storage, read_write> groups: array<f32>;
const G_SUN_SKY: u32 = 0u;
const G_ENV: u32 = 1u;
const G_EMISSIVE: u32 = 2u;
const G_VOLUME: u32 = 3u;
var<private> mix_l: array<vec3<f32>, 12>;
// Path throughput the light functions weight their per-light share by.
var<private> mix_beta: vec3<f32>;

fn mix_add(g: u32, c: vec3<f32>) {
    if (FEAT_LIGHTMIX) { mix_l[min(g, 11u)] += c; }
}

// Group of light `l` (packed in `misc.w`).
fn light_group(l: Light) -> u32 {
    return u32(l.misc.w + 0.5);
}

fn pcg(v: u32) -> u32 {
    let s = v * 747796405u + 2891336453u;
    let w = ((s >> ((s >> 28u) + 4u)) ^ s) * 277803737u;
    return (w >> 22u) ^ w;
}

fn rnd() -> f32 {
    rng_state = pcg(rng_state);
    return f32(rng_state >> 8u) / 16777216.0;
}

fn ball() -> vec3<f32> {
    for (var i = 0; i < 16; i++) {
        let p = vec3<f32>(rnd(), rnd(), rnd()) * 2.0 - 1.0;
        if (dot(p, p) <= 1.0) { return p; }
    }
    return vec3<f32>(0.0);
}

fn onb(n: vec3<f32>) -> mat3x3<f32> {
    let s = select(-1.0, 1.0, n.z >= 0.0);
    let a = -1.0 / (s + n.z);
    let b = n.x * n.y * a;
    return mat3x3<f32>(vec3<f32>(1.0 + s * n.x * n.x * a, s * b, -s * n.x), vec3<f32>(b, s + n.y * n.y * a, -n.y), n);
}

fn cosine_dir(n: vec3<f32>) -> vec3<f32> {
    let r = sqrt(rnd());
    let phi = rnd() * 2.0 * PI;
    return normalize(onb(n) * vec3<f32>(r * cos(phi), r * sin(phi), sqrt(max(0.0, 1.0 - r * r))));
}
