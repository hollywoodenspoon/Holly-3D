//! Top Main Toolbar (36px): menus, Q/W/E/R, coordinate system, pivot, snaps.

use super::theme;
use super::widgets::toggle_button;
use crate::editor::{CoordSystem, Editor, PivotMode, TransformMode};
use crate::scene::CloneKind;
use bevy_egui::egui::{self, RichText};

pub fn show(ui: &mut egui::Ui, ed: &mut Editor) -> egui::Rect {
    egui::Panel::top("main_toolbar")
        .resizable(false)
        .show_separator_line(false)
        .exact_size(36.0)
        .frame(egui::Frame::new().fill(theme::PANEL).inner_margin(egui::Margin::symmetric(8, 4)))
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                menus(ui, ed);
                ui.separator();
                for (mode, key, tip) in [
                    (TransformMode::Select, "Q", "Select Object"),
                    (TransformMode::Move, "W", "Select and Move"),
                    (TransformMode::Rotate, "E", "Select and Rotate"),
                    (TransformMode::Scale, "R", "Select and Scale"),
                ] {
                    if toggle_button(ui, ed.mode == mode, key).on_hover_text(tip).clicked() {
                        ed.mode = mode;
                    }
                }
                ui.separator();
                egui::ComboBox::from_id_salt("coord").width(70.0).selected_text(format!("{:?}", ed.coord)).show_ui(ui, |ui| {
                    for c in [CoordSystem::View, CoordSystem::World, CoordSystem::Local] {
                        ui.selectable_value(&mut ed.coord, c, format!("{c:?}"));
                    }
                });
                let pivot_label = |p: PivotMode| match p {
                    PivotMode::PivotPoint => "Pivot Point",
                    PivotMode::SelectionCenter => "Selection Center",
                    PivotMode::TransformCenter => "Transform Coord Center",
                };
                egui::ComboBox::from_id_salt("pivot").width(120.0).selected_text(pivot_label(ed.pivot_mode)).show_ui(ui, |ui| {
                    for p in [PivotMode::PivotPoint, PivotMode::SelectionCenter, PivotMode::TransformCenter] {
                        ui.selectable_value(&mut ed.pivot_mode, p, pivot_label(p));
                    }
                });
                ui.separator();
                let snap = toggle_button(ui, ed.snap, "S Snap").on_hover_text("3D Snaps (S) — right-click for Grid and Snap Settings");
                if snap.clicked() {
                    ed.snap = !ed.snap;
                }
                snap.context_menu(|ui| crate::ui::snaps::settings(ui, ed));
                if toggle_button(ui, ed.angle_snap, "A Angle").on_hover_text("Angle Snap 5° (A)").clicked() {
                    ed.angle_snap = !ed.angle_snap;
                }
                ui.separator();
                let any = !ed.scene.selected().is_empty();
                if ui.add_enabled(any, egui::Button::new("Mirror")).on_hover_text("Mirror the selection (live preview)").clicked() {
                    ed.open_mirror();
                }
                if ui.add_enabled(any, egui::Button::new("Array...")).on_hover_text("3ds Max Array dialog").clicked() {
                    ed.open_array();
                }
                if toggle_button(ui, ed.paint.active, "🖌 Paint")
                    .on_hover_text("Viewport Paint: VertexPaint / texture projection (PaintBox)")
                    .clicked()
                {
                    ed.toggle_paint();
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("▶ Quick Render (F9)").on_hover_text("Render the active viewport into the Rendered Frame Window").clicked()
                    {
                        ed.start_render();
                    }
                    if toggle_button(ui, ed.rfw.setup_open, "⚙ Render Setup (F10)").clicked() {
                        ed.rfw.setup_open = !ed.rfw.setup_open;
                    }
                    let open = ed.mat_editor.open;
                    if toggle_button(ui, open, "⚪ Material Editor (M)").on_hover_text("Compact Material Editor").clicked() {
                        ed.mat_editor.open = !open;
                    }
                });
            });
        })
        .response
        .rect
}

fn menus(ui: &mut egui::Ui, ed: &mut Editor) {
    ui.label(RichText::new("MiniMax").color(theme::AMBER).strong());
    ui.menu_button("File", |ui| super::file_menu::menu(ui, ed));
    ui.menu_button("Edit", |ui| {
        if ui.add_enabled(ed.history.can_undo(), egui::Button::new("Undo   Ctrl+Z")).clicked() {
            ed.undo();
            ui.close();
        }
        if ui.add_enabled(ed.history.can_redo(), egui::Button::new("Redo   Ctrl+Y")).clicked() {
            ed.redo();
            ui.close();
        }
        ui.separator();
        if ui.button("Clone").clicked() {
            ed.clone_selected(CloneKind::Copy);
            ui.close();
        }
        if ui.button("Delete   Del").clicked() {
            ed.delete_selection();
            ui.close();
        }
        ui.separator();
        if ui.button("Select All   Ctrl+A").clicked() {
            ed.select_all(true);
            ui.close();
        }
        if ui.button("Select None   Ctrl+D").clicked() {
            ed.select_all(false);
            ui.close();
        }
        if ui.button("Select Invert   Ctrl+I").clicked() {
            ed.invert_selection();
            ui.close();
        }
    });
    ui.menu_button("Tools", |ui| {
        if ui.button("Array...").clicked() {
            ed.open_array();
            ui.close();
        }
        if ui.button("Mirror...").clicked() {
            ed.open_mirror();
            ui.close();
        }
        if ui.button("Align...   Alt+A").clicked() {
            ed.start_align();
            ui.close();
        }
        if ui.button("Isolate Selection   Alt+Q").clicked() {
            ed.toggle_isolate();
            ui.close();
        }
    });
}
