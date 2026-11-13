//! Draws Help pages: text, numbered steps, screenshots with numbered
//! circles, "Show me" buttons and the live hotkey cheat sheet.

use super::pages::{Block, Mark, Page, Show};
use super::shots::SHOTS;
use crate::editor::Editor;
use crate::ui::actions::Action;
use crate::ui::actions::keys::chord_of;
use crate::ui::hotkeys::keymap;
use crate::ui::theme;
use crate::ui::widgets::dim;
use bevy_egui::egui::{self, RichText, TextureHandle};
use std::collections::HashMap;

/// Decoded screenshots, uploaded once.
pub type Textures = HashMap<&'static str, TextureHandle>;

pub fn texture(ctx: &egui::Context, cache: &mut Textures, id: &'static str) -> Option<TextureHandle> {
    if let Some(t) = cache.get(id) {
        return Some(t.clone());
    }
    let bytes = SHOTS.iter().find(|s| s.0 == id)?.1;
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let ci = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    let t = ctx.load_texture(format!("help_{id}"), ci, egui::TextureOptions::LINEAR);
    cache.insert(id, t.clone());
    Some(t)
}

/// A screenshot `width` wide with numbered circles, then the legend.
pub fn shot(ui: &mut egui::Ui, cache: &mut Textures, id: &'static str, marks: &[Mark], width: f32) {
    let Some(tex) = texture(ui.ctx(), cache, id) else {
        ui.label(dim(&format!("(screenshot {id} missing)")));
        return;
    };
    ui.add_space(10.0);
    let [w, h] = tex.size().map(|v| v as f32);
    let size = egui::vec2(width, width * h / w);
    let r = ui.add(egui::Image::new(&tex).fit_to_exact_size(size).corner_radius(theme::R_CARD)).rect;
    let p = ui.painter();
    for (i, mk) in marks.iter().enumerate() {
        let c = r.min + egui::vec2(mk.at[0] * size.x, mk.at[1] * size.y);
        p.circle_stroke(c, 13.0, egui::Stroke::new(2.5, theme::amber()));
        badge(p, c + egui::vec2(12.0, -12.0), i + 1);
    }
    for (i, mk) in marks.iter().enumerate() {
        ui.horizontal_wrapped(|ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
            badge(ui.painter(), rect.center(), i + 1);
            ui.label(mk.label);
        });
    }
}

fn badge(p: &egui::Painter, c: egui::Pos2, n: usize) {
    p.circle_filled(c, 8.5, theme::amber());
    p.text(c, egui::Align2::CENTER_CENTER, n.to_string(), egui::FontId::proportional(11.0), theme::on_accent());
}

/// Runs a "Show me" button.
pub fn show_me(ed: &mut Editor, s: Show) {
    match s {
        Show::Act(a) => {
            if !a.checked(ed).unwrap_or(false) {
                a.run(ed);
            }
        }
        Show::Tab(t) => ed.tab = t,
    }
}

pub fn page(ui: &mut egui::Ui, ed: &mut Editor, cache: &mut Textures, p: &Page) {
    ui.label(RichText::new(p.title).heading().color(theme::amber()));
    let width = ui.available_width().min(820.0);
    for b in p.blocks {
        ui.add_space(4.0);
        match *b {
            Block::H(t) => {
                ui.add_space(4.0);
                ui.label(RichText::new(t).strong().size(15.0));
            }
            Block::P(t) => {
                ui.label(t);
            }
            Block::Tip(t) => {
                egui::Frame::new().fill(theme::card()).corner_radius(theme::R_CARD).inner_margin(8).show(ui, |ui| {
                    ui.set_width(width - 12.0);
                    ui.label(RichText::new(format!("💡 {t}")));
                });
            }
            Block::Steps(v) => {
                for (i, t) in v.iter().enumerate() {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(format!("{}.", i + 1)).color(theme::amber()).strong());
                        ui.label(*t);
                    });
                }
            }
            Block::Shot(id, marks) => shot(ui, cache, id, marks, width),
            Block::ShowMe(t, s) => {
                if ui.button(format!("▶ {t}")).clicked() {
                    show_me(ed, s);
                }
            }
            Block::Keys => cheat_sheet(ui),
        }
    }
}

fn cheat_sheet(ui: &mut egui::Ui) {
    let map = keymap();
    egui::Grid::new("help_keys").num_columns(4).striped(true).spacing([14.0, 4.0]).show(ui, |ui| {
        let keyed: Vec<_> = Action::all().into_iter().filter_map(|a| chord_of(&map, a).map(|c| (a, c))).collect();
        for (i, (a, c)) in keyed.iter().enumerate() {
            ui.label(RichText::new(c.text()).color(theme::amber()).monospace());
            ui.label(a.label());
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
}
