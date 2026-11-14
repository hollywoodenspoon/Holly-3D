//! What each action does, whether it is on (toggle buttons light up in the
//! accent colour) and whether it can run right now (greyed-out buttons).

use super::Action;
use crate::editor::{CommandTab, Editor, Tool, TransformMode};
use crate::mesh::SubObject;
use crate::mesh::ops::PolyOp;
use crate::render::sky::SkyMode;
use crate::render::sky_presets::SkyPreset;
use crate::scene::CloneKind;
use crate::ui::file_menu;
use crate::viewport::layout::Shading;

impl Action {
    pub fn run(self, ed: &mut Editor) {
        use Action::*;
        crate::diagnostics::breadcrumbs::record("Action", self.label().to_string());
        let any = !ed.scene.selected().is_empty();
        match self {
            NewScene => ed.new_scene(),
            OpenScene => file_menu::open(ed),
            SaveScene => file_menu::save(ed),
            SaveSceneAs => file_menu::save_as(ed),
            Import => file_menu::import(ed),
            ImportVdb => file_menu::import_vdb(ed),
            ExportAll => file_menu::export(ed, false),
            ExportSelected => file_menu::export(ed, true),
            Undo => ed.undo(),
            Redo => ed.redo(),
            Clone => ed.clone_selected(CloneKind::Copy),
            Delete => ed.delete_selection(),
            SelectAll => ed.select_all(true),
            SelectNone => ed.select_all(false),
            SelectInvert => ed.invert_selection(),
            SelectByName => flip(&mut ed.iface.select_by_name_open),
            LockSelection => flip(&mut ed.iface.lock_selection),
            WindowCrossing => flip(&mut ed.iface.window_select),
            Cancel => {
                if ed.caddy.is_some() {
                    ed.cancel_caddy();
                } else if ed.iface.nav_mode.is_some() {
                    ed.iface.nav_mode = None;
                } else if !ed.cancel_tool() {
                    ed.quad_menu = None;
                }
            }
            RepeatLast => ed.repeat_last(),
            SelectObject => ed.select_mode_or_cycle_region(),
            Move => ed.mode = TransformMode::Move,
            Rotate => ed.mode = TransformMode::Rotate,
            Scale => ed.mode = TransformMode::Scale,
            Snap => flip(&mut ed.snap),
            AngleSnap => flip(&mut ed.angle_snap),
            PercentSnap => flip(&mut ed.iface.percent_snap),
            SnapSettings => flip(&mut ed.iface.snap_settings_open),
            Sub(s) => ed.set_sub_object(s),
            SelectRing => ed.edit_selection(|m, _| m.select_ring()),
            SelectLoop => ed.edit_selection(|m, _| m.select_loop()),
            RemoveSubObjects => match ed.sub {
                SubObject::Vertex => {
                    ed.apply_op(PolyOp::RemoveVertices);
                }
                SubObject::Edge | SubObject::Border => {
                    ed.apply_op(PolyOp::RemoveEdges { remove_verts: false });
                }
                _ => {}
            },
            Array if any => ed.open_array(),
            Mirror if any => ed.open_mirror(),
            Align => ed.start_align(),
            Isolate => ed.toggle_isolate(),
            JointTool => ed.toggle_joint_tool(),
            ContactSets => ed.toggle_contacts(),
            Paint => ed.toggle_paint(),
            ConvertToPoly => ed.convert_to_editable_poly(),
            ConvertToSpline => ed.convert_to_editable_spline(),
            Measure => flip(&mut ed.iface.measure_open),
            Constrain(c) => ed.constrain_spline(c),
            HideSelected => ed.hide_selected(),
            HideUnselected => ed.hide_unselected(),
            UnhideAll => ed.unhide_all(),
            FreezeSelected => ed.freeze_selected(),
            UnfreezeAll => ed.unfreeze_all(),
            SeeThrough => ed.toggle_see_through(),
            ShadowCatcher => ed.toggle_shadow_catcher(),
            ToggleGrid => flip(&mut ed.viewports.active_view_mut().grid),
            OrientationCube => {
                flip(&mut ed.prefs.viewport.show_cube);
                ed.iface.layout_dirty = true;
            }
            Wireframe => {
                let v = ed.viewports.active_view_mut();
                v.shading = if v.shading == Shading::Wireframe { Shading::Default } else { Shading::Wireframe };
            }
            EdgedFaces => flip(&mut ed.viewports.active_view_mut().edged_faces),
            PreviewVolumes => flip(&mut ed.prefs.viewport.preview_volumes),
            ShowGizmo => flip(&mut ed.prefs.viewport.show_gizmo),
            GizmoSmaller | GizmoBigger => {
                let g = &mut ed.prefs.viewport.gizmo_size;
                *g = (*g * if self == GizmoBigger { 1.15 } else { 1.0 / 1.15 }).clamp(30.0, 300.0);
            }
            ZoomExtents => ed.zoom_extents(false),
            ZoomExtentsAll => ed.zoom_extents(true),
            Maximize => flip(&mut ed.viewports.maximized),
            SceneExplorer => flip(&mut ed.explorer_visible),
            CameraView => ed.camera_hotkey(),
            Ribbon => flip(&mut ed.iface.ribbon),
            Nav(m) => ed.toggle_nav_mode(m),
            Render => ed.start_render(),
            RenderSetup => ed.toggle_tab(CommandTab::Render),
            FrameWindow => flip(&mut ed.rfw.rfw_open),
            Turntable => flip(&mut ed.rfw.sheet.open),
            BuildingsBrowser => flip(&mut ed.buildings_browser_open),
            CharacterStudio => flip(&mut ed.iface.character_studio_open),
            FaceCloner => flip(&mut ed.iface.face_cloner_open),
            LightMix => {
                ed.rfw.lightmix_open = !(ed.rfw.lightmix_open && ed.rfw.rfw_open);
                ed.rfw.rfw_open |= ed.rfw.lightmix_open;
            }
            ColorGrading => {
                ed.rfw.grading_open = !(ed.rfw.grading_open && ed.rfw.rfw_open);
                ed.rfw.rfw_open |= ed.rfw.grading_open;
            }
            MaterialEditor => flip(&mut ed.mat_editor.open),
            MaterialLibrary => flip(&mut ed.mat_editor.library),
            Environment => ed.toggle_tab(CommandTab::Environment),
            DefaultLights => ed.toggle_default_lights(),
            Sky(p) => {
                ed.apply_sky_preset(p);
                if p != SkyPreset::Off {
                    ed.tab = CommandTab::Environment;
                }
            }
            Help => {
                flip(&mut ed.iface.help_open);
                ed.iface.help_context = true;
            }
            Tour => ed.iface.tour = Some(0),
            HotkeyList => flip(&mut ed.iface.hotkeys_open),
            About => flip(&mut ed.iface.about_open),
            CrashReports => flip(&mut ed.iface.crash_open),
            Preferences => flip(&mut ed.iface.prefs_open),
            Array | Mirror => ed.status = "Select an object first".into(),
        }
    }

    /// `Some(on)` for toggles (drawn in the accent when on), `None` for one-shot actions.
    pub fn checked(self, ed: &Editor) -> Option<bool> {
        use Action::*;
        let v = ed.viewports.views.get(ed.viewports.active);
        Some(match self {
            LockSelection => ed.iface.lock_selection,
            WindowCrossing => ed.iface.window_select,
            SelectObject => ed.mode == TransformMode::Select,
            Move => ed.mode == TransformMode::Move,
            Rotate => ed.mode == TransformMode::Rotate,
            Scale => ed.mode == TransformMode::Scale,
            Snap => ed.snap,
            AngleSnap => ed.angle_snap,
            PercentSnap => ed.iface.percent_snap,
            SnapSettings => ed.iface.snap_settings_open,
            Sub(s) => ed.sub == s,
            SelectByName => ed.iface.select_by_name_open,
            Isolate => ed.isolate.is_some(),
            JointTool => matches!(ed.tool, Tool::Joint { .. }),
            ContactSets => ed.scene.assembly.contacts_on,
            Paint => ed.paint.active,
            Measure => ed.iface.measure_open,
            ToggleGrid => v.is_some_and(|v| v.grid),
            OrientationCube => ed.prefs.viewport.show_cube,
            Wireframe => v.is_some_and(|v| v.shading == Shading::Wireframe),
            EdgedFaces => v.is_some_and(|v| v.edged_faces),
            PreviewVolumes => ed.prefs.viewport.preview_volumes,
            ShowGizmo => ed.prefs.viewport.show_gizmo,
            Maximize => ed.viewports.maximized,
            SceneExplorer => ed.explorer_visible,
            Ribbon => ed.iface.ribbon,
            Nav(m) => ed.iface.nav_mode == Some(m),
            RenderSetup => ed.tab == CommandTab::Render,
            FrameWindow => ed.rfw.rfw_open,
            ColorGrading => ed.rfw.rfw_open && ed.rfw.grading_open,
            LightMix => ed.rfw.rfw_open && ed.rfw.lightmix_open,
            Turntable => ed.rfw.sheet.open,
            BuildingsBrowser => ed.buildings_browser_open,
            CharacterStudio => ed.iface.character_studio_open,
            FaceCloner => ed.iface.face_cloner_open,
            ShadowCatcher => { let sel = ed.scene.selected(); !sel.is_empty() && sel.iter().all(|id| ed.scene.objects[*id].shadow_catcher) }
            MaterialEditor => ed.mat_editor.open,
            MaterialLibrary => ed.mat_editor.library,
            Environment => ed.tab == CommandTab::Environment,
            DefaultLights => ed.uses_default_lights(),
            Sky(p) => SkyPreset::matching(&ed.scene.environment.sky) == Some(p),
            Help => ed.iface.help_open,
            Tour => ed.iface.tour.is_some(),
            HotkeyList => ed.iface.hotkeys_open,
            About => ed.iface.about_open,
            CrashReports => ed.iface.crash_open,
            Preferences => ed.iface.prefs_open,
            _ => return None,
        })
    }

    /// Whether the action can run now (buttons grey out otherwise).
    pub fn enabled(self, ed: &Editor) -> bool {
        use Action::*;
        let any = !ed.scene.selected().is_empty();
        let edge_like = ed.in_sub_object() && ed.sub.is_edge_like();
        match self {
            Undo => ed.history.can_undo(),
            Redo => ed.history.can_redo(),
            Clone | Delete | ExportSelected | Array | Mirror | HideSelected | FreezeSelected | SeeThrough | ShadowCatcher | ConvertToPoly
            | ConvertToSpline => any || (self == Delete && ed.in_sub_object()),
            Sub(s) => s == SubObject::Object || ed.current().is_some(),
            SelectRing | SelectLoop => edge_like,
            RemoveSubObjects => matches!(ed.sub, SubObject::Vertex | SubObject::Edge | SubObject::Border) && ed.in_sub_object(),
            RepeatLast => ed.in_sub_object(),
            Constrain(c) => ed.spline_level() == Some(c.level()),
            Sky(_) => ed.scene.environment.sky.mode != SkyMode::Off || self != Sky(SkyPreset::Off),
            _ => true,
        }
    }
}

fn flip(b: &mut bool) {
    *b = !*b;
}
