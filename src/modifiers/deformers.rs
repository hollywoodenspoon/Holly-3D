//! Classic 3ds Max deformers: Bend, Twist, Taper and Push. Bend, Twist and
//! Taper work in the object's local space around the gizmo `center`
//! (the pivot by default), measured along the chosen axis over the extent
//! of the input mesh, like 3ds Max's gizmo fitted to the bounding box.

use crate::mesh::EditablePoly;
use bevy::math::Vec3;

/// `[ ] Limit Effect` with Upper / Lower limits along the deform axis.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Limits {
    pub on: bool,
    pub upper: f32,
    pub lower: f32,
}

impl Limits {
    fn clamp(&self, w: f32) -> f32 {
        if self.on { w.clamp(self.lower.min(self.upper), self.upper.max(self.lower)) } else { w }
    }
}

/// Local frame (u, v, w) with w along `axis` (0 = X, 1 = Y, 2 = Z).
fn frame(axis: usize) -> [usize; 3] {
    let w = axis.min(2);
    [(w + 1) % 3, (w + 2) % 3, w]
}

/// Applies `f(u, v, w) -> (u, v, w)` to every vertex in the axis frame
/// around `center`; `len` is the mesh extent along the axis.
fn deform(m: &mut EditablePoly, axis: usize, center: Vec3, f: impl Fn(f32, f32, f32, f32) -> (f32, f32, f32)) {
    let [iu, iv, iw] = frame(axis);
    let Some(b) = m.bounds() else { return };
    let len = b.size()[iw].max(1e-4);
    for v in m.vertices.values_mut() {
        let p = v.position - center;
        let (u, vv, w) = f(p[iu], p[iv], p[iw], len);
        let mut q = Vec3::ZERO;
        q[iu] = u;
        q[iv] = vv;
        q[iw] = w;
        v.position = q + center;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BendMod {
    /// Degrees of bend over the whole extent.
    pub angle: f32,
    /// Degrees: rotates the bend plane around the bend axis.
    pub direction: f32,
    pub axis: usize,
    pub limits: Limits,
    pub center: Vec3,
}

impl Default for BendMod {
    fn default() -> Self {
        Self { angle: 0.0, direction: 0.0, axis: 2, limits: Limits { on: false, upper: 1.0, lower: 0.0 }, center: Vec3::ZERO }
    }
}

impl BendMod {
    pub fn apply(&self, m: &mut EditablePoly) {
        let theta = self.angle.to_radians();
        if theta.abs() < 1e-6 {
            return;
        }
        let (s, c) = self.direction.to_radians().sin_cos();
        let lim = self.limits;
        deform(m, self.axis, self.center, |u, v, w, len| {
            // rotate so the bend happens in the (x, w) plane
            let (x, y) = (u * c + v * s, -u * s + v * c);
            let r = len / theta;
            let wc = lim.clamp(w);
            let a = theta * wc / len;
            let (sa, ca) = a.sin_cos();
            // past a limit the mesh continues straight along the tangent
            let extra = w - wc;
            let nx = r - (r - x) * ca + extra * sa;
            let nw = (r - x) * sa + extra * ca;
            (nx * c - y * s, nx * s + y * c, nw)
        });
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TwistMod {
    pub angle: f32,
    /// -100..100: pushes the twist towards the ends (+) or the center (-).
    pub bias: f32,
    pub axis: usize,
    pub limits: Limits,
    pub center: Vec3,
}

impl Default for TwistMod {
    fn default() -> Self {
        Self { angle: 0.0, bias: 0.0, axis: 2, limits: Limits { on: false, upper: 1.0, lower: 0.0 }, center: Vec3::ZERO }
    }
}

impl TwistMod {
    pub fn apply(&self, m: &mut EditablePoly) {
        let theta = self.angle.to_radians();
        if theta.abs() < 1e-6 {
            return;
        }
        let b = (self.bias / 100.0).clamp(-0.99, 0.99);
        let exp = if b >= 0.0 { 1.0 / (1.0 + 3.0 * b) } else { 1.0 - 3.0 * b };
        let lim = self.limits;
        deform(m, self.axis, self.center, |u, v, w, len| {
            let t = lim.clamp(w) / len;
            let a = theta * t.signum() * t.abs().powf(exp);
            let (s, c) = a.sin_cos();
            (u * c - v * s, u * s + v * c, w)
        });
    }
}

/// Which local axes Taper scales.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TaperEffect {
    #[default]
    Both,
    First,
    Second,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TaperMod {
    /// -10..10: scale change over the extent.
    pub amount: f32,
    /// Bulges (+) or pinches (-) the middle.
    pub curve: f32,
    pub axis: usize,
    pub effect: TaperEffect,
    /// Taper both directions away from the center.
    pub symmetry: bool,
    pub limits: Limits,
    pub center: Vec3,
}

impl Default for TaperMod {
    fn default() -> Self {
        Self {
            amount: 0.0,
            curve: 0.0,
            axis: 2,
            effect: TaperEffect::Both,
            symmetry: false,
            limits: Limits { on: false, upper: 1.0, lower: 0.0 },
            center: Vec3::ZERO,
        }
    }
}

impl TaperMod {
    pub fn apply(&self, m: &mut EditablePoly) {
        if self.amount == 0.0 && self.curve == 0.0 {
            return;
        }
        let (amt, curve, effect, sym, lim) = (self.amount.clamp(-10.0, 10.0), self.curve, self.effect, self.symmetry, self.limits);
        deform(m, self.axis, self.center, |u, v, w, len| {
            let mut t = lim.clamp(w) / len;
            if sym {
                t = t.abs();
            }
            let k = (1.0 + amt * t + curve * 4.0 * t * (1.0 - t)).max(0.0);
            let (ku, kv) = match effect {
                TaperEffect::Both => (k, k),
                TaperEffect::First => (k, 1.0),
                TaperEffect::Second => (1.0, k),
            };
            (u * ku, v * kv, w)
        });
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct PushMod {
    /// Offset along the averaged vertex normal.
    pub value: f32,
}

impl PushMod {
    pub fn apply(&self, m: &mut EditablePoly) {
        if self.value == 0.0 {
            return;
        }
        let normals = m.vertex_normals();
        for (id, v) in m.vertices.iter_mut() {
            if let Some(n) = normals.get(&id) {
                v.position += *n * self.value;
            }
        }
    }
}
