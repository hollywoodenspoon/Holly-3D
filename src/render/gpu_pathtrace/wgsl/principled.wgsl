// Principled lobes (FEAT_PRINCIPLED / FEAT_SSS; matches src/render/bsdf/
// mod.rs and microfacet.rs): anisotropic GGX in a tangent frame, clearcoat
// lacquer, cloth sheen, the SSS entry lobe and the white Lambert exit, plus
// the per-material record (textures.rs `record`) and the random walk
// (src/render/pathtrace/subsurface.rs).

const NO_REC: u32 = 0xffffffffu;

fn schlick_w(c: f32) -> f32 {
    return pow(1.0 - clamp(c, 0.0, 1.0), 5.0);
}

fn max3(v: vec3<f32>) -> f32 {
    return max(v.x, max(v.y, v.z));
}

// Word index of the triangle's material record (NO_REC = none).
fn rec_of(tri: u32) -> u32 {
    if (u.tex.w == 0u) { return NO_REC; }
    let r = aux[tri * 16u + 7u];
    if (r == NO_REC) { return NO_REC; }
    return u.tex.z + r * 28u; // textures::REC_WORDS
}

// Cauchy IOR at a wavelength (µm) from the base IOR and an Abbe number.
fn ior_at(ior: f32, abbe: f32, l: f32) -> f32 {
    let f = 0.4861;
    let c = 0.6563;
    let d = 0.5893;
    let b = (ior - 1.0) / (abbe * (1.0 / (f * f) - 1.0 / (c * c)));
    return max(ior - b / (d * d) + b / (l * l), 1.0);
}

// Frame::new: tangent from world Z wrapped round the surface, turned by `rot` degrees.
fn frame_t(n: vec3<f32>, rot: f32) -> vec3<f32> {
    let up = select(vec3<f32>(0.0, 0.0, 1.0), vec3<f32>(1.0, 0.0, 0.0), abs(n.z) > 0.999);
    let t0 = normalize(cross(up, n));
    let b0 = cross(n, t0);
    let a = radians(rot);
    return t0 * cos(a) + b0 * sin(a);
}

// Bsdf::principled: the material record's lobes on a plain surface.
fn principled(s0: Surf, rec: u32, roughness: f32) -> Surf {
    var s = s0;
    var rot = 0.0;
    if (rec != NO_REC) { rot = auxf(rec + 1u); }
    s.t = frame_t(s.n, select(0.0, rot, rec != NO_REC && auxf(rec) > 0.0));
    s.b = cross(s.n, s.t);
    if (rec == NO_REC) { return s; }
    let aniso = auxf(rec);
    if (aniso > 0.0) {
        let r = clamp(roughness, 0.0, 1.0);
        let a = max(r * r, 2e-3);
        let aspect = sqrt(1.0 - 0.9 * aniso);
        s.ax = min(a / aspect, 1.0);
        s.ay = max(a * aspect, 1e-3);
    }
    s.clearcoat = auxf(rec + 2u);
    let cr = auxf(rec + 3u);
    s.cc_alpha = max(cr * cr, 2e-3);
    let tint = mix(vec3<f32>(1.0), s.base / max(max3(s.base), 1e-4), auxf(rec + 5u));
    s.sheen = tint * auxf(rec + 4u);
    s.sss = auxf(rec + 6u);
    let abbe = auxf(rec + 7u);
    if (abbe > 0.0) {
        s.ior_rgb = vec3<f32>(ior_at(s.ior, abbe, 0.61), ior_at(s.ior, abbe, 0.55), ior_at(s.ior, abbe, 0.465));
    }
    return s;
}

fn d_aniso(h: vec3<f32>, ax: f32, ay: f32) -> f32 {
    if (h.z <= 0.0) { return 0.0; }
    let k = (h.x / ax) * (h.x / ax) + (h.y / ay) * (h.y / ay) + h.z * h.z;
    return 1.0 / max(PI * ax * ay * k * k, 1e-12);
}

fn g1_aniso(v: vec3<f32>, ax: f32, ay: f32) -> f32 {
    if (v.z <= 0.0) { return 0.0; }
    let z2 = max(v.z * v.z, 1e-12);
    let t = ((ax * v.x) * (ax * v.x) + (ay * v.y) * (ay * v.y)) / z2;
    return 1.0 / (1.0 + 0.5 * (sqrt(1.0 + t) - 1.0));
}

fn mf_pdf(v: vec3<f32>, h: vec3<f32>, ax: f32, ay: f32) -> f32 {
    return d_aniso(h, ax, ay) * g1_aniso(v, ax, ay) / max(4.0 * v.z, 1e-12);
}

fn mf_spec(v: vec3<f32>, l: vec3<f32>, ax: f32, ay: f32) -> f32 {
    let h = normalize(v + l);
    return d_aniso(h, ax, ay) * g1_aniso(v, ax, ay) * g1_aniso(l, ax, ay) / max(4.0 * v.z, 1e-12);
}

// Heitz 2018 visible-normal sampling in the local frame.
fn vndf_local(v: vec3<f32>, ax: f32, ay: f32) -> vec3<f32> {
    let vh = normalize(vec3<f32>(ax * v.x, ay * v.y, v.z));
    let len2 = vh.x * vh.x + vh.y * vh.y;
    let t1 = select(vec3<f32>(1.0, 0.0, 0.0), vec3<f32>(-vh.y, vh.x, 0.0) / sqrt(len2), len2 > 0.0);
    let t2 = cross(vh, t1);
    let r = sqrt(rnd());
    let phi = 2.0 * PI * rnd();
    let p1 = r * cos(phi);
    let sv = 0.5 * (1.0 + vh.z);
    let p2 = (1.0 - sv) * sqrt(max(0.0, 1.0 - p1 * p1)) + sv * r * sin(phi);
    let nh = t1 * p1 + t2 * p2 + vh * sqrt(max(0.0, 1.0 - p1 * p1 - p2 * p2));
    return normalize(vec3<f32>(ax * nh.x, ay * nh.y, max(nh.z, 0.0)));
}

fn to_local(s: Surf, v: vec3<f32>) -> vec3<f32> {
    return vec3<f32>(dot(v, s.t), dot(v, s.b), dot(v, s.n));
}

fn to_world(s: Surf, v: vec3<f32>) -> vec3<f32> {
    return s.t * v.x + s.b * v.y + s.n * v.z;
}

fn pr_eval(s: Surf, v: vec3<f32>, l: vec3<f32>) -> Eval {
    let vl = to_local(s, v);
    let ll = to_local(s, l);
    let opaque = 1.0 - s.transmission;
    if (vl.z <= 1e-5 || ll.z <= 0.0 || opaque <= 0.0) { return Eval(vec3<f32>(0.0), 0.0); }
    if (s.white) { return Eval(vec3<f32>(ll.z / PI), ll.z / PI); }
    let hl = normalize(vl + ll);
    let vh = max(dot(vl, hl), 0.0);
    let kd = (1.0 - s.metallic) * ll.z;
    let under = 1.0 - 0.3 * max3(s.sheen) * schlick_w(vl.z);
    let diffuse = s.base * (kd * (1.0 - s.sss) * under / PI) + s.sheen * (kd * schlick_w(dot(ll, hl)));
    let f0 = mix(vec3<f32>(0.04), s.base, s.metallic);
    let spec = schlick(vh, f0) * mf_spec(vl, ll, s.ax, s.ay);
    let pd = 0.5 * (1.0 - s.metallic);
    let pc = s.clearcoat * 0.25;
    var pdf = pd * (1.0 - s.sss) * ll.z / PI + (1.0 - pd) * mf_pdf(vl, hl, s.ax, s.ay);
    var f = diffuse + spec;
    if (s.clearcoat > 0.0) {
        let fc = 0.04 + 0.96 * schlick_w(vh);
        let coat = s.clearcoat * fc * mf_spec(vl, ll, s.cc_alpha, s.cc_alpha);
        f = f * (1.0 - s.clearcoat * (0.04 + 0.96 * schlick_w(vl.z))) + vec3<f32>(coat);
        pdf = (1.0 - pc) * pdf + pc * mf_pdf(vl, hl, s.cc_alpha, s.cc_alpha);
    }
    return Eval(f * opaque, pdf * opaque);
}

// Bsdf::sample's opaque part (the glass lobe is sample_glass).
fn pr_sample(s: Surf, v: vec3<f32>) -> Bs {
    var none = bs(vec3<f32>(0.0), vec3<f32>(0.0), 0.0, 0u, false);
    none.ok = false;
    let vl = to_local(s, v);
    if (vl.z <= 1e-5) { return none; }
    if (s.white) {
        let dir = cosine_dir(s.n);
        return bs(dir, vec3<f32>(1.0), max(dot(s.n, dir), 1e-6) / PI, 0u, false);
    }
    let pd = 0.5 * (1.0 - s.metallic);
    let pc = s.clearcoat * 0.25;
    var dir: vec3<f32>;
    var lobe = 1u;
    if (pc > 0.0 && rnd() < pc) {
        let h = vndf_local(vl, s.cc_alpha, s.cc_alpha);
        dir = to_world(s, h * 2.0 * dot(vl, h) - vl);
    } else if (rnd() < pd) {
        if (s.sss > 0.0 && rnd() < s.sss) {
            // The diffuse energy the (1 - sss) reflection gave up enters the walk.
            let coat = 1.0 - s.clearcoat * (0.04 + 0.96 * schlick_w(vl.z));
            let w = (1.0 - s.metallic) * coat / max(pd * (1.0 - pc), 1e-6);
            var o = bs(-s.n, s.base * w, 0.0, 0u, true);
            o.sss = true;
            return o;
        }
        dir = cosine_dir(s.n);
        lobe = 0u;
    } else {
        let h = vndf_local(vl, s.ax, s.ay);
        dir = to_world(s, h * 2.0 * dot(vl, h) - vl);
    }
    let e = pr_eval(s, v, dir);
    if (e.pdf <= 1e-8 || max3(e.f) <= 0.0) { return none; }
    return bs(dir, e.f / e.pdf, e.pdf, lobe, false);
}

fn sphere_dir() -> vec3<f32> {
    let z = 1.0 - 2.0 * rnd();
    let r = sqrt(max(1.0 - z * z, 0.0));
    let phi = 2.0 * PI * rnd();
    return vec3<f32>(r * cos(phi), r * sin(phi), z);
}

// Where a random walk leaves (normals facing out) and its throughput.
struct Walk { ok: bool, p: vec3<f32>, ng: vec3<f32>, n: vec3<f32>, tri: u32, w: vec3<f32> }

// Exponential free flights with isotropic scattering and Russian roulette
// until the path crosses a surface (PathTracer::random_walk).
fn random_walk(rec: u32, p: vec3<f32>, ng: vec3<f32>, n: vec3<f32>) -> Walk {
    let z = vec3<f32>(0.0);
    let fail = Walk(false, z, z, z, 0u, z);
    if (!FEAT_SSS || rec == NO_REC) { return fail; }
    let sigma = auxf(rec + 12u);
    let albedo = vec3<f32>(auxf(rec + 13u), auxf(rec + 14u), auxf(rec + 15u));
    var o = p - ng * u.background.w;
    var d = cosine_dir(-n);
    var w = vec3<f32>(1.0);
    for (var i = 0; i < 256; i++) {
        let t = -log(max(1.0 - rnd(), 1e-12)) / sigma;
        let h = closest(o, d, t);
        if (h.tri != 0xffffffffu) {
            let tr = tris[h.tri];
            let bw = vec3<f32>(1.0 - h.u - h.v, h.u, h.v);
            let hp = tr.p0.xyz * bw.x + tr.p1.xyz * bw.y + tr.p2.xyz * bw.z;
            let geo = normalize(cross(tr.p1.xyz - tr.p0.xyz, tr.p2.xyz - tr.p0.xyz));
            let gin = select(-geo, geo, dot(geo, d) < 0.0);
            var hn = normalize(tr.n0.xyz * bw.x + tr.n1.xyz * bw.y + tr.n2.xyz * bw.z);
            if (dot(hn, gin) < 0.0) { hn = -hn; }
            if (FEAT_TEXTURES) {
                let tx = tex_shade(h.tri, bw, hp, gin, hn);
                if (tx.ok) { hn = tx.n; }
            }
            return Walk(true, hp, -gin, -hn, h.tri, w);
        }
        o += d * t;
        w *= albedo;
        let survive = clamp(max3(w), 0.0, 0.98);
        if (rnd() >= survive) { return fail; }
        w /= survive;
        d = sphere_dir();
    }
    return fail;
}
