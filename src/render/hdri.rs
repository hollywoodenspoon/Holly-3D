//! HDRI environment (`8`): an equirectangular `.hdr` / `.exr` image used as
//! the F9 background / reflections, optional image-based ambient light, and
//! a skybox in Perspective viewports. Z-up: the image's middle row is the
//! horizon, its center column looks down +X.

use bevy::math::Vec3;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// Linear RGB pixels of an equirectangular image.
pub struct HdrImage {
    pub w: u32,
    pub h: u32,
    pub px: Vec<Vec3>,
}

impl std::fmt::Debug for HdrImage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "HdrImage({}x{})", self.w, self.h)
    }
}

impl HdrImage {
    pub fn load(path: &Path) -> Result<Self, String> {
        let img = image::open(path).map_err(|e| e.to_string())?.to_rgb32f();
        let (w, h) = img.dimensions();
        let px = img.pixels().map(|p| Vec3::from_array(p.0).max(Vec3::ZERO)).collect();
        Ok(Self { w, h, px })
    }

    /// Bilinear sample in direction `d` (world, Z-up), the image turned by
    /// `rotation_deg` about Z.
    pub fn sample(&self, d: Vec3, rotation_deg: f32) -> Vec3 {
        if self.px.is_empty() {
            return Vec3::ZERO;
        }
        let d = d.normalize_or(Vec3::Z);
        let tau = std::f32::consts::TAU;
        let u = (0.5 - d.y.atan2(d.x) / tau + rotation_deg / 360.0).rem_euclid(1.0);
        let v = d.z.clamp(-1.0, 1.0).acos() / std::f32::consts::PI;
        let (fx, fy) = (u * self.w as f32 - 0.5, (v * self.h as f32 - 0.5).clamp(0.0, (self.h - 1) as f32));
        let (x0, y0) = (fx.floor(), fy.floor());
        let (tx, ty) = (fx - x0, fy - y0);
        let at = |x: f32, y: f32| {
            let x = (x as i64).rem_euclid(self.w as i64) as u32;
            let y = (y as u32).min(self.h - 1);
            self.px[(y * self.w + x) as usize]
        };
        let top = at(x0, y0).lerp(at(x0 + 1.0, y0), tx);
        let bot = at(x0, y0 + 1.0).lerp(at(x0 + 1.0, y0 + 1.0), tx);
        top.lerp(bot, ty)
    }

    /// Solid-angle weighted average radiance (the image's ambient term).
    pub fn average(&self) -> Vec3 {
        let (mut sum, mut wsum) = (Vec3::ZERO, 0.0);
        for y in 0..self.h {
            let wt = (std::f32::consts::PI * (y as f32 + 0.5) / self.h as f32).sin();
            for x in 0..self.w {
                sum += self.px[(y * self.w + x) as usize] * wt;
                wsum += wt;
            }
        }
        if wsum > 0.0 { sum / wsum } else { Vec3::ZERO }
    }

    /// Six `size`² RGBA f32 cube faces (+X, -X, +Y, -Y, +Z, -Z) for Bevy's
    /// skybox, which samples `world_dir * (1, 1, -1)`.
    pub fn cube_faces(&self, size: u32, rotation_deg: f32) -> Vec<[f32; 4]> {
        cube_faces_with(size, |d| self.sample(d, rotation_deg))
    }
}

/// Six `size`² RGBA f32 cube faces (+X, -X, +Y, -Y, +Z, -Z) of the radiance
/// `f(world_dir)`, one thread per face.
pub fn cube_faces_with(size: u32, f: impl Fn(Vec3) -> Vec3 + Sync) -> Vec<[f32; 4]> {
    let face = |face: u32| {
        let mut out = Vec::with_capacity((size * size) as usize);
        for j in 0..size {
            for i in 0..size {
                let u = (i as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                let v = (j as f32 + 0.5) / size as f32 * 2.0 - 1.0;
                let s = match face {
                    0 => Vec3::new(1.0, -v, -u),
                    1 => Vec3::new(-1.0, -v, u),
                    2 => Vec3::new(u, 1.0, v),
                    3 => Vec3::new(u, -1.0, -v),
                    4 => Vec3::new(u, -v, 1.0),
                    _ => Vec3::new(-u, -v, -1.0),
                };
                let c = f(s * Vec3::new(1.0, 1.0, -1.0));
                out.push([c.x, c.y, c.z, 1.0]);
            }
        }
        out
    };
    let f = &face;
    std::thread::scope(|sc| {
        let hs: Vec<_> = (0..6).map(|k| sc.spawn(move || f(k))).collect();
        hs.into_iter().flat_map(|h| h.join().expect("cube face thread")).collect()
    })
}

/// The Environment's HDRI map and how it is used.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Hdri {
    pub path: PathBuf,
    pub strength: f32,
    /// Turns the image about the world Z axis (degrees).
    pub rotation: f32,
    /// Adds the image's average radiance to the ambient light (F9).
    pub lighting: bool,
    /// Shows the image behind Perspective viewports.
    pub show_in_viewport: bool,
    #[serde(skip)]
    pub image: Option<Arc<HdrImage>>,
}

impl PartialEq for Hdri {
    fn eq(&self, o: &Self) -> bool {
        let img = match (&self.image, &o.image) {
            (Some(a), Some(b)) => Arc::ptr_eq(a, b),
            (a, b) => a.is_none() == b.is_none(),
        };
        img && self.path == o.path
            && self.strength == o.strength
            && self.rotation == o.rotation
            && self.lighting == o.lighting
            && self.show_in_viewport == o.show_in_viewport
    }
}

impl Hdri {
    /// Loads the image at `path` with default settings.
    pub fn open(path: &Path) -> Result<Self, String> {
        let img = HdrImage::load(path)?;
        Ok(Self::with_image(path.to_path_buf(), img))
    }

    pub fn with_image(path: PathBuf, img: HdrImage) -> Self {
        Self { path, strength: 1.0, rotation: 0.0, lighting: true, show_in_viewport: true, image: Some(Arc::new(img)) }
    }

    /// Reloads the image after a scene is opened (it is not saved inline).
    pub fn reload(&mut self) -> Result<(), String> {
        if self.image.is_none() {
            self.image = Some(Arc::new(HdrImage::load(&self.path)?));
        }
        Ok(())
    }

    /// Background radiance in direction `d` (linear, strength applied).
    pub fn radiance(&self, d: Vec3) -> Option<Vec3> {
        self.image.as_ref().map(|i| i.sample(d, self.rotation) * self.strength)
    }

    /// Makes sure `path` holds the image (a baked procedural sky is written
    /// as Radiance `.hdr` the first time Cycles / Blender needs it).
    pub fn ensure_file(&self) -> Result<(), String> {
        let Some(img) = self.image.as_ref().filter(|_| !self.path.exists()) else { return Ok(()) };
        let px: Vec<image::Rgb<f32>> = img.px.iter().map(|c| image::Rgb(c.to_array())).collect();
        let f = std::fs::File::create(&self.path).map_err(|e| e.to_string())?;
        image::codecs::hdr::HdrEncoder::new(std::io::BufWriter::new(f))
            .encode(&px, img.w as usize, img.h as usize)
            .map_err(|e| e.to_string())
    }

    /// Stable key for caching GPU uploads.
    pub fn key(&self) -> Option<(usize, u32)> {
        self.image.as_ref().map(|i| (Arc::as_ptr(i) as usize, self.rotation.to_bits()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 8x4 image: sky row(s) bright blue, ground dark; one red pixel on the
    /// horizon at the center column (+X).
    fn img() -> HdrImage {
        let (w, h) = (8, 4);
        let mut px: Vec<Vec3> = (0..w * h).map(|i| if i / w < 2 { Vec3::new(0.2, 0.4, 2.0) } else { Vec3::splat(0.1) }).collect();
        px[(w + 3) as usize] = Vec3::new(4.0, 0.0, 0.0);
        px[(w + 4) as usize] = Vec3::new(4.0, 0.0, 0.0);
        HdrImage { w, h, px }
    }

    #[test]
    fn equirect_sampling_rotation_and_average() {
        let i = img();
        assert!(i.sample(Vec3::Z, 0.0).z > 1.9, "zenith is sky");
        assert!(i.sample(-Vec3::Z, 0.0).z < 0.2, "nadir is ground");
        let fwd = i.sample(Vec3::new(1.0, 0.0, 0.2), 0.0);
        assert!(fwd.x > 3.0, "+X looks at the image center: {fwd}");
        assert!(i.sample(Vec3::new(-1.0, 0.0, 0.2), 0.0).x < 1.0);
        // Turning the image 180° brings the red spot behind the camera.
        assert!(i.sample(Vec3::new(-1.0, 0.0, 0.2), 180.0).x > 3.0);
        let avg = i.average();
        assert!(avg.z > 0.5 && avg.z < 2.0 && avg.min_element() > 0.0, "{avg}");
        // Bevy looks up `dir * (1, 1, -1)`: its -Z face is our sky, +Z the ground.
        let f = i.cube_faces(4, 0.0);
        assert_eq!(f.len(), 6 * 16);
        let face = |k: usize| f[k * 16 + 5];
        assert!(face(5)[2] > 1.9 && face(4)[2] < 0.2);
        let h = Hdri::with_image("x.hdr".into(), img());
        assert_eq!(h.radiance(Vec3::Z).map(|c| c.z > 1.9), Some(true));
        let mut c = h.clone();
        assert_eq!(c, h);
        c.strength = 2.0;
        assert_ne!(c, h);
    }
}
