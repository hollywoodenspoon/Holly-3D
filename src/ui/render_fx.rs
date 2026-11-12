//! Render Setup (F10) "GI & Post-FX" tab: the GI & AO rollout and the
//! Post-FX rollout (bloom, depth of field, tonemapping); plus the live tone
//! row (operator, Exposure EV, Contrast) shared with the F9 Frame Window.

use crate::editor::{Editor, Tool};
use crate::render::post::{Tone, Tonemap};
use crate::ui::widgets::{dim, max_spinner_range, rollout, toggle_button};
use bevy_egui::egui;

fn grid(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([10.0, 6.0]).min_col_width(130.0).show(ui, body);
}

fn gi_rollout(ui: &mut egui::Ui, ed: &mut Editor) {
    let g = &mut ed.render.gi;
    rollout(ui, "GI & AO", true, |ui| {
        ui.checkbox(&mut g.ao, "Enable Ambient Occlusion");
        ui.add_enabled_ui(g.ao, |ui| {
            grid(ui, "fx_ao", |ui| {
                ui.label("AO Samples");
                max_spinner_range(ui, &mut g.ao_samples, 0.2, 16, 4..=64);
                ui.end_row();
                ui.label("AO Radius");
                max_spinner_range(ui, &mut g.ao_radius, 0.01, 0.5, 0.01..=100.0);
                ui.end_row();
                ui.label("AO Strength");
                max_spinner_range(ui, &mut g.ao_strength, 0.01, 0.8, 0.0..=1.0);
                ui.end_row();
            });
        });
        ui.checkbox(&mut g.gi, "Enable 1-Bounce Indirect GI (Color Bleed)");
        grid(ui, "fx_gi", |ui| {
            ui.label("GI Bounce Rays");
            ui.add_enabled_ui(g.gi, |ui| max_spinner_range(ui, &mut g.gi_samples, 0.2, 16, 1..=256));
            ui.end_row();
            ui.label("Emitter Samples");
            max_spinner_range(ui, &mut g.emitter_samples, 0.2, 8, 0..=64)
                .on_hover_text("Next-event samples on emissive polygons (Emission Multiplier > 0)");
            ui.end_row();
        });
        ui.checkbox(&mut g.denoise, "Edge-Preserving Denoiser (A-Trous on the Path Tracer)");
        ui.checkbox(&mut g.softbox, "Studio Softbox Reflection Dome").on_hover_text("Used by reflection rays when no HDRI is loaded");
    });
}

/// Tone operator + Exposure + Contrast; true when anything changed.
pub fn tone_row(ui: &mut egui::Ui, tone: &mut Tone) -> bool {
    let before = *tone;
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt(ui.id().with("tonemap")).width(110.0).selected_text(tone.op.label()).show_ui(ui, |ui| {
            for op in Tonemap::ALL {
                ui.selectable_value(&mut tone.op, op, op.label());
            }
        });
        ui.label("EV");
        max_spinner_range(ui, &mut tone.exposure, 0.02, 0.0, -10.0..=10.0).on_hover_text("Exposure in stops");
        ui.label("Contrast");
        max_spinner_range(ui, &mut tone.contrast, 0.01, 1.0, 0.2..=3.0);
    });
    ui.checkbox(&mut tone.affect_background, "Tonemap Affects Background");
    *tone != before
}

fn post_rollout(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Post-FX", true, |ui| {
        let p = &mut ed.render.post;
        ui.checkbox(&mut p.bloom, "Enable Bloom / Glare");
        ui.add_enabled_ui(p.bloom, |ui| {
            grid(ui, "fx_bloom", |ui| {
                ui.label("Threshold");
                max_spinner_range(ui, &mut p.bloom_threshold, 0.01, 1.0, 0.0..=100.0);
                ui.end_row();
                ui.label("Intensity");
                max_spinner_range(ui, &mut p.bloom_intensity, 0.01, 0.6, 0.0..=10.0);
                ui.end_row();
                ui.label("Radius (px @ 800)");
                max_spinner_range(ui, &mut p.bloom_radius, 0.1, 12.0, 1.0..=128.0);
                ui.end_row();
            });
        });
        ui.checkbox(&mut p.dof, "Enable Depth of Field");
        let dof = p.dof;
        ui.add_enabled_ui(dof, |ui| {
            grid(ui, "fx_dof", |ui| {
                ui.label("Focus Distance");
                ui.add_enabled_ui(ed.render.post.focus_object.is_none(), |ui| {
                    max_spinner_range(ui, &mut ed.render.post.focus_distance, 0.02, 5.0, 0.01..=10000.0);
                });
                ui.end_row();
                ui.label("Aperture (f-stop)");
                max_spinner_range(ui, &mut ed.render.post.f_stop, 0.02, 2.8, 0.5..=64.0);
                ui.end_row();
            });
            ui.horizontal(|ui| {
                let picking = ed.tool == Tool::PickFocus;
                if toggle_button(ui, picking, "Pick Focus Object").clicked() {
                    ed.tool = if picking { Tool::None } else { Tool::PickFocus };
                }
                let name = ed.render.post.focus_object.and_then(|id| ed.scene.objects.get(id)).map(|o| o.name.clone());
                match name {
                    Some(n) => {
                        ui.label(n);
                        if ui.small_button("✖").on_hover_text("Use Focus Distance").clicked() {
                            ed.render.post.focus_object = None;
                        }
                    }
                    None => {
                        ui.label(dim("none"));
                    }
                }
            });
        });
        ui.separator();
        ui.label("Tonemapping");
        if tone_row(ui, &mut ed.render.post.tone) {
            ed.retone_render();
        }
    });
}

/// The tab body.
pub fn tab(ui: &mut egui::Ui, ed: &mut Editor) {
    gi_rollout(ui, ed);
    post_rollout(ui, ed);
    super::render_lens::rollout_ui(ui, ed);
    if ui.button("🎞 Render Turntable / Sprite Sheet...").on_hover_text("360° PNG sequence or 4 / 8-direction isometric sprite atlas + JSON").clicked() {
        ed.rfw.sheet.open = true;
    }
    ui.label(dim("Tone controls also live in the F9 window and update the image without re-rendering."));
}
