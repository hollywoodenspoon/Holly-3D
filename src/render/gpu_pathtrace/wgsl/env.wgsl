// Environment without an HDRI (matches Tracer::environment).

fn panel(d: vec3<f32>, axis: vec3<f32>, t: vec3<f32>, b: vec3<f32>, hx: f32, hy: f32, p: f32) -> f32 {
    let k = dot(d, axis);
    if (k <= 0.05) { return 0.0; }
    let x = abs(dot(d, t) / k) / hx;
    let y = abs(dot(d, b) / k) / hy;
    return p * clamp((1.0 - max(x, y)) / 0.12, 0.0, 1.0);
}

fn strip(d: vec3<f32>, side: f32) -> f32 {
    let axis = normalize(vec3<f32>(side, 0.3, 0.25));
    let t = normalize(cross(vec3<f32>(0.0, 0.0, 1.0), axis));
    return panel(d, axis, t, normalize(cross(axis, t)), 0.18, 0.75, 1.6);
}

fn environment(d: vec3<f32>) -> vec3<f32> {
    let up = clamp(0.5 + 0.5 * d.z, 0.0, 1.0);
    var c = u.background.xyz + u.sky.xyz + u.ambient.xyz * (1.0 + 5.0 * up * up);
    if (FEAT_ENV_MAP) {
        // PR 1: the baked sky / HDRI equirect image replaces the flat colours.
    }
    if ((u.counts.w & F_SOFTBOX) != 0u) {
        let key = normalize(vec3<f32>(0.25, -0.35, 1.0));
        let kt = vec3<f32>(1.0, 0.0, 0.0);
        let s = panel(d, key, kt, normalize(cross(key, kt)), 0.55, 0.4, 3.0) + strip(d, 1.0) + strip(d, -1.0);
        c += vec3<f32>(1.0, 0.98, 0.95) * s;
    }
    return c;
}
