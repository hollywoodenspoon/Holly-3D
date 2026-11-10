//! Roof and arch shapes for the kit: gabled and pyramid / hipped roofs,
//! conical spires, round and pointed (Gothic) arch curves, and an arched
//! wall face (the wall above an arch, built as a column of quads).

use super::Kit;
use bevy::math::{Vec2, Vec3};
use std::f32::consts::PI;

/// Arch curve from (-span/2, 0) to (span/2, 0) through `rise` at the
/// crown: a semicircle-like round arch, or a two-centred pointed arch.
pub fn arch_curve(span: f32, rise: f32, pointed: bool, n: usize) -> Vec<Vec2> {
    let n = n.max(2);
    let hw = span * 0.5;
    let rise = if pointed { rise.max(hw) } else { rise };
    (0..=n)
        .map(|i| {
            let t = i as f32 / n as f32;
            if pointed {
                // Each half is an arc whose centre sits on the far springing line.
                let r = (hw * hw + rise * rise) / (2.0 * hw).max(1e-4);
                let (side, tt) = if t <= 0.5 { (-1.0, t * 2.0) } else { (1.0, (1.0 - t) * 2.0) };
                let c = Vec2::new(side * (hw - r), 0.0);
                let a1 = rise.atan2(r - hw);
                let a = a1 * tt;
                Vec2::new(c.x + side * r * a.cos(), r * a.sin())
            } else {
                let a = PI * (1.0 - t);
                Vec2::new(a.cos() * hw, a.sin() * rise)
            }
        })
        .collect()
}

impl Kit {
    /// Gabled roof over a `2hx` x `2hy` footprint (ridge along local X) with
    /// its eaves at z = 0. `wall` is the Material ID of the gable ends.
    pub fn gable(&mut self, hx: f32, hy: f32, rise: f32, overhang: f32, wall: u16) {
        let (ox, oy) = (hx + overhang, hy + overhang);
        let drop = rise * overhang / hy.max(1e-3);
        let p = |x: f32, y: f32, z: f32| Vec3::new(x, y, z);
        self.quad(p(-ox, -oy, -drop), p(ox, -oy, -drop), p(ox, 0.0, rise), p(-ox, 0.0, rise));
        self.quad(p(ox, oy, -drop), p(-ox, oy, -drop), p(-ox, 0.0, rise), p(ox, 0.0, rise));
        // Undersides so the eaves read from below.
        self.quad(p(-ox, 0.0, rise - 0.05), p(ox, 0.0, rise - 0.05), p(ox, -oy, -drop - 0.05), p(-ox, -oy, -drop - 0.05));
        self.quad(p(ox, 0.0, rise - 0.05), p(-ox, 0.0, rise - 0.05), p(-ox, oy, -drop - 0.05), p(ox, oy, -drop - 0.05));
        let roof = self.mat;
        self.mat = wall;
        self.poly(&[p(-hx, hy, 0.0), p(-hx, -hy, 0.0), p(-hx, 0.0, rise)]);
        self.poly(&[p(hx, -hy, 0.0), p(hx, hy, 0.0), p(hx, 0.0, rise)]);
        self.mat = roof;
    }

    /// Hipped roof (pyramid when hx == hy) with eaves at z = 0.
    pub fn hipped(&mut self, hx: f32, hy: f32, rise: f32, overhang: f32) {
        let (ox, oy) = (hx + overhang, hy + overhang);
        let r = (hx - hy).max(0.0);
        let c = [Vec3::new(-ox, -oy, 0.0), Vec3::new(ox, -oy, 0.0), Vec3::new(ox, oy, 0.0), Vec3::new(-ox, oy, 0.0)];
        let (a, b) = (Vec3::new(-r, 0.0, rise), Vec3::new(r, 0.0, rise));
        if r > 1e-4 {
            self.quad(c[0], c[1], b, a);
            self.quad(c[2], c[3], a, b);
        } else {
            self.poly(&[c[0], c[1], b]);
            self.poly(&[c[2], c[3], b]);
        }
        self.poly(&[c[1], c[2], b]);
        self.poly(&[c[3], c[0], a]);
        self.poly(&[c[3], c[2], c[1], c[0]]);
    }

    /// Wall face of thickness `t` (front at y = -t/2) above an arch of
    /// `span` x `rise` (springing at z = 0) up to `top`, plus the arch
    /// soffit. Built as one column of quads per arch segment.
    pub fn arch_wall(&mut self, span: f32, rise: f32, top: f32, t: f32, pointed: bool) {
        let curve = arch_curve(span, rise, pointed, 12);
        let (f, b) = (-t * 0.5, t * 0.5);
        for w in curve.windows(2) {
            let (p, q) = (w[0], w[1]);
            let top_z = top.max(p.y.max(q.y) + 0.01);
            self.quad(Vec3::new(p.x, f, p.y), Vec3::new(q.x, f, q.y), Vec3::new(q.x, f, top_z), Vec3::new(p.x, f, top_z));
            self.quad(Vec3::new(q.x, b, q.y), Vec3::new(p.x, b, p.y), Vec3::new(p.x, b, top_z), Vec3::new(q.x, b, top_z));
            // Soffit (underside of the arch).
            self.quad(Vec3::new(q.x, f, q.y), Vec3::new(p.x, f, p.y), Vec3::new(p.x, b, p.y), Vec3::new(q.x, b, q.y));
        }
    }

    /// Flat arched panel (window glass, door leaf) in the local XZ plane,
    /// facing -Y: a rectangle of `w` x `h` topped by a round / pointed arch.
    pub fn arched_panel(&mut self, w: f32, h: f32, pointed: bool) {
        let rise = if pointed { w * 0.75 } else { w * 0.5 };
        let mut pts = vec![Vec3::new(-w * 0.5, 0.0, 0.0), Vec3::new(w * 0.5, 0.0, 0.0)];
        let curve = arch_curve(w, rise, pointed, 8);
        pts.extend(curve.iter().rev().map(|p| Vec3::new(p.x, 0.0, h - rise + p.y)));
        pts.dedup_by(|a, b| a.distance(*b) < 1e-5);
        self.poly(&pts);
    }
}
