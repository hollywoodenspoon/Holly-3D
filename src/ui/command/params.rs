//! Parameters rollout for live parametric primitives.

use crate::editor::Editor;
use crate::primitives::Primitive;
use crate::ui::widgets::{dim, max_spinner_range, rollout};
use bevy_egui::egui;

struct Rows<'a> {
    changed: bool,
    started: bool,
    ui: &'a mut egui::Ui,
}

impl Rows<'_> {
    fn f(&mut self, label: &str, v: &mut f32, default: f32) {
        self.ui.label(label);
        let r = max_spinner_range(self.ui, v, 0.01, default, 0.0..=10_000.0);
        self.track(r);
    }
    fn u(&mut self, label: &str, v: &mut u32, default: u32, max: u32) {
        self.ui.label(label);
        let r = max_spinner_range(self.ui, v, 0.1, default, 1..=max);
        self.track(r);
    }
    fn track(&mut self, r: egui::Response) {
        self.changed |= r.changed();
        self.started |= r.drag_started() || r.gained_focus() || r.secondary_clicked();
        self.ui.end_row();
    }
}

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) {
    let Some(id) = ed.current() else { return };
    let Some(mut prim) = ed.scene.objects[id].primitive_mut().cloned() else { return };
    let mut changed = false;
    let mut started = false;
    rollout(ui, "Parameters", true, |ui| {
        egui::Grid::new("prim_params").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            let mut r = Rows { changed: false, started: false, ui };
            match &mut prim {
                Primitive::Box { length, width, height, length_segs, width_segs, height_segs } => {
                    r.f("Length:", length, 1.0);
                    r.f("Width:", width, 1.0);
                    r.f("Height:", height, 1.0);
                    r.u("Length Segs:", length_segs, 1, 100);
                    r.u("Width Segs:", width_segs, 1, 100);
                    r.u("Height Segs:", height_segs, 1, 100);
                }
                Primitive::Sphere { radius, segments, hemisphere } => {
                    r.f("Radius:", radius, 0.6);
                    r.u("Segments:", segments, 32, 200);
                    r.ui.label("Hemisphere:");
                    let rr = max_spinner_range(r.ui, hemisphere, 0.005, 0.0, 0.0..=0.95);
                    r.track(rr);
                }
                Primitive::Cylinder { radius, height, height_segs, cap_segs, sides } => {
                    r.f("Radius:", radius, 0.5);
                    r.f("Height:", height, 1.2);
                    r.u("Height Segs:", height_segs, 1, 100);
                    r.u("Cap Segs:", cap_segs, 1, 50);
                    r.u("Sides:", sides, 18, 200);
                }
                Primitive::Cone { radius1, radius2, height, height_segs, cap_segs, sides } => {
                    r.f("Radius 1:", radius1, 0.6);
                    r.f("Radius 2:", radius2, 0.0);
                    r.f("Height:", height, 1.2);
                    r.u("Height Segs:", height_segs, 1, 100);
                    r.u("Cap Segs:", cap_segs, 1, 50);
                    r.u("Sides:", sides, 24, 200);
                }
                Primitive::Plane { length, width, length_segs, width_segs } => {
                    r.f("Length:", length, 2.0);
                    r.f("Width:", width, 2.0);
                    r.u("Length Segs:", length_segs, 4, 200);
                    r.u("Width Segs:", width_segs, 4, 200);
                }
                Primitive::Torus { radius1, radius2, segments, sides } => {
                    r.f("Radius 1:", radius1, 0.7);
                    r.f("Radius 2:", radius2, 0.2);
                    r.u("Segments:", segments, 24, 200);
                    r.u("Sides:", sides, 12, 100);
                }
            }
            changed = r.changed;
            started = r.started;
        });
        ui.label(dim("Right-click any spinner to reset it."));
    });
    if started {
        ed.checkpoint("Parameters");
    }
    if changed {
        let o = &mut ed.scene.objects[id];
        if let Some(p) = o.primitive_mut() {
            *p = prim;
        }
        o.evaluate();
        ed.scene.propagate_from(id);
    }
}
