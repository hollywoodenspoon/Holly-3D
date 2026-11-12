use super::*;
use crate::generators::kit::{Kit, ids};
use bevy::math::{Mat4, Vec2};

/// A small block: ground grid, three buildings with windows and gable
/// roofs, a lamp post and a column.
fn town() -> EditablePoly {
    let mut k = Kit::new();
    k.mat(ids::LOT).ground(Vec2::splat(-30.0), Vec2::splat(30.0), 2.0, |_| 0.0, |_| true);
    k.mat(ids::ROAD).strip(Vec2::new(-30.0, -12.0), Vec2::new(30.0, -12.0), 6.0, 2.0, |_| 0.02);
    for (i, x) in [-14.0f32, 0.0, 14.0].into_iter().enumerate() {
        k.at(Mat4::from_translation(Vec3::new(x, 0.0, 0.0)), |k| {
            k.mat(ids::FACADE).block(Vec3::new(-4.0, -3.0, 0.0), Vec3::new(4.0, 3.0, 9.0 + i as f32 * 3.0));
            for z in [2.0, 5.0] {
                for wx in [-2.0, 0.0, 2.0] {
                    k.mat(ids::GLASS).quad(
                        Vec3::new(wx - 0.5, -3.05, z),
                        Vec3::new(wx + 0.5, -3.05, z),
                        Vec3::new(wx + 0.5, -3.05, z + 1.2),
                        Vec3::new(wx - 0.5, -3.05, z + 1.2),
                    );
                }
            }
            for mx in 0..6 {
                let x = -3.5 + mx as f32 * 1.4;
                k.mat(ids::FACADE).block(Vec3::new(x, -3.0, 9.0 + i as f32 * 3.0), Vec3::new(x + 0.7, -2.4, 10.0 + i as f32 * 3.0));
            }
            k.at(Mat4::from_translation(Vec3::Z * (9.0 + i as f32 * 3.0)), |k| k.mat(ids::ROOF).gable(4.0, 3.0, 3.0, 0.3, ids::FACADE));
        });
    }
    k.mat(ids::IRON).frustum(Vec3::new(6.0, -8.0, 0.0), 0.12, 0.1, 5.0, 8, true);
    k.mat(ids::FACADE).frustum(Vec3::new(-6.0, 8.0, 0.0), 0.4, 0.35, 5.0, 12, true);
    k.finish()
}

fn run(mode: DestructionMode, f: impl FnOnce(&mut BattleParams)) -> EditablePoly {
    let mut m = town();
    let mut p = BattleParams::of(mode);
    f(&mut p);
    BattlefieldMod { params: p, zones: vec![] }.apply(&mut m);
    m
}

fn count(m: &EditablePoly, id: u16) -> usize {
    m.polygons.values().filter(|p| p.material_id == id).count()
}

#[test]
fn zero_intensity_leaves_mesh_untouched() {
    for mode in DestructionMode::ALL {
        let src = town();
        let out = run(mode, |p| p.intensity = 0.0);
        assert_eq!(out.counts(), src.counts(), "{mode:?}");
        let a: Vec<_> = src.vertices.values().map(|v| v.position).collect();
        let b: Vec<_> = out.vertices.values().map(|v| v.position).collect();
        assert_eq!(a, b);
    }
}

#[test]
fn every_mode_damages_validly_and_deterministically() {
    let src = town();
    for mode in DestructionMode::ALL {
        let a = run(mode, |_| {});
        let b = run(mode, |_| {});
        a.validate().unwrap_or_else(|e| panic!("{mode:?}: {e}"));
        assert_eq!(a.counts(), b.counts(), "{mode:?} deterministic");
        assert_ne!(a.counts(), src.counts(), "{mode:?} changed the mesh");
        // Craters / sinkholes dig below the original ground.
        assert!(a.bounds().unwrap().min.z < -0.3, "{mode:?} craters");
        assert!(a.polygons.values().all(|p| p.corners.len() >= 3));
        let other = run(mode, |p| p.seed = 99);
        assert_ne!(other.counts(), a.counts(), "{mode:?} seed matters");
    }
}

#[test]
fn modern_mode_has_rebar_and_shattered_glass() {
    let m = run(DestructionMode::Modern, |p| p.intensity = 90.0);
    assert!(count(&m, ids::REBAR) > 0, "bent rebar");
    assert!(count(&m, ids::GLASS) > 0);
    assert!(m.polygons.values().any(|p| p.material_id == ids::GLASS && p.corners.len() == 3), "glass shards");
    assert_eq!(count(&m, ids::CHARRED), 0);
}

#[test]
fn medieval_siege_burns_roofs_without_rebar() {
    let src = town();
    let m = run(DestructionMode::MedievalSiege, |p| p.intensity = 90.0);
    assert_eq!(count(&m, ids::REBAR), 0, "no rebar in a medieval siege");
    assert!(count(&m, ids::CHARRED) > 0, "charred rafters");
    assert!(count(&m, ids::ROOF) < count(&src, ids::ROOF), "roofs burned away");
    assert!(m.vertices.values().any(|v| v.color.x < 0.2), "charred / sooty vertex colours");
}

#[test]
fn ancient_ruins_overgrow_and_erode() {
    let src = town();
    let m = run(DestructionMode::AncientRuins, |p| p.intensity = 80.0);
    assert_eq!(count(&m, ids::REBAR), 0);
    assert!(count(&m, ids::FOLIAGE) > 0, "moss");
    assert!(m.vertices.values().any(|v| v.color == weather::MOSS), "green moss vertex colour");
    assert!(m.bounds().unwrap().max.z < src.bounds().unwrap().max.z, "eroded tops");
}

#[test]
fn user_blast_zone_only_damages_its_area() {
    let mut m = town();
    let p = BattleParams { crater_count: 1, crater_radius: 3.0, ..BattleParams::of(DestructionMode::Modern) };
    let fx = BattlefieldMod { params: p, zones: vec![BlastZone::new(Vec3::new(14.0, 0.0, 0.0), 7.0)] };
    fx.apply(&mut m);
    // The tallest building (x = 14) is cut down; the far one (x = -14) keeps its glass.
    let far_glass = m.polygons.values().filter(|p| p.material_id == ids::GLASS).filter(|p| m.pos(p.corners[0].vertex_id).x < -10.0).count();
    assert_eq!(far_glass, 6);
    // Concrete only: bent rebar may poke up out of the sheared top.
    let near_top = m
        .polygons
        .values()
        .filter(|p| p.material_id == ids::FACADE)
        .flat_map(|p| p.vertex_ids())
        .map(|v| m.pos(v))
        .filter(|q| q.x > 10.0 && q.x < 18.0 && q.y.abs() < 3.1)
        .map(|q| q.z)
        .fold(0.0, f32::max);
    assert!(near_top < 15.0, "struck building lost its top: {near_top}");
}

#[test]
fn zone_add_and_remove() {
    let mut b = BattlefieldMod::default();
    let bounds = town().bounds();
    b.add_zone(bounds);
    b.add_zone(bounds);
    assert_eq!(b.zones.len(), 2);
    assert!(b.zones[1].selected && !b.zones[0].selected);
    assert!(b.zones[1].center.x > b.zones[0].center.x);
    assert_eq!(b.remove_selected(), 1);
    assert_eq!(b.zones.len(), 1);
}

#[test]
fn crater_profiles_dig_and_rim() {
    use craters::{Crater, Kind};
    for kind in [Kind::Bomb, Kind::Pit, Kind::Sinkhole, Kind::Footprint(0.3)] {
        let c = Crater { c: Vec2::ZERO, r: 4.0, depth: 2.0, kind };
        assert!(c.profile(Vec2::ZERO) < -0.5 || matches!(kind, Kind::Footprint(_)), "{kind:?} bowl");
        assert_eq!(c.profile(Vec2::new(c.reach() + 0.1, 0.0)), 0.0, "{kind:?} flat beyond reach");
    }
    let bomb = Crater { c: Vec2::ZERO, r: 4.0, depth: 2.0, kind: Kind::Bomb };
    assert!(bomb.profile(Vec2::new(4.0, 0.0)) > 0.3, "raised rim");
    let foot = Crater { c: Vec2::ZERO, r: 4.0, depth: 2.0, kind: Kind::Footprint(0.0) };
    assert!(foot.profile(Vec2::new(-1.8, 0.0)) < -0.5, "heel");
    assert!(foot.profile(Vec2::new(2.2, 0.0)) < -0.5, "middle toe");
}

#[test]
fn pr7_building_destruction_wrecks_each_building() {
    use crate::generators::ruin::DestructionLevel;
    let intact = town();
    assert_eq!(clusters::buildings(&intact).len(), 5, "three buildings, a lamp and a column; ground skipped");
    let top = intact.bounds().unwrap().max.z;
    let ruined = run(DestructionMode::Modern, |p| {
        p.intensity = 0.0;
        p.buildings.level = DestructionLevel::Ruin;
    });
    ruined.validate().unwrap();
    assert!(ruined.bounds().unwrap().max.z < top, "buildings brought down");
    assert!(count(&ruined, ids::DUST) > 0, "rubble mounds");
    assert!(count(&ruined, ids::LOT) == count(&intact, ids::LOT), "ground untouched");
    let siege = run(DestructionMode::MedievalSiege, |p| p.buildings.level = DestructionLevel::Collapse);
    siege.validate().unwrap();
    assert_eq!(count(&siege, ids::REBAR), 0, "medieval era from the mode");
    let off = run(DestructionMode::Modern, |p| p.intensity = 0.0);
    assert_eq!(off.counts(), intact.counts(), "Intact + 0 % leaves the input alone");
}

#[test]
fn pr8_spawns_rising_smoke_plumes_from_the_wrecks() {
    let ids_of = |m: &EditablePoly| m.polygons.values().map(|p| p.material_id).collect::<std::collections::HashSet<_>>();
    let mut off = town();
    BattlefieldMod::default().apply(&mut off);
    assert!(!ids_of(&off).contains(&ids::SMOKE), "off by default");
    let p = BattleParams { plumes: true, plume_count: 2, ..BattleParams::default() };
    let run = || {
        let mut m = town();
        BattlefieldMod { params: p.clone(), zones: vec![] }.apply(&mut m);
        m
    };
    let m = run();
    m.validate().unwrap();
    assert!(ids_of(&m).contains(&ids::SMOKE) && ids_of(&m).contains(&ids::EMBERS));
    assert!(m.bounds().unwrap().max.z > off.bounds().unwrap().max.z + 20.0, "columns rise into the sky");
    assert_eq!(run().counts(), m.counts(), "deterministic");
    let more = {
        let mut m = town();
        BattlefieldMod { params: BattleParams { plume_count: 5, ..p.clone() }, zones: vec![] }.apply(&mut m);
        m.polygons.len()
    };
    assert!(more > m.polygons.len(), "Smoke Plume Count");
}
