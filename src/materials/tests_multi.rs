//! Multi/Sub-Object materials: ID lookup, wrapping, On/Off, Set Number,
//! Standard <-> Multi conversion and the three pack presets.

use super::multi::{DEFAULT_SUBS, MultiSub};
use super::*;

#[test]
fn ids_resolve_wrap_and_switch_off() {
    let mut ms = MultiSub::new(3);
    assert_eq!(ms.subs.iter().map(|s| s.id).collect::<Vec<_>>(), vec![1, 2, 3]);
    ms.subs[1].mat.base_color = [1.0, 0.0, 0.0];
    assert_eq!(ms.resolve(2).base_color, [1.0, 0.0, 0.0]);
    // ID 5 wraps to the 2nd entry of a 3-entry list.
    assert_eq!(ms.index_for(5), Some(1));
    assert_eq!(ms.index_for(0), Some(0), "ID 0 is treated as ID 1");
    // A renumbered entry is found by its ID, not its position.
    ms.subs[2].id = 9;
    assert_eq!(ms.index_for(9), Some(2));
    ms.subs[1].enabled = false;
    assert_eq!(ms.resolve(2).name, "Default", "Off renders the default material");
    // Set Number grows with fresh IDs above the highest and shrinks from the end.
    ms.set_number(5);
    assert_eq!(ms.subs.iter().map(|s| s.id).collect::<Vec<_>>(), vec![1, 2, 9, 10, 11]);
    ms.set_number(2);
    assert_eq!(ms.subs.len(), 2);
    ms.set_number(0);
    assert_eq!(ms.subs.len(), 1, "at least one sub-material");
}

#[test]
fn standard_multi_round_trip_and_target() {
    let mut m = Preset::PolishedGold.material();
    m.name = "03 - Gold".into();
    assert!(std::ptr::eq(m.for_id(7), &m), "a standard material renders every ID");
    m.make_multi();
    let ms = m.multi.as_ref().unwrap();
    assert_eq!(ms.subs.len(), DEFAULT_SUBS);
    assert_eq!((ms.subs[0].name.as_str(), ms.subs[0].mat.metallic), ("Gold", 1.0), "the old material becomes ID 1");
    assert_eq!(m.name, "03 - Gold", "the slot keeps its name");
    m.target_mut(3).roughness = 0.9;
    assert_eq!(m.for_id(4).roughness, 0.9, "editing sub 4 changes what ID 4 renders");
    assert_eq!(m.target(99).name, m.multi.as_ref().unwrap().subs[DEFAULT_SUBS - 1].mat.name, "sub index is clamped");
    m.make_standard(3);
    assert!(!m.is_multi());
    assert_eq!((m.roughness, m.name.as_str()), (0.9, "03 - Gold"));
    // A standard material is its own edit target.
    m.target_mut(5).metallic = 0.25;
    assert_eq!(m.metallic, 0.25);
}

#[test]
fn pack_presets_and_save_round_trip() {
    for p in MultiPreset::ALL {
        let mut m = Material::named("05 - Slot");
        p.apply(&mut m);
        let ms = m.multi.as_ref().unwrap();
        let n = ms.subs.len() as u16;
        assert!(n >= 2, "{}", p.label());
        assert_eq!(ms.subs.iter().map(|s| s.id).collect::<Vec<_>>(), (1..=n).collect::<Vec<u16>>(), "{}", p.label());
        assert_eq!(m.name, "05 - Slot");
        let names: Vec<&str> = ms.subs.iter().map(|s| s.name.as_str()).collect();
        assert!(names.iter().all(|n| !n.is_empty()));
        let text = ron::to_string(&m).unwrap();
        let back: Material = ron::from_str(&text).unwrap();
        assert_eq!(back, m, "{} survives save / open", p.label());
    }
    let castle = MultiPreset::MedievalCastle.build();
    assert!(castle.resolve(7).emissive_strength > 0.0, "the torch flame is an emitter");
    assert_eq!(castle.resolve(2).name, "Mortar", "ID 2 (the Panel Groove default) is the mortar");
    let city = MultiPreset::ModernCity.build();
    assert!(city.subs.iter().filter(|s| s.mat.emissive_strength > 0.0).count() >= 2, "neon and windows glow");
    let alpine = MultiPreset::AlpineTerrain.build();
    assert_eq!(alpine.resolve(6).model, ShadingModel::Crystal, "glacier ice refracts");
    assert_eq!((alpine.resolve(1).name.as_str(), alpine.resolve(4).name.as_str()), ("Alpine Grass", "Riverbed Sand"), "matches Terrain IDs");
    assert_eq!(MultiPreset::OceanWater.build().subs.len(), 2);
    assert_eq!(MultiPreset::PlanetStudio.build().subs.len(), 7);
    // Older scene files (no `multi` field) still load.
    let old = ron::to_string(&Material::default()).unwrap().replace(",multi:None", "");
    assert!(!old.contains("multi"));
    assert_eq!(ron::from_str::<Material>(&old).unwrap(), Material::default());
}
