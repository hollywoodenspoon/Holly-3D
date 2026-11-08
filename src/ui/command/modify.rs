//! Modify tab: name + wire color, Modifier List, stack and context rollouts.

use super::{CONTENT_W, params, stack};
use crate::editor::Editor;
use crate::mesh::SubObject;
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
    stack::modifier_list(ui, ed, st);
    stack::stack_box(ui, ed);
    stack::stack_actions(ui, ed);
    ui.add_space(6.0);
    let o = &ed.scene.objects[id];
    if o.active_level.is_none() && o.primitive_mut_ref().is_some() {
        params::show(ui, ed);
        ui.label(dim("Convert to Editable Poly (right-click quad menu) or add an Edit Poly modifier to model."));
        if action(ui, "Convert to Editable Poly", true, CONTENT_W) {
            ed.convert_to_editable_poly();
        }
        return;
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
