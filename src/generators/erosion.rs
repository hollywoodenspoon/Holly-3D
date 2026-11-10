//! Square heightmap with fast thermal (talus slumping) and hydraulic
//! (seeded rain droplets that erode and deposit sediment) erosion passes.
//! Hydraulic erosion carves branching drainage gullies; thermal erosion
//! relaxes over-steep slopes into talus aprons.

use super::hash;

#[derive(Clone, Debug, PartialEq)]
pub struct Heightmap {
    /// Samples per side (segments + 1).
    pub n: usize,
    /// Row-major heights (row = Y), normalized roughly 0..1.
    pub h: Vec<f32>,
}

impl Heightmap {
    /// Samples `f(u, v)` (u, v in 0..1) on an `n` x `n` lattice.
    pub fn from_fn(n: usize, f: impl Fn(f32, f32) -> f32) -> Self {
        let n = n.max(2);
        let d = (n - 1) as f32;
        let h = (0..n * n).map(|k| f((k % n) as f32 / d, (k / n) as f32 / d)).collect();
        Self { n, h }
    }

    pub fn at(&self, i: usize, j: usize) -> f32 {
        self.h[j.min(self.n - 1) * self.n + i.min(self.n - 1)]
    }

    /// Bilinear sample at u, v in 0..1.
    pub fn sample(&self, u: f32, v: f32) -> f32 {
        let d = (self.n - 1) as f32;
        let (x, y) = (u.clamp(0.0, 1.0) * d, v.clamp(0.0, 1.0) * d);
        let (i, j) = (x.floor() as usize, y.floor() as usize);
        let (tx, ty) = (x - i as f32, y - j as f32);
        let top = self.at(i, j) + (self.at(i + 1, j) - self.at(i, j)) * tx;
        let bot = self.at(i, j + 1) + (self.at(i + 1, j + 1) - self.at(i, j + 1)) * tx;
        top + (bot - top) * ty
    }

    /// Largest height difference between 4-neighbours (slope proxy).
    pub fn max_step(&self) -> f32 {
        let n = self.n;
        let mut m: f32 = 0.0;
        for j in 0..n {
            for i in 0..n {
                if i + 1 < n {
                    m = m.max((self.at(i, j) - self.at(i + 1, j)).abs());
                }
                if j + 1 < n {
                    m = m.max((self.at(i, j) - self.at(i, j + 1)).abs());
                }
            }
        }
        m
    }

    /// Talus slumping: material above the `talus` step (per cell) slides to
    /// lower neighbours, `iters` times.
    pub fn thermal(&mut self, iters: u32, talus: f32) {
        let n = self.n as i32;
        for _ in 0..iters {
            let mut delta = vec![0.0f32; self.h.len()];
            for j in 0..n {
                for i in 0..n {
                    let k = (j * n + i) as usize;
                    let h = self.h[k];
                    for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                        let (x, y) = (i + di, j + dj);
                        if x < 0 || y < 0 || x >= n || y >= n {
                            continue;
                        }
                        let kk = (y * n + x) as usize;
                        let d = h - self.h[kk];
                        if d > talus {
                            let m = (d - talus) * 0.12;
                            delta[k] -= m;
                            delta[kk] += m;
                        }
                    }
                }
            }
            self.h.iter_mut().zip(&delta).for_each(|(h, d)| *h += d);
        }
    }

    fn gradient(&self, x: f32, y: f32) -> (f32, f32, f32) {
        let (i, j) = (x.floor() as usize, y.floor() as usize);
        let (u, v) = (x - i as f32, y - j as f32);
        let (a, b, c, d) = (self.at(i, j), self.at(i + 1, j), self.at(i, j + 1), self.at(i + 1, j + 1));
        let gx = (b - a) * (1.0 - v) + (d - c) * v;
        let gy = (c - a) * (1.0 - u) + (d - b) * u;
        (gx, gy, a * (1.0 - u) * (1.0 - v) + b * u * (1.0 - v) + c * (1.0 - u) * v + d * u * v)
    }

    fn deposit(&mut self, x: f32, y: f32, amount: f32) {
        let (i, j) = (x.floor() as usize, y.floor() as usize);
        let (u, v) = (x - i as f32, y - j as f32);
        let n = self.n;
        for (di, dj, w) in [(0, 0, (1.0 - u) * (1.0 - v)), (1, 0, u * (1.0 - v)), (0, 1, (1.0 - u) * v), (1, 1, u * v)] {
            let (a, b) = ((i + di).min(n - 1), (j + dj).min(n - 1));
            self.h[b * n + a] += amount * w;
        }
    }

    /// Rain droplets (`drops` of them, seeded) run downhill picking up
    /// sediment on steep fast stretches and dropping it where they slow,
    /// carving gullies. `strength` scales the erosion rate.
    pub fn hydraulic(&mut self, seed: u32, drops: u32, strength: f32) {
        let d = (self.n - 1) as f32;
        let (inertia, capacity, deposit, erode, evaporate) = (0.08, 6.0, 0.25, 0.35 * strength, 0.03);
        for k in 0..drops {
            let (mut x, mut y) = (hash(seed, k * 2) * (d - 1.0), hash(seed, k * 2 + 1) * (d - 1.0));
            let (mut dx, mut dy, mut speed, mut water, mut sed) = (0.0f32, 0.0f32, 1.0f32, 1.0f32, 0.0f32);
            for _ in 0..48 {
                let (gx, gy, h0) = self.gradient(x, y);
                dx = dx * inertia - gx * (1.0 - inertia);
                dy = dy * inertia - gy * (1.0 - inertia);
                let len = (dx * dx + dy * dy).sqrt();
                if len < 1e-9 {
                    break;
                }
                (dx, dy) = (dx / len, dy / len);
                let (nx, ny) = (x + dx, y + dy);
                if nx < 0.0 || ny < 0.0 || nx >= d || ny >= d {
                    break;
                }
                let h1 = self.gradient(nx, ny).2;
                let dh = h1 - h0;
                let cap = (-dh).max(0.002) * speed * water * capacity;
                if sed > cap || dh > 0.0 {
                    let amt = if dh > 0.0 { dh.min(sed) } else { (sed - cap) * deposit };
                    sed -= amt;
                    self.deposit(x, y, amt);
                } else {
                    let amt = ((cap - sed) * erode).min(-dh);
                    sed += amt;
                    self.deposit(x, y, -amt);
                }
                speed = (speed * speed + dh.abs() * 4.0).sqrt().min(4.0);
                water *= 1.0 - evaporate;
                (x, y) = (nx, ny);
            }
        }
    }

    /// Erosion pass used by the Terrain generator: hydraulic droplets
    /// proportional to the map area, then thermal talus relaxation.
    pub fn erode(&mut self, seed: u32, strength: f32) {
        let s = strength.clamp(0.0, 1.0);
        if s <= 0.0 {
            return;
        }
        let drops = (self.h.len() as f32 * 1.5 * s) as u32;
        self.hydraulic(seed ^ 0xE205, drops, 0.5 + s);
        let talus = 3.0 / (self.n - 1) as f32;
        self.thermal((6.0 + 24.0 * s) as u32, talus);
    }
}
