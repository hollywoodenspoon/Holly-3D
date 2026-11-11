//! Color Grading & Camera RAW for the Rendered Frame Window (after
//! LightCraft's develop pipeline, MIT / Apache-2.0). Runs on the finished
//! display image, so every control updates live without re-rendering:
//! chromatic aberration → white balance → clarity and tone → vibrance /
//! saturation → split toning → vignette → film grain.

use crate::materials::shading::{linear_to_srgb, srgb_to_linear};
use bevy::math::{Vec2, Vec3, Vec4};

/// A colour wheel: which hue to push towards and how far (0..=1).
#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Wheel {
    /// Degrees, 0 = red, 120 = green, 240 = blue.
    pub hue: f32,
    pub amount: f32,
}

impl Wheel {
    /// The wheel's colour shift with its brightness taken out.
    pub fn shift(&self) -> Vec3 {
        if self.amount <= 0.0 {
            return Vec3::ZERO;
        }
        let c = hue_rgb(self.hue);
        (c - Vec3::splat(luma(c))) * self.amount.clamp(0.0, 1.0)
    }
}

/// All grading controls. Percent sliders run -100..=100 (0 = neutral).
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Grading {
    pub enabled: bool,
    pub highlights: f32,
    pub shadows: f32,
    pub whites: f32,
    pub blacks: f32,
    /// Local contrast.
    pub clarity: f32,
    pub vibrance: f32,
    pub saturation: f32,
    /// The light the image is balanced for, in Kelvin (6500 = neutral):
    /// lower is cooler, higher is warmer, as in Camera RAW.
    pub temperature: f32,
    /// Green (-) to magenta (+).
    pub tint: f32,
    pub shadow_wheel: Wheel,
    pub midtone_wheel: Wheel,
    pub highlight_wheel: Wheel,
    /// Darkens (-) or lightens (+) the corners.
    pub vignette: f32,
    /// 0..=100: how far from the centre the vignette begins.
    pub vignette_midpoint: f32,
    /// 0..=100: red / blue fringe towards the corners.
    pub chromatic_aberration: f32,
    /// 0..=100.
    pub grain: f32,
    pub grain_seed: u32,
}

impl Default for Grading {
    fn default() -> Self {
        Self {
            enabled: true,
            highlights: 0.0,
            shadows: 0.0,
            whites: 0.0,
            blacks: 0.0,
            clarity: 0.0,
            vibrance: 0.0,
            saturation: 0.0,
            temperature: NEUTRAL_K,
            tint: 0.0,
            shadow_wheel: Wheel::default(),
            midtone_wheel: Wheel::default(),
            highlight_wheel: Wheel::default(),
            vignette: 0.0,
            vignette_midpoint: 50.0,
            chromatic_aberration: 0.0,
            grain: 0.0,
            grain_seed: 0,
        }
    }
}

pub const NEUTRAL_K: f32 = 6500.0;

impl Grading {
    /// True when grading would leave the image as it is.
    pub fn is_neutral(&self) -> bool {
        let n = Grading { enabled: self.enabled, vignette_midpoint: self.vignette_midpoint, grain_seed: self.grain_seed, ..Default::default() };
        !self.enabled || *self == n
    }
}

pub fn luma(c: Vec3) -> f32 {
    c.dot(Vec3::new(0.2126, 0.7152, 0.0722))
}

fn hue_rgb(deg: f32) -> Vec3 {
    let h = deg.rem_euclid(360.0) / 60.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    match h as u32 {
        0 => Vec3::new(1.0, x, 0.0),
        1 => Vec3::new(x, 1.0, 0.0),
        2 => Vec3::new(0.0, 1.0, x),
        3 => Vec3::new(0.0, x, 1.0),
        4 => Vec3::new(x, 0.0, 1.0),
        _ => Vec3::new(1.0, 0.0, x),
    }
}

/// Black-body colour (linear, 0..1) of `k` Kelvin (Tanner Helland's fit).
pub fn kelvin_rgb(k: f32) -> Vec3 {
    let t = k.clamp(1000.0, 40000.0) / 100.0;
    let r = if t <= 66.0 { 255.0 } else { 329.699 * (t - 60.0).powf(-0.133_204_76) };
    let g = if t <= 66.0 { 99.4708 * t.ln() - 161.1196 } else { 288.1222 * (t - 60.0).powf(-0.075_514_85) };
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.5177 * (t - 10.0).ln() - 305.0448
    };
    Vec3::new(r, g, b).map(|c| srgb_to_linear((c / 255.0).clamp(0.0, 1.0)))
}

/// Linear-light gains for the white balance controls (luminance kept).
pub fn white_balance(temperature: f32, tint: f32) -> Vec3 {
    // The image is balanced for light of this colour: a bluer (higher)
    // light is compensated by warming, as in Camera RAW.
    let src = kelvin_rgb(temperature.clamp(1500.0, 25000.0));
    let mut g = kelvin_rgb(NEUTRAL_K) / src.max(Vec3::splat(1e-3));
    g.y *= 1.0 - tint.clamp(-100.0, 100.0) * 0.003;
    g / luma(g).max(1e-4)
}

fn smooth(e0: f32, e1: f32, x: f32) -> f32 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The tone curve on display luminance (`detail` = clarity's local detail).
pub fn tone_curve(g: &Grading, l: f32, detail: f32) -> f32 {
    let p = |v: f32| v.clamp(-100.0, 100.0) / 100.0;
    let mut y = l + p(g.clarity) * detail * (1.0 - (2.0 * l - 1.0).powi(2)).max(0.0) * 1.5;
    y += p(g.shadows) * 0.35 * (1.0 - smooth(0.0, 0.55, y)) * (1.0 - y);
    y += p(g.highlights) * 0.35 * smooth(0.45, 1.0, y) * y.min(1.0);
    let (bp, wp) = (-p(g.blacks) * 0.12, 1.0 - p(g.whites) * 0.12);
    (y - bp) / (wp - bp).max(1e-3)
}

/// Integer hash to 0..1.
fn hash(x: u32, y: u32, seed: u32) -> f32 {
    let mut h = x.wrapping_mul(0x8da6_b343) ^ y.wrapping_mul(0xd816_3841) ^ seed.wrapping_mul(0xcb1a_b31f);
    h ^= h >> 13;
    h = h.wrapping_mul(0x5bd1_e995);
    h ^= h >> 15;
    (h & 0x00ff_ffff) as f32 / 16_777_216.0
}

fn bilinear(img: &[Vec4], w: usize, h: usize, p: Vec2) -> Vec4 {
    let p = p.clamp(Vec2::ZERO, Vec2::new(w as f32 - 1.0, h as f32 - 1.0));
    let (x0, y0) = (p.x.floor() as usize, p.y.floor() as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = (p.x - x0 as f32, p.y - y0 as f32);
    let at = |x: usize, y: usize| img[y * w + x];
    at(x0, y0).lerp(at(x1, y0), fx).lerp(at(x0, y1).lerp(at(x1, y1), fx), fy)
}

/// Box blur of a scalar image (two passes ≈ a soft Gaussian).
pub fn box_blur(src: &[f32], w: usize, h: usize, r: usize) -> Vec<f32> {
    let pass = |src: &[f32], horizontal: bool| -> Vec<f32> {
        let (n, lines) = if horizontal { (w, h) } else { (h, w) };
        let idx = |line: usize, i: usize| if horizontal { line * w + i } else { i * w + line };
        let mut out = vec![0.0; src.len()];
        for line in 0..lines {
            let mut sum = 0.0;
            let get = |i: isize| src[idx(line, i.clamp(0, n as isize - 1) as usize)];
            for i in -(r as isize)..=(r as isize) {
                sum += get(i);
            }
            for i in 0..n {
                out[idx(line, i)] = sum / (2 * r + 1) as f32;
                sum += get(i as isize + r as isize + 1) - get(i as isize - r as isize);
            }
        }
        out
    };
    let mut img = src.to_vec();
    for _ in 0..2 {
        img = pass(&img, true);
        img = pass(&img, false);
    }
    img
}

/// Grades a display image (sRGB 0..1, straight alpha) in place.
pub fn grade(img: &mut [Vec4], w: usize, h: usize, g: &Grading) {
    if g.is_neutral() || w == 0 || h == 0 || img.len() != w * h {
        return;
    }
    let center = Vec2::new(w as f32 - 1.0, h as f32 - 1.0) * 0.5;
    let half = center.max(Vec2::ONE);
    // Chromatic aberration: red pushed out, blue pulled in, radially.
    let ca = g.chromatic_aberration.clamp(0.0, 100.0) / 100.0 * 0.006;
    if ca > 0.0 {
        let src = img.to_vec();
        for (i, px) in img.iter_mut().enumerate() {
            let p = Vec2::new((i % w) as f32, (i / w) as f32);
            let d = p - center;
            px.x = bilinear(&src, w, h, center + d * (1.0 + ca)).x;
            px.z = bilinear(&src, w, h, center + d * (1.0 - ca)).z;
        }
    }
    let wb = white_balance(g.temperature, g.tint);
    let balance = (wb - Vec3::ONE).abs().max_element() > 1e-4;
    if balance {
        for px in img.iter_mut() {
            let lin = px.truncate().map(srgb_to_linear) * wb;
            *px = lin.map(|c| linear_to_srgb(c.max(0.0))).extend(px.w);
        }
    }
    // Clarity: detail against a blur a little over 1% of the width.
    let detail = if g.clarity != 0.0 {
        let l: Vec<f32> = img.iter().map(|p| luma(p.truncate())).collect();
        let blur = box_blur(&l, w, h, (w / 90).max(2));
        l.iter().zip(blur).map(|(a, b)| a - b).collect()
    } else {
        vec![]
    };
    let (sat, vib) = (g.saturation.clamp(-100.0, 100.0) / 100.0, g.vibrance.clamp(-100.0, 100.0) / 100.0);
    let wheels = [g.shadow_wheel.shift(), g.midtone_wheel.shift(), g.highlight_wheel.shift()];
    let vig = g.vignette.clamp(-100.0, 100.0) / 100.0;
    let mid = g.vignette_midpoint.clamp(0.0, 100.0) / 100.0;
    let grain = g.grain.clamp(0.0, 100.0) / 100.0 * 0.18;
    for (i, px) in img.iter_mut().enumerate() {
        let mut c = px.truncate();
        let l = luma(c);
        let t = tone_curve(g, l, detail.get(i).copied().unwrap_or(0.0));
        c = if l > 1e-4 { c * (t / l) } else { Vec3::splat(t) };
        let l = luma(c);
        let spread = c.max_element() - c.min_element();
        c = Vec3::splat(l) + (c - Vec3::splat(l)) * ((1.0 + sat) * (1.0 + vib * (1.0 - spread.clamp(0.0, 1.0)))).max(0.0);
        let lc = l.clamp(0.0, 1.0);
        let weights = [(1.0 - lc).powi(2), 4.0 * lc * (1.0 - lc), lc * lc];
        c += wheels[0] * weights[0] * 0.3 + wheels[1] * weights[1] * 0.3 + wheels[2] * weights[2] * 0.3;
        if vig != 0.0 {
            let d = ((Vec2::new((i % w) as f32, (i / w) as f32) - center) / half).length() / std::f32::consts::SQRT_2;
            let f = smooth(mid, 1.0 + mid * 0.25, d * (1.0 + 0.5 * (1.0 - mid)));
            c = if vig < 0.0 { c * (1.0 + vig * f) } else { c + (Vec3::ONE - c) * vig * f };
        }
        if grain > 0.0 {
            let n = hash((i % w) as u32, (i / w) as u32, g.grain_seed) - 0.5;
            c += Vec3::splat(n * grain * (0.35 + 2.6 * lc * (1.0 - lc)));
        }
        *px = c.clamp(Vec3::ZERO, Vec3::ONE).extend(px.w);
    }
}
