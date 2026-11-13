use super::*;
use crate::generators::kit::ids;

fn vehicles() -> impl Iterator<Item = BuildingKind> {
    Category::Vehicles.kinds()
}

fn design(k: BuildingKind, d: u32, detail: Detail) -> BuildingParams {
    BuildingParams { floors: d, detail, ..BuildingParams::of(k) }
}

#[test]
fn vehicle_kinds_are_appended_with_three_designs_each() {
    // Appended after Room Shells so saved scenes keep their meaning.
    assert_eq!(BuildingKind::Sedan as usize, 191);
    assert_eq!(vehicles().count(), 8);
    for k in vehicles() {
        assert!(!k.has_floors(), "{k:?} picks a design, not floors");
        assert_eq!(k.designs().len(), 3, "{k:?} designs");
        for d in 1..=3 {
            for detail in Detail::ALL {
                let p = design(k, d, detail);
                let m = p.build();
                m.validate().unwrap_or_else(|e| panic!("{k:?} {d}: {e}"));
                let (v, e, f) = m.counts();
                assert!((60..80_000).contains(&f), "{k:?} {d} poly budget: {f}");
                assert_eq!(p.build().counts(), (v, e, f), "{k:?} {d} deterministic");
                assert!(m.polygons.values().all(|p| p.corners.iter().all(|c| c.uv.is_finite())), "{k:?} UVs");
                let ids: std::collections::HashSet<u16> = m.polygons.values().map(|p| p.material_id).collect();
                assert!(ids.contains(&ids::GLASS) || ids.contains(&ids::LIT), "{k:?} {d} glass");
                assert!(ids.contains(&ids::ASPHALT), "{k:?} {d} tyres");
                let b = m.bounds().unwrap();
                assert!(b.min.z > -0.01, "{k:?} {d} on the road: {b:?}");
                assert!(b.max.y - b.min.y > b.max.x - b.min.x, "{k:?} {d} is longer than wide");
            }
        }
    }
}

#[test]
fn designs_differ_and_mid_adds_parts() {
    for k in vehicles() {
        let shapes: Vec<_> = (1..=3)
            .map(|d| {
                let m = design(k, d, Detail::Standard).build();
                let b = m.bounds().unwrap();
                (m.counts().2, ((b.max.z - b.min.z) * 100.0) as i32)
            })
            .collect();
        assert!(shapes[0] != shapes[1] && shapes[1] != shapes[2] && shapes[0] != shapes[2], "{k:?} designs: {shapes:?}");
        for d in 1..=3 {
            let (s, m) = (design(k, d, Detail::Standard).build().counts().2, design(k, d, Detail::Mid).build().counts().2);
            assert!(m > s, "{k:?} {d}: Mid {m} > Standard {s}");
        }
    }
}

#[test]
fn vehicles_have_real_heights() {
    let tall = |k, d| design(k, d, Detail::Standard).build().bounds().unwrap().max.z;
    let sedan = tall(BuildingKind::Sedan, 1);
    assert!((1.3..1.6).contains(&sedan), "sedan height {sedan}");
    assert!(tall(BuildingKind::Van, 1) > sedan + 0.4, "van taller");
    assert!(tall(BuildingKind::SportsCar, 1) < sedan, "sports car lower");
    assert!(tall(BuildingKind::PoliceCar, 1) > sedan, "light bar on the roof");
}
