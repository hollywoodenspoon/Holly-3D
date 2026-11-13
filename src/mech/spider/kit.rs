//! Mesh helpers for the Spider Platform prefabs: blocks, cylinders, lofts
//! between two rings, beams between two points and spikes. Every piece
//! is a closed shell with outward normals; `finish` sets smoothing
//! groups by angle and packs Smart UVs.

use crate::mesh::EditablePoly;
use crate::modifiers::smart_uv::smart_uv_project;
use crate::primitives::Primitive;
use bevy::math::{Mat4, Quat, Vec2, Vec3};

/// Material IDs shared by the whole mech line (spec, Material IDs table).
pub mod mat {
    pub const HULL: u16 = 1;
    pub const ARMOUR: u16 = 2;
    pub const LEG: u16 = 3;
    pub const JOINT: u16 = 4;
    pub const ROD: u16 = 5;
    pub const BARREL: u16 = 6;
    pub const RADIATOR: u16 = 7;
    pub const WEAPON: u16 = 8;
    pub const GLASS: u16 = 9;
    pub const FOOT: u16 = 10;
}

/// A part mesh being assembled from pieces.
#[derive(Default)]
pub struct Kit(pub EditablePoly);

impl Kit {
    /// Appends `piece` with every polygon set to `mat`.
    pub fn add(&mut self, mut piece: EditablePoly, mat: u16) -> &mut Self {
        for p in piece.polygons.values_mut() {
            p.material_id = mat;
            p.smoothing_group = 0;
        }
        self.0.append(&piece, Mat4::IDENTITY);
        self
    }

    /// Smoothing groups by angle (35°) and Smart UVs (66°, margin 0.02).
    pub fn finish(mut self) -> EditablePoly {
        let m = &mut self.0;
        m.rebuild_edges();
        m.auto_smooth(&[], 35.0);
        if let Ok((uvs, _)) = smart_uv_project(m, 66.0, 0.02, true) {
            for (p, uv) in m.polygons.values_mut().zip(uvs) {
                for (c, u) in p.corners.iter_mut().zip(uv) {
                    c.uv = u;
                }
            }
        }
        self.0
    }
}

/// Box of `size` centred on `c`.
pub fn block(size: Vec3, c: Vec3) -> EditablePoly {
    let mut m = Primitive::Box { length: size.y, width: size.x, height: size.z, length_segs: 1, width_segs: 1, height_segs: 1 }.to_editable_poly();
    m.transform(Mat4::from_translation(c - Vec3::Z * size.z * 0.5));
    m
}

/// Cylinder of `radius` from `from` along `axis` for `len`.
pub fn cyl(radius: f32, len: f32, sides: u32, from: Vec3, axis: Vec3) -> EditablePoly {
    let mut m = Primitive::Cylinder { radius, height: len, height_segs: 1, cap_segs: 1, sides }.to_editable_poly();
    let q = Quat::from_rotation_arc(Vec3::Z, axis.normalize_or(Vec3::Z));
    m.transform(Mat4::from_rotation_translation(q, from));
    m
}

/// Cylinder of `radius` and `len` centred on `c` along `axis`.
pub fn cyl_mid(radius: f32, len: f32, sides: u32, c: Vec3, axis: Vec3) -> EditablePoly {
    let a = axis.normalize_or(Vec3::Z);
    cyl(radius, len, sides, c - a * len * 0.5, a)
}

/// Closed shell between two rings of equal size. Both rings wind
/// counter-clockwise about the direction from `low` to `high`.
pub fn loft(low: &[Vec3], high: &[Vec3]) -> EditablePoly {
    assert_eq!(low.len(), high.len(), "loft rings must match");
    let mut m = EditablePoly::new();
    let a: Vec<_> = low.iter().map(|&p| m.add_vertex(p)).collect();
    let b: Vec<_> = high.iter().map(|&p| m.add_vertex(p)).collect();
    let n = a.len();
    let uv = |k: usize| vec![Vec2::ZERO; k];
    let rev: Vec<_> = a.iter().rev().copied().collect();
    m.add_polygon_uv(&rev, &uv(n));
    m.add_polygon_uv(&b, &uv(n));
    for i in 0..n {
        let j = (i + 1) % n;
        m.add_polygon_uv(&[a[i], a[j], b[j], b[i]], &uv(4));
    }
    m
}

/// Rectangle ring of width `w` (along `side`) and height `h` about `c`,
/// counter-clockwise about `dir`.
pub fn rect_ring(c: Vec3, dir: Vec3, side: Vec3, w: f32, h: f32) -> Vec<Vec3> {
    let d = dir.normalize_or(Vec3::Z);
    let u = (side - d * side.dot(d)).normalize_or(Vec3::X);
    let v = d.cross(u);
    [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)].iter().map(|&(x, y)| c + u * (x * w * 0.5) + v * (y * h * 0.5)).collect()
}

/// Tapered rectangular beam from `a` (w0 × h0) to `b` (w1 × h1); width
/// runs along `side`.
pub fn beam(a: Vec3, b: Vec3, side: Vec3, w0: f32, h0: f32, w1: f32, h1: f32) -> EditablePoly {
    let d = b - a;
    loft(&rect_ring(a, d, side, w0, h0), &rect_ring(b, d, side, w1, h1))
}

/// Ring from a counter-clockwise (seen from +Z) XY profile at height `z`.
pub fn profile(pts: &[(f32, f32)], z: f32) -> Vec<Vec3> {
    pts.iter().map(|&(x, y)| Vec3::new(x, y, z)).collect()
}

/// Four-sided spike from a square base at `base` pointing along `dir`.
pub fn spike(base: Vec3, dir: Vec3, side: Vec3, half: f32, len: f32) -> EditablePoly {
    let d = dir.normalize_or(Vec3::X);
    let ring = rect_ring(base, d, side, half * 2.0, half * 2.0);
    let mut m = EditablePoly::new();
    let a: Vec<_> = ring.iter().map(|&p| m.add_vertex(p)).collect();
    let tip = m.add_vertex(base + d * len);
    let rev: Vec<_> = a.iter().rev().copied().collect();
    m.add_polygon_uv(&rev, &[Vec2::ZERO; 4]);
    for i in 0..4 {
        m.add_polygon_uv(&[a[i], a[(i + 1) % 4], tip], &[Vec2::ZERO; 3]);
    }
    m
}

/// Signed volume of a closed mesh (positive when the normals face out).
pub fn volume(m: &EditablePoly) -> f32 {
    let mut v = 0.0;
    for p in m.polygons.values() {
        let pts: Vec<Vec3> = p.vertex_ids().map(|id| m.pos(id)).collect();
        for i in 1..pts.len().saturating_sub(1) {
            v += pts[0].dot(pts[i].cross(pts[i + 1])) / 6.0;
        }
    }
    v
}
