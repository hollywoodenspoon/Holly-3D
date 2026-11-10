//! Rendered Frame Window image: display sRGB (straight alpha) + linear HDR,
//! channel views (R/G/B/A/Mono) and PNG / JPG / EXR export.

use super::post::{PixelLayers, Tone, compose};
use bevy::math::{Vec3, Vec4};

#[derive(Clone, Debug, PartialEq)]
pub struct FrameBuffer {
    pub width: u32,
    pub height: u32,
    /// Tonemapped sRGB 0..1 + straight alpha.
    pub display: Vec<Vec4>,
    /// Linear radiance + alpha (EXR output).
    pub hdr: Vec<Vec4>,
    /// Bumped on every write so the UI re-uploads its texture.
    pub rev: u64,
    /// Built-in raytracer layers, kept so the F9 tone controls (operator,
    /// Exposure, Contrast) re-tonemap without re-rendering. Empty for Cycles.
    pub layers: Vec<PixelLayers>,
    /// The background is the plain Environment color.
    pub plain_bg: bool,
    pub transparent: bool,
}

/// RFW channel toggles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelView {
    pub r: bool,
    pub g: bool,
    pub b: bool,
    /// Show the alpha channel as grayscale.
    pub alpha: bool,
    pub mono: bool,
}

impl Default for ChannelView {
    fn default() -> Self {
        Self { r: true, g: true, b: true, alpha: false, mono: false }
    }
}

impl FrameBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let n = (width * height) as usize;
        Self {
            width,
            height,
            display: vec![Vec4::ZERO; n],
            hdr: vec![Vec4::ZERO; n],
            rev: 0,
            layers: vec![],
            plain_bg: true,
            transparent: false,
        }
    }

    /// Writes a block of rows `y0..` covering columns `x0..x0+w`.
    pub fn put_block(&mut self, x0: u32, y0: u32, w: u32, display: &[Vec4], hdr: &[Vec4]) {
        if w == 0 {
            return;
        }
        for (r, (drow, hrow)) in display.chunks(w as usize).zip(hdr.chunks(w as usize)).enumerate() {
            let y = y0 + r as u32;
            if y >= self.height {
                break;
            }
            let start = (y * self.width + x0) as usize;
            let n = drow.len().min((self.width - x0) as usize);
            self.display[start..start + n].copy_from_slice(&drow[..n]);
            self.hdr[start..start + n].copy_from_slice(&hrow[..n]);
        }
        self.rev += 1;
    }

    /// Stores layers for a block (same layout as [`Self::put_block`]).
    pub fn put_layers(&mut self, x0: u32, y0: u32, w: u32, layers: &[PixelLayers]) {
        let n = (self.width * self.height) as usize;
        if self.layers.len() != n {
            self.layers = vec![PixelLayers::ZERO; n];
        }
        for (r, row) in layers.chunks(w.max(1) as usize).enumerate() {
            let y = y0 + r as u32;
            if y >= self.height {
                break;
            }
            let start = (y * self.width + x0) as usize;
            let k = row.len().min((self.width - x0) as usize);
            self.layers[start..start + k].copy_from_slice(&row[..k]);
        }
    }

    /// True when the image can be re-tonemapped live.
    pub fn can_retone(&self) -> bool {
        self.layers.len() == self.display.len() && !self.layers.is_empty()
    }

    /// Re-runs the compositor with new tone controls (no re-render).
    pub fn retone(&mut self, tone: &Tone) {
        if !self.can_retone() {
            return;
        }
        for (d, l) in self.display.iter_mut().zip(&self.layers) {
            *d = compose(l, tone, self.plain_bg, self.transparent);
        }
        self.rev += 1;
    }

    /// RGBA8 straight-alpha pixels (file output).
    pub fn rgba8(&self) -> Vec<u8> {
        self.display.iter().flat_map(|p| p.to_array().map(|c| (c.clamp(0.0, 1.0) * 255.0 + 0.5) as u8)).collect()
    }

    /// RGBA8 view for the window: channel filter, composited over a checker
    /// where the image is transparent.
    pub fn view_rgba8(&self, ch: ChannelView) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.display.len() * 4);
        for (i, p) in self.display.iter().enumerate() {
            let (x, y) = (i as u32 % self.width.max(1), i as u32 / self.width.max(1));
            let checker = if (x / 8 + y / 8) % 2 == 0 { 0.32 } else { 0.22 };
            let c = if ch.alpha {
                Vec3::splat(p.w)
            } else {
                let mut c = p.truncate() * p.w + Vec3::splat(checker) * (1.0 - p.w);
                let mask = Vec3::new(ch.r as u8 as f32, ch.g as u8 as f32, ch.b as u8 as f32);
                c *= mask;
                if ch.mono {
                    let n = mask.x + mask.y + mask.z;
                    let l = if n > 0.0 { (c.x + c.y + c.z) / n } else { 0.0 };
                    c = Vec3::splat(l);
                }
                c
            };
            out.extend(c.to_array().map(|v| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8));
            out.push(255);
        }
        out
    }

    /// Saves by extension: `.png` (RGBA8), `.jpg` / `.jpeg` (RGB8), `.exr` (RGBA32F linear).
    pub fn save(&self, path: &str) -> Result<(), String> {
        let ext = std::path::Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
        let (w, h) = (self.width, self.height);
        let r = match ext.as_str() {
            "png" => image::save_buffer(path, &self.rgba8(), w, h, image::ExtendedColorType::Rgba8),
            "jpg" | "jpeg" => {
                let rgb: Vec<u8> = self.rgba8().chunks(4).flat_map(|p| [p[0], p[1], p[2]]).collect();
                image::save_buffer(path, &rgb, w, h, image::ExtendedColorType::Rgb8)
            }
            "exr" => {
                let data: Vec<f32> = self.hdr.iter().flat_map(|p| p.to_array()).collect();
                match image::Rgba32FImage::from_raw(w, h, data) {
                    Some(img) => img.save(path),
                    None => return Err("EXR buffer size mismatch".into()),
                }
            }
            _ => return Err(format!("Unknown image type '.{ext}' — use .png, .jpg or .exr")),
        };
        r.map_err(|e| e.to_string())
    }

    /// Loads an 8-bit image (Cycles output) into a frame buffer.
    pub fn from_image(img: image::DynamicImage) -> Self {
        let rgba = img.to_rgba32f();
        let (w, h) = rgba.dimensions();
        let mut fb = Self::new(w, h);
        for (i, p) in rgba.pixels().enumerate() {
            let v = Vec4::from_array(p.0);
            fb.display[i] = v;
            let l = Vec3::from_array([v.x, v.y, v.z].map(crate::materials::shading::srgb_to_linear));
            fb.hdr[i] = l.extend(v.w);
        }
        fb.rev = 1;
        fb
    }
}
