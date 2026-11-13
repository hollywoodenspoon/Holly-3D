//! Analytic area lights (Rect softbox, Disc, Sphere, Tube / Neon bar) and
//! built-in IES photometric profiles. Each light sample picks a point on
//! the facing part of the shape, so shadows soften with the light's size
//! and highlights take its shape. A multiplier-1 area light gives the same
//! on-axis power as a multiplier-1 Omni, so swapping types keeps the
//! exposure. `Visible to Camera` shows the emitting surface in F9.

use super::lighting::LightKind;
use super::scene_build::RShape;
use super::trace::{Rng, Shading, Tracer};
use crate::materials::shading::LightIn;
use bevy::math::{Quat, Vec3};
use std::f32::consts::{PI, TAU};

/// Real-world luminaire angular distributions (analytic IES webs).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum IesProfile {
    #[default]
    None,
    WallScallop,
    StreetlampWide,
    TacticalFlashlight,
    HighBay,
    StageNarrow,
}

fn gauss(x: f32, s: f32) -> f32 {
    (-x * x / (2.0 * s * s)).exp()
}

/// 1 inside `a` degrees, smoothly 0 by `b` degrees.
fn cut(th: f32, a: f32, b: f32) -> f32 {
    let t = ((b - th) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl IesProfile {
    pub const ALL: [Self; 6] = [Self::None, Self::WallScallop, Self::StreetlampWide, Self::TacticalFlashlight, Self::HighBay, Self::StageNarrow];

    pub fn label(self) -> &'static str {
        match self {
            Self::None => "None (Uniform)",
            Self::WallScallop => "Architectural Wall Scallop",
            Self::StreetlampWide => "Streetlamp Wide Throw",
            Self::TacticalFlashlight => "Double-Ring Tactical Flashlight",
            Self::HighBay => "Industrial High-Bay",
            Self::StageNarrow => "Stage Narrow Beam",
        }
    }

    /// Relative intensity (peak 1) at `cos` of the angle from the light's
    /// emission axis.
    pub fn factor(self, cos: f32) -> f32 {
        let th = cos.clamp(-1.0, 1.0).acos().to_degrees();
        let f = match self {
            Self::None => return 1.0,
            // Downlight grazing a wall: bright arc at ~30°, hard cut at 60°.
            Self::WallScallop => (0.35 + 0.65 * gauss(th - 30.0, 11.0)) * cut(th, 52.0, 64.0),
            // Batwing: most light thrown sideways down the street.
            Self::StreetlampWide => (0.25 + 0.75 * gauss(th - 62.0, 11.0)) * cut(th, 74.0, 86.0),
            // Hot center plus two concentric reflector rings.
            Self::TacticalFlashlight => {
                (gauss(th, 3.5) + 0.45 * gauss(th - 11.0, 2.0) + 0.25 * gauss(th - 21.0, 2.5)).min(1.0) * cut(th, 26.0, 34.0)
            }
            Self::HighBay => cos.max(0.0).powf(1.3) * cut(th, 62.0, 75.0),
            Self::StageNarrow => cos.max(0.0).powi(80) * cut(th, 12.0, 16.0),
        };
        f.clamp(0.0, 1.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AreaShape {
    Rect,
    Disc,
    Sphere,
    Tube,
}

impl AreaShape {
    pub fn of(kind: LightKind) -> Option<Self> {
        match kind {
            LightKind::RectArea => Some(Self::Rect),
            LightKind::DiscArea => Some(Self::Disc),
            LightKind::SphereArea => Some(Self::Sphere),
            LightKind::TubeArea => Some(Self::Tube),
            _ => None,
        }
    }
}

/// An area light in world space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Area {
    pub shape: AreaShape,
    /// Emission axis (local -Z) and the local X / Y axes.
    pub n: Vec3,
    pub u: Vec3,
    pub v: Vec3,
    /// Half width / half length (Rect, Tube length along `u`) or radius.
    pub hw: f32,
    pub hl: f32,
    pub visible: bool,
}

impl Area {
    /// `size`: Rect width × length; Disc / Sphere radius; Tube radius × length.
    pub fn new(shape: AreaShape, rot: Quat, size: [f32; 2], visible: bool) -> Self {
        let (a, b) = (size[0].max(1e-3), size[1].max(1e-3));
        let (hw, hl) = match shape {
            AreaShape::Rect => (a * 0.5, b * 0.5),
            AreaShape::Tube => (a, b * 0.5),
            AreaShape::Disc | AreaShape::Sphere => (a, a),
        };
        Self { shape, n: rot * Vec3::NEG_Z, u: rot * Vec3::X, v: rot * Vec3::Y, hw, hl, visible }
    }

    /// Area seen head-on: radiance = on-axis intensity / this.
    pub fn projected_area(&self) -> f32 {
        match self.shape {
            AreaShape::Rect => 4.0 * self.hw * self.hl,
            AreaShape::Disc | AreaShape::Sphere => PI * self.hw * self.hw,
            AreaShape::Tube => 4.0 * self.hw * self.hl,
        }
    }

    /// A point on the part of the light facing `p`, its outward normal and
    /// the estimator scale `A Le / I0` (so `k = scale cos_l / d²`).
    pub fn sample(&self, c: Vec3, p: Vec3, rng: &mut Rng) -> (Vec3, Vec3, f32) {
        let (a, b) = (rng.f(), rng.f());
        match self.shape {
            AreaShape::Rect => (c + self.u * self.hw * (2.0 * a - 1.0) + self.v * self.hl * (2.0 * b - 1.0), self.n, 1.0),
            AreaShape::Disc => {
                let (r, phi) = (self.hw * a.sqrt(), TAU * b);
                (c + (self.u * phi.cos() + self.v * phi.sin()) * r, self.n, 1.0)
            }
            AreaShape::Sphere => {
                // Uniform on the hemisphere facing p: A/2 Le / I0 = 2.
                let z = 1.0 - 2.0 * a;
                let s = (1.0 - z * z).max(0.0).sqrt();
                let mut d = Vec3::new(s * (TAU * b).cos(), s * (TAU * b).sin(), z);
                if d.dot(p - c) < 0.0 {
                    d = -d;
                }
                (c + d * self.hw, d, 2.0)
            }
            AreaShape::Tube => {
                // Half cylinder facing p: (π r L) / (2 r L) = π / 2.
                let w = self.n.cross(self.u);
                let phi = TAU * b;
                let mut d = self.n * phi.cos() + w * phi.sin();
                if d.dot(p - c) < 0.0 {
                    d = -d;
                }
                (c + self.u * self.hl * (2.0 * a - 1.0) + d * self.hw, d, PI * 0.5)
            }
        }
    }

    /// Distance along the ray `o + t d` to the emitting surface.
    pub fn hit(&self, c: Vec3, o: Vec3, d: Vec3) -> Option<f32> {
        let rel = o - c;
        let t = match self.shape {
            AreaShape::Rect | AreaShape::Disc => {
                let dn = d.dot(self.n);
                if dn >= -1e-6 {
                    return None; // only the emitting face is lit
                }
                let t = -rel.dot(self.n) / dn;
                let q = rel + d * t;
                let (x, y) = (q.dot(self.u), q.dot(self.v));
                let inside = if self.shape == AreaShape::Rect { x.abs() <= self.hw && y.abs() <= self.hl } else { x * x + y * y <= self.hw * self.hw };
                inside.then_some(t)?
            }
            AreaShape::Sphere => {
                let (b, cc) = (rel.dot(d), rel.length_squared() - self.hw * self.hw);
                let disc = b * b - cc;
                (disc >= 0.0).then(|| -b - disc.sqrt())?
            }
            AreaShape::Tube => {
                // Infinite cylinder around `u`, clipped to the length.
                let (dp, rp) = (d - self.u * d.dot(self.u), rel - self.u * rel.dot(self.u));
                let (a, b, cc) = (dp.length_squared(), dp.dot(rp), rp.length_squared() - self.hw * self.hw);
                let disc = b * b - a * cc;
                if a < 1e-9 || disc < 0.0 {
                    return None;
                }
                let t = (-b - disc.sqrt()) / a;
                ((rel + d * t).dot(self.u).abs() <= self.hl).then_some(t)?
            }
        };
        (t > 1e-4).then_some(t)
    }
}

impl Tracer<'_> {
    /// Calls `f(light group, light)` for each shadowed light sample reaching
    /// `sh` (area lights take `shadow_samples` points on their surface).
    pub(super) fn punctual_each(&self, sh: &Shading, rng: &mut Rng, mut f: impl FnMut(u8, LightIn)) {
        let o = sh.p + sh.ng * self.eps;
        for l in &self.rs.lights {
            let n = if matches!(l.shape, RShape::Area(_)) { self.cfg.shadow_samples.max(1) } else { 1 };
            for _ in 0..n {
                let Some((dir, dist, k)) = super::volumetrics::incident(l, sh.p, rng) else { continue };
                if sh.n.dot(dir) <= 0.0 {
                    continue;
                }
                let mut radiance = l.radiance * k / n as f32;
                if self.cfg.shadows && l.shadows {
                    let vis = self.visibility(o, dir, dist, l.softness, rng);
                    radiance *= l.shadow_tint + (Vec3::ONE - l.shadow_tint) * vis;
                }
                f(l.group, LightIn { dir, radiance });
            }
        }
    }

    /// Radiance and LightMix group of the nearest camera-visible area light
    /// closer than `tmax`.
    pub(crate) fn area_seen(&self, o: Vec3, d: Vec3, tmax: f32) -> Option<(Vec3, u8)> {
        let mut best: Option<(f32, (Vec3, u8))> = None;
        for l in &self.rs.lights {
            let RShape::Area(a) = l.shape else { continue };
            if !a.visible {
                continue;
            }
            if let Some(t) = a.hit(l.pos, o, d).filter(|t| *t < tmax && best.is_none_or(|(b, _)| *t < b)) {
                best = Some((t, (l.radiance / a.projected_area(), l.group)));
            }
        }
        best.map(|(_, le)| le)
    }
}
