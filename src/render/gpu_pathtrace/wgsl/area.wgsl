// Area lights (Rect, Disc, Sphere, Tube) and IES profiles (matches
// src/render/area_lights.rs). FEAT_AREA guards the area code; IES shapes
// spots too. Each shading point takes `area.w` samples per area light.

fn gauss(x: f32, s: f32) -> f32 {
    return exp(-x * x / (2.0 * s * s));
}

fn cut(th: f32, a: f32, b: f32) -> f32 {
    let t = clamp((b - th) / (b - a), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

fn pow_pos(c: f32, e: f32) -> f32 {
    return select(0.0, pow(c, e), c > 0.0);
}

// Relative intensity of IES profile `id` (IesProfile::ALL order, 0 = none)
// at `c`, the cosine from the emission axis.
fn ies(id: u32, c: f32) -> f32 {
    if (id == 0u) { return 1.0; }
    let cc = clamp(c, -1.0, 1.0);
    let th = degrees(acos(cc));
    var f = 1.0;
    switch (id) {
        case 1u: { f = (0.35 + 0.65 * gauss(th - 30.0, 11.0)) * cut(th, 52.0, 64.0); }
        case 2u: { f = (0.25 + 0.75 * gauss(th - 62.0, 11.0)) * cut(th, 74.0, 86.0); }
        case 3u: { f = min(gauss(th, 3.5) + 0.45 * gauss(th - 11.0, 2.0) + 0.25 * gauss(th - 21.0, 2.5), 1.0) * cut(th, 26.0, 34.0); }
        case 4u: { f = pow_pos(cc, 1.3) * cut(th, 62.0, 75.0); }
        case 5u: { f = pow_pos(cc, 80.0) * cut(th, 12.0, 16.0); }
        default: {}
    }
    return clamp(f, 0.0, 1.0);
}

// One point on the side of area light `l` facing `p`: (point, normal) and
// the estimator scale A Le / I0.
struct AreaPt { q: vec3<f32>, nq: vec3<f32>, scale: f32 }

fn area_point(l: Light, p: vec3<f32>) -> AreaPt {
    let a = rnd();
    let b = rnd();
    let c = l.pos.xyz;
    let n = l.dir.xyz;
    let shape = u32(l.area.x + 0.5);
    if (shape == 0u) {
        return AreaPt(c + l.au.xyz * l.au.w * (2.0 * a - 1.0) + l.av.xyz * l.av.w * (2.0 * b - 1.0), n, 1.0);
    }
    if (shape == 1u) {
        let r = l.au.w * sqrt(a);
        let phi = 2.0 * PI * b;
        return AreaPt(c + (l.au.xyz * cos(phi) + l.av.xyz * sin(phi)) * r, n, 1.0);
    }
    if (shape == 2u) {
        let z = 1.0 - 2.0 * a;
        let s = sqrt(max(1.0 - z * z, 0.0));
        var d = vec3<f32>(s * cos(2.0 * PI * b), s * sin(2.0 * PI * b), z);
        if (dot(d, p - c) < 0.0) { d = -d; }
        return AreaPt(c + d * l.au.w, d, 2.0);
    }
    let w = cross(n, l.au.xyz);
    let phi = 2.0 * PI * b;
    var d = n * cos(phi) + w * sin(phi);
    if (dot(d, p - c) < 0.0) { d = -d; }
    return AreaPt(c + l.au.xyz * l.av.w * (2.0 * a - 1.0) + d * l.au.w, d, PI * 0.5);
}

fn area_incident(l: Light, p: vec3<f32>) -> Inc {
    if (!FEAT_AREA) { return Inc(vec3<f32>(0.0, 0.0, 1.0), 1.0, 0.0); }
    let s = area_point(l, p);
    let v = s.q - p;
    let d = max(length(v), 1e-4);
    let dir = v / d;
    let cos_l = dot(s.nq, -dir);
    if (cos_l <= 0.0) { return Inc(dir, d, 0.0); }
    let k = s.scale * cos_l * window(l, distance(l.pos.xyz, p)) / max(d * d, 1e-4);
    return Inc(dir, d, k * ies(u32(l.area.z), dot(-dir, l.dir.xyz)));
}

// Area lights at a shading point, `area.w` shadowed samples each.
fn area_nee(p: vec3<f32>, ng: vec3<f32>, s: Surf, v: vec3<f32>) -> vec3<f32> {
    var c = vec3<f32>(0.0);
    if (!FEAT_AREA) { return c; }
    let o = p + ng * u.background.w;
    for (var i = 0u; i < u.counts.x; i++) {
        let l = lights[i];
        if (l.pos.w < 2.5) { continue; }
        let n = max(u32(l.area.w), 1u);
        for (var j = 0u; j < n; j++) {
            let inc = area_incident(l, p);
            if (inc.k <= 0.0 || dot(s.n, inc.dir) <= 0.0) { continue; }
            let li = bsdf_eval(s, v, inc.dir).f * l.rad.xyz * (inc.k / f32(n)) * shadow(l, p, o, inc.dir, inc.dist);
            c += li;
            mix_add(light_group(l), mix_beta * li);
        }
    }
    return c;
}

// Distance along `o + t d` to the emitting surface of area light `l` (-1 = miss).
fn area_hit(l: Light, o: vec3<f32>, d: vec3<f32>) -> f32 {
    let rel = o - l.pos.xyz;
    let n = l.dir.xyz;
    let shape = u32(l.area.x + 0.5);
    var t = -1.0;
    if (shape <= 1u) {
        let dn = dot(d, n);
        if (dn >= -1e-6) { return -1.0; }
        t = -dot(rel, n) / dn;
        let q = rel + d * t;
        let x = dot(q, l.au.xyz);
        let y = dot(q, l.av.xyz);
        let inside = select(x * x + y * y <= l.au.w * l.au.w, abs(x) <= l.au.w && abs(y) <= l.av.w, shape == 0u);
        if (!inside) { return -1.0; }
    } else if (shape == 2u) {
        let b = dot(rel, d);
        let disc = b * b - (dot(rel, rel) - l.au.w * l.au.w);
        if (disc < 0.0) { return -1.0; }
        t = -b - sqrt(disc);
    } else {
        let ax = l.au.xyz;
        let dp = d - ax * dot(d, ax);
        let rp = rel - ax * dot(rel, ax);
        let a = dot(dp, dp);
        let b = dot(dp, rp);
        let disc = b * b - a * (dot(rp, rp) - l.au.w * l.au.w);
        if (a < 1e-9 || disc < 0.0) { return -1.0; }
        t = (-b - sqrt(disc)) / a;
        if (abs(dot(rel + d * t, ax)) > l.av.w) { return -1.0; }
    }
    return select(-1.0, t, t > 1e-4);
}

// Radiance of the nearest camera-visible area light closer than `tmax`
// (w = 1 + its LightMix group when one is seen, 0 otherwise).
fn area_seen(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> vec4<f32> {
    var best = tmax;
    var out = vec4<f32>(0.0);
    if (!FEAT_AREA) { return out; }
    for (var i = 0u; i < u.counts.x; i++) {
        let l = lights[i];
        if (l.pos.w < 2.5 || l.area.y < 0.5) { continue; }
        let t = area_hit(l, o, d);
        if (t > 0.0 && t < best) {
            best = t;
            let shape = u32(l.area.x + 0.5);
            let proj = select(4.0 * l.au.w * l.av.w, PI * l.au.w * l.au.w, shape == 1u || shape == 2u);
            out = vec4<f32>(l.rad.xyz / proj, 1.0 + f32(light_group(l)));
        }
    }
    return out;
}
