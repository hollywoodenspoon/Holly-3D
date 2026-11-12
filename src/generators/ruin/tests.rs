use super::*;
use crate::generators::buildings::{BuildingKind, BuildingParams};
use crate::generators::kit::ids;
use std::collections::HashSet;

fn mat_ids(m: &EditablePoly) -> HashSet<u16> {
    m.polygons.values().map(|p| p.material_id).collect()
}

fn wreck(kind: BuildingKind, level: DestructionLevel, seed: u32) -> EditablePoly {
    let mut m = BuildingParams::of(kind).build();
    apply(&mut m, &RuinParams { level, seed, ..RuinParams::default() }, kind.is_medieval());
    m
}

const KINDS: [BuildingKind; 5] =
    [BuildingKind::OfficeBlock, BuildingKind::GlassSkyscraper, BuildingKind::BrickTenement, BuildingKind::Tavern, BuildingKind::CastleKeep];

#[test]
fn every_level_builds_valid_deterministic_meshes() {
    for k in KINDS {
        for level in DestructionLevel::ALL {
            let m = wreck(k, level, 3);
            m.validate().unwrap_or_else(|e| panic!("{k:?} {level:?}: {e}"));
            assert!(m.polygons.len() > 10, "{k:?} {level:?}");
            assert!(m.vertices.values().all(|v| v.position.is_finite()));
            assert!(m.polygons.values().all(|p| p.corners.iter().all(|c| c.uv.is_finite())), "{k:?} {level:?} UVs");
            assert_eq!(wreck(k, level, 3).counts(), m.counts(), "{k:?} {level:?} deterministic");
            assert!(mat_ids(&m).iter().all(|&i| (1..=ids::MAX).contains(&i)), "{k:?} {level:?} IDs in the pack");
        }
    }
}

#[test]
fn intact_is_unchanged() {
    for k in KINDS {
        assert_eq!(wreck(k, DestructionLevel::Intact, 3).counts(), BuildingParams::of(k).build().counts());
    }
}

#[test]
fn damage_grows_with_the_level() {
    for k in KINDS {
        let top = |l| wreck(k, l, 5).bounds().unwrap().max.z;
        let intact = top(DestructionLevel::Intact);
        let collapse = top(DestructionLevel::Collapse);
        let ruin = top(DestructionLevel::Ruin);
        let rubble = top(DestructionLevel::Rubble);
        assert!(collapse < intact * 0.9, "{k:?}: collapse lowers the top {collapse} < {intact}");
        assert!(ruin < intact * 0.6, "{k:?}: ruin is a stump {ruin}");
        assert!(rubble < 14.0 && rubble < intact * 0.5, "{k:?}: rubble only {rubble}");
        // Light keeps the shell but cracks and scars it.
        let light = mat_ids(&wreck(k, DestructionLevel::Light, 5));
        assert!(light.contains(&ids::SCAR), "{k:?}: bullet scars / soot");
        let heavy = wreck(k, DestructionLevel::Heavy, 5);
        assert!(mat_ids(&heavy).contains(&ids::DUST), "{k:?}: heavy has rubble");
        let lit = |m: &EditablePoly| m.polygons.values().filter(|p| p.material_id == ids::LIT).count();
        assert!(lit(&wreck(k, DestructionLevel::Light, 5)) <= lit(&BuildingParams::of(k).build()));
    }
}

#[test]
fn period_accurate_debris() {
    for level in [DestructionLevel::Heavy, DestructionLevel::Collapse, DestructionLevel::Ruin, DestructionLevel::Rubble] {
        for k in [BuildingKind::Tavern, BuildingKind::CastleKeep, BuildingKind::StoneChapel] {
            let ids = mat_ids(&wreck(k, level, 9));
            assert!(!ids.contains(&ids::REBAR), "{k:?} {level:?}: no rebar in the middle ages");
        }
        let modern = mat_ids(&wreck(BuildingKind::OfficeBlock, level, 9));
        assert!(modern.contains(&ids::REBAR) || level == DestructionLevel::Heavy, "{level:?}: modern rebar");
    }
    let mut m = BuildingParams::of(BuildingKind::OfficeBlock).build();
    apply(&mut m, &RuinParams { level: DestructionLevel::Ruin, era: RuinEra::Ancient, ..RuinParams::default() }, false);
    assert!(!mat_ids(&m).contains(&ids::REBAR), "Ancient era override");
}

#[test]
fn embers_and_gore_toggles() {
    let run = |embers, gore| {
        let mut m = BuildingParams::of(BuildingKind::BrickTenement).build();
        apply(&mut m, &RuinParams { level: DestructionLevel::Collapse, embers, gore, gore_amount: 80.0, ..RuinParams::default() }, false);
        mat_ids(&m)
    };
    let on = run(true, true);
    assert!(on.contains(&ids::EMBERS) && on.contains(&ids::BLOOD));
    let off = run(false, false);
    assert!(!off.contains(&ids::EMBERS) && !off.contains(&ids::BLOOD));
}

#[test]
fn rubble_pile_is_low_and_spread() {
    let p = RuinParams { pile_height: 2.5, ..RuinParams::default() };
    let m = rubble_pile(bevy::math::Vec2::new(5.0, 4.0), &p, false);
    m.validate().unwrap();
    let b = m.bounds().unwrap();
    assert!(b.max.z < 5.5 && b.max.x > 4.0 && b.min.y < -3.0, "{b:?}");
    let tall = rubble_pile(bevy::math::Vec2::new(5.0, 4.0), &RuinParams { pile_height: 5.0, ..p.clone() }, false);
    assert!(tall.bounds().unwrap().max.z > b.max.z);
    let dense = rubble_pile(bevy::math::Vec2::new(5.0, 4.0), &RuinParams { rubble: 1.0, ..p.clone() }, false);
    let sparse = rubble_pile(bevy::math::Vec2::new(5.0, 4.0), &RuinParams { rubble: 0.1, ..p }, false);
    assert!(dense.polygons.len() > sparse.polygons.len());
}

#[test]
fn stress_fails_lower_storeys_harder() {
    use bevy::math::Vec3;
    let b = crate::mesh::Aabb { min: Vec3::new(-10.0, -10.0, 0.0), max: Vec3::new(10.0, 10.0, 60.0) };
    let mut r = crate::generators::kit::rng(1, 1);
    let light = stress::Stress::new(&b, DestructionLevel::Light, &mut r);
    let mut r = crate::generators::kit::rng(1, 1);
    let ruin = stress::Stress::new(&b, DestructionLevel::Ruin, &mut r);
    assert!(ruin.blasts.len() > light.blasts.len());
    assert!(ruin.failed.is_some(), "a ruin's load path fails");
    assert!(ruin.tilt.is_normalized());
}
