use super::*;
use crate::generators::kit::ids;

fn garden() -> impl Iterator<Item = BuildingKind> {
    Category::Garden.kinds()
}

#[test]
fn garden_kinds_are_appended_and_build_valid_meshes() {
    // Appended after Parks so saved scenes and seeds keep their meaning.
    assert_eq!(BuildingKind::Mailbox as usize, 115);
    assert_eq!(garden().count(), 16);
    for k in garden() {
        assert!(!k.has_floors(), "{k:?} has no floors slider");
        for detail in Detail::ALL {
            let p = BuildingParams { detail, ..BuildingParams::of(k) };
            let m = p.build();
            m.validate().unwrap_or_else(|e| panic!("{k:?}: {e}"));
            let (v, e, f) = m.counts();
            assert!(f >= 12, "{k:?} has some detail: {f}");
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
fn garden_kinds_stand_on_the_ground_inside_their_lot() {
    for k in garden() {
        let p = BuildingParams::of(k);
        let b = p.build().bounds().unwrap();
        assert!(b.min.z > -0.01, "{k:?} stands on the ground: {b:?}");
        let reach = p.width.max(p.length) * 0.5 + 14.0;
        assert!(b.min.x > -reach && b.max.x < reach && b.min.y > -reach && b.max.y < reach, "{k:?} fits its lot: {b:?}");
    }
    let tall = |k| BuildingParams::of(k).build().bounds().unwrap().max.z;
    assert!(tall(BuildingKind::GardenGnome) < 1.0, "gnome is small");
    assert!(tall(BuildingKind::GardenShed) > 2.0);
    let ids_of = |k| BuildingParams::of(k).build().polygons.values().map(|p| p.material_id).collect::<std::collections::HashSet<u16>>();
    assert!(ids_of(BuildingKind::BackyardPool).contains(&ids::WATER), "pool water");
    assert!(ids_of(BuildingKind::Hedge).contains(&ids::FOLIAGE), "hedge");
}
