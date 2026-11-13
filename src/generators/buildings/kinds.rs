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
    // Services & transport (17)
    FireStation, PoliceStation, Clinic, University, Library, TownHall, PostOffice, Temple, Cemetery,
    BusStop, BusDepot, TrainStation, MetroEntrance, TramStop, AirportTerminal, ControlTower, HarbourOffice,
    // Parks & leisure (10)
    Playground, SportsCourt, SportsField, Stadium, SwimmingPool, Plaza, Monument, ParkPond, Bandstand, SkatePark,
    // Garden & lot details (16)
    Mailbox, GardenBeds, Hedge, PicketFence, Patio, Barbecue, SwingSet, Trampoline,
    GardenShed, Doghouse, WashingLine, WheelieBins, GardenGnome, HoseReel, BackyardPool, PorchSteps,
    // Rural (8)
    Barn, Greenhouse, Polytunnel, Henhouse, Haystacks, Orchard, Paddock, Beehives,
    // Furniture: seating & tables (10)
    DiningChair, OfficeChair, Armchair, BarStool, Sofa, SectionalSofa, DiningTable, CoffeeTable, SideTable, Desk,
    // Furniture: bedroom & storage (9)
    SingleBed, DoubleBed, BunkBed, Wardrobe, Dresser, Nightstand, Bookshelf, TvStand, DisplayCabinet,
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
    Services,
    Parks,
    Garden,
    Rural,
    Furniture,
}

impl Category {
    pub const ALL: [Category; 12] = [Self::Residential, Self::Commercial, Self::Industrial, Self::Services, Self::Parks, Self::Garden, Self::Rural, Self::Furniture, Self::Modern, Self::Medieval, Self::Infrastructure, Self::Props];
    pub fn label(self) -> &'static str {
        match self {
            Self::Modern => "Modern Buildings",
            Self::Medieval => "Medieval Buildings",
            Self::Infrastructure => "Bridges & Infrastructure",
            Self::Props => "Doodads & Street Props",
            Self::Residential => "Residential",
            Self::Commercial => "Commercial & Office",
            Self::Industrial => "Industrial & Utilities",
            Self::Services => "Services & Transport",
            Self::Parks => "Parks & Leisure",
            Self::Garden => "Garden & Lot",
            Self::Rural => "Rural & Farm",
            Self::Furniture => "Furniture",
        }
    }
    pub fn kinds(self) -> impl Iterator<Item = BuildingKind> {
        BuildingKind::ALL.into_iter().filter(move |k| k.category() == self)
    }
}

impl BuildingKind {
    pub const ALL: [BuildingKind; 158] = {
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
            FireStation, PoliceStation, Clinic, University, Library, TownHall, PostOffice, Temple, Cemetery,
            BusStop, BusDepot, TrainStation, MetroEntrance, TramStop, AirportTerminal, ControlTower, HarbourOffice,
            Playground, SportsCourt, SportsField, Stadium, SwimmingPool, Plaza, Monument, ParkPond, Bandstand, SkatePark,
            Mailbox, GardenBeds, Hedge, PicketFence, Patio, Barbecue, SwingSet, Trampoline,
            GardenShed, Doghouse, WashingLine, WheelieBins, GardenGnome, HoseReel, BackyardPool, PorchSteps,
            Barn, Greenhouse, Polytunnel, Henhouse, Haystacks, Orchard, Paddock, Beehives,
            DiningChair, OfficeChair, Armchair, BarStool, Sofa, SectionalSofa, DiningTable, CoffeeTable, SideTable, Desk,
            SingleBed, DoubleBed, BunkBed, Wardrobe, Dresser, Nightstand, Bookshelf, TvStand, DisplayCabinet,
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
            74..=87 => Category::Industrial,
            88..=104 => Category::Services,
            105..=114 => Category::Parks,
            115..=130 => Category::Garden,
            131..=138 => Category::Rural,
            _ => Category::Furniture,
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
        (!matches!(self.category(), Category::Props | Category::Industrial | Category::Parks | Category::Garden | Category::Rural | Category::Furniture) || matches!(self, SandbagWall)) && !matches!(self, LogCabin | AFrame | BeachHut | Cafe | Diner | Restaurant | Pharmacy | Supermarket | Bank | Cinema | CarDealer | DriveThru | Clinic | PostOffice | Temple | Cemetery | BusStop | BusDepot | MetroEntrance | TramStop | AirportTerminal)
    }
}
