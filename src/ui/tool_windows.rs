//! Small tool windows opened from the toolbar and menus: Select From Scene,
//! Measure, Grid and Snap Settings, the Hotkey List and About.

use super::action_ui::hotkey_text;
use super::actions::Action;
use super::theme;
use super::widgets::dim;
use crate::editor::{Editor, ObjCategory};
use bevy_egui::egui::{self, RichText};

pub fn show(ctx: &egui::Context, ed: &mut Editor) {
    select_by_name(ctx, ed);
    measure(ctx, ed);
    snap_settings(ctx, ed);
    hotkey_list(ctx, ed);
    about(ctx, ed);
}

fn select_by_name(ctx: &egui::Context, ed: &mut Editor) {
    let mut open = ed.iface.select_by_name_open;
    let mut pick: Option<(Vec<crate::scene::ObjId>, bool)> = None;
    egui::Window::new("Select From Scene").open(&mut open).default_size([280.0, 360.0]).collapsible(false).show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut ed.iface.select_by_name_filter).hint_text("Find…").desired_width(150.0));
            egui::ComboBox::from_id_salt("sbn_filter").width(80.0).selected_text(ed.iface.select_filter.label()).show_ui(ui, |ui| {
                for c in ObjCategory::ALL {
                    ui.selectable_value(&mut ed.iface.select_filter, c, c.label());
                }
            });
        });
        let filter = ed.iface.select_by_name_filter.to_lowercase();
        let cat = ed.iface.select_filter;
        let rows: Vec<_> = ed
            .scene
            .objects
            .iter()
            .filter(|(_, o)| cat.accepts(o) && (filter.is_empty() || o.name.to_lowercase().contains(&filter)))
            .map(|(id, o)| (id, o.name.clone(), o.selected, o.hidden || o.frozen, ObjCategory::of(o).label()))
            .collect();
        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
            for (id, name, selected, locked, kind) in &rows {
                let text = if *locked { RichText::new(format!("{name}  ({kind}, hidden/frozen)")).color(theme::TEXT_DIM) } else { RichText::new(format!("{name}  ·  {kind}")) };
                let r = ui.add_enabled(!locked, egui::Button::selectable(*selected, text));
                if r.clicked() {
                    let add = ui.input(|i| i.modifiers.ctrl || i.modifiers.command);
                    pick = Some((vec![*id], add));
                }
            }
        });
        ui.horizontal(|ui| {
            if ui.button("All").clicked() {
                pick = Some((rows.iter().filter(|r| !r.3).map(|r| r.0).collect(), false));
            }
            if ui.button("None").clicked() {
                pick = Some((vec![], false));
            }
            ui.label(dim("Ctrl+click adds"));
        });
    });
    if let Some((ids, add)) = pick {
        ed.select_ids(&ids, add);
    }
    ed.iface.select_by_name_open = open;
}

fn measure(ctx: &egui::Context, ed: &mut Editor) {
    let mut open = ed.iface.measure_open;
    egui::Window::new("Measure").open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
        let sel = ed.scene.selected();
        ui.label(if sel.is_empty() { "Nothing selected: measuring the whole scene".to_string() } else { format!("{} object(s) selected", sel.len()) });
        match ed.selection_bounds() {
            Some(b) => {
                let (size, c) = (b.max - b.min, (b.max + b.min) * 0.5);
                egui::Grid::new("measure").num_columns(2).show(ui, |ui| {
                    ui.label("Size (X × Y × Z)");
                    ui.label(format!("{:.3} × {:.3} × {:.3}", size.x, size.y, size.z));
                    ui.end_row();
                    ui.label("Centre");
                    ui.label(format!("{:.3}, {:.3}, {:.3}", c.x, c.y, c.z));
                    ui.end_row();
                    ui.label("Diagonal");
                    ui.label(format!("{:.3}", size.length()));
                    ui.end_row();
                });
            }
            None => {
                ui.label(dim("No geometry"));
            }
        }
        let (v, e, f) = sel.iter().fold((0, 0, 0), |acc, id| {
            let (v, e, f) = ed.scene.objects[*id].evaluated_mesh.counts();
            (acc.0 + v, acc.1 + e, acc.2 + f)
        });
        ui.label(dim(&format!("Selected: {v} verts · {e} edges · {f} polys")));
    });
    ed.iface.measure_open = open;
}

fn snap_settings(ctx: &egui::Context, ed: &mut Editor) {
    let mut open = ed.iface.snap_settings_open;
    egui::Window::new("Grid and Snap Settings").open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
        super::snaps::settings(ui, ed);
    });
    ed.iface.snap_settings_open = open;
}

fn hotkey_list(ctx: &egui::Context, ed: &mut Editor) {
    let mut open = ed.iface.hotkeys_open;
    egui::Window::new("Hotkey List").open(&mut open).default_size([360.0, 420.0]).collapsible(false).show(ctx, |ui| {
        ui.add(egui::TextEdit::singleline(&mut ed.iface.hotkeys_filter).hint_text("Find an action or key…").desired_width(f32::INFINITY));
        let filter = ed.iface.hotkeys_filter.to_lowercase();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("hotkeys").num_columns(2).striped(true).show(ui, |ui| {
                for a in Action::all() {
                    let (label, key) = (a.label(), hotkey_text(a));
                    if !filter.is_empty() && !label.to_lowercase().contains(&filter) && !key.to_lowercase().contains(&filter) {
                        continue;
                    }
                    ui.label(label).on_hover_text(a.tip());
                    ui.label(if key.is_empty() { dim("—") } else { RichText::new(key).color(theme::AMBER) });
                    ui.end_row();
                }
            });
        });
        ui.label(dim("Changing hotkeys arrives with Preferences."));
    });
    ed.iface.hotkeys_open = open;
}

fn about(ctx: &egui::Context, ed: &mut Editor) {
    let mut open = ed.iface.about_open;
    egui::Window::new("About MiniMax").open(&mut open).resizable(false).collapsible(false).show(ctx, |ui| {
        ui.label(RichText::new("MiniMax").color(theme::AMBER).strong().size(20.0));
        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
        ui.label("A clean 3ds Max-style modeler, painter and renderer, written in Rust with Bevy and egui.");
        ui.label(dim("Every command has a button or a menu entry; hotkeys are shortcuts."));
    });
    ed.iface.about_open = open;
}
