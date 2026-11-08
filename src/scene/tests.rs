use super::*;
use crate::mesh::ops::{ExtrudeMode, PolyOp};
use crate::mesh::SubObject;
use crate::modifiers::make_modifier;
use crate::primitives::{Primitive, PrimitiveKind};

fn box_scene() -> (Scene, ObjId) {
    let mut s = Scene::default();
    let id = s.add(BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Box)), Vec3::ZERO);
    (s, id)
}

fn select_top(o: &mut SceneObject) {
    let m = &mut o.level_mesh;
    let top: Vec<_> = m.polygons.keys().filter(|&p| m.poly_normal(p).z > 0.9).collect();
    m.clear_selection();
    m.polygons[top[0]].selected = true;
    o.commit_selection();
}

#[test]
fn names_are_max_style() {
    let (mut s, id) = box_scene();
    assert_eq!(s.objects[id].name, "Box001");
    let b = s.add(BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Box)), Vec3::X);
    assert_eq!(s.objects[b].name, "Box002");
}

#[test]
fn primitive_needs_conversion_for_ops() {
    let (mut s, id) = box_scene();
    let o = &mut s.objects[id];
    assert!(!o.sub_object_editable());
    o.collapse_to_editable_poly();
    assert!(o.sub_object_editable());
    select_top(o);
    assert!(o.apply_op(PolyOp::ExtrudePolygons { mode: ExtrudeMode::Group, height: 0.5 }));
    assert_eq!(o.evaluated_mesh.counts(), (12, 20, 10));
}

#[test]
fn edit_poly_modifier_on_live_primitive() {
    let (mut s, id) = box_scene();
    let o = &mut s.objects[id];
    o.add_modifier(make_modifier("Edit Poly").unwrap());
    assert!(o.sub_object_editable());
    select_top(o);
    assert!(o.apply_op(PolyOp::ExtrudePolygons { mode: ExtrudeMode::Group, height: 0.5 }));
    assert_eq!(o.evaluated_mesh.counts(), (12, 20, 10));
    // changing the primitive below re-runs the recorded op
    if let Some(Primitive::Box { height, .. }) = o.primitive_mut() {
        *height = 2.0;
    }
    o.evaluate();
    assert_eq!(o.evaluated_mesh.counts(), (12, 20, 10));
    let maxz = o.evaluated_mesh.vertices.values().map(|v| v.position.z).fold(0.0, f32::max);
    assert!((maxz - 2.5).abs() < 1e-4);
    // selection stays at the modifier level
    assert_eq!(o.level_mesh.selection_count(SubObject::Polygon), 1);
    o.delete_active_modifier();
    assert_eq!(o.evaluated_mesh.counts(), (8, 12, 6));
}

#[test]
fn instances_and_references_follow_edits() {
    let (mut s, id) = box_scene();
    let inst = s.clone_object(id, CloneKind::Instance, Xform::default()).unwrap();
    let refr = s.clone_object(id, CloneKind::Reference, Xform::default()).unwrap();
    let copy = s.clone_object(id, CloneKind::Copy, Xform::default()).unwrap();
    s.objects[refr].add_modifier(make_modifier("Edit Poly").unwrap());
    let o = &mut s.objects[id];
    if let Some(Primitive::Box { width, .. }) = o.primitive_mut() {
        *width = 3.0;
    }
    o.evaluate();
    s.propagate_from(id);
    let w = |s: &Scene, k: ObjId| s.objects[k].evaluated_mesh.bounds().unwrap().size().x;
    assert!((w(&s, inst) - 3.0).abs() < 1e-5);
    assert!((w(&s, refr) - 3.0).abs() < 1e-5);
    assert!((w(&s, copy) - 1.0).abs() < 1e-5);
    assert_eq!(s.objects[refr].modifiers.len(), 1, "reference keeps its own modifier");
}

#[test]
fn pivot_tools_keep_world_geometry() {
    let (mut s, id) = box_scene();
    let o = &mut s.objects[id];
    o.transform.translation = Vec3::new(2.0, 0.0, 0.0);
    o.transform.rotation = Quat::from_rotation_z(0.7);
    let probe = |o: &SceneObject| {
        let v = o.evaluated_mesh.vertices.values().next().unwrap().position;
        o.world_matrix().transform_point3(v)
    };
    let before = probe(o);
    o.center_pivot();
    assert!((probe(o) - before).length() < 1e-4);
    assert!((o.transform.translation - Vec3::new(2.0, 0.0, 0.5)).length() < 1e-4);
    o.align_pivot_to_world();
    o.reset_xform();
    o.bottom_center_pivot();
    assert!((probe(o) - before).length() < 1e-4);
}
