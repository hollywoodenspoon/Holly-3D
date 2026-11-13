use super::*;
use crate::generators::kit::ids;

fn commercial() -> impl Iterator<Item = BuildingKind> {
    Category::Commercial.kinds()
}

#[test]
fn commercial_kinds_are_appended_and_build_valid_meshes() {
    // Appended after Residential so saved scenes and seeds keep their meaning.
    assert_eq!(BuildingKind::CornerShop as usize, 57);
    assert_eq!(commercial().count(), 17);
    for k in commercial() {
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
fn shops_and_offices_have_glass_signs_and_stand_on_the_ground() {
    for k in commercial() {
        let p = BuildingParams::of(k);
        let m = p.build();
        let ids: std::collections::HashSet<u16> = m.polygons.values().map(|p| p.material_id).collect();
        assert!(ids.contains(&ids::GLASS) || ids.contains(&ids::LIT), "{k:?} has glazing");
        let b = m.bounds().unwrap();
        assert!(b.min.z > -0.01, "{k:?} stands on the ground");
        let reach = p.width.max(p.length) * 0.5 + 14.0;
        assert!(b.min.x > -reach && b.max.x < reach && b.min.y > -reach && b.max.y < reach, "{k:?} fits its lot: {b:?}");
    }
    let tall = |k| BuildingParams::of(k).build().bounds().unwrap().max.z;
    assert!(tall(BuildingKind::Hotel) > 40.0, "14-storey hotel is tall");
    assert!(tall(BuildingKind::MidRiseOffice) > tall(BuildingKind::LowRiseOffice));
    assert!(tall(BuildingKind::Cafe) < 8.0, "café is low");
    // Parking lots bring painted bays.
    let m = BuildingParams::of(BuildingKind::Supermarket).build();
    assert!(m.polygons.values().any(|p| p.material_id == ids::LANE), "car park stripes");
    // Signs off removes the glowing sign faces.
    let lit = |signs| {
        let mut p = BuildingParams::of(BuildingKind::Cinema);
        (p.parts.signs, p.emissive) = (signs, true);
        p.build().polygons.values().filter(|q| q.material_id == ids::NEON || q.material_id == ids::LIT).count()
    };
    assert!(lit(true) > lit(false), "cinema marquee glows");
}
