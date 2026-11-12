//! The 24-ID GhostTown Material Pack, rebuilt from the Material Library's
//! MaterialCraft procedural textures (PR 5.5) with Triplanar mapping, and a
//! Photo variant that swaps in installed Poly Haven photo textures where a
//! matching group is installed (falling back to the procedural look).

use super::{Material, library};
use crate::assets::polyhaven::Group;

/// (ID label, library item, fallback colour, metallic, roughness, photo group).
type Row = (&'static str, &'static str, [f32; 3], f32, f32, Option<Group>);

/// IDs 1..=24, matching `generators::kit::ids`.
const ROWS: [Row; 24] = [
    ("Road / Cobblestone", "Worn Asphalt", [0.3, 0.3, 0.31], 0.0, 0.9, Some(Group::Roads)),
    ("Sidewalk / Lot / Dirt", "Dirt Path", [0.5, 0.45, 0.38], 0.0, 0.95, Some(Group::Ground)),
    ("Primary Facade / Stone", "Limestone", [0.62, 0.6, 0.55], 0.0, 0.85, Some(Group::Stone)),
    ("Timber / Brick Trim", "Weathered Wood", [0.36, 0.22, 0.13], 0.0, 0.8, Some(Group::Wood)),
    ("Roof Tiles / Thatch", "", [0.45, 0.2, 0.14], 0.0, 0.75, Some(Group::Roofs)),
    ("Window Glass", "", [0.2, 0.28, 0.34], 0.7, 0.08, None),
    ("Lit Windows / Lamps", "", [1.0, 0.82, 0.5], 0.0, 0.4, None),
    ("Foliage / Moss", "Moss", [0.25, 0.42, 0.16], 0.0, 0.9, None),
    ("Charred Timber", "Charred Timber Planks", [0.06, 0.05, 0.045], 0.0, 0.95, None),
    ("Wrought Iron / Steel", "Cast Iron", [0.24, 0.24, 0.26], 0.9, 0.45, Some(Group::Metals)),
    ("Plaster / Mortar", "", [0.86, 0.82, 0.72], 0.0, 0.9, None),
    ("Rusted Rebar", "", [0.42, 0.22, 0.12], 0.75, 0.7, None),
    ("Asphalt", "New Asphalt", [0.1, 0.1, 0.11], 0.0, 0.92, Some(Group::Roads)),
    ("Painted Lane Lines", "", [0.95, 0.9, 0.75], 0.0, 0.6, None),
    ("Sidewalk Slabs", "Sidewalk Slabs", [0.6, 0.6, 0.58], 0.0, 0.85, None),
    ("Curbs", "Curb Stone", [0.55, 0.55, 0.53], 0.0, 0.8, None),
    ("Cobbles", "Medieval Cobbles", [0.42, 0.4, 0.37], 0.0, 0.85, None),
    ("River Water", "", [0.55, 0.7, 0.72], 0.0, 0.03, None),
    ("Brick", "Old Brick Wall", [0.55, 0.24, 0.16], 0.0, 0.85, Some(Group::Walls)),
    ("Stucco / Plaster B", "", [0.8, 0.68, 0.5], 0.0, 0.9, None),
    ("Slate / Metal Roof", "Slate", [0.24, 0.27, 0.32], 0.0, 0.6, None),
    ("Rusted Metal", "Heavy Rust", [0.45, 0.25, 0.14], 0.6, 0.75, None),
    ("Broken Glass", "", [0.3, 0.36, 0.38], 0.4, 0.25, None),
    ("Neon / Lantern Glow", "", [1.0, 0.45, 0.25], 0.0, 0.4, None),
];

/// Flat preview colour of an ID (browser thumbnails).
pub fn swatch(id: u16) -> [f32; 3] {
    ROWS.get((id as usize).wrapping_sub(1)).map_or([0.6; 3], |r| r.2)
}

/// The pack's materials, IDs 1..=24 in order (`photo` = Photo style).
pub fn materials(photo: bool) -> Vec<Material> {
    let photos = if photo { crate::assets::photo::all() } else { vec![] };
    ROWS.iter()
        .enumerate()
        .map(|(i, &(label, item, rgb, metallic, roughness, group))| {
            let id = i as u16 + 1;
            let from_photo = group.and_then(|g| photos.iter().find(|p| p.group == g)).map(|p| p.material());
            let mut m = from_photo
                .or_else(|| (!item.is_empty()).then(|| library::find(item).map(|it| it.material())).flatten())
                .unwrap_or_else(|| Material { base_color: rgb, metallic, roughness, ..Material::named(label) });
            m.name = label.into();
            match id {
                6 | 23 => (m.metallic, m.roughness) = (metallic, roughness),
                7 => (m.emissive, m.emissive_strength) = (rgb, 5.0),
                24 => (m.emissive, m.emissive_strength) = (rgb, 10.0),
                18 => {
                    let mut w = super::Preset::Crystal.material();
                    (w.base_color, w.attenuation_color, w.roughness, w.ior) = (rgb, [0.2, 0.4, 0.38], roughness, 1.33);
                    w.name = label.into();
                    m = w;
                }
                _ => {}
            }
            m
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_has_24_named_ids_built_from_the_library() {
        let mats = materials(false);
        assert_eq!(mats.len(), 24);
        assert_eq!(mats[18].name, "Brick");
        // Library items carry their MaterialCraft procedural surface.
        assert!(mats[18].proc != Material::default().proc, "brick is a MaterialCraft pattern");
        assert!(mats[6].emissive_strength > 0.0 && mats[23].emissive_strength > 0.0, "lit windows and neon glow");
        assert!(mats[0].emissive_strength == 0.0);
        // No photos installed in tests: the Photo style falls back.
        assert_eq!(materials(true).len(), 24);
    }
}
