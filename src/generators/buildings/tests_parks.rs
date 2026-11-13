use super::*;
use crate::generators::kit::ids;

fn parks() -> impl Iterator<Item = BuildingKind> {
    Category::Parks.kinds()
}

#[test]
fn park_kinds_are_appended_and_build_valid_meshes() {
    // Appended after Services so saved scenes and seeds keep their meaning.
    assert_eq!(BuildingKind::Playground as usize, 105);
    assert_eq!(parks().count(), 10);
    for k in parks() {
        assert!(!k.has_floors(), "{k:?} has no floors slider");
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
fn park_kinds_stand_on_the_ground_inside_their_lot() {
    for k in parks() {
        let p = BuildingParams::of(k);
        let b = p.build().bounds().unwrap();
        assert!(b.min.z > -0.01, "{k:?} stands on the ground: {b:?}");
        let reach = p.width.max(p.length) * 0.5 + 14.0;
        assert!(b.min.x > -reach && b.max.x < reach && b.min.y > -reach && b.max.y < reach, "{k:?} fits its lot: {b:?}");
    }
    let tall = |k| BuildingParams::of(k).build().bounds().unwrap().max.z;
    assert!(tall(BuildingKind::Stadium) > 30.0, "stadium floodlights");
    assert!(tall(BuildingKind::SkatePark) < 6.0);
    let ids_of = |k| BuildingParams::of(k).build().polygons.values().map(|p| p.material_id).collect::<std::collections::HashSet<u16>>();
    for k in [BuildingKind::SwimmingPool, BuildingKind::ParkPond, BuildingKind::Plaza] {
        assert!(ids_of(k).contains(&ids::WATER), "{k:?} has water");
    }
    for k in [BuildingKind::SportsField, BuildingKind::SportsCourt, BuildingKind::Stadium] {
        assert!(ids_of(k).contains(&ids::LANE), "{k:?} has markings");
    }
}
