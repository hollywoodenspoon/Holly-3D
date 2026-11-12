use super::paint::PaintStroke;
use super::*;
use crate::materials::Preset;
use crate::modifiers::{ModifierKind, make_modifier};
use crate::paint::{Alpha, BlendMode, LayerKind, PaintMode, SmartKind, VertexStroke, uv_overlap};

fn box_editor() -> (Editor, ObjId) {
    let mut ed = Editor::default();
    ed.create_primitive(PrimitiveKind::Box, Vec3::ZERO);
    ed.paint.canvas_size = 128;
    ed.paint.bake.size = 64;
    ed.paint.bake.ao_rays = 4;
    let id = ed.current().unwrap();
    (ed, id)
}

/// Top-centre of the box in world space.
fn top(ed: &Editor, id: ObjId) -> Vec3 {
    let o = &ed.scene.objects[id];
    let b = o.evaluated_mesh.bounds().unwrap();
    o.world_matrix().transform_point3(Vec3::new(b.center().x, b.center().y, b.max.z))
}

/// One texture-paint click on top of the box, seen from above.
fn click(ed: &mut Editor, id: ObjId) {
    let hit = top(ed, id);
    ed.paint.mode = PaintMode::Texture;
    ed.checkpoint("Texture Paint");
    assert!(ed.begin_layer_stroke(id, hit, Vec3::NEG_Z));
    ed.stamp_at(hit, Vec3::Z, None);
    ed.paint_end();
}

fn painted_tiles(ed: &Editor) -> usize {
    ed.paint_stack().map_or(0, |s| s.layers.iter().map(|l| if let LayerKind::Paint(t) = &l.kind { t.allocated() } else { 0 }).sum())
}

#[test]
fn vertex_stroke_adds_modifier_and_undoes_in_one_step() {
    let (mut ed, id) = box_editor();
    let top = top(&ed, id);
    ed.paint.mode = PaintMode::Vertex;
    ed.checkpoint("Vertex Paint");
    let i = ed.vertex_paint_modifier(true).unwrap();
    let o = &ed.scene.objects[id];
    let stroke = VertexStroke::new(&o.input_of(i), o.world_matrix(), &Default::default(), false);
    ed.paint_stroke = Some(PaintStroke::Vertex { obj: id, modifier: i, stroke, last: top });
    ed.paint.brush.size = 2.0;
    ed.stamp_at(top, Vec3::Z, None);
    ed.paint_end();
    let o = &ed.scene.objects[id];
    assert_eq!(o.modifiers.len(), 1);
    assert!(o.evaluated_mesh.vertices.values().any(|v| v.color.x > v.color.y));
    ed.undo();
    assert!(ed.scene.objects[id].modifiers.is_empty());
}

#[test]
fn the_first_stroke_keeps_the_material_as_layer_0_and_paints_a_layer_above() {
    let (mut ed, id) = box_editor();
    // The box wears a library material.
    let steel = ed.scene.materials.mats.insert(Preset::BrushedSteel.material());
    ed.scene.objects[id].material = Some(steel);
    let base = ed.scene.materials.mats[steel].clone();
    click(&mut ed, id);
    // A primitive's stacked UVs are no layout: it got a Smart UV Project.
    let o = &ed.scene.objects[id];
    assert!(matches!(o.modifiers.last().map(|m| &m.kind), Some(ModifierKind::Unwrap(_))));
    assert!(uv_overlap(&o.evaluated_mesh) < 0.02);
    // Same material, untouched, now with a paint stack on top.
    assert_eq!(o.material, Some(steel), "an unshared material is painted in place");
    let m = &ed.scene.materials.mats[steel];
    assert_eq!((m.metallic, m.roughness, m.base_color), (base.metallic, base.roughness, base.base_color));
    let stack = m.paint.as_ref().unwrap();
    assert_eq!((stack.size, stack.layers.len(), stack.layers[0].name.as_str()), (128, 1, "Paint Layer"));
    assert!(painted_tiles(&ed) > 0, "the stroke landed on the layer");
    // One Ctrl+Z removes the stroke, the layers and the unwrap together.
    ed.undo();
    assert!(ed.scene.objects[id].modifiers.is_empty());
    assert_eq!(ed.scene.materials.mats[steel], base);
}

#[test]
fn existing_uvs_are_never_overwritten() {
    let (mut ed, id) = box_editor();
    // A UVW Map the user set up: overlapping or not, it is theirs.
    ed.scene.objects[id].add_modifier(make_modifier("UVW Map").unwrap());
    let uvs = |ed: &Editor| ed.scene.objects[id].evaluated_mesh.polygons.values().flat_map(|p| p.corners.iter().map(|c| c.uv)).collect::<Vec<_>>();
    let (before, mods) = (uvs(&ed), ed.scene.objects[id].modifiers.len());
    ed.auto_setup_canvas();
    assert_eq!(ed.scene.objects[id].modifiers.len(), mods, "no Unwrap UVW was added");
    assert_eq!(uvs(&ed), before, "UVs kept exactly");
    assert!(ed.paint_stack().is_some() && ed.paint.mode == PaintMode::Texture);
    // A collapsed / imported mesh with UVs is kept as well.
    let (mut ed, id) = box_editor();
    ed.convert_to_editable_poly();
    let before = ed.scene.objects[id].evaluated_mesh.polygons.values().flat_map(|p| p.corners.iter().map(|c| c.uv)).collect::<Vec<_>>();
    ed.auto_setup_canvas();
    assert!(ed.scene.objects[id].modifiers.is_empty());
    let after = ed.scene.objects[id].evaluated_mesh.polygons.values().flat_map(|p| p.corners.iter().map(|c| c.uv)).collect::<Vec<_>>();
    assert_eq!(after, before);
    assert!(ed.status.contains("existing UVs kept"), "{}", ed.status);
}

#[test]
fn shared_materials_are_copied_and_objects_without_one_get_their_wire_colour() {
    let (mut ed, a) = box_editor();
    ed.create_primitive(PrimitiveKind::Box, Vec3::new(5.0, 0.0, 0.0));
    let b = ed.current().unwrap();
    let shared = ed.scene.materials.mats.insert(Preset::Clay.material());
    ed.scene.objects[a].material = Some(shared);
    ed.scene.objects[b].material = Some(shared);
    ed.auto_setup_canvas();
    let mine = ed.scene.objects[b].material.unwrap();
    assert_ne!(mine, shared, "painting one object must not show on the other");
    assert!(ed.scene.materials.mats[shared].paint.is_none() && ed.scene.materials.mats[mine].paint.is_some());
    assert!(ed.scene.materials.mats[mine].name.ends_with(" Paint"));
    // No material at all: the wire colour becomes the base.
    let (mut ed, id) = box_editor();
    let wire = ed.scene.objects[id].wire_color;
    ed.auto_setup_canvas();
    assert_eq!(ed.scene.materials.mats[ed.scene.objects[id].material.unwrap()].base_color, wire);
}

#[test]
fn layer_commands_are_undoable_and_erasing_reveals_the_base() {
    let (mut ed, id) = box_editor();
    click(&mut ed, id);
    let after_one = painted_tiles(&ed);
    ed.add_paint_layer();
    ed.add_fill_layer();
    let names = |ed: &Editor| ed.paint_stack().unwrap().layers.iter().map(|l| l.name.clone()).collect::<Vec<_>>();
    assert_eq!(names(&ed), ["Paint Layer", "Paint Layer 2", "Fill: Color"]);
    assert_eq!(ed.paint_stack().unwrap().active, 2);
    ed.shift_paint_layer(2, false);
    assert_eq!(names(&ed)[1], "Fill: Color");
    ed.edit_paint_stack("Layer Blend Mode", |s| s.layers[1].blend = BlendMode::Multiply);
    ed.remove_paint_layer(1);
    assert_eq!(names(&ed), ["Paint Layer", "Paint Layer 2"]);
    for _ in 0..5 {
        ed.undo();
    }
    assert_eq!(names(&ed), ["Paint Layer"], "five commands, five undo steps");
    assert_eq!(painted_tiles(&ed), after_one, "the stroke is still there");
    // Erase over the same spot: the layer is empty again there.
    ed.set_active_paint_layer(0);
    ed.paint.brush.tool = crate::paint::BrushTool::Erase;
    ed.paint.brush.size = 10.0;
    click(&mut ed, id);
    let m = ed.scene.objects[id].material.unwrap();
    let mat = &ed.scene.materials.mats[m];
    let o = &ed.scene.objects[id];
    let top_uv = o.evaluated_mesh.polygons.values().find(|p| o.evaluated_mesh.poly_normal_raw(o.evaluated_mesh.polygons.iter().find(|(_, q)| std::ptr::eq(*q, *p)).unwrap().0).z > 0.9).map(|p| p.corners.iter().map(|c| c.uv).sum::<Vec2>() / p.corners.len() as f32).unwrap();
    let mut bare = mat.clone();
    bare.paint = None;
    let shade = |m: &crate::materials::Material| crate::materials::shading::surface(m, top_uv, Vec3::ZERO, bevy::math::Vec4::ONE);
    assert_eq!(shade(mat), shade(&bare), "erased: the untouched base material shows");
}

#[test]
fn fill_bake_smart_layers_and_export() {
    let (mut ed, id) = box_editor();
    ed.paint.mode = PaintMode::Texture;
    ed.paint.brush.alpha = Alpha::BulletHole;
    ed.paint_fill();
    assert!(ed.status.starts_with("Fill:"), "{}", ed.status);
    assert!(painted_tiles(&ed) > 0);
    // Bake + four smart layers.
    let maps = ed.bake_mesh_maps().unwrap();
    assert_eq!((maps.size, maps.version), (64, ed.scene.objects[id].version));
    for kind in SmartKind::ALL {
        ed.add_smart_layer(kind, 0);
    }
    let stack = ed.paint_stack().unwrap();
    assert_eq!(stack.layers.len(), 5);
    assert!(stack.layers[1..].iter().all(|l| l.smart.is_some() && l.is_fill()));
    assert_eq!(stack.layers[1].name, "Edge Wear");
    let LayerKind::Fill { mask: Some(wear), .. } = &stack.layers[1].kind else { panic!("mask") };
    assert!(!wear.is_empty(), "a box has edges to wear");
    // Changing Amount rebuilds the mask.
    let before = wear.clone();
    let m = ed.scene.objects[id].material.unwrap();
    ed.scene.materials.mats[m].paint.as_mut().unwrap().layers[1].smart.as_mut().unwrap().amount = 1.0;
    ed.rebuild_smart_layer(1);
    let LayerKind::Fill { mask: Some(more), .. } = &ed.paint_stack().unwrap().layers[1].kind else { panic!("mask") };
    assert!(!more.changed_tiles(&before).is_empty());
    ed.undo();
    assert_eq!(ed.paint_stack().unwrap().layers.len(), 4, "a smart layer is one undo step");
    // Export: four material textures + four mesh maps.
    let dir = std::env::temp_dir().join(format!("ilex_paint_export_{}", std::process::id()));
    ed.export_paint_textures(&dir.to_string_lossy());
    assert!(ed.status.contains("8 PNGs"), "{}", ed.status);
    let name = ed.scene.objects[id].name.replace(|c: char| !c.is_ascii_alphanumeric(), "_");
    for kind in ["basecolor", "orm", "normal", "emissive", "curvature", "ao", "world_normal", "height"] {
        assert!(dir.join(format!("{name}_{kind}.png")).exists(), "{kind}");
    }
    let img = image::open(dir.join(format!("{name}_basecolor.png"))).unwrap();
    assert_eq!((img.width(), img.height()), (128, 128));
    let _ = std::fs::remove_dir_all(dir);
}
