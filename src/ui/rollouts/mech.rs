//! Mech Parameters rollout (Modify tab): Involute Gear, Hydraulic Piston
//! part and Hose / Mecha Cable, including the hose's Pick Top / Bottom End
//! Object buttons.

use super::spline::{grid, num_row, u_row};
use crate::editor::{Editor, Tool};
use crate::mech::{Fastener, FastenerKind, Gear, GearType, Hose, HoseCurve, MechPart, PistonPart, ThreadStd};
use crate::ui::command::CONTENT_W;
use crate::ui::widgets::{Track, dim, rollout};
use bevy::math::Vec3;
use bevy_egui::egui::{self, RichText};

fn gear(ui: &mut egui::Ui, t: &mut Track, g: &mut Gear) {
    ui.label(RichText::new("Type").strong());
    egui::ComboBox::from_id_salt("gear_type").width(CONTENT_W - 12.0).selected_text(g.kind.label()).show_ui(ui, |ui| {
        for k in GearType::ALL {
            t.r(ui.selectable_value(&mut g.kind, k, k.label()));
        }
    });
    grid(ui, "gear_params", |ui| {
        u_row(ui, t, "Teeth:", &mut g.teeth, 20, 6..=200);
        num_row(ui, t, "Module:", &mut g.module, 0.005..=10.0, 0.1);
        num_row(ui, t, "Pressure Angle:", &mut g.pressure_angle, 5.0..=40.0, 20.0);
        num_row(ui, t, "Thickness:", &mut g.thickness, 0.001..=100.0, 0.3);
        if g.kind == GearType::Spur {
            num_row(ui, t, "Bore Radius:", &mut g.bore, 0.0..=100.0, 0.2);
        }
        num_row(ui, t, "Helix Angle:", &mut g.helix, -60.0..=60.0, 0.0);
    });
    let what = if g.kind == GearType::Rack { "Circular pitch" } else { "Pitch radius" };
    let v = if g.kind == GearType::Rack { std::f32::consts::PI * g.module } else { g.pitch_radius() };
    ui.label(dim(&format!("{what}: {v:.4}. Meshing gears need the same Module and Pressure Angle.")));
}

fn piston(ui: &mut egui::Ui, t: &mut Track, p: &mut PistonPart) {
    ui.label(dim(if p.rod { "Piston Rod (slides in its housing)" } else { "Piston Housing (cylinder)" }));
    let d = if p.rod { PistonPart::rod() } else { PistonPart::housing() };
    grid(ui, "piston_params", |ui| {
        num_row(ui, t, "Radius:", &mut p.radius, 0.005..=100.0, d.radius);
        num_row(ui, t, "Length:", &mut p.length, 0.01..=100.0, d.length);
        num_row(ui, t, "Eyelet Radius:", &mut p.eyelet, 0.01..=100.0, d.eyelet);
        u_row(ui, t, "Sides:", &mut p.sides, d.sides, 6..=64);
    });
}

fn fastener(ui: &mut egui::Ui, t: &mut Track, f: &mut Fastener) {
    ui.label(RichText::new(f.kind.label()).strong());
    ui.horizontal(|ui| {
        for std in [ThreadStd::Metric, ThreadStd::Imperial] {
            if t.r(ui.radio_value(&mut f.standard, std, std.label())).changed() {
                f.size = f.size.min(std.sizes().len() - 1);
            }
        }
    });
    egui::ComboBox::from_id_salt("fastener_size").width(CONTENT_W - 12.0).selected_text(f.spec().name).show_ui(ui, |ui| {
        for (i, s) in f.standard.sizes().iter().enumerate() {
            t.r(ui.selectable_value(&mut f.size, i, format!("{}  (pitch {:.2} mm)", s.name, s.pitch)));
        }
    });
    let d = Fastener::new(f.kind);
    grid(ui, "fastener_params", |ui| {
        match f.kind {
            FastenerKind::Bolt => {
                num_row(ui, t, "Length (mm):", &mut f.length, 1.0..=500.0, d.length);
                num_row(ui, t, "Thread Length:", &mut f.thread_length, 0.5..=500.0, d.thread_length);
            }
            FastenerKind::TappedHole => {
                num_row(ui, t, "Depth (mm):", &mut f.length, 1.0..=500.0, d.length);
                num_row(ui, t, "Block Width:", &mut f.block, 1.0..=1000.0, d.block);
            }
            FastenerKind::Nut => {}
        }
        u_row(ui, t, "Sides:", &mut f.sides, d.sides, 12..=192);
        u_row(ui, t, "Thread Detail:", &mut f.detail, d.detail, 3..=24);
        num_row(ui, t, "Scale:", &mut f.scale, 0.01..=1000.0, d.scale);
    });
    t.r(ui.checkbox(&mut f.threads, "Model Threads"));
    let s = f.spec();
    ui.label(dim(&format!("Ø {:.2} mm, pitch {:.3} mm, hex {:.2} mm across flats. Scale 1 = true size (1 unit = 1 m).", s.d, s.pitch, s.flats)));
}

fn vec_row(ui: &mut egui::Ui, t: &mut Track, label: &str, v: &mut Vec3) {
    ui.label(label);
    ui.horizontal(|ui| {
        for c in [&mut v.x, &mut v.y, &mut v.z] {
            t.r(ui.add(egui::DragValue::new(c).speed(0.01).max_decimals(3)));
        }
    });
    ui.end_row();
}

fn hose(ui: &mut egui::Ui, t: &mut Track, h: &mut Hose) {
    ui.horizontal(|ui| {
        t.r(ui.radio_value(&mut h.curve, HoseCurve::Bezier, "Bezier"));
        t.r(ui.radio_value(&mut h.curve, HoseCurve::CatmullRom, "Catmull-Rom"));
    });
    grid(ui, "hose_params", |ui| {
        if h.top.is_none() {
            vec_row(ui, t, "Start:", &mut h.start);
        }
        if h.bottom.is_none() {
            vec_row(ui, t, "End:", &mut h.end);
        }
        num_row(ui, t, "Tension:", &mut h.tension, 0.0..=2.0, 0.5);
        num_row(ui, t, "Slack:", &mut h.slack, 0.0..=2.0, 0.0);
        num_row(ui, t, "Gravity Sag:", &mut h.sag, 0.0..=100.0, 0.4);
        num_row(ui, t, "Diameter:", &mut h.diameter, 0.001..=100.0, 0.12);
        u_row(ui, t, "Sides:", &mut h.sides, 12, 3..=64);
        u_row(ui, t, "Segments:", &mut h.segments, 32, 2..=512);
    });
    ui.label(RichText::new("Corrugated Rings").strong());
    grid(ui, "hose_rings", |ui| {
        u_row(ui, t, "Ring Count:", &mut h.rings, 0, 0..=200);
        num_row(ui, t, "Ring Depth:", &mut h.ring_depth, 0.0..=10.0, 0.015);
    });
    ui.horizontal(|ui| {
        t.r(ui.radio_value(&mut h.square_rings, false, "Round"));
        t.r(ui.radio_value(&mut h.square_rings, true, "Square"));
    });
}

/// The hose's end bindings: Pick / Free per end.
fn hose_ends(ui: &mut egui::Ui, ed: &mut Editor, h: &Hose) {
    for (top, end, label) in [(true, h.top, "Top End Object"), (false, h.bottom, "Bottom End Object")] {
        let name = end.and_then(|o| ed.scene.objects.get(o)).map_or("(free)".to_string(), |o| o.name.clone());
        ui.label(RichText::new(label).strong());
        ui.horizontal(|ui| {
            let picking = ed.tool == Tool::PickHoseEnd { top };
            let pick = ui.add_sized([CONTENT_W * 0.6, 24.0], egui::Button::new(format!("Pick: {name}")).selected(picking));
            if pick.clicked() {
                ed.tool = if picking { Tool::None } else { Tool::PickHoseEnd { top } };
            }
            if ui.add_enabled(end.is_some(), egui::Button::new("Free")).clicked() {
                ed.bind_hose(top, None);
            }
        });
    }
    ui.label(dim(
        "A bound hose leaves the top object down its -Z axis and enters the bottom object up its +Z; it flexes live as they move.",
    ));
}

/// Mech base object parameters.
pub fn base(ui: &mut egui::Ui, ed: &mut Editor) {
    let Some(id) = ed.current() else { return };
    let Some(mut p) = ed.scene.objects[id].mech().cloned() else { return };
    let mut t = Track::default();
    let title = match &p {
        MechPart::Gear(_) => "Gear Parameters",
        MechPart::Piston(_) => "Piston Parameters",
        MechPart::Hose(_) => "Hose Parameters",
        MechPart::Fastener(_) => "Fastener Parameters",
    };
    rollout(ui, title, true, |ui| match &mut p {
        MechPart::Gear(g) => gear(ui, &mut t, g),
        MechPart::Piston(pp) => piston(ui, &mut t, pp),
        MechPart::Hose(h) => hose(ui, &mut t, h),
        MechPart::Fastener(f) => fastener(ui, &mut t, f),
    });
    if let MechPart::Hose(h) = &p {
        rollout(ui, "Hose Ends", true, |ui| hose_ends(ui, ed, h));
    }
    if t.started {
        ed.checkpoint(title);
    }
    if t.changed {
        // The end bindings may have changed through the buttons above.
        let o = &mut ed.scene.objects[id];
        if let (Some(MechPart::Hose(cur)), MechPart::Hose(h)) = (o.mech(), &mut p) {
            (h.top, h.bottom) = (cur.top, cur.bottom);
        }
        if let Some(m) = o.mech_mut() {
            *m = p;
        }
        o.evaluate();
        ed.scene.update_hoses();
        ed.scene.propagate_from(id);
    }
}
