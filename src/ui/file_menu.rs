//! File menu: New / Open / Save / Save As (`.ilex`; Open also reads legacy `.mmax`), Import and Export
//! (All or Selected) through native file dialogs.

use crate::editor::Editor;
use crate::io::{Format, scene_file};
use bevy_egui::egui;

type MenuItem = (&'static str, bool, fn(&mut Editor));

const IMPORT_EXTS: [&str; 4] = ["obj", "gltf", "glb", "fbx"];

/// Open lists `.ilex` and the pre-rename `.mmax` together; Save writes `.ilex`.
fn scene_dialog(ed: &Editor, open: bool) -> rfd::FileDialog {
    let d = rfd::FileDialog::new();
    let d = if open { d.add_filter("Ilex Scene (*.ilex, *.mmax)", &scene_file::ALL_EXTS) } else { d.add_filter("Ilex Scene (*.ilex)", &[scene_file::EXT]) };
    match ed.scene_path.as_ref().and_then(|p| p.parent()) {
        Some(dir) => d.set_directory(dir),
        None => d,
    }
}

/// Open Scene (`Ctrl+O`).
pub fn open(ed: &mut Editor) {
    if let Some(p) = scene_dialog(ed, true).add_filter("Importable meshes", &IMPORT_EXTS).pick_file() {
        let r = ed.open_any(&p);
        ed.report(r);
    }
}

/// Save (`Ctrl+S`): writes the current file, or asks like Save As.
pub fn save(ed: &mut Editor) {
    match ed.scene_path.clone() {
        Some(p) => {
            let r = ed.save_scene(&p);
            ed.report(r);
        }
        None => save_as(ed),
    }
}

pub fn save_as(ed: &mut Editor) {
    if let Some(p) = scene_dialog(ed, false).set_file_name("Untitled.ilex").save_file() {
        let r = ed.save_scene(&p);
        ed.report(r);
    }
}

pub fn import(ed: &mut Editor) {
    let mut d = rfd::FileDialog::new().add_filter("All supported", &IMPORT_EXTS);
    for f in Format::ALL {
        d = d.add_filter(f.label(), &[f.ext()]);
    }
    if let Some(p) = d.pick_file() {
        let r = ed.import_file(&p);
        ed.report(r);
    }
}

/// File -> Import VDB Volume…
pub fn import_vdb(ed: &mut Editor) {
    if let Some(p) = crate::ui::rollouts::vdb::pick_file() {
        let r = ed.import_vdb(&p).map(|_| ());
        ed.report(r);
    }
}

pub fn export(ed: &mut Editor, selected_only: bool) {
    let mut d = rfd::FileDialog::new();
    for f in [Format::Fbx, Format::Obj, Format::Glb, Format::Gltf] {
        d = d.add_filter(f.label(), &[f.ext()]);
    }
    if let Some(mut p) = d.set_file_name("Export.fbx").save_file() {
        if Format::from_path(&p).is_none() {
            p.set_extension("fbx");
        }
        let r = ed.export_file(&p, selected_only);
        ed.report(r);
    }
}

pub fn menu(ui: &mut egui::Ui, ed: &mut Editor) {
    let any_selected = ed.scene.objects.values().any(|o| o.selected);
    let items: [MenuItem; 7] = [
        ("New Scene", true, |ed| ed.new_scene()),
        ("Open...   Ctrl+O", true, open),
        ("Save   Ctrl+S", true, save),
        ("Save As...", true, save_as),
        ("Import (OBJ / glTF / FBX)...", true, import),
        ("Export All...", true, |ed| export(ed, false)),
        ("Export Selected...", any_selected, |ed| export(ed, true)),
    ];
    for (i, (label, enabled, action)) in items.into_iter().enumerate() {
        if i == 4 {
            ui.separator();
        }
        if ui.add_enabled(enabled, egui::Button::new(label)).clicked() {
            ui.close();
            action(ed);
        }
    }
}
