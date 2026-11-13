//! The one-row header: logo and menus on the left, the transform tools in
//! the middle (Undo/Redo, Q W E R, coordinate system, pivot, snaps and a
//! `»` menu with everything else), quick buttons on the right (Materials,
//! MakeHuman, Render Setup, Render). Built from `actions::layout`; the row
//! wraps on narrow windows instead of hiding buttons.

use super::action_ui::{self, menu_entries, menu_item};
use super::actions::Action;
use super::actions::layout::{HEADER_RIGHT, HEADER_TOOLS, MENUS, OVERFLOW, Slot};
use super::menubar::{create_menu, modifiers_menu};
use super::theme;
use crate::editor::{CoordSystem, Editor, ObjCategory, PivotMode, TransformMode};
use crate::render::sky_presets::SkyPreset;
use crate::viewport::region::RegionShape;
use bevy_egui::egui::{self, RichText, Stroke};

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) -> egui::Rect {
    egui::Panel::top("header")
        .resizable(false)
        .show_separator_line(false)
        .min_size(32.0)
        .frame(egui::Frame::new().fill(theme::panel()).inner_margin(egui::Margin::symmetric(8, 3)))
        .show(ui, |ui| {
            let right_edge = ui.max_rect().max.x;
            ui.horizontal_wrapped(|ui| {
                ui.set_min_height(26.0);
                ui.spacing_mut().item_spacing.x = 4.0;
                menus(ui, ed);
                ui.add_space(10.0);
                for s in HEADER_TOOLS {
                    slot(ui, ed, *s);
                }
                ui.menu_button("»", |ui| overflow(ui, ed)).response.on_hover_text("More tools");
                right_group(ui, ed, right_edge);
            });
        })
        .response
        .rect
}

fn menus(ui: &mut egui::Ui, ed: &mut Editor) {
    super::logo::mark(ui, 16.0).on_hover_text("Ilex");
    ui.label(RichText::new("Ilex").color(theme::accent()).strong());
    ui.add_space(4.0);
    for (title, entries) in MENUS {
        if *title == "Rendering" {
            ui.menu_button("Create", |ui| create_menu(ui, ed));
            ui.menu_button("Modifiers", |ui| modifiers_menu(ui, ed));
        }
        ui.menu_button(*title, |ui| menu_entries(ui, ed, entries));
    }
}

/// Materials · MakeHuman · Setup · Render, right-aligned when they fit on
/// this row, otherwise inline (the row wraps). Their width is remembered
/// from the last right-aligned frame. `right_edge` is the panel's own edge:
/// a wrapped row's max rect grows with its contents, so it can't be used.
fn right_group(ui: &mut egui::Ui, ed: &mut Editor, right_edge: f32) {
    let id = egui::Id::new("header_right_w");
    let need: f32 = ui.ctx().data(|d| d.get_temp(id)).unwrap_or(380.0);
    let room = right_edge - ui.cursor().min.x;
    if room < need {
        for s in HEADER_RIGHT {
            slot(ui, ed, *s);
        }
        return;
    }
    let r = ui.allocate_ui_with_layout(egui::vec2(room, 26.0), egui::Layout::right_to_left(egui::Align::Center), |ui| {
        for s in HEADER_RIGHT.iter().rev() {
            slot(ui, ed, *s);
        }
        ui.min_rect().width()
    });
    ui.ctx().data_mut(|d| d.insert_temp(id, r.inner + 4.0));
}

/// The short caption a header button shows (the full name is in its tooltip).
pub fn caption(a: Action) -> String {
    match a {
        Action::SelectObject => "Q".into(),
        Action::Move => "W".into(),
        Action::Rotate => "E".into(),
        Action::Scale => "R".into(),
        Action::Snap => "S".into(),
        Action::AngleSnap => "A°".into(),
        Action::PercentSnap => "%".into(),
        Action::MaterialEditor => "◑ Materials".into(),
        Action::CharacterStudio => "♀ MakeHuman".into(),
        Action::RenderSetup => "⚙ Setup".into(),
        _ => a.short(),
    }
}

fn slot(ui: &mut egui::Ui, ed: &mut Editor, s: Slot) {
    match s {
        Slot::Sep => {
            ui.add_space(6.0);
        }
        Slot::Group(actions) => group(ui, ed, actions),
        Slot::A(Action::Render) => {
            let busy = ed.is_rendering();
            let text = RichText::new(caption(Action::Render)).color(theme::on_accent()).strong();
            let b = egui::Button::new(text).fill(theme::accent());
            if ui.add_enabled(!busy, b).on_hover_text(action_ui::tooltip(Action::Render)).clicked() {
                Action::Render.run(ed);
            }
        }
        Slot::A(a) => {
            action_ui::button_with(ui, ed, a, &caption(a));
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
        Slot::Region => {
            ui.menu_button(format!("Region: {}", ed.region_shape.label()), |ui| {
                for shape in RegionShape::ALL {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 12.0), egui::Sense::hover());
                        region_icon(ui.painter(), rect, shape, theme::text());
                        if ui.selectable_label(ed.region_shape == shape, shape.label()).clicked() {
                            ed.region_shape = shape;
                            ed.mode = TransformMode::Select;
                        }
                    });
                }
            });
        }
        Slot::Filter => {
            ui.menu_button(format!("Selection Filter: {}", ed.iface.select_filter.label()), |ui| {
                for c in ObjCategory::ALL {
                    ui.selectable_value(&mut ed.iface.select_filter, c, c.label());
                }
            });
        }
        Slot::SkyMenu => {
            let current = SkyPreset::matching(&ed.scene.environment.sky).map_or("Off", |p| p.label());
            ui.menu_button(format!("Sky: {current}"), |ui| {
                for p in SkyPreset::ALL {
                    menu_item(ui, ed, Action::Sky(p));
                }
            });
        }
    }
}

/// Buttons drawn touching each other on one dark rounded strip (Q W E R, snaps).
fn group(ui: &mut egui::Ui, ed: &mut Editor, actions: &[Action]) {
    egui::Frame::new().fill(theme::input()).corner_radius(theme::R_CONTROL).inner_margin(egui::Margin::same(2)).show(ui, |ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        ui.spacing_mut().button_padding = egui::vec2(8.0, 3.0);
        for &a in actions {
            let r = action_ui::button_with(ui, ed, a, &caption(a));
            if a == Action::SelectObject {
                let icon = egui::Rect::from_center_size(r.rect.right_bottom() - egui::vec2(4.0, 4.0), egui::vec2(6.0, 5.0));
                let on = ed.mode == TransformMode::Select;
                region_icon(ui.painter(), icon, ed.region_shape, if on { theme::on_accent() } else { theme::text_dim() });
                r.context_menu(|ui| {
                    for shape in RegionShape::ALL {
                        if ui.selectable_label(ed.region_shape == shape, shape.label()).clicked() {
                            ed.region_shape = shape;
                            ed.mode = TransformMode::Select;
                            ui.close();
                        }
                    }
                });
            } else if a == Action::Snap {
                r.context_menu(|ui| crate::ui::snaps::settings(ui, ed));
            }
        }
    });
}

/// The `»` menu: every tool that has no room on the header row.
fn overflow(ui: &mut egui::Ui, ed: &mut Editor) {
    for s in OVERFLOW {
        match *s {
            Slot::A(a) => menu_item(ui, ed, a),
            Slot::Sep => {
                ui.separator();
            }
            other => slot(ui, ed, other),
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
