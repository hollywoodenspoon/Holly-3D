// Texture store lookups (FEAT_TEXTURES; layout in src/render/gpu_pathtrace/textures.rs).
// Matches Tracer::interp: UV maps with a normal map tilting the shading
// normal along the triangle's UV frame, or World / Triplanar projections
// blended by the object-space geometric normal. Live-map materials
// (flag 8) evaluate their maps per hit (live.wgsl) instead of a lookup.

@group(0) @binding(9) var<storage, read> aux: array<u32>;

// One texel: (base rgb, alpha), (emission rgb, metallic), (roughness, tangent normal).
struct Tx { a: vec4<f32>, b: vec4<f32>, c: vec4<f32> }

struct TexHit { ok: bool, base: vec3<f32>, emit: vec3<f32>, mr: vec2<f32>, n: vec3<f32>, alpha: f32 }

fn auxf(i: u32) -> f32 { return bitcast<f32>(aux[i]); }

fn texel(off: u32, i: u32) -> Tx {
    let b = off + i * 6u;
    return Tx(
        vec4<f32>(unpack2x16float(aux[b]), unpack2x16float(aux[b + 1u])),
        vec4<f32>(unpack2x16float(aux[b + 2u]), unpack2x16float(aux[b + 3u])),
        vec4<f32>(unpack2x16float(aux[b + 4u]), unpack2x16float(aux[b + 5u])),
    );
}

fn tx_mad(acc: Tx, x: Tx, k: f32) -> Tx {
    return Tx(acc.a + x.a * k, acc.b + x.b * k, acc.c + x.c * k);
}

// Cut-out materials (flag 16): the filtered alpha snaps back to 0 or 1,
// as the CPU thresholds it at the exact UV.
fn cut(x: Tx, flags: u32) -> Tx {
    if ((flags & 16u) == 0u) { return x; }
    return Tx(vec4<f32>(x.a.xyz, select(0.0, 1.0, x.a.w >= 0.5)), x.b, x.c);
}

fn tex_index(i: i32, n: u32, wrap: bool) -> u32 {
    let m = i32(n);
    if (wrap) { return u32(((i % m) + m) % m); }
    return u32(clamp(i, 0, m - 1));
}

// Bilinear lookup in texture entry `e` (word index of its table row).
fn tex_lookup(e: u32, uv: vec2<f32>) -> Tx {
    let off = aux[e];
    let w = aux[e + 1u];
    let h = aux[e + 2u];
    let wrap = (aux[e + 3u] & 1u) != 0u;
    let lo = vec2<f32>(auxf(e + 4u), auxf(e + 5u));
    let span = vec2<f32>(auxf(e + 6u), auxf(e + 7u));
    var g = (uv - lo) / span;
    if (wrap) { g = fract(g); }
    let f = g * vec2<f32>(f32(w), f32(h)) - 0.5;
    let fl = floor(f);
    let fr = f - fl;
    let x0 = tex_index(i32(fl.x), w, wrap);
    let x1 = tex_index(i32(fl.x) + 1, w, wrap);
    let y0 = tex_index(i32(fl.y), h, wrap);
    let y1 = tex_index(i32(fl.y) + 1, h, wrap);
    var acc = Tx(vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0));
    acc = tx_mad(acc, texel(off, y0 * w + x0), (1.0 - fr.x) * (1.0 - fr.y));
    acc = tx_mad(acc, texel(off, y0 * w + x1), fr.x * (1.0 - fr.y));
    acc = tx_mad(acc, texel(off, y1 * w + x0), (1.0 - fr.x) * fr.y);
    acc = tx_mad(acc, texel(off, y1 * w + x1), fr.x * fr.y);
    return acc;
}

// relief::perturb: tangent normal `tn` in the frame (dpdu, dpdv, n).
fn perturb(n: vec3<f32>, dpdu: vec3<f32>, dpdv: vec3<f32>, tn: vec3<f32>) -> vec3<f32> {
    let t0 = dpdu - n * dot(n, dpdu);
    if (dot(t0, t0) < 1e-24) { return n; }
    let t = normalize(t0);
    let b0 = dpdv - n * dot(n, dpdv) - t * dot(t, dpdv);
    let b = select(cross(n, t), normalize(b0), dot(b0, b0) > 1e-24);
    let r = t * tn.x + b * tn.y + n * tn.z;
    return select(n, normalize(r), dot(r, r) > 1e-24);
}

fn qrot(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    return v + 2.0 * cross(q.xyz, cross(q.xyz, v) + q.w * v);
}

fn tex_shade(tri: u32, w: vec3<f32>, p: vec3<f32>, ng: vec3<f32>, n: vec3<f32>) -> TexHit {
    let x = tri * 16u;
    let id = aux[x + 6u];
    if (!FEAT_TEXTURES || id == 0xffffffffu) { return TexHit(false, vec3<f32>(0.0), vec3<f32>(0.0), vec2<f32>(0.0), n, 1.0); }
    let e = u.tex.x + id * 12u;
    let flags = aux[e + 3u];
    let tilt = (flags & 4u) != 0u;
    var s: Tx;
    var nn = n;
    let evaluated = (flags & 8u) != 0u;
    if ((flags & 2u) == 0u) {
        let uv0 = vec2<f32>(auxf(x), auxf(x + 1u));
        let uv1 = vec2<f32>(auxf(x + 2u), auxf(x + 3u));
        let uv2 = vec2<f32>(auxf(x + 4u), auxf(x + 5u));
        let uv = uv0 * w.x + uv1 * w.y + uv2 * w.z;
        let t = tris[tri];
        let e1 = t.p1.xyz - t.p0.xyz;
        let e2 = t.p2.xyz - t.p0.xyz;
        let d1 = uv1 - uv0;
        let d2 = uv2 - uv0;
        let det = d1.x * d2.y - d2.x * d1.y;
        let framed = abs(det) > 1e-12;
        let dpdu = select(vec3<f32>(0.0), (e1 * d2.y - e2 * d1.y) / det, framed);
        let dpdv = select(vec3<f32>(0.0), (e2 * d1.x - e1 * d2.x) / det, framed);
        if (evaluated) { s = live_shade(aux[e], uv, p, dpdu, dpdv); } else { s = cut(tex_lookup(e, uv), flags); }
        if (tilt && framed) {
            let b = perturb(n, dpdu, dpdv, s.c.yzw);
            if (dot(b, ng) > 0.0) { nn = b; }
        }
    } else {
        // World / Triplanar in the object's own space.
        let q = vec4<f32>(auxf(x + 8u), auxf(x + 9u), auxf(x + 10u), auxf(x + 11u));
        let qi = vec4<f32>(-q.xyz, q.w);
        let origin = vec3<f32>(auxf(x + 12u), auxf(x + 13u), auxf(x + 14u));
        let tp = qrot(qi, p - origin);
        let tn = qrot(qi, ng);
        let scale = auxf(e + 8u);
        let size = 1.0 / scale;
        var wt = pow(abs(tn), vec3<f32>(auxf(e + 9u)));
        let sum = wt.x + wt.y + wt.z;
        wt = select(vec3<f32>(0.0, 0.0, 1.0), wt / sum, sum > 1e-12);
        let live = select(vec3<f32>(0.0), wt, wt > vec3<f32>(0.01));
        let k = live / (live.x + live.y + live.z);
        let c = tp * scale;
        var uvs = array<vec2<f32>, 3>(c.yz, c.xz, c.xy);
        let ax = qrot(q, vec3<f32>(size, 0.0, 0.0));
        let ay = qrot(q, vec3<f32>(0.0, size, 0.0));
        let az = qrot(q, vec3<f32>(0.0, 0.0, size));
        var du = array<vec3<f32>, 3>(ay, ax, ax);
        var dv = array<vec3<f32>, 3>(az, az, ay);
        // The same frames in projection space (relief::triplanar_normal).
        var fu = array<vec3<f32>, 3>(vec3<f32>(0.0, size, 0.0), vec3<f32>(size, 0.0, 0.0), vec3<f32>(size, 0.0, 0.0));
        var fv = array<vec3<f32>, 3>(vec3<f32>(0.0, 0.0, size), vec3<f32>(0.0, 0.0, size), vec3<f32>(0.0, size, 0.0));
        s = Tx(vec4<f32>(0.0), vec4<f32>(0.0), vec4<f32>(0.0));
        var nacc = vec3<f32>(0.0);
        for (var j = 0u; j < 3u; j++) {
            if (k[j] <= 0.0) { continue; }
            var sj: Tx;
            if (evaluated) { sj = live_shade(aux[e], uvs[j], tp, fu[j], fv[j]); } else { sj = cut(tex_lookup(e, uvs[j]), flags); }
            s = tx_mad(s, sj, k[j]);
            nacc += select(n, perturb(n, du[j], dv[j], sj.c.yzw), tilt) * k[j];
        }
        let b = select(n, normalize(nacc), dot(nacc, nacc) > 1e-24);
        if (dot(b, ng) > 0.0) { nn = b; }
    }
    return TexHit(true, s.a.xyz, s.b.xyz, vec2<f32>(s.b.w, s.c.x), nn, clamp(s.a.w, 0.0, 1.0));
}
