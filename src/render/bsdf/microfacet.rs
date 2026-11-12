//! Anisotropic GGX microfacet math in a local shading frame (z = normal):
//! distribution, Smith masking, Heitz 2018 visible-normal sampling and the
//! resulting reflection density. With `ax == ay` it reduces exactly to the
//! isotropic lobe the GPU kernel uses.

use bevy::math::Vec3;
use std::f32::consts::PI;

/// Orthonormal shading frame: tangent `t`, bitangent `b`, normal `n`.
#[derive(Clone, Copy, Debug)]
pub struct Frame {
    pub t: Vec3,
    pub b: Vec3,
    pub n: Vec3,
}

impl Frame {
    /// Frame around `n` whose tangent follows world Z wrapped around the
    /// surface (circular brushing on upright cylinders), turned by `rot_deg`.
    pub fn new(n: Vec3, rot_deg: f32) -> Self {
        let up = if n.z.abs() > 0.999 { Vec3::X } else { Vec3::Z };
        let t0 = up.cross(n).normalize();
        let b0 = n.cross(t0);
        let (s, c) = rot_deg.to_radians().sin_cos();
        let t = t0 * c + b0 * s;
        Self { t, b: n.cross(t), n }
    }

    pub fn local(&self, v: Vec3) -> Vec3 {
        Vec3::new(v.dot(self.t), v.dot(self.b), v.dot(self.n))
    }

    pub fn world(&self, v: Vec3) -> Vec3 {
        self.t * v.x + self.b * v.y + self.n * v.z
    }
}

/// Disney mapping of roughness + anisotropy to the two GGX alphas.
pub fn alphas(roughness: f32, anisotropy: f32) -> (f32, f32) {
    let a = (roughness * roughness).max(2e-3);
    let aspect = (1.0 - 0.9 * anisotropy.clamp(0.0, 1.0)).sqrt();
    ((a / aspect).min(1.0), (a * aspect).max(1e-3))
}

/// Anisotropic GGX distribution `D(h)` (h local).
pub fn d_aniso(h: Vec3, ax: f32, ay: f32) -> f32 {
    if h.z <= 0.0 {
        return 0.0;
    }
    let k = (h.x / ax).powi(2) + (h.y / ay).powi(2) + h.z * h.z;
    1.0 / (PI * ax * ay * k * k).max(1e-12)
}

/// Smith `Λ(v)` for anisotropic GGX.
fn lambda(v: Vec3, ax: f32, ay: f32) -> f32 {
    let z2 = (v.z * v.z).max(1e-12);
    let t = ((ax * v.x).powi(2) + (ay * v.y).powi(2)) / z2;
    0.5 * ((1.0 + t).sqrt() - 1.0)
}

/// Smith masking `G1(v)`.
pub fn g1(v: Vec3, ax: f32, ay: f32) -> f32 {
    if v.z <= 0.0 { 0.0 } else { 1.0 / (1.0 + lambda(v, ax, ay)) }
}

/// Density of a VNDF-sampled reflection: `D(h) G1(v) / (4 v.z)`.
pub fn pdf(v: Vec3, h: Vec3, ax: f32, ay: f32) -> f32 {
    d_aniso(h, ax, ay) * g1(v, ax, ay) / (4.0 * v.z).max(1e-12)
}

/// `F D G / (4 nv)` without Fresnel: the reflection's `f cos` per unit F.
pub fn spec(v: Vec3, l: Vec3, ax: f32, ay: f32) -> f32 {
    let h = (v + l).normalize_or_zero();
    d_aniso(h, ax, ay) * g1(v, ax, ay) * g1(l, ax, ay) / (4.0 * v.z).max(1e-12)
}

/// Heitz 2018 visible-normal sampling (local frame).
pub fn sample_vndf(v: Vec3, ax: f32, ay: f32, u1: f32, u2: f32) -> Vec3 {
    let vh = Vec3::new(ax * v.x, ay * v.y, v.z).normalize();
    let len2 = vh.x * vh.x + vh.y * vh.y;
    let t1 = if len2 > 0.0 { Vec3::new(-vh.y, vh.x, 0.0) / len2.sqrt() } else { Vec3::X };
    let t2 = vh.cross(t1);
    let r = u1.sqrt();
    let phi = 2.0 * PI * u2;
    let p1 = r * phi.cos();
    let s = 0.5 * (1.0 + vh.z);
    let p2 = (1.0 - s) * (1.0 - p1 * p1).max(0.0).sqrt() + s * r * phi.sin();
    let nh = t1 * p1 + t2 * p2 + vh * (1.0 - p1 * p1 - p2 * p2).max(0.0).sqrt();
    Vec3::new(ax * nh.x, ay * nh.y, nh.z.max(0.0)).normalize()
}

/// Isotropic GGX `D(h)` from `n·h` and `alpha²`.
pub fn distribution(nh: f32, a2: f32) -> f32 {
    let d = nh * nh * (a2 - 1.0) + 1.0;
    a2 / (PI * d * d).max(1e-12)
}

/// Isotropic Smith masking from `n·x` and `alpha²`.
pub fn smith_g1(nx: f32, a2: f32) -> f32 {
    2.0 * nx / (nx + (a2 + (1.0 - a2) * nx * nx).sqrt()).max(1e-12)
}

/// Isotropic VNDF sampling around a world normal `n` (tests, GPU parity).
pub fn sample_vndf_world(n: Vec3, v: Vec3, alpha: f32, u1: f32, u2: f32) -> Vec3 {
    let f = Frame::new(n, 0.0);
    f.world(sample_vndf(f.local(v), alpha, alpha, u1, u2)).normalize()
}

/// Schlick's weight `(1 - c)^5`.
pub fn schlick_w(c: f32) -> f32 {
    (1.0 - c.clamp(0.0, 1.0)).powi(5)
}
