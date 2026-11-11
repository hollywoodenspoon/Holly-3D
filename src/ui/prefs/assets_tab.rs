//! Preferences -> Assets: the download folder and the three CC0 packs,
//! each with a checkbox, Download / Cancel and Remove buttons and a
//! progress bar.

use crate::assets::{Pack, PackState, Sources};
use crate::editor::Editor;
use crate::prefs::AssetPrefs;
use crate::ui::theme;
use crate::ui::widgets::dim;
use bevy_egui::egui::{self, RichText};

fn heading(ui: &mut egui::Ui, s: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(s).color(theme::amber()).strong());
}

fn mb(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
}

/// The pack's checkbox in Preferences.
fn wanted(a: &mut AssetPrefs, pack: Pack) -> &mut bool {
    match pack {
        Pack::Photos => &mut a.photos,
        Pack::LowPoly => &mut a.lowpoly,
        Pack::Realistic => &mut a.realistic,
    }
}

fn pack_row(ui: &mut egui::Ui, ed: &mut Editor, pack: Pack) {
    let progress = ed.downloads.get(&pack).map(|j| j.snapshot());
    let state = ed.pack_state(pack);
    ui.horizontal(|ui| {
        ui.checkbox(wanted(&mut ed.prefs.assets, pack), RichText::new(pack.label()).strong()).on_hover_text("Include this pack in Download Selected");
        ui.label(dim(pack.size_hint()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let installed = !matches!(state, PackState::Missing);
            if ui.add_enabled(installed || progress.is_some(), egui::Button::new("Remove")).on_hover_text("Delete this pack from the asset folder").clicked() {
                ed.remove_pack(pack);
            }
            if progress.is_some() {
                if ui.button("Cancel").clicked() {
                    ed.cancel_download(pack);
                }
            } else {
                let label = match state {
                    PackState::Missing => "Download",
                    PackState::Partial { .. } => "Resume",
                    PackState::Installed(_) => "Update",
                };
                if ui.button(label).on_hover_text("Fetch the pack into the asset folder").clicked() {
                    ed.download_pack(pack);
                }
            }
        });
    });
    ui.label(dim(pack.about()));
    match (&progress, state) {
        (Some(p), _) => {
            let text = if p.files_total == 0 { p.note.clone() } else { format!("{} / {} files · {} · {}", p.files_done, p.files_total, mb(p.bytes), p.note) };
            ui.add(egui::ProgressBar::new(p.fraction()).text(text).animate(true));
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(200));
        }
        (None, PackState::Installed(i)) => {
            ui.add(egui::ProgressBar::new(1.0).text(format!("Installed: {} files, {}", i.files, mb(i.bytes))));
        }
        (None, PackState::Partial { files }) => {
            ui.add(egui::ProgressBar::new(0.5).text(format!("Partly downloaded ({files} files). Resume to finish.")));
        }
        (None, PackState::Missing) => {
            ui.add(egui::ProgressBar::new(0.0).text("Not downloaded"));
        }
    }
    ui.add_space(6.0);
}

pub fn assets(ui: &mut egui::Ui, ed: &mut Editor) {
    heading(ui, "Download folder");
    ui.horizontal(|ui| {
        let a = &mut ed.prefs.assets;
        ui.add(egui::TextEdit::singleline(&mut a.folder).desired_width(320.0));
        if ui.button("Browse...").clicked()
            && let Some(p) = rfd::FileDialog::new().set_title("Asset folder").pick_folder()
        {
            a.folder = p.display().to_string();
        }
    });
    ui.label(dim("Packs are kept here, outside your projects. All three are CC0: free, no credit required."));
    heading(ui, "Packs (free, CC0)");
    for pack in Pack::ALL {
        pack_row(ui, ed, pack);
    }
    ui.horizontal(|ui| {
        if ui.button("Download Selected").on_hover_text("Download every ticked pack that is not installed yet").clicked() {
            for pack in Pack::ALL {
                if *wanted(&mut ed.prefs.assets, pack) && !matches!(ed.pack_state(pack), PackState::Installed(_)) {
                    ed.download_pack(pack);
                }
            }
        }
        ui.label("Photo size");
        let res = &mut ed.prefs.assets.sources.resolution;
        for (value, label) in [("1k", "1K"), ("2k", "2K")] {
            if ui.selectable_label(res == value, label).on_hover_text("Resolution of downloaded photo textures and model textures").clicked() {
                *res = value.to_string();
            }
        }
    });
    ui.label(dim("Photo materials appear in the Material Library (filter: Procedural | Photo | Both) once the photo pack is installed."));
    if ui.button("Reset asset settings").on_hover_text("Default folder, hosts and pack choices").clicked() {
        ed.prefs.assets = AssetPrefs::default();
        ed.prefs.assets.sources = Sources::default();
    }
}
