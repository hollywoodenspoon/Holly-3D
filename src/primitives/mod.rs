//! Parametric primitives (Create -> Geometry -> Standard Primitives).
//! World is Z-up; every primitive's pivot sits at its base center.

mod boxes;
mod round;
#[cfg(test)]
mod tests;

use crate::mesh::{EditablePoly, VertexId};
use bevy::math::{Vec2, Vec3};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Primitive {
    Box { length: f32, width: f32, height: f32, length_segs: u32, width_segs: u32, height_segs: u32 },
    Sphere { radius: f32, segments: u32, hemisphere: f32 },
    Cylinder { radius: f32, height: f32, height_segs: u32, cap_segs: u32, sides: u32 },
    Cone { radius1: f32, radius2: f32, height: f32, height_segs: u32, cap_segs: u32, sides: u32 },
    Plane { length: f32, width: f32, length_segs: u32, width_segs: u32 },
    Torus { radius1: f32, radius2: f32, segments: u32, sides: u32 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PrimitiveKind {
    Box,
    Sphere,
    Cylinder,
    Cone,
    Plane,
    Torus,
}

impl PrimitiveKind {
    pub const ALL: [PrimitiveKind; 6] = [Self::Box, Self::Sphere, Self::Cylinder, Self::Plane, Self::Torus, Self::Cone];
    pub fn label(self) -> &'static str {
        match self {
            Self::Box => "Box",
            Self::Sphere => "Sphere",
            Self::Cylinder => "Cylinder",
            Self::Cone => "Cone",
            Self::Plane => "Plane",
            Self::Torus => "Torus",
        }
    }
}

impl Primitive {
    pub fn default_of(kind: PrimitiveKind) -> Self {
        match kind {
            PrimitiveKind::Box => Self::Box { length: 1.0, width: 1.0, height: 1.0, length_segs: 1, width_segs: 1, height_segs: 1 },
            PrimitiveKind::Sphere => Self::Sphere { radius: 0.6, segments: 32, hemisphere: 0.0 },
            PrimitiveKind::Cylinder => Self::Cylinder { radius: 0.5, height: 1.2, height_segs: 1, cap_segs: 1, sides: 18 },
            PrimitiveKind::Cone => Self::Cone { radius1: 0.6, radius2: 0.0, height: 1.2, height_segs: 1, cap_segs: 1, sides: 24 },
            PrimitiveKind::Plane => Self::Plane { length: 2.0, width: 2.0, length_segs: 4, width_segs: 4 },
            PrimitiveKind::Torus => Self::Torus { radius1: 0.7, radius2: 0.2, segments: 24, sides: 12 },
        }
    }

    pub fn kind(&self) -> PrimitiveKind {
        match self {
            Self::Box { .. } => PrimitiveKind::Box,
            Self::Sphere { .. } => PrimitiveKind::Sphere,
            Self::Cylinder { .. } => PrimitiveKind::Cylinder,
            Self::Cone { .. } => PrimitiveKind::Cone,
            Self::Plane { .. } => PrimitiveKind::Plane,
            Self::Torus { .. } => PrimitiveKind::Torus,
        }
    }

    pub fn name(&self) -> &'static str {
        self.kind().label()
    }

    pub fn to_editable_poly(&self) -> EditablePoly {
        let mut m = match *self {
            Self::Box { length, width, height, length_segs, width_segs, height_segs } => {
                boxes::build_box(width, length, height, width_segs.max(1), length_segs.max(1), height_segs.max(1))
            }
            Self::Plane { length, width, length_segs, width_segs } => {
                boxes::build_plane(width, length, width_segs.max(1), length_segs.max(1))
            }
            Self::Sphere { radius, segments, hemisphere } => round::build_sphere(radius, segments.max(4), hemisphere),
            Self::Cylinder { radius, height, height_segs, cap_segs, sides } => {
                round::build_cone(radius, radius, height, height_segs.max(1), cap_segs.max(1), sides.max(3))
            }
            Self::Cone { radius1, radius2, height, height_segs, cap_segs, sides } => {
                round::build_cone(radius1, radius2, height, height_segs.max(1), cap_segs.max(1), sides.max(3))
            }
            Self::Torus { radius1, radius2, segments, sides } => round::build_torus(radius1, radius2, segments.max(3), sides.max(3)),
        };
        m.rebuild_edges();
        m
    }
}

/// Connects consecutive vertex rings into quads (or triangles at single-vertex poles).
/// Rings wrap around; `v_coords` gives the V texture coordinate of each ring.
pub(crate) fn stitch_rings(m: &mut EditablePoly, rings: &[Vec<VertexId>], v_coords: &[f32], sides: usize, sg: u32, mat: u16) {
    for r in 0..rings.len().saturating_sub(1) {
        let (a, b) = (&rings[r], &rings[r + 1]);
        let (va, vb) = (v_coords[r], v_coords[r + 1]);
        for j in 0..sides {
            let j1 = j + 1;
            let u0 = j as f32 / sides as f32;
            let u1 = j1 as f32 / sides as f32;
            let pick = |ring: &Vec<VertexId>, k: usize| ring[if ring.len() == 1 { 0 } else { k % sides }];
            let mut vs = vec![pick(a, j), pick(a, j1), pick(b, j1), pick(b, j)];
            let mut uv = vec![Vec2::new(u0, va), Vec2::new(u1, va), Vec2::new(u1, vb), Vec2::new(u0, vb)];
            if b.len() == 1 {
                vs.remove(3);
                uv.remove(3);
                uv[2].x = (u0 + u1) * 0.5;
            }
            if a.len() == 1 {
                vs.remove(0);
                uv.remove(0);
                uv[0].x = (u0 + u1) * 0.5;
            }
            if vs.len() >= 3 {
                let p = m.add_polygon_uv(&vs, &uv);
                m.polygons[p].smoothing_group = sg;
                m.polygons[p].material_id = mat;
            }
        }
    }
}

pub(crate) fn ring(m: &mut EditablePoly, radius: f32, z: f32, sides: usize) -> Vec<VertexId> {
    if radius.abs() < 1e-6 {
        return vec![m.add_vertex(Vec3::new(0.0, 0.0, z))];
    }
    (0..sides)
        .map(|j| {
            let a = j as f32 / sides as f32 * std::f32::consts::TAU;
            m.add_vertex(Vec3::new(radius * a.cos(), radius * a.sin(), z))
        })
        .collect()
}
