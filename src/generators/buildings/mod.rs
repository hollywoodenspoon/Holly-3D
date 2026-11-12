//! Standalone Buildings & Props (Create -> Geometry -> Buildings & Props):
//! 14 modern and 12 medieval buildings, 5 bridges / infrastructure pieces
//! and 13 street doodads (including the PR 7 Rubble Pile), each its own parametric object with Width,
//! Length, Floors, Seed, part toggles, `[✓] Emissive Lights` and a
//! Procedural | Photo material style. Ground at z = 0, front facing -Y,
//! GhostTown Material IDs (`kit::ids`).

pub mod arch;
mod doodads;
pub mod infra;
mod kinds;
mod medieval_a;
mod medieval_b;
mod modern_a;
mod modern_b;
pub mod parts;
#[cfg(test)]
mod tests;

pub use arch::StairKind;
pub use kinds::{BuildingKind, Category};
pub use parts::Parts;

use super::kit::{Kit, ids, rng};
use super::ruin::{self, RuinParams};
use crate::mesh::EditablePoly;
use rand::Rng;
use rand_chacha::ChaCha8Rng;

/// Material style of the assigned GhostTown pack.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MatStyle {
    #[default]
    Procedural,
    Photo,
}

impl MatStyle {
    pub const ALL: [MatStyle; 2] = [Self::Procedural, Self::Photo];
    pub fn label(self) -> &'static str {
        match self {
            Self::Procedural => "Procedural",
            Self::Photo => "Photo",
        }
    }
    pub fn pack(self) -> crate::materials::MultiPreset {
        match self {
            Self::Procedural => crate::materials::MultiPreset::GhostTown,
            Self::Photo => crate::materials::MultiPreset::GhostTownPhoto,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BuildingParams {
    pub kind: BuildingKind,
    pub width: f32,
    pub length: f32,
    pub floors: u32,
    pub seed: u32,
    pub parts: Parts,
    /// `[✓] Emissive Lights`: lit windows, lamps, lanterns, neon, fires.
    pub emissive: bool,
    pub style: MatStyle,
    /// Staircase form (porches, stair turrets, fire escapes, quays).
    pub stairs: StairKind,
    /// PR 7 Destruction: level, era, rubble, embers and gore (also the
    /// Rubble Pile's density, height and era).
    pub ruin: RuinParams,
}

impl Default for BuildingParams {
    fn default() -> Self {
        Self::of(BuildingKind::OfficeBlock)
    }
}

impl BuildingParams {
    /// The type's default size and floor count.
    pub fn of(kind: BuildingKind) -> Self {
        let (width, length, floors) = kind.defaults();
        Self {
            kind,
            width,
            length,
            floors,
            seed: 1,
            parts: Parts::default(),
            emissive: true,
            style: MatStyle::Procedural,
            stairs: StairKind::Straight,
            ruin: RuinParams::default(),
        }
    }

    /// Builds the object centred on the origin, standing on z = 0.
    pub fn build(&self) -> EditablePoly {
        if self.kind == BuildingKind::RubblePile {
            let half = bevy::math::Vec2::new(self.width, self.length) * 0.5;
            return ruin::rubble_pile(half, &self.ruin, false);
        }
        let mut m = self.build_intact();
        ruin::apply(&mut m, &self.ruin, self.kind.is_medieval());
        m
    }

    /// The undamaged object.
    fn build_intact(&self) -> EditablePoly {
        let mut b = Ctx { kit: Kit::new(), p: self.clone(), r: rng(self.seed, 0xB01D + self.kind as u64) };
        b.kit.sg = 0;
        let hx = (self.width.max(0.2)) * 0.5;
        let hy = (self.length.max(0.2)) * 0.5;
        let floors = self.floors.clamp(1, 200);
        use BuildingKind as K;
        match self.kind.category() {
            Category::Modern => {
                if matches!(
                    self.kind,
                    K::GlassSkyscraper | K::OfficeBlock | K::BrickTenement | K::ApartmentBlock | K::ParkingGarage | K::School | K::Hospital
                ) {
                    modern_a::build(&mut b, hx, hy, floors)
                } else {
                    modern_b::build(&mut b, hx, hy, floors)
                }
            }
            Category::Medieval => {
                if matches!(self.kind, K::CastleKeep | K::GuildHall | K::MarketHall | K::Tavern | K::StoneChapel | K::Gatehouse) {
                    medieval_a::build(&mut b, hx, hy, floors)
                } else {
                    medieval_b::build(&mut b, hx, hy, floors)
                }
            }
            Category::Infrastructure => infra::build(&mut b, hx, hy, floors),
            Category::Props => doodads::build(&mut b, hx, hy),
        }
        b.kit.finish()
    }
}

/// Builder context: the kit, the parameters and the seeded RNG.
pub(crate) struct Ctx {
    pub kit: Kit,
    pub p: BuildingParams,
    pub r: ChaCha8Rng,
}

impl Ctx {
    /// A window pane ID: some lit at night when Emissive Lights is on.
    pub fn pane(&mut self, lit_share: f64) -> u16 {
        pane_id(&mut self.r, self.p.emissive, lit_share)
    }
    /// A lamp / lantern / sign glow ID (`neon` for coloured signs).
    pub fn glow(&self, neon: bool) -> u16 {
        match (self.p.emissive, neon) {
            (true, true) => ids::NEON,
            (true, false) => ids::LIT,
            (false, _) => ids::GLASS,
        }
    }
}

/// A window pane ID without borrowing a whole `Ctx`.
pub(crate) fn pane_id(r: &mut ChaCha8Rng, emissive: bool, lit_share: f64) -> u16 {
    if emissive && r.gen_bool(lit_share.clamp(0.0, 1.0)) { ids::LIT } else { ids::GLASS }
}

/// A massed block `2hx` x `2hy` from `z0` with `floors` storeys of `fh`,
/// wall Material `wall`, and a row of window panes (`ww` x `wh`, one per
/// `bay` metres) on every side and floor.
#[allow(clippy::too_many_arguments)]
pub(crate) fn massing(b: &mut Ctx, hx: f32, hy: f32, z0: f32, floors: u32, fh: f32, wall: u16, bay: f32, ww: f32, wh: f32, lit: f64) {
    use bevy::math::Vec3;
    b.kit.mat(wall).block(Vec3::new(-hx, -hy, z0), Vec3::new(hx, hy, z0 + fh * floors as f32));
    let (r, em) = (&mut b.r, b.p.emissive);
    super::kit::sides(&mut b.kit, hx, hy, |k, _, w| {
        let n = ((w / bay).floor() as usize).max(1);
        for f in 0..floors {
            for i in 0..n {
                let x = -w * 0.5 + w * (i as f32 + 0.5) / n as f32;
                let id = pane_id(r, em, lit);
                super::settlement::window(k, x, 0.0, z0 + f as f32 * fh + (fh - wh) * 0.5, ww, wh, id);
            }
        }
    });
}
