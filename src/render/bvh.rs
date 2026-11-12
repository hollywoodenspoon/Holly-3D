//! Flattened render triangles + a median-split BVH, collapsed to a 4-wide
//! SIMD BVH (`bvh_wide`) for ray queries.

use bevy::math::{Vec2, Vec3, Vec4};

#[derive(Clone, Debug)]
pub struct Tri {
    pub p: [Vec3; 3],
    /// Shading normals (world, normalized).
    pub n: [Vec3; 3],
    pub uv: [Vec2; 3],
    /// Vertex paint colors (sRGB + alpha).
    pub col: [Vec4; 3],
    /// Index into the render scene's material table.
    pub mat: usize,
    /// Object index (outline pass).
    pub obj: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Hit {
    pub t: f32,
    pub tri: usize,
    /// Barycentrics of corners 1 and 2.
    pub u: f32,
    pub v: f32,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct Node {
    pub(super) lo: Vec3,
    pub(super) hi: Vec3,
    /// Leaf: first triangle index; inner: right child index.
    pub(super) start: u32,
    /// Leaf triangle count (0 = inner node, left child = self + 1).
    pub(super) count: u32,
}

pub struct Bvh {
    pub tris: Vec<Tri>,
    /// Binary nodes (depth-first; inner: count 0, left child next, right at
    /// `start`): the layout the GPU path tracer traverses.
    nodes: Vec<Node>,
    wide: Vec<super::bvh_wide::Node4>,
}

pub(super) fn intersect(t: &Tri, o: Vec3, d: Vec3) -> Option<(f32, f32, f32)> {
    let e1 = t.p[1] - t.p[0];
    let e2 = t.p[2] - t.p[0];
    let pv = d.cross(e2);
    let det = e1.dot(pv);
    if det.abs() < 1e-12 {
        return None;
    }
    let inv = 1.0 / det;
    let s = o - t.p[0];
    let u = s.dot(pv) * inv;
    if !(0.0..=1.0).contains(&u) {
        return None;
    }
    let q = s.cross(e1);
    let v = d.dot(q) * inv;
    if v < 0.0 || u + v > 1.0 {
        return None;
    }
    Some((e2.dot(q) * inv, u, v))
}

#[cfg(test)]
fn slab(lo: Vec3, hi: Vec3, o: Vec3, inv: Vec3, tmax: f32) -> bool {
    let a = (lo - o) * inv;
    let b = (hi - o) * inv;
    let t0 = a.min(b).max_element().max(0.0);
    let t1 = a.max(b).min_element().min(tmax);
    t0 <= t1
}

#[cfg(test)]
impl Tri {
    /// Ray distance by direct intersection (brute-force reference).
    pub fn ray(&self, o: Vec3, d: Vec3) -> Option<f32> {
        intersect(self, o, d).map(|h| h.0)
    }
}

impl Bvh {
    pub fn build(mut tris: Vec<Tri>) -> Self {
        let mut nodes = Vec::with_capacity(tris.len() * 2 / 3 + 1);
        if !tris.is_empty() {
            let n = tris.len();
            Self::split(&mut tris, &mut nodes, 0, n);
        }
        let wide = super::bvh_wide::collapse(&nodes);
        Self {
            tris,
            nodes,
            wide,
        }
    }

    pub(super) fn nodes(&self) -> &[Node] {
        &self.nodes
    }

    fn bounds(tris: &[Tri]) -> (Vec3, Vec3) {
        tris.iter().fold((Vec3::splat(f32::MAX), Vec3::splat(f32::MIN)), |(lo, hi), t| {
            (lo.min(t.p[0]).min(t.p[1]).min(t.p[2]), hi.max(t.p[0]).max(t.p[1]).max(t.p[2]))
        })
    }

    fn split(tris: &mut [Tri], nodes: &mut Vec<Node>, start: usize, end: usize) -> usize {
        let (lo, hi) = Self::bounds(&tris[start..end]);
        let me = nodes.len();
        nodes.push(Node { lo, hi, start: start as u32, count: (end - start) as u32 });
        if end - start <= 4 {
            return me;
        }
        let centroid = |t: &Tri| (t.p[0] + t.p[1] + t.p[2]) / 3.0;
        let ext = hi - lo;
        let axis = if ext.x >= ext.y && ext.x >= ext.z {
            0
        } else if ext.y >= ext.z {
            1
        } else {
            2
        };
        let mid = (start + end) / 2;
        tris[start..end].select_nth_unstable_by(mid - start, |a, b| centroid(a)[axis].total_cmp(&centroid(b)[axis]));
        Self::split(tris, nodes, start, mid);
        let right = Self::split(tris, nodes, mid, end);
        nodes[me].start = right as u32;
        nodes[me].count = 0;
        me
    }

    /// Closest hit along `o + t d` with `t` in (eps, tmax).
    pub fn closest(&self, o: Vec3, d: Vec3, tmax: f32) -> Option<Hit> {
        super::bvh_wide::walk(&self.wide, &self.tris, o, d, tmax, false)
    }

    /// True when anything blocks the segment (shadow rays).
    pub fn occluded(&self, o: Vec3, d: Vec3, tmax: f32) -> bool {
        super::bvh_wide::walk(&self.wide, &self.tris, o, d, tmax, true).is_some()
    }

    /// Binary-tree traversal (reference for the wide BVH's tests).
    #[cfg(test)]
    pub(super) fn walk_binary(&self, o: Vec3, d: Vec3, tmax: f32, any: bool) -> Option<Hit> {
        if self.nodes.is_empty() {
            return None;
        }
        let inv = Vec3::ONE / d;
        let mut best: Option<Hit> = None;
        let mut limit = tmax;
        let mut stack = [0u32; 64];
        let mut sp = 1;
        while sp > 0 {
            sp -= 1;
            let idx = stack[sp];
            let n = self.nodes[idx as usize];
            if !slab(n.lo, n.hi, o, inv, limit) {
                continue;
            }
            if n.count > 0 {
                for i in n.start as usize..(n.start + n.count) as usize {
                    if let Some((t, u, v)) = intersect(&self.tris[i], o, d)
                        && t > 1e-4
                        && t < limit
                    {
                        limit = t;
                        best = Some(Hit { t, tri: i, u, v });
                        if any {
                            return best;
                        }
                    }
                }
            } else if sp + 2 <= stack.len() {
                stack[sp] = n.start;
                stack[sp + 1] = idx + 1;
                sp += 2;
            }
        }
        best
    }
}
