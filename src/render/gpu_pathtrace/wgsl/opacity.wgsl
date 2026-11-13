// See-through surfaces (FEAT_OPACITY; matches Tracer::transmittance and the
// path integrator's stochastic opacity). Alpha is the corners' value (vertex
// alpha, or the material's opacity) times the texture's alpha, if any.

fn alpha_of(tri: u32, w: vec3<f32>, tx: TexHit) -> f32 {
    let t = tris[tri];
    var a = t.c0.w * w.x + t.c1.w * w.y + t.c2.w * w.z;
    if (tx.ok) { a *= tx.alpha; }
    return clamp(a, 0.0, 1.0);
}

// Alpha of hit `h` on a ray along `d`.
fn hit_alpha(h: Hit, o: vec3<f32>, d: vec3<f32>) -> f32 {
    let t = tris[h.tri];
    let w = vec3<f32>(1.0 - h.u - h.v, h.u, h.v);
    let p = t.p0.xyz * w.x + t.p1.xyz * w.y + t.p2.xyz * w.z;
    let geo = normalize(cross(t.p1.xyz - t.p0.xyz, t.p2.xyz - t.p0.xyz));
    let ng = select(-geo, geo, dot(geo, d) < 0.0);
    var n = normalize(t.n0.xyz * w.x + t.n1.xyz * w.y + t.n2.xyz * w.z);
    if (dot(n, ng) < 0.0) { n = -n; }
    var tx = TexHit(false, vec3<f32>(0.0), vec3<f32>(0.0), vec2<f32>(0.0), n, 1.0);
    if (FEAT_TEXTURES) { tx = tex_shade(h.tri, w, p, ng, n); }
    return alpha_of(h.tri, w, tx);
}

// Light passing along a shadow ray (0 = blocked): opaque hits stop it,
// see-through ones dim it by their alpha and let it on (up to 8 of them).
fn transmit(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> f32 {
    if (!FEAT_OPACITY) { return select(1.0, 0.0, occluded(o, d, tmax)); }
    var at = o;
    var left = tmax;
    var tr = 1.0;
    for (var i = 0; i < 8; i++) {
        let h = closest(at, d, left);
        if (h.tri == 0xffffffffu) { return tr; }
        let a = hit_alpha(h, at, d);
        if (a >= 0.999) { return 0.0; }
        tr *= 1.0 - a;
        if (tr < 1e-3) { return 0.0; }
        let step = h.t + u.background.w;
        at += d * step;
        left -= step;
    }
    return tr;
}
