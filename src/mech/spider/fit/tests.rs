use super::rig_selection;
use crate::mech::spider::kit::{Kit, beam, block, cyl, cyl_mid};
use crate::mech::spider::leg::{ANKLE, HIP, KNEE};
use crate::mesh::{Aabb, EditablePoly};
use crate::scene::{BaseObject, ObjId, Scene, Shared};
use bevy::math::{Quat, Vec3};

fn add(s: &mut Scene, name: &str, k: Kit) -> ObjId {
    let id = s.add(BaseObject::EditablePoly(Shared::new(k.finish())), Vec3::ZERO);
    s.objects[id].name = name.into();
    s.objects[id].evaluate();
    id
}

fn kit(pieces: Vec<EditablePoly>) -> Kit {
    let mut k = Kit::default();
    for p in pieces {
        k.add(p, 1);
    }
    k
}

/// A stand-in for an imported spider: plain parts posed in world space,
/// each leg with bolts at its pins and a fixed strut from shin to thigh.
fn imported_spider(s: &mut Scene) -> Vec<ObjId> {
    let mut ids = vec![add(s, "Box017", kit(vec![block(Vec3::new(4.0, 5.0, 0.8), Vec3::Z * 2.0)]))];
    for (sx, sy) in [(1.0, -1.0), (-1.0, -1.0), (1.0, 1.0), (-1.0, 1.0f32)] {
        let r = Quat::from_rotation_z(sy.atan2(sx));
        let pin = r * Vec3::Y;
        let drum = Vec3::new(1.8 * sx, 2.2 * sy, 2.2);
        let (hip, knee) = (drum + r * HIP, drum + r * (HIP + KNEE));
        let ankle = knee + r * ANKLE;
        let bolt = |at: Vec3| cyl_mid(0.08, 0.9, 12, at, pin);
        ids.push(add(s, "Tube", kit(vec![cyl(0.5, 0.5, 16, drum - Vec3::Z * 0.25, Vec3::Z), beam(drum, hip, pin, 0.5, 0.3, 0.5, 0.3), bolt(hip)])));
        ids.push(add(s, "Thigh", kit(vec![beam(hip, knee, pin, 0.4, 0.4, 0.36, 0.36), bolt(knee)])));
        let (d, up) = ((ankle - knee).normalize(), (hip - knee).normalize());
        let w = (up - d * up.dot(d)).normalize();
        // Strut: a bent brace in 5 pieces from the shin up under the thigh.
        let (a, b) = (knee + d * 1.8 + w * 0.1, knee + (hip - knee) * 0.45 - w * 0.1);
        let pts: Vec<Vec3> = (0..=5).map(|i| a.lerp(b, i as f32 / 5.0) + w * (i as f32 * (5 - i) as f32 * 0.03)).collect();
        let mut shin = vec![beam(knee - d * 0.3, ankle, pin, 0.4, 0.4, 0.3, 0.3), bolt(ankle)];
        shin.extend(pts.windows(2).map(|p| beam(p[0], p[1], pin, 0.12, 0.12, 0.12, 0.12)));
        ids.push(add(s, "Shin", kit(shin)));
        ids.push(add(s, "Foot", kit(vec![cyl(0.25, ankle.z + 0.05, 12, Vec3::new(ankle.x, ankle.y, 0.0), Vec3::Z)])));
    }
    ids.push(add(s, "Box025", kit(vec![block(Vec3::new(2.0, 2.0, 0.8), Vec3::Z * 2.8)])));
    ids.push(add(s, "Box026", kit(vec![beam(Vec3::new(0.0, 0.6, 3.0), Vec3::new(0.0, -3.0, 3.2), Vec3::X, 0.3, 0.3, 0.25, 0.25)])));
    ids.push(add(s, "Box024", kit(vec![block(Vec3::new(1.0, 0.6, 0.4), Vec3::new(0.0, 2.0, 2.6))])));
    ids
}

fn world_box(s: &Scene, id: ObjId) -> Aabb {
    let m = super::geom::world_mesh(s, id);
    Aabb::from_points(m.vertices.values().map(|v| v.position)).unwrap()
}

fn set(s: &mut Scene, id: ObjId, v: f32) {
    let i = s.assembly.joint_of(id).unwrap();
    s.assembly.joints[i].kind.set_value(v);
    for _ in 0..4 {
        s.solve_assembly();
    }
}

fn pos(s: &Scene, id: ObjId) -> Vec3 {
    s.objects[id].transform.translation
}

#[test]
fn rigs_an_imported_spider_in_place() {
    let mut s = Scene::default();
    let ids = imported_spider(&mut s);
    let before: Vec<Aabb> = ids.iter().map(|&i| world_box(&s, i)).collect();
    let f = rig_selection(&mut s, &ids).unwrap();
    let rig = &f.rig;
    assert_eq!(f.bolts, 12, "every hinge sits on its bolt");
    assert_eq!(s.assembly.joints.len(), 4 * 8 + 2 + 1, "legs, turret, gun and the welded rear block");
    assert_eq!(s.assembly.links.len(), 4);
    assert_eq!(s.objects[rig.gun].name, "Spider_Railgun");
    assert_eq!(s.objects[rig.turret].name, "Spider_RailgunTurret");
    assert!(s.assembly.grounded.contains(&rig.hull) && rig.hull == ids[0]);
    let fl = rig.legs[0];
    assert_eq!(s.objects[fl.foot].name, "Spider_FL_Foot");
    assert!(pos(&s, fl.foot).x > 0.0 && pos(&s, fl.foot).y < 0.0, "front = gun side (-Y), left = +X");
    for (k, &id) in ids.iter().enumerate() {
        let (a, b) = (before[k], world_box(&s, id));
        let shin = rig.legs.iter().any(|l| l.shin == id);
        if !shin {
            assert!((a.min - b.min).length() < 1e-3 && (a.max - b.max).length() < 1e-3, "{} moved", s.objects[id].name);
        }
    }
    for (l, &c) in rig.legs.iter().zip(&f.cut) {
        assert!(c >= 12, "most of the strut is cut ({c})");
        let (k, a) = (pos(&s, l.shin), pos(&s, l.foot));
        let off = |p: Vec3| (p - k).reject_from((a - k).normalize()).length();
        let far = super::geom::world_mesh(&s, l.shin).vertices.values().map(|v| off(v.position)).fold(0.0, f32::max);
        assert!(far < 0.5, "strut gone from the shin ({far})");
        assert!(s.objects[l.housing].evaluated_mesh.validate().is_ok());
    }
    assert!(!s.solve_assembly(), "rest pose is the imported pose");
    // Bending the knee slides the carriage and strokes the piston a little.
    let plen = |s: &Scene| pos(s, fl.housing).distance(pos(s, fl.rod));
    let (l0, c0) = (plen(&s), pos(&s, fl.carriage));
    set(&mut s, fl.shin, 25.0);
    assert!(pos(&s, fl.carriage).distance(c0) > 0.05, "carriage moved with the knee");
    let dl = (plen(&s) - l0).abs();
    assert!(dl > 1e-3 && dl < 0.3 * l0, "piston strokes but stays assembled ({dl} of {l0})");
    // Gun lift raises the muzzle; turret turn swings it round.
    let z0 = world_box(&s, rig.gun).max.z;
    set(&mut s, rig.gun, 30.0);
    assert!(world_box(&s, rig.gun).max.z > z0 + 0.5);
    set(&mut s, rig.turret, 90.0);
    assert!(world_box(&s, rig.gun).size().x > 2.0, "gun turned to the side");
}

#[test]
fn refuses_part_of_a_spider() {
    let mut s = Scene::default();
    let ids = imported_spider(&mut s);
    assert!(rig_selection(&mut s, &ids[..10]).is_err());
    let mut s2 = Scene::default();
    let mut ids2 = imported_spider(&mut s2);
    ids2.retain(|&i| s2.objects[i].name != "Foot");
    let e = rig_selection(&mut s2, &ids2).err();
    assert!(e.is_some(), "no feet, no legs");
}

/// Runs the fit on the owner's own OBJ (never committed) when `REF_OBJ`
/// points at it; `REF_BOX` = "minx,miny,maxx,maxy" picks one mech (Z-up).
#[test]
#[ignore]
fn fits_reference_obj() {
    let Ok(src) = std::env::var("REF_OBJ") else { return };
    let b: Vec<f32> = std::env::var("REF_BOX").unwrap().split(',').map(|v| v.parse().unwrap()).collect();
    let mut ed = crate::editor::Editor::default();
    let ids = ed.import_file(std::path::Path::new(&src)).unwrap();
    let mut s: Scene = std::mem::take(&mut ed.scene);
    let pick: Vec<_> = ids
        .into_iter()
        .filter(|&i| {
            let m = super::geom::world_mesh(&s, i);
            let c = crate::mesh::Aabb::from_points(m.vertices.values().map(|v| v.position)).unwrap().center();
            c.x > b[0] && c.y > b[1] && c.x < b[2] && c.y < b[3]
        })
        .collect();
    s.objects.retain(|k, _| pick.contains(&k));
    if std::env::var("REF_DEBUG").is_ok() {
        let ms: Vec<_> = pick.iter().map(|&i| super::geom::world_mesh(&s, i)).collect();
        let bb = ms.iter().filter_map(|m| crate::mesh::Aabb::from_points(m.vertices.values().map(|v| v.position))).reduce(|a, b| a.union(&b)).unwrap();
        let tol = bb.size().length() * 0.01;
        let parts: Vec<_> = ms.into_iter().map(|m| super::classify::Part::new(m, tol)).collect();
        let adj = super::classify::contact_graph(&parts, tol);
        for (k, &i) in pick.iter().enumerate() {
            println!("{k} {} {:?}", s.objects[i].name, adj[k]);
        }
    }
    if std::env::var("REF_SAVE").is_ok() {
        // Centre the mech on the origin, standing on the ground.
        let bb = pick.iter().map(|&i| world_box(&s, i)).reduce(|a, b| a.union(&b)).unwrap();
        let shift = Vec3::new(bb.center().x, bb.center().y, bb.min.z);
        for &i in &pick {
            s.objects[i].transform.translation -= shift;
        }
    }
    let t = std::time::Instant::now();
    let f = rig_selection(&mut s, &pick).unwrap();
    if let Ok(out) = std::env::var("REF_SAVE") {
        crate::io::scene_file::save(&s, std::path::Path::new(&out)).unwrap();
        let back = crate::io::scene_file::load(std::path::Path::new(&out)).unwrap();
        assert_eq!(back.assembly.joints.len(), s.assembly.joints.len());
        assert_eq!(back.assembly.links.len(), 4);
        println!("saved {out}: {} objects, {} joints", back.objects.len(), back.assembly.joints.len());
    }
    println!("{} in {:?}; cut {:?}", f.summary(), t.elapsed(), f.cut);
    for (i, l) in f.rig.legs.iter().enumerate() {
        println!("leg {i}: {:?}", [l.drum, l.thigh, l.shin, l.foot].map(|id| s.objects[id].transform.translation));
    }
    println!("turret {} gun {}", s.objects[f.rig.turret].transform.translation, s.objects[f.rig.gun].transform.translation);
    println!("links {:?}", s.assembly.links.iter().map(|l| l.ratio).collect::<Vec<_>>());
    // Check renders (SPIDER_SHOTS=dir): rest, crouched, gun raised and turned.
    let bb = f.rig.parts().iter().map(|&i| world_box(&s, i)).reduce(|a, b| a.union(&b)).unwrap();
    let ground = s.add(BaseObject::EditablePoly(Shared::new(block(Vec3::new(60.0, 60.0, 0.1), Vec3::ZERO))), bb.center().with_z(bb.min.z - 0.05));
    s.objects[ground].wire_color = [0.42, 0.43, 0.45];
    s.objects[ground].evaluate();
    for l in &f.rig.legs {
        s.objects[l.carriage].wire_color = [0.9, 0.55, 0.15];
        s.objects[l.housing].wire_color = [0.9, 0.55, 0.15];
        s.objects[l.rod].wire_color = [0.85, 0.85, 0.9];
    }
    let (c, d) = (bb.center(), bb.size().length() * 1.1);
    let shot = crate::mech::spider::tests_render::shot;
    shot(&s, "fit_01_rest", c, d, 60.0, -20.0);
    shot(&s, "fit_02_side", c, d * 0.9, -90.0, -5.0);
    let fl = f.rig.legs[0];
    let k = s.objects[fl.shin].transform.translation;
    shot(&s, "fit_03_knee", k - Vec3::Z * 1.5, d * 0.35, -60.0, -10.0);
    let plen = |s: &Scene| pos(s, fl.housing).distance(pos(s, fl.rod));
    let l0 = plen(&s);
    for l in f.rig.legs.clone() {
        set(&mut s, l.thigh, 20.0);
        set(&mut s, l.shin, -25.0);
    }
    println!("piston FL: rest {l0}, crouched {}", plen(&s));
    shot(&s, "fit_04_crouch", c, d, 60.0, -20.0);
    shot(&s, "fit_05_crouch_knee", s.objects[fl.shin].transform.translation - Vec3::Z * 1.5, d * 0.35, -60.0, -10.0);
    set(&mut s, f.rig.gun, 35.0);
    set(&mut s, f.rig.turret, 40.0);
    shot(&s, "fit_06_gun", c, d, 120.0, -15.0);
}
