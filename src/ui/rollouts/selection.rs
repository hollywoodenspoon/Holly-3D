//! Selection & Soft Selection rollouts.

use super::HALF;
use crate::editor::Editor;
use crate::mesh::SubObject;
use crate::ui::command::CONTENT_W;
use crate::ui::theme;
use crate::ui::widgets::{dim, max_spinner_range, rollout};
use bevy_egui::egui::{self, RichText};

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Selection", true, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            let w = (CONTENT_W - 16.0) / 5.0 - 3.0;
            for (s, label, tip) in [
                (SubObject::Vertex, "1: Vtx", "Vertex (1)"),
                (SubObject::Edge, "2: Edg", "Edge (2)"),
                (SubObject::Border, "3: Brd", "Border (3)"),
                (SubObject::Polygon, "4: Pol", "Polygon (4)"),
                (SubObject::Element, "5: Elm", "Element (5)"),
            ] {
                let on = ed.sub == s;
                let text = if on { RichText::new(label).color(theme::input()).strong() } else { RichText::new(label) };
                if ui.add_sized([w, 30.0], egui::Button::new(text).selected(on)).on_hover_text(tip).clicked() {
                    ed.set_sub_object(s);
                }
            }
        });
        ui.horizontal(|ui| {
            ui.checkbox(&mut ed.by_vertex, "By Vertex");
            ui.checkbox(&mut ed.ignore_backfacing, "Ignore Backfacing");
        });
        let sub = ed.in_sub_object();
        ui.add_enabled_ui(sub, |ui| {
            ui.horizontal(|ui| {
                if ui.add_sized([HALF, 24.0], egui::Button::new("Shrink")).clicked() {
                    ed.edit_selection(|m, s| m.shrink_selection(s));
                }
                if ui.add_sized([HALF, 24.0], egui::Button::new("Grow")).clicked() {
                    ed.edit_selection(|m, s| m.grow_selection(s));
                }
            });
            ui.add_enabled_ui(ed.sub.is_edge_like(), |ui| {
                ui.horizontal(|ui| {
                    let w = CONTENT_W / 2.0 - 50.0;
                    if ui.add_sized([w, 24.0], egui::Button::new("Ring")).on_hover_text("Alt+R").clicked() {
                        ed.edit_selection(|m, _| m.select_ring());
                    }
                    shift_arrows(ui, ed, true);
                    if ui.add_sized([w, 24.0], egui::Button::new("Loop")).on_hover_text("Alt+L / double-click").clicked() {
                        ed.edit_selection(|m, _| m.select_loop());
                    }
                    shift_arrows(ui, ed, false);
                });
            });
        });
        let count = ed.current_obj().map(|o| o.level_mesh.selection_count(ed.sub)).unwrap_or(0);
        let text = if sub {
            format!("{count} {}{} Selected", ed.sub.label(), if count == 1 { "" } else { "s" })
        } else {
            "Object level — press 1–5".into()
        };
        ui.label(dim(&text));
    });
}

/// Ring / Loop shift arrows: move the edge selection one step along its rings or loops.
fn shift_arrows(ui: &mut egui::Ui, ed: &mut Editor, ring: bool) {
    ui.spacing_mut().item_spacing.x = 2.0;
    let what = if ring { "Ring" } else { "Loop" };
    for (label, fwd) in [("◀", false), ("▶", true)] {
        if ui.add_sized([16.0, 24.0], egui::Button::new(label)).on_hover_text(format!("{what} Shift")).clicked() {
            ed.edit_selection(|m, _| {
                if ring {
                    m.shift_ring(fwd);
                } else {
                    m.shift_loop(fwd);
                }
            });
        }
    }
    ui.spacing_mut().item_spacing.x = 6.0;
}

pub fn soft_selection(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Soft Selection", false, |ui| {
        ui.checkbox(&mut ed.soft.enabled, "Use Soft Selection");
        ui.add_enabled_ui(ed.soft.enabled, |ui| {
            egui::Grid::new("soft").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
                ui.label("Falloff:");
                max_spinner_range(ui, &mut ed.soft.falloff, 0.01, 1.0, 0.0..=1000.0);
                ui.end_row();
                ui.label("Pinch:");
                max_spinner_range(ui, &mut ed.soft.pinch, 0.01, 0.0, -10.0..=10.0);
                ui.end_row();
                ui.label("Bubble:");
                max_spinner_range(ui, &mut ed.soft.bubble, 0.01, 0.0, -10.0..=10.0);
                ui.end_row();
            });
            ui.label(dim("Vertices within Falloff move with a red->blue weight gradient."));
        });
    });
}
