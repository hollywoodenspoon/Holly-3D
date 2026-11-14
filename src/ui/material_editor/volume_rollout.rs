//! Volume shading model: preset, scattering, phase and glow.

use super::params::{color, num};
use crate::materials::Material;
use crate::materials::volume::{EmissionRamp, VolumePreset};
use crate::ui::widgets::{Track, dim};
use bevy_egui::egui;

pub(super) fn volume(ui: &mut egui::Ui, t: &mut Track, m: &mut Material) {
    ui.label("Preset");
    egui::ComboBox::from_id_salt("vol_preset").selected_text("Pick a look...").show_ui(ui, |ui| {
        for p in VolumePreset::ALL {
            if t.r(ui.selectable_label(false, p.label())).clicked() {
                m.volume = p.params();
            }
        }
    });
    ui.end_row();
    let v = &mut m.volume;
    color(ui, t, "Scatter Colour", &mut v.scatter);
    num(ui, t, "Absorption", &mut v.absorption, 0.0..=1.0, 0.0);
    num(ui, t, "Density (per m)", &mut v.density, 0.0..=100.0, 0.05);
    num(ui, t, "Forward g", &mut v.g_forward, -0.95..=0.95, 0.8);
    num(ui, t, "Back g", &mut v.g_back, -0.95..=0.95, -0.3);
    num(ui, t, "Back Lobe Blend", &mut v.blend, 0.0..=1.0, 0.2);
    ui.label("Glow Ramp");
    egui::ComboBox::from_id_salt("vol_ramp").selected_text(v.ramp.label()).show_ui(ui, |ui| {
        for r in EmissionRamp::ALL {
            t.r(ui.selectable_value(&mut v.ramp, r, r.label()));
        }
    });
    ui.end_row();
    ui.add_enabled_ui(v.ramp != EmissionRamp::None, |ui| ui.label("Glow Strength"));
    ui.add_enabled_ui(v.ramp != EmissionRamp::None, |ui| t.r(crate::ui::widgets::max_spinner_range(ui, &mut v.emission, 0.05, 0.0, 0.0..=100.0)));
    ui.end_row();
    num(ui, t, "Temperature Cooling", &mut v.cooling, 0.0..=1.0, 0.5);
    ui.label(dim("Renders as a volume in F9."));
    ui.label(dim("Clouds use their puffs."));
    ui.end_row();
}
