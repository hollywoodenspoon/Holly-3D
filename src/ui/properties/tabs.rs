//! The vertical icon tab strip on the Properties column's left edge.

use crate::editor::{CommandTab, Editor};
use crate::ui::theme;
use bevy_egui::egui::{self, Color32};

/// (tab, icon, title, tooltip). A `None` row draws a divider between groups.
pub const TABS: [Option<(CommandTab, &str, &str, &str)>; 11] = [
    Some((CommandTab::Create, "✚", "Create", "Create: primitives, shapes, lights, cameras, mech")),
    Some((CommandTab::Modify, "🛠", "Modify", "Modify: name, colour, modifier stack and parameters")),
    Some((CommandTab::Hierarchy, "⌖", "Hierarchy", "Hierarchy: pivot and assembly")),
    Some((CommandTab::Display, "👁", "Display", "Display: hide, freeze and display properties")),
    Some((CommandTab::Utilities, "⚒", "Utilities", "Utilities: Reset XForm, Collapse, Measure")),
    None,
    Some((CommandTab::Render, "▶", "Render", "Render Setup (F10): engine, output size, sampling, effects")),
    Some((CommandTab::Environment, "☀", "Environment", "Environment (8): lighting rig, background, sky, weather, fog")),
    None,
    Some((CommandTab::Demiurge, "🌍", "Demiurge", "Demiurge: terrain, ocean, planets, smoke, settlements, buildings")),
    Some((CommandTab::Characters, "♀", "Characters", "Characters: humans, creatures and the Face Cloner")),
];

/// A tab's title for the body header.
pub fn title(tab: CommandTab) -> &'static str {
    TABS.iter().flatten().find(|t| t.0 == tab).map_or("", |t| t.2)
}

/// Halfway between the panel and the input fill: the strip's background.
fn strip_fill() -> Color32 {
    let (a, b) = (theme::panel(), theme::input());
    Color32::from_rgb(((a.r() as u16 + b.r() as u16) / 2) as u8, ((a.g() as u16 + b.g() as u16) / 2) as u8, ((a.b() as u16 + b.b() as u16) / 2) as u8)
}

pub fn strip(ui: &mut egui::Ui, ed: &mut Editor) {
    let full = ui.max_rect();
    ui.painter().rect_filled(full, 0.0, strip_fill());
    let mut y = full.min.y + 6.0;
    for row in TABS {
        let Some((tab, icon, name, tip)) = row else {
            let x = full.center().x;
            ui.painter().line_segment([egui::pos2(x - 11.0, y + 3.0), egui::pos2(x + 11.0, y + 3.0)], egui::Stroke::new(1.0, theme::border()));
            y += 8.0;
            continue;
        };
        let rect = egui::Rect::from_center_size(egui::pos2(full.center().x, y + 18.0), egui::vec2(36.0, 36.0));
        y += 40.0;
        let r = ui.interact(rect, ui.id().with(("tab", name)), egui::Sense::click()).on_hover_text(tip);
        let on = ed.tab == tab;
        let fill = if on {
            theme::card()
        } else if r.hovered() {
            theme::button_hover()
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, theme::R_CONTROL, fill);
        if on {
            let bar = egui::Rect::from_min_size(rect.left_top() + egui::vec2(0.0, 8.0), egui::vec2(2.0, 20.0));
            ui.painter().rect_filled(bar, 1.0, theme::accent());
        }
        let color = if on { theme::hover() } else if r.hovered() { theme::text() } else { theme::text_dim() };
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, icon, egui::FontId::proportional(16.0), color);
        if r.clicked() {
            ed.tab = tab;
        }
    }
}
