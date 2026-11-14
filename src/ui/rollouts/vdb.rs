//! Step C rollout: a VDB Volume (imported OpenVDB smoke and fire).

use super::spline::grid;
use super::worlds::row;
use crate::generators::vdb::{UpAxis, VdbParams, VdbStyle, float_grids};
use crate::ui::properties::content_w;
use crate::ui::widgets::{Track, dim};
use bevy_egui::egui;

/// Picks a .vdb file (the file dialog of File -> Import VDB Volume…).
pub fn pick_file() -> Option<String> {
    rfd::FileDialog::new().add_filter("OpenVDB volume", &["vdb"]).pick_file().map(|p| p.to_string_lossy().into_owned())
}

fn grid_combo(ui: &mut egui::Ui, t: &mut Track, id: &str, label: &str, value: &mut String, grids: &[String]) {
    ui.horizontal(|ui| {
        ui.label(label);
        let shown = if value.is_empty() { "(none)" } else { value.as_str() };
        egui::ComboBox::from_id_salt(id).width(content_w() - 110.0).selected_text(shown).show_ui(ui, |ui| {
            t.r(ui.selectable_value(value, String::new(), "(none)"));
            for g in grids {
                t.r(ui.selectable_value(value, g.clone(), g));
            }
        });
    });
}

/// The VDB Volume rollout.
pub fn vdb(ui: &mut egui::Ui, t: &mut Track, p: &mut VdbParams) {
    let name = std::path::Path::new(&p.path).file_name().map_or("(no file)".into(), |n| n.to_string_lossy().into_owned());
    ui.horizontal(|ui| {
        ui.label("File:");
        ui.label(egui::RichText::new(name).strong()).on_hover_text(&p.path);
    });
    let pick = ui.button("Pick VDB File…").on_hover_text("Choose another .vdb file; its density and temperature grids are picked automatically");
    if let Some(path) = pick.clicked().then(pick_file).flatten() {
        if let Ok(n) = VdbParams::for_file(&path) {
            *p = VdbParams { style: p.style, up: p.up, scale: p.scale, resolution: p.resolution, density: p.density, fire: p.fire, ..n };
        } else {
            p.path = path;
        }
        (t.started, t.changed) = (true, true);
    }
    let grids = float_grids(&p.path).unwrap_or_default();
    grid_combo(ui, t, "vdb_density", "Density Grid:", &mut p.density_grid, &grids);
    grid_combo(ui, t, "vdb_fire", "Fire Grid:", &mut p.fire_grid, &grids);
    ui.horizontal(|ui| {
        ui.label("Smoke Look:");
        egui::ComboBox::from_id_salt("vdb_style").width(content_w() - 110.0).selected_text(p.style.label()).show_ui(ui, |ui| {
            for s in VdbStyle::ALL {
                t.r(ui.selectable_value(&mut p.style, s, s.label()));
            }
        });
    });
    ui.horizontal(|ui| {
        ui.label("Up Axis:");
        t.r(ui.selectable_value(&mut p.up, UpAxis::Y, "Y (most simulators)"));
        t.r(ui.selectable_value(&mut p.up, UpAxis::Z, "Z"));
    });
    let d = VdbParams::default();
    grid(ui, "vdb_params", |ui| {
        row(ui, t, "Scale (m per unit):", &mut p.scale, 0.001..=1000.0, d.scale, 0.005);
        row(ui, t, "Max Resolution:", &mut p.resolution, 16..=384, d.resolution, 1.0);
        row(ui, t, "Smoke Density:", &mut p.density, 0.0..=100.0, d.density, 0.02);
        row(ui, t, "Fire Intensity (%):", &mut p.fire, 0.0..=100.0, d.fire, 0.5);
    });
    let problem = p.problem();
    if !problem.is_empty() {
        ui.colored_label(egui::Color32::from_rgb(230, 110, 90), problem);
        return;
    }
    let info: Vec<String> = [&p.density_grid, &p.fire_grid]
        .into_iter()
        .filter(|n| !n.is_empty())
        .filter_map(|n| crate::generators::vdb::grid(&p.path, n, p.up, p.resolution).ok().map(|g| (n, g)))
        .map(|(n, g)| format!("{n}: {} active voxels, resampled to {} x {} x {}", g.active, g.vox.dims[0], g.vox.dims[1], g.vox.dims[2]))
        .collect();
    ui.label(dim(&format!(
        "{}. The scene saves the file path, not the voxels; keep the .vdb next to the scene. Max Resolution trades detail for memory and render speed.",
        info.join("; ")
    )));
}
