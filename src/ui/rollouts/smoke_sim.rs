//! Step D rollout: a Smoke Simulation (grid smoke and fire solver).

use super::spline::grid;
use super::worlds::{row, seed_row};
use crate::generators::smoke_sim::{SimParams, SmokeSimParams, bake};
use crate::generators::vdb::VdbStyle;
use crate::ui::properties::content_w;
use crate::ui::widgets::{Track, dim};
use bevy_egui::egui::{self, RichText};

fn head(ui: &mut egui::Ui, s: &str) {
    ui.add_space(4.0);
    ui.label(RichText::new(s).strong());
}

fn playback(ui: &mut egui::Ui, t: &mut Track, p: &mut SmokeSimParams) {
    let b = p.bake();
    let last = p.sim.frames.max(1) - 1;
    ui.horizontal(|ui| {
        ui.label("Frame:");
        let mut f = p.frame.min(last) + 1;
        t.r(ui.add(egui::Slider::new(&mut f, 1..=last + 1)));
        p.frame = f - 1;
    });
    let (ready, total) = (b.ready(), b.total);
    ui.add(egui::ProgressBar::new(ready as f32 / total.max(1) as f32).desired_width(content_w() - 12.0).text(p.status()));
    if ready < total {
        ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
    }
    if ui.button("Re-bake").on_hover_text("Throw the baked frames away and simulate again").clicked() {
        bake::rebake(&p.sim);
        (t.started, t.changed) = (true, true);
    }
}

fn simulation(ui: &mut egui::Ui, t: &mut Track, s: &mut SimParams) {
    let d = SimParams::default();
    head(ui, "Domain");
    grid(ui, "sim_domain", |ui| {
        row(ui, t, "Width (X):", &mut s.size[0], 0.25..=200.0, d.size[0], 0.02);
        row(ui, t, "Depth (Y):", &mut s.size[1], 0.25..=200.0, d.size[1], 0.02);
        row(ui, t, "Height (Z):", &mut s.size[2], 0.25..=400.0, d.size[2], 0.02);
        row(ui, t, "Resolution:", &mut s.resolution, 16..=256, d.resolution, 0.5);
    });
    head(ui, "Source");
    grid(ui, "sim_source", |ui| {
        row(ui, t, "Radius:", &mut s.source_radius, 0.02..=50.0, d.source_radius, 0.005);
        row(ui, t, "Height:", &mut s.source_height, 0.0..=100.0, d.source_height, 0.005);
        row(ui, t, "Speed (m/s):", &mut s.source_speed, -20.0..=50.0, d.source_speed, 0.02);
        row(ui, t, "Smoke:", &mut s.smoke, 0.0..=5.0, d.smoke, 0.01);
        row(ui, t, "Heat:", &mut s.heat, 0.0..=5.0, d.heat, 0.01);
        row(ui, t, "Fuel (per s):", &mut s.fuel, 0.0..=20.0, d.fuel, 0.02);
        row(ui, t, "Turbulence:", &mut s.turbulence, 0.0..=2.0, d.turbulence, 0.01);
    });
    head(ui, "Forces");
    grid(ui, "sim_forces", |ui| {
        row(ui, t, "Buoyancy:", &mut s.buoyancy, -10.0..=20.0, d.buoyancy, 0.02);
        row(ui, t, "Smoke Weight:", &mut s.weight, 0.0..=10.0, d.weight, 0.01);
        row(ui, t, "Swirl:", &mut s.swirl, 0.0..=4.0, d.swirl, 0.01);
        row(ui, t, "Wind X (m/s):", &mut s.wind[0], -30.0..=30.0, 0.0, 0.02);
        row(ui, t, "Wind Y (m/s):", &mut s.wind[1], -30.0..=30.0, 0.0, 0.02);
    });
    if s.fuel > 0.0 {
        head(ui, "Burning");
        grid(ui, "sim_burn", |ui| {
            row(ui, t, "Burn Rate:", &mut s.burn_rate, 0.0..=20.0, d.burn_rate, 0.02);
            row(ui, t, "Ignition Temp:", &mut s.ignition, 0.0..=5.0, d.ignition, 0.01);
            row(ui, t, "Heat Release:", &mut s.heat_release, 0.0..=20.0, d.heat_release, 0.02);
            row(ui, t, "Soot:", &mut s.soot, 0.0..=5.0, d.soot, 0.01);
        });
    }
    head(ui, "Fading & Timing");
    grid(ui, "sim_time", |ui| {
        row(ui, t, "Smoke Fade (per s):", &mut s.dissipation, 0.0..=5.0, d.dissipation, 0.005);
        row(ui, t, "Cooling (per s):", &mut s.cooling, 0.0..=10.0, d.cooling, 0.01);
        row(ui, t, "Frames:", &mut s.frames, 1..=2000, d.frames, 0.5);
        row(ui, t, "Frames per Second:", &mut s.fps, 1.0..=120.0, d.fps, 0.1);
        row(ui, t, "Pressure Iterations:", &mut s.pressure_iters, 4..=200, d.pressure_iters, 0.3);
    });
    seed_row(ui, t, &mut s.seed);
    t.r(ui.checkbox(&mut s.gpu, "Solve on the GPU").on_hover_text("Uses the graphics card when one is available (much faster at high resolution); off, or without a usable GPU, the CPU solves it."));
}

/// The Smoke Simulation rollout.
/// Returns true when `[ Render Frames… ]` was clicked.
pub fn sim(ui: &mut egui::Ui, t: &mut Track, p: &mut SmokeSimParams) -> bool {
    playback(ui, t, p);
    let render = ui.button("🎞 Render Frames…").on_hover_text("Render a range of simulation frames from the render camera as numbered PNGs").clicked();
    head(ui, "Look");
    ui.horizontal(|ui| {
        ui.label("Smoke Look:");
        egui::ComboBox::from_id_salt("sim_style").width(content_w() - 110.0).selected_text(p.style.label()).show_ui(ui, |ui| {
            for s in VdbStyle::ALL {
                t.r(ui.selectable_value(&mut p.style, s, s.label()));
            }
        });
    });
    let d = SmokeSimParams::default();
    grid(ui, "sim_look", |ui| {
        row(ui, t, "Smoke Density:", &mut p.density, 0.0..=100.0, d.density, 0.02);
        row(ui, t, "Fire Intensity (%):", &mut p.fire, 0.0..=100.0, d.fire, 0.5);
    });
    simulation(ui, t, &mut p.sim);
    let (_, n) = p.sim.grid();
    ui.label(dim(&format!(
        "Grid {} x {} x {} cells, about {:.0} MB for all frames. Changing a simulation setting bakes again in the background; Look and Frame don't.",
        n[0],
        n[1],
        n[2],
        p.memory_mb()
    )));
    render
}
