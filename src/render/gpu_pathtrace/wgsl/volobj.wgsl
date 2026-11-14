// Volume objects (FEAT_VOLOBJ; matches src/render/volumes): clouds, smoke
// and fire as participating media. Delta tracking finds real collisions,
// ratio tracking gives shadow transmittance; both walk each volume's
// majorant grid with a 3D DDA in local space (the direction is not
// normalised there, so `t` stays in world units). Buffer layout: see
// `super::volobj`.

@group(0) @binding(13) var<storage, read> vobj: array<f32>;

const VO_REC: u32 = 88u;
const VO_EDGE: f32 = 0.35;
const VO_TMIN: f32 = 1e-3;
// Tracking steps one ray may take through all cells (a safety net).
const VO_STEPS: u32 = 4096u;

fn vu(i: u32) -> u32 { return bitcast<u32>(vobj[i]); }
fn vv3(i: u32) -> vec3<f32> { return vec3<f32>(vobj[i], vobj[i + 1u], vobj[i + 2u]); }

fn vo_count() -> u32 {
    if (!FEAT_VOLOBJ) { return 0u; }
    return vu(0u);
}

fn vo_rec(v: u32) -> u32 { return 4u + v * VO_REC; }

fn vo_inv(b: u32) -> mat4x4<f32> {
    return mat4x4<f32>(vec4<f32>(vv3(b), vobj[b + 3u]), vec4<f32>(vv3(b + 4u), vobj[b + 7u]), vec4<f32>(vv3(b + 8u), vobj[b + 11u]), vec4<f32>(vv3(b + 12u), vobj[b + 15u]));
}

fn vo_falloff(d2: f32) -> f32 {
    if (d2 >= 1.0) { return 0.0; }
    let x = min((1.0 - sqrt(d2)) / VO_EDGE, 1.0);
    return x * x * (3.0 - 2.0 * x);
}

// 0..=1 shape density at local `p` in grid cell `cell`.
fn vo_density(b: u32, p: vec3<f32>, cell: u32) -> f32 {
    if (vu(b + 23u) == 0u) { return 1.0; }
    let pair = vu(b + 34u) + cell * 2u;
    let start = vu(pair);
    let n = vu(pair + 1u);
    let puffs = vu(b + 35u);
    var keep = 1.0;
    for (var k = 0u; k < n; k++) {
        let q = puffs + vu(start + k) * 6u;
        let r = vobj[q + 3u];
        let h = vec3<f32>(r, r, r * max(vobj[q + 4u], 1e-3));
        var x = (p - vv3(q)) / h;
        if (x.z < 0.0) { x.z /= max(vobj[q + 5u], 1e-3); }
        keep *= 1.0 - clamp(vo_falloff(dot(x, x)), 0.0, 1.0);
    }
    return 1.0 - keep;
}

// One volume along `o + d t`, `t` in 0..tmax. Collide mode: the nearest
// real collision (`t < 0` = none) and its density. Shadow mode: the
// ratio-tracked transmittance in `tr`.
struct VoTrack { t: f32, dens: f32, tr: f32 }

fn vo_track(v: u32, o: vec3<f32>, d: vec3<f32>, tmax: f32, collide: bool) -> VoTrack {
    var res = VoTrack(-1.0, 0.0, 1.0);
    let b = vo_rec(v);
    let m = vo_inv(b);
    let ol = (m * vec4<f32>(o, 1.0)).xyz;
    var dl = (m * vec4<f32>(d, 0.0)).xyz;
    dl = select(dl, vec3<f32>(1e-12), abs(dl) < vec3<f32>(1e-12));
    let lo = vv3(b + 16u);
    let hi = vv3(b + 20u);
    let inv = 1.0 / dl;
    let ta = (lo - ol) * inv;
    let tb = (hi - ol) * inv;
    let t0 = max(max(max(min(ta.x, tb.x), min(ta.y, tb.y)), min(ta.z, tb.z)), 0.0);
    let t1 = min(min(min(max(ta.x, tb.x), max(ta.y, tb.y)), max(ta.z, tb.z)), tmax);
    if (t0 >= t1) { return res; }
    let sigma = vobj[b + 19u];
    let dims = vec3<i32>(i32(vu(b + 24u)), i32(vu(b + 25u)), i32(vu(b + 26u)));
    let mj_at = vu(b + 27u);
    let even = vu(b + 23u) == 0u;
    let cs = (hi - lo) / vec3<f32>(dims);
    var c = clamp(vec3<i32>(floor((ol + dl * (t0 + 1e-4 * (t1 - t0)) - lo) / max(cs, vec3<f32>(1e-9)))), vec3<i32>(0), dims - 1);
    let stp = select(vec3<i32>(-1), vec3<i32>(1), dl > vec3<f32>(0.0));
    let edge = lo + (vec3<f32>(c) + select(vec3<f32>(0.0), vec3<f32>(1.0), dl > vec3<f32>(0.0))) * cs;
    var next = (edge - ol) * inv;
    let delta = cs * abs(inv);
    var t = t0;
    var steps = 0u;
    for (var guard = 0; guard < 256; guard++) {
        var k = 2;
        if (next.x <= next.y && next.x <= next.z) { k = 0; } else if (next.y <= next.z) { k = 1; }
        let exit = min(next[k], t1);
        if (exit > t) {
            let cell = u32((c.z * dims.y + c.y) * dims.x + c.x);
            let mx = vobj[mj_at + cell];
            let mj = mx * sigma;
            if (mj > 0.0) {
                if (!collide && even && mx >= 1.0) {
                    res.tr *= exp(-mj * (exit - t));
                    if (res.tr <= VO_TMIN) { res.tr = 0.0; return res; }
                } else {
                    var tt = t;
                    loop {
                        tt += -log(max(1.0 - rnd(), 1e-7)) / mj;
                        steps += 1u;
                        if (tt >= exit || steps > VO_STEPS) { break; }
                        let dn = vo_density(b, ol + dl * tt, cell);
                        if (collide) {
                            if (rnd() * mj < dn * sigma) { return VoTrack(tt, dn, 0.0); }
                        } else {
                            res.tr *= 1.0 - min(dn * sigma / mj, 1.0);
                            if (res.tr < VO_TMIN) { res.tr = 0.0; return res; }
                        }
                    }
                }
            }
        }
        if (next[k] >= t1 || steps > VO_STEPS) { break; }
        t = exit;
        c[k] += stp[k];
        if (c[k] < 0 || c[k] >= dims[k]) { break; }
        next[k] += delta[k];
    }
    return res;
}

// The nearest real collision with any volume before `tmax`.
struct VoHit { t: f32, dens: f32, v: i32 }

fn vo_collide(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> VoHit {
    var best = VoHit(tmax, 0.0, -1);
    for (var v = 0u; v < vo_count(); v++) {
        let h = vo_track(v, o, d, best.t, true);
        if (h.t >= 0.0) { best = VoHit(h.t, h.dens, i32(v)); }
    }
    return best;
}

// Ratio-tracked transmittance through every volume, averaged over the
// Shadow Quality's estimates.
fn vo_transmit(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> f32 {
    let n = vo_count();
    if (n == 0u) { return 1.0; }
    let est = max(u32(vobj[3u] + 0.5), 1u);
    var sum = 0.0;
    for (var e = 0u; e < est; e++) {
        var tr = 1.0;
        for (var v = 0u; v < n; v++) {
            tr *= vo_track(v, o, d, tmax, false).tr;
            if (tr <= 0.0) { break; }
        }
        sum += tr;
    }
    return sum / f32(est);
}

fn vo_phase(b: u32, c: f32, s: f32) -> f32 {
    let bl = vobj[b + 33u];
    return hg_phase(c, vobj[b + 31u] * s) * (1.0 - bl) + hg_phase(c, vobj[b + 32u] * s) * bl;
}

fn vo_albedo(v: i32) -> vec3<f32> { return vv3(vo_rec(u32(v)) + 28u); }

// Glow at density `dn` (the host samples the material's ramp).
fn vo_glow(v: i32, dn: f32) -> vec3<f32> {
    let b = vo_rec(u32(v));
    if (vobj[b + 36u] < 0.5) { return vec3<f32>(0.0); }
    let x = clamp(dn, 0.0, 1.0) * 15.0;
    let i = min(u32(x), 14u);
    let a = b + 37u + i * 3u;
    return mix(vv3(a), vv3(a + 3u), x - f32(i));
}

fn sample_hg(w: vec3<f32>, g0: f32) -> vec3<f32> {
    let g = clamp(g0, -0.99, 0.99);
    let u1 = rnd();
    var c = 1.0 - 2.0 * u1;
    if (abs(g) >= 1e-3) {
        let s = (1.0 - g * g) / (1.0 + g - 2.0 * g * u1);
        c = (1.0 + g * g - s * s) / (2.0 * g);
    }
    c = clamp(c, -1.0, 1.0);
    let sn = sqrt(max(1.0 - c * c, 0.0));
    let phi = 2.0 * PI * rnd();
    return normalize(onb(w) * vec3<f32>(sn * cos(phi), sn * sin(phi), c));
}

fn vo_sample_phase(v: i32, d: vec3<f32>) -> vec3<f32> {
    let b = vo_rec(u32(v));
    let g = select(vobj[b + 31u], vobj[b + 32u], rnd() < vobj[b + 33u]);
    return sample_hg(d, g);
}

// Light scattered towards `-d` at `p` inside volume `v` (before the
// albedo): every light, shadowed by surfaces and volumes, times the phase
// plus the Cloud Bounce Boost octaves.
fn vo_nee(v: i32, p: vec3<f32>, d: vec3<f32>) -> vec3<f32> {
    let b = vo_rec(u32(v));
    let boost = vobj[2u];
    var c = vec3<f32>(0.0);
    for (var i = 0u; i < u.counts.x; i++) {
        let l = lights[i];
        let inc = incident(l, p);
        if (inc.k <= 0.0) { continue; }
        let cs = dot(d, inc.dir);
        let len = min(inc.dist - u.background.w, 1e30);
        var surf = 1.0;
        var tv = 1.0;
        if ((u.counts.w & F_SHADOWS) != 0u && l.misc.y > 0.5) {
            surf = transmit_surface(p, inc.dir, len);
            tv = select(0.0, vo_transmit(p, inc.dir, len), surf > 0.0);
        }
        if (surf <= 0.0) { continue; }
        var pv = vo_phase(b, cs, 1.0) * tv;
        if (boost > 0.0) {
            pv += boost * (0.5 * vo_phase(b, cs, 0.5) * pow(tv, 0.5) + 0.25 * vo_phase(b, cs, 0.25) * pow(tv, 0.25));
        }
        if (pv > 0.0) {
            let li = l.rad.xyz * (inc.k * surf * pv);
            c += li;
            mix_add(light_group(l), mix_beta * li);
        }
    }
    return c;
}
