//! Small shape builders shared by hair, outfits and creatures: sphere
//! patches (caps, beards, shells), spikes, ellipsoids and swept tube
//! chains (ponytails, tails, legs, serpent bodies).

use crate::generators::kit::Kit;
use bevy::math::{Mat4, Quat, Vec3};
use std::f32::consts::{PI, TAU};

/// Direction on the unit sphere: azimuth 0 = facing -Y (the character's
/// front), elevation in radians (+ up).
pub fn dir(az: f32, el: f32) -> Vec3 {
    Vec3::new(az.sin() * el.cos(), -az.cos() * el.cos(), el.sin())
}

/// A lat-long sphere patch around `c` with per-direction radius `r(d)`;
/// cells whose centre direction fails `keep(az, el)` are skipped. Smooth.
pub fn patch(kit: &mut Kit, c: Vec3, segs: (usize, usize), r: impl Fn(Vec3) -> f32, keep: impl Fn(f32, f32) -> bool) {
    let (nu, nv) = segs;
    let pt = |i: usize, j: usize| {
        let (az, el) = (TAU * i as f32 / nu as f32, -PI / 2.0 + PI * j as f32 / nv as f32);
        let d = dir(az, el);
        c + d * r(d)
    };
    let sg = kit.sg;
    kit.sg = 1;
    for j in 0..nv {
        for i in 0..nu {
            let (az, el) = (TAU * (i as f32 + 0.5) / nu as f32, -PI / 2.0 + PI * (j as f32 + 0.5) / nv as f32);
            let az = if az > PI { az - TAU } else { az };
            if keep(az, el) {
                kit.quad(pt(i, j), pt(i + 1, j), pt(i + 1, j + 1), pt(i, j + 1));
            }
        }
    }
    kit.sg = sg;
}

/// A full ellipsoid with semi-axes `r`.
pub fn ellipsoid(kit: &mut Kit, c: Vec3, r: Vec3, segs: usize) {
    patch(kit, c, (segs, segs / 2), |d| ellipse_r(d, r), |_, _| true);
}

fn ellipse_r(d: Vec3, r: Vec3) -> f32 {
    1.0 / ((d / r).length()).max(1e-6)
}

/// A cone from `base` pointing along `axis` (`len` long).
pub fn spike(kit: &mut Kit, base: Vec3, axis: Vec3, radius: f32, len: f32, sides: u32) {
    let a = axis.normalize_or(Vec3::Z);
    kit.at(Mat4::from_rotation_translation(Quat::from_rotation_arc(Vec3::Z, a), base), |k| k.frustum(Vec3::ZERO, radius, 0.0, len, sides, true));
}

/// A smooth tube swept through `pts` with per-point radii (rings are
/// oriented by the averaged tangent; ends capped).
pub fn chain(kit: &mut Kit, pts: &[Vec3], radii: &[f32], sides: usize) {
    if pts.len() < 2 {
        return;
    }
    let n = pts.len();
    let mut up = Vec3::Z;
    let mut rings = Vec::with_capacity(n);
    for i in 0..n {
        let t = (pts[(i + 1).min(n - 1)] - pts[i.saturating_sub(1)]).normalize_or(Vec3::X);
        if t.cross(up).length_squared() < 1e-4 {
            up = Vec3::X;
        }
        let side = t.cross(up).normalize();
        up = side.cross(t).normalize();
        let r = radii[i.min(radii.len() - 1)].max(1e-4);
        let ring: Vec<_> = (0..sides)
            .map(|s| {
                let a = TAU * s as f32 / sides as f32;
                kit.vtx(pts[i] + (side * a.cos() + up * a.sin()) * r)
            })
            .collect();
        rings.push(ring);
    }
    let sg = kit.sg;
    kit.sg = 1;
    for w in rings.windows(2) {
        for s in 0..sides {
            let s2 = (s + 1) % sides;
            kit.face(&[w[0][s], w[1][s], w[1][s2], w[0][s2]]);
        }
    }
    kit.sg = sg;
    let last: Vec<_> = rings[n - 1].iter().rev().copied().collect();
    kit.face(&rings[0].clone());
    kit.face(&last);
}
