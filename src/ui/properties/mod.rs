//! Right Properties column: the Scene Explorer on top, a drag bar, then a
//! vertical icon tab strip beside the tab body (Create · Modify · Hierarchy ·
//! Display · Utilities | Render · Environment | Demiurge · Characters). Drag
//! its left edge to resize it (260–640 px); the width and the Explorer's
//! height are remembered in Preferences.

mod assembly;
mod camera_params;
mod characters;
mod create;
mod demiurge;
mod display;
mod light_params;
mod modifier_list;
mod modify;
mod params;
mod pivot;
mod stack;
mod tabs;
mod utilities;

pub use tabs::TABS;

use super::UiState;
use super::theme;
use crate::editor::{CommandTab, Editor};
use crate::prefs::LayoutPrefs;
use bevy_egui::egui::{self, UiBuilder};
use std::sync::atomic::{AtomicU32, Ordering};

/// Width of the vertical tab strip.
const STRIP_W: f32 = 44.0;
/// Width of the content inside the tab body before the first frame.
const FALLBACK_W: f32 = 264.0;

static CONTENT: AtomicU32 = AtomicU32::new(0);

/// Usable content width inside the tab body this frame (it follows the
/// column's width; rollouts size their buttons from it).
pub fn content_w() -> f32 {
    match CONTENT.load(Ordering::Relaxed) {
        0 => FALLBACK_W,
        bits => f32::from_bits(bits),
    }
}

fn set_content_w(w: f32) {
    CONTENT.store(w.max(120.0).to_bits(), Ordering::Relaxed);
}

/// Content width for a column `width` px wide.
pub fn content_for(width: f32) -> f32 {
    width - STRIP_W - 34.0
}

pub fn show(ui: &mut egui::Ui, ed: &mut Editor, st: &mut UiState) -> egui::Rect {
    let width = ed.prefs.layout.properties_width.clamp(LayoutPrefs::MIN_W, LayoutPrefs::MAX_W);
    let rect = egui::Panel::right("properties")
        .resizable(true)
        .show_separator_line(false)
        .default_size(width)
        .size_range(LayoutPrefs::MIN_W..=LayoutPrefs::MAX_W)
        .frame(egui::Frame::new().fill(theme::panel()))
        .show(ui, |ui| {
            if ed.explorer_visible {
                explorer_split(ui, ed);
            }
            let rest = ui.available_rect_before_wrap();
            let strip = egui::Rect::from_min_max(rest.min, egui::pos2(rest.min.x + STRIP_W, rest.max.y));
            let body = egui::Rect::from_min_max(egui::pos2(strip.max.x, rest.min.y), rest.max);
            tabs::strip(&mut clipped(ui, strip), ed);
            body_ui(&mut clipped(ui, body.shrink2(egui::vec2(8.0, 4.0))), ed, st);
            ui.allocate_rect(rest, egui::Sense::hover());
        })
        .response
        .rect;
    remember_width(ui, ed, rect.width());
    rect
}

/// A child Ui confined to `r`: what it draws is clipped there and never
/// widens the column (so wide rollouts can't push the column wider).
fn clipped(ui: &mut egui::Ui, r: egui::Rect) -> egui::Ui {
    let mut child = ui.new_child(UiBuilder::new().max_rect(r));
    child.set_clip_rect(r.intersect(ui.clip_rect()));
    child
}

/// The Scene Explorer and the drag bar under it.
fn explorer_split(ui: &mut egui::Ui, ed: &mut Editor) {
    let avail = ui.available_rect_before_wrap();
    let max_h = (avail.height() - 220.0).max(LayoutPrefs::MIN_EXPLORER);
    let h = ed.prefs.layout.explorer_height.clamp(LayoutPrefs::MIN_EXPLORER, max_h);
    let top = egui::Rect::from_min_size(avail.min, egui::vec2(avail.width(), h));
    super::explorer::show(&mut clipped(ui, top.shrink2(egui::vec2(8.0, 6.0))), ed);
    let bar = egui::Rect::from_min_size(egui::pos2(avail.min.x, top.max.y), egui::vec2(avail.width(), 8.0));
    let r = ui.interact(bar, ui.id().with("explorer_bar"), egui::Sense::drag()).on_hover_cursor(egui::CursorIcon::ResizeVertical);
    let grip = if r.hovered() || r.dragged() { theme::accent() } else { theme::border() };
    ui.painter().line_segment([bar.left_top(), bar.right_top()], egui::Stroke::new(1.0, theme::card()));
    ui.painter().rect_filled(egui::Rect::from_center_size(bar.center(), egui::vec2(32.0, 3.0)), 2.0, grip);
    if r.dragged() {
        ed.prefs.layout.explorer_height = (h + r.drag_delta().y).clamp(LayoutPrefs::MIN_EXPLORER, max_h);
    }
    if r.drag_stopped() {
        ed.iface.layout_dirty = true;
    }
    ui.allocate_rect(egui::Rect::from_min_max(avail.min, bar.max), egui::Sense::hover());
}

/// The selected tab's title and contents.
fn body_ui(ui: &mut egui::Ui, ed: &mut Editor, st: &mut UiState) {
    set_content_w(ui.available_width() - 18.0);
    ui.horizontal(|ui| {
        ui.set_min_height(28.0);
        ui.label(egui::RichText::new(tabs::title(ed.tab)).size(14.0).strong());
    });
    egui::ScrollArea::vertical().id_salt("props_body").auto_shrink([false, false]).show(ui, |ui| {
        ui.set_max_width(content_w() + 16.0);
        ui.spacing_mut().button_padding = egui::vec2(5.0, 4.0);
        match ed.tab {
            CommandTab::Create => create::show(ui, ed),
            CommandTab::Modify => modify::show(ui, ed, st),
            CommandTab::Hierarchy => pivot::show(ui, ed),
            CommandTab::Display => display::show(ui, ed),
            CommandTab::Utilities => utilities::show(ui, ed),
            CommandTab::Render => super::render_setup::body(ui, ed, &mut st.rfw.setup_tab),
            CommandTab::Environment => super::env_dialog::body(ui, ed),
            CommandTab::Demiurge => demiurge::show(ui, ed),
            CommandTab::Characters => characters::show(ui, ed),
        }
    });
}

/// Keeps the dragged width in Preferences and saves it once the mouse is up.
fn remember_width(ui: &egui::Ui, ed: &mut Editor, w: f32) {
    if (w - ed.prefs.layout.properties_width).abs() > 0.5 {
        ed.prefs.layout.properties_width = w;
        ed.iface.layout_dirty = true;
    }
    if ed.iface.layout_dirty && !ui.input(|i| i.pointer.any_down()) {
        ed.iface.layout_dirty = false;
        if let Err(e) = ed.prefs.save_to(&crate::prefs::Preferences::default_path()) {
            ed.status = e;
        }
    }
}
