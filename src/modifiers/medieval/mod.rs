//! "Medieval Architect": a scale-aware modifier that turns a simple base
//! (Cylinder, Box, Editable Spline or Editable Poly) into a detailed
//! medieval structure. Brick courses, crenellations, floors, windows,
//! buttresses and arches are recounted from the input's size, so resizing
//! the base adds or removes detail instead of stretching it.

mod bridge;
mod cathedral;
mod house;
pub mod parts;
mod tower;
mod wall;

#[cfg(test)]
mod tests;

use crate::generators::kit::Kit;
use crate::mesh::EditablePoly;
use crate::modifiers::battlefield::{BattleParams, BattlefieldMod, DestructionMode};
use bevy::math::{Mat4, Vec2, Vec3};
use parts::Masonry;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ArchPreset {
    #[default]
    Watchtower,
    CurtainWall,
    TimberHouse,
    Bridge,
    Cathedral,
}

impl ArchPreset {
    pub const ALL: [ArchPreset; 5] = [Self::Watchtower, Self::CurtainWall, Self::TimberHouse, Self::Bridge, Self::Cathedral];
    pub fn label(self) -> &'static str {
        match self {
            Self::Watchtower => "Watchtower / Castle Keep",
            Self::CurtainWall => "Castle Curtain Wall & Gatehouse",
            Self::TimberHouse => "Half-Timbered House / Tavern",
            Self::Bridge => "Arched Stone Bridge / Aqueduct",
            Self::Cathedral => "Gothic Cathedral Vault & Colonnade",
        }
    }
}

/// Plan shape of the base: Auto reads the input (no vertices near the
/// bounding-box corners means a cylinder).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum PlanShape {
    #[default]
    Auto,
    Round,
    Square,
}

/// The base the structure is built on (local space of the object).
#[derive(Clone, Debug, PartialEq)]
pub struct Footprint {
    pub min: Vec3,
    pub max: Vec3,
    pub round: bool,
    /// Spline path (curtain walls follow it).
    pub path: Option<(Vec<Vec2>, bool)>,
}

impl Footprint {
    pub fn size(&self) -> Vec3 {
        (self.max - self.min).max(Vec3::splat(0.1))
    }
    pub fn of_mesh(m: &EditablePoly, shape: PlanShape) -> Option<Self> {
        let b = m.bounds()?;
        let (c, h) = ((b.min + b.max) * 0.5, ((b.max - b.min) * 0.5).max(Vec3::splat(1e-4)));
        let cornered = m.vertices.values().any(|v| {
            let q = (v.position - c) / h;
            q.x.abs() > 0.92 && q.y.abs() > 0.92
        });
        let round = match shape {
            PlanShape::Auto => !cornered,
            PlanShape::Round => true,
            PlanShape::Square => false,
        };
        Some(Self { min: b.min, max: b.max, round, path: None })
    }
}

/// How many of each detail the current size receives (for the UI read-out
/// and the scale-awareness tests).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ArchLayout {
    pub floors: u32,
    pub courses: u32,
    pub bricks_per_course: u32,
    pub merlons: u32,
    pub windows_per_floor: u32,
    pub buttresses: u32,
    pub arches: u32,
    /// Brick size actually used (grows only when the brick budget is hit).
    pub brick: Vec2,
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ArchitectMod {
    pub preset: ArchPreset,
    pub shape: PlanShape,
    pub bricks: bool,
    pub crenellations: bool,
    pub machicolations: bool,
    pub windows: bool,
    pub door: bool,
    pub buttresses: bool,
    pub roof: bool,
    /// Master Brick / Block Scale (1 = 0.8 x 0.4 m blocks).
    pub brick_scale: f32,
    pub floor_height: f32,
    pub wall_thickness: f32,
    /// Merlon width : crenel (gap) width.
    pub merlon_ratio: f32,
    pub window_spacing: f32,
    /// Wall height used on spline paths (shapes have no height).
    pub spline_height: f32,
    pub seed: u32,
    pub damaged: bool,
    pub damage: BattleParams,
}

impl Default for ArchitectMod {
    fn default() -> Self {
        Self {
            preset: ArchPreset::Watchtower,
            shape: PlanShape::Auto,
            bricks: true,
            crenellations: true,
            machicolations: true,
            windows: true,
            door: true,
            buttresses: true,
            roof: true,
            brick_scale: 1.0,
            floor_height: 3.5,
            wall_thickness: 2.0,
            merlon_ratio: 1.5,
            window_spacing: 3.0,
            spline_height: 9.0,
            seed: 1,
            damaged: false,
            damage: BattleParams::of(DestructionMode::MedievalSiege),
        }
    }
}

/// Above this many bricks the block size grows (keeps meshes interactive).
pub const BRICK_BUDGET: f32 = 14_000.0;

impl ArchitectMod {
    pub fn masonry(&self, fp: &Footprint) -> Masonry {
        let base = Vec2::new(0.8, 0.4) * self.brick_scale.clamp(0.1, 20.0);
        let s = fp.size();
        let area = match self.preset {
            ArchPreset::Watchtower if fp.round => std::f32::consts::TAU * s.x.min(s.y) * 0.5 * s.z,
            ArchPreset::Bridge => s.x.max(s.y) * s.z * 2.0,
            ArchPreset::CurtainWall if fp.path.is_some() => path_len(fp) * self.spline_height * 2.0,
            _ => 2.0 * (s.x + s.y) * s.z,
        };
        let n = area / (base.x * base.y);
        let grow = if self.bricks && n > BRICK_BUDGET { (n / BRICK_BUDGET).sqrt() } else { 1.0 };
        Masonry {
            bricks: self.bricks,
            brick: base * grow,
            crenels: self.crenellations,
            merlon: (0.9 * self.brick_scale.clamp(0.5, 3.0)).max(0.5),
            ratio: self.merlon_ratio.clamp(0.25, 6.0),
            ..Masonry::default()
        }
    }

    pub fn layout(&self, fp: &Footprint) -> ArchLayout {
        let m = self.masonry(fp);
        let s = fp.size();
        let (long, short) = (s.x.max(s.y), s.x.min(s.y));
        let ring = if fp.round { std::f32::consts::PI * short } else { 2.0 * (s.x + s.y) };
        let face = match self.preset {
            ArchPreset::Watchtower if fp.round => ring,
            ArchPreset::CurtainWall if fp.path.is_some() => path_len(fp),
            ArchPreset::Watchtower => s.x,
            _ => long,
        };
        let height = if fp.path.is_some() { self.spline_height } else { s.z };
        let spacing = self.window_spacing.max(0.5);
        ArchLayout {
            floors: (height / self.floor_height.max(1.0)).floor().max(1.0) as u32,
            courses: parts::course_count(height, m.brick.y) as u32,
            bricks_per_course: parts::bricks_per_course(face, m.brick.x) as u32,
            merlons: parts::merlon_count(if self.preset == ArchPreset::Watchtower { ring } else { face }, m.merlon, m.ratio) as u32,
            windows_per_floor: ((face / spacing).floor() as u32).max(1),
            buttresses: ((face / (spacing * 2.5)).floor() as u32).max(1),
            arches: bridge::span_count(long, height) as u32,
            brick: m.brick,
        }
    }

    /// Builds the structure for a footprint (object space).
    pub fn build(&self, fp: &Footprint) -> EditablePoly {
        let mut kit = Kit::new();
        let lay = self.layout(fp);
        let m = self.masonry(fp);
        let c = ((fp.min + fp.max) * 0.5).truncate().extend(fp.min.z);
        if fp.path.is_some() && self.preset == ArchPreset::CurtainWall {
            kit.xf = Mat4::from_translation(Vec3::Z * fp.min.z);
            wall::path(&mut kit, self, fp, &m);
        } else {
            kit.xf = Mat4::from_translation(c);
            let s = fp.size();
            match self.preset {
                ArchPreset::Watchtower => tower::build(&mut kit, self, s, fp.round, &lay, &m),
                ArchPreset::CurtainWall => wall::build(&mut kit, self, s, &lay, &m),
                ArchPreset::TimberHouse => house::build(&mut kit, self, s, &lay),
                ArchPreset::Bridge => bridge::build(&mut kit, self, s, &lay, &m),
                ArchPreset::Cathedral => cathedral::build(&mut kit, self, s, &lay, &m),
            }
        }
        let mut out = kit.finish();
        if self.damaged {
            BattlefieldMod { params: BattleParams { seed: self.damage.seed ^ self.seed, ..self.damage.clone() }, zones: vec![] }.apply(&mut out);
        }
        out
    }

    pub fn apply(&self, mesh: &mut EditablePoly) {
        if let Some(fp) = Footprint::of_mesh(mesh, self.shape) {
            *mesh = self.build(&fp);
        }
    }

    /// Spline input: curtain walls follow the path; other presets use its bounds.
    pub fn build_from_shape(&self, shape: &crate::splines::Shape) -> EditablePoly {
        let Some(pl) = shape.polylines().into_iter().find(|p| p.pts.len() >= 2) else { return EditablePoly::new() };
        let Some(b) = crate::mesh::Aabb::from_points(pl.pts.iter().copied()) else { return EditablePoly::new() };
        let path = Some((pl.pts.iter().map(|p| p.truncate()).collect(), pl.closed));
        let h = self.spline_height.max(1.0);
        let fp = Footprint { min: b.min, max: Vec3::new(b.max.x, b.max.y, b.min.z + h), round: false, path };
        self.build(&fp)
    }
}

fn path_len(fp: &Footprint) -> f32 {
    let Some((pts, closed)) = &fp.path else { return 0.0 };
    let open: f32 = pts.windows(2).map(|w| w[0].distance(w[1])).sum();
    open + if *closed && pts.len() > 2 { pts[0].distance(pts[pts.len() - 1]) } else { 0.0 }
}
