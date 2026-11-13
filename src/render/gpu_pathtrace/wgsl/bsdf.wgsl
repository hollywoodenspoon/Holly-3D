// BSDF: Lambert + GGX + smooth dielectric (matches src/render/bsdf/mod.rs);
// the Principled lobes are in principled.wgsl (FEAT_PRINCIPLED / FEAT_SSS).

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

// Shading point. `t`, `b`, `ax`, `ay` onwards only matter with the
// Principled switches on: the anisotropic frame and alphas, clearcoat,
// sheen, SSS weight, per-channel IOR (dispersion, x < 0 = off), the
// path's dispersion channel (-1 = none yet) and the white random-walk exit.
struct Surf {
    n: vec3<f32>, base: vec3<f32>, metallic: f32, alpha: f32, transmission: f32, ior: f32, entering: bool,
    t: vec3<f32>, b: vec3<f32>, ax: f32, ay: f32, clearcoat: f32, cc_alpha: f32, sheen: vec3<f32>, sss: f32,
    ior_rgb: vec3<f32>, channel: i32, white: bool,
}

fn surf(n: vec3<f32>, base: vec3<f32>, metallic: f32, alpha: f32, transmission: f32, ior: f32, entering: bool) -> Surf {
    let z = vec3<f32>(0.0);
    return Surf(n, base, metallic, alpha, transmission, ior, entering, z, z, alpha, alpha, 0.0, 0.0, z, 0.0, vec3<f32>(-1.0), -1, false);
}

struct Eval { f: vec3<f32>, pdf: f32 }

fn schlick(c: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (1.0 - f0) * pow(1.0 - clamp(c, 0.0, 1.0), 5.0);
}

fn bsdf_eval(s: Surf, v: vec3<f32>, l: vec3<f32>) -> Eval {
    if (FEAT_PRINCIPLED || FEAT_SSS) { return pr_eval(s, v, l); }
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

// `sss`: the path enters for a random walk; `channel`: dispersion picked it (-1 = no).
struct Bs { dir: vec3<f32>, weight: vec3<f32>, pdf: f32, lobe: u32, delta: bool, ok: bool, sss: bool, channel: i32 }

fn bs(dir: vec3<f32>, weight: vec3<f32>, pdf: f32, lobe: u32, delta: bool) -> Bs {
    return Bs(dir, weight, pdf, lobe, delta, true, false, -1);
}

// Smooth / frosted dielectric; with dispersion the path follows one colour
// channel (picked here once, weighted x3) with that channel's IOR.
fn sample_glass(s: Surf, v: vec3<f32>) -> Bs {
    var ior = s.ior;
    var channel = -1;
    var spectral = vec3<f32>(1.0);
    if ((FEAT_PRINCIPLED || FEAT_SSS) && s.ior_rgb.x > 0.0) {
        var k = s.channel;
        if (k < 0) {
            k = min(i32(rnd() * 3.0), 2);
            channel = k;
            spectral = vec3<f32>(0.0);
            spectral[k] = 3.0;
        }
        ior = s.ior_rgb[k];
    }
    let d = -v;
    let eta = select(ior, 1.0 / ior, s.entering);
    let c = clamp(dot(s.n, v), 0.0, 1.0);
    let k2 = 1.0 - eta * eta * (1.0 - c * c);
    var fr = 1.0;
    if (k2 >= 0.0) {
        let f0 = pow((1.0 - eta) / (1.0 + eta), 2.0);
        fr = f0 + (1.0 - f0) * pow(1.0 - c, 5.0);
    }
    if (rnd() < fr) {
        let r = d - s.n * 2.0 * dot(d, s.n);
        return bs(normalize(r + ball() * s.alpha * 0.5), vec3<f32>(1.0), 0.0, 1u, true);
    }
    let t = normalize(d * eta + s.n * (eta * c - sqrt(k2)));
    let tint = select(vec3<f32>(1.0), s.base, s.entering);
    var o = bs(normalize(t + ball() * s.alpha * 0.5), tint * spectral, 0.0, 2u, true);
    o.channel = channel;
    return o;
}

fn bsdf_sample(s: Surf, v: vec3<f32>) -> Bs {
    var none = bs(vec3<f32>(0.0), vec3<f32>(0.0), 0.0, 0u, false);
    none.ok = false;
    if (s.transmission > 0.0 && rnd() < s.transmission) { return sample_glass(s, v); }
    if (FEAT_PRINCIPLED || FEAT_SSS) { return pr_sample(s, v); }
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
    return bs(dir, e.f / e.pdf, e.pdf, lobe, false);
}

fn power(a: f32, b: f32) -> f32 {
    if (a <= 0.0) { return 0.0; }
    let r = b / a;
    return 1.0 / (1.0 + r * r);
}
