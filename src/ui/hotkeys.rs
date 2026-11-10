//! 3ds Max keyboard shortcuts (never Blender's modal G/R/S).

use crate::editor::{Editor, TransformMode};
use crate::mesh::SubObject;
use crate::mesh::ops::PolyOp;
use crate::viewport::layout::Shading;
use bevy_egui::egui::{self, Key};

pub fn handle(ctx: &egui::Context, ed: &mut Editor) {
    if ctx.egui_wants_keyboard_input() {
        return;
    }
    // Array / Mirror dialogs own the keyboard: Esc cancels, Enter confirms.
    if ed.array_dialog.is_some() || ed.mirror_dialog.is_some() {
        let (esc, enter) = ctx.input(|i| (i.key_pressed(Key::Escape), i.key_pressed(Key::Enter)));
        match (esc, enter, ed.array_dialog.is_some()) {
            (true, _, true) => ed.cancel_array(),
            (true, _, false) => ed.cancel_mirror(),
            (_, true, true) => ed.confirm_array(),
            (_, true, false) => ed.confirm_mirror(),
            _ => {}
        }
        return;
    }
    let (held, keys): (egui::Modifiers, Vec<(Key, egui::Modifiers)>) = ctx.input(|i| {
        let keys = i
            .events
            .iter()
            .filter_map(|e| match e {
                egui::Event::Key { key, pressed: true, repeat: false, modifiers, .. } => Some((*key, *modifiers)),
                _ => None,
            })
            .collect();
        (i.modifiers, keys)
    });
    for (key, ev_mods) in keys {
        // Combine the event's modifiers with the held state (robust to event ordering).
        let m = egui::Modifiers {
            alt: ev_mods.alt || held.alt,
            ctrl: ev_mods.ctrl || held.ctrl,
            shift: ev_mods.shift || held.shift,
            ..ev_mods
        };
        let ctrl = m.command || m.ctrl;
        if m.alt && !matches!(key, Key::W | Key::X | Key::Q | Key::R | Key::L) {
            continue;
        }
        match key {
            Key::Z if ctrl => ed.undo(),
            Key::S if ctrl && m.shift => super::file_menu::save_as(ed),
            Key::S if ctrl => super::file_menu::save(ed),
            Key::O if ctrl => super::file_menu::open(ed),
            Key::L if ctrl => ed.toggle_default_lights(),
            Key::Y if ctrl => ed.redo(),
            Key::A if ctrl => ed.select_all(true),
            Key::D if ctrl => ed.select_all(false),
            Key::I if ctrl => ed.invert_selection(),
            Key::W if m.alt => {
                ed.viewports.maximized = !ed.viewports.maximized;
            }
            Key::X if m.alt => {
                let ids = ed.scene.selected();
                for id in ids {
                    let o = &mut ed.scene.objects[id];
                    o.see_through = !o.see_through;
                }
            }
            Key::Q if m.alt => ed.toggle_isolate(),
            Key::R if m.alt => ed.edit_selection(|mesh, _| mesh.select_ring()),
            Key::L if m.alt => ed.edit_selection(|mesh, _| mesh.select_loop()),
            Key::Q => ed.mode = TransformMode::Select,
            Key::W => ed.mode = TransformMode::Move,
            Key::E => ed.mode = TransformMode::Rotate,
            Key::R => ed.mode = TransformMode::Scale,
            Key::C if !ctrl => ed.camera_hotkey(),
            Key::Num1 => ed.set_sub_object(SubObject::Vertex),
            Key::Num2 => ed.set_sub_object(SubObject::Edge),
            Key::Num3 => ed.set_sub_object(SubObject::Border),
            Key::Num4 => ed.set_sub_object(SubObject::Polygon),
            Key::Num5 => ed.set_sub_object(SubObject::Element),
            Key::Num6 => ed.set_sub_object(SubObject::Object),
            Key::Z => ed.zoom_extents(m.shift),
            Key::G => {
                let v = ed.viewports.active_view_mut();
                v.grid = !v.grid;
            }
            Key::F3 => {
                let v = ed.viewports.active_view_mut();
                v.shading = if v.shading == Shading::Wireframe { Shading::Default } else { Shading::Wireframe };
            }
            Key::F4 => {
                let v = ed.viewports.active_view_mut();
                v.edged_faces = !v.edged_faces;
            }
            Key::S => ed.snap = !ed.snap,
            Key::A => ed.angle_snap = !ed.angle_snap,
            Key::H => ed.explorer_visible = !ed.explorer_visible,
            Key::Delete => ed.delete_selection(),
            Key::Backspace if ed.in_sub_object() => match ed.sub {
                SubObject::Vertex => {
                    ed.apply_op(PolyOp::RemoveVertices);
                }
                SubObject::Edge | SubObject::Border => {
                    ed.apply_op(PolyOp::RemoveEdges { remove_verts: ctrl });
                }
                _ => {}
            },
            Key::Escape => {
                if ed.caddy.is_some() {
                    ed.cancel_caddy();
                } else if !ed.cancel_tool() {
                    ed.quad_menu = None;
                }
            }
            Key::M => ed.mat_editor.open = !ed.mat_editor.open,
            Key::Num8 => ed.env_open = !ed.env_open,
            Key::F9 => ed.start_render(),
            Key::F10 => ed.rfw.setup_open = !ed.rfw.setup_open,
            _ => {}
        }
    }
}
