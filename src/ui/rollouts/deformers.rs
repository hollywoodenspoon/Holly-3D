//! Bend / Twist / Taper / Push rollouts.

use super::spline::{grid, mod_params, num_row};
use crate::editor::Editor;
use crate::modifiers::{Limits, ModifierKind, TaperEffect};
use crate::ui::widgets::{Track, dim, max_spinner_range, rollout};
use bevy::math::Vec3;
use bevy_egui::egui::{self, RichText};

/// Spinner row with an explicit drag speed (angles, percentages).
pub(crate) fn speed_row(
    ui: &mut egui::Ui,
    t: &mut Track,
    label: &str,
    v: &mut f32,
    speed: f64,
    default: f32,
    range: std::ops::RangeInclusive<f32>,
) {
    ui.label(label);
    t.r(max_spinner_range(ui, v, speed, default, range));
    ui.end_row();
}

pub(crate) fn axis_row(ui: &mut egui::Ui, t: &mut Track, label: &str, axis: &mut usize) {
    ui.label(RichText::new(label).strong());
    ui.horizontal(|ui| {
        for (i, n) in ["X", "Y", "Z"].iter().enumerate() {
            t.r(ui.radio_value(axis, i, *n));
        }
    });
}

fn limits(ui: &mut egui::Ui, t: &mut Track, id: &str, l: &mut Limits) {
    ui.label(RichText::new("Limits").strong());
    t.r(ui.checkbox(&mut l.on, "Limit Effect"));
    ui.add_enabled_ui(l.on, |ui| {
        grid(ui, id, |ui| {
            num_row(ui, t, "Upper Limit:", &mut l.upper, -1e4..=1e4, 1.0);
            num_row(ui, t, "Lower Limit:", &mut l.lower, -1e4..=1e4, 0.0);
        });
    });
}

fn center(ui: &mut egui::Ui, t: &mut Track, c: &mut Vec3) {
    ui.label(RichText::new("Center").strong()).on_hover_text("Gizmo center in object space (the pivot by default)");
    ui.horizontal(|ui| {
        for (i, n) in ["X:", "Y:", "Z:"].iter().enumerate() {
            ui.label(*n);
            t.r(max_spinner_range(ui, &mut c[i], 0.01, 0.0, -1e4..=1e4));
        }
    });
}

pub fn bend(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Bend Parameters", true, |ui| {
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::Bend(b) = k else { return };
            grid(ui, "bend", |ui| {
                speed_row(ui, t, "Angle:", &mut b.angle, 0.5, 0.0, -3600.0..=3600.0);
                speed_row(ui, t, "Direction:", &mut b.direction, 0.5, 0.0, -3600.0..=3600.0);
            });
            axis_row(ui, t, "Bend Axis", &mut b.axis);
            limits(ui, t, "bend_lim", &mut b.limits);
            center(ui, t, &mut b.center);
        });
    });
}

pub fn twist(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Twist Parameters", true, |ui| {
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::Twist(w) = k else { return };
            grid(ui, "twist", |ui| {
                speed_row(ui, t, "Angle:", &mut w.angle, 0.5, 0.0, -3600.0..=3600.0);
                speed_row(ui, t, "Bias:", &mut w.bias, 0.5, 0.0, -100.0..=100.0);
            });
            axis_row(ui, t, "Twist Axis", &mut w.axis);
            limits(ui, t, "twist_lim", &mut w.limits);
            center(ui, t, &mut w.center);
        });
    });
}

pub fn taper(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Taper Parameters", true, |ui| {
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::Taper(p) = k else { return };
            grid(ui, "taper", |ui| {
                num_row(ui, t, "Amount:", &mut p.amount, -10.0..=10.0, 0.0);
                num_row(ui, t, "Curve:", &mut p.curve, -10.0..=10.0, 0.0);
            });
            axis_row(ui, t, "Primary Axis", &mut p.axis);
            let names = match p.axis {
                0 => ["YZ", "Y", "Z"],
                1 => ["ZX", "Z", "X"],
                _ => ["XY", "X", "Y"],
            };
            ui.label(RichText::new("Effect").strong());
            ui.horizontal(|ui| {
                for (e, n) in [TaperEffect::Both, TaperEffect::First, TaperEffect::Second].into_iter().zip(names) {
                    t.r(ui.radio_value(&mut p.effect, e, n));
                }
            });
            t.r(ui.checkbox(&mut p.symmetry, "Symmetry"));
            limits(ui, t, "taper_lim", &mut p.limits);
            center(ui, t, &mut p.center);
        });
    });
}

pub fn push(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Push Parameters", true, |ui| {
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::Push(p) = k else { return };
            grid(ui, "push", |ui| num_row(ui, t, "Push Value:", &mut p.value, -1e4..=1e4, 0.0));
            ui.label(dim("Offsets every vertex along its averaged normal."));
        });
    });
}
