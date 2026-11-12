use super::kit::{mat, volume};
use super::leg::{self, ANKLE};
use super::{hull, railgun, spawn_railgun};
use crate::mesh::EditablePoly;
use crate::mesh::adjacency::Adjacency;
use crate::scene::{ObjId, Scene};
use bevy::math::{Quat, Vec3};
use std::collections::HashSet;

fn parts() -> Vec<(&'static str, EditablePoly)> {
    vec![
        ("hull", hull::hull()),
        ("drum", leg::hip_drum()),
        ("thigh", leg::thigh()),
        ("shin", leg::shin()),
        ("carriage", leg::carriage()),
        ("foot", leg::foot()),
        ("housing", leg::piston(false)),
        ("rod", leg::piston(true)),
        ("turret", railgun::turret()),
        ("railgun", railgun::railgun()),
    ]
}

#[test]
fn parts_are_closed_outward_shells_with_uvs_and_ids() {
    for (name, m) in parts() {
        m.validate().unwrap_or_else(|e| panic!("{name}: {e}"));
        assert!(m.vertices.len() >= 8 && m.polygons.len() >= 6, "{name}");
        assert!(!m.edges.is_empty(), "{name} has edges");
        let adj = Adjacency::build(&m);
        for e in m.edges.values() {
            assert_eq!(adj.faces_of_edge(e.v0, e.v1).len() % 2, 0, "{name}: open edge");
        }
        assert!(volume(&m) > 0.0, "{name}: normals face out ({})", volume(&m));
        for p in m.polygons.values() {
            assert!((1..=10).contains(&p.material_id), "{name}: Material ID {}", p.material_id);
            assert!(p.smoothing_group != 0 || p.corners.len() >= 3, "{name}");
            for c in &p.corners {
                assert!(c.uv.x.is_finite() && (-0.01..=1.01).contains(&c.uv.x) && (-0.01..=1.01).contains(&c.uv.y), "{name}: UV {:?}", c.uv);
            }
        }
    }
}

#[test]
fn material_ids_follow_the_spec() {
    let ids = |m: &EditablePoly| m.polygons.values().map(|p| p.material_id).collect::<HashSet<_>>();
    assert!(ids(&hull::hull()).is_superset(&[mat::HULL, mat::ARMOUR, mat::JOINT, mat::RADIATOR].into()));
    assert!(ids(&leg::foot()).contains(&mat::FOOT));
    assert_eq!(ids(&leg::piston(true)), [mat::ROD].into());
    assert!(ids(&railgun::turret()).is_superset(&[mat::GLASS, mat::RADIATOR, mat::ARMOUR].into()));
    assert!(ids(&railgun::railgun()).contains(&mat::WEAPON));
}

fn pos(s: &Scene, id: ObjId) -> Vec3 {
    s.objects[id].transform.translation
}

fn set(s: &mut Scene, id: ObjId, v: f32) {
    let i = s.assembly.joint_of(id).unwrap();
    s.assembly.joints[i].kind.set_value(v);
    for _ in 0..4 {
        s.solve_assembly();
    }
}

fn piston_len(s: &Scene, l: &super::LegIds) -> f32 {
    pos(s, l.housing).distance(pos(s, l.rod))
}

#[test]
fn railgun_spider_stands_on_its_claws() {
    let mut s = Scene::default();
    let rig = spawn_railgun(&mut s, Vec3::ZERO);
    assert_eq!(rig.parts().len(), 3 + 4 * 7);
    // 8 joints per leg (7 placing + the piston), turret and gun hinges.
    assert_eq!(s.assembly.joints.len(), 4 * 8 + 2);
    assert_eq!(s.assembly.links.len(), 4);
    assert!(!s.solve_assembly(), "the rest pose is settled");
    let mut low = f32::MAX;
    for l in &rig.legs {
        let ankle = pos(&s, l.foot);
        let want = super::HULL_Z + super::PLATE_H + leg::HIP.z + leg::KNEE.z + ANKLE.z;
        assert!((ankle.z - want).abs() < 1e-3 && (want - 0.55).abs() < 1e-3, "{ankle}");
        let m = &s.objects[l.foot].evaluated_mesh;
        let x = s.objects[l.foot].transform;
        for v in m.vertices.values() {
            low = low.min((x.translation + x.rotation * v.position).z);
        }
    }
    assert!(low.abs() < 0.06, "claws touch the ground, lowest point {low}");
    // Legs sit on the diagonals, one per quadrant.
    let quads: HashSet<(bool, bool)> = rig.legs.iter().map(|l| (pos(&s, l.foot).x > 0.0, pos(&s, l.foot).y > 0.0)).collect();
    assert_eq!(quads.len(), 4);
}

#[test]
fn knee_drives_the_carriage_and_the_piston_stays_in_its_stroke() {
    let mut s = Scene::default();
    let rig = spawn_railgun(&mut s, Vec3::ZERO);
    let l = rig.legs[0];
    let rest_len = piston_len(&s, &l);
    let car0 = s.joint_value(l.carriage).unwrap();
    for knee in [-40.0, -20.0, 0.0, 20.0, 40.0] {
        set(&mut s, l.shin, knee);
        let len = piston_len(&s, &l);
        assert!((0.7..1.05).contains(&len), "knee {knee}: piston {len}");
        let car = s.joint_value(l.carriage).unwrap();
        assert!((car - car0 - super::limits::CARRIAGE_RATIO * knee).abs() < 1e-3, "carriage follows the knee");
        // The housing and rod point at each other.
        let (h, r) = (&s.objects[l.housing].transform, &s.objects[l.rod].transform);
        let d = (r.translation - h.translation).normalize();
        assert!((h.rotation * Vec3::Z).dot(d) > 0.999 && (r.rotation * Vec3::Z).dot(-d) > 0.999);
    }
    assert!(rest_len > 0.7);
}

#[test]
fn turret_turns_and_the_gun_lifts_almost_straight_up() {
    let mut s = Scene::default();
    let rig = spawn_railgun(&mut s, Vec3::ZERO);
    let muzzle = |s: &Scene| {
        let x = &s.objects[rig.gun].transform;
        x.rotation * Vec3::NEG_Y
    };
    set(&mut s, rig.gun, 120.0);
    assert!((s.joint_value(rig.gun).unwrap() - 85.0).abs() < 1e-3, "clamped at the +85° stop");
    assert!(muzzle(&s).z > 0.99, "points almost straight up: {}", muzzle(&s));
    set(&mut s, rig.gun, 0.0);
    set(&mut s, rig.turret, 90.0);
    assert!(muzzle(&s).distance(Quat::from_rotation_z(90f32.to_radians()) * Vec3::NEG_Y) < 1e-3, "the gun turns with the turret");
}

#[test]
fn dragging_a_foot_bends_its_leg_and_the_hull_stays() {
    let mut s = Scene::default();
    let rig = spawn_railgun(&mut s, Vec3::ZERO);
    let foot = rig.legs[1].foot;
    let hull0 = pos(&s, rig.hull);
    let target = pos(&s, foot) + Vec3::new(0.0, 0.0, 0.8);
    s.objects[foot].transform.translation = target;
    for _ in 0..4 {
        s.solve_assembly();
    }
    assert!(pos(&s, foot).distance(target) < 0.05, "IK reached {}", pos(&s, foot));
    assert!(pos(&s, rig.hull).distance(hull0) < 1e-4, "Grounded hull");
    assert!(s.joint_value(rig.legs[1].thigh).unwrap().abs() > 1.0 || s.joint_value(rig.legs[1].shin).unwrap().abs() > 1.0);
}
