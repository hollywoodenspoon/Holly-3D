use super::*;
use crate::generators::kit::ids;

fn services() -> impl Iterator<Item = BuildingKind> {
    Category::Services.kinds()
}

#[test]
fn service_kinds_are_appended_and_build_valid_meshes() {
    // Appended after Industrial so saved scenes and seeds keep their meaning.
    assert_eq!(BuildingKind::FireStation as usize, 88);
    assert_eq!(services().count(), 17);
    for k in services() {
        for detail in Detail::ALL {
            let p = BuildingParams { detail, ..BuildingParams::of(k) };
            let m = p.build();
            m.validate().unwrap_or_else(|e| panic!("{k:?}: {e}"));
            let (v, e, f) = m.counts();
            assert!(f >= 40, "{k:?} has some detail: {f}");
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
fn service_kinds_stand_on_the_ground_inside_their_lot() {
    for k in services() {
        let p = BuildingParams::of(k);
        let b = p.build().bounds().unwrap();
        // Only the metro stairs go below street level.
        let floor = if k == BuildingKind::MetroEntrance { -3.5 } else { -0.01 };
        assert!(b.min.z > floor, "{k:?} stands on the ground: {b:?}");
        let reach = p.width.max(p.length) * 0.5 + 14.0;
        assert!(b.min.x > -reach && b.max.x < reach && b.min.y > -reach && b.max.y < reach, "{k:?} fits its lot: {b:?}");
    }
    let metro = BuildingParams::of(BuildingKind::MetroEntrance).build().bounds().unwrap();
    assert!(metro.min.z < -2.0, "metro stairs go down: {metro:?}");
    let tall = |k| BuildingParams::of(k).build().bounds().unwrap().max.z;
    assert!(tall(BuildingKind::ControlTower) > 25.0, "control tower");
    assert!(tall(BuildingKind::BusStop) < 5.0, "bus stop is small");
    let ids_of = |k| BuildingParams::of(k).build().polygons.values().map(|p| p.material_id).collect::<std::collections::HashSet<u16>>();
    assert!(ids_of(BuildingKind::HarbourOffice).contains(&ids::WATER), "harbour water");
    assert!(ids_of(BuildingKind::TrainStation).contains(&ids::IRON), "rails");
}
