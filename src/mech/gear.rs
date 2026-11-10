//! Involute Gear: External Spur, Internal Ring or Linear Rack, with module,
//! pressure angle, thickness, bore and helix angle. The gear lies in the XY
//! plane (pivot at its base center), extruded along +Z.

use crate::mesh::{EditablePoly, VertexId};
use bevy::math::{Vec2, Vec3};
use std::f32::consts::{PI, TAU};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GearType {
    #[default]
    Spur,
    Ring,
    Rack,
}

impl GearType {
    pub const ALL: [GearType; 3] = [GearType::Spur, GearType::Ring, GearType::Rack];
    pub fn label(self) -> &'static str {
        match self {
            GearType::Spur => "External Spur",
            GearType::Ring => "Internal Ring",
            GearType::Rack => "Linear Rack",
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Gear {
    pub kind: GearType,
    pub teeth: u32,
    /// Module: pitch diameter per tooth (scene units).
    pub module: f32,
    pub pressure_angle: f32,
    pub thickness: f32,
    pub bore: f32,
    pub helix: f32,
}

impl Default for Gear {
    fn default() -> Self {
        Self { kind: GearType::Spur, teeth: 20, module: 0.1, pressure_angle: 20.0, thickness: 0.3, bore: 0.2, helix: 0.0 }
    }
}

/// Points sampled per involute flank.
const FLANK: usize = 5;

fn inv(phi: f32) -> f32 {
    phi.tan() - phi
}

impl Gear {
    pub fn teeth(&self) -> u32 {
        self.teeth.clamp(6, 200)
    }

    /// Pitch radius (where meshing gears roll on each other).
    pub fn pitch_radius(&self) -> f32 {
        self.module.max(1e-4) * self.teeth() as f32 * 0.5
    }

    /// Closed outline (counterclockwise) of an involute tooth ring between
    /// radii `rf` (root) and `ra` (tip).
    fn outline(&self, rf: f32, ra: f32) -> Vec<Vec2> {
        let z = self.teeth() as f32;
        let (r, alpha) = (self.pitch_radius(), self.pressure_angle.clamp(5.0, 40.0).to_radians());
        let rb = r * alpha.cos();
        // Half the tooth's angular width at radius rho.
        let psi = |rho: f32| (PI / (2.0 * z) + inv(alpha) - inv((rb / rho.max(rb)).clamp(-1.0, 1.0).acos())).max(0.002);
        let start = rf.max(rb);
        let radii: Vec<f32> = (0..=FLANK).map(|i| start + (ra - start) * i as f32 / FLANK as f32).collect();
        let at = |a: f32, rho: f32| Vec2::new(a.cos(), a.sin()) * rho;
        let mut out = vec![];
        for k in 0..self.teeth() {
            let c = TAU * k as f32 / z;
            if rf < rb {
                out.push(at(c - psi(rb), rf));
            }
            out.extend(radii.iter().map(|&rho| at(c - psi(rho), rho)));
            out.push(at(c, ra));
            out.extend(radii.iter().rev().map(|&rho| at(c + psi(rho), rho)));
            if rf < rb {
                out.push(at(c + psi(rb), rf));
            }
            out.push(at(c + PI / z, rf)); // middle of the root land
        }
        out
    }

    pub fn build(&self) -> EditablePoly {
        let m = self.module.max(1e-4);
        let r = self.pitch_radius();
        let polar = |p: &Vec2, rad: f32| Vec2::new(p.y.atan2(p.x).cos(), p.y.atan2(p.x).sin()) * rad;
        let (a, b, closed) = match self.kind {
            GearType::Spur => {
                let a = self.outline((r - 1.25 * m).max(m * 0.2), r + m);
                let bore = self.bore.clamp(0.0, (r - 1.25 * m) * 0.95);
                let b = if bore > 1e-4 { a.iter().map(|p| polar(p, bore)).collect() } else { vec![Vec2::ZERO] };
                (a, b, true)
            }
            GearType::Ring => {
                let a = self.outline(r - m, r + 1.25 * m);
                let rim = r + 1.25 * m + 2.5 * m;
                (a.clone(), a.iter().map(|p| polar(p, rim)).collect(), true)
            }
            GearType::Rack => {
                let (p, ta) = (PI * m, self.pressure_angle.clamp(5.0, 40.0).to_radians().tan());
                let n = self.teeth();
                let x0 = -p * n as f32 * 0.5;
                let mut a = vec![];
                for k in 0..n {
                    let cx = x0 + p * (k as f32 + 0.5);
                    let half = |h: f32| p / 4.0 - h * ta;
                    a.push(Vec2::new(cx - p / 2.0, -1.25 * m));
                    a.push(Vec2::new(cx - half(-1.25 * m).min(p / 2.0 - 1e-4), -1.25 * m));
                    a.push(Vec2::new(cx - half(m).max(0.002), m));
                    a.push(Vec2::new(cx + half(m).max(0.002), m));
                    a.push(Vec2::new(cx + half(-1.25 * m).min(p / 2.0 - 1e-4), -1.25 * m));
                }
                a.push(Vec2::new(-x0, -1.25 * m));
                let back = -1.25 * m - 2.0 * m;
                let b = a.iter().map(|q| Vec2::new(q.x, back)).collect();
                (a, b, false)
            }
        };
        let helix = self.helix.clamp(-60.0, 60.0).to_radians().tan();
        let rack = self.kind == GearType::Rack;
        extrude(&a, &b, closed, self.thickness.max(1e-3), |p, h| {
            if rack {
                Vec3::new(p.x + h * helix, p.y, h)
            } else {
                let t = h * helix / r;
                Vec3::new(p.x * t.cos() - p.y * t.sin(), p.x * t.sin() + p.y * t.cos(), h)
            }
        })
    }
}

fn area(pts: &[Vec2]) -> f32 {
    (0..pts.len()).map(|i| pts[i].perp_dot(pts[(i + 1) % pts.len()])).sum::<f32>() * 0.5
}

/// Extrudes the region between the matched point rows `a` and `b` (b may
/// be a single center point) from h = 0 to `height` in layers, placing each
/// point with `place(point, h)`. Closed rows wrap around; open rows are
/// joined at their ends. Caps are strips of quads between the rows.
fn extrude(a: &[Vec2], b: &[Vec2], closed: bool, height: f32, place: impl Fn(Vec2, f32) -> Vec3) -> EditablePoly {
    let layers = 6;
    let mut m = EditablePoly::new();
    let pts: Vec<Vec2> = a.iter().chain(b).copied().collect();
    let ids: Vec<Vec<VertexId>> = (0..=layers)
        .map(|l| {
            let h = height * l as f32 / layers as f32;
            // A single center point is only used by the two caps.
            let n = if b.len() == 1 && l != 0 && l != layers { a.len() } else { pts.len() };
            pts[..n].iter().map(|&p| m.add_vertex(place(p, h))).collect()
        })
        .collect();
    let (na, nb) = (a.len(), b.len());
    let bi = |i: usize| na + if nb == 1 { 0 } else { i % nb };
    let span = if closed { na } else { na - 1 };
    // Caps: top faces +Z, the bottom is mirrored.
    let first = [a[0], a[1 % na], pts[bi(1)], pts[bi(0)]];
    let flip_cap = area(&first) < 0.0;
    for (layer, top) in [(0, false), (layers, true)] {
        for i in 0..span {
            let mut q = vec![i, (i + 1) % na, bi(i + 1), bi(i)];
            q.dedup();
            if q.first() == q.last() {
                q.pop();
            }
            if top == flip_cap {
                q.reverse();
            }
            let vs: Vec<VertexId> = q.iter().map(|&k| ids[layer][k]).collect();
            let uv: Vec<Vec2> = q.iter().map(|&k| pts[k] * 0.5 + Vec2::splat(0.5)).collect();
            let p = m.add_polygon_uv(&vs, &uv);
            m.polygons[p].smoothing_group = 1;
        }
    }
    // Walls: each closed loop faces away from the solid.
    let wall_loops: Vec<(Vec<usize>, bool)> = if closed {
        let mut l = vec![((0..na).collect(), false)];
        if nb > 1 {
            l.push(((na..na + nb).collect(), true));
        }
        // A ring gear's toothed row is the inside boundary.
        if nb > 1 && area(b).abs() > area(a).abs() {
            l = l.into_iter().map(|(v, hole)| (v, !hole)).collect();
        }
        l
    } else {
        (vec![((0..na).chain((na..na + nb).rev()).collect(), false)]).into_iter().collect()
    };
    for (lp, hole) in wall_loops {
        let ring: Vec<Vec2> = lp.iter().map(|&k| pts[k]).collect();
        let keep = (area(&ring) > 0.0) != hole;
        let n = lp.len();
        for l in 0..layers {
            for i in 0..n {
                let (k0, k1) = (lp[i], lp[(i + 1) % n]);
                let mut q = [ids[l][k0], ids[l][k1], ids[l + 1][k1], ids[l + 1][k0]];
                if !keep {
                    q.reverse();
                }
                let (u0, u1) = (i as f32 / n as f32, (i + 1) as f32 / n as f32);
                let (v0, v1) = (l as f32 / layers as f32, (l + 1) as f32 / layers as f32);
                let mut uv = [Vec2::new(u0, v0), Vec2::new(u1, v0), Vec2::new(u1, v1), Vec2::new(u0, v1)];
                if !keep {
                    uv.reverse();
                }
                let p = m.add_polygon_uv(&q, &uv);
                m.polygons[p].smoothing_group = if hole { 4 } else { 2 };
            }
        }
    }
    m.rebuild_edges();
    m
}
