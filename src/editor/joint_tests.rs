//! J tool click flow in a Top view.

use super::*;
use crate::assembly::{JointKind, JointType};
use crate::primitives::Primitive;
use crate::scene::BaseObject;
use bevy::math::Quat;

fn editor() -> (Editor, ObjId, ObjId) {
    let mut ed = Editor::default();
    let b = Primitive::Box { length: 2.0, width: 2.0, height: 2.0, length_segs: 1, width_segs: 1, height_segs: 1 };
    let [a, p] = [-3.0, 3.0].map(|x| ed.scene.add(BaseObject::Primitive(b.clone()), Vec3::new(x, 0.0, 0.0)));
    (ed.viewports.maximized, ed.viewports.active) = (true, 0);
    ed.viewports.layout(crate::viewport::layout::ScreenRect { min: Vec2::ZERO, max: Vec2::new(800.0, 600.0) }, 0.0);
    (ed, a, p)
}

fn click(ed: &mut Editor, world: Vec3) {
    let s = ed.viewports.views[0].to_screen(world).unwrap();
    assert!(ed.tool_click(0, s));
}

#[test]
fn j_tool_mates_the_child_on_the_parent_and_links_them() {
    let (mut ed, a, p) = editor();
    ed.joint_type = JointType::Revolute;
    ed.toggle_joint_tool();
    // Top faces: A's top center onto B's top center (pick off-center, away from edges).
    click(&mut ed, Vec3::new(-3.0, 0.0, 2.0));
    assert!(matches!(ed.tool, Tool::Joint { first: Some(_) }) && ed.scene.objects[a].selected);
    click(&mut ed, Vec3::new(3.0, 0.0, 2.0));
    assert_eq!(ed.tool, Tool::None);
    assert_eq!(ed.scene.assembly.parent_of(a), Some(p));
    // A is flipped face to face onto B's top: its pivot (base) ends up at z = 4.
    let x = ed.scene.objects[a].transform;
    assert!(x.translation.distance(Vec3::new(3.0, 0.0, 4.0)) < 1e-3, "{}", x.translation);
    assert!((x.rotation * Vec3::Z).z < -0.999);
    // Moving the parent carries the hinged child; undo removes the joint.
    ed.scene.objects[p].transform.translation.y = 2.0;
    ed.scene.solve_assembly();
    assert!((ed.scene.objects[a].transform.translation.y - 2.0).abs() < 1e-3);
    ed.undo();
    assert!(ed.scene.assembly.joints.is_empty());
    assert!((ed.scene.objects[a].transform.translation.x + 3.0).abs() < 1e-3, "undo puts A back");
}

#[test]
fn joint_rollout_edits_and_breaks_joints() {
    let (mut ed, a, p) = editor();
    ed.toggle_joint_tool();
    click(&mut ed, Vec3::new(-3.0, 0.0, 2.0));
    click(&mut ed, Vec3::new(3.0, 0.0, 2.0));
    let z0 = ed.scene.objects[a].transform.rotation;
    ed.edit_joint("Joint Parameters", true, |k| {
        if let JointKind::Revolute { angle, .. } = k {
            *angle = 90.0;
        }
    });
    let turned = ed.scene.objects[a].transform.rotation;
    assert!((z0.inverse() * turned).angle_between(Quat::IDENTITY).to_degrees() > 89.0);
    ed.change_joint_type(JointType::Slider);
    assert!(matches!(ed.scene.assembly.joints[0].kind, JointKind::Slider { .. }));
    // Cannot joint the parent back onto its own child.
    let r = ed.assemble(
        crate::assembly::mount::Mount { obj: p, point: Vec3::ZERO, normal: Vec3::Z },
        crate::assembly::mount::Mount { obj: a, point: Vec3::ZERO, normal: Vec3::Z },
    );
    assert!(r.is_err() && ed.scene.assembly.joints.len() == 1);
    ed.break_joints();
    assert!(ed.scene.assembly.joints.is_empty());
}

#[test]
fn motion_links_and_contact_members_from_the_rollout() {
    let (mut ed, a, p) = editor();
    let b = Primitive::Box { length: 2.0, width: 2.0, height: 2.0, length_segs: 1, width_segs: 1, height_segs: 1 };
    let c = ed.scene.add(BaseObject::Primitive(b), Vec3::new(0.0, 6.0, 0.0));
    let m = |obj, x: f32, y: f32| crate::assembly::mount::Mount { obj, point: Vec3::new(x, y, 2.0), normal: Vec3::Z };
    ed.assemble(m(a, -3.0, 0.0), m(p, 2.5, 0.0)).unwrap();
    ed.assemble(m(c, 0.0, 6.0), m(p, 3.5, 0.0)).unwrap();
    ed.scene.select_only(Some(c));
    ed.set_driver(Some(a));
    let l = ed.scene.assembly.links[0].clone();
    assert_eq!((l.driver, l.driven), (a, c));
    assert!((l.ratio + 1.0).abs() < 1e-3, "equal parts: auto-ratio -1, got {}", l.ratio);
    ed.scene.select_only(Some(a));
    ed.edit_joint("Joint Parameters", true, |k| k.set_value(30.0));
    assert!((ed.scene.joint_value(c).unwrap() + 30.0).abs() < 1e-3, "the driven part follows");
    ed.set_driver(Some(c));
    assert_eq!(ed.scene.assembly.links.len(), 1, "a loop of links is refused");
    ed.set_contact_member(a, true);
    ed.toggle_contacts();
    assert!(ed.scene.assembly.contacts_on && ed.scene.assembly.contact_members == vec![a]);
    ed.undo();
    assert!(ed.scene.assembly.contact_members.is_empty());
    ed.undo();
    ed.undo();
    assert!(ed.scene.assembly.links.is_empty(), "undo removes the link");
}

#[test]
fn create_mech_piston_is_two_parts_and_a_piston_joint() {
    let (mut ed, ..) = editor();
    let n = ed.scene.objects.len();
    ed.tool = Tool::CreateMech(crate::mech::MechKind::Piston);
    click(&mut ed, Vec3::new(0.0, 4.0, 0.0));
    assert_eq!(ed.scene.objects.len(), n + 2);
    let j = ed.scene.assembly.joints.last().unwrap();
    assert!(matches!(j.kind, JointKind::Piston));
    assert!(ed.scene.objects[j.child].mech().is_some() && ed.scene.objects[j.parent].mech().is_some());
    ed.undo();
    assert_eq!(ed.scene.objects.len(), n);
    assert!(ed.scene.assembly.joints.is_empty());
}

#[test]
fn bound_hose_follows_its_end_objects() {
    use crate::mech::MechPart;
    let (mut ed, a, p) = editor();
    ed.create_mech(crate::mech::MechKind::Hose, Vec3::ZERO);
    let h = ed.current().unwrap();
    ed.tool = Tool::PickHoseEnd { top: true };
    click(&mut ed, Vec3::new(-3.0, 0.0, 1.0));
    assert_eq!(ed.tool, Tool::None);
    ed.bind_hose(false, Some(p));
    let hose = |ed: &Editor| match ed.scene.objects[h].mech() {
        Some(MechPart::Hose(x)) => x.clone(),
        _ => unreachable!(),
    };
    assert_eq!((hose(&ed).top, hose(&ed).bottom), (Some(a), Some(p)));
    assert!(hose(&ed).start.distance(Vec3::new(-3.0, 0.0, 0.0)) < 1e-5);
    // Moving the bottom object flexes the hose (and its mesh) live.
    ed.scene.objects[p].transform.translation = Vec3::new(3.0, 5.0, 0.0);
    assert!(ed.scene.update_hoses());
    assert!(hose(&ed).end.distance(Vec3::new(3.0, 5.0, 0.0)) < 1e-5);
    assert!(ed.scene.objects[h].evaluated_mesh.bounds().unwrap().max.y > 4.9);
    // Deleting an end object frees that end where it was.
    ed.scene.delete(p);
    ed.scene.update_hoses();
    assert_eq!(hose(&ed).bottom, None);
    assert!(hose(&ed).end.distance(Vec3::new(3.0, 5.0, 0.0)) < 1e-5);
    ed.bind_hose(true, None);
    assert_eq!(hose(&ed).top, None);
}
