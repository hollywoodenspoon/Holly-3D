//! Greeble, Chamfer, Panel Groove, Symmetry, Shell and TurboSmooth rollouts.

use super::deformers::{axis_row, speed_row};
use super::spline::{grid, mod_params, num_row, u_row};
use crate::editor::Editor;
use crate::modifiers::{ChamferFilter, ModifierKind};
use crate::ui::properties::content_w;
use crate::ui::widgets::{Track, dim, max_spinner_range, rollout};
use bevy_egui::egui::{self, RichText};

pub fn greeble(ui: &mut egui::Ui, ed: &mut Editor) {
    mod_params(ui, ed, |ui, k, t| {
        let ModifierKind::Greeble(g) = k else { return };
        rollout(ui, "Panels", true, |ui| {
            ui.horizontal(|ui| {
                ui.label("Random Seed:");
                t.r(max_spinner_range(ui, &mut g.seed, 0.2, 1, 0..=99_999));
                if ui.button("🎲 New Seed").clicked() {
                    g.seed = rand::random::<u32>() % 100_000;
                    t.started = true;
                    t.changed = true;
                }
            });
            grid(ui, "greeble_panels", |ui| {
                num_row(ui, t, "Min Height:", &mut g.min_height, 0.0..=100.0, 0.02);
                num_row(ui, t, "Max Height:", &mut g.max_height, 0.0..=100.0, 0.1);
                num_row(ui, t, "Taper:", &mut g.taper, 0.0..=0.95, 0.15);
                ui.label("Panel Material ID:");
                t.r(max_spinner_range(ui, &mut g.panel_mat, 0.1, 1, 1..=64));
                ui.end_row();
            });
            if g.min_height > g.max_height {
                g.max_height = g.min_height;
            }
            ui.horizontal(|ui| {
                t.r(ui.checkbox(&mut g.panels, "Generate Panels"));
                t.r(ui.checkbox(&mut g.select_tops, "Select Tops"));
            });
        });
        rollout(ui, "Widgets / Nurnies", true, |ui| {
            grid(ui, "greeble_widgets", |ui| {
                u_row(ui, t, "Subdivide Levels:", &mut g.levels, 2, 1..=3);
                num_row(ui, t, "Widget Density:", &mut g.widget_density, 0.0..=1.0, 0.5);
                num_row(ui, t, "Min Widget Size:", &mut g.min_widget, 0.0..=100.0, 0.03);
                num_row(ui, t, "Max Widget Size:", &mut g.max_widget, 0.0..=100.0, 0.08);
                ui.label("Widget Material ID:");
                t.r(max_spinner_range(ui, &mut g.widget_mat, 0.1, 2, 1..=64));
                ui.end_row();
            });
            ui.horizontal(|ui| {
                t.r(ui.checkbox(&mut g.boxes, "Boxes"));
                t.r(ui.checkbox(&mut g.cylinders, "Cylinders"));
                t.r(ui.checkbox(&mut g.ribs, "Ribs/Ledges"));
            });
            ui.label(dim("Applies to the selected polygons passed up the stack, or to every polygon."));
        });
    });
}

pub fn chamfer(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Chamfer Parameters", true, |ui| {
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::Chamfer(c) = k else { return };
            grid(ui, "chamfer_mod", |ui| {
                num_row(ui, t, "Amount:", &mut c.amount, 0.0..=1e4, 0.05);
                u_row(ui, t, "Segments:", &mut c.segments, 1, 1..=16);
                num_row(ui, t, "Tension:", &mut c.tension, 0.0..=1.0, 0.5);
            });
            filter(ui, t, &mut c.filter, "chamfer");
        });
    });
}

/// Edge Selection Filter (Chamfer and Panel Groove modifiers).
fn filter(ui: &mut egui::Ui, t: &mut Track, f: &mut ChamferFilter, id: &str) {
    ui.label(RichText::new("Selection Filter").strong());
    let current = *f;
    egui::ComboBox::from_id_salt(format!("{id}_filter")).width(content_w() - 12.0).selected_text(current.label()).show_ui(ui, |ui| {
        let angle = match current {
            ChamferFilter::Angle { .. } => current,
            _ => ChamferFilter::Angle { min: 30.0, max: 180.0 },
        };
        for c in [ChamferFilter::All, ChamferFilter::Selected, ChamferFilter::SmoothingBoundaries, angle] {
            if ui.selectable_label(std::mem::discriminant(&c) == std::mem::discriminant(&current), c.label()).clicked() {
                *f = c;
                t.started = true;
                t.changed = true;
            }
        }
    });
    if let ChamferFilter::Angle { min, max } = f {
        grid(ui, &format!("{id}_angle"), |ui| {
            speed_row(ui, t, "Min Angle:", min, 0.5, 30.0, 0.0..=180.0);
            speed_row(ui, t, "Max Angle:", max, 0.5, 180.0, 0.0..=180.0);
        });
    }
}

pub fn panel_groove(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Panel Groove Parameters", true, |ui| {
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::PanelGroove(g) = k else { return };
            grid(ui, "panel_groove", |ui| {
                num_row(ui, t, "Groove Width:", &mut g.width, 0.0..=1e4, 0.04);
                num_row(ui, t, "Groove Depth:", &mut g.depth, 0.0..=1e4, 0.03);
                num_row(ui, t, "Bevel Lip Width:", &mut g.lip, 0.0..=1e4, 0.008);
                ui.label("Inner Groove Mat ID:");
                t.r(max_spinner_range(ui, &mut g.material_id, 0.1, 2, 1..=64));
                ui.end_row();
            });
            filter(ui, t, &mut g.filter, "groove");
            ui.label(dim("Selected Edges uses the edges selected below in the stack (e.g. an Edit Poly modifier)."));
        });
    });
}

pub fn symmetry(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Symmetry Parameters", true, |ui| {
        super::modifiers::sub_buttons(ui, ed, &[crate::modifiers::ModSub::Mirror]);
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::Symmetry(s) = k else { return };
            axis_row(ui, t, "Mirror Axis", &mut s.axis);
            t.r(ui.checkbox(&mut s.flip, "Flip"));
            t.r(ui.checkbox(&mut s.slice, "Slice Along Mirror"));
            ui.horizontal(|ui| {
                t.r(ui.checkbox(&mut s.weld, "Weld Seam"));
                ui.label("Threshold:");
                t.r(max_spinner_range(ui, &mut s.threshold, 0.001, 0.01, 0.0..=10.0));
            });
            grid(ui, "sym_offset", |ui| num_row(ui, t, "Mirror Offset:", &mut s.offset, -1e4..=1e4, 0.0));
            ui.label(dim("Mirror Offset moves the plane along its axis; the Mirror sub-object moves and rotates it with W / E."));
        });
    });
}

pub fn shell(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Shell Parameters", true, |ui| {
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::Shell(s) = k else { return };
            grid(ui, "shell", |ui| {
                num_row(ui, t, "Inner Amount:", &mut s.inner, -1e4..=1e4, 0.05);
                num_row(ui, t, "Outer Amount:", &mut s.outer, -1e4..=1e4, 0.0);
                u_row(ui, t, "Segments:", &mut s.segments, 1, 1..=32);
            });
            t.r(ui.checkbox(&mut s.straighten, "Straighten Corners"));
            ui.horizontal(|ui| {
                let mut on = s.override_edge_mat.is_some();
                if t.r(ui.checkbox(&mut on, "Override Edge Mat ID:")).changed() {
                    s.override_edge_mat = on.then_some(1);
                }
                if let Some(id) = &mut s.override_edge_mat {
                    t.r(max_spinner_range(ui, id, 0.1, 1, 1..=64));
                }
            });
            ui.label(RichText::new("Selection").strong());
            t.r(ui.checkbox(&mut s.select_edges, "Select Edge Faces"));
            ui.horizontal(|ui| {
                t.r(ui.checkbox(&mut s.select_inner, "Select Inner Faces"));
                t.r(ui.checkbox(&mut s.select_outer, "Select Outer Faces"));
            });
        });
    });
}

pub fn turbosmooth(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "TurboSmooth", true, |ui| {
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::TurboSmooth(s) = k else { return };
            grid(ui, "turbosmooth", |ui| u_row(ui, t, "Iterations:", &mut s.iterations, 1, 0..=4));
            t.r(ui.checkbox(&mut s.isoline, "Isoline Display"));
            ui.label(RichText::new("Surface Parameters").strong());
            t.r(ui.checkbox(&mut s.separate_smoothing, "Separate by Smoothing Groups"));
            t.r(ui.checkbox(&mut s.separate_materials, "Separate by Materials"));
            ui.label(dim("Each iteration splits every n-gon into n quads."));
        });
    });
}
