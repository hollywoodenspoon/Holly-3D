// Participating media (PR 3: godrays and height mist, ported from
// src/render/volumetrics.rs). Until then the hook leaves the path as is.

// In-scattered light along a segment of length `t` from `o` along `d`;
// scales `beta` by the segment's transmittance.
fn volume_segment(o: vec3<f32>, d: vec3<f32>, t: f32, beta: ptr<function, vec3<f32>>) -> vec3<f32> {
    if (FEAT_VOLUME) {
        // PR 3: jittered ray-march, Henyey-Greenstein phase, shadow rays.
    }
    return vec3<f32>(0.0);
}
