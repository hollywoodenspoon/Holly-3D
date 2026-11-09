//! Edit Normals, Boolean and ProOptimizer rollouts.

use super::HALF;
use super::spline::{grid, mod_params};
use crate::editor::{Editor, Tool};
use crate::modifiers::{BoolOp, ModifierKind, NormalSelBy};
use crate::ui::command::CONTENT_W;
use crate::ui::theme;
use crate::ui::widgets::{action, dim, max_spinner_range, rollout};
use bevy_egui::egui::{self, RichText};

/// Runs an Edit Normals command on the active modifier with its input mesh.
fn normals_cmd(ed: &mut Editor, label: &str, f: impl FnOnce(&mut crate::modifiers::EditNormalsMod, &crate::mesh::EditablePoly)) {
    ed.edit_active_modifier(label, |k, input| {
        if let ModifierKind::EditNormals(e) = k {
            f(e, input);
        }
    });
}

pub fn edit_normals(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Edit Normals Parameters", true, |ui| {
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::EditNormals(e) = k else { return };
            ui.label(RichText::new("Select By").strong());
            ui.horizontal(|ui| {
                for by in NormalSelBy::ALL {
                    t.r(ui.radio_value(&mut e.select_by, by, by.label()));
                }
            });
            grid(ui, "edit_normals", |ui| {
                ui.label("Display Length:");
                t.r(max_spinner_range(ui, &mut e.display_length, 0.005, 0.1, 0.001..=100.0));
                ui.end_row();
            });
        });
        ui.horizontal(|ui| {
            if action(ui, "Break", true, HALF) {
                normals_cmd(ed, "Break Normals", |e, m| e.break_selected(m));
            }
            if action(ui, "Unify", true, HALF) {
                normals_cmd(ed, "Unify Normals", |e, m| e.unify_selected(m));
            }
        });
        ui.horizontal(|ui| {
            if action(ui, "Reset", true, HALF) {
                normals_cmd(ed, "Reset Normals", |e, _| e.reset_selected());
            }
            if action(ui, "Make Explicit", true, HALF) {
                normals_cmd(ed, "Make Explicit", |e, m| e.make_explicit(m));
            }
        });
        if let Some(ModifierKind::EditNormals(e)) = ed.current_obj().and_then(|o| o.active_modifier()) {
            ui.label(dim(&format!("{} normals selected, {} explicit", e.selected.len(), e.explicit.len())));
        }
        ui.label(dim("Click or drag to select normals; press E and rotate to aim them. Reset with nothing selected clears all."));
    });
}

pub fn boolean(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Boolean Parameters", true, |ui| {
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::Boolean(b) = k else { return };
            ui.label(RichText::new("Operation").strong());
            let before = b.op;
            egui::Grid::new("bool_ops").num_columns(2).show(ui, |ui| {
                for (i, op) in BoolOp::ALL.into_iter().enumerate() {
                    t.r(ui.radio_value(&mut b.op, op, op.label()));
                    if i % 2 == 1 {
                        ui.end_row();
                    }
                }
            });
            // changing the operation also re-labels the highlighted operand
            if b.op != before
                && let Some(o) = b.selected.and_then(|i| b.operands.get_mut(i))
            {
                o.op = b.op;
            }
            t.r(ui.checkbox(&mut b.merge_coplanar, "Merge Coplanar Faces"));
        });
        let picking = ed.tool == Tool::PickBoolean;
        if ui
            .add_sized([CONTENT_W, 28.0], egui::Button::new(if picking { "Picking Operands…" } else { "Pick Operand B" }).selected(picking))
            .on_hover_text("Click objects in a viewport; each becomes an operand and is removed from the scene")
            .clicked()
        {
            ed.tool = if picking { Tool::None } else { Tool::PickBoolean };
        }
        ui.label(RichText::new("Operands").strong());
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::Boolean(b) = k else { return };
            egui::Frame::new().fill(theme::INPUT).corner_radius(4).inner_margin(6).show(ui, |ui| {
                ui.set_width(CONTENT_W - 12.0);
                ui.set_min_height(60.0);
                if b.operands.is_empty() {
                    ui.label(dim("A: this object. No operands picked yet."));
                }
                for i in 0..b.operands.len() {
                    ui.horizontal(|ui| {
                        t.r(ui.checkbox(&mut b.operands[i].enabled, ""));
                        let o = &b.operands[i];
                        let text = format!("{}  —  {}", o.name, o.op.label());
                        if ui.selectable_label(b.selected == Some(i), text).clicked() {
                            b.selected = Some(i);
                            b.op = b.operands[i].op;
                            t.started = true;
                            t.changed = true;
                        }
                    });
                }
            });
        });
        let has_sel = matches!(
            ed.current_obj().and_then(|o| o.active_modifier()),
            Some(ModifierKind::Boolean(b)) if b.selected.is_some_and(|i| i < b.operands.len())
        );
        ui.horizontal(|ui| {
            if action(ui, "Remove Operand", has_sel, HALF) {
                ed.edit_active_modifier("Remove Operand", |k, _| {
                    if let ModifierKind::Boolean(b) = k {
                        b.take_selected();
                    }
                });
            }
            if action(ui, "Extract Operand", has_sel, HALF) {
                ed.extract_boolean_operand();
            }
        });
        ui.label(dim("Operands must be closed (manifold) meshes; open ones are skipped."));
    });
}

pub fn pro_optimizer(ui: &mut egui::Ui, ed: &mut Editor) {
    rollout(ui, "Optimization Level", true, |ui| {
        if action(ui, "Calculate", true, CONTENT_W) {
            ed.edit_active_modifier("Calculate", |k, input| {
                if let ModifierKind::ProOptimizer(p) = k {
                    p.calculate(input);
                }
            });
        }
        let after = ed.current_obj().map_or(0, |o| o.level_mesh.vertices.len());
        mod_params(ui, ed, |ui, k, t| {
            let ModifierKind::ProOptimizer(p) = k else { return };
            ui.add_enabled_ui(p.calculated, |ui| {
                grid(ui, "proopt", |ui| {
                    ui.label("Vertex %:");
                    t.r(max_spinner_range(ui, &mut p.percent, 0.2, 100.0, 1.0..=100.0));
                    ui.end_row();
                    ui.label("Vertex Count:");
                    let mut n = p.target_verts() as u32;
                    let max = p.source_verts.max(1) as u32;
                    if t.r(max_spinner_range(ui, &mut n, 1.0, max, 1..=max)).changed() {
                        p.set_target_verts(n as usize);
                    }
                    ui.end_row();
                });
            });
            ui.label(RichText::new("Options").strong());
            ui.horizontal(|ui| {
                t.r(ui.checkbox(&mut p.keep_materials, "Keep Material Boundaries"));
            });
            ui.horizontal(|ui| {
                t.r(ui.checkbox(&mut p.keep_uvs, "Keep UV Boundaries"));
                t.r(ui.checkbox(&mut p.protect_borders, "Protect Borders"));
            });
            ui.horizontal(|ui| {
                t.r(ui.checkbox(&mut p.keep_colors, "Keep Vertex Colors"));
                t.r(ui.checkbox(&mut p.keep_normals, "Keep Normals"));
            });
            let status = if p.calculated {
                format!("Before: {} verts   After: {} verts", p.source_verts, after)
            } else {
                "Press Calculate to analyse the input.".into()
            };
            ui.label(dim(&status));
        });
    });
}
