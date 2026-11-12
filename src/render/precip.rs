//! PR 8: F9 precipitation overlay. Rain streaks, snowflakes and ash with
//! glowing cinders are scattered at seeded depths in front of the camera
//! and composited per pixel only where they are nearer than the geometry
//! (G-buffer depth), so a wall hides the rain behind it. Near particles
//! are bigger and fall faster (longer streaks): simple parallax, no
//! particle simulation.

use super::sample::Pixel;
use super::weather::{Precip, Weather};
use bevy::math::{Vec2, Vec3};
use rand::{Rng, SeedableRng};

/// Particle depth range (scene units in front of the camera).
const NEAR: f32 = 1.5;
const FAR: f32 = 40.0;

struct Drop {
    at: Vec2,
    depth: f32,
}

fn drops(w: usize, h: usize, wx: &Weather) -> Vec<Drop> {
    let per_px = match wx.precip {
        Precip::None => 0.0,
        Precip::Rain => 1.0 / 260.0,
        Precip::Snow => 1.0 / 420.0,
        Precip::Ash => 1.0 / 650.0,
    };
    let n = ((w * h) as f32 * per_px * wx.amount.clamp(0.0, 1.0)).round() as usize;
    let mut r = rand_chacha::ChaCha8Rng::seed_from_u64(0x9EA7 ^ wx.seed as u64);
    (0..n)
        .map(|_| {
            let at = Vec2::new(r.gen_range(0.0..w as f32), r.gen_range(0.0..h as f32));
            // More particles far away (uniform in volume ~ depth^2).
            let depth = NEAR + (FAR - NEAR) * r.gen_range(0.0f32..1.0).sqrt();
            Drop { at, depth }
        })
        .collect()
}

/// Blends `col` at `alpha` over pixel `p` if the particle is in front.
fn put(p: &mut Pixel, depth: f32, col: Vec3, alpha: f32, glow: Vec3) {
    if depth >= p.g.depth || alpha <= 0.0 {
        return;
    }
    let a = alpha.min(1.0);
    let l = &mut p.layers;
    let cov = l.geom.w;
    let g = l.geom.truncate() * (1.0 - a) + col * a * cov;
    l.geom = g.extend(cov);
    l.back = l.back * (1.0 - a) + col * a;
    l.glow += glow * a;
}

/// Draws the precipitation over a finished (pre-compose) image.
pub fn overlay(px: &mut [Pixel], w: usize, h: usize, wx: &Weather) {
    if wx.precip == Precip::None || w == 0 || h == 0 {
        return;
    }
    let scale = h as f32 / 540.0;
    let fall = Vec2::new(wx.slant.clamp(-60.0, 60.0).to_radians().sin(), 1.0).normalize();
    for (i, d) in drops(w, h, wx).into_iter().enumerate() {
        let near = (6.0 / d.depth).min(3.0);
        match wx.precip {
            Precip::Rain => {
                let len = (14.0 * near * scale).max(2.0);
                let steps = len.ceil() as usize;
                let col = Vec3::splat(0.75);
                for i in 0..steps {
                    let q = d.at + fall * i as f32;
                    if let Some(p) = at(px, w, h, q) {
                        let fade = 1.0 - (i as f32 / steps as f32 - 0.5).abs();
                        put(p, d.depth, col, 0.18 * near.min(1.5) * fade, Vec3::ZERO);
                    }
                }
            }
            Precip::Snow => disc(px, w, h, &d, (1.6 * near * scale).max(0.7), Vec3::splat(0.95), 0.85, Vec3::ZERO),
            Precip::Ash => {
                let cinder = i % 5 == 0;
                let (col, glow) = if cinder { (Vec3::new(1.0, 0.45, 0.1), Vec3::new(6.0, 2.0, 0.4)) } else { (Vec3::splat(0.12), Vec3::ZERO) };
                let size = if cinder { 0.8 } else { 1.3 };
                disc(px, w, h, &d, (size * near * scale).max(0.6), col, 0.8, glow);
            }
            Precip::None => {}
        }
    }
}

fn at(px: &mut [Pixel], w: usize, h: usize, q: Vec2) -> Option<&mut Pixel> {
    let (x, y) = (q.x.floor() as isize, q.y.floor() as isize);
    (x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h).then(|| &mut px[y as usize * w + x as usize])
}

#[allow(clippy::too_many_arguments)]
fn disc(px: &mut [Pixel], w: usize, h: usize, d: &Drop, rad: f32, col: Vec3, alpha: f32, glow: Vec3) {
    let r = rad.ceil() as isize;
    for dy in -r..=r {
        for dx in -r..=r {
            let o = Vec2::new(dx as f32, dy as f32);
            let k = 1.0 - (o.length() / (rad + 0.5)).min(1.0);
            if let Some(p) = at(px, w, h, d.at + o) {
                put(p, d.depth, col, alpha * k * k.sqrt(), glow);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::render::post::PixelLayers;
    use crate::render::trace::GSample;
    use bevy::math::Vec4;

    fn image(w: usize, h: usize, depth: f32) -> Vec<Pixel> {
        let g = GSample { depth, ..GSample::EMPTY };
        let layers = PixelLayers { geom: Vec4::new(0.3, 0.3, 0.3, 1.0), ..PixelLayers::ZERO };
        vec![Pixel { display: Vec4::ZERO, hdr: Vec4::ZERO, g, layers }; w * h]
    }

    fn changed(px: &[Pixel]) -> usize {
        px.iter().filter(|p| p.layers.geom.truncate() != Vec3::splat(0.3) || p.layers.glow != Vec3::ZERO).count()
    }

    #[test]
    fn precipitation_is_drawn_and_hidden_behind_near_geometry() {
        let (w, h) = (160, 90);
        for precip in [Precip::Rain, Precip::Snow, Precip::Ash] {
            let wx = Weather { precip, amount: 1.0, ..Default::default() };
            let mut far = image(w, h, f32::INFINITY);
            overlay(&mut far, w, h, &wx);
            assert!(changed(&far) > 50, "{precip:?}: {}", changed(&far));
            // A wall 1 unit away hides every particle.
            let mut wall = image(w, h, 1.0);
            overlay(&mut wall, w, h, &wx);
            assert_eq!(changed(&wall), 0, "{precip:?}");
        }
        let mut none = image(w, h, f32::INFINITY);
        overlay(&mut none, w, h, &Weather::default());
        assert_eq!(changed(&none), 0);
    }

    #[test]
    fn rain_brightens_snow_whitens_ash_darkens_and_cinders_glow() {
        let (w, h) = (160, 90);
        let mean = |precip| {
            let mut px = image(w, h, f32::INFINITY);
            overlay(&mut px, w, h, &Weather { precip, amount: 1.0, ..Default::default() });
            let glow = px.iter().any(|p| p.layers.glow.x > 0.0);
            (px.iter().map(|p| p.layers.geom.z).sum::<f32>() / px.len() as f32, glow)
        };
        let (rain, _) = mean(Precip::Rain);
        let (snow, _) = mean(Precip::Snow);
        let (ash, ash_glow) = mean(Precip::Ash);
        assert!(rain > 0.3 && snow > 0.3 && ash < 0.3, "{rain} {snow} {ash}");
        assert!(ash_glow, "glowing cinders");
        let more = {
            let mut px = image(w, h, f32::INFINITY);
            overlay(&mut px, w, h, &Weather { precip: Precip::Snow, amount: 0.2, ..Default::default() });
            changed(&px)
        };
        let mut px = image(w, h, f32::INFINITY);
        overlay(&mut px, w, h, &Weather { precip: Precip::Snow, amount: 1.0, ..Default::default() });
        assert!(changed(&px) > more, "Amount scales the particle count");
    }
}
