// See-through surfaces (FEAT_OPACITY; matches Tracer::transmittance and the
// path integrator's stochastic opacity). Alpha is the corners' value (vertex
// alpha, or the material's opacity) times the texture's alpha, if any.

fn alpha_of(tri: u32, w: vec3<f32>, tx: TexHit) -> f32 {
    let t = tris[tri];
    var a = t.c0.w * w.x + t.c1.w * w.y + t.c2.w * w.z;
    if (tx.ok) { a *= tx.alpha; }
    return clamp(a, 0.0, 1.0);
}

// Alpha of hit `h` on a ray along `d` (texture alpha only, no shading).
fn hit_alpha(h: Hit, d: vec3<f32>) -> f32 {
    let t = tris[h.tri];
    let w = vec3<f32>(1.0 - h.u - h.v, h.u, h.v);
    var a = t.c0.w * w.x + t.c1.w * w.y + t.c2.w * w.z;
    if (FEAT_TEXTURES) {
        let p = t.p0.xyz * w.x + t.p1.xyz * w.y + t.p2.xyz * w.z;
        let geo = normalize(cross(t.p1.xyz - t.p0.xyz, t.p2.xyz - t.p0.xyz));
        a *= tex_alpha(h.tri, w, p, select(-geo, geo, dot(geo, d) < 0.0));
    }
    return clamp(a, 0.0, 1.0);
}

// Light passing along a shadow ray (0 = blocked): opaque hits stop it,
// see-through ones dim it by their alpha and let it on (up to `u.vol4.w`,
// 8, of them: a uniform so drivers don't unroll the BVH walk 8 times).
// Volume objects dim it further (`transmit`).
fn transmit_surface(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> f32 {
    if (!FEAT_OPACITY) { return select(1.0, 0.0, occluded(o, d, tmax)); }
    var at = o;
    var left = tmax;
    var tr = 1.0;
    let hits = u32(u.vol4.w + 0.5);
    for (var i = 0u; i < hits; i++) {
        let h = closest(at, d, left);
        if (h.tri == 0xffffffffu) { return tr; }
        let a = hit_alpha(h, d);
        if (a >= 0.999) { return 0.0; }
        tr *= 1.0 - a;
        if (tr < 1e-3) { return 0.0; }
        let step = h.t + u.background.w;
        at += d * step;
        left -= step;
    }
    return tr;
}

// Light passing along a shadow ray through surfaces and volume objects.
fn transmit(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> f32 {
    let s = transmit_surface(o, d, tmax);
    if (!FEAT_VOLOBJ || s <= 0.0) { return s; }
    return s * vo_transmit(o, d, tmax);
}
