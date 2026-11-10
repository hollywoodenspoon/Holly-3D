//! Modify tab: name + wire color, Modifier List, stack and context rollouts.

use super::{CONTENT_W, light_params, params, stack};
use crate::editor::Editor;
use crate::mesh::SubObject;
use crate::modifiers::ModifierKind;
use crate::ui::UiState;
use crate::ui::rollouts;
use crate::ui::widgets::{action, dim};
use bevy_egui::egui;

pub fn show(ui: &mut egui::Ui, ed: &mut Editor, st: &mut UiState) {
    let Some(id) = ed.current() else {
        ui.add_space(8.0);
        ui.label(dim("Select an object to modify it."));
        return;
    };
    ui.horizontal(|ui| {
        let o = &mut ed.scene.objects[id];
        ui.add(egui::TextEdit::singleline(&mut o.name).desired_width(CONTENT_W - 40.0));
        ui.color_edit_button_rgb(&mut o.wire_color).on_hover_text("Wireframe color");
    });
    // Lights and targets have no modifier stack.
    if ed.scene.objects[id].light().is_some() {
        ui.add_space(6.0);
        return light_params::show(ui, ed);
    }
    if ed.scene.objects[id].camera().is_some() {
        ui.add_space(6.0);
        return super::camera_params::show(ui, ed);
    }
    if ed.scene.objects[id].is_helper() {
        let owner = ed.scene.objects.values().find(|o| o.target_id() == Some(id)).map(|o| o.name.clone());
        ui.add_space(6.0);
        ui.label(dim(&format!("Target of {} — move it with W to aim it.", owner.unwrap_or_default())));
        return;
    }
    stack::modifier_list(ui, ed, st);
    stack::stack_box(ui, ed);
    stack::stack_actions(ui, ed);
    ui.add_space(6.0);
    let o = &ed.scene.objects[id];
    if o.active_level.is_none() && o.shape_object().is_some() {
        return rollouts::spline::shape_base(ui, ed);
    }
    if o.active_level.is_none() && matches!(o.base, crate::scene::BaseObject::World(_)) {
        rollouts::worlds::base(ui, ed);
        if action(ui, "Convert to Editable Poly", true, CONTENT_W) {
            ed.convert_to_editable_poly();
        }
        return;
    }
    let mech = matches!(o.base, crate::scene::BaseObject::Mech(_));
    if o.active_level.is_none() && (mech || matches!(o.base, crate::scene::BaseObject::GeoGen(_))) {
        if mech {
            rollouts::mech::base(ui, ed);
        } else {
            rollouts::geogen::base(ui, ed);
        }
        if action(ui, "Convert to Editable Poly", true, CONTENT_W) {
            ed.convert_to_editable_poly();
        }
        return;
    }
    if o.active_level.is_none() && o.primitive_mut_ref().is_some() {
        params::show(ui, ed);
        ui.label(dim("Convert to Editable Poly (right-click quad menu) or add an Edit Poly modifier to model."));
        if action(ui, "Convert to Editable Poly", true, CONTENT_W) {
            ed.convert_to_editable_poly();
        }
        return;
    }
    match o.active_modifier() {
        Some(ModifierKind::Ffd(_)) => return rollouts::modifiers::ffd(ui, ed),
        Some(ModifierKind::UvwMap(_)) => return rollouts::modifiers::uvw_map(ui, ed),
        Some(ModifierKind::Unwrap(_)) => return rollouts::modifiers::unwrap(ui, ed),
        Some(ModifierKind::VertexPaint(_)) => return crate::ui::paint_box::modifier_rollout(ui, ed),
        Some(ModifierKind::Lathe(_)) => return rollouts::spline::lathe(ui, ed),
        Some(ModifierKind::Extrude(_)) => return rollouts::spline::extrude(ui, ed),
        Some(ModifierKind::GeoGen(_)) => return rollouts::geogen::modifier(ui, ed),
        Some(ModifierKind::Terrain(_)) => return rollouts::worlds::terrain_modifier(ui, ed),
        Some(ModifierKind::Ocean(_)) => return rollouts::worlds::ocean_modifier(ui, ed),
        Some(ModifierKind::Greeble(_)) => return rollouts::stack_mods::greeble(ui, ed),
        Some(ModifierKind::Chamfer(_)) => return rollouts::stack_mods::chamfer(ui, ed),
        Some(ModifierKind::PanelGroove(_)) => return rollouts::stack_mods::panel_groove(ui, ed),
        Some(ModifierKind::Symmetry(_)) => return rollouts::stack_mods::symmetry(ui, ed),
        Some(ModifierKind::Shell(_)) => return rollouts::stack_mods::shell(ui, ed),
        Some(ModifierKind::TurboSmooth(_)) => return rollouts::stack_mods::turbosmooth(ui, ed),
        Some(ModifierKind::Bend(_)) => return rollouts::deformers::bend(ui, ed),
        Some(ModifierKind::Twist(_)) => return rollouts::deformers::twist(ui, ed),
        Some(ModifierKind::Taper(_)) => return rollouts::deformers::taper(ui, ed),
        Some(ModifierKind::Push(_)) => return rollouts::deformers::push(ui, ed),
        Some(ModifierKind::EditNormals(_)) => return rollouts::adv_mods::edit_normals(ui, ed),
        Some(ModifierKind::Boolean(_)) => return rollouts::adv_mods::boolean(ui, ed),
        Some(ModifierKind::ProOptimizer(_)) => return rollouts::adv_mods::pro_optimizer(ui, ed),
        _ => {}
    }
    if !o.sub_object_editable() {
        return;
    }
    rollouts::selection::show(ui, ed);
    rollouts::selection::soft_selection(ui, ed);
    match ed.sub {
        SubObject::Vertex => rollouts::edit_sub::vertices(ui, ed),
        SubObject::Edge | SubObject::Border => rollouts::edit_sub::edges(ui, ed),
        SubObject::Polygon | SubObject::Element => rollouts::edit_sub::polygons(ui, ed),
        SubObject::Object => {}
    }
    rollouts::edit_geometry::show(ui, ed);
    if ed.sub.is_face_like() {
        rollouts::smoothing::show(ui, ed);
    }
}
