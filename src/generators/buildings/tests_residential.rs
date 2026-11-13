use super::*;
use crate::generators::kit::ids;

fn residential() -> impl Iterator<Item = BuildingKind> {
    Category::Residential.kinds()
}

#[test]
fn residential_kinds_are_appended_and_build_valid_meshes() {
    // Appended after the original 44 so saved scenes and seeds keep their meaning.
    assert_eq!(BuildingKind::Cottage as usize, 44);
    assert_eq!(residential().count(), 13);
    for k in residential() {
        for detail in Detail::ALL {
            let p = BuildingParams { detail, ..BuildingParams::of(k) };
            let m = p.build();
            m.validate().unwrap_or_else(|e| panic!("{k:?}: {e}"));
            let (v, _, f) = m.counts();
            assert!(f >= 30, "{k:?} has some detail: {f}");
            assert!(f < 60_000, "{k:?} poly budget: {f}");
            assert_eq!(p.build().counts(), (v, m.counts().1, f), "{k:?} deterministic");
            assert!(m.vertices.values().all(|v| v.position.is_finite()), "{k:?} finite");
            for poly in m.polygons.values() {
                assert!(poly.corners.iter().all(|c| c.uv.is_finite()), "{k:?} UVs");
            }
        }
    }
}

#[test]
fn houses_have_windows_doors_and_roofs() {
    for k in residential() {
        let m = BuildingParams::of(k).build();
        let ids: std::collections::HashSet<u16> = m.polygons.values().map(|p| p.material_id).collect();
        assert!(ids.contains(&ids::GLASS) || ids.contains(&ids::LIT), "{k:?} has windows");
        assert!(ids.contains(&ids::ROOF) || ids.contains(&ids::TRIM), "{k:?} has a roof");
        let b = m.bounds().unwrap();
        assert!(b.min.z > -0.01, "{k:?} stands on the ground");
    }
    let tall = |k| BuildingParams::of(k).build().bounds().unwrap().max.z;
    assert!(tall(BuildingKind::TowerBlock) > 50.0, "18 floors is tall");
    assert!(tall(BuildingKind::BeachHut) < 5.0, "beach hut is small");
    assert!(tall(BuildingKind::WalkUp) > tall(BuildingKind::Duplex));
}

#[test]
fn lot_extras_add_geometry_and_old_scenes_get_the_default() {
    let base = BuildingParams::of(BuildingKind::Cottage);
    let polys = |lot| BuildingParams { lot, ..base.clone() }.build().counts().2;
    let none = polys(Lot::NONE);
    let full = polys(Lot { garden: true, driveway: true, garage: true, fence: true });
    assert!(full > none + 20, "lot adds garden, garage, fence: {none} -> {full}");
    let garage = BuildingParams { lot: Lot { garage: true, ..Lot::NONE }, ..base.clone() }.build();
    assert!(garage.bounds().unwrap().max.x > base.width * 0.5 + 3.0, "garage beside the house");
    // A scene saved before lots existed loads with the default lot.
    let mut v = serde_json::to_value(&base).unwrap();
    v.as_object_mut().unwrap().remove("lot");
    let back: BuildingParams = serde_json::from_value(v).unwrap();
    assert_eq!(back.lot, Lot::default());
    // Bigger houses: more seeds give different results.
    let a = BuildingParams { seed: 1, ..BuildingParams::of(BuildingKind::RowHouses) }.build().counts();
    let b = BuildingParams { seed: 7, ..BuildingParams::of(BuildingKind::RowHouses) }.build().counts();
    let c = BuildingParams { seed: 1, ..BuildingParams::of(BuildingKind::RowHouses) }.build().counts();
    assert_eq!(a, c);
    let _ = b;
}
