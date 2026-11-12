//! Ilex's egui theme: flat grey panels with one accent colour for active
//! state (blue-green in the default Graphite & Teal look). The colours come from Preferences -> Theme and can change while
//! the program runs, so panels read them through the accessors below.

use crate::prefs::{Palette, Rgb, ThemePrefs, ViewportPrefs};
use bevy_egui::egui::{self, Color32, CornerRadius, FontId, Stroke, TextStyle, Vec2};
use std::sync::RwLock;

#[derive(Clone, Copy)]
struct Live {
    pal: Palette,
    vertex: Rgb,
    vertex_sel: Rgb,
}

static LIVE: RwLock<Option<Live>> = RwLock::new(None);

fn live() -> Live {
    let l = *LIVE.read().unwrap_or_else(|e| e.into_inner());
    l.unwrap_or_else(|| {
        let v = ViewportPrefs::default();
        Live { pal: Palette::default(), vertex: v.vertex, vertex_sel: v.vertex_selected }
    })
}

fn c(rgb: Rgb) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

pub fn panel() -> Color32 {
    c(live().pal.panel)
}
pub fn card() -> Color32 {
    c(live().pal.card)
}
pub fn input() -> Color32 {
    c(live().pal.input)
}
/// Active-state accent (blue-green in the default theme).
pub fn accent() -> Color32 {
    c(live().pal.accent)
}
/// Hover / focus accent.
pub fn hover() -> Color32 {
    c(live().pal.hover)
}
/// Older name for [`accent`], kept while panels move over.
pub fn amber() -> Color32 {
    accent()
}
/// Older name for [`hover`], kept while panels move over.
pub fn blue() -> Color32 {
    hover()
}
pub fn text() -> Color32 {
    c(live().pal.text)
}
pub fn text_dim() -> Color32 {
    c(live().pal.text_dim)
}
pub fn button() -> Color32 {
    c(live().pal.button)
}
pub fn button_hover() -> Color32 {
    c(live().pal.button_hover)
}
pub fn border() -> Color32 {
    c(live().pal.border)
}
pub fn vertex() -> Color32 {
    c(live().vertex)
}
pub fn vertex_sel() -> Color32 {
    c(live().vertex_sel)
}

/// Text that stays readable on an accent-filled (active) button.
pub fn on_accent() -> Color32 {
    readable_on(live().pal.accent)
}

/// Dark text on light fills, white on dark ones.
fn readable_on(fill: Rgb) -> Color32 {
    if crate::prefs::palette::luminance(fill) > 0.35 { Color32::from_rgb(0x14, 0x15, 0x18) } else { Color32::WHITE }
}

/// Applies Preferences -> Theme (colours, UI size, font size) to egui.
pub fn apply(ctx: &egui::Context, t: &ThemePrefs, vp: &ViewportPrefs) {
    let pal = t.palette();
    *LIVE.write().unwrap_or_else(|e| e.into_inner()) = Some(Live { pal, vertex: vp.vertex, vertex_sel: vp.vertex_selected });
    ctx.set_zoom_factor(t.ui_scale.clamp(0.9, 1.5));
    let mut style = (*ctx.global_style()).clone();
    let fs = t.font_size.clamp(9.0, 20.0);
    style.text_styles = [
        (TextStyle::Small, FontId::proportional(fs * 0.72)),
        (TextStyle::Body, FontId::proportional(fs)),
        (TextStyle::Button, FontId::proportional(fs)),
        (TextStyle::Heading, FontId::new(fs * 1.45, super::fonts::heading_family(ctx))),
        (TextStyle::Monospace, FontId::monospace(fs)),
    ]
    .into();
    let r = CornerRadius::same(4);
    style.spacing.item_spacing = Vec2::new(6.0, 6.0);
    style.spacing.button_padding = Vec2::new(8.0, 5.0);
    style.spacing.interact_size = Vec2::new(40.0, 22.0);
    style.spacing.indent = 14.0;
    let (panel, card, input, accent, hover) = (c(pal.panel), c(pal.card), c(pal.input), c(pal.accent), c(pal.hover));
    let (text, button, button_hover, border) = (c(pal.text), c(pal.button), c(pal.button_hover), c(pal.border));
    let v = &mut style.visuals;
    *v = if pal.light { egui::Visuals::light() } else { egui::Visuals::dark() };
    v.panel_fill = panel;
    v.window_fill = panel;
    v.faint_bg_color = card;
    v.extreme_bg_color = input;
    v.code_bg_color = input;
    v.window_corner_radius = CornerRadius::same(6);
    v.menu_corner_radius = r;
    v.window_stroke = Stroke::new(1.0, border);
    // No global text override: selected (accent-filled) buttons use
    // `selection.stroke`, which picks dark or light text for readability.
    v.override_text_color = None;
    v.hyperlink_color = hover;
    v.selection.bg_fill = accent;
    v.selection.stroke = Stroke::new(1.0, on_accent());
    v.striped = false;
    let strong = if pal.light { Color32::BLACK } else { Color32::WHITE };
    let w = &mut v.widgets;
    for wv in [&mut w.noninteractive, &mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        wv.corner_radius = r;
    }
    w.noninteractive.bg_fill = panel;
    w.noninteractive.weak_bg_fill = card;
    w.noninteractive.bg_stroke = Stroke::new(1.0, border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, text);
    w.inactive.bg_fill = button;
    w.inactive.weak_bg_fill = button;
    w.inactive.bg_stroke = Stroke::NONE;
    w.inactive.fg_stroke = Stroke::new(1.0, text);
    w.hovered.bg_fill = button_hover;
    w.hovered.weak_bg_fill = button_hover;
    w.hovered.bg_stroke = Stroke::new(1.0, hover);
    w.hovered.fg_stroke = Stroke::new(1.0, strong);
    w.active.bg_fill = hover;
    w.active.weak_bg_fill = hover;
    w.active.bg_stroke = Stroke::new(1.0, hover);
    w.active.fg_stroke = Stroke::new(1.0, readable_on(pal.hover));
    w.open.bg_fill = card;
    w.open.weak_bg_fill = card;
    ctx.set_global_style(style);
}
