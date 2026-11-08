//! "Clean 3ds Max" egui theme: flat slate panels, Max amber for active state.

use bevy_egui::egui::{self, Color32, CornerRadius, Stroke, Vec2};

pub const PANEL: Color32 = Color32::from_rgb(0x22, 0x23, 0x27);
pub const CARD: Color32 = Color32::from_rgb(0x2B, 0x2D, 0x31);
pub const INPUT: Color32 = Color32::from_rgb(0x18, 0x19, 0x1C);
pub const AMBER: Color32 = Color32::from_rgb(0xE5, 0x9C, 0x24);
pub const BLUE: Color32 = Color32::from_rgb(0x3A, 0x7A, 0xFE);
pub const TEXT: Color32 = Color32::from_rgb(0xD6, 0xD8, 0xDD);
pub const TEXT_DIM: Color32 = Color32::from_rgb(0x8C, 0x90, 0x99);
pub const BUTTON: Color32 = Color32::from_rgb(0x36, 0x38, 0x3E);
pub const BUTTON_HOVER: Color32 = Color32::from_rgb(0x40, 0x43, 0x4A);
pub const BORDER: Color32 = Color32::from_rgb(0x3A, 0x3C, 0x42);
pub const VERTEX: Color32 = Color32::from_rgb(0x3D, 0x8B, 0xFF);
pub const VERTEX_SEL: Color32 = Color32::from_rgb(0xFF, 0x2E, 0x1F);

pub fn apply(ctx: &egui::Context) {
    let mut style = (*ctx.global_style()).clone();
    let r = CornerRadius::same(4);
    style.spacing.item_spacing = Vec2::new(6.0, 6.0);
    style.spacing.button_padding = Vec2::new(8.0, 5.0);
    style.spacing.interact_size = Vec2::new(40.0, 22.0);
    style.spacing.indent = 14.0;
    let v = &mut style.visuals;
    *v = egui::Visuals::dark();
    v.panel_fill = PANEL;
    v.window_fill = PANEL;
    v.faint_bg_color = CARD;
    v.extreme_bg_color = INPUT;
    v.code_bg_color = INPUT;
    v.window_corner_radius = CornerRadius::same(6);
    v.menu_corner_radius = r;
    v.window_stroke = Stroke::new(1.0, BORDER);
    v.override_text_color = Some(TEXT);
    v.hyperlink_color = BLUE;
    v.selection.bg_fill = AMBER;
    v.selection.stroke = Stroke::new(1.0, INPUT);
    v.striped = false;
    let w = &mut v.widgets;
    for wv in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        wv.corner_radius = r;
    }
    w.noninteractive.bg_fill = PANEL;
    w.noninteractive.weak_bg_fill = CARD;
    w.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    w.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    w.inactive.bg_fill = BUTTON;
    w.inactive.weak_bg_fill = BUTTON;
    w.inactive.bg_stroke = Stroke::NONE;
    w.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    w.hovered.bg_fill = BUTTON_HOVER;
    w.hovered.weak_bg_fill = BUTTON_HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0, BLUE);
    w.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    w.active.bg_fill = BLUE;
    w.active.weak_bg_fill = BLUE;
    w.active.bg_stroke = Stroke::new(1.0, BLUE);
    w.active.fg_stroke = Stroke::new(1.0, Color32::WHITE);
    w.open.bg_fill = CARD;
    w.open.weak_bg_fill = CARD;
    ctx.set_global_style(style);
}
