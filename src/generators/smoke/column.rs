//! Buoyant plume physics for the smoke generator, written natively from
//! classic plume theory (Morton-Taylor-Turner entrainment, Briggs'
//! bent-over plume rise, Karman vortex shedding). No CFD solver: these
//! closed-form laws shape the column and place the puffs.

use super::SmokeParams;
use bevy::math::{Vec2, Vec3};
use std::f32::consts::TAU;

/// Strouhal number of vortex shedding behind a bluff column.
pub const STROUHAL: f32 = 0.2;

/// The plume's centreline and width as functions of normalised height
/// `s` (0 = source, 1 = top).
#[derive(Clone, Copy, Debug)]
pub struct Column {
    pub height: f32,
    pub r0: f32,
    pub r1: f32,
    /// tan(lean): horizontal drift at the top per metre of height.
    pub lean: f32,
    /// Downwind direction (XY unit vector).
    pub wind: Vec2,
    pub turbulence: f32,
    pub phase: f32,
}

impl Column {
    pub fn new(p: &SmokeParams) -> Self {
        let r0 = p.base_radius.clamp(0.05, 100.0);
        Self {
            height: p.height.clamp(0.5, 2000.0),
            r0,
            r1: r0 * p.top_spread.clamp(1.0, 12.0),
            lean: p.wind_lean.clamp(0.0, 80.0).to_radians().tan(),
            wind: Vec2::from_angle(p.wind_dir.to_radians()),
            turbulence: p.turbulence.clamp(0.0, 1.0),
            phase: p.phase.rem_euclid(1.0),
        }
    }

    /// Entrainment: the radius grows (almost) linearly with height.
    pub fn radius(&self, s: f32) -> f32 {
        self.r0 + (self.r1 - self.r0) * s.clamp(0.0, 1.0).powf(0.9)
    }

    /// Vertical speed relative to the source: a buoyant point-source plume
    /// slows as z^(-1/3) (clamped near the source).
    pub fn speed(&self, s: f32) -> f32 {
        (s.max(0.02) / 0.02).powf(-1.0 / 3.0)
    }

    /// Normalised height reached after normalised rise time `t` (0..1): with
    /// w ~ z^(-1/3), z ~ t^(3/4), so puffs bunch up toward the top.
    pub fn height_at(t: f32) -> f32 {
        t.clamp(0.0, 1.0).powf(0.75)
    }

    /// Centreline point at height `s`: bent over downwind (Briggs: rise ~
    /// drift^(2/3), so drift ~ z^(3/2)) and swaying across the wind as
    /// vortices shed at wavelength 2r / St, travelling up with the phase.
    pub fn centre(&self, s: f32) -> Vec3 {
        let s = s.clamp(0.0, 1.0);
        let z = s * self.height;
        let drift = self.lean * self.height * s.powf(1.5);
        let mean_r = (self.r0 + self.radius(s)) * 0.5;
        let wavelength = 2.0 * mean_r / STROUHAL;
        let sway = self.turbulence * self.radius(s) * 0.45 * (TAU * (z / wavelength - self.phase)).sin() * s.sqrt();
        (self.wind * drift + self.wind.perp() * sway).extend(z)
    }

    /// Unit tangent of the centreline (for orienting the puffs).
    pub fn tangent(&self, s: f32) -> Vec3 {
        let e = 0.01;
        (self.centre((s + e).min(1.0)) - self.centre((s - e).max(0.0))).normalize_or(Vec3::Z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plume_widens_slows_and_bends_downwind() {
        let p = SmokeParams { wind_lean: 30.0, turbulence: 0.0, ..Default::default() };
        let c = Column::new(&p);
        assert!(c.radius(1.0) > c.radius(0.5) && c.radius(0.5) > c.radius(0.0));
        assert!(c.speed(0.9) < c.speed(0.1), "buoyant plume slows with height");
        assert!(Column::height_at(0.5) > 0.5, "puffs bunch toward the top");
        let top = c.centre(1.0);
        assert!((top.z - p.height).abs() < 1e-3);
        assert!(top.x > 1.0 && top.y.abs() < 1e-3, "bent downwind along +X: {top}");
        // Briggs: the bend grows faster than linearly with height.
        assert!(c.centre(0.5).x < top.x * 0.5);
        let still = Column::new(&SmokeParams { wind_lean: 0.0, turbulence: 0.0, ..Default::default() });
        assert!(still.centre(1.0).truncate().length() < 1e-3);
    }

    #[test]
    fn vortex_shedding_sways_across_the_wind_and_travels_with_phase() {
        let p = SmokeParams { wind_lean: 0.0, turbulence: 1.0, ..Default::default() };
        let a = Column::new(&p);
        let b = Column::new(&SmokeParams { phase: 0.25, ..p.clone() });
        let sways: Vec<f32> = (1..20).map(|i| a.centre(i as f32 / 20.0).y).collect();
        assert!(sways.iter().any(|&y| y > 0.1) && sways.iter().any(|&y| y < -0.1), "alternating vortices {sways:?}");
        assert!((a.centre(0.6).y - b.centre(0.6).y).abs() > 0.05, "phase scrubs the sway");
    }
}
