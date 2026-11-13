//! MakeHuman window → Library tab: import hair, clothes and other
//! MakeHuman assets from a folder, `.zip` or `.mhclo`, and remove them.

use crate::character::makehuman::library::{pretty, Category};
use crate::character::makehuman::user_lib;
use crate::ui::widgets::dim;
use bevy_egui::egui;

#[derive(Clone, Default)]
struct LibUi {
    /// None = guess from the folder names and tags.
    force: Option<Category>,
    msg: String,
    /// Item waiting for a second click on Remove.
    confirm: Option<(Category, String)>,
}

fn run(ui_state: &mut LibUi, src: Option<std::path::PathBuf>) {
    let Some(src) = src else { return };
    ui_state.msg = match user_lib::import(&src, ui_state.force) {
        Ok(added) => {
            let list: Vec<String> = added.iter().map(|(c, n)| format!("{} ({})", pretty(n), c.label())).collect();
            format!("Imported: {}", list.join(", "))
        }
        Err(e) => format!("Could not import {}: {e}", src.display()),
    };
}

pub fn show(ui: &mut egui::Ui) {
    let id = egui::Id::new("mh_library_ui");
    let mut st: LibUi = ui.data_mut(|d| d.get_temp(id).unwrap_or_default());
    ui.label(egui::RichText::new("Import MakeHuman Assets").strong());
    ui.label(dim("Hair, clothes, eyebrows, eyelashes, teeth, tongue or body proxies made for MakeHuman 1.x (a folder with the .mhclo, .obj, .mhmat and textures, or a .zip of it). Check each asset's licence before you share renders or models."));
    ui.horizontal(|ui| {
        ui.label("Import as:");
        let shown = st.force.map_or("Auto-detect", Category::label);
        egui::ComboBox::from_id_salt("mh_import_as").selected_text(shown).show_ui(ui, |ui| {
            ui.selectable_value(&mut st.force, None, "Auto-detect");
            for c in Category::ALL {
                ui.selectable_value(&mut st.force, Some(c), c.label());
            }
        });
    });
    ui.horizontal_wrapped(|ui| {
        if ui.button("Import Folder…").clicked() {
            run(&mut st, rfd::FileDialog::new().set_title("MakeHuman asset folder").pick_folder());
        }
        if ui.button("Import .zip / .mhclo…").clicked() {
            run(&mut st, rfd::FileDialog::new().add_filter("MakeHuman asset (*.zip, *.mhclo, *.proxy)", &["zip", "mhclo", "proxy"]).pick_file());
        }
    });
    if !st.msg.is_empty() {
        ui.label(st.msg.as_str());
    }
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Your Imported Assets").strong());
    ui.label(dim(&format!("Kept in {}", user_lib::root().display())));
    let mut any = false;
    for cat in Category::ALL {
        for name in user_lib::names(cat) {
            any = true;
            ui.horizontal(|ui| {
                ui.label(format!("{} · {}", cat.label(), pretty(&name)));
                let armed = st.confirm.as_ref() == Some(&(cat, name.clone()));
                let label = if armed { "Really remove?" } else { "Remove" };
                if ui.button(label).on_hover_text("Deletes Ilex's copy; the files you imported from are not touched").clicked() {
                    if armed {
                        st.msg = match user_lib::remove(cat, &name) {
                            Ok(()) => format!("Removed {}", pretty(&name)),
                            Err(e) => format!("Could not remove {}: {e}", pretty(&name)),
                        };
                        st.confirm = None;
                    } else {
                        st.confirm = Some((cat, name.clone()));
                    }
                }
            });
        }
    }
    if !any {
        ui.label(dim("Nothing imported yet."));
    }
    ui.data_mut(|d| d.insert_temp(id, st));
}
