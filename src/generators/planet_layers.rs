//! Planet Studio outer layers: the Weather & Cloud Shell (Coriolis cyclone
//! spirals and latitude weather bands in the vertex alpha), the Atmosphere
//! Rim Halo (nested see-through shells that stack into a Fresnel-like limb
//! glow, with an orange sunset band along the terminator) and Saturn-style
//! Planetary Rings (radial band noise in the vertex alpha).

use super::sphere::{annulus, equal_angle_sphere};
use super::{hash, rgba, smooth};
use crate::mesh::EditablePoly;
use bevy::math::{Mat4, Quat, Vec3};
use noise::{Fbm, MultiFractal, NoiseFn, Perlin};

pub const ID_CLOUDS: u16 = 5;
pub const ID_ATMOSPHERE: u16 = 6;
pub const ID_RINGS: u16 = 7;
/// Cloud shell radius relative to the planet.
pub const CLOUD_SHELL: f32 = 1.015;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct CloudShell {
    pub enabled: bool,
    pub coverage: f32,
    /// How far the cyclones wind their spiral arms (0..1).
    pub swirl: f32,
    pub storm_eyes: u32,
    pub opacity: f32,
    /// Custom cloud map: the Cloud sub-material uses this image as its
    /// color and opacity map; the shell is then fully opaque in alpha.
    pub custom_map: Option<String>,
}

impl Default for CloudShell {
    fn default() -> Self {
        Self { enabled: true, coverage: 0.5, swirl: 0.6, storm_eyes: 3, opacity: 0.85, custom_map: None }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Atmosphere {
    pub enabled: bool,
    /// sRGB scattering color.
    pub color: [f32; 3],
    pub density: f32,
    /// Orange glow along the day / night terminator (0..1).
    pub sunset_glow: f32,
}

impl Default for Atmosphere {
    fn default() -> Self {
        Self { enabled: true, color: [0.35, 0.6, 1.0], density: 0.5, sunset_glow: 0.6 }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Rings {
    pub enabled: bool,
    /// Inner / outer radius as multiples of the planet radius.
    pub inner: f32,
    pub outer: f32,
    /// Tilt about X (degrees).
    pub tilt: f32,
    /// Radial band noise (0 = uniform disc, 1 = strongly banded with gaps).
    pub band_noise: f32,
    pub opacity: f32,
}

impl Default for Rings {
    fn default() -> Self {
        Self { enabled: false, inner: 1.35, outer: 2.3, tilt: 20.0, band_noise: 0.7, opacity: 0.8 }
    }
}

fn set_layer(m: &mut EditablePoly, id: u16) {
    m.polygons.values_mut().for_each(|p| p.material_id = id);
}

/// Coriolis cyclones: directions near a storm eye are rotated about it
/// (counter-clockwise in the north, clockwise in the south) by an angle
/// that falls off with distance, so the noise winds into spiral arms.
fn cyclone_warp(d: Vec3, eyes: &[(Vec3, f32)], swirl: f32) -> (Vec3, f32) {
    let (mut p, mut eye_clear) = (d, 0.0f32);
    for &(e, r) in eyes {
        let dist = d.dot(e).clamp(-1.0, 1.0).acos();
        let fall = (-dist / r).exp();
        let angle = swirl * 9.0 * fall * e.z.signum();
        p = Quat::from_axis_angle(e, angle) * p;
        eye_clear = eye_clear.max(1.0 - smooth(r * 0.08, r * 0.2, dist));
    }
    (p, eye_clear)
}

impl CloudShell {
    /// Cloud alpha (0..1) in direction `d` (before Opacity).
    pub fn alpha(&self, d: Vec3, fbm: &Fbm<Perlin>, eyes: &[(Vec3, f32)]) -> f32 {
        let (p, clear) = cyclone_warp(d, eyes, self.swirl.clamp(0.0, 1.0));
        let n = fbm.get((p * 2.2).as_dvec3().to_array()) as f32 * 0.5 + 0.5;
        // Weather bands: wet equator and 60° storm tracks, dry subtropics.
        let lat = d.z.clamp(-1.0, 1.0).asin();
        let band = 0.75 + 0.25 * (lat * 6.0).cos();
        let cov = self.coverage.clamp(0.0, 1.0);
        let a = smooth(1.0 - cov - 0.08, 1.0 - cov + 0.18, n * band + 0.12 * (1.0 - band.min(1.0)));
        a * (1.0 - clear)
    }

    pub fn build(&self, seed: u32, subdivisions: u32) -> EditablePoly {
        let mut m = equal_angle_sphere(subdivisions);
        let fbm = Fbm::<Perlin>::new(seed.wrapping_add(31)).set_octaves(6).set_persistence(0.55);
        let eyes: Vec<(Vec3, f32)> = (0..self.storm_eyes.min(12))
            .map(|i| {
                let lat = (15.0 + 45.0 * hash(seed, 300 + i)).to_radians() * if hash(seed, 320 + i) < 0.5 { 1.0 } else { -1.0 };
                let lon = hash(seed, 340 + i) * std::f32::consts::TAU;
                (Vec3::new(lat.cos() * lon.cos(), lat.cos() * lon.sin(), lat.sin()), 0.12 + 0.1 * hash(seed, 360 + i))
            })
            .collect();
        let custom = self.custom_map.is_some();
        for v in m.vertices.values_mut() {
            let a = if custom { 1.0 } else { self.alpha(v.position, &fbm, &eyes) * self.opacity.clamp(0.0, 1.0) };
            v.color = rgba(Vec3::ONE, a);
            v.position *= CLOUD_SHELL;
        }
        set_layer(&mut m, ID_CLOUDS);
        m
    }
}

impl Atmosphere {
    /// Shell radii (relative to the planet): stacked thin layers make the
    /// limb, seen edge-on through several layers, glow brightest.
    pub const SHELLS: [f32; 3] = [1.02, 1.035, 1.05];

    pub fn build(&self, subdivisions: u32, sun: Vec3) -> EditablePoly {
        let mut out = EditablePoly::new();
        let base = Vec3::from_array(self.color);
        let per = (self.density.clamp(0.0, 2.0) * 0.16).min(0.6);
        for (k, r) in Self::SHELLS.into_iter().enumerate() {
            let mut m = equal_angle_sphere(subdivisions);
            for v in m.vertices.values_mut() {
                let mu = v.position.dot(sun);
                let day = smooth(-0.25, 0.25, mu);
                let sunset = (1.0 - (mu / 0.22).powi(2)).max(0.0) * self.sunset_glow.clamp(0.0, 1.0);
                let col = base.lerp(Vec3::new(1.0, 0.45, 0.15), sunset);
                // Outer shells are thinner; the night side keeps a faint rim.
                let a = per * (1.0 - 0.25 * k as f32) * (0.15 + 0.85 * day.max(sunset));
                v.color = rgba(col, a);
            }
            out.append(&m, Mat4::from_scale(Vec3::splat(r)));
        }
        set_layer(&mut out, ID_ATMOSPHERE);
        out
    }
}

impl Rings {
    pub fn build(&self, seed: u32) -> EditablePoly {
        let (inner, outer) = (self.inner.max(1.0), self.outer.max(self.inner.max(1.0) + 0.05));
        let bands = 48;
        let (mut m, _) = annulus(inner, outer, bands, 128);
        let fbm = Fbm::<Perlin>::new(seed.wrapping_add(41)).set_octaves(5);
        let k = self.band_noise.clamp(0.0, 1.0);
        for v in m.vertices.values_mut() {
            let r = v.position.truncate().length();
            let t = (r - inner) / (outer - inner);
            let n = fbm.get([t as f64 * 9.0, 0.5]) as f32 * 0.5 + 0.5;
            // A Cassini-style division at 62% of the span, soft edges.
            let gap = 1.0 - (1.0 - smooth(0.0, 0.03, (t - 0.62).abs())) * k;
            let edge = smooth(0.0, 0.06, t) * smooth(1.0, 0.9, t);
            let a = (1.0 - k + k * n) * gap * edge * self.opacity.clamp(0.0, 1.0);
            let col = Vec3::new(0.82, 0.74, 0.6).lerp(Vec3::new(0.6, 0.5, 0.42), n);
            v.color = rgba(col, a);
        }
        m.transform(Mat4::from_rotation_x(self.tilt.to_radians()));
        set_layer(&mut m, ID_RINGS);
        m
    }
}
