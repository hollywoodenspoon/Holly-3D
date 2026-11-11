//! Hole / Counterbore / Countersink cutter (Edit Poly, polygon mode): drills
//! a round hole into the centre of every selected polygon, straight in
//! along the polygon normal, with a manifold Boolean subtract.

use crate::mesh::{EditablePoly, PolyId};
use crate::modifiers::boolean::{BoolOp, compute};
use crate::splines::sweep::{flip, loft};
use bevy::math::{Quat, Vec2, Vec3};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HoleKind {
    #[default]
    Simple,
    /// A wider flat-bottomed step at the top (for a socket-head screw).
    Counterbore,
    /// A cone at the top (for a flat-head screw).
    Countersink,
}

impl HoleKind {
    pub const ALL: [HoleKind; 3] = [HoleKind::Simple, HoleKind::Counterbore, HoleKind::Countersink];
    pub fn label(self) -> &'static str {
        match self {
            HoleKind::Simple => "Simple Hole",
            HoleKind::Counterbore => "Counterbore",
            HoleKind::Countersink => "Countersink",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct HoleSpec {
    pub kind: HoleKind,
    pub diameter: f32,
    pub depth: f32,
    /// Ignore Depth and go all the way through.
    pub through: bool,
    /// Counterbore / countersink top diameter.
    pub top_diameter: f32,
    pub counterbore_depth: f32,
    /// Countersink included angle, degrees (82° imperial, 90° metric).
    pub countersink_angle: f32,
    pub sides: u32,
}

impl Default for HoleSpec {
    fn default() -> Self {
        Self {
            kind: HoleKind::Simple,
            diameter: 0.1,
            depth: 0.2,
            through: false,
            top_diameter: 0.18,
            counterbore_depth: 0.06,
            countersink_angle: 90.0,
            sides: 24,
        }
    }
}

impl HoleSpec {
    /// The hole's profile as (radius, depth below the surface) from the top
    /// down; the first point sits a little above the surface.
    pub fn profile(&self, depth: f32) -> Vec<Vec2> {
        let r = self.diameter.max(1e-4) * 0.5;
        let top = (self.top_diameter * 0.5).max(r * 1.01);
        let lift = -(r * 0.5).max(1e-3);
        let mut p = vec![Vec2::new(0.0, lift)];
        match self.kind {
            HoleKind::Simple => p.push(Vec2::new(r, lift)),
            HoleKind::Counterbore => {
                let d = self.counterbore_depth.clamp(1e-4, depth * 0.95);
                p.extend([Vec2::new(top, lift), Vec2::new(top, d), Vec2::new(r, d)]);
            }
            HoleKind::Countersink => {
                let half = (self.countersink_angle.clamp(30.0, 150.0) * 0.5).to_radians();
                let d = ((top - r) / half.tan()).min(depth * 0.95);
                // The cone continues above the surface so the rim is clean.
                p.extend([Vec2::new(top - lift * half.tan(), lift), Vec2::new(r, d)]);
            }
        }
        p.extend([Vec2::new(r, depth), Vec2::new(0.0, depth)]);
        p
    }

    /// A closed cutter solid at `at`, drilling along `-normal`.
    pub fn cutter(&self, at: Vec3, normal: Vec3, depth: f32) -> EditablePoly {
        let sides = self.sides.clamp(6, 128) as usize;
        let rot = Quat::from_rotation_arc(Vec3::Z, normal.normalize_or(Vec3::Z));
        let prof = self.profile(depth);
        let rows: Vec<Vec<Vec3>> = prof
            .iter()
            .map(|q| {
                (0..sides)
                    .map(|j| {
                        let a = j as f32 / sides as f32 * std::f32::consts::TAU;
                        at + rot * Vec3::new(a.cos() * q.x, a.sin() * q.x, -q.y)
                    })
                    .collect()
            })
            .collect();
        let (mut m, _) = loft(&rows, true, false, true);
        if m.signed_volume() < 0.0 {
            flip(&mut m);
            m.finalize();
        }
        m
    }
}

/// Drills `spec` into every polygon in `polys`. Fails (and changes nothing)
/// when the mesh or a cut is not a closed manifold.
pub fn drill(mesh: &EditablePoly, polys: &[PolyId], spec: &HoleSpec) -> Result<EditablePoly, String> {
    let through = mesh.bounds().map_or(1.0, |b| b.size().length() * 2.0 + 1.0);
    let depth = if spec.through { through } else { spec.depth.max(1e-4) };
    let cuts: Vec<(Vec3, Vec3)> = polys.iter().filter(|p| mesh.polygons.contains_key(**p)).map(|&p| (mesh.poly_centroid(p), mesh.poly_normal(p))).collect();
    if cuts.is_empty() {
        return Err("select the polygons to drill".into());
    }
    let mut out = mesh.clone();
    for (c, n) in cuts {
        out = compute(&out, &spec.cutter(c, n, depth), BoolOp::SubtractAB, true)?;
    }
    Ok(out)
}
