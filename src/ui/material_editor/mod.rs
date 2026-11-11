//! 3ds Max Compact Material Editor (M): 24 sample slots, material toolbar,
//! preset / shading-model pickers and the Compact or Slate parameter views.

mod craft_params;
pub mod library;
mod maps;
mod proc_rollout;
mod multi;
mod params;
mod slate;

use crate::editor::{Editor, Tool};
use crate::materials::{MatId, Material, MultiPreset, Preset, SLOT_COUNT, ShadingModel, preview};
use crate::ui::theme;
use crate::ui::widgets::{Track, dim, toggle_button};
use bevy_egui::egui::{self, Color32, RichText};
use std::collections::HashMap;

const CELL: f32 = 58.0;
const PREVIEW_PX: u32 = 64;
const COLS: usize = 6;

/// UI-only state: cached slot previews and the Slate canvas layout.
#[derive(Default)]
pub struct MatEditorUi {
    previews: HashMap<MatId, (Material, egui::TextureHandle)>,
    slate: slate::SlateState,
    pub library: library::LibraryUi,
}

impl MatEditorUi {
    /// Preview texture for a material, re-rendered only when it changed.
    fn preview(&mut self, ctx: &egui::Context, id: MatId, m: &Material) -> egui::TextureId {
        if let Some((cached, tex)) = self.previews.get(&id) {
            if cached == m {
                return tex.id();
            }
            // Procedural maps cook on a worker: show the old swatch until they are ready.
            if !crate::materials::proc_engine::worker::ready(m) {
                ctx.request_repaint();
                return tex.id();
            }
        }
        // A Multi/Sub-Object slot previews its ID 1 sub-material.
        let px = preview::render_preview(m.for_id(1), PREVIEW_PX);
        let bytes: Vec<u8> = px.iter().flatten().copied().collect();
        let img = egui::ColorImage::from_rgba_unmultiplied([PREVIEW_PX as usize; 2], &bytes);
        let tex = ctx.load_texture(format!("mat_slot_{id:?}"), img, egui::TextureOptions::LINEAR);
        let tid = tex.id();
        self.previews.insert(id, (m.clone(), tex));
        tid
    }
}

/// Edits a clone of the edited (sub-)material and writes it back on change
/// (one undo step per gesture).
pub(super) fn edit(ed: &mut Editor, body: impl FnOnce(&mut Material, &mut Track) -> Option<String>) {
    let sub = ed.mat_editor.sub;
    edit_root(ed, |m, t| body(m.target_mut(sub), t));
}

/// Like [`edit`] on the slot's material itself (name, type, sub list).
pub(super) fn edit_root(ed: &mut Editor, body: impl FnOnce(&mut Material, &mut Track) -> Option<String>) {
    let id = ed.active_material();
    let mut m = ed.scene.materials.mats[id].clone();
    let mut t = Track::default();
    let msg = body(&mut m, &mut t);
    if t.started {
        ed.checkpoint("Material Parameters");
    }
    if t.changed {
        ed.scene.materials.mats[id] = m;
    }
    if let Some(s) = msg {
        ed.status = s;
    }
}

fn slot_grid(ui: &mut egui::Ui, ed: &mut Editor, st: &mut MatEditorUi) {
    let ctx = ui.ctx().clone();
    let rows = SLOT_COUNT / COLS;
    for r in 0..rows {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            for c in 0..COLS {
                let i = r * COLS + c;
                let id = ed.scene.materials.slot(i);
                let tex = st.preview(&ctx, id, &ed.scene.materials.mats[id]);
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(CELL, CELL), egui::Sense::click());
                let p = ui.painter();
                p.image(tex, rect, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                if ed.material_is_hot(id) {
                    // Hot material: white corner triangles like 3ds Max.
                    let k = 9.0;
                    for (corner, dx, dy) in [
                        (rect.left_top(), 1.0, 1.0),
                        (rect.right_top(), -1.0, 1.0),
                        (rect.left_bottom(), 1.0, -1.0),
                        (rect.right_bottom(), -1.0, -1.0),
                    ] {
                        let pts = vec![corner, corner + egui::vec2(k * dx, 0.0), corner + egui::vec2(0.0, k * dy)];
                        p.add(egui::Shape::convex_polygon(pts, Color32::WHITE, egui::Stroke::NONE));
                    }
                }
                let active = ed.mat_editor.slot == i;
                let stroke = if active { egui::Stroke::new(2.0, theme::amber()) } else { egui::Stroke::new(1.0, theme::border()) };
                p.rect_stroke(rect, 2.0, stroke, egui::StrokeKind::Inside);
                if resp.clicked() && ed.mat_editor.slot != i {
                    (ed.mat_editor.slot, ed.mat_editor.sub) = (i, 0);
                }
                resp.on_hover_text(&ed.scene.materials.mats[id].name);
            }
        });
    }
}

fn material_toolbar(ui: &mut egui::Ui, ed: &mut Editor) {
    ui.horizontal(|ui| {
        let show = ed.scene.materials.mats[ed.active_material()].show_in_viewport;
        if ui.button("⬇ Assign").on_hover_text("Assign Material to Selection").clicked() {
            ed.assign_material_to_selection();
        }
        if toggle_button(ui, show, "👁 Show").on_hover_text("Show Shaded Material in Viewport").clicked() {
            ed.toggle_material_in_viewport();
        }
        if ui.button("⟲ Reset").on_hover_text("Reset Material Slot").clicked() {
            ed.reset_material_slot();
        }
        if ui.button("Copy").on_hover_text("Make Material Copy").clicked() {
            ed.make_material_copy();
        }
        let picking = ed.tool == Tool::PickMaterial;
        if toggle_button(ui, picking, "✒ Pick").on_hover_text("Pick Material from Object (eyedropper)").clicked() {
            ed.tool = if picking { Tool::None } else { Tool::PickMaterial };
        }
        if toggle_button(ui, ed.mat_editor.library, "📚 Library").on_hover_text("Material Library: ~280 ready-made materials").clicked() {
            ed.mat_editor.library = !ed.mat_editor.library;
        }
    });
    ui.horizontal(|ui| {
        ui.label("Mode:");
        if toggle_button(ui, !ed.mat_editor.slate, "Compact").clicked() {
            ed.mat_editor.slate = false;
        }
        if toggle_button(ui, ed.mat_editor.slate, "Slate Node Graph").clicked() {
            ed.mat_editor.slate = true;
        }
    });
}

fn header(ui: &mut egui::Ui, ed: &mut Editor) {
    let is_multi = ed.scene.materials.mats[ed.active_material()].is_multi();
    let (mut kind, mut pack) = (None, None);
    edit_root(ed, |m, t| {
        egui::Grid::new("mat_header").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
            ui.label("Name");
            t.r(ui.add(egui::TextEdit::singleline(&mut m.name).desired_width(f32::INFINITY)));
            ui.end_row();
            ui.label("Type");
            let label = if is_multi { "Multi/Sub-Object" } else { "Standard Material" };
            egui::ComboBox::from_id_salt("mat_type").width(ui.available_width()).selected_text(label).show_ui(ui, |ui| {
                for (multi, text) in [(false, "Standard Material"), (true, "Multi/Sub-Object")] {
                    if ui.selectable_label(multi == is_multi, text).clicked() && multi != is_multi {
                        kind = Some(multi);
                    }
                }
            });
            ui.end_row();
            if is_multi {
                ui.label("Pack Preset");
                egui::ComboBox::from_id_salt("multi_preset").width(ui.available_width()).selected_text("Load Pack…").show_ui(ui, |ui| {
                    for p in MultiPreset::ALL {
                        if ui.selectable_label(false, p.label()).clicked() {
                            pack = Some(p);
                        }
                    }
                });
                ui.end_row();
            }
        });
        None
    });
    if let Some(multi) = kind {
        ed.set_material_multi(multi);
    }
    if let Some(p) = pack {
        ed.apply_multi_preset(p);
    }
}

/// Quick Preset + Shading model of the edited (sub-)material.
fn look(ui: &mut egui::Ui, ed: &mut Editor) {
    let (mut preset, mut library) = (None, false);
    edit(ed, |m, t| {
        egui::Grid::new("mat_look").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
            ui.label("Quick Preset");
            egui::ComboBox::from_id_salt("mat_preset").width(ui.available_width()).selected_text("1-Click Setup…").show_ui(ui, |ui| {
                for p in Preset::ALL {
                    if ui.selectable_label(false, p.label()).clicked() {
                        preset = Some(p);
                    }
                }
                ui.separator();
                if ui.selectable_label(false, "📚 More in the Material Library...").clicked() {
                    library = true;
                }
            });
            ui.end_row();
            ui.label("Shading");
            egui::ComboBox::from_id_salt("mat_model").width(ui.available_width()).selected_text(m.model.label()).show_ui(ui, |ui| {
                for s in ShadingModel::ALL {
                    t.r(ui.selectable_value(&mut m.model, s, s.label()));
                }
            });
            ui.end_row();
        });
        None
    });
    if let Some(p) = preset {
        ed.apply_preset(p);
    }
    if library {
        ed.mat_editor.library = true;
    }
}

pub fn show(ctx: &egui::Context, ed: &mut Editor, st: &mut MatEditorUi) {
    if !ed.mat_editor.open {
        return;
    }
    let mut open = true;
    let width = COLS as f32 * (CELL + 4.0) + 12.0;
    egui::Window::new(RichText::new("Material Editor").strong())
        .id(egui::Id::new("material_editor"))
        .open(&mut open)
        .default_pos([300.0, 70.0])
        .default_width(width)
        .min_width(width)
        .default_height(780.0)
        .resizable(true)
        .show(ctx, |ui| {
            slot_grid(ui, ed, st);
            material_toolbar(ui, ed);
            ui.separator();
            header(ui, ed);
            ui.separator();
            multi::rollout_ui(ui, ed);
            look(ui, ed);
            if ed.mat_editor.slate {
                slate::show(ui, ed, &mut st.slate);
            } else {
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    proc_rollout::rollout(ui, ed);
                    params::basic(ui, ed);
                    params::outline(ui, ed);
                    maps::rollout(ui, ed);
                    ui.label(dim("Right-click any spinner to reset it."));
                });
            }
        });
    if !open {
        ed.mat_editor.open = false;
        if ed.tool == Tool::PickMaterial {
            ed.tool = Tool::None;
        }
    }
    // Drop previews of materials that left the slots.
    let live: Vec<MatId> = ed.scene.materials.slots.clone();
    st.previews.retain(|k, _| live.contains(k));
}
