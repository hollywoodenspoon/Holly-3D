use super::*;

fn furniture() -> impl Iterator<Item = BuildingKind> {
    Category::Furniture.kinds()
}

#[test]
fn furniture_kinds_are_appended_and_build_valid_meshes() {
    // Appended after Rural so saved scenes and seeds keep their meaning.
    assert_eq!(BuildingKind::DiningChair as usize, 139);
    assert_eq!(furniture().count(), 10);
    for k in furniture() {
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
fn furniture_kinds_stand_on_the_ground_inside_their_lot() {
    for k in furniture() {
        let p = BuildingParams::of(k);
        let b = p.build().bounds().unwrap();
        assert!(b.min.z > -0.01, "{k:?} stands on the ground: {b:?}");
        let reach = p.width.max(p.length) * 0.5 + 14.0;
        assert!(b.min.x > -reach && b.max.x < reach && b.min.y > -reach && b.max.y < reach, "{k:?} fits its lot: {b:?}");
    }
    let tall = |k| BuildingParams::of(k).build().bounds().unwrap().max.z;
    assert!((0.85..1.2).contains(&tall(BuildingKind::DiningChair)), "chair back height");
    assert!((0.74..0.95).contains(&tall(BuildingKind::DiningTable)), "table height");
    assert!(tall(BuildingKind::CoffeeTable) < 0.6, "coffee table is low");
    // Mid detail rounds the cushions, so it adds faces.
    let faces = |k, detail| BuildingParams { detail, ..BuildingParams::of(k) }.build().counts().2;
    assert!(faces(BuildingKind::Sofa, Detail::Mid) > faces(BuildingKind::Sofa, Detail::Standard));
}
