//! Scene Explorer at the top of the Properties column (H toggles it).

use super::theme;
use super::widgets::{color32, dim};
use crate::editor::Editor;
use crate::scene::{BaseObject, ObjId};
use bevy_egui::egui::{self, RichText};

/// (id, name, selected, hidden, frozen, wire color, icon)
type Row = (ObjId, String, bool, bool, bool, [f32; 3], &'static str);

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
    ui.horizontal(|ui| {
        ui.label(RichText::new("SCENE").size(11.0).color(theme::text_dim()).strong());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("⏶").on_hover_text("Hide the Scene Explorer (H)").clicked() {
                ed.explorer_visible = false;
            }
            ui.add(egui::TextEdit::singleline(&mut ed.explorer_filter).hint_text("🔍 Filter…").desired_width(ui.available_width()));
        });
    });
    ui.add_space(2.0);
    let filter = ed.explorer_filter.to_lowercase();
    let rows: Vec<Row> = ed
        .scene
        .objects
        .iter()
        .filter(|(_, o)| filter.is_empty() || o.name.to_lowercase().contains(&filter))
        .map(|(k, o)| {
            let icon = match o.base {
                BaseObject::Primitive(_) => "◼",
                BaseObject::EditablePoly(_) => "⬜",
                BaseObject::Light(_) => "💡",
                BaseObject::Target => "◎",
                BaseObject::Camera(_) => "🎥",
                BaseObject::Shape(_) => "〰",
                BaseObject::GeoGen(_) => "💎",
                BaseObject::Mech(_) => "⚙",
                BaseObject::World(_) => "🌍",
                BaseObject::Character(_) => "♀",
            };
            (k, o.name.clone(), o.selected, o.hidden, o.frozen, o.wire_color, icon)
        })
        .collect();
    if rows.is_empty() {
        ui.label(dim("No objects yet — use Create -> Geometry."));
    }
    egui::ScrollArea::vertical().id_salt("explorer_rows").auto_shrink([false, false]).show(ui, |ui| {
        for (id, name, sel, hidden, frozen, color, icon) in rows {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                if ui.selectable_label(!hidden, "👁").on_hover_text("Visibility").clicked() {
                    ed.scene.objects[id].hidden = !hidden;
                }
                if ui.selectable_label(frozen, "❄").on_hover_text("Freeze").clicked() {
                    ed.scene.objects[id].frozen = !frozen;
                }
                ui.label(RichText::new(icon).color(color32(color)));
                let text = if sel { RichText::new(&name).color(theme::hover()).strong() } else { RichText::new(&name) };
                let r = ui.add(egui::Button::new(text).frame(false));
                if r.clicked() {
                    let add = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
                    if add {
                        ed.scene.objects[id].selected = !sel;
                    } else {
                        ed.scene.select_only(Some(id));
                        ed.sub = crate::mesh::SubObject::Object;
                    }
                }
            });
        }
    });
}
