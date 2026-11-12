//! Physical lens effects for F9 (after EffectCraft, MIT / Apache-2.0):
//! a polygonal aperture (5..9 blades, rotation, cat's-eye vignetting) that
//! shapes depth-of-field bokeh, an anamorphic horizontal streak flare and a
//! 4 / 6 / 8-point starburst glare on the brightest highlights.

use super::sample::Pixel;
use bevy::math::{Vec2, Vec3};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct LensFx {
    /// Aperture blades: 0 = circular, else 5..=9.
    pub blades: u32,
    /// Blade rotation in degrees.
    pub blade_rotation: f32,
    /// 0..1: off-centre bokeh is clipped into a cat's-eye shape.
    pub cats_eye: f32,
    pub anamorphic: bool,
    pub anamorphic_strength: f32,
    /// Streak tint (sRGB): classic sci-fi blue.
    pub anamorphic_tint: [f32; 3],
    /// Starburst points: 0 (off), 4, 6 or 8.
    pub star_points: u32,
    pub star_strength: f32,
    /// Linear luminance above which highlights flare.
    pub threshold: f32,
}

impl Default for LensFx {
    fn default() -> Self {
        Self {
            blades: 0,
            blade_rotation: 0.0,
            cats_eye: 0.0,
            anamorphic: false,
            anamorphic_strength: 0.5,
            anamorphic_tint: [0.35, 0.55, 1.0],
            star_points: 0,
            star_strength: 0.5,
            threshold: 1.5,
        }
    }
}

impl LensFx {
    pub const STAR_POINTS: [u32; 4] = [0, 4, 6, 8];

    pub fn glare_on(&self) -> bool {
        (self.anamorphic && self.anamorphic_strength > 0.0) || (self.star_points > 0 && self.star_strength > 0.0)
    }
}

/// Distance from the centre to the aperture edge at angle `theta`, for a
/// unit-circumradius polygon of `blades` sides (1 for a circle).
pub fn aperture_radius(theta: f32, blades: u32, rotation_deg: f32) -> f32 {
    if blades < 3 {
        return 1.0;
    }
    let seg = TAU / blades as f32;
    let t = (theta - rotation_deg.to_radians()).rem_euclid(seg) - seg * 0.5;
    (PI / blades as f32).cos() / t.cos()
}

/// Whether a bokeh tap at `offset` (pixels, relative to a disc of radius
/// `r`) survives cat's-eye clipping for a pixel at `from_center` (-1..1
/// screen position): the aperture is cut by a second, shifted disc.
pub fn cats_eye_keeps(offset: Vec2, r: f32, from_center: Vec2, amount: f32) -> bool {
    if amount <= 0.0 || r <= 0.0 {
        return true;
    }
    let shift = from_center * amount * r * 1.2;
    (offset + shift).length() <= r * 1.05
}

/// Adds the anamorphic streak and starburst glare to each pixel's glow.
pub fn glare(px: &mut [Pixel], w: usize, h: usize, fx: &LensFx) {
    if !fx.glare_on() || w == 0 || h == 0 {
        return;
    }
    let bright: Vec<Vec3> = px
        .iter()
        .map(|p| {
            let c = p.layers.geom.truncate();
            let l = c.dot(Vec3::new(0.2126, 0.7152, 0.0722));
            if l > fx.threshold { c * ((l - fx.threshold) / l) } else { Vec3::ZERO }
        })
        .collect();
    if bright.iter().all(|c| *c == Vec3::ZERO) {
        return;
    }
    let mut glow = vec![Vec3::ZERO; w * h];
    if fx.anamorphic {
        let tint = Vec3::from_array(fx.anamorphic_tint.map(crate::materials::shading::srgb_to_linear));
        streak(&bright, &mut glow, w, h, Vec2::X, w as f32 * 0.25, tint * fx.anamorphic_strength);
    }
    if fx.star_points > 0 {
        let rays = fx.star_points / 2;
        let len = w.max(h) as f32 * 0.08;
        for k in 0..rays {
            let a = PI * k as f32 / rays as f32 + PI / 4.0 * f32::from(fx.star_points == 4);
            streak(&bright, &mut glow, w, h, Vec2::new(a.cos(), a.sin()), len, Vec3::splat(fx.star_strength));
        }
    }
    for (p, g) in px.iter_mut().zip(glow) {
        p.layers.glow += g;
    }
}

/// Smears `src` both ways along `dir` with an exponential falloff of
/// length `len` pixels, adding `gain`-scaled light into `dst`.
fn streak(src: &[Vec3], dst: &mut [Vec3], w: usize, h: usize, dir: Vec2, len: f32, gain: Vec3) {
    let taps = (len * 2.5).ceil() as i32;
    for y in 0..h {
        for x in 0..w {
            let c = src[y * w + x];
            if c == Vec3::ZERO {
                continue;
            }
            for s in 1..=taps {
                let k = (-(s as f32) / len).exp() / len;
                for sign in [-1.0, 1.0] {
                    let p = Vec2::new(x as f32, y as f32) + dir * s as f32 * sign;
                    let (sx, sy) = (p.x.round() as i32, p.y.round() as i32);
                    if sx >= 0 && sy >= 0 && (sx as usize) < w && (sy as usize) < h {
                        dst[sy as usize * w + sx as usize] += c * gain * k;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::math::Vec4;

    #[test]
    fn polygonal_aperture_has_flat_blades() {
        assert_eq!(aperture_radius(1.234, 0, 0.0), 1.0, "circular");
        for n in 5..=9 {
            let r: Vec<f32> = (0..720).map(|i| aperture_radius(i as f32 / 720.0 * TAU, n, 0.0)).collect();
            let (lo, hi) = r.iter().fold((f32::MAX, 0.0f32), |(a, b), v| (a.min(*v), b.max(*v)));
            assert!((lo - (PI / n as f32).cos()).abs() < 1e-3 && (hi - 1.0).abs() < 1e-2, "{n} blades: {lo}..{hi}");
        }
        // Rotating by one blade's angle gives the same shape.
        assert!((aperture_radius(0.3, 6, 0.0) - aperture_radius(0.3, 6, 60.0)).abs() < 1e-5);
    }

    #[test]
    fn cats_eye_clips_only_off_centre() {
        let tap = Vec2::new(9.0, 0.0);
        assert!(cats_eye_keeps(tap, 10.0, Vec2::ZERO, 1.0), "centre keeps the full disc");
        assert!(!cats_eye_keeps(tap, 10.0, Vec2::new(1.0, 0.0), 1.0), "edge pixel loses its outer side");
        assert!(cats_eye_keeps(-tap, 10.0, Vec2::new(1.0, 0.0), 0.2));
    }

    fn spot(fx: &LensFx) -> (Vec<Pixel>, usize) {
        let n = 81;
        let mut px = vec![Pixel::EMPTY; n * n];
        px[40 * n + 40].layers.geom = Vec4::new(50.0, 50.0, 50.0, 1.0);
        glare(&mut px, n, n, fx);
        (px, n)
    }

    #[test]
    fn anamorphic_streak_is_horizontal_and_tinted() {
        let (px, n) = spot(&LensFx { anamorphic: true, ..Default::default() });
        let g = |x: usize, y: usize| px[y * n + x].layers.glow;
        assert!(g(60, 40).max_element() > 0.0 && g(40, 60) == Vec3::ZERO, "horizontal only");
        assert!(g(60, 40).z > g(60, 40).x, "blue streak");
        assert!(g(45, 40).z > g(70, 40).z, "fades with distance");
    }

    #[test]
    fn starburst_points_follow_the_count() {
        let rays = |pts| {
            let (px, n) = spot(&LensFx { star_points: pts, ..Default::default() });
            let lit = |x: usize, y: usize| px[y * n + x].layers.glow.max_element() > 0.0;
            (lit(44, 40), lit(44, 44))
        };
        assert_eq!(rays(4), (false, true), "4 points: diagonal X");
        assert_eq!(rays(8), (true, true), "8 points: axes and diagonals");
        let (px, _) = spot(&LensFx::default());
        assert!(px.iter().all(|p| p.layers.glow == Vec3::ZERO), "off by default");
    }
}
