//! Pure viewport camera math (navigation, Z-up). Matrices match
//! Bevy's reverse-Z conventions so picking agrees with what is rendered.

use bevy::math::{Mat4, Vec2, Vec3, Vec4Swizzles, proj};
use std::f32::consts::FRAC_PI_2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewKind {
    Perspective,
    Top,
    Front,
    Left,
    /// Ortho view that has been orbited ("Orthographic").
    User,
    /// Looking through a Target / Free Camera object.
    Camera,
}

impl ViewKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Perspective => "Perspective",
            Self::Top => "Top",
            Self::Front => "Front",
            Self::Left => "Left",
            Self::User => "Orthographic",
            Self::Camera => "Camera",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewCamera {
    pub kind: ViewKind,
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub fov: f32,
    /// Visible world height for orthographic views.
    pub ortho_height: f32,
    /// `fov` is horizontal (camera objects) instead of vertical.
    pub hfov: bool,
    /// Perspective clipping planes (`far` = infinity: no far clip).
    pub near: f32,
    pub far: f32,
}

pub const ORTHO_DEPTH: f32 = 5000.0;

/// A world-space ray.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub dir: Vec3,
}

impl Ray {
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.dir * t
    }
    /// Intersection parameter with plane (point, normal).
    pub fn plane(&self, p: Vec3, n: Vec3) -> Option<f32> {
        let d = self.dir.dot(n);
        if d.abs() < 1e-9 {
            return None;
        }
        Some((p - self.origin).dot(n) / d)
    }
}

impl ViewCamera {
    pub fn new(kind: ViewKind) -> Self {
        let (yaw, pitch) = match kind {
            ViewKind::Top => (FRAC_PI_2, -FRAC_PI_2),
            ViewKind::Front => (FRAC_PI_2, 0.0),
            ViewKind::Left => (0.0, 0.0),
            _ => (135f32.to_radians(), -28f32.to_radians()),
        };
        Self {
            kind,
            target: Vec3::new(0.0, 0.0, 0.4),
            distance: 7.0,
            yaw,
            pitch,
            fov: 45f32.to_radians(),
            ortho_height: 6.0,
            hfov: false,
            near: 0.02,
            far: f32::INFINITY,
        }
    }

    pub fn is_ortho(&self) -> bool {
        !matches!(self.kind, ViewKind::Perspective | ViewKind::Camera)
    }

    /// Vertical field of view for an image of `aspect` (width / height).
    pub fn fov_y(&self, aspect: f32) -> f32 {
        if self.hfov { 2.0 * ((self.fov * 0.5).tan() / aspect.max(1e-3)).atan() } else { self.fov }
    }

    pub fn forward(&self) -> Vec3 {
        Vec3::new(self.pitch.cos() * self.yaw.cos(), self.pitch.cos() * self.yaw.sin(), self.pitch.sin()).normalize()
    }

    pub fn right(&self) -> Vec3 {
        Vec3::new(self.yaw.sin(), -self.yaw.cos(), 0.0)
    }

    pub fn up(&self) -> Vec3 {
        self.right().cross(self.forward()).normalize()
    }

    pub fn eye(&self) -> Vec3 {
        let d = if self.is_ortho() { ORTHO_DEPTH * 0.5 } else { self.distance };
        self.target - self.forward() * d
    }

    #[allow(deprecated)]
    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_rh(self.eye(), self.forward(), self.up())
    }

    pub fn world_from_view(&self) -> Mat4 {
        self.view_matrix().inverse()
    }

    pub fn ortho_size(&self, aspect: f32) -> Vec2 {
        Vec2::new(self.ortho_height * aspect, self.ortho_height)
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        if self.is_ortho() {
            let s = self.ortho_size(aspect) * 0.5;
            proj::orthographic(-s.x, s.x, -s.y, s.y, ORTHO_DEPTH, 0.0)
        } else {
            let fy = self.fov_y(aspect);
            if self.far.is_finite() && self.far > self.near {
                // Swapped planes give Bevy's reverse-Z depth (near = 1, far = 0).
                proj::perspective(fy, aspect, self.far, self.near)
            } else {
                proj::perspective_infinite_reverse(fy, aspect, self.near)
            }
        }
    }

    pub fn view_proj(&self, aspect: f32) -> Mat4 {
        self.projection_matrix(aspect) * self.view_matrix()
    }

    /// Ray through normalized device coords (x right, y up, both -1..1).
    pub fn ray(&self, ndc: Vec2, aspect: f32) -> Ray {
        let inv = self.view_proj(aspect).inverse();
        let near = inv.project_point3(ndc.extend(1.0));
        let far = inv.project_point3(ndc.extend(if self.is_ortho() { 0.0 } else { 1e-4 }));
        Ray { origin: near, dir: (far - near).normalize() }
    }

    /// World -> ndc (x, y) + depth flag (false if behind the camera).
    pub fn project(&self, p: Vec3, aspect: f32) -> Option<Vec2> {
        let clip = self.view_proj(aspect) * p.extend(1.0);
        if clip.w <= 1e-6 {
            return None;
        }
        Some(clip.xy() / clip.w)
    }

    /// World units per screen pixel at point `p` for a viewport `height_px` tall.
    pub fn units_per_pixel(&self, p: Vec3, height_px: f32) -> f32 {
        let h = if self.is_ortho() {
            self.ortho_height
        } else {
            let d = (p - self.eye()).dot(self.forward()).max(0.05);
            2.0 * d * (self.fov * 0.5).tan()
        };
        h / height_px.max(1.0)
    }

    // ---- navigation ------------------------------------------------

    /// MMB drag: pan by pixel delta.
    pub fn pan(&mut self, delta_px: Vec2, height_px: f32) {
        let upp = self.units_per_pixel(self.target, height_px);
        self.target += (-self.right() * delta_px.x + self.up() * delta_px.y) * upp;
    }

    /// Alt+MMB: orbit around `pivot` (selection center). Ortho views become "Orthographic".
    pub fn orbit(&mut self, delta_px: Vec2, pivot: Vec3) {
        if matches!(self.kind, ViewKind::Top | ViewKind::Front | ViewKind::Left) {
            self.kind = ViewKind::User;
        }
        let eye_before = self.target - pivot;
        let old = (self.yaw, self.pitch);
        self.yaw -= delta_px.x * 0.008;
        self.pitch = (self.pitch - delta_px.y * 0.008).clamp(-1.5605, 1.5605);
        // rotate the target around the pivot by the same rotation
        let r_old = Mat4::from_rotation_z(old.0) * Mat4::from_rotation_y(-old.1);
        let r_new = Mat4::from_rotation_z(self.yaw) * Mat4::from_rotation_y(-self.pitch);
        let local = r_old.inverse().transform_vector3(eye_before);
        self.target = pivot + r_new.transform_vector3(local);
    }

    /// Wheel / Ctrl+Alt+MMB: zoom (positive = in).
    pub fn zoom(&mut self, steps: f32) {
        let f = 0.88f32.powf(steps);
        if self.is_ortho() {
            self.ortho_height = (self.ortho_height * f).clamp(0.01, 10_000.0);
        } else {
            self.distance = (self.distance * f).clamp(0.05, 10_000.0);
        }
    }

    /// Perspective -> Orthographic ("U"), same angle and framing: the
    /// ortho height matches what the perspective view showed at its target.
    pub fn to_orthographic(&mut self) {
        self.ortho_height = (2.0 * self.distance * (self.fov_y(1.0) * 0.5).tan()).clamp(0.01, 10_000.0);
        (self.kind, self.hfov, self.fov) = (ViewKind::User, false, 45f32.to_radians());
    }

    /// Back to Perspective, same angle; the distance shows the same height.
    pub fn to_perspective(&mut self) {
        (self.kind, self.hfov, self.fov) = (ViewKind::Perspective, false, 45f32.to_radians());
        self.distance = (self.ortho_height * 0.5 / (self.fov * 0.5).tan()).clamp(0.05, 10_000.0);
    }

    /// Z: Zoom Extents — frame a bounding box (min, max).
    pub fn frame(&mut self, min: Vec3, max: Vec3) {
        let c = (min + max) * 0.5;
        let r = ((max - min).length() * 0.5).max(0.05);
        let fov = if self.hfov { self.fov.min(self.fov_y(1.0)) } else { self.fov };
        self.target = c;
        self.ortho_height = r * 2.4;
        self.distance = r / (fov * 0.5).sin() * 1.15;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_views_axes() {
        let top = ViewCamera::new(ViewKind::Top);
        assert!((top.forward() - -Vec3::Z).length() < 1e-5);
        assert!((top.right() - Vec3::X).length() < 1e-5);
        assert!((top.up() - Vec3::Y).length() < 1e-5);
        let front = ViewCamera::new(ViewKind::Front);
        assert!((front.forward() - Vec3::Y).length() < 1e-5);
        assert!((front.up() - Vec3::Z).length() < 1e-5);
        let left = ViewCamera::new(ViewKind::Left);
        assert!((left.forward() - Vec3::X).length() < 1e-5);
        assert!((left.right() - -Vec3::Y).length() < 1e-5);
    }

    #[test]
    fn orthographic_keeps_angle_and_framing() {
        let mut c = ViewCamera::new(ViewKind::Perspective);
        let (fwd, upp) = (c.forward(), c.units_per_pixel(c.target, 600.0));
        c.to_orthographic();
        assert!(c.is_ortho() && (c.forward() - fwd).length() < 1e-6);
        assert!((c.units_per_pixel(c.target, 600.0) - upp).abs() < upp * 1e-3);
        c.to_perspective();
        assert!(!c.is_ortho() && (c.distance - 7.0).abs() < 1e-3);
    }

    #[test]
    fn project_and_ray_roundtrip() {
        for kind in [ViewKind::Perspective, ViewKind::Top, ViewKind::Front] {
            let cam = ViewCamera::new(kind);
            let p = Vec3::new(0.3, -0.2, 0.5);
            let ndc = cam.project(p, 1.5).unwrap();
            let ray = cam.ray(ndc, 1.5);
            let closest = ray.origin + ray.dir * (p - ray.origin).dot(ray.dir);
            assert!((closest - p).length() < 1e-3, "{kind:?}");
            let c = cam.project(cam.target, 1.5).unwrap();
            assert!(c.length() < 1e-4);
        }
    }

    #[test]
    fn orbit_keeps_pivot_distance_and_zoom() {
        let mut cam = ViewCamera::new(ViewKind::Perspective);
        let pivot = Vec3::new(1.0, 1.0, 0.0);
        let d0 = (cam.eye() - pivot).length();
        cam.orbit(Vec2::new(40.0, 25.0), pivot);
        assert!(((cam.eye() - pivot).length() - d0).abs() < 1e-3);
        let dist = cam.distance;
        cam.zoom(1.0);
        assert!(cam.distance < dist);
        // camera views keep their kind and clip planes reach the projection
        let mut c = ViewCamera::new(ViewKind::Camera);
        c.orbit(Vec2::new(5.0, 0.0), Vec3::ZERO);
        assert_eq!(c.kind, ViewKind::Camera);
        assert!(!c.is_ortho());
        (c.near, c.far, c.hfov) = (1.0, 20.0, true);
        let ndc = |d: f32| c.view_proj(1.5).project_point3(c.eye() + c.forward() * d).z;
        assert!((ndc(1.0) - 1.0).abs() < 1e-3 && ndc(20.0).abs() < 1e-3, "reverse-Z near/far");
        assert!(c.fov_y(1.5) < c.fov);
        let mut top = ViewCamera::new(ViewKind::Top);
        top.orbit(Vec2::new(10.0, 0.0), Vec3::ZERO);
        assert_eq!(top.kind, ViewKind::User);
    }
}
