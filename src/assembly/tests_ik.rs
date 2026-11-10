use super::mount::{Mount, joint_at};
use super::*;
use crate::editor::Editor;
use crate::editor::mirror::MirrorAxis;
use crate::primitives::{Primitive, PrimitiveKind};
use crate::scene::{BaseObject, Scene};
use bevy::math::Vec3;

fn hinge(limits: Limits) -> JointKind {
    // Mount Z = world +X, so mount Y = world Y: the hinge turns in the XZ plane.
    JointKind::Revolute { axis: Axis::Y, angle: 0.0, limits }
}

const FREE: Limits = Limits { on: false, min: 0.0, max: 0.0 };

/// Root box at the origin, a two-link arm along +X: hinges at x = 1 and 3,
/// link pivots at x = 2 and 4.
fn arm(s: &mut Scene) -> (ObjId, ObjId, ObjId) {
    let b = BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Box));
    let root = s.add(b.clone(), Vec3::ZERO);
    let l1 = s.add(b.clone(), Vec3::new(2.0, 0.0, 0.0));
    let l2 = s.add(b, Vec3::new(4.0, 0.0, 0.0));
    let f = |s: &Scene, id| Frame::of(&s.objects[id].transform);
    for (c, p, x) in [(l1, root, 1.0), (l2, l1, 3.0)] {
        let m = Mount { obj: p, point: Vec3::new(x, 0.0, 0.0), normal: Vec3::X };
        s.add_joint(joint_at(hinge(FREE), (c, f(s, c)), (p, f(s, p)), &m)).unwrap();
    }
    (root, l1, l2)
}

fn pos(s: &Scene, id: ObjId) -> Vec3 {
    s.objects[id].transform.translation
}

#[test]
fn ik_drag_turns_the_hinge_to_reach() {
    let mut s = Scene::default();
    let (root, l1, _) = arm(&mut s);
    s.objects[l1].transform.translation = Vec3::new(1.0, 0.0, 1.0);
    s.solve_assembly();
    assert!(pos(&s, l1).distance(Vec3::new(1.0, 0.0, 1.0)) < 1e-3, "{}", pos(&s, l1));
    assert!(matches!(s.assembly.joints[0].kind, JointKind::Revolute { angle, .. } if (angle.abs() - 90.0).abs() < 0.1));
    assert_eq!(pos(&s, root), Vec3::ZERO, "the root never moves");
}

#[test]
fn ik_drag_bends_the_whole_chain_unless_grounded_or_off() {
    let mut s = Scene::default();
    let (_, l1, l2) = arm(&mut s);
    let target = Vec3::new(2.0, 0.0, 2.0);
    s.objects[l2].transform.translation = target;
    s.solve_assembly();
    assert!(pos(&s, l2).distance(target) < 0.02, "end reached: {}", pos(&s, l2));
    assert!((pos(&s, l1).distance(Vec3::new(1.0, 0.0, 0.0)) - 1.0).abs() < 1e-3, "link 1 stays on its hinge");
    assert!(pos(&s, l1).distance(Vec3::new(2.0, 0.0, 0.0)) > 0.1, "link 1 bent");
    assert!(!s.solve_assembly(), "settled");

    // Grounded link 1: only link 2's own hinge turns, toward the target.
    let mut s = Scene::default();
    let (_, l1, l2) = arm(&mut s);
    s.assembly.grounded.push(l1);
    s.objects[l2].transform.translation = target;
    s.solve_assembly();
    assert!(pos(&s, l1).distance(Vec3::new(2.0, 0.0, 0.0)) < 1e-4);
    let want = Vec3::new(3.0, 0.0, 0.0) + Vec3::new(-1.0, 0.0, 2.0).normalize();
    assert!(pos(&s, l2).distance(want) < 1e-3, "closest reach {}", pos(&s, l2));

    // IK Drag off: the move is projected away, as before.
    let mut s = Scene::default();
    let (_, _, l2) = arm(&mut s);
    s.assembly.no_ik = true;
    s.objects[l2].transform.translation = target;
    s.solve_assembly();
    assert!(pos(&s, l2).distance(Vec3::new(4.0, 0.0, 0.0)) < 1e-4);
}

#[test]
fn ik_respects_joint_stops() {
    let mut s = Scene::default();
    let (_, l1, _) = arm(&mut s);
    s.assembly.joints[0].kind = hinge(Limits { on: true, min: -30.0, max: 30.0 });
    s.objects[l1].transform.translation = Vec3::new(1.0, 0.0, 1.0);
    s.solve_assembly();
    let JointKind::Revolute { angle, .. } = s.assembly.joints[0].kind else { unreachable!() };
    assert!((angle.abs() - 30.0).abs() < 1e-3, "{angle}");
}

#[test]
fn mirror_mech_limb_copies_the_chain_with_mirrored_joints() {
    let mut ed = Editor::default();
    let (root, l1, l2) = arm(&mut ed.scene);
    ed.scene.assembly.joints[0].kind =
        JointKind::Revolute { axis: Axis::Y, angle: 20.0, limits: Limits { on: true, min: -10.0, max: 60.0 } };
    ed.scene.solve_assembly();
    let (p1, p2) = (pos(&ed.scene, l1), pos(&ed.scene, l2));
    ed.scene.select_only(Some(l1));
    ed.mirror_limb(MirrorAxis::X);
    assert_eq!(ed.scene.objects.len(), 5);
    let m1 = ed.current().unwrap();
    let m2 = ed.scene.assembly.descendants(m1)[0];
    let flip = |p: Vec3| Vec3::new(-p.x, p.y, p.z);
    assert!(pos(&ed.scene, m1).distance(flip(p1)) < 1e-4 && pos(&ed.scene, m2).distance(flip(p2)) < 1e-4);
    assert_eq!(ed.scene.assembly.parent_of(m1), Some(root), "hangs off the same root");
    assert!(ed.scene.objects[m1].world_matrix().determinant() < 0.0, "mirrored copy");
    let i = ed.scene.assembly.joint_of(m1).unwrap();
    let JointKind::Revolute { angle, limits, .. } = ed.scene.assembly.joints[i].kind else { unreachable!() };
    assert!((angle + 20.0).abs() < 1e-3 && limits.min == -60.0 && limits.max == 10.0, "{angle} {limits:?}");
    assert!(!ed.scene.solve_assembly(), "mirrored poses already satisfy the joints");
    // Turning both hinges by opposite angles keeps the limbs mirror images.
    for (j, a) in [(0, 45.0), (i, -45.0)] {
        if let JointKind::Revolute { angle, .. } = &mut ed.scene.assembly.joints[j].kind {
            *angle = a;
        }
    }
    ed.scene.solve_assembly();
    assert!(pos(&ed.scene, m2).distance(flip(pos(&ed.scene, l2))) < 1e-4);
    ed.undo();
    assert_eq!(ed.scene.objects.len(), 3);
}
