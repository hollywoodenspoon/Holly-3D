//! Render Setup (F10): Common tab (output size, aspect lock, alpha, view)
//! and Renderer tab (path tracer or classic raytracer settings) and the
//! GI & Post-FX tab (see `render_fx`).

use crate::editor::Editor;
use crate::render::settings::{AA_LEVELS, RESOLUTIONS};
use crate::render::{Integrator, RenderSettings};
use crate::ui::widgets::{dim, max_spinner_range};
use bevy_egui::egui::{self, RichText};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum SetupTab {
    #[default]
    Common,
    Renderer,
    /// GI & AO + Post-FX rollouts (built-in raytracer).
    Effects,
}

fn grid(ui: &mut egui::Ui, id: &str, body: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id).num_columns(2).spacing([10.0, 6.0]).min_col_width(130.0).show(ui, body);
}

/// "Active Viewport", a numbered viewport or a camera object; shared with the RFW dropdown.
pub fn view_combo(ui: &mut egui::Ui, ed: &mut Editor, id: &str, width: f32) {
    let label = |ed: &Editor, v: Option<usize>| match v {
        None => format!("Active ({})", ed.view_label(ed.viewports.active)),
        Some(i) => format!("{}: {}", i + 1, ed.view_label(i)),
    };
    let cur = if ed.render.camera.is_some() { ed.render_label() } else { label(ed, ed.render.view) };
    egui::ComboBox::from_id_salt(id).width(width).selected_text(cur).show_ui(ui, |ui| {
        for v in std::iter::once(None).chain((0..ed.viewports.views.len()).map(Some)) {
            let text = label(ed, v);
            if ui.selectable_label(ed.render.camera.is_none() && ed.render.view == v, text).clicked() {
                (ed.render.view, ed.render.camera) = (v, None);
            }
        }
        let cams = ed.scene.cameras();
        if !cams.is_empty() {
            ui.separator();
        }
        for c in cams {
            let text = format!("🎥 {}", ed.scene.objects[c].name);
            if ui.selectable_label(ed.render.camera == Some(c), text).clicked() {
                ed.render.camera = Some(c);
            }
        }
    });
}

fn common(ui: &mut egui::Ui, ed: &mut Editor) {
    ui.label(RichText::new("Output Size").strong());
    ui.horizontal_wrapped(|ui| {
        for (w, h) in RESOLUTIONS {
            let on = ed.render.width == w && ed.render.height == h;
            if ui.selectable_label(on, format!("{w}x{h}")).clicked() {
                (ed.render.width, ed.render.height) = (w, h);
            }
        }
    });
    let r = &mut ed.render;
    let (mut w, mut h) = (r.width, r.height);
    grid(ui, "rs_size", |ui| {
        ui.label("Width");
        if max_spinner_range(ui, &mut w, 4.0, 800, 16..=8192).changed() {
            r.set_width(w);
        }
        ui.end_row();
        ui.label("Height");
        if max_spinner_range(ui, &mut h, 4.0, 600, 16..=8192).changed() {
            r.set_height(h);
        }
        ui.end_row();
        ui.label("Image Aspect");
        ui.horizontal(|ui| {
            ui.label(format!("{:.3}", r.aspect()));
            ui.checkbox(&mut r.lock_aspect, "🔒 Lock");
        });
        ui.end_row();
    });
    ui.checkbox(&mut r.transparent, "Transparent Background (Alpha)");
    ui.separator();
    ui.label(RichText::new("View to Render").strong());
    view_combo(ui, ed, "rs_view", 260.0);
}

fn raytracer(ui: &mut egui::Ui, r: &mut RenderSettings) {
    grid(ui, "rs_rt", |ui| {
        ui.label("Anti-Aliasing");
        let cur = AA_LEVELS.iter().find(|(n, _)| *n == r.aa).map_or("Custom", |(_, l)| l);
        egui::ComboBox::from_id_salt("rs_aa").width(150.0).selected_text(cur).show_ui(ui, |ui| {
            for (n, l) in AA_LEVELS {
                ui.selectable_value(&mut r.aa, n, l);
            }
        });
        ui.end_row();
        ui.label("Max Ray Depth");
        max_spinner_range(ui, &mut r.max_depth, 0.1, 4, 0..=16);
        ui.end_row();
        ui.label("Skylight / AO Rays");
        max_spinner_range(ui, &mut r.sky_samples, 0.2, 8, 1..=64);
        ui.end_row();
    });
    ui.checkbox(&mut r.shadows, "Shadows");
    ui.add_enabled_ui(r.shadows, |ui| {
        grid(ui, "rs_sh", |ui| {
            ui.label("Soft Shadow Rays");
            max_spinner_range(ui, &mut r.shadow_samples, 0.2, 8, 1..=64);
            ui.end_row();
        });
    });
    ui.checkbox(&mut r.outlines, "Universal Ink Outline Pass");
    ui.label(dim("Soft shadows, glossy reflections, crystal refraction, fog and tonemapping."));
}

pub fn show(ctx: &egui::Context, ed: &mut Editor, tab: &mut SetupTab) {
    if !ed.rfw.setup_open {
        return;
    }
    let mut open = true;
    egui::Window::new(RichText::new("Render Setup").strong())
        .id(egui::Id::new("render_setup"))
        .open(&mut open)
        .default_pos([1150.0, 80.0])
        .default_width(330.0)
        .resizable(false)
        .show(ctx, |ui| {
            crate::ui::render_path::engine_rows(ui, &mut ed.render);
            ui.separator();
            ui.horizontal(|ui| {
                for (t, label) in [(SetupTab::Common, "Common"), (SetupTab::Renderer, "Renderer"), (SetupTab::Effects, "GI & Post-FX")] {
                    if crate::ui::widgets::toggle_button(ui, *tab == t, label).clicked() {
                        *tab = t;
                    }
                }
            });
            ui.separator();
            match *tab {
                SetupTab::Common => common(ui, ed),
                SetupTab::Renderer => match ed.render.path.integrator {
                    Integrator::Classic => raytracer(ui, &mut ed.render),
                    Integrator::PathTracer => crate::ui::render_path::tab(ui, &mut ed.render),
                },
                SetupTab::Effects => {
                    egui::ScrollArea::vertical().max_height(520.0).show(ui, |ui| crate::ui::render_fx::tab(ui, ed));
                }
            }
            ui.separator();
            ui.horizontal(|ui| {
                let busy = ed.is_rendering();
                if ui.add_enabled(!busy, egui::Button::new(RichText::new("▶ Render").strong()).min_size(egui::vec2(120.0, 26.0))).clicked()
                {
                    ed.start_render();
                }
                if busy && ui.button("Cancel").clicked() {
                    ed.cancel_render();
                }
            });
        });
    if !open {
        ed.rfw.setup_open = false;
    }
}
