//! Standalone Buildings & Props (Create -> Geometry -> Buildings & Props):
//! 13 residential, 17 commercial & office, 14 industrial & utility, 17 services & transport, 10 parks & leisure, 16 garden & lot, 14 modern and 12 medieval buildings, 5 bridges / infrastructure pieces
//! and 13 street doodads (including the PR 7 Rubble Pile), each its own parametric object with Width,
//! Length, Floors, Seed, part toggles, `[✓] Emissive Lights` and a
//! Procedural | Photo material style. Ground at z = 0, front facing -Y,
//! Settlement Material IDs (`kit::ids`).

pub mod arch;
mod appliances;
mod appliances_b;
mod commercial_a;
mod commercial_b;
mod commercial_c;
mod decor;
mod doodads;
mod furniture;
mod furniture_b;
mod furniture_c;
mod garden;
mod garden_b;
mod houses;
mod industrial;
mod industry;
mod kitchen_bath;
pub mod infra;
mod kinds;
mod kinds_defaults;
mod kinds_info;
mod medieval_a;
mod medieval_b;
mod parks;
mod parks_b;
mod modern_a;
mod modern_b;
pub mod parts;
mod residential_a;
mod residential_b;
mod rooms;
mod rural;
mod services;
mod services_b;
mod shops;
mod transport;
mod utilities;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_appliances;
#[cfg(test)]
mod tests_commercial;
#[cfg(test)]
mod tests_furniture;
#[cfg(test)]
mod tests_garden;
#[cfg(test)]
mod tests_rural;
#[cfg(test)]
mod tests_industrial;
#[cfg(test)]
mod tests_parks;
#[cfg(test)]
mod tests_residential;
#[cfg(test)]
mod tests_services;
#[cfg(test)]
mod tests_shots;

pub use arch::StairKind;
pub use houses::Lot;
pub use kinds::{BuildingKind, Category};
pub use parts::Parts;

use super::kit::{Kit, ids, rng};
use super::ruin::{self, RuinParams};
use crate::mesh::EditablePoly;
use rand::Rng;
use rand_chacha::ChaCha8Rng;

/// Prefab detail level: Standard is the light settlement look; Mid adds
/// window frames and sills, roof trims and other small parts.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Detail {
    #[default]
    Standard,
    Mid,
}

impl Detail {
    pub const ALL: [Detail; 2] = [Self::Standard, Self::Mid];
    pub fn label(self) -> &'static str {
        match self {
            Self::Standard => "Standard",
            Self::Mid => "Mid",
        }
    }
}

/// Material style of the assigned Settlement pack.
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
            Self::Procedural => crate::materials::MultiPreset::Settlement,
            Self::Photo => crate::materials::MultiPreset::SettlementPhoto,
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
    /// Standard or Mid detail (frames, sills, trims).
    pub detail: Detail,
    /// Garden, driveway, garage and fence (Residential prefabs).
    pub lot: Lot,
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
            detail: Detail::Standard,
            lot: Lot::default(),
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
        b.kit.mid = self.detail == Detail::Mid;
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
            Category::Residential => {
                if matches!(self.kind, K::Cottage | K::Bungalow | K::Farmhouse | K::LogCabin | K::AFrame | K::BeachHut) {
                    residential_a::build(&mut b, hx, hy, floors)
                } else {
                    residential_b::build(&mut b, hx, hy, floors)
                }
            }
            Category::Commercial => match self.kind {
                K::CornerShop | K::Cafe | K::Diner | K::Restaurant | K::Bakery | K::Pharmacy => commercial_a::build(&mut b, hx, hy, floors),
                K::Supermarket | K::Mall | K::Hotel | K::Motel | K::Bank | K::Cinema => commercial_b::build(&mut b, hx, hy, floors),
                _ => commercial_c::build(&mut b, hx, hy, floors),
            },
            Category::Industrial => match self.kind {
                K::Factory | K::TruckDepot | K::SiloCluster | K::GrainElevator | K::Refinery | K::TankFarm | K::RecyclingYard => industrial::build(&mut b, hx, hy),
                _ => utilities::build(&mut b, hx, hy),
            },
            Category::Services => match self.kind {
                K::Temple | K::Cemetery => services_b::build(&mut b, hx, hy),
                K::FireStation | K::PoliceStation | K::Clinic | K::University | K::Library | K::TownHall | K::PostOffice => services::build(&mut b, hx, hy, floors),
                _ => transport::build(&mut b, hx, hy, floors),
            },
            Category::Parks => match self.kind {
                K::Playground | K::SportsCourt | K::SportsField | K::Stadium | K::SwimmingPool => parks::build(&mut b, hx, hy),
                _ => parks_b::build(&mut b, hx, hy),
            },
            Category::Garden => match self.kind {
                K::Mailbox | K::GardenBeds | K::Hedge | K::PicketFence | K::Patio | K::Barbecue | K::SwingSet | K::Trampoline => garden::build(&mut b, hx, hy),
                _ => garden_b::build(&mut b, hx, hy),
            },
            Category::Rural => rural::build(&mut b, hx, hy),
            Category::Furniture => {
                // Small parts: finer texture repeats so wood grain reads.
                b.kit.uv_scale = 2.0;
                match self.kind {
                    K::DiningTable | K::CoffeeTable | K::SideTable | K::Desk => furniture_b::build(&mut b, hx, hy),
                    K::DiningChair | K::OfficeChair | K::Armchair | K::BarStool | K::Sofa | K::SectionalSofa => furniture::build(&mut b, hx, hy),
                    _ => furniture_c::build(&mut b, hx, hy),
                }
            }
            Category::KitchenBath => {
                b.kit.uv_scale = 2.0;
                kitchen_bath::build(&mut b, hx, hy)
            }
            Category::Decor => {
                b.kit.uv_scale = 2.0;
                decor::build(&mut b, hx, hy)
            }
            Category::Appliances => {
                b.kit.uv_scale = 2.0;
                match self.kind {
                    K::Microwave | K::KettleToaster | K::CoffeeMachine | K::Television | K::PcSetup | K::Laptop | K::Speakers => appliances_b::build(&mut b, hx, hy),
                    _ => appliances::build(&mut b, hx, hy),
                }
            }
            Category::Rooms => rooms::build(&mut b, hx, hy),
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
