//! A square RGBA float image that repeats in U and V: what map nodes and
//! procedural surfaces are cooked into, and what the image filters read.

use bevy::math::{Vec2, Vec4};

pub struct Grid {
    pub size: u32,
    pub data: Vec<[f32; 4]>,
}

/// Runs `f(index)` for `0..n` on all cores and collects the results in order.
pub fn par_map<T: Send>(n: usize, f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let threads = std::thread::available_parallelism().map_or(1, |t| t.get()).min(16);
    if threads <= 1 || n < 4096 {
        return (0..n).map(f).collect();
    }
    let chunk = n.div_ceil(threads);
    let f = &f;
    std::thread::scope(|s| {
        let jobs: Vec<_> = (0..threads).map(|t| s.spawn(move || (t * chunk..((t + 1) * chunk).min(n)).map(f).collect::<Vec<T>>())).collect();
        jobs.into_iter().flat_map(|j| j.join().expect("cook thread")).collect()
    })
}

impl Grid {
    /// Evaluates `f` at every pixel centre (on all cores).
    pub fn bake(size: u32, f: impl Fn(Vec2) -> Vec4 + Sync) -> Grid {
        let n = size.max(1) as usize;
        let data = par_map(n * n, |i| f(Vec2::new(((i % n) as f32 + 0.5) / n as f32, ((i / n) as f32 + 0.5) / n as f32)).to_array());
        Grid { size: n as u32, data }
    }

    /// A grayscale grid from one value per pixel.
    pub fn gray_from(size: u32, values: &[f32]) -> Grid {
        Grid { size, data: values.iter().map(|v| [*v, *v, *v, 1.0]).collect() }
    }

    pub fn texel(&self, x: i64, y: i64) -> Vec4 {
        let n = self.size as i64;
        Vec4::from_array(self.data[(y.rem_euclid(n) * n + x.rem_euclid(n)) as usize])
    }

    /// Luminance of a pixel (the height of a grayscale map).
    pub fn gray(&self, x: i64, y: i64) -> f32 {
        let c = self.texel(x, y);
        c.x * 0.2126 + c.y * 0.7152 + c.z * 0.0722
    }

    /// Bilinear, repeating lookup.
    pub fn sample(&self, uv: Vec2) -> Vec4 {
        let (x, y) = (uv.x * self.size as f32 - 0.5, uv.y * self.size as f32 - 0.5);
        let (x0, y0) = (x.floor(), y.floor());
        let (fx, fy) = (x - x0, y - y0);
        let t = |dx: i64, dy: i64| self.texel(x0 as i64 + dx, y0 as i64 + dy);
        t(0, 0).lerp(t(1, 0), fx).lerp(t(0, 1).lerp(t(1, 1), fx), fy)
    }

    pub fn bytes(&self) -> usize {
        self.data.len() * 16
    }
}

/// Wrapping Gaussian blur of a `size` × `size` scalar field (`sigma` in pixels).
pub fn gaussian(size: u32, v: &[f32], sigma: f32) -> Vec<f32> {
    if sigma < 0.05 {
        return v.to_vec();
    }
    let n = size as i64;
    let r = (sigma * 3.0).ceil() as i64;
    let k: Vec<f32> = (-r..=r).map(|i| (-(i * i) as f32 / (2.0 * sigma * sigma)).exp()).collect();
    let sum: f32 = k.iter().sum();
    let pass = |src: &[f32], horizontal: bool| {
        par_map(src.len(), |i| {
            let (x, y) = (i as i64 % n, i as i64 / n);
            let at = |d: i64| if horizontal { src[(y * n + (x + d).rem_euclid(n)) as usize] } else { src[((y + d).rem_euclid(n) * n + x) as usize] };
            (-r..=r).map(|d| at(d) * k[(d + r) as usize]).sum::<f32>() / sum
        })
    };
    pass(&pass(v, true), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grids_wrap_and_blur_keeps_the_mean() {
        let g = Grid::bake(8, |uv| Vec4::new(uv.x, uv.y, 0.0, 1.0));
        assert_eq!(g.texel(-1, 0), g.texel(7, 0));
        assert_eq!(g.texel(3, 8), g.texel(3, 0));
        assert!((g.sample(Vec2::new(0.5, 0.5)).x - 0.5).abs() < 1e-4);
        assert_eq!(g.bytes(), 8 * 8 * 16);
        let v: Vec<f32> = (0..64 * 64).map(|i| if i % 64 == 0 { 1.0 } else { 0.0 }).collect();
        let b = gaussian(64, &v, 2.0);
        assert!((b.iter().sum::<f32>() - v.iter().sum::<f32>()).abs() < 0.01);
        // The bright column at x = 0 spreads across the seam to x = 63.
        assert!(b[0] < 1.0 && b[63] > 0.05 && b[32] < 1e-4);
        assert_eq!(par_map(10_000, |i| i * 2)[9_999], 19_998);
    }
}
