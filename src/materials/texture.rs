//! RGBA8 (sRGB) texture buffers for bitmap maps and the 3D paint canvas.
//! Shared through `Arc` so undo snapshots stay cheap (copy-on-write paint).

use bevy::math::{Vec2, Vec4};
use std::sync::Arc;

#[derive(Clone, PartialEq)]
pub struct Texture {
    pub width: u32,
    pub height: u32,
    pub data: Vec<[u8; 4]>,
}

impl std::fmt::Debug for Texture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Texture({}x{})", self.width, self.height)
    }
}

impl Texture {
    pub fn new(width: u32, height: u32, fill: [u8; 4]) -> Self {
        let (width, height) = (width.max(1), height.max(1));
        Self { width, height, data: vec![fill; (width * height) as usize] }
    }

    pub fn load(path: &str) -> Result<Self, String> {
        let img = image::open(path).map_err(|e| format!("{path}: {e}"))?.to_rgba8();
        let (width, height) = img.dimensions();
        Ok(Self { width, height, data: img.pixels().map(|p| p.0).collect() })
    }

    pub fn texel(&self, x: i64, y: i64) -> [u8; 4] {
        let x = x.rem_euclid(self.width as i64) as u32;
        let y = y.rem_euclid(self.height as i64) as u32;
        self.data[(y * self.width + x) as usize]
    }

    /// Bilinear, repeating lookup. UV v points up (row 0 is the top, v = 1).
    pub fn sample(&self, uv: Vec2) -> Vec4 {
        let x = uv.x * self.width as f32 - 0.5;
        let y = (1.0 - uv.y) * self.height as f32 - 0.5;
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let t = |dx: i64, dy: i64| Vec4::from_array(self.texel(x0 as i64 + dx, y0 as i64 + dy).map(|c| c as f32 / 255.0));
        let top = t(0, 0).lerp(t(1, 0), fx);
        let bottom = t(0, 1).lerp(t(1, 1), fx);
        top.lerp(bottom, fy)
    }

    /// Pixel coordinates of a UV (wrapped into the image).
    pub fn pixel_of(&self, uv: Vec2) -> Vec2 {
        Vec2::new(uv.x.rem_euclid(1.0) * self.width as f32, (1.0 - uv.y.rem_euclid(1.0)) * self.height as f32)
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.data.iter().flatten().copied().collect()
    }
}

/// Shared texture; equality is identity + revision so comparisons stay cheap.
#[derive(Clone, Debug)]
pub struct TexHandle {
    pub tex: Arc<Texture>,
    /// Bumped by every paint stroke so the viewport re-uploads.
    pub rev: u64,
}

impl PartialEq for TexHandle {
    fn eq(&self, o: &Self) -> bool {
        Arc::ptr_eq(&self.tex, &o.tex) && self.rev == o.rev
    }
}

impl TexHandle {
    pub fn new(t: Texture) -> Self {
        Self { tex: Arc::new(t), rev: crate::scene::next_version() }
    }
    /// Copy-on-write access for painting (undo snapshots keep the old pixels).
    pub fn make_mut(&mut self) -> &mut Texture {
        self.rev = crate::scene::next_version();
        Arc::make_mut(&mut self.tex)
    }
}
