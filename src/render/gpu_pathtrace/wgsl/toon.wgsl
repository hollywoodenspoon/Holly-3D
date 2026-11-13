// Cel / toon materials (model 3; matches shading::shade_toon over the CPU
// tracer's direct-light list: every light, area lights at their shadow
// samples, then `u.vol4.z` glowing-triangle picks). The material record
// holds steps, shadow tint, specular, rim and rim colour from word 16.

fn toon_light(c: ptr<function, vec3<f32>>, shade: vec3<f32>, base: vec3<f32>, n: vec3<f32>, v: vec3<f32>, rough: f32, steps: f32, spec: f32, dir: vec3<f32>, radiance: vec3<f32>) {
    let nl = max(dot(n, dir), 0.0);
    let band = min(floor(nl * steps) / (steps - 1.0), 1.0);
    let energy = radiance / PI;
    *c += mix(shade, base, band) * energy;
    let hv = dir + v;
    let h = select(vec3<f32>(0.0), normalize(hv), dot(hv, hv) > 1e-24);
    let shin = 2.0 / max(rough * rough, 1e-3);
    if (spec > 0.0 && nl > 0.0 && pow(max(dot(n, h), 0.0), shin) > 0.5) { *c += vec3<f32>(spec) * energy; }
}

// gi::nee_radiance: one of `n` picks of a glowing triangle.
fn nee_radiance(radiance: vec3<f32>, cos_l: f32, area: f32, d2: f32, prob: f32, n: u32) -> vec3<f32> {
    let solid = min(cos_l * area / max(d2, 0.05), 2.0 * PI);
    let w = solid / (max(prob, 1e-6) * f32(max(n, 1u)));
    return radiance * min(w, 4.0 * 2.0 * PI / f32(max(n, 1u)));
}

fn toon(p: vec3<f32>, ng: vec3<f32>, n: vec3<f32>, v: vec3<f32>, base: vec3<f32>, rough: f32, rec: u32, self_tri: u32) -> vec3<f32> {
    let steps = clamp(auxf(rec + 16u), 2.0, 5.0);
    let shade = base * vec3<f32>(auxf(rec + 17u), auxf(rec + 18u), auxf(rec + 19u));
    let spec = auxf(rec + 20u);
    let rim = auxf(rec + 21u);
    var c = shade * min(max(u.ambient.x, max(u.ambient.y, u.ambient.z)), 1.0) * 0.6;
    let o = p + ng * u.background.w;
    for (var i = 0u; i < u.counts.x; i++) {
        let l = lights[i];
        let k = select(1u, max(u32(l.area.w), 1u), l.pos.w > 2.5);
        for (var j = 0u; j < k; j++) {
            let inc = incident(l, p);
            if (inc.k <= 0.0 || dot(n, inc.dir) <= 0.0) { continue; }
            let rad = l.rad.xyz * (inc.k / f32(k)) * shadow(l, p, o, inc.dir, inc.dist);
            toon_light(&c, shade, base, n, v, rough, steps, spec, inc.dir, rad);
        }
    }
    let picks = u32(u.vol4.z + 0.5);
    let ne = u.counts.y;
    if (ne > 0u) {
        for (var j = 0u; j < picks; j++) {
            let x = rnd();
            var lo = 0u;
            var hi = ne - 1u;
            while (lo < hi) {
                let mid = (lo + hi) / 2u;
                if (emitters[mid].w <= x) { lo = mid + 1u; } else { hi = mid; }
            }
            let e = emitters[lo];
            let ti = bitcast<u32>(e.x);
            if (ti == self_tri) { continue; }
            let t = tris[ti];
            let to = point_on(t) - o;
            let dist = max(length(to), 1e-4);
            let dir = to / dist;
            let nl = normalize(cross(t.p1.xyz - t.p0.xyz, t.p2.xyz - t.p0.xyz));
            let cl = abs(dot(nl, -dir));
            if (dot(n, dir) <= 0.0 || cl <= 1e-4) { continue; }
            if ((u.counts.w & F_SHADOWS) != 0u && occluded(o, dir, dist - u.background.w * 4.0)) { continue; }
            toon_light(&c, shade, base, n, v, rough, steps, spec, dir, nee_radiance(t.emit.xyz, cl, e.z, dist * dist, e.y, picks));
        }
    }
    if (rim > 0.0 && 1.0 - max(dot(n, v), 0.0) > 0.62) {
        c += vec3<f32>(auxf(rec + 22u), auxf(rec + 23u), auxf(rec + 24u)) * rim * 0.6;
    }
    return c;
}
