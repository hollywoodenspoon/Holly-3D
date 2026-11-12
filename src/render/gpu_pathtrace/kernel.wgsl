// MiniMax GPU path tracer (Phase 11.5A): a compact port of the SolidCraft
// wgpu path tracer's design (MIT / Apache-2.0). One invocation per pixel
// traces `spp` paths: stack BVH traversal, Lambert + GGX (VNDF) + smooth
// dielectric BSDF, NEE on punctual lights and emissive triangles combined
// with BSDF sampling by the power heuristic, per-lobe bounce limits and
// Russian Roulette. Sums go to `accum` (rgb + coverage) and `stats`
// (luminance, luminance², n) for adaptive sampling on the host.

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
    pos: vec4<f32>,   // position, kind (0 point, 1 spot, 2 directional)
    dir: vec4<f32>,   // axis / travel direction, cos inner
    rad: vec4<f32>,   // radiance, cos outer
    atten: vec4<f32>, // near a, b, far a, b (negative = off)
    misc: vec4<f32>,  // range, shadows, softness
    tint: vec4<f32>,  // shadow tint
}

@group(0) @binding(0) var<uniform> u: Params;
@group(0) @binding(1) var<storage, read> tris: array<Tri>;
@group(0) @binding(2) var<storage, read> nodes: array<Node>;
@group(0) @binding(3) var<storage, read> lights: array<Light>;
@group(0) @binding(4) var<storage, read> emitters: array<vec4<f32>>; // tri, prob, area, cdf
@group(0) @binding(5) var<storage, read_write> accum: array<vec4<f32>>;
@group(0) @binding(6) var<storage, read_write> stats: array<vec4<f32>>;
@group(0) @binding(7) var<storage, read> done: array<u32>;

const PI: f32 = 3.14159265;
const F_ORTHO: u32 = 1u;
const F_SHADOWS: u32 = 2u;
const F_SOFTBOX: u32 = 4u;
const F_NEE: u32 = 8u;

var<private> rng_state: u32;

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

struct Hit { t: f32, u: f32, v: f32, tri: u32 }

fn intersect(i: u32, o: vec3<f32>, d: vec3<f32>, best: ptr<function, Hit>) {
    let t = tris[i];
    let e1 = t.p1.xyz - t.p0.xyz;
    let e2 = t.p2.xyz - t.p0.xyz;
    let pv = cross(d, e2);
    let det = dot(e1, pv);
    if (abs(det) < 1e-12) { return; }
    let inv = 1.0 / det;
    let s = o - t.p0.xyz;
    let uu = dot(s, pv) * inv;
    if (uu < 0.0 || uu > 1.0) { return; }
    let q = cross(s, e1);
    let vv = dot(d, q) * inv;
    if (vv < 0.0 || uu + vv > 1.0) { return; }
    let tt = dot(e2, q) * inv;
    if (tt > 0.0 && tt < (*best).t) { *best = Hit(tt, uu, vv, i); }
}

fn slab(n: Node, o: vec3<f32>, inv: vec3<f32>, tmax: f32) -> bool {
    let a = (n.lo.xyz - o) * inv;
    let b = (n.hi.xyz - o) * inv;
    let t0 = max(max(min(a.x, b.x), min(a.y, b.y)), max(min(a.z, b.z), 0.0));
    let t1 = min(min(max(a.x, b.x), max(a.y, b.y)), min(max(a.z, b.z), tmax));
    return t0 <= t1;
}

fn closest(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> Hit {
    var best = Hit(tmax, 0.0, 0.0, 0xffffffffu);
    if (arrayLength(&nodes) == 0u || arrayLength(&tris) == 0u) { return best; }
    let inv = 1.0 / select(d, vec3<f32>(1e-20), abs(d) < vec3<f32>(1e-20));
    var stack: array<u32, 64>;
    var sp = 1;
    stack[0] = 0u;
    while (sp > 0) {
        sp -= 1;
        let ni = stack[sp];
        let n = nodes[ni];
        if (!slab(n, o, inv, best.t)) { continue; }
        let start = bitcast<u32>(n.lo.w);
        let count = bitcast<u32>(n.hi.w);
        if (count > 0u) {
            for (var k = 0u; k < count; k++) { intersect(start + k, o, d, &best); }
        } else if (sp < 62) {
            // The left child follows its parent; `start` is the right child.
            stack[sp] = start;
            stack[sp + 1] = ni + 1u;
            sp += 2;
        }
    }
    return best;
}

fn occluded(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> bool {
    return closest(o, d, tmax).tri != 0xffffffffu;
}

// ---- BSDF (matches src/render/pathtrace/bsdf.rs) ----

fn ggx_d(nh: f32, a2: f32) -> f32 {
    let d = nh * nh * (a2 - 1.0) + 1.0;
    return a2 / max(PI * d * d, 1e-12);
}

fn smith_g1(nx: f32, a2: f32) -> f32 {
    return 2.0 * nx / max(nx + sqrt(a2 + (1.0 - a2) * nx * nx), 1e-12);
}

fn ggx_pdf(nh: f32, nv: f32, a2: f32) -> f32 {
    return ggx_d(nh, a2) / max(2.0 * (nv + sqrt(a2 + (1.0 - a2) * nv * nv)), 1e-12);
}

struct Surf { n: vec3<f32>, base: vec3<f32>, metallic: f32, alpha: f32, transmission: f32, ior: f32, entering: bool }

struct Eval { f: vec3<f32>, pdf: f32 }

fn schlick(c: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(1.0 - clamp(c, 0.0, 1.0), 5.0);
}

fn bsdf_eval(s: Surf, v: vec3<f32>, l: vec3<f32>) -> Eval {
    let nv = dot(s.n, v);
    let nl = dot(s.n, l);
    let opaque = 1.0 - s.transmission;
    if (nv <= 1e-5 || nl <= 0.0 || opaque <= 0.0) { return Eval(vec3<f32>(0.0), 0.0); }
    let h = normalize(v + l);
    let nh = max(dot(s.n, h), 0.0);
    let vh = max(dot(v, h), 0.0);
    let a2 = s.alpha * s.alpha;
    let diffuse = s.base * (1.0 - s.metallic) / PI * nl;
    let f0 = mix(vec3<f32>(0.04), s.base, s.metallic);
    let spec = schlick(vh, f0) * (ggx_d(nh, a2) * smith_g1(nv, a2) * smith_g1(nl, a2) / (4.0 * nv));
    let pd = 0.5 * (1.0 - s.metallic);
    let pdf = pd * nl / PI + (1.0 - pd) * ggx_pdf(nh, nv, a2);
    return Eval((diffuse + spec) * opaque, pdf * opaque);
}

fn sample_vndf(n: vec3<f32>, v: vec3<f32>, alpha: f32) -> vec3<f32> {
    let m = onb(n);
    let vl = transpose(m) * v;
    let vh = normalize(vec3<f32>(alpha * vl.x, alpha * vl.y, vl.z));
    let len2 = vh.x * vh.x + vh.y * vh.y;
    let t1 = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(-vh.y, vh.x, 0.0) / sqrt(len2), len2 > 0.0);
    let t2 = cross(vh, t1);
    let r = sqrt(rnd());
    let phi = 2.0 * PI * rnd();
    let p1 = r * cos(phi);
    let sv = 0.5 * (1.0 + vh.z);
    let p2 = (1.0 - sv) * sqrt(max(0.0, 1.0 - p1 * p1)) + sv * r * sin(phi);
    let nh = t1 * p1 + t2 * p2 + vh * sqrt(max(0.0, 1.0 - p1 * p1 - p2 * p2));
    return normalize(m * normalize(vec3<f32>(alpha * nh.x, alpha * nh.y, max(nh.z, 0.0))));
}

struct Bs { dir: vec3<f32>, weight: vec3<f32>, pdf: f32, lobe: u32, delta: bool, ok: bool }

fn bsdf_sample(s: Surf, v: vec3<f32>) -> Bs {
    let none = Bs(vec3<f32>(0.0), vec3<f32>(0.0), 0.0, 0u, false, false);
    if (s.transmission > 0.0 && rnd() < s.transmission) {
        let d = -v;
        let eta = select(s.ior, 1.0 / s.ior, s.entering);
        let c = clamp(dot(s.n, v), 0.0, 1.0);
        let k = 1.0 - eta * eta * (1.0 - c * c);
        var fr = 1.0;
        if (k >= 0.0) {
            let f0 = pow((1.0 - eta) / (1.0 + eta), 2.0);
            fr = f0 + (1.0 - f0) * pow(1.0 - c, 5.0);
        }
        if (rnd() < fr) {
            let r = d - s.n * 2.0 * dot(d, s.n);
            return Bs(normalize(r + ball() * s.alpha * 0.5), vec3<f32>(1.0), 0.0, 1u, true, true);
        }
        let t = normalize(d * eta + s.n * (eta * c - sqrt(k)));
        let tint = select(vec3<f32>(1.0), s.base, s.entering);
        return Bs(normalize(t + ball() * s.alpha * 0.5), tint, 0.0, 2u, true, true);
    }
    if (dot(s.n, v) <= 1e-5) { return none; }
    var dir: vec3<f32>;
    var lobe = 0u;
    if (rnd() < 0.5 * (1.0 - s.metallic)) {
        dir = cosine_dir(s.n);
    } else {
        let h = sample_vndf(s.n, v, s.alpha);
        dir = h * 2.0 * dot(v, h) - v;
        lobe = 1u;
    }
    let e = bsdf_eval(s, v, dir);
    if (e.pdf <= 1e-8 || max(e.f.x, max(e.f.y, e.f.z)) <= 0.0) { return none; }
    return Bs(dir, e.f / e.pdf, e.pdf, lobe, false, true);
}

fn power(a: f32, b: f32) -> f32 {
    if (a <= 0.0) { return 0.0; }
    let r = b / a;
    return 1.0 / (1.0 + r * r);
}

// ---- environment (matches Tracer::environment without an HDRI) ----

fn panel(d: vec3<f32>, axis: vec3<f32>, t: vec3<f32>, b: vec3<f32>, hx: f32, hy: f32, p: f32) -> f32 {
    let k = dot(d, axis);
    if (k <= 0.05) { return 0.0; }
    let x = abs(dot(d, t) / k) / hx;
    let y = abs(dot(d, b) / k) / hy;
    return p * clamp((1.0 - max(x, y)) / 0.12, 0.0, 1.0);
}

fn strip(d: vec3<f32>, side: f32) -> f32 {
    let axis = normalize(vec3<f32>(side, 0.3, 0.25));
    let t = normalize(cross(vec3<f32>(0.0, 0.0, 1.0), axis));
    return panel(d, axis, t, normalize(cross(axis, t)), 0.18, 0.75, 1.6);
}

fn environment(d: vec3<f32>) -> vec3<f32> {
    let up = clamp(0.5 + 0.5 * d.z, 0.0, 1.0);
    var c = u.background.xyz + u.sky.xyz + u.ambient.xyz * (1.0 + 5.0 * up * up);
    if ((u.counts.w & F_SOFTBOX) != 0u) {
        let key = normalize(vec3<f32>(0.25, -0.35, 1.0));
        let kt = vec3<f32>(1.0, 0.0, 0.0);
        let s = panel(d, key, kt, normalize(cross(key, kt)), 0.55, 0.4, 3.0) + strip(d, 1.0) + strip(d, -1.0);
        c += vec3<f32>(1.0, 0.98, 0.95) * s;
    }
    return c;
}

// ---- lights ----

fn ramp(d: f32, a: f32, b: f32) -> f32 {
    if (b - a < 1e-6) { return select(0.0, 1.0, d >= b); }
    return clamp((d - a) / (b - a), 0.0, 1.0);
}

fn punctual(p: vec3<f32>, ng: vec3<f32>, s: Surf, v: vec3<f32>) -> vec3<f32> {
    let o = p + ng * u.background.w;
    var c = vec3<f32>(0.0);
    for (var i = 0u; i < u.counts.x; i++) {
        let l = lights[i];
        var dir: vec3<f32>;
        var dist = 1e30;
        var k = 1.0;
        if (l.pos.w > 1.5) {
            dir = -l.dir.xyz;
        } else {
            let to = l.pos.xyz - p;
            let dd = max(length(to), 1e-4);
            dir = to / dd;
            dist = dd;
            let w = clamp(1.0 - pow(dd / max(l.misc.x, 1e-3), 4.0), 0.0, 1.0);
            var at = 1.0;
            if (l.atten.x >= 0.0) { at *= ramp(dd, l.atten.x, l.atten.y); }
            if (l.atten.z >= 0.0) { at *= 1.0 - ramp(dd, l.atten.z, l.atten.w); }
            k = w * w * at / (dd * dd);
            if (l.pos.w > 0.5) {
                let cc = dot(-dir, l.dir.xyz);
                let t = clamp((cc - l.rad.w) / max(l.dir.w - l.rad.w, 1e-4), 0.0, 1.0);
                k *= t * t * (3.0 - 2.0 * t);
            }
        }
        if (k <= 0.0 || dot(s.n, dir) <= 0.0) { continue; }
        var rad = l.rad.xyz * k;
        if ((u.counts.w & F_SHADOWS) != 0u && l.misc.y > 0.5) {
            var sd = dir;
            var len = dist;
            let soft = l.misc.z;
            if (soft > 0.0) {
                if (dist < 1e29) {
                    let aim = p + dir * dist + ball() * soft;
                    sd = normalize(aim - o);
                    len = length(aim - o);
                } else {
                    sd = normalize(dir + ball() * soft * 0.05);
                }
            }
            let vis = select(1.0, 0.0, occluded(o, sd, min(len - u.background.w, 1e30)));
            rad *= l.tint.xyz + (1.0 - l.tint.xyz) * vis;
        }
        c += bsdf_eval(s, v, dir).f * rad;
    }
    return c;
}

fn point_on(t: Tri) -> vec3<f32> {
    var a = rnd();
    var b = rnd();
    if (a + b > 1.0) { a = 1.0 - a; b = 1.0 - b; }
    return t.p0.xyz + (t.p1.xyz - t.p0.xyz) * a + (t.p2.xyz - t.p0.xyz) * b;
}

fn emitter_nee(p: vec3<f32>, ng: vec3<f32>, self_tri: u32, s: Surf, v: vec3<f32>) -> vec3<f32> {
    let n = u.counts.y;
    if (n == 0u || (u.counts.w & F_NEE) == 0u) { return vec3<f32>(0.0); }
    let x = rnd();
    var lo = 0u;
    var hi = n - 1u;
    while (lo < hi) {
        let mid = (lo + hi) / 2u;
        if (emitters[mid].w <= x) { lo = mid + 1u; } else { hi = mid; }
    }
    let e = emitters[lo];
    let ti = bitcast<u32>(e.x);
    if (ti == self_tri) { return vec3<f32>(0.0); }
    let t = tris[ti];
    let q = point_on(t);
    let o = p + ng * u.background.w;
    let to = q - o;
    let d2 = dot(to, to);
    let dir = normalize(to);
    let nl = normalize(cross(t.p1.xyz - t.p0.xyz, t.p2.xyz - t.p0.xyz));
    let cl = abs(dot(nl, dir));
    if (cl <= 1e-6) { return vec3<f32>(0.0); }
    let light_pdf = e.y / max(e.z, 1e-12) * d2 / cl;
    let ev = bsdf_eval(s, v, dir);
    if (max(ev.f.x, max(ev.f.y, ev.f.z)) <= 0.0) { return vec3<f32>(0.0); }
    if ((u.counts.w & F_SHADOWS) != 0u && occluded(o, dir, sqrt(d2) - u.background.w * 4.0)) { return vec3<f32>(0.0); }
    return ev.f * t.emit.xyz * power(light_pdf, ev.pdf) / light_pdf;
}

// ---- integrator (matches PathTracer::radiance) ----

fn camera_ray(px: vec2<f32>) -> array<vec3<f32>, 2> {
    let ndc = vec2<f32>(px.x / f32(u.dims.x) * 2.0 - 1.0, 1.0 - px.y / f32(u.dims.y) * 2.0);
    let a = u.inv_vp * vec4<f32>(ndc, 1.0, 1.0);
    let far_z = select(1e-4, 0.0, (u.counts.w & F_ORTHO) != 0u);
    let b = u.inv_vp * vec4<f32>(ndc, far_z, 1.0);
    let near = a.xyz / a.w;
    return array<vec3<f32>, 2>(near, normalize(b.xyz / b.w - near));
}

fn radiance(o_in: vec3<f32>, d_in: vec3<f32>) -> vec4<f32> {
    var o = o_in;
    var d = d_in;
    var l = vec3<f32>(0.0);
    var beta = vec3<f32>(1.0);
    var alpha = 0.0;
    var camera = true;
    var prev_pdf = 0.0;
    var prev_delta = true;
    var depth = vec4<u32>(0u);
    let eps = u.background.w;
    for (var guard = 0; guard < 64; guard++) {
        let h = closest(o, d, 1e30);
        if (h.tri == 0xffffffffu) {
            if (!camera) { l += beta * environment(d); }
            break;
        }
        let t = tris[h.tri];
        let w = vec3<f32>(1.0 - h.u - h.v, h.u, h.v);
        let p = t.p0.xyz * w.x + t.p1.xyz * w.y + t.p2.xyz * w.z;
        let geo = normalize(cross(t.p1.xyz - t.p0.xyz, t.p2.xyz - t.p0.xyz));
        let entering = dot(geo, d) < 0.0;
        let ng = select(-geo, geo, entering);
        var n = normalize(t.n0.xyz * w.x + t.n1.xyz * w.y + t.n2.xyz * w.z);
        if (dot(n, ng) < 0.0) { n = -n; }
        let base = t.c0.xyz * w.x + t.c1.xyz * w.y + t.c2.xyz * w.z;
        if (camera) { alpha = 1.0; }
        if (u.fog.w > 0.0) {
            let f = exp(-u.fog.w * h.t);
            l += beta * u.fog.xyz * (1.0 - f);
            beta *= f;
        }
        let model = u32(t.emit.w + 0.5);
        if (!entering && model == 2u) {
            beta *= pow(max(t.att.xyz, vec3<f32>(1e-3)), vec3<f32>(h.t / max(t.att.w, 1e-3)));
        }
        if (max(t.emit.x, max(t.emit.y, t.emit.z)) > 0.0) {
            var mis = 1.0;
            if (!prev_delta && u.counts.y > 0u && (u.counts.w & F_NEE) != 0u && t.light.x > 0.0) {
                let lp = t.light.x / max(t.light.y, 1e-12) * h.t * h.t / max(abs(dot(geo, d)), 1e-6);
                mis = power(prev_pdf, lp);
            }
            l += beta * t.emit.xyz * mis;
        }
        if (model == 1u) {
            l += beta * base;
            break;
        }
        camera = false;
        let s = Surf(n, base, t.mat.x, max(t.mat.y * t.mat.y, 2e-3), t.mat.z, max(t.mat.w, 1.0), entering);
        let v = -d;
        l += beta * (punctual(p, ng, s, v) + emitter_nee(p, ng, h.tri, s, v));
        let bs = bsdf_sample(s, v);
        if (!bs.ok) { break; }
        depth.x += 1u;
        depth[bs.lobe + 1u] += 1u;
        if (depth.x > u.limits.x || depth[bs.lobe + 1u] > max(u.limits[bs.lobe + 1u], 1u)) { break; }
        beta *= bs.weight;
        if (depth.x > u.counts.z) {
            let survive = clamp(max(beta.x, max(beta.y, beta.z)), 0.0, 0.95);
            if (rnd() >= survive) { break; }
            beta /= survive;
        }
        if (max(beta.x, max(beta.y, beta.z)) <= 0.0) { break; }
        prev_pdf = bs.pdf;
        prev_delta = bs.delta;
        o = p + ng * select(-eps, eps, dot(bs.dir, ng) >= 0.0);
        d = bs.dir;
    }
    return vec4<f32>(l, alpha);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= u.dims.z || id.y >= u.dims.w) { return; }
    let i = id.y * u.dims.z + id.x;
    if (done[i] != 0u) { return; }
    let x = u.origin.x + id.x;
    let y = u.origin.y + id.y;
    var a = accum[i];
    var st = stats[i];
    for (var k = 0u; k < u.origin.z; k++) {
        rng_state = pcg(pcg(x * 1973u + y * 9277u + 26699u) ^ pcg(u.origin.w * 7919u + k + u32(st.z) * 31u));
        let ray = camera_ray(vec2<f32>(f32(x) + rnd(), f32(y) + rnd()));
        var c = radiance(ray[0], ray[1]);
        if (any(c != c) || any(abs(c) > vec4<f32>(1e30))) { c = vec4<f32>(0.0, 0.0, 0.0, c.w); }
        a += c;
        let lum = dot(c.xyz, vec3<f32>(0.2126, 0.7152, 0.0722));
        st += vec4<f32>(lum, lum * lum, 1.0, 0.0);
    }
    accum[i] = a;
    stats[i] = st;
}
