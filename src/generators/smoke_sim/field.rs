//! A dense cell-centred scalar field for the smoke solver, with clamped
//! trilinear sampling and a helper that fills a field slab by slab on all
//! CPU cores (each cell is computed independently, so results don't depend
//! on the thread count).

use bevy::math::Vec3;

#[derive(Clone, Debug, PartialEq)]
pub struct Field {
    pub n: [usize; 3],
    pub d: Vec<f32>,
}

impl Field {
    pub fn new(n: [usize; 3]) -> Self {
        Self { n, d: vec![0.0; n[0] * n[1] * n[2]] }
    }

    #[inline]
    pub fn idx(&self, x: usize, y: usize, z: usize) -> usize {
        (z * self.n[1] + y) * self.n[0] + x
    }

    #[inline]
    pub fn get(&self, x: usize, y: usize, z: usize) -> f32 {
        self.d[self.idx(x, y, z)]
    }

    /// Value at a cell index that may lie outside (clamped to the edge).
    #[inline]
    pub fn at(&self, x: i64, y: i64, z: i64) -> f32 {
        let c = |v: i64, n: usize| v.clamp(0, n as i64 - 1) as usize;
        self.get(c(x, self.n[0]), c(y, self.n[1]), c(z, self.n[2]))
    }

    fn corners(&self, p: Vec3) -> ([usize; 3], [usize; 3], Vec3) {
        let hi = Vec3::new(self.n[0] as f32 - 1.0, self.n[1] as f32 - 1.0, self.n[2] as f32 - 1.0);
        let p = p.clamp(Vec3::ZERO, hi);
        let i0 = [p.x as usize, p.y as usize, p.z as usize];
        let i1 = [(i0[0] + 1).min(self.n[0] - 1), (i0[1] + 1).min(self.n[1] - 1), (i0[2] + 1).min(self.n[2] - 1)];
        (i0, i1, p - Vec3::new(i0[0] as f32, i0[1] as f32, i0[2] as f32))
    }

    /// Trilinear value at `p` in cell coordinates (cell `i` centred at `i`).
    pub fn sample(&self, p: Vec3) -> f32 {
        let (a, b, f) = self.corners(p);
        let l = |s: f32, t: f32, k: f32| s + (t - s) * k;
        let x = |y: usize, z: usize| l(self.get(a[0], y, z), self.get(b[0], y, z), f.x);
        let yz = |z: usize| l(x(a[1], z), x(b[1], z), f.y);
        l(yz(a[2]), yz(b[2]), f.z)
    }

    /// Smallest and largest of the eight values `sample(p)` blends.
    pub fn range(&self, p: Vec3) -> (f32, f32) {
        let (a, b, _) = self.corners(p);
        let mut r = (f32::MAX, f32::MIN);
        for z in [a[2], b[2]] {
            for y in [a[1], b[1]] {
                for x in [a[0], b[0]] {
                    let v = self.get(x, y, z);
                    r = (r.0.min(v), r.1.max(v));
                }
            }
        }
        r
    }

    pub fn max(&self) -> f32 {
        self.d.iter().fold(0.0f32, |m, v| m.max(*v))
    }
}

/// Fills `out` with `f(x, y, z)`, z-slabs spread over the CPU cores.
pub fn par_fill(out: &mut Field, f: impl Fn(usize, usize, usize) -> f32 + Sync) {
    let [nx, ny, nz] = out.n;
    let slab = nx * ny;
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, nz.max(1));
    let per = nz.div_ceil(threads);
    std::thread::scope(|s| {
        for (k, chunk) in out.d.chunks_mut(per * slab).enumerate() {
            let f = &f;
            s.spawn(move || {
                for (i, v) in chunk.iter_mut().enumerate() {
                    let (z, r) = (k * per + i / slab, i % slab);
                    *v = f(r % nx, r / nx, z);
                }
            });
        }
    });
}
