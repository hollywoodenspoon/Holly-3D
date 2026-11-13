//! The Option A knee support on an imported leg: cuts the fixed strut
//! (the cable/brace from the shin up to the thigh) out of the shin, then
//! sizes a carriage collar on the shin and a piston from the thigh to it.
//! All geometry here is in world space.

use super::pins::LegFit;
use crate::mech::PistonPart;
use crate::mech::spider::kit::{Kit, block, cyl_mid, mat};
use crate::mesh::EditablePoly;
use bevy::math::{Mat3, Mat4, Quat, Vec3};
use std::collections::HashSet;

/// Shin-local axes at the knee: `u` down the shin, `n` the pin axis,
/// `w` across the shin towards the hip.
#[derive(Clone, Copy, Debug)]
pub struct ShinAxes {
    pub o: Vec3,
    pub u: Vec3,
    pub n: Vec3,
    pub w: Vec3,
    pub len: f32,
}

impl ShinAxes {
    pub fn new(f: &LegFit) -> Self {
        let u = f.shin_dir();
        let n = (f.knee_axis - u * f.knee_axis.dot(u)).normalize_or(f.r * Vec3::NEG_Y);
        let mut w = u.cross(n);
        if w.dot(f.lift_at - f.knee_at) < 0.0 {
            w = -w;
        }
        Self { o: f.knee_at, u, n, w, len: f.shin_len() }
    }
    /// (along, pin, across) coordinates of `p`.
    pub fn local(&self, p: Vec3) -> Vec3 {
        let d = p - self.o;
        Vec3::new(d.dot(self.u), d.dot(self.n), d.dot(self.w))
    }
    pub fn world(&self, l: Vec3) -> Vec3 {
        self.o + self.u * l.x + self.n * l.y + self.w * l.z
    }
}

/// What was cut and where the new support goes.
#[derive(Clone, Debug)]
pub struct Support {
    /// Polygons removed from the shin.
    pub cut: usize,
    /// Carriage pivot on the shin and the rod eyelet on the carriage.
    pub carriage_at: Vec3,
    pub rod_eye: Vec3,
    /// Housing eyelet on the thigh.
    pub housing_eye: Vec3,
    /// Carriage collar, piston housing and rod meshes (world space).
    pub carriage: EditablePoly,
    pub housing: EditablePoly,
    pub rod: EditablePoly,
    /// Piston pose: eyelet pin along local Y, body along local +Z.
    pub housing_rot: Quat,
    pub rod_rot: Quat,
    /// Distance along the shin from the knee to the carriage at rest.
    pub along: f32,
}

/// Removes the strut faces from `shin` (a polygon whose every corner lies
/// well off the shin towards the hip and near the leg's mid-plane).
pub fn cut_strut(shin: &mut EditablePoly, ax: &ShinAxes) -> (usize, Vec<Vec3>) {
    let l = ax.len;
    let strut: HashSet<_> = shin
        .polygons
        .iter()
        .filter(|(_, p)| {
            p.corners.iter().all(|c| {
                let q = ax.local(shin.vertices[c.vertex_id].position);
                q.z > 0.075 * l && q.y.abs() < 0.09 * l
            })
        })
        .map(|(k, _)| k)
        .collect();
    let pts: Vec<Vec3> = strut.iter().flat_map(|&k| shin.polygons[k].corners.iter().map(|c| shin.vertices[c.vertex_id].position)).collect();
    if !strut.is_empty() {
        shin.polygons.retain(|k, _| !strut.contains(&k));
        shin.remove_isolated_vertices();
        shin.rebuild_edges();
    }
    (strut.len(), pts)
}

/// Cuts the strut and builds the carriage and piston for one leg.
pub fn support(shin: &mut EditablePoly, f: &LegFit) -> Support {
    let ax = ShinAxes::new(f);
    let l = ax.len;
    let (cut_count, pts) = cut_strut(shin, &ax);
    let loc: Vec<Vec3> = pts.iter().map(|&p| ax.local(p)).collect();
    // Shin end of the strut = where it left the shin; thigh end = its far tip.
    let (along, housing_eye) = if loc.is_empty() {
        (0.4 * l, f.knee_at + (f.lift_at - f.knee_at) * 0.45 + ax.w * 0.06 * l)
    } else {
        let cmin = loc.iter().map(|q| q.z).fold(f32::MAX, f32::min);
        let cmax = loc.iter().map(|q| q.z).fold(f32::MIN, f32::max);
        let base: Vec<_> = loc.iter().filter(|q| q.z < cmin + 0.03 * l).collect();
        let tip: Vec<_> = loc.iter().filter(|q| q.z > cmax - 0.03 * l).collect();
        let along = base.iter().map(|q| q.x).sum::<f32>() / base.len() as f32;
        let tip = tip.iter().copied().copied().sum::<Vec3>() / tip.len() as f32;
        (along.clamp(0.15 * l, 0.9 * l), ax.world(tip))
    };
    // Shin cross-section where the carriage rides.
    let sec: Vec<Vec3> = shin.vertices.values().map(|v| ax.local(v.position)).filter(|q| (q.x - along).abs() < 0.06 * l).collect();
    let (lo, hi) = if sec.is_empty() {
        (Vec3::splat(-0.04 * l), Vec3::splat(0.04 * l))
    } else {
        (sec.iter().copied().fold(Vec3::MAX, Vec3::min), sec.iter().copied().fold(Vec3::MIN, Vec3::max))
    };
    let mid = (lo + hi) * 0.5;
    let size = Vec3::new(0.12 * l, (hi.y - lo.y) * 1.2 + 0.01 * l, (hi.z - lo.z) * 1.2 + 0.01 * l);
    let carriage_at = ax.world(Vec3::new(along, mid.y, mid.z));
    let l0 = carriage_at.distance(housing_eye).max(0.05 * l);
    let eyelet = 0.085 * l0;
    let rod_eye = carriage_at + ax.w * (size.z * 0.5 + eyelet * 1.1);
    let carriage = collar(&ax, carriage_at, size, rod_eye, eyelet);
    let dir = (rod_eye - housing_eye).normalize_or(ax.w);
    let len = rod_eye.distance(housing_eye);
    let housing = PistonPart { rod: false, radius: 0.09 * len, length: 0.55 * len, eyelet, sides: 16 };
    let rod = PistonPart { rod: true, radius: 0.045 * len, length: 0.6 * len, eyelet: eyelet * 0.85, sides: 12 };
    let (housing_rot, rod_rot) = (aim(dir, ax.n), aim(-dir, ax.n));
    let place = |p: PistonPart, q: Quat, at: Vec3, id: u16| {
        let mut k = Kit::default();
        k.add(p.build(), id);
        let mut m = k.finish();
        m.transform(Mat4::from_rotation_translation(q, at));
        m
    };
    Support {
        cut: cut_count,
        carriage_at,
        rod_eye,
        housing_eye,
        carriage,
        housing: place(housing, housing_rot, housing_eye, mat::BARREL),
        rod: place(rod, rod_rot, rod_eye, mat::ROD),
        housing_rot,
        rod_rot,
        along,
    }
}

/// Rotation taking local +Z to `dir` and local Y to (the part of) `pin`
/// across it.
pub fn aim(dir: Vec3, pin: Vec3) -> Quat {
    let z = dir.normalize_or(Vec3::Z);
    let y = (pin - z * pin.dot(z)).normalize_or(z.any_orthonormal_vector());
    Quat::from_mat3(&Mat3::from_cols(y.cross(z), y, z)).normalize()
}

/// Collar box round the shin with a lug pair and pin for the rod eyelet.
fn collar(ax: &ShinAxes, at: Vec3, size: Vec3, eye: Vec3, e: f32) -> EditablePoly {
    // Local frame: X across (towards the hip), Y the pin, Z up the shin.
    let basis = Mat4::from_cols(ax.w.extend(0.0), ax.n.extend(0.0), (-ax.u).extend(0.0), at.extend(1.0));
    let mut k = Kit::default();
    k.add(block(Vec3::new(size.z, size.y, size.x), Vec3::ZERO), mat::BARREL);
    let eye_l = basis.inverse().transform_point3(eye);
    let lug = Vec3::new(eye_l.x - size.z * 0.5 + e, e * 0.4, e * 2.0);
    for s in [-1.0, 1.0] {
        let c = Vec3::new((size.z * 0.5 + eye_l.x) * 0.5 + e * 0.25, s * e * 0.9, 0.0);
        k.add(block(lug, c), mat::BARREL);
    }
    k.add(cyl_mid(e * 0.45, e * 2.6, 10, Vec3::new(eye_l.x, 0.0, 0.0), Vec3::Y), mat::JOINT);
    let mut m = k.finish();
    m.transform(basis);
    m
}

/// Carriage travel per degree of knee that keeps the piston near its rest
/// length (60% of the first-order ideal, so the piston still strokes).
pub fn link_ratio(f: &LegFit, s: &Support) -> f32 {
    let u = f.shin_dir();
    let length = |deg: f32, slide: f32| {
        let q = Quat::from_axis_angle(f.knee_axis, deg.to_radians());
        (f.knee_at + q * (s.rod_eye + u * slide - f.knee_at)).distance(s.housing_eye)
    };
    let h = 0.5;
    let dl_dt = (length(h, 0.0) - length(-h, 0.0)) / (2.0 * h);
    let ds = 0.01 * f.shin_len();
    let dl_ds = (length(0.0, ds) - length(0.0, -ds)) / (2.0 * ds);
    if dl_ds.abs() < 1e-5 { 0.0 } else { -0.6 * dl_dt / dl_ds }
}
