//! Architectural geometry kit shared by the GhostTown settlement generator,
//! the Medieval Architect and the Battlefield / Ruin modifiers: boxes,
//! beams, prisms, cylinders, cones and roofs appended straight into one
//! `EditablePoly` (edges are rebuilt once in `finish`), with world-scale
//! box-projected UVs, the current Material ID and smoothing group, and a
//! local frame (`xf`) so each building is modelled in its own coordinates.

pub mod roofs;

use crate::mesh::{EditablePoly, PolyId, VertexId};
use bevy::math::{Mat4, Quat, Vec2, Vec3};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use std::f32::consts::{FRAC_PI_2, TAU};

pub struct Kit {
    pub m: EditablePoly,
    /// Material ID of the polygons added next.
    pub mat: u16,
    /// Smoothing group of flat faces (0 = faceted).
    pub sg: u32,
    /// Local frame applied to every point added.
    pub xf: Mat4,
    /// UV tiles per metre.
    pub uv_scale: f32,
}

impl Default for Kit {
    fn default() -> Self {
        Self { m: EditablePoly::new(), mat: 1, sg: 0, xf: Mat4::IDENTITY, uv_scale: 0.5 }
    }
}

/// Seeded deterministic RNG (`salt` separates independent streams).
pub fn rng(seed: u32, salt: u64) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(((seed as u64) << 20) ^ salt.wrapping_mul(0x9E37_79B9_7F4A_7C15))
}

/// Even-odd point-in-polygon test in the XY plane.
pub fn inside(poly: &[Vec2], p: Vec2) -> bool {
    let n = poly.len();
    let mut c = false;
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + n - 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            c = !c;
        }
    }
    c
}

/// Frame at `at`, turned `yaw` radians about +Z.
pub fn frame(at: Vec3, yaw: f32) -> Mat4 {
    Mat4::from_rotation_translation(Quat::from_rotation_z(yaw), at)
}

impl Kit {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the Material ID for what follows (chainable).
    pub fn mat(&mut self, id: u16) -> &mut Self {
        self.mat = id;
        self
    }

    /// A new vertex at a local point.
    pub fn vtx(&mut self, p: Vec3) -> VertexId {
        self.m.add_vertex(self.xf.transform_point3(p))
    }

    /// A polygon over existing vertices (counter-clockwise from outside),
    /// with box-projected UVs, the current Material ID and smoothing group.
    pub fn face(&mut self, ids: &[VertexId]) -> PolyId {
        let world: Vec<Vec3> = ids.iter().map(|&v| self.m.pos(v)).collect();
        let a = newell(&world).abs();
        let s = self.uv_scale;
        let uvs: Vec<Vec2> = world
            .iter()
            .map(|p| {
                if a.z >= a.x && a.z >= a.y {
                    Vec2::new(p.x, p.y) * s
                } else if a.x >= a.y {
                    Vec2::new(p.y, p.z) * s
                } else {
                    Vec2::new(p.x, p.z) * s
                }
            })
            .collect();
        let pid = self.m.add_polygon_uv(ids, &uvs);
        let poly = &mut self.m.polygons[pid];
        (poly.material_id, poly.smoothing_group) = (self.mat, self.sg);
        pid
    }

    /// A planar polygon from local points (counter-clockwise from outside).
    pub fn poly(&mut self, pts: &[Vec3]) -> PolyId {
        let ids: Vec<VertexId> = pts.iter().map(|p| self.vtx(*p)).collect();
        self.face(&ids)
    }

    pub fn quad(&mut self, a: Vec3, b: Vec3, c: Vec3, d: Vec3) -> PolyId {
        self.poly(&[a, b, c, d])
    }

    /// Box from 8 local corners (index bits: x, y, z), faces outward.
    fn hexa(&mut self, c: [Vec3; 8], skip_bottom: bool) {
        const F: [[usize; 4]; 6] = [[0, 2, 3, 1], [4, 5, 7, 6], [0, 1, 5, 4], [2, 6, 7, 3], [0, 4, 6, 2], [1, 3, 7, 5]];
        let ids = c.map(|p| self.vtx(p));
        for (i, f) in F.iter().enumerate() {
            if !(skip_bottom && i == 0) {
                self.face(&f.map(|k| ids[k]));
            }
        }
    }

    /// Axis-aligned (in the local frame) box.
    pub fn block(&mut self, min: Vec3, max: Vec3) {
        let c = std::array::from_fn(|i| {
            Vec3::new(if i & 1 == 0 { min.x } else { max.x }, if i & 2 == 0 { min.y } else { max.y }, if i & 4 == 0 { min.z } else { max.z })
        });
        self.hexa(c, false);
    }

    /// Box with center, half extents and rotation (local frame).
    pub fn cuboid(&mut self, center: Vec3, half: Vec3, rot: Quat) {
        let c = std::array::from_fn(|i| {
            let s = Vec3::new(if i & 1 == 0 { -1.0 } else { 1.0 }, if i & 2 == 0 { -1.0 } else { 1.0 }, if i & 4 == 0 { -1.0 } else { 1.0 });
            center + rot * (s * half)
        });
        self.hexa(c, false);
    }

    /// Square-section beam from `a` to `b` (`w` wide, `h` tall section).
    pub fn beam(&mut self, a: Vec3, b: Vec3, w: f32, h: f32) {
        let d = b - a;
        let len = d.length();
        if len < 1e-5 {
            return;
        }
        let rot = Quat::from_rotation_arc(Vec3::X, d / len);
        self.cuboid((a + b) * 0.5, Vec3::new(len * 0.5, w * 0.5, h * 0.5), rot);
    }

    /// Vertical prism over a counter-clockwise base outline at `z0`.
    pub fn prism(&mut self, base: &[Vec2], z0: f32, h: f32, top: bool, bottom: bool) {
        let n = base.len();
        let lo: Vec<VertexId> = base.iter().map(|p| self.vtx(p.extend(z0))).collect();
        let hi: Vec<VertexId> = base.iter().map(|p| self.vtx(p.extend(z0 + h))).collect();
        for i in 0..n {
            let j = (i + 1) % n;
            self.face(&[lo[i], lo[j], hi[j], hi[i]]);
        }
        if top {
            self.face(&hi);
        }
        if bottom {
            self.face(&lo.iter().rev().copied().collect::<Vec<_>>());
        }
    }

    /// Cylinder / truncated cone along local +Z from `c` (r1 = 0 makes a cone).
    pub fn frustum(&mut self, c: Vec3, r0: f32, r1: f32, h: f32, sides: u32, caps: bool) {
        let n = sides.max(3) as usize;
        let ring = |kit: &mut Self, r: f32, z: f32| -> Vec<VertexId> {
            (0..n)
                .map(|i| {
                    let a = i as f32 / n as f32 * TAU;
                    kit.vtx(c + Vec3::new(a.cos() * r, a.sin() * r, z))
                })
                .collect()
        };
        let bot = ring(self, r0, 0.0);
        let cone = r1 <= 1e-5;
        let top = if cone { vec![self.vtx(c + Vec3::Z * h)] } else { ring(self, r1, h) };
        let sg = self.sg;
        self.sg = 1 << 3;
        for i in 0..n {
            let j = (i + 1) % n;
            if cone {
                self.face(&[bot[i], bot[j], top[0]]);
            } else {
                self.face(&[bot[i], bot[j], top[j], top[i]]);
            }
        }
        self.sg = sg;
        if caps {
            if !cone {
                self.face(&top);
            }
            self.face(&bot.iter().rev().copied().collect::<Vec<_>>());
        }
    }

    /// Capped cylinder between two points.
    pub fn tube(&mut self, a: Vec3, b: Vec3, r: f32, sides: u32) {
        let d = b - a;
        let len = d.length();
        if len < 1e-5 {
            return;
        }
        let saved = self.xf;
        self.xf = saved * Mat4::from_rotation_translation(Quat::from_rotation_arc(Vec3::Z, d / len), a);
        self.frustum(Vec3::ZERO, r, r, len, sides, true);
        self.xf = saved;
    }

    /// Runs `f` with the local frame multiplied by `m`, then restores it.
    pub fn at(&mut self, m: Mat4, f: impl FnOnce(&mut Self)) {
        let saved = self.xf;
        self.xf = saved * m;
        f(self);
        self.xf = saved;
    }

    /// Ground strip from `a` to `b` (`w` wide) following the height field
    /// `h`, split into pieces of about `seg` metres so it can be cratered.
    pub fn strip(&mut self, a: Vec2, b: Vec2, w: f32, seg: f32, h: impl Fn(Vec2) -> f32) {
        let d = b - a;
        let len = d.length();
        if len < 1e-4 {
            return;
        }
        let side = d.perp() / len * w * 0.5;
        let n = (len / seg.max(0.1)).ceil().max(1.0) as usize;
        let at = |q: Vec2| q.extend(h(q));
        for i in 0..n {
            let (p, q) = (a + d * (i as f32 / n as f32), a + d * ((i + 1) as f32 / n as f32));
            self.quad(at(p - side), at(q - side), at(q + side), at(p + side));
        }
    }

    /// Ground grid over a rectangle following the height field `h` (cells
    /// of about `cell`; `keep` clips cells by their centre).
    pub fn ground(&mut self, min: Vec2, max: Vec2, cell: f32, h: impl Fn(Vec2) -> f32, keep: impl Fn(Vec2) -> bool) {
        let size = max - min;
        let (nx, ny) = ((size.x / cell).ceil().clamp(1.0, 96.0) as usize, (size.y / cell).ceil().clamp(1.0, 96.0) as usize);
        let at = |i: usize, j: usize| min + size * Vec2::new(i as f32 / nx as f32, j as f32 / ny as f32);
        let mut ids = vec![None; (nx + 1) * (ny + 1)];
        for j in 0..ny {
            for i in 0..nx {
                if !keep((at(i, j) + at(i + 1, j + 1)) * 0.5) {
                    continue;
                }
                let quad = [(i, j), (i + 1, j), (i + 1, j + 1), (i, j + 1)];
                let corners = quad.map(|(x, y)| {
                    let q = at(x, y);
                    *ids[y * (nx + 1) + x].get_or_insert_with(|| self.m.add_vertex(self.xf.transform_point3(q.extend(h(q)))))
                });
                let uvs = quad.map(|(x, y)| at(x, y) * self.uv_scale);
                let pid = self.m.add_polygon_uv(&corners, &uvs);
                (self.m.polygons[pid].material_id, self.m.polygons[pid].smoothing_group) = (self.mat, 1);
            }
        }
    }

    /// Appends another mesh (already in this kit's space), keeping its
    /// Material IDs, smoothing groups, UVs and vertex colors.
    pub fn append(&mut self, other: &EditablePoly) {
        let map: std::collections::HashMap<VertexId, VertexId> =
            other.vertices.iter().map(|(id, v)| (id, self.m.vertices.insert(v.clone()))).collect();
        for p in other.polygons.values() {
            let mut np = p.clone();
            np.corners.iter_mut().for_each(|c| c.vertex_id = map[&c.vertex_id]);
            self.m.polygons.insert(np);
        }
    }

    pub fn finish(mut self) -> EditablePoly {
        self.m.finalize();
        self.m
    }
}

fn newell(pts: &[Vec3]) -> Vec3 {
    let n = pts.len();
    (0..n).fold(Vec3::ZERO, |acc, i| {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        acc + Vec3::new((a.y - b.y) * (a.z + b.z), (a.z - b.z) * (a.x + b.x), (a.x - b.x) * (a.y + b.y))
    })
}


/// Runs `f` once per side of a `2hx` x `2hy` box in a frame whose face lies
/// at local y = 0 (just proud of the wall) facing -Y, centred on x = 0;
/// `f` receives the side index and the face width.
pub fn sides(kit: &mut Kit, hx: f32, hy: f32, mut f: impl FnMut(&mut Kit, usize, f32)) {
    for k in 0..4 {
        let (w, d) = if k % 2 == 0 { (2.0 * hx, hy) } else { (2.0 * hy, hx) };
        let m = Mat4::from_rotation_z(k as f32 * FRAC_PI_2) * Mat4::from_translation(Vec3::new(0.0, -d - 0.03, 0.0));
        kit.at(m, |kit| f(kit, k, w));
    }
}

/// The GhostTown Multi/Sub-Object Material IDs shared by the settlement
/// generator, the Medieval Architect and the Battlefield / Ruin modifier.
pub mod ids {
    pub const ROAD: u16 = 1;
    pub const LOT: u16 = 2;
    pub const FACADE: u16 = 3;
    pub const TRIM: u16 = 4;
    pub const ROOF: u16 = 5;
    pub const GLASS: u16 = 6;
    pub const LIT: u16 = 7;
    pub const FOLIAGE: u16 = 8;
    pub const CHARRED: u16 = 9;
    pub const IRON: u16 = 10;
    pub const PLASTER: u16 = 11;
    pub const REBAR: u16 = 12;
    pub const ASPHALT: u16 = 13;
    pub const LANE: u16 = 14;
    pub const SIDEWALK: u16 = 15;
    pub const CURB: u16 = 16;
    pub const COBBLE: u16 = 17;
    pub const WATER: u16 = 18;
    pub const BRICK: u16 = 19;
    pub const STUCCO: u16 = 20;
    pub const SLATE: u16 = 21;
    pub const RUST: u16 = 22;
    pub const BROKEN_GLASS: u16 = 23;
    pub const NEON: u16 = 24;
    /// Highest GhostTown ID.
    pub const MAX: u16 = 24;
}

#[cfg(test)]
mod tests;
