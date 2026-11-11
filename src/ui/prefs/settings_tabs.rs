//! Preferences -> Viewports, General, Rendering and Assets.

use super::reset_button;
use crate::editor::Editor;
use crate::prefs::{GeneralPrefs, RenderPrefs, Units, ViewportPrefs};
use crate::render::Engine;
use crate::ui::theme;
use crate::ui::widgets::dim;
use bevy_egui::egui::{self, RichText};

fn heading(ui: &mut egui::Ui, s: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(s).color(theme::amber()).strong());
}

pub fn viewports(ui: &mut egui::Ui, ed: &mut Editor) {
    let v = &mut ed.prefs.viewport;
    heading(ui, "Colours");
    egui::Grid::new("vp_colours").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label("Background");
        ui.horizontal(|ui| {
            ui.checkbox(&mut v.use_env_background, "Use Environment (8) colour");
            ui.add_enabled_ui(!v.use_env_background, |ui| ui.color_edit_button_rgb(&mut v.background));
        });
        ui.end_row();
        ui.label("Grid lines");
        ui.color_edit_button_rgb(&mut v.grid);
        ui.end_row();
        ui.label("Grid axes");
        ui.color_edit_button_rgb(&mut v.grid_axis);
        ui.end_row();
        ui.label("Vertices");
        ui.color_edit_button_srgb(&mut v.vertex);
        ui.end_row();
        ui.label("Selected vertices");
        ui.color_edit_button_srgb(&mut v.vertex_selected);
        ui.end_row();
    });
    heading(ui, "Navigation");
    ui.checkbox(&mut v.invert_zoom, "Invert zoom (wheel up zooms out)");
    for (label, val) in [("Zoom speed", &mut v.zoom_speed), ("Orbit speed", &mut v.orbit_speed), ("Pan speed", &mut v.pan_speed)] {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.add(egui::Slider::new(val, 0.25..=4.0).logarithmic(true).suffix("×"));
        });
    }
    if reset_button(ui, "viewport settings") {
        *v = ViewportPrefs::default();
    }
}

pub fn general(ui: &mut egui::Ui, ed: &mut Editor) {
    let g = &mut ed.prefs.general;
    heading(ui, "Autosave");
    ui.horizontal(|ui| {
        ui.label("Save a backup every");
        ui.add(egui::DragValue::new(&mut g.autosave_minutes).range(0..=120).suffix(" min"));
        ui.label(dim(if g.autosave_minutes == 0 { "(off)" } else { "(0 turns it off)" }));
    });
    ui.label(dim(&format!("Backups go to {}", Editor::autosave_path().display())));
    heading(ui, "Undo");
    ui.horizontal(|ui| {
        ui.label("Undo steps");
        ui.add(egui::DragValue::new(&mut g.undo_steps).range(1..=500));
    });
    heading(ui, "Units");
    egui::ComboBox::from_id_salt("units").selected_text(g.units.label()).show_ui(ui, |ui| {
        for u in Units::ALL {
            ui.selectable_value(&mut g.units, u, u.label());
        }
    });
    ui.label(dim("1 scene unit is 1 metre; this changes how lengths are shown (Measure, grid size)."));
    heading(ui, "Startup");
    ui.checkbox(&mut g.show_tour, "Show the Getting Started tour on startup");
    if reset_button(ui, "general settings") {
        *g = GeneralPrefs::default();
    }
}

pub fn rendering(ui: &mut egui::Ui, ed: &mut Editor) {
    let r = &mut ed.prefs.render;
    heading(ui, "Defaults for new sessions");
    egui::ComboBox::from_id_salt("pref_engine").width(260.0).selected_text(r.engine.label()).show_ui(ui, |ui| {
        for e in Engine::ALL {
            ui.selectable_value(&mut r.engine, e, e.label());
        }
    });
    ui.horizontal(|ui| {
        ui.label("Blender");
        ui.add(egui::TextEdit::singleline(&mut r.blender_exe).desired_width(260.0));
        if ui.button("Browse...").clicked()
            && let Some(p) = rfd::FileDialog::new().set_title("Find Blender").pick_file()
        {
            r.blender_exe = p.display().to_string();
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("Resolution");
        ui.add(egui::DragValue::new(&mut r.width).range(16..=8192));
        ui.label("×");
        ui.add(egui::DragValue::new(&mut r.height).range(16..=8192));
        for (w, h) in [(800, 600), (1280, 720), (1920, 1080), (2048, 2048), (3840, 2160)] {
            if ui.selectable_label((r.width, r.height) == (w, h), format!("{w}×{h}")).clicked() {
                (r.width, r.height) = (w, h);
            }
        }
    });
    ui.horizontal(|ui| {
        if ui.button("Use these now").on_hover_text("Copies them into Render Setup (F10) for this session").clicked() {
            ed.prefs.render.apply(&mut ed.render);
            ed.status = "Render Setup now uses your default renderer and resolution".into();
        }
        if reset_button(ui, "rendering defaults") {
            ed.prefs.render = RenderPrefs::default();
        }
    });
}
