use super::*;
use crate::generators::kit::ids;

fn industrial() -> impl Iterator<Item = BuildingKind> {
    Category::Industrial.kinds()
}

#[test]
fn industrial_kinds_are_appended_and_build_valid_meshes() {
    // Appended after Commercial so saved scenes and seeds keep their meaning.
    assert_eq!(BuildingKind::Factory as usize, 74);
    assert_eq!(industrial().count(), 14);
    for k in industrial() {
        assert!(!k.has_floors(), "{k:?} has no floors slider");
        for detail in Detail::ALL {
            let p = BuildingParams { detail, ..BuildingParams::of(k) };
            let m = p.build();
            m.validate().unwrap_or_else(|e| panic!("{k:?}: {e}"));
            let (v, e, f) = m.counts();
            assert!(f >= 60, "{k:?} has some detail: {f}");
            assert!(f < 80_000, "{k:?} poly budget: {f}");
            assert_eq!(p.build().counts(), (v, e, f), "{k:?} deterministic");
            assert!(m.vertices.values().all(|v| v.position.is_finite()), "{k:?} finite");
            for poly in m.polygons.values() {
                assert!(poly.corners.iter().all(|c| c.uv.is_finite()), "{k:?} UVs");
            }
        }
    }
}

#[test]
fn industrial_kinds_stand_on_the_ground_inside_their_lot() {
    for k in industrial() {
        let p = BuildingParams::of(k);
        let b = p.build().bounds().unwrap();
        assert!(b.min.z > -0.01, "{k:?} stands on the ground: {b:?}");
        let reach = p.width.max(p.length) * 0.5 + 14.0;
        assert!(b.min.x > -reach && b.max.x < reach && b.min.y > -reach && b.max.y < reach, "{k:?} fits its lot: {b:?}");
    }
    let tall = |k| BuildingParams::of(k).build().bounds().unwrap().max.z;
    assert!(tall(BuildingKind::WindTurbine) > 100.0, "turbine with 40 m blades");
    assert!(tall(BuildingKind::PowerPlant) > 80.0, "power plant stack");
    assert!(tall(BuildingKind::PowerPylon) > 30.0);
    assert!(tall(BuildingKind::PumpStation) < 12.0);
    // A wider rotor makes a taller turbine.
    let small = BuildingParams { width: 40.0, ..BuildingParams::of(BuildingKind::WindTurbine) };
    assert!(small.build().bounds().unwrap().max.z < tall(BuildingKind::WindTurbine));
    // Fenced yards use the fence material; solar panels are glass.
    let ids_of = |k| BuildingParams::of(k).build().polygons.values().map(|p| p.material_id).collect::<std::collections::HashSet<u16>>();
    for k in [BuildingKind::Refinery, BuildingKind::Substation, BuildingKind::SolarFarm, BuildingKind::CellTower] {
        assert!(ids_of(k).contains(&ids::REBAR), "{k:?} is fenced");
    }
    assert!(ids_of(BuildingKind::SolarFarm).contains(&ids::GLASS), "solar panels");
}
