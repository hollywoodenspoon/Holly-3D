//! Orientation Cube math: picking a face / edge / corner of the cube and
//! turning it into a camera direction, plus the short eased camera flight.
//! Pure functions so they are unit-tested without a window.

use super::camera::{ViewCamera, ViewKind};
use bevy::math::{IVec3, Vec2, Vec3};
use std::f32::consts::{FRAC_PI_2, PI, TAU};

/// In-face coordinates beyond this pick an edge or corner instead of the face.
pub const EDGE_BAND: f32 = 0.6;
/// Seconds a cube click takes to fly the camera.
pub const FLIGHT_SECS: f32 = 0.3;

/// The six faces: outward direction and label (Front looks along +Y).
pub const FACES: [(IVec3, &str); 6] = [
    (IVec3::new(0, 0, 1), "TOP"),
    (IVec3::new(0, 0, -1), "BOTTOM"),
    (IVec3::new(0, -1, 0), "FRONT"),
    (IVec3::new(0, 1, 0), "BACK"),
    (IVec3::new(-1, 0, 0), "LEFT"),
    (IVec3::new(1, 0, 0), "RIGHT"),
];

/// Yaw and pitch of a camera looking at the cube from direction `d`
/// (a face, edge or corner: components in -1..=1, not all zero).
pub fn yaw_pitch(d: IVec3) -> (f32, f32) {
    let f = -d.as_vec3().normalize();
    let pitch = f.z.clamp(-1.0, 1.0).asin();
    let yaw = if f.x.abs() < 1e-6 && f.y.abs() < 1e-6 { FRAC_PI_2 } else { f.y.atan2(f.x) };
    (yaw, pitch)
}

/// The named view a cube direction gives an orthographic camera.
pub fn ortho_kind(d: IVec3) -> ViewKind {
    match (d.x, d.y, d.z) {
        (0, 0, 1) => ViewKind::Top,
        (0, -1, 0) => ViewKind::Front,
        (-1, 0, 0) => ViewKind::Left,
        _ => ViewKind::User,
    }
}

/// Screen axes of the cube widget for `cam`: (right, up, forward).
pub fn axes(cam: &ViewCamera) -> (Vec3, Vec3, Vec3) {
    (cam.right(), cam.up(), cam.forward())
}

/// World point on the unit cube under widget coordinate `s` (cube half-size
/// = 1, x right, y up), with the face it lies on.
pub fn hit(cam: &ViewCamera, s: Vec2) -> Option<(Vec3, IVec3)> {
    let (r, u, f) = axes(cam);
    let origin = r * s.x + u * s.y - f * 10.0;
    let (mut t_near, mut t_far, mut axis) = (f32::NEG_INFINITY, f32::INFINITY, 0usize);
    for i in 0..3 {
        if f[i].abs() < 1e-7 {
            if origin[i].abs() > 1.0 {
                return None;
            }
            continue;
        }
        let (a, b) = ((-1.0 - origin[i]) / f[i], (1.0 - origin[i]) / f[i]);
        let (lo, hi) = (a.min(b), a.max(b));
        if lo > t_near {
            (t_near, axis) = (lo, i);
        }
        t_far = t_far.min(hi);
    }
    if t_near > t_far {
        return None;
    }
    let p = origin + f * t_near;
    let mut n = IVec3::ZERO;
    n[axis] = if f[axis] > 0.0 { -1 } else { 1 };
    Some((p, n))
}

/// Face, edge or corner direction for point `p` on face `n`.
pub fn target_at(p: Vec3, n: IVec3) -> IVec3 {
    let mut d = n;
    for i in 0..3 {
        if n[i] == 0 && p[i].abs() > EDGE_BAND {
            d[i] = if p[i] > 0.0 { 1 } else { -1 };
        }
    }
    d
}

/// What the cursor at widget coordinate `s` points at on the cube.
pub fn pick(cam: &ViewCamera, s: Vec2) -> Option<IVec3> {
    hit(cam, s).map(|(p, n)| target_at(p, n))
}

/// The camera after flying to cube direction `d` (same target and zoom).
pub fn camera_for(cam: &ViewCamera, d: IVec3) -> ViewCamera {
    let mut c = *cam;
    (c.yaw, c.pitch) = yaw_pitch(d);
    if c.is_ortho() {
        c.kind = ortho_kind(d);
    }
    c
}

/// Switches between perspective and orthographic, keeping the framing.
pub fn toggle_projection(cam: &mut ViewCamera) {
    let half = (cam.fov * 0.5).tan();
    if cam.is_ortho() {
        cam.distance = (cam.ortho_height * 0.5 / half).max(0.05);
        cam.kind = ViewKind::Perspective;
    } else {
        cam.ortho_height = (2.0 * cam.distance * half).max(0.01);
        cam.kind = ViewKind::User;
    }
}

/// A camera flight from one view to another.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flight {
    pub from: ViewCamera,
    pub to: ViewCamera,
    pub t: f32,
}

impl Flight {
    pub fn new(from: ViewCamera, to: ViewCamera) -> Self {
        Self { from, to, t: 0.0 }
    }

    /// Advances by `dt` seconds; returns the camera to show and whether it landed.
    pub fn step(&mut self, dt: f32) -> (ViewCamera, bool) {
        self.t = (self.t + dt / FLIGHT_SECS).min(1.0);
        if self.t >= 1.0 {
            return (self.to, true);
        }
        let k = self.t * self.t * (3.0 - 2.0 * self.t);
        let (a, b) = (self.from, self.to);
        let mut c = b;
        // Shortest way round for yaw.
        let dy = (b.yaw - a.yaw + PI).rem_euclid(TAU) - PI;
        c.yaw = a.yaw + dy * k;
        c.pitch = a.pitch + (b.pitch - a.pitch) * k;
        c.target = a.target.lerp(b.target, k);
        c.distance = a.distance + (b.distance - a.distance) * k;
        c.ortho_height = a.ortho_height + (b.ortho_height - a.ortho_height) * k;
        // Named views only once the camera is there.
        if c.is_ortho() && b.kind != a.kind {
            c.kind = ViewKind::User;
        }
        (c, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    #[test]
    fn faces_match_the_standard_views() {
        for (kind, d) in [(ViewKind::Top, IVec3::Z), (ViewKind::Front, -IVec3::Y), (ViewKind::Left, -IVec3::X)] {
            let std = ViewCamera::new(kind);
            let (y, p) = yaw_pitch(d);
            assert!(close(y, std.yaw) && close(p, std.pitch), "{kind:?}");
            assert_eq!(ortho_kind(d), kind);
        }
        // Edges sit at 45°, corners are isometric (35.26° down).
        let (y, p) = yaw_pitch(IVec3::new(0, -1, 1));
        assert!(close(y, FRAC_PI_2) && close(p, -FRAC_PI_2 * 0.5));
        let (y, p) = yaw_pitch(IVec3::new(1, -1, 1));
        assert!(close(y, 135f32.to_radians()) && close(p, -(1.0f32 / 3f32.sqrt()).asin()));
        assert_eq!(ortho_kind(IVec3::new(1, -1, 1)), ViewKind::User);
    }

    #[test]
    fn picking_finds_faces_edges_and_corners() {
        // Looking straight at the Front face.
        let cam = ViewCamera::new(ViewKind::Front);
        assert_eq!(pick(&cam, Vec2::ZERO), Some(-IVec3::Y));
        assert_eq!(pick(&cam, Vec2::new(0.0, 0.8)), Some(IVec3::new(0, -1, 1)), "top edge");
        assert_eq!(pick(&cam, Vec2::new(-0.8, 0.0)), Some(IVec3::new(-1, -1, 0)), "left edge");
        assert_eq!(pick(&cam, Vec2::new(0.9, -0.9)), Some(IVec3::new(1, -1, -1)), "corner");
        assert_eq!(pick(&cam, Vec2::new(1.5, 0.0)), None, "outside");
        // From the Top the centre is TOP and screen-up is +Y (Back).
        let top = ViewCamera::new(ViewKind::Top);
        assert_eq!(pick(&top, Vec2::ZERO), Some(IVec3::Z));
        assert_eq!(pick(&top, Vec2::new(0.0, 0.9)), Some(IVec3::new(0, 1, 1)));
        // From the default perspective the near top corner is visible.
        let persp = ViewCamera::new(ViewKind::Perspective);
        let corner = Vec3::new(1.0, -1.0, 1.0) * 0.98;
        let s = Vec2::new(corner.dot(persp.right()), corner.dot(persp.up()));
        assert_eq!(pick(&persp, s), Some(IVec3::new(1, -1, 1)));
    }

    #[test]
    fn every_target_is_reachable_and_flies_there() {
        let mut n = 0;
        for x in -1..=1 {
            for y in -1..=1 {
                for z in -1..=1 {
                    let d = IVec3::new(x, y, z);
                    if d == IVec3::ZERO {
                        continue;
                    }
                    n += 1;
                    let start = ViewCamera::new(ViewKind::Perspective);
                    let to = camera_for(&start, d);
                    assert!((to.forward() + d.as_vec3().normalize()).length() < 1e-4, "{d}");
                    let mut f = Flight::new(start, to);
                    let mut landed = false;
                    for _ in 0..40 {
                        let (c, done) = f.step(1.0 / 60.0);
                        assert!(c.yaw.is_finite() && c.pitch.is_finite());
                        landed = done;
                    }
                    assert!(landed && f.step(0.0).0 == to);
                }
            }
        }
        assert_eq!(n, 26);
    }

    #[test]
    fn home_round_trips_and_projection_keeps_framing() {
        let mut cam = ViewCamera::new(ViewKind::Perspective);
        cam.target = Vec3::new(2.0, -1.0, 0.5);
        cam.distance = 12.0;
        let home = cam;
        cam.orbit(Vec2::new(80.0, -30.0), cam.target);
        cam.zoom(3.0);
        let mut f = Flight::new(cam, home);
        let (back, done) = f.step(1.0);
        assert!(done && back == home);
        // Ortho height matches the perspective view at the target, and back.
        let mut c = home;
        toggle_projection(&mut c);
        assert!(c.is_ortho() && close(c.ortho_height, 2.0 * 12.0 * (c.fov * 0.5).tan()));
        toggle_projection(&mut c);
        assert!(!c.is_ortho() && close(c.distance, 12.0));
        // Yaw takes the short way round across ±π.
        let (mut a, mut b) = (home, home);
        (a.yaw, b.yaw) = (PI - 0.1, -PI + 0.1);
        let (mid, _) = Flight::new(a, b).step(FLIGHT_SECS * 0.5);
        assert!(mid.yaw.cos() < -0.99, "went the long way: {}", mid.yaw);
    }
}
