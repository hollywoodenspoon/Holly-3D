use super::arch::{self, Opening, StairKind};
use super::*;
use crate::generators::kit::{Kit, ids};
use bevy::math::{Vec2, Vec3};
use std::collections::HashSet;

fn mat_ids(m: &EditablePoly) -> HashSet<u16> {
    m.polygons.values().map(|p| p.material_id).collect()
}

#[test]
fn all_139_types_build_valid_deterministic_meshes() {
    assert_eq!(BuildingKind::ALL.len(), 139);
    let counts: Vec<usize> = Category::ALL.iter().map(|c| c.kinds().count()).collect();
    assert_eq!(counts, vec![13, 17, 14, 17, 10, 16, 8, 14, 12, 5, 13]);
    for k in BuildingKind::ALL {
        let p = BuildingParams::of(k);
        let m = p.build();
        m.validate().unwrap_or_else(|e| panic!("{k:?}: {e}"));
        let (v, e, f) = m.counts();
        assert!(f >= 6 && v >= 8 && e >= 12, "{k:?}: {v} {e} {f}");
        assert_eq!(p.build().counts(), (v, e, f), "{k:?} deterministic");
        assert!(mat_ids(&m).iter().all(|&i| (1..=ids::MAX).contains(&i)), "{k:?} IDs in the 29-ID pack");
        let b = m.bounds().unwrap();
        assert!(b.min.z > -3.5 && b.max.z > 0.3, "{k:?} stands on the ground: {b:?}");
        let reach = p.width.max(p.length) * 0.5 + 14.0;
        assert!(b.min.x > -reach && b.max.x < reach && b.min.y > -reach && b.max.y < reach, "{k:?} near its footprint: {b:?}");
        assert!(m.vertices.values().all(|v| v.position.is_finite()), "{k:?} finite");
    }
}

#[test]
fn signatures_per_category() {
    let ids_of = |k| mat_ids(&BuildingParams::of(k).build());
    assert!(ids_of(BuildingKind::GlassSkyscraper).contains(&ids::GLASS));
    assert!(BuildingParams::of(BuildingKind::GlassSkyscraper).build().bounds().unwrap().max.z > 100.0, "40 floors is tall");
    assert!(ids_of(BuildingKind::BrickTenement).contains(&ids::BRICK));
    assert!(ids_of(BuildingKind::Tavern).contains(&ids::PLASTER) && ids_of(BuildingKind::Tavern).contains(&ids::TRIM), "half-timber");
    assert!(ids_of(BuildingKind::Watermill).contains(&ids::WATER), "mill race");
    assert!(ids_of(BuildingKind::ArchBridge).contains(&ids::COBBLE));
    assert!(ids_of(BuildingKind::TrussBridge).contains(&ids::IRON) && ids_of(BuildingKind::TrussBridge).contains(&ids::ASPHALT));
    assert!(ids_of(BuildingKind::TankTrap).contains(&ids::RUST));
    for k in BuildingKind::ALL.into_iter().filter(|k| k.is_medieval()) {
        assert!(!ids_of(k).contains(&ids::REBAR) && !ids_of(k).contains(&ids::ASPHALT), "{k:?}: no modern materials");
    }
}

#[test]
fn emissive_toggle_turns_every_light_off() {
    let mut any_lit = false;
    for k in BuildingKind::ALL {
        let on = mat_ids(&BuildingParams { seed: 3, ..BuildingParams::of(k) }.build());
        any_lit |= on.contains(&ids::LIT) || on.contains(&ids::NEON);
        let off = mat_ids(&BuildingParams { seed: 3, emissive: false, ..BuildingParams::of(k) }.build());
        assert!(!off.contains(&ids::LIT) && !off.contains(&ids::NEON), "{k:?}: nothing glows");
    }
    assert!(any_lit);
}

#[test]
fn parameters_drive_the_geometry() {
    let base = BuildingParams::of(BuildingKind::BrickTenement);
    let full = base.build().counts().2;
    let bare = BuildingParams { parts: Parts::NONE, ..base.clone() }.build().counts().2;
    assert!(bare < full, "part toggles remove detail: {bare} vs {full}");
    let tall = BuildingParams { floors: 9, ..base.clone() }.build().bounds().unwrap().max.z;
    assert!(tall > base.build().bounds().unwrap().max.z + 10.0, "more floors are taller");
    let wide = BuildingParams { width: 30.0, ..base.clone() }.build().bounds().unwrap();
    assert!(wide.max.x - wide.min.x > 29.0, "width scales the footprint");
    let clutter = BuildingParams::of(BuildingKind::BarrelCrates);
    let sum = |m: EditablePoly| m.vertices.values().map(|v| v.position).sum::<Vec3>();
    assert_ne!(sum(clutter.build()), sum(BuildingParams { seed: 9, ..clutter.clone() }.build()), "seeded clutter");
    for s in StairKind::ALL {
        let m = BuildingParams { stairs: s, ..BuildingParams::of(BuildingKind::CastleKeep) }.build();
        m.validate().unwrap();
    }
}

#[test]
fn mitred_walls_share_their_corners() {
    let sq = [Vec2::new(-5.0, -5.0), Vec2::new(5.0, -5.0), Vec2::new(5.0, 5.0), Vec2::new(-5.0, 5.0)];
    let o = arch::miter_offsets(&sq, true, 0.2);
    // Corner offsets lie on the diagonal at half-thickness * sqrt(2).
    for (i, (a, b)) in o.iter().enumerate() {
        assert!((a.distance(sq[i]) - 0.2 * 2f32.sqrt()).abs() < 1e-4 && (b.distance(sq[i]) - 0.2 * 2f32.sqrt()).abs() < 1e-4);
        assert!((*a - sq[i]).abs().min_element() > 0.19, "mitred, not squared");
    }
    let mut k = Kit::new();
    arch::wall_path(&mut k, &sq, true, 0.4, 0.0, 3.0);
    let m = k.finish();
    m.validate().unwrap();
    assert_eq!(m.counts().2, 4 * 5, "four wall boxes without bottoms");
    let b = m.bounds().unwrap();
    assert!((b.max.x - 5.2).abs() < 1e-4 && (b.min.y + 5.2).abs() < 1e-4, "outer face at half thickness: {b:?}");
}

#[test]
fn openings_cut_framed_holes_in_walls() {
    let mut k = Kit::new();
    k.mat(ids::FACADE);
    let ops = [
        Opening { at: 2.0, width: 1.0, sill: 0.0, height: 2.2, fill: ids::TRIM },
        Opening { at: 5.0, width: 1.2, sill: 0.9, height: 1.2, fill: ids::GLASS },
    ];
    arch::wall_with_openings(&mut k, 8.0, 0.3, 3.0, &ops);
    let m = k.finish();
    m.validate().unwrap();
    // No wall polygon covers the centre of either opening on the front face.
    for (x, z) in [(2.0, 1.0), (5.0, 1.5)] {
        let covered = m.polygons.values().filter(|p| p.material_id == ids::FACADE).any(|p| {
            let pts: Vec<Vec3> = p.vertex_ids().map(|v| m.pos(v)).collect();
            let (lo, hi) = pts.iter().fold((Vec3::MAX, Vec3::MIN), |(a, b), q| (a.min(*q), b.max(*q)));
            lo.y.abs() < 1e-4 && hi.y.abs() < 1e-4 && lo.x < x && hi.x > x && lo.z < z && hi.z > z
        });
        assert!(!covered, "opening at {x} is open");
    }
    let ids = mat_ids(&m);
    assert!(ids.contains(&ids::GLASS) && ids.contains(&ids::TRIM), "pane, door and frames");
}

#[test]
fn staircases_climb_their_rise() {
    for (kind, want_xy) in [(StairKind::Straight, false), (StairKind::LShaped, true), (StairKind::Spiral, true)] {
        let mut k = Kit::new();
        let n = arch::stairs(&mut k, kind, 1.2, 3.0);
        assert_eq!(n, 17, "{kind:?}: 3 m / 0.18 m risers");
        let m = k.finish();
        m.validate().unwrap();
        let b = m.bounds().unwrap();
        assert!(b.max.z >= 3.0 - 1e-3, "{kind:?} reaches the top: {b:?}");
        let turns = b.max.x > 1.0;
        assert_eq!(turns, want_xy, "{kind:?} turns: {b:?}");
    }
}

#[test]
fn photo_style_picks_the_photo_pack() {
    assert_eq!(MatStyle::Procedural.pack(), crate::materials::MultiPreset::Settlement);
    assert_eq!(MatStyle::Photo.pack(), crate::materials::MultiPreset::SettlementPhoto);
    assert_eq!(crate::materials::MultiPreset::Settlement.build().subs.len(), 29);
}

#[test]
fn mid_detail_adds_window_frames_and_old_scenes_default_to_standard() {
    let std = BuildingParams::of(BuildingKind::BrickTenement);
    let mid = BuildingParams { detail: super::Detail::Mid, ..std.clone() };
    let (a, b) = (std.build(), mid.build());
    assert!(b.polygons.len() > a.polygons.len() * 2, "{} vs {}", a.polygons.len(), b.polygons.len());
    assert!(b.polygons.values().any(|p| p.material_id == crate::generators::kit::ids::TRIM));
    for (pid, _) in &b.polygons {
        assert!(b.poly_normal(pid).is_finite());
    }
    // A scene saved before Detail existed loads as Standard.
    let json = serde_json::to_string(&std).unwrap().replace(",\"detail\":\"Standard\"", "");
    assert!(!json.contains("detail"));
    let back: BuildingParams = serde_json::from_str(&json).unwrap();
    assert_eq!(back.detail, super::Detail::Standard);
}
