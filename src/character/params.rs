//! Character Studio parameters (the `BaseObject::Character` payload).

use super::makehuman::macros::Macros;
use std::collections::BTreeMap;

/// Material IDs of every generated character (the Character Pack).
pub mod ids {
    pub const SKIN: u16 = 1;
    pub const EYES: u16 = 2;
    pub const HAIR: u16 = 3;
    pub const CLOTH: u16 = 4;
    pub const METAL: u16 = 5;
    pub const LEATHER: u16 = 6;
    // MakeHuman system assets (`character::wardrobe`).
    pub const EYEBROWS: u16 = 7;
    pub const EYELASHES: u16 = 8;
    pub const TEETH: u16 = 9;
    pub const TONGUE: u16 = 10;
    pub const MH_HAIR: u16 = 11;
    /// The first clothing item; the k-th worn item is `CLOTHES + k`.
    pub const CLOTHES: u16 = 12;
}

macro_rules! labelled {
    ($name:ident { $($v:ident => $l:literal),+ $(,)? }) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
        pub enum $name {
            #[default]
            $($v),+
        }
        impl $name {
            pub const ALL: &'static [$name] = &[$(Self::$v),+];
            pub fn label(self) -> &'static str {
                match self { $(Self::$v => $l),+ }
            }
        }
    };
}

labelled!(Archetype {
    Human => "Human (MakeHuman Default)",
    Elf => "Elf",
    Dwarf => "Dwarf",
    Goblin => "Goblin",
    Orc => "Orc",
    Chibi => "Chibi",
    AnimeHero => "Anime Hero",
    AnimeHeroine => "Anime Heroine",
});

labelled!(Hair {
    Bald => "Bald",
    Crop => "Short Crop",
    Long => "Long Straight",
    Ponytail => "Ponytail",
    Spiky => "Spiky Anime",
    Bun => "Top Bun",
    Beard => "Crop + Full Beard",
});

labelled!(Outfit {
    None => "None (Base Body)",
    Bodysuit => "Bodysuit / Tights",
    Tunic => "Tunic & Skirt",
    Knight => "Knight Plate Armor",
    Ranger => "Leather Ranger",
    Robe => "Mage Robe",
});

labelled!(Weapon {
    None => "None",
    Sword => "Longsword",
    Axe => "Battle Axe",
    Staff => "Mage Staff",
    Shield => "Sword & Round Shield",
});

labelled!(Eyes {
    Low => "Low-Poly Eyes",
    High => "High-Poly Eyes",
    None => "No Eyes",
});

labelled!(Creature {
    Humanoid => "(Humanoid — use the body above)",
    Wolf => "Dire Wolf",
    Dragon => "Dragon",
    Spider => "Giant Spider",
    Slime => "Slime",
    Golem => "Stone Golem",
    Serpent => "Sea Serpent",
});

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct CharacterParams {
    pub archetype: Archetype,
    pub macros: Macros,
    /// Micro slider values by key (see `makehuman::modifiers`).
    pub micro: BTreeMap<String, f32>,
    /// Anime Ratio 0..=1: big eyes, V-jaw, larger head, toon normals.
    pub anime: f32,
    pub eyes: Eyes,
    pub hair: Hair,
    pub hair_length: f32,
    pub outfit: Outfit,
    pub weapon: Weapon,
    pub creature: Creature,
    pub creature_size: f32,
    pub seed: u32,
    /// Phase 10B Easy Pose (empty = the A-pose rest).
    pub pose: super::rig::Pose,
    /// MakeHuman eyebrows, lashes, teeth, hair, clothes, skin, eye colour.
    pub mh: super::wardrobe::MhAssets,
}

impl Default for CharacterParams {
    fn default() -> Self {
        Self {
            archetype: Archetype::Human,
            macros: Macros::default(),
            micro: BTreeMap::new(),
            anime: 0.0,
            eyes: Eyes::Low,
            hair: Hair::Crop,
            hair_length: 0.5,
            outfit: Outfit::Bodysuit,
            weapon: Weapon::None,
            creature: Creature::Humanoid,
            creature_size: 1.0,
            seed: 1,
            pose: Default::default(),
            mh: Default::default(),
        }
    }
}

impl CharacterParams {
    pub fn short(&self) -> &'static str {
        if self.creature != Creature::Humanoid {
            return self.creature.label();
        }
        match self.archetype {
            Archetype::Human => "Character",
            a => a.label(),
        }
    }

    /// Sets a micro slider, mirroring `r-` keys onto their `l-` twin.
    pub fn set_micro(&mut self, key: &str, v: f32) {
        self.micro.insert(key.to_string(), v);
        if let Some(i) = key.find("/r-") {
            self.micro.insert(format!("{}/l-{}", &key[..i], &key[i + 3..]), v);
        }
    }
}
