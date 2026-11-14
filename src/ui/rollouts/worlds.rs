//! Procedural world rollouts: Terrain and Ocean parameters (shared by the
//! base objects and their stack modifiers) and the base-object dispatch
//! (the Planet Studio rollouts live in `planet.rs`).

use super::spline::{grid, mod_params};
use crate::editor::Editor;
use crate::generators::{Biome, OceanParams, TerrainIds, TerrainParams, WorldGen};
use crate::modifiers::ModifierKind;
use crate::scene::BaseObject;
use crate::ui::properties::content_w;
use crate::ui::widgets::{Track, dim, max_spinner_range, rollout};
use bevy_egui::egui::{self, RichText};

/// Spinner row with an explicit drag speed.
pub(crate) fn row<T: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    t: &mut Track,
    label: &str,
    v: &mut T,
    range: std::ops::RangeInclusive<T>,
    default: T,
    speed: f64,
) {
    ui.label(label);
    t.r(max_spinner_range(ui, v, speed, default, range));
    ui.end_row();
}

/// Random Seed spinner + `[ 🎲 New Seed ]`.
pub(crate) fn seed_row(ui: &mut egui::Ui, t: &mut Track, seed: &mut u32) {
    ui.horizontal(|ui| {
        ui.label("Random Seed:");
        t.r(max_spinner_range(ui, seed, 0.2, 1, 0..=99_999));
        if ui.button("🎲 New Seed").clicked() {
            *seed = rand::random::<u32>() % 100_000;
            (t.started, t.changed) = (true, true);
        }
    });
}

fn terrain_ids(ui: &mut egui::Ui, t: &mut Track, ids: &mut TerrainIds) {
    t.r(ui.checkbox(&mut ids.enabled, "Auto-Assign Material IDs by Slope & Altitude"));
    if ids.enabled {
        grid(ui, "terrain_ids", |ui| {
            row(ui, t, "Cliff Angle:", &mut ids.cliff_angle, 5.0..=89.0, 38.0, 0.5);
            row(ui, t, "Snow Line:", &mut ids.snow_line, 0.0..=1.1, 0.88, 0.005);
            row(ui, t, "Sand Line:", &mut ids.sand_line, 0.0..=1.1, 0.1, 0.005);
        });
        ui.label(dim("ID 1 Grass · 2 Cliff Rock · 3 Snow · 4 Sand / Riverbed — the Alpine Terrain Pack."));
    }
}

/// Terrain Parameters (base object and modifier; `sized` hides Width /
/// Length, which the modifier takes from its input's bounds).
pub fn terrain_params(ui: &mut egui::Ui, t: &mut Track, p: &mut TerrainParams, sized: bool) {
    ui.label(RichText::new("Biome Preset").strong());
    egui::ComboBox::from_id_salt("terrain_biome").width(content_w() - 12.0).selected_text(p.biome.label()).show_ui(ui, |ui| {
        for b in Biome::ALL {
            if ui.selectable_label(p.biome == b, b.label()).clicked() && p.biome != b {
                // A new biome starts from its defaults, keeping size and seed.
                *p = TerrainParams { width: p.width, length: p.length, seed: p.seed, ..TerrainParams::of(b) };
                (t.started, t.changed) = (true, true);
            }
        }
    });
    seed_row(ui, t, &mut p.seed);
    grid(ui, "terrain_params", |ui| {
        if sized {
            row(ui, t, "Width:", &mut p.width, 0.1..=100_000.0, 200.0, 0.5);
            row(ui, t, "Length:", &mut p.length, 0.1..=100_000.0, 200.0, 0.5);
        }
        row(ui, t, "Segments:", &mut p.segments, 2..=512, 96, 0.2);
        row(ui, t, "Max Elevation:", &mut p.max_elevation, -10_000.0..=10_000.0, 25.0, 0.1);
        row(ui, t, "Octaves:", &mut p.octaves, 1..=10, 5, 0.05);
        row(ui, t, "Roughness:", &mut p.roughness, 0.1..=0.9, 0.5, 0.005);
        row(ui, t, "Terrace Steps:", &mut p.terraces, 0..=40, 0, 0.1);
        row(ui, t, "Erosion Strength:", &mut p.erosion, 0.0..=1.0, 0.0, 0.005);
        if p.biome == Biome::Dunes {
            row(ui, t, "Wind Direction:", &mut p.wind_angle, -360.0..=360.0, 30.0, 0.5);
        }
    });
    terrain_ids(ui, t, &mut p.ids);
}

/// Ocean Parameters (base object and modifier).
pub fn ocean_params(ui: &mut egui::Ui, t: &mut Track, p: &mut OceanParams, sized: bool) {
    grid(ui, "ocean_params", |ui| {
        if sized {
            row(ui, t, "Width:", &mut p.width, 0.1..=100_000.0, 200.0, 0.5);
            row(ui, t, "Length:", &mut p.length, 0.1..=100_000.0, 200.0, 0.5);
            row(ui, t, "Segments:", &mut p.segments, 1..=512, 128, 0.2);
        }
        row(ui, t, "Wave Height:", &mut p.wave_height, 0.0..=1_000.0, 2.5, 0.02);
        row(ui, t, "Choppiness:", &mut p.choppiness, 0.0..=1.0, 0.7, 0.005);
        row(ui, t, "Wavelength:", &mut p.wavelength, 0.1..=10_000.0, 40.0, 0.1);
        row(ui, t, "Wind Direction:", &mut p.wind_angle, -360.0..=360.0, 0.0, 0.5);
    });
    ui.horizontal(|ui| {
        ui.label("Phase / Time:");
        t.r(ui.add(egui::Slider::new(&mut p.time, 0.0..=60.0).suffix(" s")));
    });
    t.r(ui.checkbox(&mut p.auto_foam, "Auto-Crest Foam"));
    if p.auto_foam {
        grid(ui, "ocean_foam", |ui| row(ui, t, "Foam Threshold:", &mut p.foam_threshold, 0.0..=0.99, 0.6, 0.005));
        ui.label(dim("Foam whitens the vertex colors and sets Material ID 2 on high crests (Ocean Water Pack)."));
    }
}

/// Parameters rollout of a world base object (Terrain / Ocean / Planet).
pub fn base(ui: &mut egui::Ui, ed: &mut Editor) {
    let Some(id) = ed.current() else { return };
    let BaseObject::World(w) = &ed.scene.objects[id].base else { return };
    let mut w = w.clone();
    let (mut t, mut tex, mut style) = (Track::default(), None, None);
    let mut ruin = super::ruin::Out::default();
    let (mut split, mut pick_path, mut sim_frames) = (false, false, None);
    match &mut w {
        WorldGen::Terrain(p) => rollout(ui, "Demiurge Terrain Parameters", true, |ui| terrain_params(ui, &mut t, p, true)),
        WorldGen::Ocean(p) => rollout(ui, "Demiurge Ocean Parameters", true, |ui| ocean_params(ui, &mut t, p, true)),
        WorldGen::Planet(p) => tex = super::planet::params(ui, &mut t, p),
        WorldGen::Settlement(p) => {
            rollout(ui, "Demiurge Settlement", true, |ui| ruin = super::ghosttown::settlement_params(ui, &mut t, p, true))
        }
        WorldGen::Building(p) => rollout(ui, "Building Parameters", true, |ui| (style, ruin) = super::buildings::params(ui, &mut t, p)),
        WorldGen::Smoke(p) => rollout(ui, "Demiurge Smoke & Fire Plume", true, |ui| {
            super::smoke::params(ui, &mut t, p, "base");
            super::clouds::look(ui, &mut t, &mut p.look, "smoke_look");
        }),
        WorldGen::Cloud(p) => rollout(ui, "Demiurge Cloud", true, |ui| super::clouds::cloud(ui, &mut t, p)),
        WorldGen::CloudField(p) => rollout(ui, "Demiurge Cloud Field", true, |ui| split = super::clouds::field(ui, &mut t, p)),
        WorldGen::Trail(p) => rollout(ui, "Demiurge Trail", true, |ui| pick_path = super::trails::trail(ui, &mut t, p)),
        WorldGen::Pyro(p) => rollout(ui, "Demiurge Explosion", true, |ui| super::pyro::pyro(ui, &mut t, p)),
        WorldGen::Vdb(p) => rollout(ui, "VDB Volume", true, |ui| super::vdb::vdb(ui, &mut t, p)),
        WorldGen::SmokeSim(p) => rollout(ui, "Demiurge Smoke Simulation", true, |ui| {
            if super::smoke_sim::sim(ui, &mut t, p) {
                sim_frames = Some(p.sim.frames.clamp(1, 2000));
            }
        }),
    }
    if let Some(n) = sim_frames {
        ed.open_sim_sequence(n);
    }
    if pick_path {
        ed.tool = crate::editor::Tool::PickTrailPath;
        ed.status = "Pick Path Spline: click a spline in a viewport".into();
    }
    if split {
        ed.break_cloud_field(id);
        return;
    }
    if let WorldGen::Settlement(_) | WorldGen::Building(_) = &w {
        super::ghosttown::pack_button(ui, ed);
    }
    if t.started {
        ed.checkpoint("World Parameters");
    }
    if t.changed {
        ed.set_world(id, w);
    }
    if let Some(s) = style {
        ed.assign_pack(s.pack());
    }
    if let Some(g) = ruin.glow {
        ed.set_ember_glow(id, g);
    }
    if let Some((part, path)) = tex {
        ed.planet_custom_texture(id, part, path.as_deref());
    }
}

/// Terrain stack modifier.
pub fn terrain_modifier(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Demiurge Terrain", true, |ui| {
        ui.label(dim("Raises the input mesh along local Z over its XY bounds; subdivide it for detail."));
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::Terrain(m) = k else { return };
            terrain_params(ui, t, &mut m.params, false);
        });
    });
}

/// Ocean stack modifier.
pub fn ocean_modifier(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Demiurge Ocean", true, |ui| {
        ui.label(dim("Gerstner waves on the input mesh: local XY is the rest position, crests rise along Z."));
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::Ocean(m) = k else { return };
            ocean_params(ui, t, &mut m.params, false);
        });
    });
}
