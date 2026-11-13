// Integrator and entry point (matches PathTracer::radiance).

fn camera_ray(px: vec2<f32>) -> array<vec3<f32>, 2> {
    let ndc = vec2<f32>(px.x / f32(u.dims.x) * 2.0 - 1.0, 1.0 - px.y / f32(u.dims.y) * 2.0);
    let a = u.inv_vp * vec4<f32>(ndc, 1.0, 1.0);
    let far_z = select(1e-4, 0.0, (u.counts.w & F_ORTHO) != 0u);
    let b = u.inv_vp * vec4<f32>(ndc, far_z, 1.0);
    let near = a.xyz / a.w;
    return array<vec3<f32>, 2>(near, normalize(b.xyz / b.w - near));
}

fn radiance(o_in: vec3<f32>, d_in: vec3<f32>) -> vec4<f32> {
    var o = o_in;
    var d = d_in;
    var l = vec3<f32>(0.0);
    var beta = vec3<f32>(1.0);
    var alpha = 0.0;
    var camera = true;
    var prev_pdf = 0.0;
    var prev_delta = true;
    var depth = vec4<u32>(0u);
    // Dispersion: the colour channel this path follows once split (-1 = none).
    var channel = -1;
    var passes = 0u;
    // Shadow catcher seen by the camera: its shadow, AO and reflection weight
    // while the nested reflection path runs (`alpha` is then that path's coverage).
    var caught = false;
    var catch_d = vec2<f32>(0.0);
    var catch_f = 0.0;
    let eps = u.background.w;
    // Room for every bounce plus 32 see-through passes.
    for (var guard = 0; guard < 128; guard++) {
        let h = closest(o, d, 1e30);
        if (FEAT_VOLUME) {
            let b0 = beta;
            let lv = b0 * volume_segment(o, d, h.t, &beta);
            l += lv;
            mix_add(G_VOLUME, lv);
        }
        if (FEAT_AREA && camera) {
            let a = area_seen(o, d, h.t);
            if (a.w > 0.5) {
                l += beta * a.xyz;
                mix_add(u32(a.w - 0.5), beta * a.xyz);
                alpha = 1.0;
                break;
            }
        }
        if (h.tri == 0xffffffffu) {
            if (!camera) {
                let le = beta * environment(d);
                l += le;
                mix_add(select(G_ENV, G_SUN_SKY, (u.counts.w & F_PROC_SKY) != 0u), le);
            }
            break;
        }
        let t = tris[h.tri];
        let w = vec3<f32>(1.0 - h.u - h.v, h.u, h.v);
        let p = t.p0.xyz * w.x + t.p1.xyz * w.y + t.p2.xyz * w.z;
        let geo = normalize(cross(t.p1.xyz - t.p0.xyz, t.p2.xyz - t.p0.xyz));
        let entering = dot(geo, d) < 0.0;
        let ng = select(-geo, geo, entering);
        var n = normalize(t.n0.xyz * w.x + t.n1.xyz * w.y + t.n2.xyz * w.z);
        if (dot(n, ng) < 0.0) { n = -n; }
        var base = t.c0.xyz * w.x + t.c1.xyz * w.y + t.c2.xyz * w.z;
        var mr = t.mat.xy;
        var emit = t.emit.xyz;
        var tx = TexHit(false, vec3<f32>(0.0), vec3<f32>(0.0), vec2<f32>(0.0), n, 1.0);
        if (FEAT_TEXTURES) {
            tx = tex_shade(h.tri, w, p, ng, n);
            if (tx.ok) {
                base *= tx.base;
                emit = tx.emit;
                mr = tx.mr;
                n = tx.n;
            }
        }
        if (FEAT_OPACITY) {
            // Stochastic opacity: pass straight through with probability 1 - alpha.
            let a = alpha_of(h.tri, w, tx);
            if (a < 0.999 && passes < 32u && rnd() >= a) {
                passes += 1u;
                o = p + d * eps;
                continue;
            }
        }
        let rec = rec_of(h.tri);
        if (camera) {
            alpha = 1.0;
            // Preserve Texture Appearance: the camera sees the surface's own colour.
            if (rec != NO_REC && auxf(rec + 11u) >= 0.0) { emit = base * auxf(rec + 11u); }
            if (FEAT_CATCHER && t.light.z > 0.5) {
                let dark = catch_dark(p, ng, n);
                alpha = catcher_alpha(dark.x, dark.y, 0.0);
                if (caught) { break; }
                // The reflection, unless it sees another catcher, runs as a nested camera path.
                let ro = p + ng * eps;
                let rd = d - 2.0 * dot(d, n) * n;
                let rh = closest(ro, rd, 1e30);
                if (rh.tri == 0xffffffffu || tris[rh.tri].light.z > 0.5) { break; }
                caught = true;
                catch_d = dark;
                catch_f = catch_fresnel(n, d, mr.y);
                o = ro;
                d = rd;
                beta = vec3<f32>(catch_f);
                alpha = 0.0;
                prev_pdf = 0.0;
                prev_delta = true;
                depth = vec4<u32>(0u);
                channel = -1;
                passes = 0u;
                continue;
            }
        }
        if (u.fog.w > 0.0) {
            let f = exp(-u.fog.w * h.t);
            l += beta * u.fog.xyz * (1.0 - f);
            mix_add(G_ENV, beta * u.fog.xyz * (1.0 - f));
            beta *= f;
        }
        let model = u32(t.emit.w + 0.5);
        if (!entering && model == 2u) {
            beta *= pow(max(t.att.xyz, vec3<f32>(1e-3)), vec3<f32>(h.t / max(t.att.w, 1e-3)));
        }
        if (FEAT_PRINCIPLED && !entering && t.mat.z > 0.0 && rec != NO_REC) {
            // Beer-Lambert absorption inside the transmissive volume.
            beta *= exp(vec3<f32>(auxf(rec + 8u), auxf(rec + 9u), auxf(rec + 10u)) * h.t);
        }
        if (max(emit.x, max(emit.y, emit.z)) > 0.0) {
            var mis = 1.0;
            if (!prev_delta && u.counts.y > 0u && (u.counts.w & F_NEE) != 0u && t.light.x > 0.0) {
                let lp = t.light.x / max(t.light.y, 1e-12) * h.t * h.t / max(abs(dot(geo, d)), 1e-6);
                mis = power(prev_pdf, lp);
            }
            l += beta * emit * mis;
            mix_add(G_EMISSIVE, beta * emit * mis);
        }
        if (model == 1u) {
            l += beta * base;
            mix_add(G_EMISSIVE, beta * base);
            break;
        }
        if (FEAT_TOON && model == 3u && rec != NO_REC) {
            // Like the CPU, toon shading is all direct light, in the Environment group.
            let lt = beta * toon(p, ng, n, -d, base, mr.y, rec, h.tri);
            l += lt;
            mix_add(G_ENV, lt);
            break;
        }
        camera = false;
        var s = surf(n, base, mr.x, max(mr.y * mr.y, 2e-3), t.mat.z, max(t.mat.w, 1.0), entering);
        if (FEAT_PRINCIPLED || FEAT_SSS) {
            s = principled(s, rec, mr.y);
            s.channel = channel;
        }
        let v = -d;
        mix_beta = beta;
        l += beta * (punctual(p, ng, s, v) + emitter_nee(p, ng, h.tri, s, v) + area_nee(p, ng, s, v));
        var bs = bsdf_sample(s, v);
        if (!bs.ok) { break; }
        depth.x += 1u;
        depth[bs.lobe + 1u] += 1u;
        if (depth.x > u.limits.x || depth[bs.lobe + 1u] > max(u.limits[bs.lobe + 1u], 1u)) { break; }
        beta *= bs.weight;
        if (channel < 0) { channel = bs.channel; }
        var at = p;
        var at_ng = ng;
        if (FEAT_SSS && bs.sss) {
            // Random walk inside, then a white Lambert exit with NEE.
            let wk = random_walk(rec, p, ng, n);
            if (!wk.ok) { break; }
            beta *= wk.w;
            var ex = surf(wk.n, vec3<f32>(1.0), 0.0, 1.0, 0.0, 1.5, false);
            ex.t = frame_t(wk.n, 0.0);
            ex.b = cross(wk.n, ex.t);
            ex.white = true;
            mix_beta = beta;
            l += beta * (punctual(wk.p, wk.ng, ex, wk.n) + emitter_nee(wk.p, wk.ng, wk.tri, ex, wk.n) + area_nee(wk.p, wk.ng, ex, wk.n));
            bs = bsdf_sample(ex, wk.n);
            if (!bs.ok) { break; }
            at = wk.p;
            at_ng = wk.ng;
        }
        if (depth.x > u.counts.z) {
            let survive = clamp(max(beta.x, max(beta.y, beta.z)), 0.0, 0.95);
            if (rnd() >= survive) { break; }
            beta /= survive;
        }
        if (max(beta.x, max(beta.y, beta.z)) <= 0.0) { break; }
        prev_pdf = bs.pdf;
        prev_delta = bs.delta;
        o = at + at_ng * select(-eps, eps, dot(bs.dir, at_ng) >= 0.0);
        d = bs.dir;
    }
    if (caught) { alpha = catcher_alpha(catch_d.x, catch_d.y, catch_f * alpha); }
    return vec4<f32>(l, alpha);
}

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    if (id.x >= u.dims.z || id.y >= u.dims.w) { return; }
    // Keeps the env and texture bindings in every variant's layout (the host always binds them).
    _ = envmap[0];
    _ = aux[0];
    let i = id.y * u.dims.z + id.x;
    if (done[i] != 0u) { return; }
    let x = u.origin.x + id.x;
    let y = u.origin.y + id.y;
    _ = groups[0];
    var a = accum[i];
    var st = stats[i];
    var gsum: array<vec3<f32>, 12>;
    for (var k = 0u; k < u.origin.z; k++) {
        if (FEAT_LIGHTMIX) { mix_l = array<vec3<f32>, 12>(); }
        rng_state = pcg(pcg(x * 1973u + y * 9277u + 26699u) ^ pcg(u.origin.w * 7919u + k + u32(st.z) * 31u));
        let ray = camera_ray(vec2<f32>(f32(x) + rnd(), f32(y) + rnd()));
        var c = radiance(ray[0], ray[1]);
        let bad = any(c != c) || any(abs(c) > vec4<f32>(1e30));
        if (bad) { c = vec4<f32>(0.0, 0.0, 0.0, c.w); }
        // Like the CPU, a sample that is not finite adds to no group.
        if (FEAT_LIGHTMIX && !bad) {
            for (var g = 0u; g < 12u; g++) { gsum[g] += mix_l[g]; }
        }
        a += c;
        let lum = dot(c.xyz, vec3<f32>(0.2126, 0.7152, 0.0722));
        st += vec4<f32>(lum, lum * lum, 1.0, 0.0);
    }
    accum[i] = a;
    stats[i] = st;
    if (FEAT_LIGHTMIX) {
        let k = u32(u.vol4.y + 0.5);
        for (var g = 0u; g < k; g++) {
            let j = (i * k + g) * 3u;
            groups[j] += gsum[g].x;
            groups[j + 1u] += gsum[g].y;
            groups[j + 2u] += gsum[g].z;
        }
    }
}
