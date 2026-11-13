//! The standalone building / bridge / doodad types and their defaults.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum BuildingKind {
    // Modern (14)
    GlassSkyscraper,
    #[default]
    OfficeBlock,
    BrickTenement,
    ApartmentBlock,
    Shop,
    GasStation,
    Warehouse,
    SawtoothFactory,
    ParkingGarage,
    School,
    Hospital,
    ModernChurch,
    WaterTower,
    Kiosk,
    // Medieval (12)
    CastleKeep,
    GuildHall,
    MarketHall,
    Tavern,
    StoneChapel,
    Windmill,
    Watermill,
    Forge,
    Stable,
    Granary,
    Gatehouse,
    WellHouse,
    // Bridges & infrastructure (5)
    ArchBridge,
    TrussBridge,
    Drawbridge,
    Quay,
    Crane,
    // Doodads & street props (13)
    Streetlamp,
    LanternPost,
    Car,
    Cart,
    MarketStall,
    BarrelCrates,
    Dumpster,
    SandbagWall,
    TankTrap,
    BarbedWire,
    Bench,
    Fountain,
    RubblePile,
    // Residential (13), appended so saved indices and seeds stay put
    Cottage,
    Bungalow,
    Farmhouse,
    LogCabin,
    AFrame,
    Villa,
    Victorian,
    ModernBox,
    RowHouses,
    Duplex,
    BeachHut,
    WalkUp,
    TowerBlock,
    // Commercial & office (17), appended after Residential
    CornerShop, Cafe, Diner, Restaurant, Bakery, Pharmacy,
    Supermarket, Mall, Hotel, Motel, Bank, Cinema,
    CarDealer, DriveThru, LowRiseOffice, MidRiseOffice, OfficeCampus,
    // Industrial & utilities (14)
    Factory, TruckDepot, SiloCluster, GrainElevator, Refinery, TankFarm, RecyclingYard,
    PowerPlant, WindTurbine, SolarFarm, PumpStation, Substation, PowerPylon, CellTower,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Modern,
    Medieval,
    Infrastructure,
    Props,
    Residential,
    Commercial,
    Industrial,
}

impl Category {
    pub const ALL: [Category; 7] = [Self::Residential, Self::Commercial, Self::Industrial, Self::Modern, Self::Medieval, Self::Infrastructure, Self::Props];
    pub fn label(self) -> &'static str {
        match self {
            Self::Modern => "Modern Buildings",
            Self::Medieval => "Medieval Buildings",
            Self::Infrastructure => "Bridges & Infrastructure",
            Self::Props => "Doodads & Street Props",
            Self::Residential => "Residential",
            Self::Commercial => "Commercial & Office",
            Self::Industrial => "Industrial & Utilities",
        }
    }
    pub fn kinds(self) -> impl Iterator<Item = BuildingKind> {
        BuildingKind::ALL.into_iter().filter(move |k| k.category() == self)
    }
}

impl BuildingKind {
    pub const ALL: [BuildingKind; 88] = {
        use BuildingKind::*;
        [
            GlassSkyscraper,
            OfficeBlock,
            BrickTenement,
            ApartmentBlock,
            Shop,
            GasStation,
            Warehouse,
            SawtoothFactory,
            ParkingGarage,
            School,
            Hospital,
            ModernChurch,
            WaterTower,
            Kiosk,
            CastleKeep,
            GuildHall,
            MarketHall,
            Tavern,
            StoneChapel,
            Windmill,
            Watermill,
            Forge,
            Stable,
            Granary,
            Gatehouse,
            WellHouse,
            ArchBridge,
            TrussBridge,
            Drawbridge,
            Quay,
            Crane,
            Streetlamp,
            LanternPost,
            Car,
            Cart,
            MarketStall,
            BarrelCrates,
            Dumpster,
            SandbagWall,
            TankTrap,
            BarbedWire,
            Bench,
            Fountain,
            RubblePile,
            Cottage, Bungalow, Farmhouse, LogCabin, AFrame, Villa, Victorian,
            ModernBox, RowHouses, Duplex, BeachHut, WalkUp, TowerBlock,
            CornerShop, Cafe, Diner, Restaurant, Bakery, Pharmacy,
            Supermarket, Mall, Hotel, Motel, Bank, Cinema,
            CarDealer, DriveThru, LowRiseOffice, MidRiseOffice, OfficeCampus,
            Factory, TruckDepot, SiloCluster, GrainElevator, Refinery, TankFarm, RecyclingYard,
            PowerPlant, WindTurbine, SolarFarm, PumpStation, Substation, PowerPylon, CellTower,
        ]
    };

    pub fn category(self) -> Category {
        let i = self as usize;
        match i {
            0..=13 => Category::Modern,
            14..=25 => Category::Medieval,
            26..=30 => Category::Infrastructure,
            31..=43 => Category::Props,
            44..=56 => Category::Residential,
            57..=73 => Category::Commercial,
            _ => Category::Industrial,
        }
    }

    pub fn is_medieval(self) -> bool {
        use BuildingKind::*;
        self.category() == Category::Medieval
            || matches!(self, ArchBridge | Drawbridge | LanternPost | Cart | MarketStall | BarrelCrates | Fountain)
    }

    /// Whether Floors means anything for this type.
    pub fn has_floors(self) -> bool {
        use BuildingKind::*;
        (!matches!(self.category(), Category::Props | Category::Industrial) || matches!(self, SandbagWall)) && !matches!(self, LogCabin | AFrame | BeachHut | Cafe | Diner | Restaurant | Pharmacy | Supermarket | Bank | Cinema | CarDealer | DriveThru)
    }
}
