//! Dynamic sky (`8` dialog): Earth mode with a Time of Day (00:00..24:00)
//! driving the sun / moon path and an analytic Rayleigh / Mie look (noon
//! blue, golden-hour sunset, indigo starlit night) plus procedural clouds;
//! Deep Space mode with a starfield, nebula bands and one bright star. The
//! sky is baked to an equirect image and used like an HDRI (F9 background,
//! reflections, ambient, viewport skybox). Z-up.

use super::hdri::HdrImage;
use super::sky_noise::{fbm, hash3, smooth};
use bevy::math::{Quat, Vec3};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum SkyMode {
    #[default]
    Off,
    Earth,
    Space,
}

impl SkyMode {
    pub const ALL: [SkyMode; 3] = [Self::Off, Self::Earth, Self::Space];
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Off (Background Color / HDRI)",
            Self::Earth => "Dynamic Earth Sky",
            Self::Space => "Deep Space",
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Clouds {
    pub enabled: bool,
    /// Fraction of the sky covered (0..1).
    pub coverage: f32,
    /// Opacity of the cloud layer (0..1).
    pub density: f32,
    /// Cloud deck height (km); higher decks look smaller and flatter.
    pub altitude: f32,
    /// Drift (km per hour of Time of Day) along +X.
    pub wind_speed: f32,
}

impl Default for Clouds {
    fn default() -> Self {
        Self { enabled: true, coverage: 0.4, density: 0.8, altitude: 2.0, wind_speed: 0.5 }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Space {
    /// Fraction of sky cells holding a star (0..0.1).
    pub star_density: f32,
    pub star_brightness: f32,
    pub nebula: bool,
    pub nebula_a: [f32; 3],
    pub nebula_b: [f32; 3],
    pub nebula_density: f32,
    /// The single star (sun) lighting the scene: azimuth / elevation (deg).
    pub star_azimuth: f32,
    pub star_elevation: f32,
    pub star_color: [f32; 3],
}

impl Default for Space {
    fn default() -> Self {
        Self {
            star_density: 0.02,
            star_brightness: 1.0,
            nebula: true,
            nebula_a: [0.45, 0.12, 0.6],
            nebula_b: [0.05, 0.35, 0.6],
            nebula_density: 0.5,
            star_azimuth: 40.0,
            star_elevation: 25.0,
            star_color: [1.0, 0.95, 0.88],
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct SkySettings {
    pub mode: SkyMode,
    /// Hours, 0..24 (12 = noon).
    pub time_of_day: f32,
    /// Compass turn of the sun path about Z (degrees; 0 = rises at -X).
    pub sun_azimuth: f32,
    /// Sky radiance multiplier.
    pub intensity: f32,
    /// `[✓] Link Main Directional Sun to Sky Time`.
    pub link_sun: bool,
    /// Adds the sky's average to the ambient light (F9).
    pub lighting: bool,
    pub show_in_viewport: bool,
    pub clouds: Clouds,
    pub space: Space,
}

impl Default for SkySettings {
    fn default() -> Self {
        Self {
            mode: SkyMode::Off,
            time_of_day: 14.0,
            sun_azimuth: 0.0,
            intensity: 1.0,
            link_sun: true,
            lighting: true,
            show_in_viewport: true,
            clouds: Clouds::default(),
            space: Space::default(),
        }
    }
}

/// Tilt of the daily sun path from the zenith (noon elevation = 90 - tilt).
const PATH_TILT: f32 = 35.0;
/// Default multiplier of the linked moonlight at night.
pub const MOONLIGHT: f32 = 0.22;

impl SkySettings {
    /// Unit direction toward the sun for the current Time of Day.
    pub fn sun_dir(&self) -> Vec3 {
        let h = (self.time_of_day - 12.0) / 24.0 * TAU;
        let t = PATH_TILT.to_radians();
        let d = Vec3::new(h.sin(), -h.cos() * t.sin(), h.cos() * t.cos());
        Quat::from_rotation_z(self.sun_azimuth.to_radians()) * d
    }

    /// The moon rides opposite the sun.
    pub fn moon_dir(&self) -> Vec3 {
        let s = self.sun_dir();
        Vec3::new(-s.x, -s.y, -s.z)
    }

    fn star_dir(&self) -> Vec3 {
        let (az, el) = (self.space.star_azimuth.to_radians(), self.space.star_elevation.to_radians());
        Vec3::new(el.cos() * az.cos(), el.cos() * az.sin(), el.sin())
    }

    /// The light the linked main directional light should cast: direction
    /// toward the light, linear color and Multiplier. Night switches to cool
    /// moonlight.
    pub fn key_light(&self) -> Option<(Vec3, Vec3, f32)> {
        match self.mode {
            SkyMode::Off => None,
            SkyMode::Space => Some((self.star_dir(), Vec3::from_array(self.space.star_color), 1.6)),
            SkyMode::Earth => {
                let s = self.sun_dir();
                if s.z > -0.03 {
                    let warm = 1.0 - smooth(0.0, 0.35, s.z);
                    let col = Vec3::new(1.0, 0.96, 0.9).lerp(Vec3::new(1.0, 0.55, 0.25), warm);
                    Some((s, col, 1.4 * smooth(-0.03, 0.08, s.z)))
                } else {
                    Some((self.moon_dir(), Vec3::new(0.55, 0.65, 1.0), MOONLIGHT))
                }
            }
        }
    }

    /// Cool night skylight added to the ambient term (viewport and F9) so
    /// unlit surfaces stay readable under moonlight; zero by day.
    pub fn night_ambient(&self) -> Vec3 {
        if self.mode != SkyMode::Earth {
            return Vec3::ZERO;
        }
        let night = 1.0 - smooth(-0.12, 0.2, self.sun_dir().z);
        Vec3::new(0.05, 0.06, 0.1) * night * self.intensity.max(0.0)
    }

    /// Six `size`² cube faces evaluated straight from the sky (no equirect
    /// bake in between), so stars stay crisp in the viewport skybox.
    pub fn cube_faces(&self, size: u32) -> Vec<[f32; 4]> {
        super::hdri::cube_faces_with(size, |d| self.radiance(d))
    }

    /// Linear sky radiance in direction `d`.
    pub fn radiance(&self, d: Vec3) -> Vec3 {
        let d = d.normalize_or(Vec3::Z);
        let c = match self.mode {
            SkyMode::Off => Vec3::ZERO,
            SkyMode::Earth => self.earth(d),
            SkyMode::Space => self.deep_space(d),
        };
        c * self.intensity.max(0.0)
    }

    fn stars(&self, d: Vec3, density: f32, scale: f32) -> Vec3 {
        let p = d * scale;
        let cell = p.floor();
        let (x, y, z) = (cell.x as i32, cell.y as i32, cell.z as i32);
        let h = hash3(x, y, z);
        if h > density {
            return Vec3::ZERO;
        }
        let center = cell + Vec3::new(hash3(x, z, y), hash3(y, x, z), hash3(z, y, x));
        let r = (p - center).length();
        let k = (h / density.max(1e-6)).powf(4.0);
        let tint = Vec3::new(1.0, 0.92, 0.85).lerp(Vec3::new(0.75, 0.85, 1.0), hash3(z, x, y));
        tint * (1.0 - r * 2.2).max(0.0).powi(2) * (0.4 + 6.0 * k)
    }

    fn clouds(&self, d: Vec3, sun: Vec3, day: f32) -> Option<(f32, Vec3)> {
        let c = &self.clouds;
        if !c.enabled || d.z <= 0.01 || c.coverage <= 0.0 {
            return None;
        }
        let alt = c.altitude.clamp(0.2, 20.0);
        let drift = Vec3::new(c.wind_speed * self.time_of_day, 0.0, 0.0);
        let p = Vec3::new(d.x / d.z, d.y / d.z, 0.0) * alt * 0.6 + drift;
        let n = fbm(p * 0.9 + Vec3::new(0.0, 0.0, alt), 5);
        let cov = c.coverage.clamp(0.0, 1.0);
        let a = smooth(1.0 - cov, 1.0 - cov + 0.25, n) * c.density.clamp(0.0, 1.0) * smooth(0.01, 0.12, d.z);
        if a <= 0.0 {
            return None;
        }
        let low = 1.0 - smooth(0.0, 0.3, sun.z);
        let lit = Vec3::ONE.lerp(Vec3::new(1.0, 0.55, 0.3), low) * (0.25 + 1.6 * day);
        let moon = Vec3::new(0.05, 0.06, 0.09) * (1.0 - day);
        let silver = d.dot(sun).max(0.0).powi(6) * day;
        Some((a, (lit + Vec3::splat(silver)) * (1.15 - 0.5 * n) + moon))
    }

    fn earth(&self, d: Vec3) -> Vec3 {
        let sun = self.sun_dir();
        let e = sun.z;
        let day = smooth(-0.12, 0.2, e);
        let up = d.z.max(0.0);
        // Rayleigh: deep blue zenith, pale horizon; Mie: glow around the sun.
        let zenith = Vec3::new(0.18, 0.38, 0.95) * day + Vec3::new(0.02, 0.025, 0.065) * (1.0 - day);
        let horizon = Vec3::new(0.65, 0.75, 0.9) * day + Vec3::new(0.05, 0.058, 0.1) * (1.0 - day);
        let mut c = horizon.lerp(zenith, up.powf(0.45));
        let golden = (-(e.abs() * 7.0)).exp();
        let toward = d.dot(Vec3::new(sun.x, sun.y, 0.0).normalize_or_zero()).max(0.0);
        c += Vec3::new(1.0, 0.42, 0.12) * golden * toward.powi(3) * (1.0 - up).powi(4) * 1.6;
        let mu = d.dot(sun).max(0.0);
        c += Vec3::new(1.0, 0.85, 0.65) * (mu.powi(24) * 0.6 + mu.powi(400) * 4.0) * smooth(-0.05, 0.05, e);
        if mu > 0.9995 && e > -0.02 {
            c += Vec3::new(1.0, 0.9, 0.7) * 60.0;
        }
        // Night: stars and the moon.
        if day < 1.0 && d.z > 0.0 {
            c += self.stars(d, 0.03, 140.0) * (1.0 - day) * up.sqrt();
            if d.dot(self.moon_dir()) > 0.9997 {
                c += Vec3::new(0.85, 0.88, 0.95) * 3.0 * (1.0 - day);
            }
        }
        if let Some((a, col)) = self.clouds(d, sun, day) {
            c = c.lerp(col * (horizon + zenith).max_element().max(0.04), a);
        }
        if d.z < 0.0 {
            // Ground: dim earth tone fading into the horizon haze.
            let ground = Vec3::new(0.09, 0.085, 0.075) * (0.08 + day);
            c = c.lerp(ground, smooth(0.0, 0.06, -d.z));
        }
        c
    }

    fn deep_space(&self, d: Vec3) -> Vec3 {
        let s = &self.space;
        let mut c = Vec3::new(0.002, 0.002, 0.004);
        c += self.stars(d, s.star_density.clamp(0.0, 0.2), 180.0) * s.star_brightness;
        c += self.stars(d, s.star_density.clamp(0.0, 0.2) * 0.5, 60.0) * s.star_brightness * 0.6;
        if s.nebula {
            let band_axis = Vec3::new(0.3, -0.5, 0.8).normalize();
            let band = (-(d.dot(band_axis) * 3.0).powi(2)).exp();
            let n = fbm(d * 3.0 + Vec3::splat(7.0), 5);
            let m = fbm(d * 6.0 - Vec3::splat(3.0), 4);
            let k = smooth(0.35, 0.8, n) * band * s.nebula_density.clamp(0.0, 2.0);
            c += Vec3::from_array(s.nebula_a).lerp(Vec3::from_array(s.nebula_b), m) * k * 0.6;
        }
        let star = self.star_dir();
        let mu = d.dot(star).max(0.0);
        let col = Vec3::from_array(s.star_color);
        c += col * (mu.powi(800) * 3.0 + mu.powi(60) * 0.15);
        if mu > 0.9997 {
            c += col * 80.0;
        }
        c
    }

    /// Bakes the sky to a `w` x `h` equirect image (same layout as HDRIs).
    pub fn bake(&self, w: u32, h: u32) -> HdrImage {
        let mut px = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            let theta = PI * (y as f32 + 0.5) / h as f32;
            for x in 0..w {
                // Inverse of `HdrImage::sample` with rotation 0.
                let phi = (0.5 - (x as f32 + 0.5) / w as f32) * TAU;
                px.push(self.radiance(Vec3::new(theta.sin() * phi.cos(), theta.sin() * phi.sin(), theta.cos())));
            }
        }
        HdrImage { w, h, px }
    }

    /// Cache key: changes whenever the baked image would.
    pub fn key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        format!("{self:?}").hash(&mut h);
        h.finish()
    }
}

/// The sky baked into an HDRI-like background (`w` x `w/2`), or None when Off.
pub fn procedural_hdri(sky: &SkySettings, w: u32) -> Option<super::hdri::Hdri> {
    (sky.mode != SkyMode::Off).then(|| super::hdri::Hdri {
        path: std::env::temp_dir().join(format!("minimax_sky_{:016x}_{w}.hdr", sky.key())),
        strength: 1.0,
        rotation: 0.0,
        lighting: sky.lighting,
        show_in_viewport: sky.show_in_viewport,
        image: Some(std::sync::Arc::new(sky.bake(w, w / 2))),
    })
}
