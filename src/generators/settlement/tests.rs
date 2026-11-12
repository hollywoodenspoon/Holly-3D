use super::*;
use crate::generators::kit::ids;
use std::collections::HashSet;

fn small(era: Era) -> SettlementParams {
    SettlementParams { width: 120.0, length: 120.0, ..SettlementParams::of(era) }
}

fn mat_ids(m: &EditablePoly) -> HashSet<u16> {
    m.polygons.values().map(|p| p.material_id).collect()
}

#[test]
fn every_era_builds_a_valid_deterministic_settlement() {
    for era in Era::ALL {
        let p = small(era);
        let m = p.build();
        m.validate().unwrap_or_else(|e| panic!("{era:?}: {e}"));
        let (v, e, f) = m.counts();
        assert!(f > 500 && v > f / 2 && e > f, "{era:?}: {v} {e} {f}");
        assert_eq!(p.build().counts(), (v, e, f), "{era:?} deterministic");
        assert_ne!(SettlementParams { seed: 42, ..p.clone() }.build().counts(), (v, e, f), "{era:?} seed");
        let ids = mat_ids(&m);
        assert!([ids::ROAD, ids::ASPHALT, ids::COBBLE].iter().any(|i| ids.contains(i)), "{era:?} has streets");
        let roofs = [ids::ROOF, ids::SLATE, ids::IRON, ids::RUST];
        assert!(roofs.iter().any(|i| ids.contains(i)), "{era:?} has roofs");
        let walls = [ids::FACADE, ids::BRICK, ids::STUCCO, ids::PLASTER];
        assert!(walls.iter().any(|i| ids.contains(i)), "{era:?} has walls");
        for id in [ids::LOT, ids::FOLIAGE] {
            assert!(ids.contains(&id), "{era:?} has ID {id}");
        }
        assert!(ids.iter().all(|&i| (1..=ids::MAX).contains(&i)), "{era:?} IDs in the GhostTown pack");
        assert!(!ids.contains(&ids::CHARRED) && !ids.contains(&ids::REBAR), "{era:?} undamaged");
        // Fits its area (props may overhang a little).
        let b = m.bounds().unwrap();
        assert!(b.min.x > -70.0 && b.max.x < 70.0 && b.min.y > -70.0 && b.max.y < 70.0, "{era:?} bounds {b:?}");
    }
}

#[test]
fn era_signatures() {
    let ids_of = |era| mat_ids(&small(era).build());
    let down = ids_of(Era::Downtown);
    assert!(down.contains(&ids::GLASS) && down.contains(&ids::LIT), "glass towers with lit windows");
    let downtown = small(Era::Downtown).build();
    let tallest = downtown.bounds().unwrap().max.z;
    assert!(tallest > 25.0, "skyscrapers: {tallest}");
    let sub = ids_of(Era::Suburbs);
    assert!(sub.contains(&ids::GLASS) && sub.contains(&ids::TRIM), "houses with windows and garages");
    let suburb_top = small(Era::Suburbs).build().bounds().unwrap().max.z;
    assert!(suburb_top < 16.0, "low houses: {suburb_top}");
    let walled = ids_of(Era::WalledCity);
    assert!(walled.contains(&ids::TRIM) && walled.contains(&ids::PLASTER), "half-timbered townhouses");
    assert!(walled.contains(&ids::IRON), "gate portcullis");
    assert!(!walled.contains(&ids::REBAR), "no concrete in the middle ages");
    let village = ids_of(Era::Village);
    assert!(village.contains(&ids::TRIM) && !village.contains(&ids::REBAR));
}

#[test]
fn density_and_floors_scale_the_buildings() {
    let sparse = SettlementParams { density: 0.2, ..small(Era::Downtown) }.build();
    let dense = SettlementParams { density: 1.0, ..small(Era::Downtown) }.build();
    assert!(dense.polygons.len() > sparse.polygons.len());
    let low = SettlementParams { min_floors: 2, max_floors: 3, ..small(Era::Downtown) }.build();
    assert!(low.bounds().unwrap().max.z < dense.bounds().unwrap().max.z);
}

#[test]
fn modifier_fills_a_plane_and_a_closed_spline() {
    let mut plane = EditablePoly::new();
    let v: Vec<_> = [(0., 0.), (1., 0.), (1., 1.), (0., 1.)].iter().map(|&(x, y)| plane.add_vertex(Vec3::new(x, y, 0.))).collect();
    plane.add_polygon(&v);
    plane.rebuild_edges();
    plane.transform(Mat4::from_scale(Vec3::new(50.0, 40.0, 1.0)) * Mat4::from_translation(Vec3::new(1.0, 1.0, 3.0)));
    let b0 = plane.bounds().unwrap();
    let md = SettlementMod { params: SettlementParams::of(Era::Village) };
    md.apply(&mut plane);
    plane.validate().unwrap();
    let b = plane.bounds().unwrap();
    assert!(plane.polygons.len() > 200);
    assert!((b.min.z - b0.min.z).abs() < 1.0, "sits on the plane");
    let c = (b.min + b.max) * 0.5;
    let c0 = (b0.min + b0.max) * 0.5;
    assert!(c.truncate().distance(c0.truncate()) < 8.0, "centred on the input");

    use crate::splines::{Knot, Shape, Spline};
    let pts = [Vec3::ZERO, Vec3::new(80.0, 0.0, 0.0), Vec3::new(90.0, 60.0, 0.0), Vec3::new(10.0, 70.0, 0.0)];
    let shape = Shape {
        splines: vec![Spline { knots: pts.iter().map(|&p| Knot::corner(p)).collect(), closed: true, selected: false }],
        ..Default::default()
    };
    let m = SettlementMod { params: SettlementParams::of(Era::WalledCity) }.build_from_shape(&shape);
    m.validate().unwrap();
    let b = m.bounds().unwrap();
    assert!(b.min.x > -8.0 && b.max.x < 98.0 && b.min.y > -8.0 && b.max.y < 78.0, "inside the footprint: {b:?}");
}

#[test]
fn damaged_mode_is_period_accurate() {
    for era in Era::ALL {
        let mut p = small(era);
        p.damaged = true;
        p.damage.intensity = 85.0;
        let m = p.build();
        m.validate().unwrap_or_else(|e| panic!("{era:?}: {e}"));
        let ids = mat_ids(&m);
        if era.is_medieval() {
            assert!(!ids.contains(&ids::REBAR), "{era:?}: no rebar or concrete");
            assert!(ids.contains(&ids::CHARRED), "{era:?}: burned roofs");
        } else {
            assert!(ids.contains(&ids::REBAR), "{era:?}: exposed rebar");
        }
        assert!(m.bounds().unwrap().min.z < -0.3, "{era:?}: craters");
    }
}

#[test]
fn pr6_toggles_shape_the_town() {
    let lit = |m: &EditablePoly| mat_ids(m).iter().any(|&i| i == ids::LIT || i == ids::NEON);
    for era in [Era::RiverCity, Era::Downtown, Era::CastleTown] {
        let dark = SettlementParams { emissive: false, ..small(era) }.build();
        assert!(!lit(&dark), "{era:?}: Emissive Lights off");
    }
    let rc = small(Era::RiverCity);
    assert!(mat_ids(&rc.build()).contains(&ids::WATER), "river");
    let mut dry = rc.clone();
    dry.features.river = false;
    assert!(!mat_ids(&dry.build()).contains(&ids::WATER), "River off");
    let all = rc.build().polygons.len();
    let fewer = SettlementParams { skip: Era::RiverCity.palette()[..4].to_vec(), ..rc.clone() }.build().polygons.len();
    assert_ne!(all, fewer, "unchecked building types are left out");
    let real = SettlementParams { prop_style: PropStyle::Realistic, ..rc.clone() }.build().polygons.len();
    let low = SettlementParams { prop_style: PropStyle::LowPoly, ..rc.clone() }.build().polygons.len();
    assert!(real > low && low > all, "prop styles: {all} {low} {real}");
    let mut open = small(Era::StarFort);
    open.features.walls = false;
    assert!(open.build().polygons.len() < small(Era::StarFort).build().polygons.len(), "Town Walls off");
}

#[test]
fn pr7_destruction_levels_wreck_the_town() {
    use crate::generators::ruin::DestructionLevel;
    let intact = small(Era::Downtown).build();
    let mut p = small(Era::Downtown);
    // Ruin jitters to Collapse..Rubble: every tower comes down.
    p.ruin.level = DestructionLevel::Ruin;
    let wrecked = p.build();
    wrecked.validate().unwrap();
    assert!(mat_ids(&wrecked).contains(&ids::DUST), "rubble mounds");
    assert!(wrecked.bounds().unwrap().max.z < intact.bounds().unwrap().max.z, "towers brought down");
    assert_eq!(p.build().counts(), wrecked.counts(), "deterministic");
    let mut walled = small(Era::WalledCity);
    walled.ruin.level = DestructionLevel::Collapse;
    let w = walled.build();
    w.validate().unwrap();
    assert!(!mat_ids(&w).contains(&ids::REBAR), "medieval rubble has no rebar");
    assert!(mat_ids(&w).contains(&ids::DUST));
}
