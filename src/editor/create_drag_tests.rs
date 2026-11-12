//! Simulated mouse presses and moves for click-and-drag creation.

use super::*;
use crate::viewport::layout::ScreenRect;

/// An empty scene with the Top view (0) maximized at 800×600.
fn editor(view: usize) -> Editor {
    let mut ed = Editor::default();
    (ed.viewports.maximized, ed.viewports.active) = (true, view);
    ed.viewports.layout(ScreenRect { min: Vec2::ZERO, max: Vec2::new(800.0, 600.0) }, 0.0);
    ed
}

fn screen(ed: &Editor, view: usize, p: Vec3) -> Vec2 {
    ed.viewports.views[view].to_screen(p).unwrap()
}

/// Press at `a`, drag to `b`, release; returns the created object.
fn drag(ed: &mut Editor, view: usize, a: Vec3, b: Vec3) -> Option<ObjId> {
    let (sa, sb) = (screen(ed, view, a), screen(ed, view, b));
    assert!(ed.create_press(view, sa));
    ed.create_move(sa.lerp(sb, 0.5));
    ed.create_move(sb);
    ed.create_release(sb);
    ed.iface.create_drag.as_ref().and_then(|d| d.id).or_else(|| ed.current())
}

/// Moves the mouse `px` pixels up from where the last step ended, then clicks.
fn lift_and_click(ed: &mut Editor, px: f32) {
    let a = ed.iface.create_drag.as_ref().unwrap().anchor;
    let p = a - Vec2::Y * px;
    ed.create_move(p);
    ed.create_press(0, p);
}

fn prim(ed: &Editor, id: ObjId) -> Primitive {
    match &ed.scene.objects[id].base {
        BaseObject::Primitive(p) => p.clone(),
        b => panic!("not a primitive: {}", b.label()),
    }
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.05 * b.abs().max(0.1)
}

#[test]
fn box_base_then_height_in_top_view() {
    let mut ed = editor(0);
    ed.tool = Tool::Create(PrimitiveKind::Box);
    let id = drag(&mut ed, 0, Vec3::new(-1.0, -2.0, 0.0), Vec3::new(3.0, 0.0, 0.0)).unwrap();
    assert_eq!(ed.iface.create_drag.as_ref().unwrap().step, 1, "waits for the height");
    let Primitive::Box { width, length, height, .. } = prim(&ed, id) else { panic!() };
    assert!(near(width, 4.0) && near(length, 2.0), "{width} × {length}");
    assert!(height < 0.05, "flat until the height step");
    lift_and_click(&mut ed, 50.0);
    assert!(ed.iface.create_drag.is_none(), "finished");
    let Primitive::Box { height, .. } = prim(&ed, id) else { panic!() };
    let per_px = ed.viewports.views[0]
        .construction_point(Vec2::new(400.0, 301.0))
        .distance(ed.viewports.views[0].construction_point(Vec2::new(400.0, 300.0)));
    assert!(near(height, 50.0 * per_px), "height {height}");
    let t = ed.scene.objects[id].transform.translation;
    assert!(t.distance(Vec3::new(1.0, -1.0, 0.0)) < 0.1, "base centre {t}");
    assert_eq!(ed.tool, Tool::Create(PrimitiveKind::Box), "tool stays active");
    // One undo removes the whole box.
    ed.undo();
    assert!(ed.scene.objects.is_empty());
}

#[test]
fn sphere_plane_and_cube_finish_on_release() {
    let mut ed = editor(0);
    ed.tool = Tool::Create(PrimitiveKind::Sphere);
    let id = drag(&mut ed, 0, Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0)).unwrap();
    assert!(ed.iface.create_drag.is_none());
    let Primitive::Sphere { radius, .. } = prim(&ed, id) else { panic!() };
    assert!(near(radius, 2.0), "{radius}");
    // Edge method: the drag is the diameter.
    ed.iface.create_method.edge = true;
    let id = drag(&mut ed, 0, Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0)).unwrap();
    let Primitive::Sphere { radius, .. } = prim(&ed, id) else { panic!() };
    assert!(near(radius, 1.0), "{radius}");
    assert!(ed.scene.objects[id].transform.translation.distance(Vec3::X) < 0.05);
    ed.tool = Tool::Create(PrimitiveKind::Plane);
    let id = drag(&mut ed, 0, Vec3::ZERO, Vec3::new(-3.0, 1.0, 0.0)).unwrap();
    let Primitive::Plane { width, length, .. } = prim(&ed, id) else { panic!() };
    assert!(near(width, 3.0) && near(length, 1.0));
    ed.iface.create_method.cube = true;
    ed.tool = Tool::Create(PrimitiveKind::Box);
    let id = drag(&mut ed, 0, Vec3::ZERO, Vec3::new(1.0, 0.5, 0.0)).unwrap();
    assert!(ed.iface.create_drag.is_none(), "cube is one drag");
    let Primitive::Box { width, length, height, .. } = prim(&ed, id) else { panic!() };
    assert!(near(width, 2.0) && width == length && width == height);
    assert_eq!(ed.scene.objects.len(), 4);
}

#[test]
fn cone_has_three_steps_and_torus_two() {
    let mut ed = editor(0);
    ed.tool = Tool::Create(PrimitiveKind::Cone);
    let id = drag(&mut ed, 0, Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0)).unwrap();
    lift_and_click(&mut ed, 40.0);
    assert_eq!(ed.iface.create_drag.as_ref().unwrap().step, 2, "now the top radius");
    let p = screen(&ed, 0, Vec3::new(0.5, 0.0, 0.0));
    ed.create_move(p);
    ed.create_press(0, p);
    assert!(ed.iface.create_drag.is_none());
    let Primitive::Cone { radius1, radius2, height, .. } = prim(&ed, id) else { panic!() };
    assert!(near(radius1, 2.0) && near(radius2, 0.5) && height > 0.1, "{radius1} {radius2} {height}");
    ed.tool = Tool::Create(PrimitiveKind::Torus);
    let id = drag(&mut ed, 0, Vec3::ZERO, Vec3::new(2.0, 0.0, 0.0)).unwrap();
    let p = screen(&ed, 0, Vec3::new(2.4, 0.0, 0.0));
    ed.create_move(p);
    ed.create_press(0, p);
    let Primitive::Torus { radius1, radius2, .. } = prim(&ed, id) else { panic!() };
    assert!(near(radius1, 2.0) && near(radius2, 0.4), "{radius1} {radius2}");
    let o = &ed.scene.objects[id];
    assert!(!o.evaluated_mesh.polygons.is_empty() && o.evaluated_mesh.vertices.len() > 50);
}

#[test]
fn right_click_cancels_and_plain_click_is_left_to_the_click() {
    let mut ed = editor(0);
    ed.tool = Tool::Create(PrimitiveKind::Cylinder);
    drag(&mut ed, 0, Vec3::ZERO, Vec3::new(1.0, 1.0, 0.0)).unwrap();
    assert_eq!(ed.scene.objects.len(), 1);
    assert!(ed.cancel_tool(), "right-click cancels the half-made cylinder");
    assert!(ed.scene.objects.is_empty() && ed.iface.create_drag.is_none());
    assert_eq!(ed.tool, Tool::Create(PrimitiveKind::Cylinder), "tool stays");
    // A press without movement creates nothing; the click path places a default one.
    let s = screen(&ed, 0, Vec3::ZERO);
    ed.create_press(0, s);
    ed.create_move(s + Vec2::X);
    ed.create_release(s + Vec2::X);
    assert!(ed.scene.objects.is_empty() && ed.iface.create_drag.is_none());
    assert!(ed.tool_click(0, s));
    let Primitive::Cylinder { radius, .. } = prim(&ed, ed.current().unwrap()) else { panic!() };
    assert_eq!(radius, 0.5, "default size");
}

#[test]
fn front_view_stands_objects_on_its_plane() {
    let mut ed = editor(1);
    ed.tool = Tool::Create(PrimitiveKind::Plane);
    let id = drag(&mut ed, 1, Vec3::ZERO, Vec3::new(2.0, 0.0, 1.0)).unwrap();
    let o = &ed.scene.objects[id];
    let b =
        crate::mesh::Aabb::from_points(o.evaluated_mesh.vertices.values().map(|v| o.world_matrix().transform_point3(v.position))).unwrap();
    let size = b.max - b.min;
    assert!(near(size.x, 2.0) && near(size.z, 1.0) && size.y < 0.01, "upright in XZ: {size}");
}

#[test]
fn geogen_world_and_gear_take_their_size_from_the_drag() {
    let mut ed = editor(0);
    ed.tool = Tool::CreateGeoGen(GeoMode::Rock);
    let id = drag(&mut ed, 0, Vec3::ZERO, Vec3::new(1.5, 0.0, 0.0)).unwrap();
    lift_and_click(&mut ed, 30.0);
    let BaseObject::GeoGen(g) = &ed.scene.objects[id].base else { panic!() };
    assert!(near(g.base_radius, 1.5) && g.max_height > 0.0 && g.min_height <= g.max_height);
    ed.tool = Tool::CreateWorld(WorldKind::Planet);
    let id = drag(&mut ed, 0, Vec3::ZERO, Vec3::new(3.0, 0.0, 0.0)).unwrap();
    let BaseObject::World(WorldGen::Planet(p)) = &ed.scene.objects[id].base else { panic!() };
    assert!(near(p.radius, 3.0));
    ed.tool = Tool::CreateWorld(WorldKind::Ocean);
    let id = drag(&mut ed, 0, Vec3::new(-4.0, -2.0, 0.0), Vec3::new(4.0, 2.0, 0.0)).unwrap();
    lift_and_click(&mut ed, 20.0);
    let BaseObject::World(WorldGen::Ocean(o)) = &ed.scene.objects[id].base else { panic!() };
    assert!(near(o.width, 8.0) && near(o.length, 4.0) && o.wave_height > 0.0);
    ed.tool = Tool::CreateMech(MechKind::Gear);
    let id = drag(&mut ed, 0, Vec3::ZERO, Vec3::new(1.0, 0.0, 0.0)).unwrap();
    let BaseObject::Mech(MechPart::Gear(g)) = &ed.scene.objects[id].base else { panic!() };
    assert!(near(g.module * g.teeth as f32 / 2.0, 1.0), "pitch radius from the drag");
    assert_eq!(ed.tab, CommandTab::Create, "the Create panel stays open while drawing");
}

#[test]
fn settlement_area_follows_the_drag() {
    let mut ed = editor(0);
    ed.tool = Tool::CreateWorld(WorldKind::Settlement(crate::generators::Era::Village));
    let id = drag(&mut ed, 0, Vec3::new(-20.0, -15.0, 0.0), Vec3::new(20.0, 15.0, 0.0)).unwrap();
    let BaseObject::World(WorldGen::Settlement(s)) = &ed.scene.objects[id].base else { panic!() };
    assert!(near(s.width, 40.0) && near(s.length, 30.0), "{} × {}", s.width, s.length);
}

#[test]
fn building_footprint_follows_the_drag_and_gets_its_pack() {
    use crate::generators::buildings::BuildingKind;
    let mut ed = editor(0);
    ed.tool = Tool::CreateWorld(WorldKind::Building(BuildingKind::Tavern));
    let id = drag(&mut ed, 0, Vec3::new(-6.0, -4.0, 0.0), Vec3::new(6.0, 4.0, 0.0)).unwrap();
    let BaseObject::World(WorldGen::Building(b)) = &ed.scene.objects[id].base else { panic!() };
    assert!(near(b.width, 12.0) && near(b.length, 8.0), "{} × {}", b.width, b.length);
    let o = &ed.scene.objects[id];
    let m = &ed.scene.materials.mats[o.material.unwrap()];
    assert_eq!(m.multi.as_ref().map(|x| x.subs.len()), Some(29), "29-ID GhostTown pack");
    assert!(o.evaluated_mesh.polygons.len() > 50);
}
