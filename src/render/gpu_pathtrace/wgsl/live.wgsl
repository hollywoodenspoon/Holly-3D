// Live maps (layout in src/render/gpu_pathtrace/live.rs): MapNode::sample
// and materials/patterns.rs evaluated per hit, with the noise crate's
// permutation tables (live::SEEDS) read from the texture store.

var<private> live_nt: u32;

fn perm(t: u32, i: i32) -> i32 { return i32(aux[live_nt + t * 256u + u32(i & 255)]); }

// PermutationTable::hash of a 3D cell.
fn nhash(t: u32, c: vec3<i32>) -> i32 {
    var a = perm(t, c.x) ^ (c.y & 255);
    a = perm(t, a) ^ (c.z & 255);
    return perm(t, a);
}

fn grad3(h: i32, p: vec3<f32>) -> f32 {
    switch (h & 15) {
        case 0, 12: { return p.x + p.y; }
        case 1, 13: { return -p.x + p.y; }
        case 2: { return p.x - p.y; }
        case 3: { return -p.x - p.y; }
        case 4: { return p.x + p.z; }
        case 5: { return -p.x + p.z; }
        case 6: { return p.x - p.z; }
        case 7: { return -p.x - p.z; }
        case 8: { return p.y + p.z; }
        case 9, 14: { return -p.y + p.z; }
        case 10: { return p.y - p.z; }
        default: { return -p.y - p.z; }
    }
}

// noise::core::perlin::perlin_3d, about -1..1.
fn nperlin(t: u32, p: vec3<f32>) -> f32 {
    let fl = floor(p);
    let c = vec3<i32>(fl);
    let d = p - fl;
    let x = clamp(d, vec3<f32>(0.0), vec3<f32>(1.0));
    let s = x * x * x * (x * (x * 6.0 - 15.0) + 10.0);
    let g000 = grad3(nhash(t, c), d);
    let g100 = grad3(nhash(t, c + vec3<i32>(1, 0, 0)), d - vec3<f32>(1.0, 0.0, 0.0));
    let g010 = grad3(nhash(t, c + vec3<i32>(0, 1, 0)), d - vec3<f32>(0.0, 1.0, 0.0));
    let g110 = grad3(nhash(t, c + vec3<i32>(1, 1, 0)), d - vec3<f32>(1.0, 1.0, 0.0));
    let g001 = grad3(nhash(t, c + vec3<i32>(0, 0, 1)), d - vec3<f32>(0.0, 0.0, 1.0));
    let g101 = grad3(nhash(t, c + vec3<i32>(1, 0, 1)), d - vec3<f32>(1.0, 0.0, 1.0));
    let g011 = grad3(nhash(t, c + vec3<i32>(0, 1, 1)), d - vec3<f32>(0.0, 1.0, 1.0));
    let g111 = grad3(nhash(t, c + vec3<i32>(1, 1, 1)), d - vec3<f32>(1.0, 1.0, 1.0));
    let r = mix(mix(mix(g000, g001, s.z), mix(g010, g011, s.z), s.y), mix(mix(g100, g101, s.z), mix(g110, g111, s.z), s.y), s.x);
    return clamp(r * 1.1547005383792515, -1.0, 1.0);
}

// noise::Fbm<Perlin>::new(11): six octaves, lacunarity 2π/3.
fn nfbm(p0: vec3<f32>) -> f32 {
    var p = p0;
    var r = 0.0;
    var att = 0.5;
    for (var i = 0u; i < 6u; i++) {
        r += nperlin(1u + i, p) * att;
        att *= 0.5;
        p *= 2.0943951023931953;
    }
    return r / 0.984375;
}

fn wpoint(i: i32, c: vec3<i32>) -> vec3<f32> {
    let len = f32((i & 0xE0) >> 5u) * 0.5 / 7.0;
    let dg = len * 0.7071067811865476;
    var v: vec3<f32>;
    switch (i % 18) {
        case 0: { v = vec3<f32>(dg, dg, 0.0); }
        case 1: { v = vec3<f32>(dg, -dg, 0.0); }
        case 2: { v = vec3<f32>(-dg, dg, 0.0); }
        case 3: { v = vec3<f32>(-dg, -dg, 0.0); }
        case 4: { v = vec3<f32>(dg, 0.0, dg); }
        case 5: { v = vec3<f32>(dg, 0.0, -dg); }
        case 6: { v = vec3<f32>(-dg, 0.0, dg); }
        case 7: { v = vec3<f32>(-dg, 0.0, -dg); }
        case 8: { v = vec3<f32>(0.0, dg, dg); }
        case 9: { v = vec3<f32>(0.0, dg, -dg); }
        case 10: { v = vec3<f32>(0.0, -dg, dg); }
        case 11: { v = vec3<f32>(0.0, -dg, -dg); }
        case 12: { v = vec3<f32>(len, 0.0, 0.0); }
        case 13: { v = vec3<f32>(0.0, len, 0.0); }
        case 14: { v = vec3<f32>(0.0, 0.0, len); }
        case 15: { v = vec3<f32>(-len, 0.0, 0.0); }
        case 16: { v = vec3<f32>(0.0, -len, 0.0); }
        default: { v = vec3<f32>(0.0, 0.0, -len); }
    }
    return v + vec3<f32>(c);
}

// noise::core::worley::worley_3d (Euclidean, Value), -1..1.
fn nworley(p: vec3<f32>) -> f32 {
    let fl = floor(p);
    let cell = vec3<i32>(fl);
    let fr = p - fl;
    let half = vec3<i32>(fr > vec3<f32>(0.5));
    let near = cell + half;
    let far = cell + vec3<i32>(1) - half;
    var best = near;
    var dist = distance(p, wpoint(nhash(7u, near), near));
    let rg = (0.5 - fr) * (0.5 - fr);
    var tests = array<vec3<i32>, 7>(
        vec3<i32>(far.x, near.y, near.z), vec3<i32>(near.x, far.y, near.z), vec3<i32>(near.x, near.y, far.z),
        vec3<i32>(far.x, far.y, near.z), vec3<i32>(far.x, near.y, far.z), vec3<i32>(near.x, far.y, far.z),
        vec3<i32>(far.x, far.y, far.z));
    for (var k = 0u; k < 7u; k++) {
        var ok: bool;
        switch (k) {
            case 0u: { ok = rg.x < dist; }
            case 1u: { ok = rg.y < dist; }
            case 2u: { ok = rg.z < dist; }
            case 3u: { ok = rg.x < dist && rg.y < dist; }
            case 4u: { ok = rg.x < dist && rg.z < dist; }
            case 5u: { ok = rg.y < dist && rg.z < dist; }
            default: { ok = rg.x < dist && rg.y < dist && rg.z < dist; }
        }
        if (ok) {
            let c = tests[k];
            let dd = distance(p, wpoint(nhash(7u, c), c));
            if (dd < dist) { dist = dd; best = c; }
        }
    }
    return f32(nhash(7u, best)) / 255.0 * 2.0 - 1.0;
}

// patterns::hash: integer hash to 0..1.
fn phash(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    var h = (bitcast<u32>(x) * 0x8da6b343u) ^ (bitcast<u32>(y) * 0xd8163841u) ^ (bitcast<u32>(z) * 0xcb1ab31fu);
    h ^= seed * 0x2c1b3c6du;
    h = (h ^ (h >> 15u)) * 0x2c1b3c6du;
    h = (h ^ (h >> 12u)) * 0x297a2d39u;
    h ^= h >> 15u;
    return f32(h & 0x00ffffffu) / 16777215.0;
}

// patterns::fbm: four octaves of Perlin(29), about 0..1.
fn pfbm(p: vec3<f32>) -> f32 {
    var s = 0.0;
    var a = 0.5;
    var q = p;
    for (var i = 0; i < 4; i++) {
        s += nperlin(8u, q) * a;
        q = q * 2.03 + vec3<f32>(5.1);
        a *= 0.5;
    }
    return clamp(s + 0.5, 0.0, 1.0);
}

fn psmooth(a: f32, b: f32, x: f32) -> f32 {
    let t = clamp((x - a) / (b - a), 0.0, 1.0);
    return t * t * (3.0 - 2.0 * t);
}

fn pvaried(c: vec3<f32>, amount: f32, h: f32) -> vec3<f32> {
    return clamp(c * (1.0 + (h - 0.5) * amount), vec3<f32>(0.0), vec3<f32>(1.0));
}

// Rust's f32::fract (keeps the sign).
fn rfract(x: f32) -> f32 { return x - trunc(x); }

fn pvoronoi(p: vec3<f32>, seed: u32) -> vec3<f32> {
    let c = vec3<i32>(floor(p));
    var f1 = 9.0;
    var f2 = 9.0;
    var id = 0.0;
    for (var z = -1; z <= 1; z++) {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let i = c + vec3<i32>(x, y, z);
                let o = vec3<f32>(phash(i.x, i.y, i.z, seed), phash(i.x, i.y, i.z, seed + 1u), phash(i.x, i.y, i.z, seed + 2u));
                let d = length(vec3<f32>(i) + o - p);
                if (d < f1) {
                    f2 = f1;
                    f1 = d;
                    id = phash(i.x, i.y, i.z, seed + 3u);
                } else if (d < f2) {
                    f2 = d;
                }
            }
        }
    }
    return vec3<f32>(f1, f2, id);
}

struct LiveSlot { kind: u32, sub: u32, seed: u32, amount: f32, s: vec4<f32>, a: vec3<f32>, b: vec3<f32> }

fn live_slot(r: u32, k: u32) -> LiveSlot {
    let o = r + 16u + k * 16u;
    return LiveSlot(aux[o], aux[o + 1u], aux[o + 2u], auxf(o + 3u),
        vec4<f32>(auxf(o + 4u), auxf(o + 5u), auxf(o + 6u), auxf(o + 7u)),
        vec3<f32>(auxf(o + 8u), auxf(o + 9u), auxf(o + 10u)), vec3<f32>(auxf(o + 11u), auxf(o + 12u), auxf(o + 13u)));
}

fn pbricks(m: LiveSlot, uv: vec2<f32>) -> vec3<f32> {
    let cols = max(m.s.x, 0.5);
    let rows = max(m.s.y, 0.5);
    let q = uv * vec2<f32>(cols, rows);
    let row = floor(q.y);
    var shift = 0.0;
    if (m.sub == 0u || m.sub == 3u) { shift = (row - 2.0 * floor(row / 2.0)) * 0.5; }
    if (m.sub == 2u) { shift = phash(i32(row), 0, 1, m.seed); }
    let x = q.x + shift;
    let f = vec2<f32>(rfract(x), q.y - row);
    let aspect = rows / cols;
    let dx = min(f.x, 1.0 - f.x) / aspect;
    let dy = min(f.y, 1.0 - f.y);
    var d = min(dx, dy);
    if (m.sub == 3u) {
        let cx = max(0.3 - dx, 0.0);
        let cy = max(0.3 - dy, 0.0);
        d = 0.3 - sqrt(cx * cx + cy * cy) - pfbm(vec3<f32>(q.x * 3.0, q.y * 3.0, f32(m.seed))) * 0.12;
    }
    let t = psmooth(m.s.z * 0.5, m.s.z * 0.5 + 0.04, d);
    let h = phash(i32(floor(x)), i32(row), 0, m.seed);
    let grain = pfbm(vec3<f32>(q.x * 6.0, q.y * 6.0, h * 9.0)) - 0.5;
    let b = pvaried(m.a, m.s.w, h) * (1.0 + grain * 0.25);
    return mix(m.b, clamp(b, vec3<f32>(0.0), vec3<f32>(1.0)), t);
}

fn pcracks(m: LiveSlot, p: vec3<f32>) -> vec3<f32> {
    let q = p * max(m.s.x, 0.01);
    let warp = vec3<f32>(pfbm(q * 0.7), pfbm(q * 0.7 + vec3<f32>(3.3)), 0.0) * 0.6;
    let v = pvoronoi(q + warp, m.seed);
    let t = psmooth(m.s.y * 0.5, m.s.y * 0.5 + 0.05, v.y - v.x);
    let s = pvaried(m.a, m.s.z, v.z) * (0.9 + pfbm(q * 4.0) * 0.2);
    return mix(m.b, clamp(s, vec3<f32>(0.0), vec3<f32>(1.0)), t);
}

fn pgrunge(m: LiveSlot, p: vec3<f32>) -> vec3<f32> {
    let q = p * max(m.s.x, 0.01) + vec3<f32>(f32(m.seed) * 7.31);
    let a = clamp(m.s.y, 0.0, 1.0);
    var t = 0.0;
    if (m.sub == 0u) {
        t = psmooth(0.75 - a * 0.6, 0.85 - a * 0.6, pfbm(q));
    } else if (m.sub == 1u) {
        let f1 = pvoronoi(q * 2.0, m.seed).x;
        let n = pfbm(q) * 0.7 + (1.0 - min(f1, 1.0)) * 0.3;
        t = psmooth(0.72 - a * 0.55, 0.8 - a * 0.55, n) * (0.75 + pfbm(q * 8.0) * 0.25);
    } else if (m.sub == 2u) {
        var dirs = array<vec3<f32>, 3>(vec3<f32>(1.0, 0.3, 0.1), vec3<f32>(-0.4, 1.0, 0.2), vec3<f32>(0.6, -0.7, 0.3));
        for (var i = 0u; i < 3u; i++) {
            let d = normalize(dirs[i]);
            let along = dot(q, d);
            let across = (q - d * along) * 40.0;
            let line = nperlin(8u, across + vec3<f32>(f32(i) * 11.0));
            let mask = pfbm(q * 0.8 + vec3<f32>(f32(i) * 3.0));
            t = max(t, max(1.0 - abs(line) * 12.0, 0.0) * psmooth(0.65 - a * 0.4, 0.75 - a * 0.4, mask));
        }
    } else {
        let n = pfbm(q * 0.5);
        let ring = 1.0 - min(abs(n - (0.7 - a * 0.4)) * 14.0, 1.0);
        t = ring * 0.7 + psmooth(0.72 - a * 0.4, 0.9 - a * 0.4, n) * 0.4;
    }
    return mix(m.a, m.b, clamp(t, 0.0, 1.0));
}

// MapNode::sample (rgb) for the kinds live::kind lists.
fn live_node(m: LiveSlot, uv: vec2<f32>, p: vec3<f32>) -> vec3<f32> {
    switch (m.kind) {
        case 1u: {
            let q = p * m.s.x + vec3<f32>(f32(m.seed) * 17.13);
            var t: f32;
            if (m.sub == 0u) { t = nperlin(0u, q); } else if (m.sub == 1u) { t = nfbm(q); } else { t = nworley(q); }
            return mix(m.a, m.b, clamp(t * 0.5 + 0.5, 0.0, 1.0));
        }
        case 2u: { return pbricks(m, uv); }
        case 3u: { return pcracks(m, p); }
        case 4u: { return pgrunge(m, p); }
        case 5u: {
            let q = p * m.s.x;
            let n = nfbm(q * vec3<f32>(2.0, 2.0, 0.4));
            let r = length(q.xy) * m.s.y + n * m.s.z * 4.0;
            let band = sin(rfract(r) * 6.283185307179586) * 0.5 + 0.5;
            return mix(m.a, m.b, pow(band, 2.5));
        }
        case 6u: {
            let c = floor(uv * m.s.x);
            let k = c.x + c.y;
            return select(m.b, m.a, k - 2.0 * floor(k / 2.0) < 1.0);
        }
        case 7u: {
            var t = clamp(uv.y, 0.0, 1.0);
            if (m.sub == 1u) { t = min(length(uv - vec2<f32>(0.5)) * 2.0, 1.0); }
            let md = clamp(m.s.x, 0.01, 0.99);
            if (t < md) { return mix(m.a, m.b, t / md); }
            return mix(m.b, m.s.yzw, (t - md) / (1.0 - md));
        }
        case 8u: {
            let along_u = (m.sub & 2u) != 0u;
            let across = select(uv.x, uv.y, along_u);
            let along = select(uv.y, uv.x, along_u);
            let f = rfract(across * max(m.s.x, 0.01));
            if ((m.sub & 1u) != 0u) { return mix(m.b, m.a, 0.5 + 0.5 * cos(f * 6.283185307179586)); }
            let w = clamp(m.s.y, 0.0, 1.0) * 0.5;
            var t = 1.0 - psmooth(w - 0.01, w + 0.01, abs(f - 0.5));
            if (m.s.z > 0.0 && rfract(along * m.s.z) > 0.5) { t = 0.0; }
            return mix(m.b, m.a, t);
        }
        default: { return vec3<f32>(0.0); }
    }
}

fn lum(c: vec3<f32>) -> f32 { return c.x * 0.2126 + c.y * 0.7152 + c.z * 0.0722; }

fn srgb_lin(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
}

// shading::base_surface + relief::base_tangent_normal for live record `r`
// at `uv` / `p`; `du`, `dv`: how the point moves with U and V.
fn live_shade(r: u32, uv: vec2<f32>, p: vec3<f32>, du: vec3<f32>, dv: vec3<f32>) -> Tx {
    live_nt = aux[r + 9u];
    var base = vec3<f32>(auxf(r), auxf(r + 1u), auxf(r + 2u));
    var metal = auxf(r + 3u);
    var rough = auxf(r + 4u);
    var emit = vec3<f32>(auxf(r + 5u), auxf(r + 6u), auxf(r + 7u));
    let s0 = live_slot(r, 0u);
    if (s0.kind != 0u) { base = mix(base, live_node(s0, uv, p), s0.amount); }
    let s1 = live_slot(r, 1u);
    if (s1.kind != 0u) { metal = metal * (1.0 - s1.amount) + lum(live_node(s1, uv, p)) * s1.amount; }
    let s2 = live_slot(r, 2u);
    if (s2.kind != 0u) { rough = rough * (1.0 - s2.amount) + lum(live_node(s2, uv, p)) * s2.amount; }
    let s4 = live_slot(r, 4u);
    if (s4.kind != 0u) { emit += srgb_lin(live_node(s4, uv, p)) * s4.amount * auxf(r + 8u); }
    var tn = vec3<f32>(0.0, 0.0, 1.0);
    let s3 = live_slot(r, 3u);
    if (s3.kind != 0u) {
        let e = 1.0 / 512.0;
        let hx0 = lum(live_node(s3, uv + vec2<f32>(e, 0.0), p + du * e));
        let hx1 = lum(live_node(s3, uv - vec2<f32>(e, 0.0), p - du * e));
        let hy0 = lum(live_node(s3, uv + vec2<f32>(0.0, e), p + dv * e));
        let hy1 = lum(live_node(s3, uv - vec2<f32>(0.0, e), p - dv * e));
        let dx = (hx0 - hx1) / (2.0 * e);
        let dy = (hy0 - hy1) / (2.0 * e);
        let t = normalize(vec3<f32>(-dx / 32.0, -dy / 32.0, 1.0));
        let l = mix(vec3<f32>(0.0, 0.0, 1.0), t, s3.amount);
        tn = select(vec3<f32>(0.0, 0.0, 1.0), normalize(l), dot(l, l) > 1e-12);
    }
    let b = srgb_lin(base);
    return Tx(vec4<f32>(b, 1.0), vec4<f32>(emit, clamp(metal, 0.0, 1.0)), vec4<f32>(clamp(rough, 0.02, 1.0), tn));
}
