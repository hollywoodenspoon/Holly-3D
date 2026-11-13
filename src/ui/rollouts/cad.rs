//! SolidCraft-style CAD rollouts: the multi-profile Loft modifier.

use super::spline::{grid, mod_params, num_row, u_row};
use crate::editor::{Editor, Tool};
use crate::modifiers::ModifierKind;
use crate::ui::properties::content_w;
use crate::ui::theme;
use crate::ui::widgets::{dim, max_spinner_range, rollout};
use bevy_egui::egui::{self, RichText};

/// Sections list: path % spinner per section, click to highlight.
fn sections(ui: &mut egui::Ui, ed: &mut Editor) {
    mod_params(ui, ed, |ui, k, t| {
        let ModifierKind::Loft(l) = k else { return };
        egui::Frame::new().fill(theme::input()).corner_radius(theme::R_CARD).inner_margin(8).show(ui, |ui| {
            ui.set_width(content_w() - 12.0);
            ui.set_min_height(60.0);
            if l.sections.is_empty() {
                ui.label(dim("No sections yet: a 0.2 circle is used."));
            }
            let mut resort = false;
            for i in 0..l.sections.len() {
                ui.horizontal(|ui| {
                    let on = l.selected == Some(i);
                    let s = &mut l.sections[i];
                    let label = format!("{} ({} pts{})", s.name, s.points.len(), if s.closed { "" } else { ", open" });
                    if ui.selectable_label(on, label).clicked() {
                        l.selected = if on { None } else { Some(i) };
                        t.changed = true;
                    }
                    let r = max_spinner_range(ui, &mut s.at, 0.5, 0.0, 0.0..=100.0).on_hover_text("Where on the path, % (right-click: 0)");
                    resort |= r.drag_stopped() || r.lost_focus() || r.secondary_clicked();
                    t.r(r);
                    ui.label("%");
                });
            }
            if resort {
                let sel = l.selected.map(|i| l.sections[i].clone());
                l.sections.sort_by(|a, b| a.at.total_cmp(&b.at));
                l.selected = sel.and_then(|s| l.sections.iter().position(|x| *x == s));
            }
        });
        grid(ui, "loft_next", |ui| num_row(ui, t, "Next Section At %:", &mut l.next_at, 0.0..=100.0, 100.0));
    });
}

pub fn loft(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Loft Sections", true, |ui| {
        ui.label(dim("The object under the modifier is the path; pick shapes as cross-sections."));
        let picking = ed.tool == Tool::PickLoftShape;
        ui.horizontal(|ui| {
            let w = content_w() / 2.0 - 4.0;
            if ui
                .add_sized([w, 28.0], egui::Button::new(if picking { "Picking Shapes…" } else { "Pick Shape" }).selected(picking))
                .on_hover_text("Click shape objects in a viewport; each becomes a section at \"Next Section At %\"")
                .clicked()
            {
                ed.tool = if picking { Tool::None } else { Tool::PickLoftShape };
            }
            if ui.add_sized([w, 28.0], egui::Button::new("Remove Section")).on_hover_text("Remove the highlighted section").clicked() {
                ed.remove_loft_section();
            }
        });
        sections(ui, ed);
    });
    rollout(ui, "Skin Parameters", true, |ui| {
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::Loft(l) = k else { return };
            grid(ui, "loft_skin", |ui| {
                u_row(ui, t, "Path Steps:", &mut l.path_steps, 48, 2..=1024);
                u_row(ui, t, "Shape Steps:", &mut l.shape_steps, 32, 3..=512);
            });
            ui.label(RichText::new("Deformations").strong());
            grid(ui, "loft_deform", |ui| {
                num_row(ui, t, "Twist (°):", &mut l.twist, -3600.0..=3600.0, 0.0);
                num_row(ui, t, "Scale Start %:", &mut l.scale_start, 0.0..=1000.0, 100.0);
                num_row(ui, t, "Scale End %:", &mut l.scale_end, 0.0..=1000.0, 100.0);
                num_row(ui, t, "Scale Curve:", &mut l.scale_curve, 0.1..=10.0, 1.0);
            });
            ui.horizontal(|ui| {
                t.r(ui.checkbox(&mut l.cap_start, "Cap Start"));
                t.r(ui.checkbox(&mut l.cap_end, "Cap End"));
                t.r(ui.checkbox(&mut l.flip, "Flip Normals"));
            });
            ui.label(dim("Scale Curve 1 = linear; below 1 eases out, above eases in."));
        });
    });
}
