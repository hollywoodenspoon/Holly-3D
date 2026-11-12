//! Flattens a [`RenderScene`] into the GPU path tracer's storage buffers:
//! triangles (positions, normals, per-vertex surface color, material, emission),
//! binary BVH nodes, punctual lights and the emitter CDF. Also decides
//! whether a scene needs features only the CPU tracer has.

use crate::materials::ShadingModel;
use crate::materials::proc_engine::Pattern;
use crate::materials::shading::{lin3, surface};
use crate::render::scene_build::{RShape, RenderScene};
use bevy::math::{Vec3, Vec4};

/// Little-endian byte buffer of 16-byte aligned vec4 records.
#[derive(Default)]
pub struct Buf(pub Vec<u8>);

impl Buf {
    pub fn v4(&mut self, v: Vec4) {
        for f in v.to_array() {
            self.0.extend_from_slice(&f.to_le_bytes());
        }
    }
    pub fn v3(&mut self, v: Vec3, w: f32) {
        self.v4(v.extend(w));
    }
    /// A vec4 whose `w` carries a bit-cast `u32`.
    pub fn v3u(&mut self, v: Vec3, w: u32) {
        self.v4(v.extend(f32::from_bits(w)));
    }
    /// Storage bindings must not be empty.
    pub fn or_dummy(mut self, record: usize) -> Vec<u8> {
        if self.0.is_empty() {
            self.0.resize(record, 0);
        }
        self.0
    }
}

/// Why this scene renders on the CPU instead (`None` = the GPU handles it).
pub fn unsupported(rs: &RenderScene) -> Option<&'static str> {
    if rs.hdri.is_some() || rs.procedural.is_some() {
        return Some("HDRI / procedural sky environment");
    }
    if rs.volume.is_some() {
        return Some("volumetric godrays / mist");
    }
    if rs.lights.iter().any(|l| matches!(l.shape, RShape::Area(_))) {
        return Some("area lights");
    }
    if rs.lights.iter().any(|l| l.ies != crate::render::IesProfile::None) {
        return Some("IES light profiles");
    }
    if rs.translucent {
        return Some("see-through (opacity) surfaces");
    }
    for m in &rs.materials {
        if m.model == ShadingModel::Toon {
            return Some("cel / toon materials");
        }
        let mapped = m.maps.iter().any(|s| s.enabled && s.node.is_some()) || !m.slate_nodes.is_empty();
        if mapped || m.proc.pattern != Pattern::None || m.paint.is_some() || m.projection.on() {
            return Some("texture / procedural maps");
        }
        if m.principled.active() {
            return Some("Principled SSS / clearcoat / sheen / anisotropy / dispersion");
        }
        if m.decoupled() {
            return Some("Preserve Texture Appearance materials");
        }
    }
    None
}

pub struct Packed {
    pub tris: Vec<u8>,
    pub nodes: Vec<u8>,
    pub lights: Vec<u8>,
    pub emitters: Vec<u8>,
    pub n_lights: u32,
    pub n_emitters: u32,
}

pub fn pack(rs: &RenderScene) -> Packed {
    let mut tris = Buf::default();
    for (i, t) in rs.bvh.tris.iter().enumerate() {
        let m = &rs.materials[t.mat];
        let surf = |k: usize| surface(m, t.uv[k], t.p[k], t.col[k]);
        let s = [surf(0), surf(1), surf(2)];
        t.p.iter().for_each(|p| tris.v3(*p, 0.0));
        t.n.iter().for_each(|n| tris.v3(*n, 0.0));
        s.iter().for_each(|s| tris.v3(s.base, s.alpha));
        let s0 = s[0];
        tris.v4(Vec4::new(s0.metallic, s0.roughness, s0.transmission, s0.ior));
        let model = match m.model {
            ShadingModel::Unlit => 1.0,
            ShadingModel::Crystal => 2.0,
            _ => 0.0,
        };
        tris.v3(s0.emissive, model);
        tris.v3(lin3(m.attenuation_color), m.thickness);
        let (prob, area) = rs.emitters.lookup(i).map_or((0.0, 0.0), |(e, p)| (p, e.area));
        tris.v4(Vec4::new(prob, area, 0.0, 0.0));
    }
    let mut nodes = Buf::default();
    for n in rs.bvh.nodes() {
        nodes.v3u(n.lo, n.start);
        nodes.v3u(n.hi, n.count);
    }
    let mut lights = Buf::default();
    for l in &rs.lights {
        let (kind, axis, cin, cout) = match l.shape {
            RShape::Point => (0.0, Vec3::Z, 1.0, 0.0),
            RShape::Spot { dir, cos_in, cos_out } => (1.0, dir, cos_in, cos_out),
            RShape::Directional { dir } => (2.0, dir, 1.0, 0.0),
            // Never packed: `unsupported` sends area lights to the CPU.
            RShape::Area(_) => (0.0, Vec3::Z, 1.0, 0.0),
        };
        lights.v3(l.pos, kind);
        lights.v3(axis, cin);
        lights.v3(l.radiance, cout);
        let (na, nb) = l.atten.near.unwrap_or((-1.0, -1.0));
        let (fa, fb) = l.atten.far.unwrap_or((-1.0, -1.0));
        lights.v4(Vec4::new(na, nb, fa, fb));
        lights.v4(Vec4::new(l.range, f32::from(u8::from(l.shadows)), l.softness, 0.0));
        lights.v3(l.shadow_tint, 0.0);
    }
    let mut emitters = Buf::default();
    let mut cdf = 0.0;
    for e in &rs.emitters.list {
        let p = rs.emitters.prob_of(e.tri);
        cdf += p;
        emitters.v4(Vec4::new(f32::from_bits(e.tri as u32), p, e.area, cdf));
    }
    Packed {
        tris: tris.or_dummy(13 * 16),
        nodes: nodes.or_dummy(32),
        lights: lights.or_dummy(6 * 16),
        emitters: emitters.or_dummy(16),
        n_lights: rs.lights.len() as u32,
        n_emitters: rs.emitters.list.len() as u32,
    }
}
