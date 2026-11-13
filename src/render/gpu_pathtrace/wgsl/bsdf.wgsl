// BSDF: Lambert + GGX + smooth dielectric (matches src/render/pathtrace/bsdf.rs).

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

// Hook for texture lookups (PR 1) and Principled lobes (PR 2): returns the
// surface unchanged until those land.
fn shade_hook(s: Surf, tri: u32, w: vec3<f32>) -> Surf {
    var o = s;
    if (FEAT_TEXTURES) {
        // PR 1: base colour, metallic / roughness and normal from the texture store.
    }
    if (FEAT_PRINCIPLED || FEAT_SSS) {
        // PR 2: clearcoat, sheen, anisotropy, absorption, dispersion, SSS.
    }
    return o;
}
