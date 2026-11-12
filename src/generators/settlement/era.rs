//! Settlement presets (eras), their Buildings & Props palettes, the
//! town-feature toggles and the street-prop style.

use crate::generators::buildings::BuildingKind;
use crate::modifiers::battlefield::DestructionMode;

/// Settlement preset. New variants go at the end (the seed salt is the
/// discriminant); `ALL` lists them in menu order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum Era {
    #[default]
    Downtown,
    Suburbs,
    WalledCity,
    Village,
    RiverCity,
    HarbourTown,
    Industrial,
    OldTownSprawl,
    SquareWalled,
    RiverTown,
    CastleTown,
    StarFort,
    Crossroads,
    Monastery,
}

impl Era {
    pub const ALL: [Era; 14] = {
        use Era::*;
        [Downtown, Suburbs, RiverCity, HarbourTown, Industrial, OldTownSprawl, WalledCity, SquareWalled, RiverTown, CastleTown, StarFort, Village, Crossroads, Monastery]
    };
    pub fn label(self) -> &'static str {
        match self {
            Self::Downtown => "Modern Downtown City",
            Self::Suburbs => "Modern Suburbs / Subdivision",
            Self::WalledCity => "Medieval Walled City",
            Self::Village => "Medieval Village / Hamlet",
            Self::RiverCity => "River City (river + bridges + quays)",
            Self::HarbourTown => "Harbour Town (piers + cranes + warehouses)",
            Self::Industrial => "Industrial District (factories + rail yard + tanks)",
            Self::OldTownSprawl => "Old Town + Modern Sprawl",
            Self::SquareWalled => "Square Walled City",
            Self::RiverTown => "River Town (stone bridges + watermills + docks)",
            Self::CastleTown => "Castle & Town (hilltop castle above town)",
            Self::StarFort => "Star Fort (bastioned walls + moat)",
            Self::Crossroads => "Crossroads Hamlet",
            Self::Monastery => "Monastery",
        }
    }
    pub fn short(self) -> &'static str {
        match self {
            Self::Downtown => "Downtown",
            Self::Suburbs => "Suburbs",
            Self::WalledCity => "Walled City",
            Self::Village => "Village",
            Self::RiverCity => "River City",
            Self::HarbourTown => "Harbour",
            Self::Industrial => "Industrial",
            Self::OldTownSprawl => "Old+New Town",
            Self::SquareWalled => "Square Walls",
            Self::RiverTown => "River Town",
            Self::CastleTown => "Castle Town",
            Self::StarFort => "Star Fort",
            Self::Crossroads => "Crossroads",
            Self::Monastery => "Monastery",
        }
    }
    pub fn is_medieval(self) -> bool {
        use Era::*;
        matches!(self, WalledCity | Village | SquareWalled | RiverTown | CastleTown | StarFort | Crossroads | Monastery)
    }
    /// Storey height in metres.
    pub fn floor_height(self) -> f32 {
        match self {
            Self::Downtown | Self::RiverCity | Self::HarbourTown | Self::Industrial | Self::OldTownSprawl => 3.6,
            Self::Suburbs => 3.0,
            _ => 3.1,
        }
    }
    /// The period-accurate destruction mode for the Damaged toggle.
    pub fn destruction(self) -> DestructionMode {
        if self.is_medieval() { DestructionMode::MedievalSiege } else { DestructionMode::Modern }
    }
    /// Buildings & Props types this preset places (each gets a checkbox).
    /// The four original presets use their own bespoke houses.
    pub fn palette(self) -> &'static [BuildingKind] {
        use BuildingKind::*;
        match self {
            Self::RiverCity => &[OfficeBlock, GlassSkyscraper, BrickTenement, ApartmentBlock, Shop, Hospital, School, ModernChurch, ParkingGarage, Kiosk],
            Self::HarbourTown => &[Warehouse, Crane, BrickTenement, Shop, ApartmentBlock, WaterTower, Kiosk, GasStation],
            Self::Industrial => &[SawtoothFactory, Warehouse, WaterTower, GasStation, ParkingGarage, OfficeBlock],
            Self::OldTownSprawl => &[Tavern, GuildHall, MarketHall, StoneChapel, OfficeBlock, ApartmentBlock, Shop, GasStation, School],
            Self::SquareWalled => &[MarketHall, GuildHall, Tavern, StoneChapel, Forge, Stable, WellHouse],
            Self::RiverTown => &[Watermill, Tavern, Forge, Granary, Stable, WellHouse, ArchBridge],
            Self::CastleTown => &[CastleKeep, Tavern, Forge, Stable, Granary, StoneChapel, MarketHall],
            Self::StarFort => &[Gatehouse, GuildHall, Stable, Granary, StoneChapel, Forge, Drawbridge],
            Self::Crossroads => &[Tavern, Forge, Stable, Granary, WellHouse, Windmill],
            Self::Monastery => &[StoneChapel, GuildHall, Granary, WellHouse, Stable],
            _ => &[],
        }
    }
}

/// Street dressing added over the preset's own props.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PropStyle {
    /// Only the preset's built-in trees, lamps and wells.
    #[default]
    Generated,
    /// Plus minimal box / cone stand-ins (cheap for large scenes).
    LowPoly,
    /// Plus the detailed Buildings & Props doodads (cars, carts, benches...).
    Realistic,
}

impl PropStyle {
    pub const ALL: [PropStyle; 3] = [Self::Generated, Self::LowPoly, Self::Realistic];
    pub fn label(self) -> &'static str {
        match self {
            Self::Generated => "Generated",
            Self::LowPoly => "Low-poly pack",
            Self::Realistic => "Realistic pack",
        }
    }
}

/// Town-feature checkboxes (each preset uses the ones it has).
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Features {
    pub river: bool,
    pub bridges: bool,
    pub docks: bool,
    pub walls: bool,
    pub moat: bool,
    pub rail: bool,
    pub market: bool,
    pub trees: bool,
    pub farms: bool,
}

impl Default for Features {
    fn default() -> Self {
        Self { river: true, bridges: true, docks: true, walls: true, moat: true, rail: true, market: true, trees: true, farms: true }
    }
}

impl Features {
    pub fn flags(&mut self) -> [(&'static str, &mut bool); 9] {
        [
            ("River / Sea", &mut self.river),
            ("Bridges", &mut self.bridges),
            ("Docks / Quays", &mut self.docks),
            ("Town Walls", &mut self.walls),
            ("Moat", &mut self.moat),
            ("Rail Yard", &mut self.rail),
            ("Market Square", &mut self.market),
            ("Trees / Parks", &mut self.trees),
            ("Farm Plots", &mut self.farms),
        ]
    }
}
