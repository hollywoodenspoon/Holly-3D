// Shadow & Reflection Catcher (matches pathtrace::catcher). A catcher seen
// by the camera is see-through except for the shadows the lights cast on
// it, its contact AO (`u.ambient.w` reach, `u.sky.w` strength) and what it
// reflects. The reflection is traced by the integrator as a nested camera
// path weighted by `catch_fresnel`; its coverage adds to the alpha.

const LUM: vec3<f32> = vec3<f32>(0.2126, 0.7152, 0.0722);

fn catcher_alpha(shadow: f32, occl: f32, refl: f32) -> f32 {
    let dark = 1.0 - (1.0 - shadow) * (1.0 - occl);
    return clamp(dark + refl * (1.0 - dark), 0.0, 1.0);
}

// (shadow fraction, AO occlusion) at a catcher point.
fn catch_dark(p: vec3<f32>, ng: vec3<f32>, n: vec3<f32>) -> vec2<f32> {
    let o = p + ng * u.background.w;
    var open = 0.0;
    var lit = 0.0;
    for (var i = 0u; i < u.counts.x; i++) {
        let l = lights[i];
        let inc = incident(l, p);
        if (inc.k <= 0.0) { continue; }
        let e = dot(l.rad.xyz, LUM) * inc.k * max(dot(n, inc.dir), 0.0);
        open += e;
        var vis = 1.0;
        if ((u.counts.w & F_SHADOWS) != 0u && l.misc.y > 0.5) { vis = visibility(l, p, o, inc.dir, inc.dist); }
        lit += e * vis;
    }
    let shadow = select(clamp(1.0 - lit / open, 0.0, 1.0), 0.0, open <= 1e-8);
    let ao = transmit(o, cosine_dir(n), max(u.ambient.w, 1e-3));
    return vec2<f32>(shadow, (1.0 - ao) * u.sky.w);
}

// Weight of the catcher's mirror reflection (Schlick at 4 %, faded by roughness).
fn catch_fresnel(n: vec3<f32>, d: vec3<f32>, rough: f32) -> f32 {
    let c = clamp(dot(n, -d), 0.0, 1.0);
    return (0.04 + 0.96 * pow(1.0 - c, 5.0)) * (1.0 - rough) * (1.0 - rough);
}
