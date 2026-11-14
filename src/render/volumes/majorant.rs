//! The majorant grid: the box of a volume split into cells, each with an
//! upper bound of the density inside (delta / ratio tracking take steps
//! sized by it) and, for puff sources, the puffs that reach the cell so a
//! density lookup only visits a few of them.

use super::density::{Source, union};
use bevy::math::Vec3;

pub struct Grid {
    pub lo: Vec3,
    pub hi: Vec3,
    pub dims: [usize; 3],
    /// Upper bound of the 0..=1 density per cell.
    pub max: Vec<f32>,
    /// Puff indices per cell (empty for grid / uniform sources).
    pub lists: Vec<Vec<u32>>,
}

/// Cells per axis: about `target` along the longest side, at least 1.
pub fn dims_for(size: Vec3, target: usize) -> [usize; 3] {
    let cell = size.max_element().max(1e-6) / target as f32;
    let n = |s: f32| ((s / cell).ceil() as usize).clamp(1, 64);
    [n(size.x), n(size.y), n(size.z)]
}

impl Grid {
    pub fn build(src: &Source, lo: Vec3, hi: Vec3, dims: [usize; 3]) -> Self {
        let n = dims[0] * dims[1] * dims[2];
        let mut g = Self { lo, hi, dims, max: vec![0.0; n], lists: vec![Vec::new(); n] };
        match src {
            Source::Uniform => g.max.fill(1.0),
            Source::Grid(v) => {
                for i in 0..n {
                    let (a, b) = g.cell_box(i);
                    let s = (hi - lo).max(Vec3::splat(1e-9));
                    g.max[i] = v.max_in((a - lo) / s, (b - lo) / s);
                }
            }
            Source::Puffs(blobs) => {
                for (bi, b) in blobs.iter().enumerate() {
                    let h = b.half();
                    let (c0, c1) = (g.cell_of(b.c - h), g.cell_of(b.c + h));
                    for z in c0[2]..=c1[2] {
                        for y in c0[1]..=c1[1] {
                            for x in c0[0]..=c1[0] {
                                let i = g.index([x, y, z]);
                                g.lists[i].push(bi as u32);
                            }
                        }
                    }
                }
                for i in 0..n {
                    let (a, b) = g.cell_box(i);
                    g.max[i] = union(g.lists[i].iter().map(|&k| blobs[k as usize].max_in(a, b)));
                }
            }
        }
        g
    }

    pub fn index(&self, c: [usize; 3]) -> usize {
        (c[2] * self.dims[1] + c[1]) * self.dims[0] + c[0]
    }

    fn cell_size(&self) -> Vec3 {
        (self.hi - self.lo) / Vec3::new(self.dims[0] as f32, self.dims[1] as f32, self.dims[2] as f32)
    }

    /// Cell holding `p` (clamped into the grid).
    pub fn cell_of(&self, p: Vec3) -> [usize; 3] {
        let f = (p - self.lo) / self.cell_size().max(Vec3::splat(1e-9));
        let c = |v: f32, n: usize| (v.floor().max(0.0) as usize).min(n - 1);
        [c(f.x, self.dims[0]), c(f.y, self.dims[1]), c(f.z, self.dims[2])]
    }

    pub fn cell_box(&self, i: usize) -> (Vec3, Vec3) {
        let [nx, ny, _] = self.dims;
        let c = Vec3::new((i % nx) as f32, ((i / nx) % ny) as f32, (i / (nx * ny)) as f32);
        let s = self.cell_size();
        (self.lo + c * s, self.lo + (c + 1.0) * s)
    }

    /// Parametric overlap of the ray `o + d t` with the box, within `0..tmax`.
    pub fn clip(&self, o: Vec3, d: Vec3, tmax: f32) -> Option<(f32, f32)> {
        let inv = d.recip();
        let (a, b) = ((self.lo - o) * inv, (self.hi - o) * inv);
        let (t0, t1) = (a.min(b).max_element().max(0.0), a.max(b).min_element().min(tmax));
        (t0 < t1).then_some((t0, t1))
    }

    /// Walks the cells along the ray from `t0` to `t1` (3D DDA), calling
    /// `f(cell, enter, exit)` until it returns false.
    pub fn walk(&self, o: Vec3, d: Vec3, t0: f32, t1: f32, mut f: impl FnMut(usize, f32, f32) -> bool) {
        let s = self.cell_size();
        let mut c = self.cell_of(o + d * (t0 + 1e-4 * (t1 - t0)));
        let mut next = [0.0f32; 3];
        let mut delta = [f32::INFINITY; 3];
        let mut step = [0i64; 3];
        for k in 0..3 {
            if d[k].abs() < 1e-12 {
                next[k] = f32::INFINITY;
                continue;
            }
            step[k] = if d[k] > 0.0 { 1 } else { -1 };
            let edge = self.lo[k] + (c[k] as f32 + if d[k] > 0.0 { 1.0 } else { 0.0 }) * s[k];
            next[k] = (edge - o[k]) / d[k];
            delta[k] = s[k] / d[k].abs();
        }
        let mut t = t0;
        loop {
            let k = if next[0] <= next[1] && next[0] <= next[2] { 0 } else if next[1] <= next[2] { 1 } else { 2 };
            let exit = next[k].min(t1);
            if exit > t && !f(self.index(c), t, exit) {
                return;
            }
            if next[k] >= t1 {
                return;
            }
            t = exit;
            let n = c[k] as i64 + step[k];
            if n < 0 || n >= self.dims[k] as i64 {
                return;
            }
            c[k] = n as usize;
            next[k] += delta[k];
        }
    }
}
