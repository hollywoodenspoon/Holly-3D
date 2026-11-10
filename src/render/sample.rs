//! Camera sampling: stratified anti-aliasing per pixel, producing the pixel's
//! layers (premultiplied geometry, background, glow) plus its display / HDR
//! values and the G-buffer sample the post passes use.

use super::post::{PixelLayers, compose};
use super::trace::{GSample, Rng, Tracer};
use bevy::math::{Vec2, Vec3, Vec4};

/// One rendered pixel: display sRGB + alpha, linear HDR, layers and G-buffer.
#[derive(Clone, Copy, Debug)]
pub struct Pixel {
    pub display: Vec4,
    pub hdr: Vec4,
    pub g: GSample,
    pub layers: PixelLayers,
}

impl Pixel {
    pub const EMPTY: Pixel = Pixel { display: Vec4::ZERO, hdr: Vec4::ZERO, g: GSample::EMPTY, layers: PixelLayers::ZERO };

    /// Recomputes display + HDR from the layers.
    pub fn finish(&mut self, tone: &super::post::Tone, plain_bg: bool, transparent: bool) {
        self.display = compose(&self.layers, tone, plain_bg, transparent);
        self.hdr = self.layers.hdr(transparent);
    }
}

impl Tracer<'_> {
    /// True when the background is the plain Environment color.
    pub fn plain_background(&self) -> bool {
        self.rs.hdri.is_none()
    }

    /// Renders pixel (x, y) of a `w` x `h` image.
    pub fn pixel(&self, x: u32, y: u32, w: u32, h: u32) -> Pixel {
        let n = self.cfg.aa.max(1);
        let aspect = w as f32 / h as f32;
        let mut rng = Rng::new(((y as u64) << 32) ^ x as u64 ^ 0xA5A5_5A5A);
        let (mut geom, mut back) = (Vec4::ZERO, Vec3::ZERO);
        let mut g = GSample::EMPTY;
        for sy in 0..n {
            for sx in 0..n {
                let jx = (sx as f32 + if n > 1 { rng.f() } else { 0.5 }) / n as f32;
                let jy = (sy as f32 + if n > 1 { rng.f() } else { 0.5 }) / n as f32;
                let ndc = Vec2::new((x as f32 + jx) / w as f32 * 2.0 - 1.0, 1.0 - (y as f32 + jy) / h as f32 * 2.0);
                let ray = self.rs.camera.ray(ndc, aspect);
                // `c` is premultiplied by the coverage `a`.
                let (c, a, gs) = self.trace(ray.origin, ray.dir, 0, &mut rng);
                if sx == n / 2 && sy == n / 2 {
                    g = gs.unwrap_or(GSample::EMPTY);
                }
                geom += c.extend(a);
                back += self.backdrop(ray.dir);
            }
        }
        let k = 1.0 / (n * n) as f32;
        let mut px = Pixel { g, layers: PixelLayers { geom: geom * k, back: back * k, ..PixelLayers::ZERO }, ..Pixel::EMPTY };
        px.finish(&self.cfg.post.tone, self.plain_background(), self.cfg.transparent);
        px
    }
}
