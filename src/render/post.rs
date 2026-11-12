//! F10 GI & AO and Post-FX settings, the tonemapping operators (AgX, ACES
//! Filmic, Reinhard, Standard sRGB) with Exposure (EV) and Contrast, and the
//! compositor that turns a pixel's layers (premultiplied geometry, the
//! background behind it, bloom glow) into display sRGB. The Frame Window
//! re-runs [`compose`] when the tone controls change, without re-rendering.

use crate::materials::shading::{linear_to_srgb, srgb_to_linear};
use bevy::math::{Mat3, Vec3, Vec4};

/// GI & AO rollout.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct GiSettings {
    pub ao: bool,
    /// AO rays per shading point (4..64).
    pub ao_samples: u32,
    /// AO ray length (scene units).
    pub ao_radius: f32,
    /// 0 = no darkening, 1 = fully occluded crevices go black.
    pub ao_strength: f32,
    /// 1-bounce indirect diffuse (color bleed).
    pub gi: bool,
    /// Cosine-weighted bounce rays per shading point.
    pub gi_samples: u32,
    /// Next-event-estimation samples on emissive polygons per shading point.
    pub emitter_samples: u32,
    /// Edge-preserving spatial denoiser (normal / depth / object aware).
    pub denoise: bool,
    /// Built-in studio softbox dome on reflection rays (no HDRI / sky).
    pub softbox: bool,
}

impl Default for GiSettings {
    fn default() -> Self {
        Self {
            ao: false,
            ao_samples: 16,
            ao_radius: 0.5,
            ao_strength: 0.8,
            gi: false,
            gi_samples: 16,
            emitter_samples: 8,
            denoise: true,
            softbox: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Tonemap {
    AgX,
    #[default]
    AcesFilmic,
    Reinhard,
    StandardSrgb,
}

impl Tonemap {
    pub const ALL: [Tonemap; 4] = [Self::AgX, Self::AcesFilmic, Self::Reinhard, Self::StandardSrgb];
    pub fn label(self) -> &'static str {
        match self {
            Self::AgX => "AgX",
            Self::AcesFilmic => "ACES Filmic",
            Self::Reinhard => "Reinhard",
            Self::StandardSrgb => "Standard sRGB",
        }
    }
}

/// Tone controls (live in the F9 window).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Tone {
    pub op: Tonemap,
    /// Exposure in stops (EV).
    pub exposure: f32,
    /// Display-space contrast around mid-gray (1 = neutral).
    pub contrast: f32,
    /// Off: a plain background color shows exactly as chosen.
    pub affect_background: bool,
}

impl Default for Tone {
    fn default() -> Self {
        Self { op: Tonemap::AcesFilmic, exposure: 0.0, contrast: 1.0, affect_background: false }
    }
}

/// Post-FX rollout.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PostFx {
    pub tone: Tone,
    pub bloom: bool,
    /// Linear luminance above which pixels glow.
    pub bloom_threshold: f32,
    pub bloom_intensity: f32,
    /// Blur radius in pixels at 800 px image width (scales with resolution).
    pub bloom_radius: f32,
    pub dof: bool,
    /// Distance from the camera that is sharp (scene units, 1 unit = 1 m).
    pub focus_distance: f32,
    /// `[ Pick Focus Object ]`: focus on this object's center instead.
    pub focus_object: Option<crate::scene::ObjId>,
    pub f_stop: f32,
    /// F9 Color Grading & Camera RAW (applied to the finished image).
    pub grade: super::grading::Grading,
    /// Polygonal bokeh, anamorphic streaks and starburst glare.
    pub lens: super::lens_fx::LensFx,
}

impl Default for PostFx {
    fn default() -> Self {
        Self {
            tone: Tone::default(),
            bloom: false,
            bloom_threshold: 1.0,
            bloom_intensity: 0.6,
            bloom_radius: 12.0,
            dof: false,
            focus_distance: 5.0,
            focus_object: None,
            f_stop: 2.8,
            grade: Default::default(),
            lens: Default::default(),
        }
    }
}

const AGX_IN: Mat3 = Mat3::from_cols_array(&[
    0.842_479_06,
    0.042_328_24,
    0.042_375_655,
    0.078_433_6,
    0.878_468_6,
    0.078_433_6,
    0.079_223_745,
    0.079_166_13,
    0.879_143,
]);
const AGX_OUT: Mat3 = Mat3::from_cols_array(&[
    1.196_879,
    -0.052_896_85,
    -0.052_971_635,
    -0.098_020_88,
    1.151_903_1,
    -0.098_043_45,
    -0.099_029_74,
    -0.098_961_18,
    1.151_073_7,
]);

fn agx(c: Vec3) -> Vec3 {
    let (lo, hi) = (-12.473_93_f32, 4.026_069_f32);
    let x = AGX_IN * c.max(Vec3::splat(1e-10));
    let x = (Vec3::new(x.x.log2(), x.y.log2(), x.z.log2()).clamp(Vec3::splat(lo), Vec3::splat(hi)) - lo) / (hi - lo);
    let p = |x: f32| {
        let (x2, x4) = (x * x, x * x * x * x);
        15.5 * x4 * x2 - 40.14 * x4 * x + 31.96 * x4 - 6.868 * x2 * x + 0.4298 * x2 + 0.1191 * x - 0.00232
    };
    (AGX_OUT * Vec3::new(p(x.x), p(x.y), p(x.z))).clamp(Vec3::ZERO, Vec3::ONE)
}

impl Tone {
    /// Linear radiance -> display sRGB (0..1).
    pub fn map(&self, c: Vec3) -> Vec3 {
        let c = c.max(Vec3::ZERO) * 2f32.powf(self.exposure);
        let srgb = |v: Vec3| Vec3::new(linear_to_srgb(v.x), linear_to_srgb(v.y), linear_to_srgb(v.z));
        let d = match self.op {
            Tonemap::AgX => agx(c),
            Tonemap::AcesFilmic => crate::materials::shading::tonemap(c),
            Tonemap::Reinhard => srgb(c / (Vec3::ONE + c)),
            Tonemap::StandardSrgb => srgb(c.min(Vec3::ONE)),
        };
        self.contrast(d)
    }

    fn contrast(&self, d: Vec3) -> Vec3 {
        if (self.contrast - 1.0).abs() < 1e-4 {
            return d;
        }
        (Vec3::splat(0.5) + (d - 0.5) * self.contrast.max(0.0)).clamp(Vec3::ZERO, Vec3::ONE)
    }
}

/// What one pixel is made of before tonemapping.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PixelLayers {
    /// Geometry radiance premultiplied by coverage (w).
    pub geom: Vec4,
    /// Background radiance behind the geometry.
    pub back: Vec3,
    /// Bloom / glare radiance added on top.
    pub glow: Vec3,
    /// Ink outline drawn over everything: display sRGB + coverage (exact
    /// color, never tonemapped).
    pub ink: Vec4,
}

impl PixelLayers {
    pub const ZERO: PixelLayers = PixelLayers { geom: Vec4::ZERO, back: Vec3::ZERO, glow: Vec3::ZERO, ink: Vec4::ZERO };

    /// Linear radiance + alpha as saved to EXR.
    pub fn hdr(&self, transparent: bool) -> Vec4 {
        let a = self.geom.w;
        let c = if transparent {
            (self.geom.truncate() + self.glow).extend(a)
        } else {
            (self.geom.truncate() + self.back * (1.0 - a) + self.glow).extend(1.0)
        };
        if self.ink.w <= 0.0 {
            return c;
        }
        let lin = Vec3::new(srgb_to_linear(self.ink.x), srgb_to_linear(self.ink.y), srgb_to_linear(self.ink.z));
        c.truncate().lerp(lin, self.ink.w).extend(c.w.max(self.ink.w))
    }
}

/// Display sRGB + straight alpha for one pixel. `plain_bg`: the background
/// is the Environment color (shown exactly unless Affect Background is on).
pub fn compose(l: &PixelLayers, tone: &Tone, plain_bg: bool, transparent: bool) -> Vec4 {
    let a = l.geom.w.clamp(0.0, 1.0);
    let surf = if a > 1e-5 { tone.map(l.geom.truncate() / a) } else { Vec3::ZERO };
    let mut out = if transparent {
        surf.extend(a)
    } else {
        let bg = if plain_bg && !tone.affect_background {
            Vec3::new(linear_to_srgb(l.back.x), linear_to_srgb(l.back.y), linear_to_srgb(l.back.z))
        } else {
            tone.map(l.back)
        };
        (surf * a + bg * (1.0 - a)).extend(1.0)
    };
    if l.glow.max_element() > 0.0 {
        // Screen-blend the tonemapped glow over the image.
        let g = tone.map(l.glow);
        let rgb = Vec3::ONE - (Vec3::ONE - out.truncate()) * (Vec3::ONE - g);
        out = rgb.extend(out.w.max(g.max_element()));
    }
    if l.ink.w > 0.0 {
        out = out.truncate().lerp(l.ink.truncate(), l.ink.w).extend(out.w.max(l.ink.w));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tonemappers_are_monotonic_and_bounded() {
        for op in Tonemap::ALL {
            let t = Tone { op, ..Default::default() };
            let mut last = -1.0;
            for i in 0..60 {
                let v = t.map(Vec3::splat(i as f32 * 0.25)).x;
                assert!((0.0..=1.0).contains(&v) && v >= last - 1e-4, "{op:?} at {i}: {v} after {last}");
                last = v;
            }
            assert!(t.map(Vec3::ZERO).max_element() < 0.02, "{op:?} keeps black black");
            assert!(t.map(Vec3::splat(0.18)).x > 0.25, "{op:?}: mid-gray is visible");
        }
        let base = Tone::default();
        let bright = Tone { exposure: 1.0, ..base };
        assert!(bright.map(Vec3::splat(0.2)).x > base.map(Vec3::splat(0.2)).x + 0.05, "+1 EV brightens");
        let punchy = Tone { contrast: 1.5, ..base };
        assert!(punchy.map(Vec3::splat(0.02)).x < base.map(Vec3::splat(0.02)).x);
        assert!(punchy.map(Vec3::splat(2.0)).x > base.map(Vec3::splat(2.0)).x);
        // AgX desaturates very bright saturated light instead of clipping hue.
        let hot = Tone { op: Tonemap::AgX, ..base }.map(Vec3::new(40.0, 4.0, 0.5));
        assert!(hot.y > 0.6, "AgX rolls bright red toward white: {hot}");
    }

    #[test]
    fn compose_keeps_the_plain_background_exact() {
        let back = Vec3::new(0.05, 0.1, 0.2);
        let l = PixelLayers { geom: Vec4::ZERO, back, glow: Vec3::ZERO, ink: Vec4::ZERO };
        let t = Tone { exposure: 2.0, ..Default::default() };
        let d = compose(&l, &t, true, false);
        assert!((d.x - linear_to_srgb(0.05)).abs() < 1e-5 && d.w == 1.0);
        let affected = compose(&l, &Tone { affect_background: true, ..t }, true, false);
        assert!(affected.z > d.z, "Affect Background exposes it too");
        // Half coverage blends half the surface over the background.
        let half = PixelLayers { geom: Vec4::new(0.5, 0.5, 0.5, 0.5), back, glow: Vec3::ZERO, ink: Vec4::ZERO };
        let c = compose(&half, &Tone::default(), true, false);
        let full = Tone::default().map(Vec3::ONE).x;
        assert!((c.x - (full * 0.5 + linear_to_srgb(0.05) * 0.5)).abs() < 1e-4);
        assert_eq!(compose(&half, &Tone::default(), true, true).w, 0.5);
        assert_eq!(half.hdr(false), Vec4::new(0.525, 0.55, 0.6, 1.0));
        let glowing = PixelLayers { glow: Vec3::splat(2.0), ..l };
        assert!(compose(&glowing, &Tone::default(), true, false).x > d.x + 0.3, "bloom brightens");
    }
}
