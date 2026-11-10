//! The bottom-right navigation buttons (Zoom, Zoom All, Zoom Extents,
//! FOV, Pan, Orbit, Maximize) and the left-drag they switch on.

use crate::editor::Editor;
use crate::ui::action_ui::sized_button;
use crate::ui::actions::layout::NAV_BUTTONS;
use crate::ui::theme;
use bevy::math::Vec2;
use bevy_egui::egui::{self, PointerButton};

const BTN: egui::Vec2 = egui::vec2(30.0, 24.0);

/// Draws the 4x2 button block in the bottom-right corner of `area`.
pub fn buttons(ui: &mut egui::Ui, ed: &mut Editor, area: egui::Rect) {
    egui::Area::new("nav_buttons".into())
        .order(egui::Order::Foreground)
        .pivot(egui::Align2::RIGHT_BOTTOM)
        .fixed_pos(area.right_bottom() - egui::vec2(6.0, 6.0))
        .show(ui.ctx(), |ui| {
            egui::Frame::new().fill(theme::PANEL).corner_radius(4).inner_margin(4).show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
                ui.spacing_mut().button_padding = egui::vec2(2.0, 2.0);
                for row in NAV_BUTTONS {
                    ui.horizontal(|ui| {
                        for a in row {
                            sized_button(ui, ed, a, &a.short(), BTN);
                        }
                    });
                }
            });
        });
}

/// Left-drag navigation while a nav button is on; right-click turns it off.
/// Returns true when the viewport input was used.
pub fn drag(ui: &egui::Ui, ed: &mut Editor, resp: &egui::Response, vi: usize, h: f32) -> bool {
    let Some(mode) = ed.iface.nav_mode else { return false };
    if resp.dragged_by(PointerButton::Primary) {
        let d = ui.input(|i| i.pointer.delta());
        ed.nav_drag(vi, mode, Vec2::new(d.x, d.y), h);
    }
    if resp.clicked_by(PointerButton::Secondary) {
        ed.iface.nav_mode = None;
    }
    if resp.hovered() {
        ui.ctx().set_cursor_icon(match mode {
            crate::editor::NavMode::Pan => egui::CursorIcon::Grab,
            crate::editor::NavMode::Orbit => egui::CursorIcon::AllScroll,
            _ => egui::CursorIcon::ResizeVertical,
        });
    }
    true
}
