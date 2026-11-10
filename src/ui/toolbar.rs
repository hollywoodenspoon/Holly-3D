//! Main Toolbar under the menu bar, built from `actions::layout`: every
//! button is an `Action`, plus the region, filter, coordinate, pivot and sky
//! dropdowns. The row wraps on narrow windows instead of hiding buttons.

use super::action_ui::{self, button, menu_item};
use super::actions::Action;
use super::actions::layout::{Slot, TOOLBAR_LEFT, TOOLBAR_RIGHT};
use super::theme;
use crate::editor::{CoordSystem, Editor, ObjCategory, PivotMode, TransformMode};
use crate::render::sky_presets::SkyPreset;
use crate::viewport::region::RegionShape;
use bevy_egui::egui::{self, Stroke};

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) -> egui::Rect {
    egui::Panel::top("main_toolbar")
        .resizable(false)
        .show_separator_line(false)
        .min_size(36.0)
        .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(8, 4)))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.set_min_height(28.0);
                for s in TOOLBAR_LEFT {
                    slot(ui, ed, *s);
                }
                // Push the rendering group to the right edge when it fits on this row.
                let id = egui::Id::new("toolbar_right_w");
                let right_w: f32 = ui.ctx().data(|d| d.get_temp(id)).unwrap_or(0.0);
                let spare = ui.available_width() - right_w - 4.0;
                if spare > 0.0 {
                    ui.add_space(spare);
                }
                let start = ui.cursor().min.x;
                for s in TOOLBAR_RIGHT {
                    slot(ui, ed, *s);
                }
                let w = (ui.min_rect().max.x - start).max(0.0);
                if spare > 0.0 || right_w == 0.0 {
                    ui.ctx().data_mut(|d| d.insert_temp(id, w));
                }
            });
        })
        .response
        .rect
}

fn slot(ui: &mut egui::Ui, ed: &mut Editor, s: Slot) {
    match s {
        Slot::Sep => {
            ui.separator();
        }
        Slot::A(Action::SelectObject) => region_button(ui, ed),
        Slot::A(Action::Snap) => {
            let r = button(ui, ed, Action::Snap);
            r.context_menu(|ui| crate::ui::snaps::settings(ui, ed));
        }
        Slot::A(a) => {
            button(ui, ed, a);
        }
        Slot::Region => {
            egui::ComboBox::from_id_salt("region").width(78.0).selected_text(ed.region_shape.label()).show_ui(ui, |ui| {
                for shape in RegionShape::ALL {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 12.0), egui::Sense::hover());
                        region_icon(ui.painter(), rect, shape, theme::TEXT);
                        if ui.selectable_label(ed.region_shape == shape, shape.label()).clicked() {
                            ed.region_shape = shape;
                            ed.mode = TransformMode::Select;
                        }
                    });
                }
            })
            .response
            .on_hover_text("Selection region shape (Q cycles it)");
        }
        Slot::Filter => {
            let f = &mut ed.iface.select_filter;
            egui::ComboBox::from_id_salt("sel_filter").width(78.0).selected_text(f.label()).show_ui(ui, |ui| {
                for c in ObjCategory::ALL {
                    ui.selectable_value(f, c, c.label());
                }
            })
            .response
            .on_hover_text("Selection Filter: only these objects can be clicked or marquee-selected");
        }
        Slot::Coord => {
            egui::ComboBox::from_id_salt("coord").width(62.0).selected_text(format!("{:?}", ed.coord)).show_ui(ui, |ui| {
                for c in [CoordSystem::View, CoordSystem::World, CoordSystem::Local] {
                    ui.selectable_value(&mut ed.coord, c, format!("{c:?}"));
                }
            })
            .response
            .on_hover_text("Reference Coordinate System");
        }
        Slot::Pivot => {
            egui::ComboBox::from_id_salt("pivot").width(112.0).selected_text(pivot_label(ed.pivot_mode)).show_ui(ui, |ui| {
                for p in [PivotMode::PivotPoint, PivotMode::SelectionCenter, PivotMode::TransformCenter] {
                    ui.selectable_value(&mut ed.pivot_mode, p, pivot_label(p));
                }
            })
            .response
            .on_hover_text("Use Center: where Rotate and Scale pivot");
        }
        Slot::SkyMenu => {
            let current = SkyPreset::matching(&ed.scene.environment.sky).map_or("Sky", |p| p.label());
            ui.menu_button(format!("{current} ⏷"), |ui| {
                for p in SkyPreset::ALL {
                    menu_item(ui, ed, Action::Sky(p));
                }
            })
            .response
            .on_hover_text("One-click sky: day, sunset, overcast, night, deep space, nebula");
        }
    }
}

fn pivot_label(p: PivotMode) -> &'static str {
    match p {
        PivotMode::PivotPoint => "Pivot Point",
        PivotMode::SelectionCenter => "Selection Center",
        PivotMode::TransformCenter => "Transform Coord Center",
    }
}

/// Small icon of a selection region shape inside `r`.
pub fn region_icon(painter: &egui::Painter, r: egui::Rect, shape: RegionShape, color: egui::Color32) {
    let st = Stroke::new(1.3, color);
    let c = r.center();
    match shape {
        RegionShape::Rectangle => {
            painter.rect_stroke(r.shrink(1.0), 0.0, st, egui::StrokeKind::Inside);
        }
        RegionShape::Circle => {
            painter.circle_stroke(c, r.width().min(r.height()) * 0.5 - 1.0, st);
        }
        RegionShape::Fence => {
            let pts = [(0.0, 0.2), (0.6, 0.0), (1.0, 0.55), (0.45, 1.0), (0.1, 0.7), (0.0, 0.2)];
            let line = pts.iter().map(|&(x, y)| r.lerp_inside(egui::vec2(x, y))).collect();
            painter.add(egui::Shape::line(line, st));
        }
        RegionShape::Lasso => {
            let line = (0..=16)
                .map(|i| {
                    let a = i as f32 / 16.0 * std::f32::consts::TAU;
                    let k = 1.0 + 0.18 * (3.0 * a).sin();
                    c + egui::vec2(a.cos(), a.sin()) * r.width().min(r.height()) * 0.45 * k
                })
                .collect();
            painter.add(egui::Shape::line(line, st));
        }
    }
}

/// `Q` Select Object, with the current region shape drawn on the button
/// (`Q` again cycles it; right-click picks one).
fn region_button(ui: &mut egui::Ui, ed: &mut Editor) {
    let on = ed.mode == TransformMode::Select;
    let resp = action_ui::button_with(ui, ed, Action::SelectObject, "Q     ");
    let r = resp.rect;
    let icon = egui::Rect::from_center_size(egui::pos2(r.right() - 13.0, r.center().y), egui::vec2(12.0, 11.0));
    region_icon(ui.painter(), icon, ed.region_shape, if on { theme::INPUT } else { theme::TEXT });
    resp.context_menu(|ui| {
        for shape in RegionShape::ALL {
            if ui.selectable_label(ed.region_shape == shape, shape.label()).clicked() {
                ed.region_shape = shape;
                ed.mode = TransformMode::Select;
                ui.close();
            }
        }
    });
}
