use super::mount::{Mount, joint_at};
use super::*;
use crate::primitives::{Primitive, PrimitiveKind};
use crate::scene::{BaseObject, Scene};
use bevy::math::Vec3;

const FREE: Limits = Limits { on: false, min: 0.0, max: 0.0 };

fn cyl(r: f32) -> BaseObject {
    BaseObject::Primitive(Primitive::Cylinder { radius: r, height: 0.5, height_segs: 1, cap_segs: 1, sides: 24 })
}

fn bx() -> BaseObject {
    BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Box))
}

/// Joins `child` to `parent` at `at`, the mount's Z along world `normal`.
fn join(s: &mut Scene, child: ObjId, parent: ObjId, at: Vec3, normal: Vec3, kind: JointKind) {
    let f = |s: &Scene, id| Frame::of(&s.objects[id].transform);
    let m = Mount { obj: parent, point: at, normal };
    s.add_joint(joint_at(kind, (child, f(s, child)), (parent, f(s, parent)), &m)).unwrap();
}

fn hinge() -> JointKind {
    JointKind::Revolute { axis: Axis::Z, angle: 0.0, limits: FREE }
}

fn slider() -> JointKind {
    JointKind::Slider { axis: Axis::Z, travel: 0.0, limits: FREE }
}

fn set(s: &mut Scene, id: ObjId, v: f32) {
    let i = s.assembly.joint_of(id).unwrap();
    s.assembly.joints[i].kind.set_value(v);
}

#[test]
fn gears_and_racks_follow_their_driver() {
    let mut s = Scene::default();
    let base = s.add(bx(), Vec3::new(0.0, 0.0, -5.0));
    let a = s.add(cyl(1.0), Vec3::ZERO);
    let b = s.add(cyl(2.0), Vec3::new(3.0, 0.0, 0.0));
    let rack = s.add(bx(), Vec3::new(0.0, -1.5, 0.0));
    join(&mut s, a, base, Vec3::ZERO, Vec3::Z, hinge());
    join(&mut s, b, base, Vec3::new(3.0, 0.0, 0.0), Vec3::Z, hinge());
    join(&mut s, rack, base, Vec3::new(0.0, -1.5, 0.0), Vec3::X, slider());
    assert!((s.joint_radius(a).unwrap() - 1.0).abs() < 1e-3);
    let ratio = s.auto_ratio(a, b).unwrap();
    assert!((ratio + 0.5).abs() < 1e-3, "meshing gears: {ratio}");
    s.link_motion(a, b, ratio).unwrap();
    let pinion = s.auto_ratio(a, rack).unwrap();
    assert!((pinion - std::f32::consts::PI / 180.0).abs() < 1e-5);
    s.link_motion(a, rack, pinion).unwrap();
    assert!(s.link_motion(b, a, 1.0).is_err(), "loop of links refused");
    assert!(s.link_motion(a, base, 1.0).is_err(), "the base has no hinge");

    set(&mut s, a, 40.0);
    s.solve_assembly();
    assert!((s.joint_value(b).unwrap() + 20.0).abs() < 1e-3);
    let turn = super::frame::twist_angle(s.objects[b].transform.rotation, Vec3::Z).to_degrees();
    assert!((turn + 20.0).abs() < 1e-2, "the driven gear turned: {turn}");
    let travel = 40f32.to_radians();
    assert!((s.objects[rack].transform.translation.x - travel).abs() < 1e-3, "rack moved by the arc length");
    // A new ratio keeps the driven part where it is.
    s.set_link_ratio(b, -1.0);
    s.solve_assembly();
    assert!((s.joint_value(b).unwrap() + 20.0).abs() < 1e-3);
    set(&mut s, a, 50.0);
    s.solve_assembly();
    assert!((s.joint_value(b).unwrap() + 30.0).abs() < 1e-3);
    // Deleting the driver drops its links.
    s.objects.remove(a);
    s.solve_assembly();
    assert!(s.assembly.links.is_empty());
}

/// Base far below; parts A at x = 0 and B at x = 3 on X sliders.
fn rail(kind_b: JointKind) -> (Scene, ObjId, ObjId) {
    let mut s = Scene::default();
    let base = s.add(bx(), Vec3::new(0.0, 0.0, -5.0));
    let a = s.add(bx(), Vec3::ZERO);
    let b = s.add(bx(), Vec3::new(3.0, 0.0, 0.0));
    join(&mut s, a, base, Vec3::ZERO, Vec3::X, slider());
    join(&mut s, b, base, Vec3::new(3.0, 0.0, 0.0), Vec3::X, kind_b);
    s.assembly.contact_members = vec![a, b];
    s.assembly.contacts_on = true;
    s.objects[a].selected = true;
    (s, a, b)
}

fn x(s: &Scene, id: ObjId) -> f32 {
    s.objects[id].transform.translation.x
}

#[test]
fn contact_members_push_along_their_slider() {
    let (mut s, a, b) = rail(slider());
    s.objects[a].transform.translation.x = 2.5;
    s.solve_assembly();
    assert!((x(&s, a) - 2.5).abs() < 1e-4, "the dragged part keeps its place");
    assert!((x(&s, b) - 3.5).abs() < 0.01, "pushed until touching: {}", x(&s, b));
    assert!(!contact::overlap(crate::editor::align::world_bounds(&s.objects[a]), crate::editor::align::world_bounds(&s.objects[b])));
    // Contact Sets off: parts pass through each other.
    let (mut s, a, b) = rail(slider());
    s.assembly.contacts_on = false;
    s.objects[a].transform.translation.x = 2.5;
    s.solve_assembly();
    assert!((x(&s, b) - 3.0).abs() < 1e-4);
}

#[test]
fn a_part_that_cannot_give_way_blocks() {
    let (mut s, a, b) = rail(JointKind::Rigid);
    s.objects[a].transform.translation.x = 2.5;
    s.solve_assembly();
    assert!((x(&s, b) - 3.0).abs() < 1e-4, "welded part stays");
    assert!((x(&s, a) - 2.0).abs() < 0.01, "the dragged part stops at it: {}", x(&s, a));
}
