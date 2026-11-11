//! Compact editor "Procedural Pattern & Weathering" rollout: pick a
//! pattern and style, set its spinners and weathering sliders, and the
//! engine fills colour, normal, roughness and cavity AO. Works on every
//! Standard material and every Multi/Sub-Object sub-material.

use super::edit;
use super::params::{color, num};
use crate::editor::Editor;
use crate::materials::proc_engine::{Pattern, worker};
use crate::ui::widgets::{Track, dim, max_spinner_range, rollout as card};
use bevy_egui::egui;

/// A 0..100 % slider over a 0..1 value.
fn percent(ui: &mut egui::Ui, t: &mut Track, label: &str, tip: &str, v: &mut f32) {
    ui.label(label).on_hover_text(tip);
    let mut pct = *v * 100.0;
    let r = t.r(ui.add(egui::Slider::new(&mut pct, 0.0..=100.0).suffix(" %").fixed_decimals(0))).on_hover_text(tip);
    if r.secondary_clicked() {
        pct = 0.0;
        t.changed = true;
    }
    *v = pct / 100.0;
    ui.end_row();
}

pub fn rollout(ui: &mut egui::Ui, ed: &mut Editor) {
    card(ui, "Procedural Pattern & Weathering", true, |ui| {
        edit(ed, |m, t| {
            let (mut pattern, mut style) = (m.proc.pattern, m.proc.style);
            egui::Grid::new("proc_pattern").num_columns(2).spacing([10.0, 6.0]).min_col_width(110.0).show(ui, |ui| {
                ui.label("Pattern");
                egui::ComboBox::from_id_salt("proc_pattern_kind").width(190.0).selected_text(pattern.label()).show_ui(ui, |ui| {
                    for p in Pattern::ALL {
                        if t.r(ui.selectable_value(&mut pattern, p, p.label())).clicked() {
                            style = 0;
                        }
                    }
                });
                ui.end_row();
                let styles = pattern.styles();
                if !styles.is_empty() {
                    ui.label("Style");
                    let current = styles[(style as usize).min(styles.len() - 1)].label;
                    egui::ComboBox::from_id_salt("proc_pattern_style").width(190.0).selected_text(current).show_ui(ui, |ui| {
                        for (i, s) in styles.iter().enumerate() {
                            t.r(ui.selectable_value(&mut style, i as u8, s.label));
                        }
                    });
                    ui.end_row();
                }
            });
            if (pattern, style) != (m.proc.pattern, m.proc.style) {
                // One click: the style's colours, gap, bevel, roughness and metalness.
                m.set_pattern(pattern, style);
                (t.started, t.changed) = (true, true);
            }
            let Some(def) = m.proc.style_def() else {
                ui.label(dim("Pick a pattern to texture this material procedurally. Bitmap and procedural maps in the Maps rollout still apply on top."));
                return None;
            };
            let p = &mut m.proc;
            egui::Grid::new("proc_spinners").num_columns(2).spacing([10.0, 6.0]).min_col_width(110.0).show(ui, |ui| {
                num(ui, t, "Tiling / Scale", &mut p.tiling, 0.25..=16.0, 1.0);
                ui.label("Seed");
                ui.horizontal(|ui| {
                    t.r(max_spinner_range(ui, &mut p.seed, 0.2, 0, 0..=9999));
                    if ui.button("🎲").on_hover_text("New random seed").clicked() {
                        p.seed = rand::random::<u32>() % 10_000;
                        (t.started, t.changed) = (true, true);
                    }
                });
                ui.end_row();
                ui.add_enabled_ui(def.user_gap, |ui| ui.label("Gap / Mortar Width"));
                ui.add_enabled_ui(def.user_gap, |ui| t.r(max_spinner_range(ui, &mut p.gap, 0.003, 0.1, 0.0..=0.6)));
                ui.end_row();
                ui.add_enabled_ui(def.user_gap, |ui| ui.label("Gap / Bevel Depth"));
                ui.add_enabled_ui(def.user_gap, |ui| t.r(max_spinner_range(ui, &mut p.bevel, 0.005, 0.15, 0.005..=2.0)));
                ui.end_row();
                num(ui, t, "Color Variation", &mut p.color_variation, 0.0..=1.0, 0.3);
                num(ui, t, "Surface Relief", &mut p.relief, 0.0..=4.0, 1.0);
                color(ui, t, "Surface Color", &mut p.color_a);
                color(ui, t, "Gap / Secondary Color", &mut p.color_b);
            });
            ui.separator();
            ui.label(egui::RichText::new("Live Weathering").strong());
            egui::Grid::new("proc_weather").num_columns(2).spacing([10.0, 6.0]).min_col_width(110.0).show(ui, |ui| {
                percent(ui, t, "Edge Wear / Chipping", "Slope-blurred wear on raised edges, baring the metal or stone underneath", &mut p.edge_wear);
                percent(ui, t, "Cavity Dirt / Grunge", "Height-blended grime that settles in recesses first", &mut p.cavity_dirt);
                percent(ui, t, "Moss / Overgrowth", "Height-blended moss that grows out of gaps and cracks", &mut p.moss);
                percent(ui, t, "Scorch / Burn / Rust", "Charring on stone and wood; rust on metal", &mut p.scorch);
                percent(ui, t, "Wetness / Puddles", "Lowers roughness and levels flat puddles in low areas", &mut p.wetness);
            });
            t.r(ui.checkbox(&mut p.auto_pbr, "Auto-Derive Normal, Roughness & Cavity AO"))
                .on_hover_text("Builds the normal map, cavity ambient occlusion and roughness variation from the pattern's height map");
            if worker::busy() > 0 {
                ui.label(dim("Cooking maps in the background..."));
                ui.ctx().request_repaint();
            }
            None
        });
    });
}

#[cfg(test)]
mod tests {
    use crate::editor::Editor;
    use crate::materials::bake::{bake_base, bake_metal_rough, bake_normal};
    use crate::materials::proc_engine::Pattern;
    use crate::materials::shading::surface;
    use bevy::math::{Vec2, Vec3, Vec4};

    #[test]
    fn compact_edits_set_the_pattern_with_undo_and_fill_the_pbr_channels() {
        let mut ed = Editor::default();
        let before = ed.editing_material().clone();
        assert!(bake_normal(&before, 16).is_none() && !before.proc.active());
        // Picking a pattern is one undoable step that also sets the style's look.
        ed.edit_material(Some("Procedural Pattern"), |m| m.set_pattern(Pattern::Bricks, 0));
        let m = ed.editing_material().clone();
        assert!(m.proc.active() && m.roughness == 0.85 && m.proc.color_a == [0.6, 0.26, 0.17]);
        // Base colour, roughness, AO and normal all come from the pattern now.
        let brick = surface(&m, Vec2::new(0.125, 0.0625), Vec3::ZERO, Vec4::ONE);
        let joint = surface(&m, Vec2::new(0.3, 0.001), Vec3::ZERO, Vec4::ONE);
        assert!(brick.base.x > brick.base.z * 2.0, "red brick: {:?}", brick.base);
        assert!(joint.ao < brick.ao && joint.roughness > brick.roughness);
        assert!(bake_base(&m, 16).is_some() && bake_metal_rough(&m, 16).is_some() && bake_normal(&m, 16).is_some());
        // A spinner edit changes the look; weathering survives a style change.
        ed.edit_material(None, |m| (m.proc.tiling, m.proc.moss) = (2.0, 0.5));
        assert_ne!(surface(ed.editing_material(), Vec2::new(0.125, 0.0625), Vec3::ZERO, Vec4::ONE), brick);
        ed.edit_material(Some("Procedural Pattern"), |m| m.set_pattern(Pattern::Bricks, 2));
        let p = &ed.editing_material().proc;
        assert_eq!((p.style, p.tiling, p.moss), (2, 2.0, 0.5));
        // Auto-derive off: no normal map.
        ed.edit_material(None, |m| m.proc.auto_pbr = false);
        assert!(bake_normal(ed.editing_material(), 16).is_none());
        // Back to no pattern, then undo both pattern picks.
        ed.edit_material(Some("Procedural Pattern"), |m| m.set_pattern(Pattern::None, 0));
        assert!(!ed.editing_material().proc.active());
        for _ in 0..3 {
            ed.undo();
        }
        assert_eq!(*ed.editing_material(), before);
    }
}
