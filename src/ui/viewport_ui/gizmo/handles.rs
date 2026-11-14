//! Gizmo layout on screen and hit-testing: which handle of the Move,
//! Rotate or Scale gizmo is under the cursor. Pure math (no egui), so the
//! picking is covered by headless tests.

use crate::editor::TransformMode;
use crate::viewport::layout::ViewState;
use bevy::math::{Quat, Vec2, Vec3};

/// Move plane squares: side length as a fraction of the axis length.
pub const PLANE: f32 = 0.3;
/// Scale: inner (uniform) triangle and outer edge of the plane strips.
pub const SCALE_IN: f32 = 0.35;
pub const SCALE_OUT: f32 = 0.6;
/// Rotate: the grey view ring sits this much outside the axis rings.
pub const VIEW_RING: f32 = 1.18;
/// Pick tolerance in pixels.
pub const PICK: f32 = 8.0;
/// Samples around a rotate ring.
pub const RING_STEPS: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Handle {
    /// X, Y or Z (0, 1, 2).
    Axis(usize),
    /// A plane, named by its normal axis: 0 = YZ, 1 = ZX, 2 = XY.
    Plane(usize),
    /// Move in the view plane (the small centre square).
    Screen,
    /// Rotate about the view direction (outer grey ring).
    ViewRing,
    /// Free rotation (inside the inner grey circle).
    Trackball,
    /// Scale all three axes (inner triangle).
    Uniform,
}

/// The two axes spanning plane `k` (cyclic, so `a × b` = axis `k`).
pub fn plane_axes(k: usize) -> (usize, usize) {
    ((k + 1) % 3, (k + 2) % 3)
}

/// A gizmo placed in one viewport.
pub struct Gz<'a> {
    pub v: &'a ViewState,
    pub center: Vec3,
    pub rot: Quat,
    pub axes: [Vec3; 3],
    /// World length of an axis (constant size on screen).
    pub len: f32,
    /// Axis length in pixels.
    pub px: f32,
    /// Unit vector from the centre towards the viewer.
    pub to_cam: Vec3,
    pub c2: Vec2,
}

impl<'a> Gz<'a> {
    pub fn new(v: &'a ViewState, center: Vec3, rot: Quat, size_px: f32) -> Option<Self> {
        let c2 = v.to_screen(center)?;
        if !v.rect.contains(c2) && (c2 - v.rect.center()).length() > v.rect.size().length() {
            return None;
        }
        let cam = &v.camera;
        let len = cam.units_per_pixel(center, v.rect.size().y) * size_px;
        let to_cam = if cam.is_ortho() { -cam.forward() } else { (cam.eye() - center).normalize_or(-cam.forward()) };
        let axes = [rot * Vec3::X, rot * Vec3::Y, rot * Vec3::Z];
        Some(Self { v, center, rot, axes, len, px: size_px, to_cam, c2 })
    }

    /// World point to screen (falls back to the centre behind the camera).
    pub fn s(&self, p: Vec3) -> Vec2 {
        self.v.to_screen(p).unwrap_or(self.c2)
    }

    /// Screen point at `f` axis lengths along axis `k`.
    pub fn along(&self, k: usize, f: f32) -> Vec2 {
        self.s(self.center + self.axes[k] * self.len * f)
    }

    /// 0 when axis `k` points straight at the viewer, 1 when it lies flat.
    pub fn axis_vis(&self, k: usize) -> f32 {
        ((self.along(k, 1.0) - self.c2).length() / self.px.max(1.0)).clamp(0.0, 1.0)
    }

    pub fn axis_shown(&self, k: usize) -> bool {
        self.axis_vis(k) > 0.2
    }

    /// A plane handle is hidden when it is seen edge-on.
    pub fn plane_shown(&self, k: usize) -> bool {
        self.axes[k].dot(self.to_cam).abs() > 0.15
    }

    /// Corners of the Move plane square for plane `k`.
    pub fn move_square(&self, k: usize) -> [Vec2; 4] {
        let (a, b) = plane_axes(k);
        let (ua, ub) = (self.axes[a] * self.len * PLANE, self.axes[b] * self.len * PLANE);
        [self.c2, self.s(self.center + ua), self.s(self.center + ua + ub), self.s(self.center + ub)]
    }

    /// Scale plane strip for plane `k` (between the two triangles).
    pub fn scale_strip(&self, k: usize) -> [Vec2; 4] {
        let (a, b) = plane_axes(k);
        [self.along(a, SCALE_IN), self.along(b, SCALE_IN), self.along(b, SCALE_OUT), self.along(a, SCALE_OUT)]
    }

    pub fn scale_triangle(&self) -> [Vec2; 3] {
        [self.along(0, SCALE_IN), self.along(1, SCALE_IN), self.along(2, SCALE_IN)]
    }

    /// Point on rotate ring `k` at angle `t` (world).
    pub fn ring_point(&self, k: usize, t: f32) -> Vec3 {
        let (a, b) = plane_axes(k);
        self.center + (self.axes[a] * t.cos() + self.axes[b] * t.sin()) * self.len
    }

    /// Whether a ring point faces the viewer (front half).
    pub fn front(&self, p: Vec3) -> bool {
        (p - self.center).dot(self.to_cam) >= -0.02 * self.len
    }

    /// Ring radius on screen (perspective makes it vary a little).
    pub fn ring_px(&self) -> f32 {
        let right = self.v.camera.right();
        (self.s(self.center + right * self.len) - self.c2).length().max(1.0)
    }

    /// Closest front-facing sample on ring `k`: (pixel distance, angle).
    pub fn ring_hit(&self, k: usize, cur: Vec2) -> Option<(f32, f32)> {
        let step = std::f32::consts::TAU / RING_STEPS as f32;
        let mut best: Option<(f32, f32)> = None;
        for i in 0..RING_STEPS {
            let (t0, t1) = (i as f32 * step, (i + 1) as f32 * step);
            let mid = self.ring_point(k, (t0 + t1) * 0.5);
            if !self.front(mid) {
                continue;
            }
            let (p0, p1) = (self.s(self.ring_point(k, t0)), self.s(self.ring_point(k, t1)));
            let (d, f) = seg_dist(cur, p0, p1);
            if best.is_none_or(|b| d < b.0) {
                best = Some((d, t0 + f * step));
            }
        }
        best
    }

    /// Handle under `cur` for the given gizmo mode.
    pub fn pick(&self, mode: TransformMode, cur: Vec2) -> Option<Handle> {
        match mode {
            TransformMode::Select => None,
            TransformMode::Move => self.pick_move(cur),
            TransformMode::Rotate => self.pick_rotate(cur),
            TransformMode::Scale => self.pick_scale(cur),
        }
    }

    fn pick_axes(&self, cur: Vec2, from: f32) -> Option<Handle> {
        (0..3)
            .filter(|&k| self.axis_shown(k))
            .map(|k| (seg_dist(cur, self.along(k, from), self.along(k, 1.12)).0, k))
            .filter(|(d, _)| *d < PICK)
            .min_by(|a, b| a.0.total_cmp(&b.0))
            .map(|(_, k)| Handle::Axis(k))
    }

    /// Of the shown planes whose handle contains the cursor, the one whose
    /// centre is nearest (handles overlap on screen in perspective).
    fn best_plane<const N: usize>(&self, cur: Vec2, shape: impl Fn(usize) -> [Vec2; N]) -> Option<usize> {
        let dist = |k: usize| (shape(k).iter().sum::<Vec2>() / N as f32 - cur).length();
        (0..3)
            .filter(|&k| self.plane_shown(k) && in_poly(cur, &shape(k)))
            .min_by(|&a, &b| dist(a).total_cmp(&dist(b)))
    }

    fn pick_move(&self, cur: Vec2) -> Option<Handle> {
        if (cur - self.c2).length() < 6.0 {
            return Some(Handle::Screen);
        }
        if let Some(k) = self.best_plane(cur, |k| self.move_square(k)) {
            return Some(Handle::Plane(k));
        }
        self.pick_axes(cur, 0.0)
    }

    fn pick_rotate(&self, cur: Vec2) -> Option<Handle> {
        let ring = (0..3)
            .filter_map(|k| self.ring_hit(k, cur).map(|(d, _)| (d, k)))
            .filter(|(d, _)| *d < PICK)
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((_, k)) = ring {
            return Some(Handle::Axis(k));
        }
        let r = self.ring_px();
        let d = (cur - self.c2).length();
        if (d - r * VIEW_RING).abs() < PICK {
            Some(Handle::ViewRing)
        } else if d < r {
            Some(Handle::Trackball)
        } else {
            None
        }
    }

    fn pick_scale(&self, cur: Vec2) -> Option<Handle> {
        if in_poly(cur, &self.scale_triangle()) {
            return Some(Handle::Uniform);
        }
        if let Some(k) = self.best_plane(cur, |k| self.scale_strip(k)) {
            return Some(Handle::Plane(k));
        }
        self.pick_axes(cur, 0.0)
    }
}

/// Distance from `p` to segment `a`–`b`, and where along it (0..1).
pub fn seg_dist(p: Vec2, a: Vec2, b: Vec2) -> (f32, f32) {
    let ab = b - a;
    let f = if ab.length_squared() < 1e-9 { 0.0 } else { ((p - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0) };
    ((a + ab * f - p).length(), f)
}

/// Point inside a convex polygon (either winding). Degenerate polygons
/// (seen edge-on) contain nothing.
pub fn in_poly(p: Vec2, pts: &[Vec2]) -> bool {
    let area: f32 = (0..pts.len()).map(|i| pts[i].perp_dot(pts[(i + 1) % pts.len()])).sum();
    if area.abs() < 4.0 {
        return false;
    }
    (0..pts.len()).all(|i| {
        let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
        (b - a).perp_dot(p - a) * area.signum() >= 0.0
    })
}
