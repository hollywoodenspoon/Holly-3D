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
