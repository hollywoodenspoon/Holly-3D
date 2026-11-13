// Direct light: punctual lights and emissive triangles (NEE).

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

// Hook for area lights and IES profiles (PR 4): no contribution until then.
fn area_nee(p: vec3<f32>, ng: vec3<f32>, s: Surf, v: vec3<f32>) -> vec3<f32> {
    if (FEAT_AREA) {
        // PR 4: rect / disc / sphere / tube sampling.
    }
    return vec3<f32>(0.0);
}
