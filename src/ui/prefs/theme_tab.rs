//! Preferences -> Theme: presets, custom colours, UI size and font size.

use super::reset_button;
use crate::editor::Editor;
use crate::prefs::{Palette, Rgb, ThemePrefs, ThemePreset};
use crate::ui::theme;
use crate::ui::widgets::dim;
use bevy_egui::egui::{self, RichText};

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
    let t = &mut ed.prefs.theme;
    ui.label(RichText::new("Theme").color(theme::amber()).strong());
    egui::Grid::new("theme_presets").num_columns(3).spacing([6.0, 6.0]).show(ui, |ui| {
        for (i, p) in ThemePreset::ALL.into_iter().enumerate() {
            if swatch_button(ui, t.preset == p, p, if p == ThemePreset::Custom { t.custom } else { p.palette() }).clicked() {
                if p == ThemePreset::Custom && t.preset != ThemePreset::Custom {
                    t.custom = t.palette();
                }
                t.preset = p;
            }
            if i % 3 == 2 {
                ui.end_row();
            }
        }
    });
    if t.preset == ThemePreset::Custom {
        ui.add_space(4.0);
        ui.label(RichText::new("Custom colours").color(theme::amber()));
        custom(ui, &mut t.custom);
    }
    let problems = t.palette().readability();
    if problems.is_empty() {
        ui.label(dim("✔ Text is readable in this theme."));
    } else {
        for p in problems {
            ui.label(RichText::new(format!("⚠ Hard to read: {p}")).color(theme::vertex_sel()));
        }
    }
    ui.add_space(6.0);
    ui.label(RichText::new("Size").color(theme::amber()).strong());
    let mut pct = (t.ui_scale * 100.0).round();
    ui.horizontal(|ui| {
        ui.label("UI size");
        if ui.add(egui::Slider::new(&mut pct, 90.0..=150.0).step_by(5.0).suffix(" %")).changed() {
            t.ui_scale = pct / 100.0;
        }
    });
    ui.horizontal(|ui| {
        ui.label("Font size");
        ui.add(egui::Slider::new(&mut t.font_size, 9.0..=20.0).step_by(0.5).suffix(" pt"));
    });
    ui.add_space(6.0);
    if reset_button(ui, "theme, UI size and font size") {
        *t = ThemePrefs::default();
    }
}

/// A preset button with three colour chips (panel, button, accent) as a strip under the name.
fn swatch_button(ui: &mut egui::Ui, on: bool, p: ThemePreset, pal: Palette) -> egui::Response {
    let r = ui.add_sized([170.0, 40.0], egui::Button::selectable(on, p.label()));
    let chip = |i: f32, c: Rgb| {
        let rect = egui::Rect::from_min_size(r.rect.center_bottom() + egui::vec2(-24.0 + i * 16.0, -9.0), egui::vec2(16.0, 5.0));
        ui.painter().rect_filled(rect, 1.0, egui::Color32::from_rgb(c[0], c[1], c[2]));
    };
    chip(0.0, pal.panel);
    chip(1.0, pal.button);
    chip(2.0, pal.accent);
    r.on_hover_text("Click to use this theme")
}

fn custom(ui: &mut egui::Ui, p: &mut Palette) {
    egui::Grid::new("theme_custom").num_columns(4).spacing([10.0, 6.0]).show(ui, |ui| {
        let rows: [(&str, &mut Rgb); 10] = [
            ("Panels", &mut p.panel),
            ("Cards / rollouts", &mut p.card),
            ("Fields / stack", &mut p.input),
            ("Accent (active)", &mut p.accent),
            ("Hover", &mut p.hover),
            ("Text", &mut p.text),
            ("Dim text", &mut p.text_dim),
            ("Buttons", &mut p.button),
            ("Buttons (hover)", &mut p.button_hover),
            ("Borders", &mut p.border),
        ];
        for (i, (label, c)) in rows.into_iter().enumerate() {
            ui.label(label);
            ui.color_edit_button_srgb(c);
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
    ui.checkbox(&mut p.light, "Light theme (dark text, light widgets)");
}
