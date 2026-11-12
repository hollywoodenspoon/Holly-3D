//! Volumetric godrays (crepuscular light shafts) and 3D height mist: a
//! heterogeneous participating medium raymarched along every path segment
//! with jittered steps, Henyey-Greenstein phase scattering and shadow rays
//! against the BVH for each light, so beams carve through gaps in geometry.
//! Settings live in the Environment (`8`) dialog.

use super::scene_build::{RLight, RShape};
use super::trace::{Rng, Tracer};
use bevy::math::Vec3;
use std::f32::consts::PI;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct VolumeSettings {
    /// `[✓] Enable Volumetric Godrays`.
    pub enabled: bool,
    /// Godray Intensity (0..10): scales the in-scattered light.
    pub intensity: f32,
    /// Scattering Anisotropy `g` (-0.9..0.95): > 0 scatters forward.
    pub anisotropy: f32,
    /// Volumetric Steps (16..128) per path segment.
    pub steps: u32,
    /// Fog Scatter Color (sRGB).
    pub color: [f32; 3],
    /// Extinction per world unit at the mist base.
    pub density: f32,
    /// Mist Base Height: full density at and below this Z.
    pub base_height: f32,
    /// Height Falloff: density halves every `ln 2 / falloff` units above the
    /// base (0 = uniform haze).
    pub falloff: f32,
    /// 3D Noise Wisps: Density (0 = smooth mist, 1 = fully patchy).
    pub wisps: f32,
    /// Wisp Scale (world units per noise cell).
    pub wisp_scale: f32,
    /// Wind Drift: XY offset of the wisp noise (world units).
    pub wind: [f32; 2],
}

impl Default for VolumeSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            intensity: 1.0,
            anisotropy: 0.6,
            steps: 48,
            color: [1.0, 0.97, 0.92],
            density: 0.04,
            base_height: 0.0,
            falloff: 0.0,
            wisps: 0.0,
            wisp_scale: 4.0,
            wind: [0.0, 0.0],
        }
    }
}

/// Henyey-Greenstein phase function: density of scattering by angle
/// `acos(cos)` between the incoming propagation and outgoing direction.
pub fn hg_phase(cos: f32, g: f32) -> f32 {
    let g = g.clamp(-0.99, 0.99);
    let d = 1.0 + g * g - 2.0 * g * cos;
    (1.0 - g * g) / (4.0 * PI * d * d.sqrt()).max(1e-12)
}

/// Samples an outgoing direction around the propagation direction `w`
/// (importance sampling of `hg_phase`).
pub fn sample_hg(w: Vec3, g: f32, u1: f32, u2: f32) -> Vec3 {
    let g = g.clamp(-0.99, 0.99);
    let cos = if g.abs() < 1e-3 {
        1.0 - 2.0 * u1
    } else {
        let s = (1.0 - g * g) / (1.0 + g - 2.0 * g * u1);
        (1.0 + g * g - s * s) / (2.0 * g)
    }
    .clamp(-1.0, 1.0);
    let sin = (1.0 - cos * cos).max(0.0).sqrt();
    let phi = 2.0 * PI * u2;
    let (t, b) = w.any_orthonormal_pair();
    (t * (sin * phi.cos()) + b * (sin * phi.sin()) + w * cos).normalize()
}

/// The medium in render space (linear color, clamped parameters).
#[derive(Clone, Debug)]
pub struct Medium {
    pub albedo: Vec3,
    pub density: f32,
    pub base: f32,
    pub falloff: f32,
    pub wisps: f32,
    pub inv_scale: f32,
    pub wind: Vec3,
    pub g: f32,
    pub steps: u32,
    pub intensity: f32,
}

impl Medium {
    pub fn new(v: &VolumeSettings) -> Option<Self> {
        (v.enabled && v.density > 0.0).then(|| Self {
            albedo: crate::materials::shading::lin3(v.color),
            density: v.density.max(0.0),
            base: v.base_height,
            falloff: v.falloff.max(0.0),
            wisps: v.wisps.clamp(0.0, 1.0),
            inv_scale: 1.0 / v.wisp_scale.max(0.01),
            wind: Vec3::new(v.wind[0], v.wind[1], 0.0),
            g: v.anisotropy.clamp(-0.9, 0.95),
            steps: v.steps.clamp(16, 128),
            intensity: v.intensity.clamp(0.0, 10.0),
        })
    }

    /// Extinction coefficient at `p`: height falloff above the base times
    /// patchy FBM wisps.
    pub fn sigma(&self, p: Vec3) -> f32 {
        let h = (p.z - self.base).max(0.0);
        let mut s = self.density * (-self.falloff * h).exp();
        if self.wisps > 0.0 {
            let n = super::sky_noise::fbm((p + self.wind) * self.inv_scale, 3);
            s *= 1.0 - self.wisps + self.wisps * (n * 2.0).clamp(0.0, 2.0);
        }
        s
    }
}

/// Direction to the light, distance and the falloff factor (distance, spot
/// cone, attenuation) of light `l` arriving at `p`; `None` outside it.
/// Direction to, distance to and intensity factor of light `l` at `p`;
/// area lights draw one point on their surface per call.
pub(super) fn incident(l: &RLight, p: Vec3, rng: &mut Rng) -> Option<(Vec3, f32, f32)> {
    let window = |d: f32| (1.0 - (d / l.range.max(1e-3)).powi(4)).clamp(0.0, 1.0).powi(2) * l.atten.factor(d);
    let (dir, dist, mut k) = match l.shape {
        RShape::Directional { dir } => (-dir, f32::INFINITY, 1.0),
        RShape::Point | RShape::Spot { .. } => {
            let v = l.pos - p;
            let d = v.length().max(1e-4);
            (v / d, d, window(d) / (d * d))
        }
        RShape::Area(a) => {
            let (q, nq, scale) = a.sample(l.pos, p, rng);
            let v = q - p;
            let d = v.length().max(1e-4);
            let cos_l = nq.dot(-v / d);
            if cos_l <= 0.0 {
                return None;
            }
            (v / d, d, scale * cos_l * window(l.pos.distance(p)) / (d * d).max(1e-4))
        }
    };
    let axis = match l.shape {
        RShape::Spot { dir: axis, cos_in, cos_out } => {
            let t = (((-dir).dot(axis) - cos_out) / (cos_in - cos_out).max(1e-4)).clamp(0.0, 1.0);
            k *= t * t * (3.0 - 2.0 * t);
            axis
        }
        RShape::Area(a) => a.n,
        _ => Vec3::ZERO,
    };
    if l.ies != super::area_lights::IesProfile::None && axis != Vec3::ZERO {
        k *= l.ies.factor((-dir).dot(axis));
    }
    (k > 0.0).then_some((dir, dist, k))
}

/// Result of marching one segment: light scattered toward the viewer and
/// the transmittance of the segment.
pub struct March {
    pub inscatter: Vec3,
    pub transmittance: f32,
}

impl Tracer<'_> {
    /// Marches the medium from `o` along `d` for `tmax` (misses: up to the
    /// far side of the scene), with one jittered sample per step.
    pub(super) fn march(&self, m: &Medium, o: Vec3, d: Vec3, tmax: f32, rng: &mut Rng) -> March {
        let len = tmax.min(self.rs.radius * 6.0 + o.distance(self.rs.camera.eye()));
        let n = m.steps;
        let dt = len / n as f32;
        let (mut tr, mut inscatter) = (1.0f32, Vec3::ZERO);
        let jitter = rng.f();
        for i in 0..n {
            let p = o + d * ((i as f32 + jitter) * dt);
            let sigma = m.sigma(p);
            if sigma <= 0.0 {
                continue;
            }
            let step_tr = (-sigma * dt).exp();
            // Energy-conserving single scattering over the step.
            let weight = tr * (1.0 - step_tr);
            let mut li = Vec3::ZERO;
            for l in &self.rs.lights {
                let Some((dir, dist, k)) = incident(l, p, rng) else { continue };
                let vis = if self.cfg.shadows && l.shadows { self.transmittance(p, dir, (dist - self.eps).min(1e30)) } else { 1.0 };
                if vis > 0.0 {
                    li += l.radiance * (k * vis * hg_phase(d.dot(dir), m.g));
                }
            }
            inscatter += li * m.albedo * (weight * m.intensity);
            tr *= step_tr;
            if tr < 1e-4 {
                tr = 0.0;
                break;
            }
        }
        March { inscatter, transmittance: tr }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hg_integrates_to_one_and_mean_cosine_is_g() {
        for g in [-0.5, 0.0, 0.3, 0.8, 0.95] {
            let n = 20_000;
            let (mut total, mut mean) = (0.0, 0.0);
            for i in 0..n {
                let c = -1.0 + 2.0 * (i as f32 + 0.5) / n as f32;
                let p = hg_phase(c, g) * 2.0 * PI * (2.0 / n as f32);
                total += p;
                mean += p * c;
            }
            assert!((total - 1.0).abs() < 2e-2, "g {g}: integral {total}");
            assert!((mean - g).abs() < 2e-2, "g {g}: mean cosine {mean}");
        }
    }

    #[test]
    fn hg_sampling_matches_the_phase_function() {
        let mut rng = Rng::new(7);
        let w = Vec3::new(0.3, -0.2, 0.93).normalize();
        for g in [0.0, 0.6, -0.4] {
            let n = 40_000;
            let mean: f32 = (0..n).map(|_| sample_hg(w, g, rng.f(), rng.f()).dot(w)).sum::<f32>() / n as f32;
            assert!((mean - g).abs() < 0.02, "g {g}: sampled mean cosine {mean}");
        }
    }

    #[test]
    fn mist_thins_with_height_and_wisps_vary() {
        let v = VolumeSettings { enabled: true, density: 0.5, falloff: 0.7, ..Default::default() };
        let m = Medium::new(&v).unwrap();
        assert!((m.sigma(Vec3::new(0.0, 0.0, -3.0)) - 0.5).abs() < 1e-6, "full density below the base");
        assert!((m.sigma(Vec3::new(0.0, 0.0, 1.0)) - 0.5 * (-0.7f32).exp()).abs() < 1e-5);
        let w = Medium::new(&VolumeSettings { wisps: 1.0, wisp_scale: 1.0, ..v.clone() }).unwrap();
        let s: Vec<f32> = (0..64).map(|i| w.sigma(Vec3::new(i as f32 * 0.37, 0.0, 0.0))).collect();
        let (lo, hi) = s.iter().fold((f32::MAX, 0.0f32), |(a, b), &x| (a.min(x), b.max(x)));
        assert!(hi - lo > 0.2, "wisps make the mist patchy: {lo}..{hi}");
        assert!(Medium::new(&VolumeSettings::default()).is_none(), "off by default");
    }
}

#[cfg(test)]
mod shaft_tests {
    use super::*;
    use crate::primitives::{Primitive, PrimitiveKind};
    use crate::render::scene_build::RenderScene;
    use crate::render::{Light, LightKind, RenderSettings};
    use crate::scene::{BaseObject, Scene};
    use crate::viewport::camera::{ViewCamera, ViewKind};

    /// A sun shining straight down past a wide box: rays under the box sit
    /// in its shadow, rays beside it cross the lit shaft.
    #[test]
    fn godrays_are_occluded_by_geometry() {
        let mut s = Scene::default();
        let id = s.add(BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Box)), Vec3::new(0.0, 0.0, 3.0));
        s.objects[id].evaluate();
        s.add_light(Light::new(LightKind::FreeDirect), None, Vec3::new(0.0, 0.0, 20.0), Vec3::ZERO);
        s.environment.volume = VolumeSettings { enabled: true, density: 0.2, anisotropy: 0.0, steps: 64, ..Default::default() };
        let rs = RenderScene::build(&s, ViewCamera::new(ViewKind::Perspective), false);
        let cfg = RenderSettings::default();
        let t = Tracer::new(&rs, &cfg);
        let m = rs.volume.as_ref().unwrap();
        let mut rng = Rng::new(3);
        // Short horizontal segments at z = 0, under the box and far beside it.
        let shade = t.march(m, Vec3::new(-0.2, -0.2, 0.0), Vec3::Y, 0.4, &mut rng).inscatter.length();
        let lit = t.march(m, Vec3::new(8.0, -3.0, 0.0), Vec3::Y, 0.4, &mut rng).inscatter.length();
        assert!(lit > 1e-4, "lit air scatters light: {lit}");
        assert!(shade < lit * 0.05, "shadow shaft under the box: {shade} vs {lit}");
        let v = t.march(m, Vec3::new(8.0, -3.0, 0.0), Vec3::Y, 10.0, &mut rng);
        assert!((v.transmittance - (-0.2f32 * 10.0).exp()).abs() < 1e-3, "Beer-Lambert over the segment: {}", v.transmittance);
    }

    #[test]
    fn forward_scattering_brightens_toward_the_light() {
        let mut s = Scene::default();
        s.add_light(Light::new(LightKind::FreeDirect), None, Vec3::new(0.0, 0.0, 20.0), Vec3::ZERO);
        s.environment.volume = VolumeSettings { enabled: true, density: 0.1, anisotropy: 0.8, ..Default::default() };
        let rs = RenderScene::build(&s, ViewCamera::new(ViewKind::Perspective), false);
        let cfg = RenderSettings::default();
        let t = Tracer::new(&rs, &cfg);
        let m = rs.volume.as_ref().unwrap();
        let mut rng = Rng::new(1);
        let up = t.march(m, Vec3::ZERO, Vec3::Z, 2.0, &mut rng).inscatter.length();
        let side = t.march(m, Vec3::ZERO, Vec3::X, 2.0, &mut rng).inscatter.length();
        assert!(up > side * 5.0, "looking into the sun shows the bright forward lobe: {up} vs {side}");
    }
}
