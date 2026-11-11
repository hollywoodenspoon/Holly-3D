//! F9 "Color Grading & Camera RAW" sidebar: tone and presence, white
//! balance with split-toning colour wheels, and lens / film effects. Every
//! change re-grades the image at once, without re-rendering.

use crate::editor::Editor;
use crate::render::grading::{Grading, NEUTRAL_K, Wheel};
use crate::ui::widgets::dim;
use bevy_egui::egui::{self, Color32, Pos2, Sense, Stroke, Vec2};

/// A labelled slider; right-click resets it to `default` (3ds Max spinner rule).
fn slider(ui: &mut egui::Ui, label: &str, v: &mut f32, range: std::ops::RangeInclusive<f32>, default: f32, tip: &str) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.add_sized([86.0, 18.0], egui::Label::new(label));
        let mut r = ui.add(egui::Slider::new(v, range).max_decimals(0));
        if r.secondary_clicked() {
            *v = default;
            r.mark_changed();
        }
        changed = r.on_hover_text(format!("{tip} • Right-click resets")).changed();
    });
    changed
}

fn hue_color(deg: f32, amount: f32) -> Color32 {
    let c = egui::ecolor::Hsva::new(deg / 360.0, amount.clamp(0.0, 1.0), 1.0, 1.0);
    c.into()
}

/// A hue/amount colour wheel: the angle picks the hue, the distance from
/// the centre the amount. Right-click clears it.
pub fn color_wheel(ui: &mut egui::Ui, label: &str, w: &mut Wheel) -> bool {
    let mut changed = false;
    ui.vertical(|ui| {
        ui.label(label);
        let size = 74.0;
        let (rect, r) = ui.allocate_exact_size(Vec2::splat(size), Sense::click_and_drag());
        let (c, rad) = (rect.center(), size * 0.5 - 2.0);
        let p = ui.painter();
        // Rings of hue segments, paler towards the centre.
        let (rings, segs) = (6, 36);
        for ri in 0..rings {
            let (r0, r1) = (rad * ri as f32 / rings as f32, rad * (ri + 1) as f32 / rings as f32);
            for si in 0..segs {
                let (a0, a1) = (si as f32 / segs as f32 * std::f32::consts::TAU, (si + 1) as f32 / segs as f32 * std::f32::consts::TAU);
                let at = |r: f32, a: f32| c + Vec2::new(a.cos(), -a.sin()) * r;
                let col = hue_color((si as f32 + 0.5) / segs as f32 * 360.0, (ri as f32 + 0.5) / rings as f32 * 0.8);
                p.add(egui::Shape::convex_polygon(vec![at(r0, a0), at(r1, a0), at(r1, a1), at(r0, a1)], col, Stroke::NONE));
            }
        }
        p.circle_stroke(c, rad, Stroke::new(1.0, Color32::from_gray(20)));
        let a = w.hue.to_radians();
        let dot: Pos2 = c + Vec2::new(a.cos(), -a.sin()) * rad * w.amount.clamp(0.0, 1.0);
        p.circle(dot, 4.0, hue_color(w.hue, w.amount), Stroke::new(1.5, Color32::WHITE));
        if let Some(m) = r.interact_pointer_pos()
            && (r.dragged() || r.clicked())
        {
            let d = m - c;
            w.hue = (-d.y).atan2(d.x).to_degrees().rem_euclid(360.0);
            w.amount = (d.length() / rad).clamp(0.0, 1.0);
            changed = true;
        }
        if r.secondary_clicked() {
            *w = Wheel::default();
            changed = true;
        }
        r.on_hover_text(format!("Hue {:.0}° • Amount {:.0}% • Right-click clears", w.hue, w.amount * 100.0));
    });
    changed
}

/// The grading controls. Returns true when anything changed.
pub fn controls(ui: &mut egui::Ui, g: &mut Grading) -> bool {
    let mut ch = false;
    ui.horizontal(|ui| {
        ch |= ui.checkbox(&mut g.enabled, "Enable Grading").changed();
        if ui.button("Reset All").on_hover_text("Back to the untouched render").clicked() {
            *g = Grading { enabled: g.enabled, ..Default::default() };
            ch = true;
        }
    });
    egui::CollapsingHeader::new("Tone & Presence").default_open(true).show(ui, |ui| {
        let pct = -100.0..=100.0;
        ch |= slider(ui, "Highlights", &mut g.highlights, pct.clone(), 0.0, "Recover (-) or brighten (+) the bright areas");
        ch |= slider(ui, "Shadows", &mut g.shadows, pct.clone(), 0.0, "Deepen (-) or open up (+) the dark areas");
        ch |= slider(ui, "Whites", &mut g.whites, pct.clone(), 0.0, "The white point");
        ch |= slider(ui, "Blacks", &mut g.blacks, pct.clone(), 0.0, "The black point");
        ch |= slider(ui, "Clarity", &mut g.clarity, pct.clone(), 0.0, "Local contrast of mid-tone detail");
        ch |= slider(ui, "Vibrance", &mut g.vibrance, pct.clone(), 0.0, "Saturation that spares already-strong colours");
        ch |= slider(ui, "Saturation", &mut g.saturation, pct, 0.0, "Colour strength of everything (-100 = black and white)");
    });
    egui::CollapsingHeader::new("White Balance & Split Toning").default_open(true).show(ui, |ui| {
        ch |= slider(ui, "Temp (K)", &mut g.temperature, 2000.0..=15000.0, NEUTRAL_K, "Light the image is balanced for: lower is cooler, higher is warmer");
        ch |= slider(ui, "Tint", &mut g.tint, -100.0..=100.0, 0.0, "Green (-) ↔ Magenta (+)");
        ui.horizontal(|ui| {
            ch |= color_wheel(ui, "Shadows", &mut g.shadow_wheel);
            ch |= color_wheel(ui, "Midtones", &mut g.midtone_wheel);
            ch |= color_wheel(ui, "Highlights", &mut g.highlight_wheel);
        });
    });
    egui::CollapsingHeader::new("Lens & Film FX").default_open(true).show(ui, |ui| {
        ch |= slider(ui, "Vignette", &mut g.vignette, -100.0..=100.0, 0.0, "Darken (-) or lighten (+) the corners");
        ch |= slider(ui, "Midpoint", &mut g.vignette_midpoint, 0.0..=100.0, 50.0, "How far from the centre the vignette starts");
        ch |= slider(ui, "Chromatic Ab.", &mut g.chromatic_aberration, 0.0..=100.0, 0.0, "Red / blue lens fringing towards the corners");
        ch |= slider(ui, "Film Grain", &mut g.grain, 0.0..=100.0, 0.0, "Fine luminance noise, strongest in the mid-tones");
        if g.grain > 0.0 && ui.button("🎲 New Grain").on_hover_text("A different grain pattern").clicked() {
            g.grain_seed = g.grain_seed.wrapping_add(1);
            ch = true;
        }
    });
    ch
}

/// The collapsible sidebar inside the F9 window.
pub fn sidebar(ui: &mut egui::Ui, ed: &mut Editor) {
    egui::Panel::right("rfw_grading").resizable(false).exact_size(300.0).show(ui, |ui| {
        ui.heading("Color Grading & Camera RAW");
        ui.label(dim("Changes update the image live — no re-render."));
        egui::ScrollArea::vertical().show(ui, |ui| {
            if controls(ui, &mut ed.render.post.grade) {
                ed.retone_render();
            }
        });
    });
}
