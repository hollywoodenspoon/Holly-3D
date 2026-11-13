// Participating media: godrays and height mist (matches
// src/render/volumetrics.rs). One jittered sample per step, single
// scattering of every punctual light with Henyey-Greenstein phase and a
// shadow ray, Beer-Lambert transmittance along the segment.
//
// u.vol0 = albedo rgb, density; u.vol1 = base height, falloff, wisps, 1 / wisp scale;
// u.vol2 = wind x, wind y, anisotropy g, intensity; u.vol3 = camera eye, reach
// (6 x scene radius); u.vol4.x = steps.

fn hash3(x: i32, y: i32, z: i32) -> f32 {
    var h = (bitcast<u32>(x) * 0x8da6b343u) ^ (bitcast<u32>(y) * 0xd8163841u) ^ (bitcast<u32>(z) * 0xcb1ab31fu);
    h ^= h >> 13u;
    h *= 0x5bd1e995u;
    h ^= h >> 15u;
    return f32(h & 0x00ffffffu) / 16777216.0;
}

fn vnoise(p: vec3<f32>) -> f32 {
    let fl = floor(p);
    let f = p - fl;
    let w = f * f * (3.0 - f * 2.0);
    let x = i32(fl.x);
    let y = i32(fl.y);
    let z = i32(fl.z);
    let a = mix(mix(hash3(x, y, z), hash3(x + 1, y, z), w.x), mix(hash3(x, y + 1, z), hash3(x + 1, y + 1, z), w.x), w.y);
    let b = mix(mix(hash3(x, y, z + 1), hash3(x + 1, y, z + 1), w.x), mix(hash3(x, y + 1, z + 1), hash3(x + 1, y + 1, z + 1), w.x), w.y);
    return mix(a, b, w.z);
}

fn fbm3(p: vec3<f32>) -> f32 {
    var s = 0.0;
    var a = 0.5;
    var f = 1.0;
    var n = 0.0;
    for (var i = 0; i < 3; i++) {
        s += vnoise(p * f) * a;
        n += a;
        a *= 0.5;
        f *= 2.03;
    }
    return s / n;
}

fn hg_phase(c: f32, g0: f32) -> f32 {
    let g = clamp(g0, -0.99, 0.99);
    let d = 1.0 + g * g - 2.0 * g * c;
    return (1.0 - g * g) / max(4.0 * PI * d * sqrt(d), 1e-12);
}

// Extinction at `p`: height falloff above the base times patchy wisps.
fn sigma_at(p: vec3<f32>) -> f32 {
    let h = max(p.z - u.vol1.x, 0.0);
    var s = u.vol0.w * exp(-u.vol1.y * h);
    let wisps = u.vol1.z;
    if (wisps > 0.0) {
        let n = fbm3((p + vec3<f32>(u.vol2.xy, 0.0)) * u.vol1.w);
        s *= 1.0 - wisps + wisps * clamp(n * 2.0, 0.0, 2.0);
    }
    return s;
}

// In-scattered light along a segment of length `t` from `o` along `d`
// (misses: up to the far side of the scene); scales `beta` by the
// segment's transmittance.
fn volume_segment(o: vec3<f32>, d: vec3<f32>, t: f32, beta: ptr<function, vec3<f32>>) -> vec3<f32> {
    if (!FEAT_VOLUME) { return vec3<f32>(0.0); }
    let len = min(t, u.vol3.w + distance(o, u.vol3.xyz));
    let n = u32(u.vol4.x);
    let dt = len / f32(n);
    var tr = 1.0;
    var inscatter = vec3<f32>(0.0);
    let jitter = rnd();
    let g = u.vol2.z;
    for (var i = 0u; i < n; i++) {
        let p = o + d * ((f32(i) + jitter) * dt);
        let sg = sigma_at(p);
        if (sg <= 0.0) { continue; }
        let step_tr = exp(-sg * dt);
        let weight = tr * (1.0 - step_tr);
        var li = vec3<f32>(0.0);
        for (var j = 0u; j < u.counts.x; j++) {
            let l = lights[j];
            let inc = incident(l, p);
            if (inc.k <= 0.0) { continue; }
            if ((u.counts.w & F_SHADOWS) != 0u && l.misc.y > 0.5 && occluded(p, inc.dir, min(inc.dist - u.background.w, 1e30))) { continue; }
            li += l.rad.xyz * (inc.k * hg_phase(dot(d, inc.dir), g));
        }
        inscatter += li * u.vol0.xyz * (weight * u.vol2.w);
        tr *= step_tr;
        if (tr < 1e-4) {
            tr = 0.0;
            break;
        }
    }
    *beta *= tr;
    return inscatter;
}
