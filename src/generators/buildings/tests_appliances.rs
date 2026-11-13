use super::*;
use crate::generators::kit::ids;

fn appliances() -> impl Iterator<Item = BuildingKind> {
    Category::Appliances.kinds()
}

fn ids_of(k: BuildingKind) -> std::collections::HashSet<u16> {
    BuildingParams::of(k).build().polygons.values().map(|p| p.material_id).collect()
}

#[test]
fn appliance_kinds_are_appended_and_build_valid_meshes() {
    // Appended after Lighting & Decor so saved scenes keep their meaning.
    assert_eq!(BuildingKind::Fridge as usize, 171);
    assert_eq!(appliances().count(), 16);
    for k in appliances() {
        assert!(!k.has_floors(), "{k:?} has no floors slider");
        for detail in Detail::ALL {
            let p = BuildingParams { detail, ..BuildingParams::of(k) };
            let m = p.build();
            m.validate().unwrap_or_else(|e| panic!("{k:?}: {e}"));
            let (v, e, f) = m.counts();
            assert!(f >= 20, "{k:?} has some detail: {f}");
            assert!(f < 80_000, "{k:?} poly budget: {f}");
            assert_eq!(p.build().counts(), (v, e, f), "{k:?} deterministic");
            assert!(m.polygons.values().all(|p| p.corners.iter().all(|c| c.uv.is_finite())), "{k:?} UVs");
            let b = m.bounds().unwrap();
            assert!(b.min.z > -0.01, "{k:?} stands on the floor: {b:?}");
            let reach = p.width.max(p.length) * 0.5 + 1.0;
            assert!(b.min.x > -reach && b.max.x < reach && b.min.y > -reach && b.max.y < reach, "{k:?} fits its lot: {b:?}");
        }
    }
}

#[test]
fn appliances_have_real_heights_and_parts() {
    let tall = |k| BuildingParams::of(k).build().bounds().unwrap().max.z;
    assert!((1.7..2.0).contains(&tall(BuildingKind::Fridge)), "fridge height");
    assert!((0.85..1.1).contains(&tall(BuildingKind::Cooker)), "cooker with splashback");
    assert!(tall(BuildingKind::Microwave) < 0.4 && tall(BuildingKind::Laptop) < 0.35, "counter-top pieces are small");
    assert!(tall(BuildingKind::VendingMachine) > 1.7 && tall(BuildingKind::ArcadeCabinet) > 1.7);
    let ac = BuildingParams::of(BuildingKind::AirConditioner).build().bounds().unwrap();
    assert!(ac.max.z > 2.2, "indoor unit hangs high on the wall: {ac:?}");
    assert!(ids_of(BuildingKind::WashingMachine).contains(&ids::IRON), "door ring");
    assert!(ids_of(BuildingKind::VendingMachine).contains(&ids::GLASS), "glass frame");
    let lit = |k| ids_of(k).iter().any(|i| [ids::LIT, ids::NEON].contains(i));
    assert!(lit(BuildingKind::Television) && lit(BuildingKind::ArcadeCabinet) && lit(BuildingKind::PcSetup), "screens glow");
    let off = BuildingParams { emissive: false, ..BuildingParams::of(BuildingKind::Television) }.build();
    assert!(off.polygons.values().all(|p| ![ids::LIT, ids::NEON].contains(&p.material_id)), "screens switch off");
}

#[test]
fn room_shells_have_walls_openings_and_a_floor_at_zero() {
    assert_eq!(BuildingKind::RoomShell as usize, 187);
    assert_eq!(Category::Rooms.kinds().count(), 4);
    for k in Category::Rooms.kinds() {
        let p = BuildingParams::of(k);
        let m = p.build();
        m.validate().unwrap_or_else(|e| panic!("{k:?}: {e}"));
        let b = m.bounds().unwrap();
        assert!((2.5..2.8).contains(&b.max.z), "{k:?} wall height: {b:?}");
        assert!(b.min.z > -0.2, "{k:?} floor slab is thin: {b:?}");
        // The walls stand outside the lot, so its full area is usable.
        assert!(b.min.x < -p.width * 0.5 && b.max.y > p.length * 0.5, "{k:?} walls outside the lot");
        // The floor finish tops out at z = 0 inside the room.
        let top = m.polygons.values().filter(|poly| poly.corners.iter().all(|c| m.vertices[c.vertex_id].position.z.abs() < 1e-4));
        assert!(top.count() > 0, "{k:?} has a floor at z = 0");
        assert!(!k.has_floors());
    }
    assert!(ids_of(BuildingKind::RoomShell).contains(&ids::GLASS), "room window glass");
    assert!(ids_of(BuildingKind::BathroomShell).contains(&ids::SIDEWALK), "bathroom tiles");
    // Openings: the door gap leaves the room's front wall in pieces, so a
    // shell has many more faces than four solid walls would.
    assert!(BuildingParams::of(BuildingKind::RoomShell).build().counts().2 > 100);
}
