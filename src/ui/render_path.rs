//! Render Setup (F10) → Renderer tab for the native path tracer: Compute
//! Device, Progressive Preview, bounce limits and adaptive sampling.

use crate::render::{ComputeDevice, Integrator, RenderSettings};
use crate::ui::widgets::{dim, max_spinner_range};
use bevy_egui::egui::{self, RichText};

pub fn combo<T: Copy + PartialEq>(ui: &mut egui::Ui, id: &str, width: f32, v: &mut T, all: &[T], label: fn(T) -> &'static str) {
    egui::ComboBox::from_id_salt(id).width(width).selected_text(label(*v)).show_ui(ui, |ui| {
        for &x in all {
            ui.selectable_value(v, x, label(x));
        }
    });
}

/// Integrator + Compute Device rows shown above the Render Setup tabs.
pub fn engine_rows(ui: &mut egui::Ui, r: &mut RenderSettings) {
    egui::Grid::new("rs_engine").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label("Renderer");
        combo(ui, "rs_integrator", 220.0, &mut r.path.integrator, &Integrator::ALL, Integrator::label);
        ui.end_row();
        ui.label("Compute Device");
        ui.add_enabled_ui(r.path.integrator == Integrator::PathTracer, |ui| {
            combo(ui, "rs_device", 220.0, &mut r.path.device, &ComputeDevice::ALL, ComputeDevice::label);
        });
        ui.end_row();
    });
}

pub fn tab(ui: &mut egui::Ui, r: &mut RenderSettings) {
    let p = &mut r.path;
    ui.checkbox(&mut p.progressive, "Stream Full-Frame Progressive Passes (1 -> 4 -> 16 -> Final SPP)");
    ui.label(RichText::new("Sampling").strong());
    egui::Grid::new("rs_pt").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        ui.label("Samples per Pixel");
        max_spinner_range(ui, &mut p.samples, 1.0, 64, 1..=4096);
        ui.end_row();
        ui.label("Max Bounces");
        max_spinner_range(ui, &mut p.max_bounces, 0.05, 6, 1..=8);
        ui.end_row();
        ui.label("Diffuse Bounces");
        max_spinner_range(ui, &mut p.diffuse_bounces, 0.05, 4, 1..=8);
        ui.end_row();
        ui.label("Glossy Bounces");
        max_spinner_range(ui, &mut p.glossy_bounces, 0.05, 4, 1..=8);
        ui.end_row();
        ui.label("Transmission Bounces");
        max_spinner_range(ui, &mut p.transmission_bounces, 0.05, 8, 1..=8);
        ui.end_row();
        ui.label("Russian Roulette After");
        max_spinner_range(ui, &mut p.rr_start, 0.05, 2, 1..=8);
        ui.end_row();
    });
    ui.checkbox(&mut p.adaptive, "Adaptive Sampling (stop converged pixels early)");
    ui.add_enabled_ui(p.adaptive, |ui| {
        egui::Grid::new("rs_pt_ad").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label("Noise Threshold");
            let mut t = p.noise_threshold;
            if ui.add(egui::DragValue::new(&mut t).speed(0.001).range(0.002..=0.10).fixed_decimals(3)).secondary_clicked() {
                t = 0.02;
            }
            p.noise_threshold = t;
            ui.end_row();
            ui.label("Min Samples");
            max_spinner_range(ui, &mut p.min_samples, 0.1, 8, 4..=32);
            ui.end_row();
        });
    });
    ui.checkbox(&mut p.lightmix, "LightMix Buffers (relight per light group in F9, CPU tracer)")
        .on_hover_text("Keeps one buffer per light group so the Frame Window's LightMix can relight the render live");
    ui.checkbox(&mut r.shadows, "Shadows");
    ui.checkbox(&mut r.outlines, "Universal Ink Outline Pass");
    ui.label(dim("Multi-bounce paths with MIS (light + BSDF sampling, power heuristic) and Russian Roulette. Auto uses the wgpu GPU kernel and falls back to the CPU."));
}
