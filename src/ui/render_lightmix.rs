//! F9 "LightMix" sidebar: one fader strip per light group (sun & sky,
//! environment, emissive, godrays, each scene light) with On, Solo,
//! Intensity and Tint. Changes re-mix the finished render at once.

use crate::editor::Editor;
use crate::ui::widgets::dim;
use bevy_egui::egui;

pub fn sidebar(ui: &mut egui::Ui, ed: &mut Editor) {
    egui::Panel::right("rfw_lightmix").resizable(false).exact_size(290.0).show(ui, |ui| {
        ui.heading("Luxon LightMix");
        let Some(mix) = ed.rfw.lightmix.as_mut() else {
            ui.label(dim("Render with the Veach Path Tracer on the CPU or GPU (F10 > Path Tracing, LightMix on) to relight the image here without re-rendering."));
            return;
        };
        ui.label(dim("Re-weights each light group's buffer — no re-render."));
        let mut changed = false;
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (i, (g, f)) in mix.groups.iter().zip(mix.faders.iter_mut()).enumerate() {
                if i >= crate::render::lightmix::FIRST_LIGHT as usize || mix.data.iter().skip(i).step_by(mix.groups.len()).any(|c| c.max_element() > 0.0) {
                    changed |= strip(ui, &g.name, f);
                }
            }
        });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("↺ Reset Mix").on_hover_text("All groups on, intensity 1, white tint").clicked() {
                mix.reset();
                changed = true;
            }
        });
        if changed {
            ed.remix_render();
        }
        if ui.button("💡 Apply Luxon LightMix to Scene Lights").on_hover_text("Bake the faders into each light's Multiplier and Color (undoable)").clicked() {
            ed.apply_lightmix();
        }
    });
}

/// One group's controls; true when anything changed.
fn strip(ui: &mut egui::Ui, name: &str, f: &mut crate::render::lightmix::Fader) -> bool {
    let mut changed = false;
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.horizontal(|ui| {
            changed |= ui.checkbox(&mut f.on, "").on_hover_text("Group on / off").changed();
            ui.strong(name);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                changed |= ui.toggle_value(&mut f.solo, "Solo").on_hover_text("Show only soloed groups").changed();
                changed |= ui.color_edit_button_rgb(&mut f.tint).on_hover_text("Tint").changed();
            });
        });
        let mut r = ui.add(egui::Slider::new(&mut f.intensity, 0.0..=10.0).text("Intensity").logarithmic(true).max_decimals(2));
        if r.secondary_clicked() {
            f.intensity = 1.0;
            r.mark_changed();
        }
        changed |= r.on_hover_text("Right-click resets to 1").changed();
    });
    changed
}
