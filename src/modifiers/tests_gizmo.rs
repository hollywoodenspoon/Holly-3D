//! Modifier gizmos: the Bend / Twist / Taper Gizmo and Center sub-objects
//! and the Symmetry Mirror plane (geometry and W/E/R drags).

use super::*;
use crate::editor::Editor;
use crate::mesh::EditablePoly;
use crate::primitives::{Primitive, PrimitiveKind};
use crate::scene::BaseObject;
use bevy::math::{Mat4, Vec3};

fn tall_box() -> EditablePoly {
    Primitive::Box { length: 0.4, width: 0.4, height: 2.0, length_segs: 1, width_segs: 1, height_segs: 8 }.to_editable_poly()
}

fn same(a: &EditablePoly, b: &EditablePoly, what: &str) {
    assert_eq!(a.counts(), b.counts(), "{what}");
    for (x, y) in a.vertices.values().zip(b.vertices.values()) {
        assert!(x.position.abs_diff_eq(y.position, 1e-3), "{what}: {} vs {}", x.position, y.position);
    }
}

/// Sorted positions, to compare meshes built in a different order.
fn points(m: &EditablePoly) -> Vec<[i32; 3]> {
    let mut v: Vec<[i32; 3]> = m.vertices.values().map(|v| (v.position * 1000.0).round().as_ivec3().to_array()).collect();
    v.sort();
    v
}

#[test]
fn deformer_gizmo_moves_the_deformation_frame() {
    let kinds = [
        ModifierKind::Bend(BendMod { angle: 80.0, direction: 30.0, center: Vec3::new(0.1, 0.0, 0.2), ..Default::default() }),
        ModifierKind::Twist(TwistMod { angle: 120.0, bias: 20.0, ..Default::default() }),
        ModifierKind::Taper(TaperMod { amount: -0.6, curve: 0.5, ..Default::default() }),
    ];
    let t = Vec3::new(0.3, -0.2, 0.5);
    for k in kinds {
        // Moving the gizmo by t is the same as moving its center by t ...
        let mut moved = k.clone();
        *moved.deformer_gizmo_mut().unwrap().0 = Mat4::from_translation(t);
        let mut shifted = k.clone();
        *shifted.deformer_gizmo_mut().unwrap().1 += t;
        let (mut a, mut b) = (tall_box(), tall_box());
        ModifierEntry::new(moved.clone()).apply(&mut a);
        ModifierEntry::new(shifted).apply(&mut b);
        same(&a, &b, k.default_name());
        assert!(a.validate().is_ok());
        // ... and the fitted box travels with it.
        let fitted = deformers::gizmo_box(&tall_box(), Mat4::from_translation(t)).unwrap();
        assert!(fitted.transform_point3(Vec3::ZERO).abs_diff_eq(tall_box().bounds().unwrap().center() + t, 1e-4));
    }
    // Twist commutes with turning its gizmo about the twist axis.
    let tw = TwistMod { angle: 90.0, ..Default::default() };
    let turned = TwistMod { gizmo: Mat4::from_rotation_z(0.8), ..tw.clone() };
    let (mut a, mut b) = (tall_box(), tall_box());
    tw.apply(&mut a);
    turned.apply(&mut b);
    same(&a, &b, "turned twist");
    // Tilting the Bend gizmo changes the result.
    let bend = BendMod { angle: 90.0, ..Default::default() };
    let tilted = BendMod { gizmo: Mat4::from_rotation_x(0.5), ..bend.clone() };
    let (mut a, mut b) = (tall_box(), tall_box());
    bend.apply(&mut a);
    tilted.apply(&mut b);
    assert!(a.vertices.values().zip(b.vertices.values()).any(|(x, y)| x.position.distance(y.position) > 0.05));
    // Moving the Twist center along its axis moves where the twist starts.
    let mut a = tall_box();
    let mut tw = TwistMod { angle: 90.0, ..Default::default() };
    tw.center = Vec3::new(0.0, 0.0, 1.0);
    tw.apply(&mut a);
    let mid = a.vertices.values().filter(|v| (v.position.z - 1.0).abs() < 1e-4).count();
    assert!(mid > 0);
    let unmoved = tall_box();
    for (x, y) in unmoved.vertices.values().zip(a.vertices.values()) {
        if (x.position.z - 1.0).abs() < 1e-4 {
            assert!(x.position.abs_diff_eq(y.position, 1e-4), "no twist at the center");
        }
    }
}

#[test]
fn symmetry_mirror_plane_can_move_and_rotate() {
    let base = Primitive::default_of(PrimitiveKind::Sphere).to_editable_poly();
    let mut moved = base.clone();
    SymmetryMod { mirror: Mat4::from_translation(Vec3::new(0.2, 0.0, 0.0)), weld: false, ..Default::default() }.apply(&mut moved);
    let mut offset = base.clone();
    SymmetryMod { offset: 0.2, weld: false, ..Default::default() }.apply(&mut offset);
    assert_eq!(points(&moved), points(&offset), "moving the Mirror = Mirror Offset");
    // X axis turned 90 degrees about Z mirrors across the XZ plane, like axis Y.
    let mut turned = base.clone();
    SymmetryMod { mirror: Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2), ..Default::default() }.apply(&mut turned);
    let mut y = base.clone();
    SymmetryMod { axis: 1, ..Default::default() }.apply(&mut y);
    assert_eq!(points(&turned), points(&y));
    assert!(turned.is_closed() && turned.validate().is_ok());
    // A tilted plane still welds into one closed mesh.
    let mut tilted = base;
    SymmetryMod { mirror: Mat4::from_rotation_y(0.4), ..Default::default() }.apply(&mut tilted);
    assert!(tilted.is_closed(), "seam welded on a tilted plane");
}

#[test]
fn gizmo_center_and_mirror_drags() {
    let mut ed = Editor::default();
    let id = ed.scene.add(BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Box)), Vec3::new(1.0, 0.0, 0.0));
    ed.scene.objects[id].evaluate();
    ed.scene.select_only(Some(id));
    ed.add_modifier("Twist");
    ed.set_mod_sub(Some(ModSub::Gizmo));
    assert_eq!(ed.active_mod_sub(), Some(ModSub::Gizmo));
    let frame = ed.mod_sub_frame().unwrap();
    assert!(frame.translation.abs_diff_eq(Vec3::new(1.0, 0.0, 0.0), 1e-4), "gizmo sits on the pivot");
    let before = ed.scene.objects[id].clone();
    let up = Mat4::from_translation(Vec3::Z * 0.5);
    ed.drag_mod_sub(id, &before, ModSub::Gizmo, up);
    let (g, c) = ed.scene.objects[id].active_modifier().unwrap().deformer_gizmo().unwrap();
    assert!(g.transform_point3(c).abs_diff_eq(Vec3::Z * 0.5, 1e-4));
    // Center moves inside the gizmo; the gizmo stays.
    ed.set_mod_sub(Some(ModSub::Center));
    let before = ed.scene.objects[id].clone();
    ed.drag_mod_sub(id, &before, ModSub::Center, Mat4::from_translation(Vec3::X * 0.25));
    let (g2, c2) = ed.scene.objects[id].active_modifier().unwrap().deformer_gizmo().unwrap();
    assert_eq!(g2, g);
    assert!(g2.transform_point3(c2).abs_diff_eq(Vec3::new(0.25, 0.0, 0.5), 1e-4));
    assert!(ed.mod_sub_frame().unwrap().translation.abs_diff_eq(Vec3::new(1.25, 0.0, 0.5), 1e-4));
    // Symmetry Mirror: rotate with E; the plane stays rigid (no scale).
    ed.add_modifier("Symmetry");
    ed.set_mod_sub(Some(ModSub::Mirror));
    assert_eq!(ed.active_mod_sub(), Some(ModSub::Mirror));
    let before = ed.scene.objects[id].clone();
    let spin = Mat4::from_rotation_z(std::f32::consts::FRAC_PI_2) * Mat4::from_scale(Vec3::splat(2.0));
    ed.drag_mod_sub(id, &before, ModSub::Mirror, spin);
    let Some(ModifierKind::Symmetry(s)) = ed.scene.objects[id].active_modifier() else { panic!("symmetry on top") };
    assert!(s.normal().abs_diff_eq(Vec3::Y, 1e-4), "{}", s.normal());
    assert!((s.mirror.determinant() - 1.0).abs() < 1e-4);
}

/// Edit Normals with `E`: the gizmo sits on the selected normals and a world
/// rotation turns them, also on a rotated object (world -> local axis).
#[test]
fn edit_normals_rotate_with_the_e_gizmo() {
    use crate::modifiers::edit_normals::NormalKind;
    use bevy::math::Quat;
    let mut ed = Editor::default();
    let id = ed.scene.add(BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Box)), Vec3::new(1.0, 0.0, 0.0));
    ed.scene.objects[id].transform.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    ed.scene.objects[id].evaluate();
    ed.scene.select_only(Some(id));
    ed.add_modifier("Edit Normals");
    ed.set_mod_sub(Some(ModSub::Normal));
    assert_eq!(ed.active_mod_sub(), Some(ModSub::Normal));
    let input = ed.scene.objects[id].level_mesh.clone();
    let top = input.polygons.keys().find(|&p| input.poly_normal(p).z > 0.9).unwrap();
    let keys: Vec<_> = (0..input.polygons[top].corners.len()).map(|i| (top, i)).collect();
    let Some(ModifierKind::EditNormals(e)) = ed.scene.objects[id].active_modifier_mut() else { panic!("Edit Normals on top") };
    e.selected = keys.iter().copied().collect();
    // E gizmo frame at the selected normals' base (the top face center).
    let c = ed.mod_sub_frame().expect("a gizmo for the selected normals").translation;
    let w = ed.scene.objects[id].world_matrix();
    assert!(c.abs_diff_eq(w.transform_point3(input.poly_centroid(top)), 1e-4), "{c}");
    // Rotate 90 degrees about world X around that point: world +Z -> -Y.
    let before = ed.scene.objects[id].clone();
    let rot = Mat4::from_translation(c) * Mat4::from_rotation_x(std::f32::consts::FRAC_PI_2) * Mat4::from_translation(-c);
    ed.drag_mod_sub(id, &before, ModSub::Normal, rot);
    let o = &ed.scene.objects[id];
    let Some(ModifierKind::EditNormals(e)) = o.active_modifier() else { panic!() };
    let cur = e.current(&o.level_mesh);
    let q = o.transform.rotation;
    for k in &keys {
        assert_eq!(cur[k].1, NormalKind::Explicit);
        assert!((q * cur[k].0).abs_diff_eq(Vec3::NEG_Y, 1e-4), "world normal {}", q * cur[k].0);
    }
    // The evaluated mesh carries the rotated normals; geometry is unchanged.
    let n = o.evaluated_mesh.corner_normals();
    assert!((q * n[&top][0]).abs_diff_eq(Vec3::NEG_Y, 1e-4));
    assert_eq!(o.evaluated_mesh.counts(), before.evaluated_mesh.counts());
}

#[test]
fn blast_zones_select_move_and_scale() {
    use crate::editor::select::SelectOp;
    let mut ed = Editor::default();
    let id = ed.scene.add(BaseObject::Primitive(Primitive::default_of(PrimitiveKind::Box)), Vec3::new(2.0, 0.0, 0.0));
    ed.scene.objects[id].evaluate();
    ed.scene.select_only(Some(id));
    ed.add_modifier("Battlefield / Ruin");
    ed.edit_active_modifier("Add Zone", |k, _| {
        if let ModifierKind::Battlefield(b) = k {
            b.add_zone(None);
            b.add_zone(None);
        }
    });
    ed.set_mod_sub(Some(ModSub::BlastZones));
    assert_eq!(ed.active_mod_sub(), Some(ModSub::BlastZones));
    let zones = |ed: &Editor| match ed.scene.objects[id].active_modifier() {
        Some(ModifierKind::Battlefield(b)) => b.zones.clone(),
        _ => panic!("battlefield on top"),
    };
    // Clicking the first zone's centre selects only it.
    let view = ed.viewports.active;
    let at = ed.viewports.views[view].to_screen(Vec3::new(2.0, 0.0, 0.0)).unwrap();
    assert!(ed.mod_click_select(view, at, SelectOp::Replace));
    let z = zones(&ed);
    assert!(z[0].selected && !z[1].selected);
    assert!(ed.mod_sub_frame().unwrap().translation.abs_diff_eq(Vec3::new(2.0, 0.0, 0.0), 1e-4));
    // W moves it, R scales its sphere.
    let r0 = z[0].radius;
    let before = ed.scene.objects[id].clone();
    let total = Mat4::from_translation(Vec3::new(2.0, 1.0, 0.0)) * Mat4::from_scale(Vec3::splat(2.0)) * Mat4::from_translation(Vec3::new(-2.0, 0.0, 0.0));
    ed.drag_mod_sub(id, &before, ModSub::BlastZones, total);
    let z = zones(&ed);
    assert!(z[0].center.abs_diff_eq(Vec3::Y, 1e-4), "{}", z[0].center);
    assert!((z[0].radius - r0 * 2.0).abs() < 1e-3);
    assert_eq!(z[1], zones(&ed)[1], "unselected zone stays");
}
