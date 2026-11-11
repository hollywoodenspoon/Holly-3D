//! Help screenshots. `minimax --help-shot <id>` sets up a scene and opens
//! the panels a Help page shows; `tools/help_shots.sh` captures each one
//! into `assets/help/<id>.jpg`, which are built into the program.

use crate::editor::{CommandTab, Editor, Tool, TransformMode};
use crate::generators::{Era, WorldKind};
use crate::mech::MechKind;
use crate::mesh::SubObject;
use crate::primitives::PrimitiveKind;
use crate::splines::ShapeKind;
use crate::ui::actions::Action;
use bevy::math::Vec3;

/// Every screenshot: id and the image built into the program.
pub const SHOTS: &[(&str, &[u8])] = &[
    ("layout", include_bytes!("../../../assets/help/layout.jpg")),
    ("create", include_bytes!("../../../assets/help/create.jpg")),
    ("transform", include_bytes!("../../../assets/help/transform.jpg")),
    ("editpoly", include_bytes!("../../../assets/help/editpoly.jpg")),
    ("ribbon", include_bytes!("../../../assets/help/ribbon.jpg")),
    ("modifiers", include_bytes!("../../../assets/help/modifiers.jpg")),
    ("splines", include_bytes!("../../../assets/help/splines.jpg")),
    ("materials", include_bytes!("../../../assets/help/materials.jpg")),
    ("library", include_bytes!("../../../assets/help/library.jpg")),
    ("procedural", include_bytes!("../../../assets/help/procedural.jpg")),
    ("paint", include_bytes!("../../../assets/help/paint.jpg")),
    ("sky", include_bytes!("../../../assets/help/sky.jpg")),
    ("render", include_bytes!("../../../assets/help/render.jpg")),
    ("grading", include_bytes!("../../../assets/help/grading.jpg")),
    ("ghosttown", include_bytes!("../../../assets/help/ghosttown.jpg")),
    ("mech", include_bytes!("../../../assets/help/mech.jpg")),
    ("prefs", include_bytes!("../../../assets/help/prefs.jpg")),
];

/// The ids `tools/help_shots.sh` captures.
pub const IDS: [&str; 17] = [
    "layout", "create", "transform", "editpoly", "ribbon", "modifiers", "splines", "materials", "library", "procedural", "paint", "sky", "render", "grading", "ghosttown", "mech", "prefs",
];

fn demo(ed: &mut Editor) {
    ed.create_primitive(PrimitiveKind::Box, Vec3::ZERO);
    ed.create_primitive(PrimitiveKind::Sphere, Vec3::new(1.8, 0.4, 0.6));
    ed.create_primitive(PrimitiveKind::Torus, Vec3::new(-1.8, 0.2, 0.25));
    let ids: Vec<_> = ed.scene.objects.keys().collect();
    for (id, slot) in ids.into_iter().zip([4, 2, 6]) {
        ed.scene.objects[id].material = Some(ed.scene.materials.slot(slot));
    }
}

fn select_first(ed: &mut Editor) {
    let id = ed.scene.objects.keys().next();
    ed.scene.select_only(id);
}

/// Sets up screenshot `id`. False for an unknown id.
pub fn setup(ed: &mut Editor, id: &str) -> bool {
    ed.prefs.general.show_tour = false;
    match id {
        "layout" => {
            demo(ed);
            select_first(ed);
        }
        "create" => {
            demo(ed);
            ed.scene.select_only(None);
            ed.tab = CommandTab::Create;
            ed.tool = Tool::Create(PrimitiveKind::Box);
        }
        "transform" => {
            demo(ed);
            select_first(ed);
            ed.mode = TransformMode::Move;
        }
        "editpoly" | "ribbon" => {
            ed.create_primitive(PrimitiveKind::Box, Vec3::ZERO);
            Action::ConvertToPoly.run(ed);
            ed.sub = SubObject::Polygon;
            ed.edit_selection(|m, _| {
                if let Some(p) = m.polygons.keys().max_by(|&a, &b| m.poly_centroid(a).z.total_cmp(&m.poly_centroid(b).z)) {
                    m.polygons[p].selected = true;
                }
            });
            ed.iface.ribbon = id == "ribbon";
        }
        "modifiers" => {
            ed.create_primitive(PrimitiveKind::Cylinder, Vec3::ZERO);
            ed.add_modifier("Bend");
            ed.add_modifier("TurboSmooth");
            ed.iface.modifier_buttons = true;
        }
        "splines" => {
            let view = ed.viewports.active;
            ed.create_shape(ShapeKind::Star, view, Vec3::ZERO);
            ed.add_modifier("Extrude");
            ed.tab = CommandTab::Modify;
        }
        "materials" => {
            demo(ed);
            select_first(ed);
            ed.mat_editor.open = true;
        }
        "library" => {
            demo(ed);
            select_first(ed);
            ed.mat_editor.open = true;
            ed.mat_editor.library = true;
            ed.apply_library(crate::materials::library::find("Red Brick").expect("in the library"));
            ed.assign_material_to_selection();
            ed.prefs.general.favourite_materials = vec!["Red Brick".into(), "Gold".into()];
        }
        "procedural" => {
            demo(ed);
            select_first(ed);
            ed.mat_editor.open = true;
            ed.edit_material(None, |m| {
                m.set_pattern(crate::materials::proc_engine::Pattern::Bricks, 0);
                (m.proc.moss, m.proc.cavity_dirt, m.proc.edge_wear) = (0.45, 0.35, 0.4);
            });
            ed.assign_material_to_selection();
        }
        "paint" => {
            demo(ed);
            select_first(ed);
            Action::Paint.run(ed);
            // Texture Paint on the box: its material stays as Layer 0,
            // with damage stamps and smart weathering layers on top.
            ed.paint.mode = crate::paint::PaintMode::Texture;
            ed.paint.canvas_size = 1024;
            ed.paint.bake.size = 256;
            if let Some(id) = ed.current() {
                let o = &ed.scene.objects[id];
                let b = o.evaluated_mesh.bounds().expect("box");
                let w = o.world_matrix();
                ed.paint.brush.size = 0.28;
                for (alpha, at, n) in [
                    (crate::paint::Alpha::BulletHole, Vec3::new(b.max.x, -0.25, 0.55), Vec3::X),
                    (crate::paint::Alpha::EmberCracks, Vec3::new(b.max.x, 0.3, 0.3), Vec3::X),
                    (crate::paint::Alpha::BloodSplatter, Vec3::new(-0.2, b.min.y, 0.5), Vec3::NEG_Y),
                    (crate::paint::Alpha::BlastStar, Vec3::new(0.25, b.min.y, 0.35), Vec3::NEG_Y),
                ] {
                    ed.paint.brush.alpha = alpha;
                    let hit = w.transform_point3(at);
                    if ed.begin_layer_stroke(id, hit, -n) {
                        ed.stamp_at(hit, n, None);
                        ed.paint_end();
                    }
                }
                ed.add_smart_layer(crate::paint::SmartKind::EdgeWear, 1);
                ed.add_smart_layer(crate::paint::SmartKind::TopDown, 1);
                ed.history = Default::default();
            }
        }
        "sky" => {
            demo(ed);
            ed.env_open = true;
        }
        "render" => {
            demo(ed);
            ed.rfw.setup_open = true;
        }
        "grading" => {
            demo(ed);
            ed.zoom_extents(true);
            (ed.render.width, ed.render.height) = (480, 360);
            let g = &mut ed.render.post.grade;
            (g.temperature, g.shadows, g.vibrance, g.clarity, g.vignette) = (7600.0, 35.0, 30.0, 25.0, -45.0);
            g.shadow_wheel = crate::render::grading::Wheel { hue: 205.0, amount: 0.45 };
            g.highlight_wheel = crate::render::grading::Wheel { hue: 35.0, amount: 0.35 };
            ed.rfw.grading_open = true;
            ed.start_render();
        }
        "ghosttown" => {
            ed.create_world(WorldKind::Settlement(Era::Village), Vec3::ZERO);
            ed.zoom_extents(true);
        }
        "mech" => {
            ed.create_mech(MechKind::Gear, Vec3::ZERO);
            ed.create_mech(MechKind::Piston, Vec3::new(2.5, 0.0, 0.0));
            ed.zoom_extents(true);
        }
        "prefs" => {
            demo(ed);
            ed.iface.prefs_open = true;
        }
        _ => return false,
    }
    ed.history = Default::default();
    true
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_shot_sets_up_and_is_built_in() {
        for id in super::IDS {
            let mut ed = crate::editor::Editor::default();
            assert!(super::setup(&mut ed, id), "{id}");
            let img = super::SHOTS.iter().find(|s| s.0 == id).unwrap_or_else(|| panic!("{id} not built in")).1;
            assert!(image::load_from_memory(img).is_ok(), "{id} decodes");
        }
        assert!(!super::setup(&mut crate::editor::Editor::default(), "nope"));
    }
}
