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
    if ed.scene.objects[id].is_helper() {
        let owner = ed.scene.objects.values().find(|o| o.light().is_some_and(|l| l.target == Some(id))).map(|o| o.name.clone());
        ui.add_space(6.0);
        ui.label(dim(&format!("Target of {} — move it with W to aim the light.", owner.unwrap_or_default())));
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
