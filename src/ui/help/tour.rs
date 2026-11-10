//! The Getting Started tour: a few screens that point at each zone.

use super::pages::{Mark, m};
use super::view::{Textures, shot};
use crate::editor::Editor;
use crate::ui::theme;
use bevy_egui::egui::{self, RichText};

pub struct Step {
    pub title: &'static str,
    pub text: &'static str,
    pub shot: &'static str,
    pub marks: &'static [Mark],
}

pub const STEPS: [Step; 8] = [
    Step {
        title: "Welcome to MiniMax",
        text: "MiniMax works like 3ds Max. This tour shows you where everything is. You can open it again from Help > Getting Started Tour.",
        shot: "layout",
        marks: &[m(40.0, 13.0, "Menus"), m(555.0, 42.0, "Main Toolbar"), m(1420.0, 116.0, "Command Panel")],
    },
    Step {
        title: "Make something",
        text: "On the Create tab, click Box, then drag in a viewport for the base, move the mouse up for the height and click.",
        shot: "create",
        marks: &[m(1369.0, 287.0, "Pick an object type"), m(1015.0, 700.0, "Drag here")],
    },
    Step {
        title: "Move, rotate and scale",
        text: "W, E and R (or the toolbar buttons) show the gizmo. Drag an axis or a square. Shift + drag clones.",
        shot: "transform",
        marks: &[m(555.0, 42.0, "Move, Rotate, Scale"), m(1015.0, 690.0, "The gizmo")],
    },
    Step {
        title: "Look around",
        text: "Middle mouse pans, Alt + middle mouse orbits, the wheel zooms. No middle button? Use the buttons at the bottom right.",
        shot: "layout",
        marks: &[m(825.0, 511.0, "View and shading menus"), m(1215.0, 858.0, "Navigation buttons")],
    },
    Step {
        title: "Change it with modifiers",
        text: "On the Modify tab, open the Modifier List and click one. Every modifier can be turned off or removed later.",
        shot: "modifiers",
        marks: &[m(1398.0, 188.0, "Modifier List"), m(1452.0, 230.0, "The stack")],
    },
    Step {
        title: "Model polygons",
        text: "Right-click an object > Convert to Editable Poly. Keys 1–5 pick vertices, edges, borders, polygons or elements.",
        shot: "editpoly",
        marks: &[m(1482.0, 474.0, "Sub-object levels"), m(1490.0, 702.0, "Tools")],
    },
    Step {
        title: "Materials, sky and rendering",
        text: "M opens the Material Editor, 8 the sky and environment, F9 renders. The Sky menu makes a sky in one click.",
        shot: "sky",
        marks: &[m(405.0, 75.0, "Sky menu"), m(260.0, 75.0, "Environment")],
    },
    Step {
        title: "Make it yours",
        text: "Preferences (⚙) changes the theme and every hotkey. Help (F1) explains any part of the screen: point at it and press F1.",
        shot: "prefs",
        marks: &[m(836.0, 75.0, "Preferences")],
    },
];

pub fn show(ctx: &egui::Context, ed: &mut Editor, cache: &mut Textures) {
    let Some(i) = ed.iface.tour else { return };
    let i = i.min(STEPS.len() - 1);
    let s = &STEPS[i];
    let mut open = true;
    let mut next = Some(i);
    egui::Window::new("Getting Started")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(700.0)
        .pivot(egui::Align2::CENTER_CENTER)
        .default_pos(ctx.content_rect().center())
        .show(ctx, |ui| {
            ui.label(RichText::new(s.title).heading().color(theme::amber()));
            ui.label(s.text);
            shot(ui, cache, s.shot, s.marks, 680.0);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.add_enabled(i > 0, egui::Button::new("◀ Back")).clicked() {
                    next = Some(i - 1);
                }
                ui.label(format!("{} / {}", i + 1, STEPS.len()));
                if i + 1 < STEPS.len() {
                    if ui.button("Next ▶").clicked() {
                        next = Some(i + 1);
                    }
                } else if ui.button("✔ Finish").clicked() {
                    next = None;
                }
                ui.checkbox(&mut ed.prefs.general.show_tour, "Show this tour at startup");
                if ui.button("Open Help").clicked() {
                    ed.iface.help_open = true;
                    next = None;
                }
            });
        });
    ed.iface.tour = if open { next } else { None };
}

#[cfg(test)]
mod tests {
    #[test]
    fn tour_shots_exist() {
        for s in super::STEPS {
            assert!(crate::ui::help::shots::SHOTS.iter().any(|x| x.0 == s.shot), "{}", s.shot);
        }
    }
}
