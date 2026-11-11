//! PaintBox mesh map baking and the 1-click Smart Mask layers.

use super::W;
use crate::editor::Editor;
use crate::paint::SmartKind;
use crate::ui::widgets::{Track, action, dim, max_spinner_range};
use bevy_egui::egui;

const BAKE_SIZES: [u32; 4] = [256, 512, 1024, 2048];

/// Amount / Contrast / Breakup of the active layer when it is a smart one.
fn smart_settings(ui: &mut egui::Ui, ed: &mut Editor) {
    let Some(s) = ed.paint_stack() else { return };
    let Some(smart) = s.active_layer().and_then(|l| l.smart.clone()) else { return };
    let i = s.active;
    let mut edit = smart.clone();
    let mut t = Track::default();
    ui.label(format!("✨ {}", smart.kind.label()));
    egui::Grid::new("paint_smart").num_columns(2).spacing([10.0, 6.0]).min_col_width(90.0).show(ui, |ui| {
        let mut pct = |ui: &mut egui::Ui, label: &str, v: &mut f32, default: f32| {
            let mut p = *v * 100.0;
            ui.label(label);
            t.r(max_spinner_range(ui, &mut p, 0.5, default, 0.0..=100.0));
            ui.end_row();
            *v = p / 100.0;
        };
        pct(ui, "Amount %", &mut edit.amount, 50.0);
        pct(ui, "Contrast %", &mut edit.contrast, 60.0);
        pct(ui, "Breakup %", &mut edit.breakup, 60.0);
        ui.label("Noise Scale");
        t.r(max_spinner_range(ui, &mut edit.scale, 0.05, 6.0, 0.5..=64.0));
        ui.end_row();
        ui.label("Seed");
        ui.horizontal(|ui| {
            t.r(max_spinner_range(ui, &mut edit.seed, 1.0, 1, 0..=u32::MAX));
            if ui.button("🎲").on_hover_text("New seed").clicked() {
                edit.seed = crate::scene::next_version() as u32;
                t.started = true;
                t.changed = true;
            }
        });
        ui.end_row();
    });
    if t.started {
        ed.checkpoint("Smart Mask");
    }
    if edit != smart
        && let Some(m) = ed.current_obj().and_then(|o| o.material)
        && let Some(s) = ed.scene.materials.mats[m].paint.as_mut()
    {
        s.layers[i].smart = Some(edit);
        ed.rebuild_smart_layer(i);
    }
}

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
    let has_obj = ed.current_obj().is_some_and(|o| !o.is_helper());
    ui.horizontal(|ui| {
        ui.label("Map Size");
        egui::ComboBox::from_id_salt("bake_size").width(100.0).selected_text(format!("{0}x{0}", ed.paint.bake.size)).show_ui(ui, |ui| {
            for s in BAKE_SIZES {
                ui.selectable_value(&mut ed.paint.bake.size, s, format!("{s}x{s}"));
            }
        });
        ui.label("AO Rays");
        max_spinner_range(ui, &mut ed.paint.bake.ao_rays, 1.0, 24, 1..=256);
    });
    if action(ui, "🔥 Bake Mesh Maps", has_obj, W) {
        ed.bake_mesh_maps();
    }
    let baked = ed.paint_stack().and_then(|s| s.mesh_maps.as_ref()).map(|m| m.size);
    ui.label(dim(&match baked {
        Some(n) => format!("Baked: Curvature, Cavity AO, World Normal, Z-Height at {n}x{n}. Export Textures saves them too."),
        None => "Curvature, Cavity AO, World Normal and Z-Height. Smart Masks bake them when needed.".to_string(),
    }));
    for kind in SmartKind::ALL {
        ui.horizontal_wrapped(|ui| {
            for (i, (name, _)) in kind.looks().iter().enumerate() {
                if ui.add_enabled(has_obj, egui::Button::new(format!("✨ {name}"))).on_hover_text(kind.label()).clicked() {
                    ed.add_smart_layer(kind, i);
                }
            }
        });
    }
    smart_settings(ui, ed);
}
