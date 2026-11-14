//! Density sources of a volume object, in its local space: a soft union of
//! cloud puffs, a voxel grid, or a box filled evenly. Densities are 0..=1;
//! the volume material's Density turns them into extinction per metre.

use bevy::math::Vec3;

/// One squashed-sphere puff: centre, radius, vertical squash and how much
/// of its lower half Base Flatness cuts away.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Blob {
    pub c: Vec3,
    pub r: f32,
    pub sq: f32,
    /// Lower-half height factor (1 = round, small = flat underside).
    pub low: f32,
    /// Peak density 0..=1 (trails fade and flames cool through it).
    pub w: f32,
}

impl Blob {
    /// Half extents of the blob's box (upper half; the lower is smaller).
    pub fn half(&self) -> Vec3 {
        Vec3::new(self.r, self.r, self.r * self.sq.max(1e-3))
    }

    /// Falloff 0..=1 at `p`: 1 at the centre, 0 on the surface; `edge` is
    /// the share of the radius that fades (Edge Softness).
    pub fn at(&self, p: Vec3, edge: f32) -> f32 {
        let mut q = (p - self.c) / self.half();
        if q.z < 0.0 {
            q.z /= self.low.max(1e-3);
        }
        falloff(q.length_squared(), edge) * self.w
    }

    /// Upper bound of the falloff over the box `lo..hi` (closest point in
    /// the blob's scaled space; the full round blob bounds the cut one).
    pub fn max_in(&self, lo: Vec3, hi: Vec3, edge: f32) -> f32 {
        let h = self.half();
        let q = Vec3::ZERO.clamp((lo - self.c) / h, (hi - self.c) / h);
        falloff(q.length_squared(), edge) * self.w
    }
}

/// Smoothstep falloff over the outer `edge` share of the radius (the rest
/// is fully dense), so clouds keep defined, cauliflower edges.
fn falloff(d2: f32, edge: f32) -> f32 {
    if d2 >= 1.0 {
        return 0.0;
    }
    let x = ((1.0 - d2.sqrt()) / edge.max(1e-3)).min(1.0);
    x * x * (3.0 - 2.0 * x)
}

/// Soft union of falloffs: 1 - prod(1 - f).
pub fn union(fs: impl Iterator<Item = f32>) -> f32 {
    1.0 - fs.fold(1.0, |acc, f| acc * (1.0 - f.clamp(0.0, 1.0)))
}

/// A voxel density grid filling the volume's box (trilinear lookups).
#[derive(Clone, Debug, PartialEq)]
pub struct Voxels {
    pub dims: [usize; 3],
    pub data: Vec<f32>,
}

impl Voxels {
    pub fn get(&self, x: usize, y: usize, z: usize) -> f32 {
        let [nx, ny, _] = self.dims;
        self.data[(z * ny + y) * nx + x]
    }

    /// Trilinear density at `u` in 0..1 box coordinates.
    pub fn sample(&self, u: Vec3) -> f32 {
        let [nx, ny, nz] = self.dims;
        let g = (u.clamp(Vec3::ZERO, Vec3::ONE) * Vec3::new(nx as f32, ny as f32, nz as f32) - 0.5).max(Vec3::ZERO);
        let i = [g.x as usize, g.y as usize, g.z as usize];
        let i0 = [i[0].min(nx - 1), i[1].min(ny - 1), i[2].min(nz - 1)];
        let i1 = [(i0[0] + 1).min(nx - 1), (i0[1] + 1).min(ny - 1), (i0[2] + 1).min(nz - 1)];
        let f = g - Vec3::new(i0[0] as f32, i0[1] as f32, i0[2] as f32);
        let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
        let x = |y: usize, z: usize| lerp(self.get(i0[0], y, z), self.get(i1[0], y, z), f.x);
        let yz = |z: usize| lerp(x(i0[1], z), x(i1[1], z), f.y);
        lerp(yz(i0[2]), yz(i1[2]), f.z)
    }

    /// Largest voxel value touching the 0..1 box range `a..b`.
    pub fn max_in(&self, a: Vec3, b: Vec3) -> f32 {
        let [nx, ny, nz] = self.dims;
        let cell = |v: f32, n: usize| ((v * n as f32 - 0.5).floor().max(0.0) as usize).min(n - 1);
        let top = |v: f32, n: usize| ((v * n as f32 + 0.5).ceil().max(0.0) as usize).min(n - 1);
        let mut m: f32 = 0.0;
        for z in cell(a.z, nz)..=top(b.z, nz) {
            for y in cell(a.y, ny)..=top(b.y, ny) {
                for x in cell(a.x, nx)..=top(b.x, nx) {
                    m = m.max(self.get(x, y, z));
                }
            }
        }
        m
    }
}

/// Where a volume's density comes from.
#[derive(Clone, Debug, PartialEq)]
pub enum Source {
    /// Cloud puffs with erosion (Cloud / Cloud Field objects).
    Puffs(super::erode::Cloud),
    /// A voxel grid over the box (shared: imported VDB grids are large).
    Grid(std::sync::Arc<Voxels>),
    /// Evenly dense box (any other mesh with a Volume material).
    Uniform,
}
