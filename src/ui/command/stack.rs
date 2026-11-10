//! Searchable Modifier List + Modifier Stack box + stack action row.

use super::CONTENT_W;
use crate::editor::Editor;
use crate::mesh::SubObject;
use crate::modifiers::{MODIFIER_LIST, ModSub};
use crate::ui::UiState;
use crate::ui::theme;
use crate::ui::widgets::{dim, toggle_button};
use bevy_egui::egui::{self, RichText};

const SUBS: [SubObject; 5] = [SubObject::Vertex, SubObject::Edge, SubObject::Border, SubObject::Polygon, SubObject::Element];

pub fn modifier_list(ui: &mut egui::Ui, ed: &mut Editor, st: &mut UiState) {
    let r =
        ui.add(egui::TextEdit::singleline(&mut ed.modifier_filter).hint_text("Modifier List…  (type to filter)").desired_width(CONTENT_W));
    if r.gained_focus() || r.changed() {
        st.modifier_list_open = true;
    }
    if !st.modifier_list_open {
        return;
    }
    let filter = ed.modifier_filter.to_lowercase();
    let mut chosen = None;
    egui::Frame::new().fill(theme::INPUT).corner_radius(4).inner_margin(6).show(ui, |ui| {
        ui.set_width(CONTENT_W - 12.0);
        egui::ScrollArea::vertical().max_height(160.0).id_salt("modlist").show(ui, |ui| {
            for item in MODIFIER_LIST.iter().filter(|m| filter.is_empty() || m.name.to_lowercase().contains(&filter)) {
                let text = if item.available {
                    RichText::new(item.name)
                } else {
                    RichText::new(format!("{}  ·  later phase", item.name)).color(theme::TEXT_DIM)
                };
                if ui.add_enabled(item.available, egui::Button::new(text).frame(false)).clicked() {
                    chosen = Some(item.name);
                }
            }
        });
        if ui.small_button("Close").clicked() {
            st.modifier_list_open = false;
        }
    });
    if let Some(name) = chosen {
        ed.add_modifier(name);
        ed.modifier_filter.clear();
        st.modifier_list_open = false;
    }
}

pub fn stack_box(ui: &mut egui::Ui, ed: &mut Editor) {
    let Some(id) = ed.current() else { return };
    let o = &ed.scene.objects[id];
    type Row = (Option<usize>, String, bool, bool, &'static [ModSub]);
    let mut rows: Vec<Row> = o
        .modifiers
        .iter()
        .enumerate()
        .rev()
        .map(|(i, m)| (Some(i), m.name.clone(), m.enabled_in_viewport, m.kind.supports_sub_objects(), m.kind.gizmo_subs()))
        .collect();
    rows.push((None, o.base.label().to_string(), true, o.is_editable_poly(), &[]));
    let mod_sub = ed.active_mod_sub();
    let mut set_mod_sub = None;
    let active = o.active_level;
    let mut set_level = None;
    let mut toggle = None;
    let mut set_sub = None;
    egui::Frame::new().fill(theme::INPUT).corner_radius(4).inner_margin(6).show(ui, |ui| {
        ui.set_width(CONTENT_W - 12.0);
        egui::ScrollArea::vertical().max_height(150.0).min_scrolled_height(150.0).id_salt("stack").show(ui, |ui| {
            ui.set_min_height(138.0);
            for (level, name, enabled, mesh_subs, gizmo_subs) in rows {
                let subs = mesh_subs || !gizmo_subs.is_empty();
                let is_active = level == active;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 4.0;
                    if let Some(i) = level {
                        if ui.selectable_label(enabled, "👁").on_hover_text("Enable in viewport").clicked() {
                            toggle = Some(i);
                        }
                    } else {
                        ui.add_space(22.0);
                    }
                    let open_id = egui::Id::new(("stack_open", id, level));
                    let mut open = ui.data(|d| d.get_temp::<bool>(open_id).unwrap_or(false));
                    if subs {
                        if ui.small_button(if open { "⏷" } else { "⏵" }).clicked() {
                            open = !open;
                            ui.data_mut(|d| d.insert_temp(open_id, open));
                        }
                    } else {
                        ui.add_space(18.0);
                    }
                    let text = if is_active { RichText::new(&name).color(theme::INPUT).strong() } else { RichText::new(&name) };
                    let w = ui.available_width();
                    if ui.add_sized([w, 20.0], egui::Button::new(text).selected(is_active)).clicked() {
                        set_level = Some(level);
                    }
                });
                let open_id = egui::Id::new(("stack_open", id, level));
                let open = subs && ui.data(|d| d.get_temp::<bool>(open_id).unwrap_or(false));
                for &g in gizmo_subs.iter().filter(|_| open) {
                    ui.horizontal(|ui| {
                        ui.add_space(48.0);
                        let on = is_active && mod_sub == Some(g);
                        let text = if on { RichText::new(g.label()).color(theme::AMBER).strong() } else { RichText::new(g.label()) };
                        if ui.add(egui::Button::new(text).frame(false)).clicked() {
                            set_level = Some(level);
                            set_mod_sub = Some(g);
                        }
                    });
                }
                if open && mesh_subs {
                    for s in SUBS {
                        ui.horizontal(|ui| {
                            ui.add_space(48.0);
                            let on = is_active && ed.sub == s;
                            let text = if on { RichText::new(s.label()).color(theme::AMBER).strong() } else { RichText::new(s.label()) };
                            if ui.add(egui::Button::new(text).frame(false)).clicked() {
                                set_level = Some(level);
                                set_sub = Some(s);
                            }
                        });
                    }
                }
            }
        });
    });
    if let Some(i) = toggle {
        ed.checkpoint("Toggle Modifier");
        let o = &mut ed.scene.objects[id];
        o.modifiers[i].enabled_in_viewport = !o.modifiers[i].enabled_in_viewport;
        o.evaluate();
        ed.scene.propagate_from(id);
    }
    if let Some(level) = set_level {
        if ed.scene.objects[id].active_level != level {
            ed.scene.objects[id].set_active_level(level);
            ed.sub = SubObject::Object;
            ed.mod_sub = None;
        }
        if let Some(g) = set_mod_sub {
            ed.set_mod_sub(Some(g));
        }
        if let Some(s) = set_sub {
            ed.sub = SubObject::Object;
            ed.set_sub_object(s);
        }
    }
}

pub fn stack_actions(ui: &mut egui::Ui, ed: &mut Editor) {
    let Some(id) = ed.current() else { return };
    let o = &ed.scene.objects[id];
    let shared = o.instance_group.is_some() || o.reference_parent.is_some();
    let instance = o.instance_group.is_some();
    let has_mod = o.active_level.is_some();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
        let pinned = ed.pinned == Some(id);
        if toggle_button(ui, pinned, "📌 Pin")
            .on_hover_text("Pin Stack: keep this object in the Modify panel while selecting others")
            .clicked()
        {
            ed.pinned = if pinned { None } else { Some(id) };
        }
        ui.add_enabled(false, egui::Button::new("Show End Result")).on_disabled_hover_text("End result is always shown");
        if ui.add_enabled(shared, egui::Button::new("Make Unique")).clicked() {
            ed.checkpoint("Make Unique");
            ed.scene.make_unique(id);
        }
        if ui.add_enabled(has_mod, egui::Button::new("🗑 Delete Modifier")).clicked() {
            ed.delete_modifier();
        }
        if ui.button("Collapse To Editable Poly").clicked() {
            ed.collapse_stack();
        }
    });
    if shared {
        ui.label(dim(if instance { "Instance — edits affect all instances" } else { "Reference — base edits follow the parent" }));
    }
}
