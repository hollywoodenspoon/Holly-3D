use super::mount::{MountAt, closed_loop, joint_at, mate, mount_point, piston_at};
use super::*;
use crate::primitives::{Primitive, PrimitiveKind};
use crate::scene::{BaseObject, Scene};
use bevy::math::{Quat, Vec3};

fn close(a: Vec3, b: Vec3) -> bool {
    a.distance(b) < 1e-3
}

/// Two boxes; the child is jointed at the parent's +X face (1, 0, 0).
fn rig(kind: JointKind) -> (Scene, ObjId, ObjId) {
    let mut s = Scene::default();
    let b = BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Box));
    let parent = s.add(b.clone(), Vec3::ZERO);
    let child = s.add(b, Vec3::new(2.0, 0.0, 0.0));
    let m = mount::Mount { obj: parent, point: Vec3::X, normal: Vec3::X };
    let (p, c) = (Frame::of(&s.objects[parent].transform), Frame::of(&s.objects[child].transform));
    s.add_joint(joint_at(kind, (child, c), (parent, p), &m)).unwrap();
    (s, parent, child)
}

fn pos(s: &Scene, id: ObjId) -> Vec3 {
    s.objects[id].transform.translation
}

#[test]
fn rigid_child_follows_the_parent() {
    let (mut s, parent, child) = rig(JointKind::Rigid);
    s.objects[parent].transform.translation = Vec3::new(0.0, 0.0, 5.0);
    s.objects[parent].transform.rotation = Quat::from_rotation_z(90f32.to_radians());
    assert!(s.solve_assembly());
    assert!(close(pos(&s, child), Vec3::new(0.0, 2.0, 5.0)), "{}", pos(&s, child));
    // A direct move of a welded part is undone.
    s.objects[child].transform.translation.z += 3.0;
    s.solve_assembly();
    assert!(close(pos(&s, child), Vec3::new(0.0, 2.0, 5.0)));
    assert!(!s.solve_assembly(), "settled");
}

#[test]
fn revolute_keeps_only_its_axis_and_clamps() {
    let limits = Limits { on: true, min: -45.0, max: 60.0 };
    let (mut s, _, child) = rig(JointKind::Revolute { axis: Axis::Z, angle: 0.0, limits });
    // Hinge axis = mount Z = world +X. Turning the child 30° about it is kept.
    let turn = |s: &mut Scene, q: Quat| {
        let x = &mut s.objects[child].transform;
        x.rotation = q * x.rotation;
        x.translation = Vec3::X + q * (x.translation - Vec3::X);
        s.solve_assembly();
    };
    turn(&mut s, Quat::from_rotation_x(30f32.to_radians()));
    let angle = |s: &Scene| match s.assembly.joints[0].kind {
        JointKind::Revolute { angle, .. } => angle,
        _ => unreachable!(),
    };
    assert!((angle(&s) - 30.0).abs() < 0.01, "{}", angle(&s));
    // A turn about another axis is projected away; the stop clamps at 60°.
    turn(&mut s, Quat::from_rotation_y(25f32.to_radians()));
    assert!((angle(&s) - 30.0).abs() < 0.5, "{}", angle(&s));
    assert!(close(pos(&s, child), Vec3::new(2.0, 0.0, 0.0)), "pinned on the hinge");
    turn(&mut s, Quat::from_rotation_x(50f32.to_radians()));
    assert!((angle(&s) - 60.0).abs() < 0.01);
    let (_, tw) = frame::swing_twist(s.objects[child].transform.rotation, Vec3::X);
    assert!((frame::twist_angle(tw, Vec3::X).to_degrees() - 60.0).abs() < 0.01);
}

#[test]
fn revolute_angle_unwraps_past_180() {
    let mut k = JointKind::Revolute { axis: Axis::Z, angle: 170.0, limits: Limits { on: false, min: 0.0, max: 0.0 } };
    solve::fit(&mut k, Frame::rot(Quat::from_rotation_z(190f32.to_radians())));
    assert!(matches!(k, JointKind::Revolute { angle, .. } if (angle - 190.0).abs() < 0.01));
}

#[test]
fn ball_swing_is_held_in_its_cone() {
    let mut k = JointKind::Ball { rot: Quat::IDENTITY, cone: 30.0, twist: 10.0 };
    solve::fit(&mut k, Frame::rot(Quat::from_rotation_x(80f32.to_radians()) * Quat::from_rotation_z(40f32.to_radians())));
    let JointKind::Ball { rot, .. } = k else { unreachable!() };
    let (swing, tw) = frame::swing_twist(rot, Vec3::Z);
    assert!((frame::angle_of(swing).to_degrees() - 30.0).abs() < 0.05);
    assert!((frame::twist_angle(tw, Vec3::Z).to_degrees() - 10.0).abs() < 0.05);
}

#[test]
fn slider_travels_along_its_axis_only() {
    let limits = Limits { on: true, min: 0.0, max: 1.5 };
    let (mut s, _, child) = rig(JointKind::Slider { axis: Axis::Z, travel: 0.0, limits });
    s.objects[child].transform.translation += Vec3::new(1.0, 0.7, 0.4);
    s.solve_assembly();
    assert!(close(pos(&s, child), Vec3::new(3.0, 0.0, 0.0)), "{}", pos(&s, child));
    s.objects[child].transform.translation.x += 5.0;
    s.solve_assembly();
    assert!(close(pos(&s, child), Vec3::new(3.5, 0.0, 0.0)), "clamped at max travel");
}

#[test]
fn chains_solve_parents_first_and_refuse_loops() {
    let (mut s, root, mid) = rig(JointKind::Rigid);
    let tip = s.add(BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Box)), Vec3::new(4.0, 0.0, 0.0));
    let m = mount::Mount { obj: mid, point: Vec3::new(3.0, 0.0, 0.0), normal: Vec3::X };
    let f = |s: &Scene, id| Frame::of(&s.objects[id].transform);
    let j = joint_at(JointKind::Rigid, (tip, f(&s, tip)), (mid, f(&s, mid)), &m);
    s.add_joint(j).unwrap();
    // Listed child-first on purpose: the solver still orders them.
    s.assembly.joints.reverse();
    s.objects[root].transform.translation.y = 1.0;
    s.solve_assembly();
    assert!(close(pos(&s, tip), Vec3::new(4.0, 1.0, 0.0)), "{}", pos(&s, tip));
    let back = joint_at(JointKind::Rigid, (root, f(&s, root)), (tip, f(&s, tip)), &m);
    assert!(s.add_joint(back).is_err(), "loop refused");
    assert_eq!(s.assembly.descendants(root), vec![mid, tip]);
    // Deleting the middle part drops both of its joints.
    s.objects.remove(mid);
    s.solve_assembly();
    assert!(s.assembly.joints.is_empty());
}

#[test]
fn piston_parts_aim_at_each_other() {
    let mut s = Scene::default();
    let b = BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Cylinder));
    let (h, r) = (s.add(b.clone(), Vec3::ZERO), s.add(b, Vec3::new(0.0, 0.0, 4.0)));
    let f = |s: &Scene, id| Frame::of(&s.objects[id].transform);
    let (ma, mb) = (
        mount::Mount { obj: h, point: Vec3::ZERO, normal: Vec3::Z },
        mount::Mount { obj: r, point: Vec3::new(0.0, 0.0, 4.0), normal: Vec3::Z },
    );
    s.add_joint(piston_at((h, f(&s, h)), (r, f(&s, r)), &ma, &mb)).unwrap();
    s.objects[r].transform.translation = Vec3::new(3.0, 0.0, 3.0);
    s.solve_assembly();
    let up = |s: &Scene, id: ObjId| s.objects[id].transform.rotation * Vec3::Z;
    let d = Vec3::new(3.0, 0.0, 3.0).normalize();
    assert!(close(up(&s, h), d), "housing aims at the rod: {}", up(&s, h));
    assert!(close(up(&s, r), d), "rod turned about its own mount");
    assert!(close(pos(&s, r), Vec3::new(3.0, 0.0, 3.0)) && close(pos(&s, h), Vec3::ZERO), "mounts stay put");
}

#[test]
fn mounts_on_faces_caps_and_edge_loops_then_mate() {
    let cyl = Primitive::Cylinder { radius: 1.0, height: 2.0, height_segs: 2, cap_segs: 1, sides: 12 };
    let m = cyl.to_editable_poly();
    let w = bevy::math::Mat4::from_translation(Vec3::new(5.0, 0.0, 0.0));
    let cap = m.polygons.keys().find(|&p| m.polygons[p].corners.len() == 12 && m.poly_normal(p).z > 0.9).unwrap();
    let (p, n) = mount_point(&m, w, cap, MountAt::Face, Vec3::Z);
    assert!(close(p, Vec3::new(5.0, 0.0, 2.0)) && close(n, Vec3::Z), "cap center {p} {n}");
    // A side edge on the middle ring: its loop is the ring around the axis.
    let mid: Vec<_> = m.vertices.keys().filter(|&v| (m.pos(v).z - 1.0).abs() < 1e-4).collect();
    let b = *mid.iter().skip(1).find(|&&v| m.edge_between(mid[0], v).is_some()).unwrap();
    assert_eq!(closed_loop(&m, mid[0], b).map(|l| l.len()), Some(12));
    let side = m.polygons.keys().find(|&p| m.polygons[p].corners.iter().any(|c| c.vertex_id == mid[0])).unwrap();
    let (p, n) = mount_point(&m, w, side, MountAt::Edge(mid[0], b), -Vec3::Z);
    assert!(close(p, Vec3::new(5.0, 0.0, 1.0)) && close(n, -Vec3::Z), "ring center {p} {n}");
    // Mating: A's top face lands on B's +X face, face to face.
    let a = mount::Mount { obj: ObjId::default(), point: Vec3::new(0.0, 0.0, 1.0), normal: Vec3::Z };
    let bm = mount::Mount { obj: ObjId::default(), point: Vec3::new(10.0, 0.0, 0.0), normal: Vec3::X };
    let f = mate(Frame::IDENTITY, &a, &bm);
    assert!(close(f.point(a.point), bm.point) && close(f.rot * a.normal, -bm.normal));
}

#[test]
fn assembly_saves_with_the_scene() {
    let limits = Limits { on: true, min: -10.0, max: 20.0 };
    let (s, _, child) = rig(JointKind::Revolute { axis: Axis::Y, angle: 5.0, limits });
    let text = ron::to_string(&s).unwrap();
    let back: Scene = ron::from_str(&text).unwrap();
    assert_eq!(back.assembly.joints.len(), 1);
    assert_eq!(back.assembly.joints[0].kind, s.assembly.joints[0].kind);
    assert_eq!(back.assembly.joint_of(child), Some(0));
    // Older scenes without joints still load.
    let old = text.replace(&format!("assembly:{},", ron::to_string(&s.assembly).unwrap()), "");
    assert!(ron::from_str::<Scene>(&old).is_ok_and(|o| o.assembly.joints.is_empty()), "{}", &old[..80.min(old.len())]);
}
