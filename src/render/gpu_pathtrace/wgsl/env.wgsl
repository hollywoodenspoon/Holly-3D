// Environment seen by paths that leave the scene (matches Tracer::environment).

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

// Bilinear lookup in the equirect image (matches HdrImage::sample).
fn env_sample(dir: vec3<f32>) -> vec3<f32> {
    let w = u.env.x;
    let h = u.env.y;
    let d = normalize(dir);
    let uu = fract(0.5 - atan2(d.y, d.x) / (2.0 * PI) + u.env.z);
    let vv = acos(clamp(d.z, -1.0, 1.0)) / PI;
    let fx = uu * w - 0.5;
    let fy = clamp(vv * h - 0.5, 0.0, h - 1.0);
    let x0 = floor(fx);
    let y0 = floor(fy);
    let tx = fx - x0;
    let ty = fy - y0;
    let top = mix(env_at(x0, y0), env_at(x0 + 1.0, y0), tx);
    let bot = mix(env_at(x0, y0 + 1.0), env_at(x0 + 1.0, y0 + 1.0), tx);
    return mix(top, bot, ty);
}

fn env_at(x: f32, y: f32) -> vec3<f32> {
    let w = i32(u.env.x);
    let xi = u32(((i32(x) % w) + w) % w);
    let yi = min(u32(max(y, 0.0)), u32(u.env.y) - 1u);
    return envmap[yi * u32(w) + xi].xyz;
}

fn environment(d: vec3<f32>) -> vec3<f32> {
    if (FEAT_ENV_MAP) {
        // The baked sky / HDRI replaces the flat colours and the softbox.
        return env_sample(d) * u.env.w + u.sky.xyz;
    }
    let up = clamp(0.5 + 0.5 * d.z, 0.0, 1.0);
    var c = u.background.xyz + u.sky.xyz + u.ambient.xyz * (1.0 + 5.0 * up * up);
    if ((u.counts.w & F_SOFTBOX) != 0u) {
        let key = normalize(vec3<f32>(0.25, -0.35, 1.0));
        let kt = vec3<f32>(1.0, 0.0, 0.0);
        let s = panel(d, key, kt, normalize(cross(key, kt)), 0.55, 0.4, 3.0) + strip(d, 1.0) + strip(d, -1.0);
        c += vec3<f32>(1.0, 0.98, 0.95) * s;
    }
    return c;
}
