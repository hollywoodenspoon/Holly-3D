//! Joint pivots on imported parts. A hinge sits on the bolt (a thin pin
//! island) nearest the contact patch between the two parts, with its axis
//! along the bolt; without a bolt the pivot is the contact patch centre and
//! the axis is the leg-plane normal, like the built-in rig.

use super::classify::{LegParts, Part};
use super::geom::{Island, contact, flat, islands};
use bevy::math::{Quat, Vec3};

/// Where one leg's joints go (world space).
#[derive(Clone, Copy, Debug)]
pub struct LegFit {
    /// Leg frame: +X out along the leg, +Z up, Y the hinge-pin axis.
    pub r: Quat,
    pub swing_at: Vec3,
    pub lift_at: Vec3,
    pub lift_axis: Vec3,
    pub knee_at: Vec3,
    pub knee_axis: Vec3,
    pub ankle_at: Vec3,
    /// How many of the three hinges found a bolt.
    pub bolts: usize,
}

impl LegFit {
    /// Down the shin, knee to ankle.
    pub fn shin_dir(&self) -> Vec3 {
        (self.ankle_at - self.knee_at).normalize_or(Vec3::NEG_Z)
    }
    pub fn shin_len(&self) -> f32 {
        self.ankle_at.distance(self.knee_at)
    }
}

/// Centre of the patch where `a` and `b` touch.
pub fn contact_centre(a: &Part, b: &Part) -> Option<Vec3> {
    let mut pts = contact(&a.samples, &b.cloud);
    pts.extend(contact(&b.samples, &a.cloud));
    (!pts.is_empty()).then(|| pts.iter().copied().sum::<Vec3>() / pts.len() as f32)
}

/// Bolt islands of a part (never its main body).
pub fn bolts(p: &Part) -> Vec<Island> {
    let mut all = islands(&p.mesh);
    // The main body is the biggest island by extent (a bolt can have more vertices).
    let size = |i: &Island| (i.max - i.min).length();
    if let Some(k) = (0..all.len()).max_by(|&a, &b| size(&all[a]).total_cmp(&size(&all[b]))) {
        all.remove(k);
    }
    all.into_iter().filter(Island::is_pin).collect()
}

/// Hinge between `a` and `b`: pivot and axis, with the axis sign matching
/// `reference`; `true` when a bolt was found.
fn hinge(a: &Part, b: &Part, pins: &[Island], reference: Vec3, reach: f32) -> Option<(Vec3, Vec3, bool)> {
    let c = contact_centre(a, b)?;
    let near = pins.iter().filter(|p| p.centre.distance(c) < reach.max(p.spread.z * 2.0)).min_by(|x, y| x.centre.distance(c).total_cmp(&y.centre.distance(c)));
    Some(match near {
        Some(p) if p.axes[2].dot(reference).abs() > 0.5 => {
            let ax = p.axes[2] * p.axes[2].dot(reference).signum();
            (p.centre, ax, true)
        }
        _ => (c, reference, false),
    })
}

/// Fits the joints of one leg of the mech whose hull centre is `hub`.
pub fn fit_leg(parts: &[Part], leg: &LegParts, hub: Vec3, reach: f32) -> Result<LegFit, String> {
    let [ring, thigh, shin, foot] = [leg.ring, leg.thigh, leg.shin, leg.foot].map(|i| &parts[i]);
    let out = flat(foot.bb.center() - hub);
    let r = Quat::from_rotation_z(out.y.atan2(out.x));
    let pin = r * Vec3::NEG_Y;
    let mut pins: Vec<Island> = [ring, thigh, shin, foot].iter().flat_map(|p| bolts(p)).collect();
    let size = |i: &Island| (i.max - i.min).length();
    let main = islands(&ring.mesh).into_iter().max_by(|a, b| size(a).total_cmp(&size(b))).ok_or("A hip ring has no polygons")?;
    let miss = |what: &str| format!("Could not find where the {what} touches");
    let (lift_at, lift_axis, b1) = hinge(ring, thigh, &pins, pin, reach).ok_or_else(|| miss("thigh and hip ring"))?;
    pins.retain(|p| p.centre.distance(lift_at) > 1e-4);
    let (knee_at, knee_axis, b2) = hinge(thigh, shin, &pins, pin, reach).ok_or_else(|| miss("shin and thigh"))?;
    pins.retain(|p| p.centre.distance(knee_at) > 1e-4);
    let (ankle_at, _, b3) = hinge(shin, foot, &pins, pin, reach).ok_or_else(|| miss("foot and shin"))?;
    // The swing turns about the ring's vertical axis, at the hip-lift height.
    let swing_at = Vec3::new(main.centre.x, main.centre.y, lift_at.z.min(main.centre.z));
    Ok(LegFit { r, swing_at, lift_at, lift_axis, knee_at, knee_axis, ankle_at, bolts: [b1, b2, b3].iter().filter(|b| **b).count() })
}

/// The gun's lift trunnion near its breech, the lift axis (positive raises
/// the muzzle) and its rest elevation in degrees.
pub fn fit_gun(gun: &Part, turret: &Part) -> (Vec3, Vec3, f32) {
    let pts: Vec<Vec3> = gun.mesh.vertices.values().map(|v| v.position).collect();
    let (c, axes, _) = super::geom::pca(&pts);
    let mut g = axes[2];
    if g.dot(c - turret.bb.center()) < 0.0 {
        g = -g;
    }
    let s = pts.iter().map(|p| (*p - c).dot(g));
    let (lo, hi) = s.fold((f32::MAX, f32::MIN), |(a, b), v| (a.min(v), b.max(v)));
    let at = c + g * (lo + 0.12 * (hi - lo));
    let axis = g.cross(bevy::math::Vec3::Z).normalize_or(Vec3::NEG_X);
    (at, axis, g.z.clamp(-1.0, 1.0).asin().to_degrees())
}

/// The turret's turn pivot: the centre of where it sits on the rest of the
/// mech (anything but the gun), on a vertical axis.
pub fn fit_turret(parts: &[Part], turret: usize, gun: usize) -> Vec3 {
    let t = &parts[turret];
    let mut pts = vec![];
    for (i, p) in parts.iter().enumerate() {
        if i != turret && i != gun && !(p.bb.min - 1e-3).cmpgt(t.bb.max).any() && !(t.bb.min - 1e-3).cmpgt(p.bb.max).any() {
            pts.extend(contact(&t.samples, &p.cloud));
        }
    }
    let c = t.bb.center();
    if pts.is_empty() {
        return Vec3::new(c.x, c.y, t.bb.min.z);
    }
    pts.iter().copied().sum::<Vec3>() / pts.len() as f32
}
