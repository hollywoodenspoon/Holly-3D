//! UVW Map: projects texture coordinates from a movable gizmo
//! (Planar, Cylindrical + Cap, Spherical, Shrink Wrap, Box, Face, XYZ to UVW).

use crate::mesh::{EditablePoly, PolyId};
use bevy::math::{Mat4, Quat, Vec2, Vec3};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum MapType {
    #[default]
    Planar,
    Cylindrical,
    Spherical,
    ShrinkWrap,
    Box,
    Face,
    XyzToUvw,
}

impl MapType {
    pub const ALL: [MapType; 7] =
        [MapType::Planar, MapType::Cylindrical, MapType::Spherical, MapType::ShrinkWrap, MapType::Box, MapType::Face, MapType::XyzToUvw];
    pub fn label(self) -> &'static str {
        match self {
            MapType::Planar => "Planar",
            MapType::Cylindrical => "Cylindrical",
            MapType::Spherical => "Spherical",
            MapType::ShrinkWrap => "Shrink Wrap",
            MapType::Box => "Box",
            MapType::Face => "Face",
            MapType::XyzToUvw => "XYZ to UVW",
        }
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct UvwMapMod {
    pub kind: MapType,
    pub cap: bool,
    /// Gizmo extents: Width (gizmo X), Length (gizmo Y), Height (gizmo Z).
    pub length: f32,
    pub width: f32,
    pub height: f32,
    pub u_tile: f32,
    pub v_tile: f32,
    pub w_tile: f32,
    pub u_flip: bool,
    pub v_flip: bool,
    pub w_flip: bool,
    /// Alignment axis (0 = X, 1 = Y, 2 = Z) the gizmo's projection axis follows.
    pub axis: usize,
    /// Gizmo placement in the modifier's input space.
    pub rotation: Quat,
    pub center: Vec3,
}

impl Default for UvwMapMod {
    fn default() -> Self {
        Self {
            kind: MapType::Planar,
            cap: false,
            length: 1.0,
            width: 1.0,
            height: 1.0,
            u_tile: 1.0,
            v_tile: 1.0,
            w_tile: 1.0,
            u_flip: false,
            v_flip: false,
            w_flip: false,
            axis: 2,
            rotation: Quat::IDENTITY,
            center: Vec3::ZERO,
        }
    }
}

/// Rotation taking gizmo +Z onto world axis `axis`.
pub fn axis_rotation(axis: usize) -> Quat {
    match axis {
        0 => Quat::from_rotation_y(FRAC_PI_2) * Quat::from_rotation_z(FRAC_PI_2),
        1 => Quat::from_rotation_x(FRAC_PI_2),
        _ => Quat::IDENTITY,
    }
}

impl UvwMapMod {
    pub fn dims(&self) -> Vec3 {
        Vec3::new(self.width, self.length, self.height).max(Vec3::splat(1e-5))
    }

    /// Gizmo matrix (unit cube [-0.5, 0.5]^3 -> input space).
    pub fn gizmo_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.dims(), self.rotation, self.center)
    }

    pub fn set_gizmo_matrix(&mut self, m: Mat4) {
        let (s, r, t) = m.to_scale_rotation_translation();
        (self.width, self.length, self.height) = (s.x.abs().max(1e-4), s.y.abs().max(1e-4), s.z.abs().max(1e-4));
        self.rotation = r.normalize();
        self.center = t;
    }

    /// Fit: sizes and centers the gizmo on the input's extents in gizmo orientation.
    pub fn fit(&mut self, input: &EditablePoly) {
        let inv = self.rotation.inverse();
        let Some(b) = crate::mesh::Aabb::from_points(input.vertices.values().map(|v| inv * v.position)) else {
            return;
        };
        let s = b.size();
        let floor = (s.max_element() * 0.01).max(1e-3);
        self.center = self.rotation * b.center();
        (self.width, self.length, self.height) = (s.x.max(floor), s.y.max(floor), s.z.max(floor));
        if matches!(self.kind, MapType::Cylindrical | MapType::Spherical | MapType::ShrinkWrap) {
            let r = s.x.max(s.y).max(floor);
            (self.width, self.length) = (r, r);
            if self.kind != MapType::Cylindrical {
                self.height = r.max(s.z);
            }
        }
    }

    /// Bitmap Fit: gives the gizmo a bitmap's `width / height` aspect,
    /// keeping its width (U) and scaling the length (V).
    pub fn bitmap_fit(&mut self, aspect: f32) {
        if aspect.is_finite() && aspect > 1e-6 {
            self.length = (self.width / aspect).max(1e-4);
        }
    }

    /// Center: moves the gizmo to the input's center (keeps size/orientation).
    pub fn center_on(&mut self, input: &EditablePoly) {
        let inv = self.rotation.inverse();
        if let Some(b) = crate::mesh::Aabb::from_points(input.vertices.values().map(|v| inv * v.position)) {
            self.center = self.rotation * b.center();
        }
    }

    pub fn set_axis(&mut self, axis: usize, input: &EditablePoly) {
        self.axis = axis.min(2);
        self.rotation = axis_rotation(self.axis);
        self.fit(input);
    }

    /// Normal Align: gizmo Z along `normal`, centered at `point` (input space).
    pub fn normal_align(&mut self, point: Vec3, normal: Vec3) {
        self.rotation = Quat::from_rotation_arc(Vec3::Z, normal.normalize_or(Vec3::Z));
        self.center = point;
    }

    /// View Align: gizmo faces the viewer (`right`, `up` in input space).
    pub fn view_align(&mut self, right: Vec3, up: Vec3, input: &EditablePoly) {
        let x = right.normalize_or(Vec3::X);
        let z = x.cross(up.normalize_or(Vec3::Y)).normalize_or(Vec3::Z);
        let y = z.cross(x);
        self.rotation = Quat::from_mat3(&bevy::math::Mat3::from_cols(x, y, z)).normalize();
        self.fit(input);
    }

    pub fn reset(&mut self, input: &EditablePoly) {
        let kind = self.kind;
        *self = Self { kind, ..Self::default() };
        self.fit(input);
    }

    fn tile(&self, uv: Vec2) -> Vec2 {
        let u = if self.u_flip { 1.0 - uv.x } else { uv.x };
        let v = if self.v_flip { 1.0 - uv.y } else { uv.y };
        Vec2::new(u * self.u_tile, v * self.v_tile)
    }

    /// Projected (untiled) UVs of one polygon's corners.
    fn project_poly(&self, m: &EditablePoly, pid: PolyId, to_gizmo: Mat4) -> Vec<Vec2> {
        let p = &m.polygons[pid];
        let g: Vec<Vec3> = p.vertex_ids().map(|v| to_gizmo.transform_point3(m.pos(v))).collect();
        let n = (self.rotation.inverse() * m.poly_normal(pid)) / self.dims();
        let n = n.normalize_or(Vec3::Z);
        let angle = |q: Vec3| q.y.atan2(q.x) / TAU + 0.5;
        let mut uvs: Vec<Vec2> = match self.kind {
            MapType::Planar => g.iter().map(|q| Vec2::new(q.x + 0.5, q.y + 0.5)).collect(),
            MapType::Cylindrical if self.cap && n.z.abs() > n.x.hypot(n.y) => {
                g.iter().map(|q| Vec2::new(q.x + 0.5, if n.z > 0.0 { q.y + 0.5 } else { 0.5 - q.y })).collect()
            }
            MapType::Cylindrical => g.iter().map(|q| Vec2::new(angle(*q), q.z + 0.5)).collect(),
            MapType::Spherical => g.iter().map(|q| Vec2::new(angle(*q), 0.5 + (q.z / q.length().max(1e-9)).asin() / PI)).collect(),
            MapType::ShrinkWrap => g
                .iter()
                .map(|q| {
                    let d = q.normalize_or(Vec3::Z);
                    let r = d.z.clamp(-1.0, 1.0).acos() / PI * 0.5;
                    let a = d.y.atan2(d.x);
                    Vec2::new(0.5 + r * a.cos(), 0.5 + r * a.sin())
                })
                .collect(),
            MapType::Box => {
                let a = n.abs();
                g.iter()
                    .map(|q| {
                        if a.x >= a.y && a.x >= a.z {
                            Vec2::new(if n.x > 0.0 { q.y } else { -q.y } + 0.5, q.z + 0.5)
                        } else if a.y >= a.z {
                            Vec2::new(if n.y < 0.0 { q.x } else { -q.x } + 0.5, q.z + 0.5)
                        } else {
                            Vec2::new(q.x + 0.5, if n.z > 0.0 { q.y } else { -q.y } + 0.5)
                        }
                    })
                    .collect()
            }
            MapType::Face => face_uvs(g.len()),
            MapType::XyzToUvw => p.vertex_ids().map(|v| m.pos(v).truncate()).collect(),
        };
        let wraps = matches!(self.kind, MapType::Spherical)
            || matches!(self.kind, MapType::Cylindrical) && !(self.cap && n.z.abs() > n.x.hypot(n.y));
        if wraps {
            let (lo, hi) = uvs.iter().fold((f32::MAX, f32::MIN), |a, uv| (a.0.min(uv.x), a.1.max(uv.x)));
            if hi - lo > 0.5 {
                for uv in uvs.iter_mut().filter(|uv| uv.x < 0.5) {
                    uv.x += 1.0;
                }
            }
        }
        uvs
    }

    pub fn apply(&self, mesh: &mut EditablePoly) {
        let to_gizmo = self.gizmo_matrix().inverse();
        let pids: Vec<PolyId> = mesh.polygons.keys().collect();
        for pid in pids {
            let uvs = self.project_poly(mesh, pid, to_gizmo);
            for (c, uv) in mesh.polygons[pid].corners.iter_mut().zip(uvs) {
                c.uv = self.tile(uv);
            }
        }
    }
}

/// Face mapping: every polygon gets the whole map.
fn face_uvs(n: usize) -> Vec<Vec2> {
    match n {
        3 => vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0)],
        4 => vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0), Vec2::new(0.0, 1.0)],
        _ => (0..n)
            .map(|i| {
                let a = i as f32 / n as f32 * TAU - PI * 0.75;
                Vec2::new(0.5 + 0.5 * a.cos(), 0.5 + 0.5 * a.sin())
            })
            .collect(),
    }
}
