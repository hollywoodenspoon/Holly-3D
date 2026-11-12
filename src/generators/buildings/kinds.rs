//! The 44 standalone building / bridge / doodad types and their defaults.

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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    Modern,
    Medieval,
    Infrastructure,
    Props,
}

impl Category {
    pub const ALL: [Category; 4] = [Self::Modern, Self::Medieval, Self::Infrastructure, Self::Props];
    pub fn label(self) -> &'static str {
        match self {
            Self::Modern => "Modern Buildings",
            Self::Medieval => "Medieval Buildings",
            Self::Infrastructure => "Bridges & Infrastructure",
            Self::Props => "Doodads & Street Props",
        }
    }
    pub fn kinds(self) -> impl Iterator<Item = BuildingKind> {
        BuildingKind::ALL.into_iter().filter(move |k| k.category() == self)
    }
}

impl BuildingKind {
    pub const ALL: [BuildingKind; 44] = {
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
        ]
    };

    pub fn category(self) -> Category {
        let i = self as usize;
        match i {
            0..=13 => Category::Modern,
            14..=25 => Category::Medieval,
            26..=30 => Category::Infrastructure,
            _ => Category::Props,
        }
    }

    pub fn is_medieval(self) -> bool {
        use BuildingKind::*;
        self.category() == Category::Medieval
            || matches!(self, ArchBridge | Drawbridge | LanternPost | Cart | MarketStall | BarrelCrates | Fountain)
    }

    pub fn label(self) -> &'static str {
        use BuildingKind::*;
        match self {
            GlassSkyscraper => "Glass Skyscraper",
            OfficeBlock => "Office Block",
            BrickTenement => "Brick Tenement",
            ApartmentBlock => "Apartment Block",
            Shop => "Shop with Awning",
            GasStation => "Gas Station",
            Warehouse => "Warehouse",
            SawtoothFactory => "Sawtooth Factory",
            ParkingGarage => "Parking Garage",
            School => "School",
            Hospital => "Hospital",
            ModernChurch => "Modern Church",
            WaterTower => "Water Tower",
            Kiosk => "Street Kiosk",
            CastleKeep => "Castle Keep",
            GuildHall => "Guild Hall",
            MarketHall => "Market Hall",
            Tavern => "Half-Timber Tavern",
            StoneChapel => "Stone Chapel",
            Windmill => "Windmill",
            Watermill => "Watermill",
            Forge => "Blacksmith Forge",
            Stable => "Stable",
            Granary => "Granary",
            Gatehouse => "Gatehouse",
            WellHouse => "Well House",
            ArchBridge => "Stone Arch Bridge",
            TrussBridge => "Steel Truss Bridge",
            Drawbridge => "Drawbridge",
            Quay => "Quay / Pier",
            Crane => "Crane",
            Streetlamp => "Streetlamp",
            LanternPost => "Lantern Post",
            Car => "Car / Truck",
            Cart => "Wooden Cart / Wagon",
            MarketStall => "Market Stall",
            BarrelCrates => "Barrel / Crate Cluster",
            Dumpster => "Dumpster",
            SandbagWall => "Sandbag Wall",
            TankTrap => "Tank Trap (Czech Hedgehog)",
            BarbedWire => "Barbed Wire Coil",
            Bench => "Park Bench",
            Fountain => "Fountain / Well",
            RubblePile => "Rubble Pile",
        }
    }

    /// Default (width, length, floors) in metres.
    pub fn defaults(self) -> (f32, f32, u32) {
        use BuildingKind::*;
        match self {
            GlassSkyscraper => (28.0, 28.0, 40),
            OfficeBlock => (24.0, 18.0, 8),
            BrickTenement => (14.0, 12.0, 5),
            ApartmentBlock => (36.0, 14.0, 9),
            Shop => (10.0, 9.0, 2),
            GasStation => (22.0, 16.0, 1),
            Warehouse => (30.0, 20.0, 1),
            SawtoothFactory => (36.0, 24.0, 1),
            ParkingGarage => (32.0, 24.0, 5),
            School => (40.0, 16.0, 2),
            Hospital => (40.0, 30.0, 6),
            ModernChurch => (16.0, 26.0, 1),
            WaterTower => (8.0, 8.0, 1),
            Kiosk => (2.6, 2.0, 1),
            CastleKeep => (16.0, 16.0, 4),
            GuildHall => (14.0, 20.0, 3),
            MarketHall => (12.0, 18.0, 2),
            Tavern => (11.0, 9.0, 2),
            StoneChapel => (8.0, 14.0, 1),
            Windmill => (7.0, 7.0, 3),
            Watermill => (9.0, 8.0, 2),
            Forge => (8.0, 7.0, 1),
            Stable => (14.0, 7.0, 1),
            Granary => (6.0, 6.0, 2),
            Gatehouse => (14.0, 8.0, 3),
            WellHouse => (3.6, 3.6, 1),
            ArchBridge => (8.0, 36.0, 3),
            TrussBridge => (9.0, 48.0, 1),
            Drawbridge => (7.0, 20.0, 1),
            Quay => (8.0, 28.0, 1),
            Crane => (6.0, 18.0, 8),
            Streetlamp => (0.6, 0.6, 1),
            LanternPost => (0.5, 0.5, 1),
            Car => (1.9, 4.4, 1),
            Cart => (1.6, 3.2, 1),
            MarketStall => (3.0, 2.2, 1),
            BarrelCrates => (2.4, 2.0, 1),
            Dumpster => (1.9, 1.2, 1),
            SandbagWall => (0.8, 4.0, 3),
            TankTrap => (1.8, 1.8, 1),
            BarbedWire => (0.9, 6.0, 1),
            Bench => (1.8, 0.6, 1),
            Fountain => (4.0, 4.0, 1),
            RubblePile => (10.0, 8.0, 1),
        }
    }

    /// Whether Floors means anything for this type.
    pub fn has_floors(self) -> bool {
        self.category() != Category::Props || matches!(self, BuildingKind::SandbagWall)
    }
}
