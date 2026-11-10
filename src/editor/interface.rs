//! Interface state behind the button-rich 3ds Max toolbar: Lock Selection,
//! Window / Crossing, the Selection Filter, Percent Snap, the viewport
//! navigation buttons, the Display tab's hide / freeze actions and the
//! small tool windows (Select From Scene, Measure, Hotkeys, About).

use super::*;

/// Object categories for the Selection Filter and Display -> Hide by Category.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ObjCategory {
    #[default]
    All,
    Geometry,
    Shapes,
    Lights,
    Cameras,
    Helpers,
}

impl ObjCategory {
    pub const ALL: [ObjCategory; 6] = [Self::All, Self::Geometry, Self::Shapes, Self::Lights, Self::Cameras, Self::Helpers];
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Geometry => "Geometry",
            Self::Shapes => "Shapes",
            Self::Lights => "Lights",
            Self::Cameras => "Cameras",
            Self::Helpers => "Helpers",
        }
    }

    /// The category an object belongs to (never `All`).
    pub fn of(o: &SceneObject) -> Self {
        if o.light().is_some() {
            Self::Lights
        } else if o.camera().is_some() {
            Self::Cameras
        } else if o.is_helper() {
            Self::Helpers
        } else if o.shape.is_some() {
            Self::Shapes
        } else {
            Self::Geometry
        }
    }

    pub fn accepts(self, o: &SceneObject) -> bool {
        self == Self::All || Self::of(o) == self
    }
}

/// Left-drag navigation modes of the bottom-right viewport controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavMode {
    Zoom,
    ZoomAll,
    Fov,
    Pan,
    Orbit,
}

impl NavMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Zoom => "Zoom: drag up / down in a viewport",
            Self::ZoomAll => "Zoom All: drag up / down to zoom every viewport",
            Self::Fov => "Field of View: drag up / down",
            Self::Pan => "Pan: drag in a viewport",
            Self::Orbit => "Orbit: drag in a viewport",
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct InterfaceState {
    /// Lock Selection (Space): clicks and marquees no longer change the selection.
    pub lock_selection: bool,
    /// Window (fully inside) instead of Crossing (touching) region selection.
    pub window_select: bool,
    pub select_filter: ObjCategory,
    /// Percent Snap: the Scale gizmo steps in 10 % increments.
    pub percent_snap: bool,
    pub nav_mode: Option<NavMode>,
    /// Modeling Ribbon below the main toolbar.
    pub ribbon: bool,
    pub snap_settings_open: bool,
    pub select_by_name_open: bool,
    pub select_by_name_filter: String,
    pub measure_open: bool,
    pub hotkeys_open: bool,
    pub hotkeys_filter: String,
    pub about_open: bool,
    /// Show Buttons: the modifier-set button grid under the stack.
    pub modifier_buttons: bool,
    /// Modifier names on the button grid, in order.
    pub modifier_set: Vec<String>,
}

/// The default Show Buttons set (the most used modifiers).
pub const DEFAULT_MODIFIER_SET: [&str; 8] = ["Edit Poly", "TurboSmooth", "Symmetry", "Shell", "FFD 3x3x3", "Bend", "UVW Map", "Chamfer"];

impl InterfaceState {
    pub fn new() -> Self {
        Self { modifier_set: DEFAULT_MODIFIER_SET.iter().map(|s| s.to_string()).collect(), ..Default::default() }
    }
}

impl Editor {
    /// Toggles a navigation button (pressing the active one turns it off).
    pub fn toggle_nav_mode(&mut self, m: NavMode) {
        self.iface.nav_mode = if self.iface.nav_mode == Some(m) { None } else { Some(m) };
        if self.iface.nav_mode.is_some() {
            self.tool = Tool::None;
        }
    }

    /// Left-drag in a viewport while a navigation button is on.
    pub fn nav_drag(&mut self, vi: usize, mode: NavMode, d: Vec2, height_px: f32) {
        let pivot = self.selection_center().unwrap_or(self.viewports.views[vi].camera.target);
        let views: Vec<usize> = if mode == NavMode::ZoomAll { (0..self.viewports.views.len()).collect() } else { vec![vi] };
        for v in views {
            let cam = &mut self.viewports.views[v].camera;
            match mode {
                NavMode::Zoom | NavMode::ZoomAll => cam.zoom(-d.y * 0.02),
                NavMode::Fov => cam.fov = (cam.fov + d.y * 0.004).clamp(5f32.to_radians(), 170f32.to_radians()),
                NavMode::Pan => cam.pan(d, height_px),
                NavMode::Orbit => cam.orbit(d, pivot),
            }
            self.push_view_to_camera(v);
        }
    }

    fn set_flag(&mut self, label: &str, ids: Vec<ObjId>, f: impl Fn(&mut SceneObject)) {
        if ids.is_empty() {
            return;
        }
        self.checkpoint(label);
        for id in ids {
            f(&mut self.scene.objects[id]);
        }
        self.status = label.to_string();
    }

    pub fn hide_selected(&mut self) {
        let ids = self.scene.selected();
        self.set_flag("Hide Selected", ids, |o| (o.hidden, o.selected) = (true, false));
    }

    pub fn hide_unselected(&mut self) {
        let ids = self.scene.objects.iter().filter(|(_, o)| !o.selected).map(|x| x.0).collect();
        self.set_flag("Hide Unselected", ids, |o| o.hidden = true);
    }

    pub fn unhide_all(&mut self) {
        let ids = self.scene.objects.iter().filter(|(_, o)| o.hidden).map(|x| x.0).collect();
        self.set_flag("Unhide All", ids, |o| o.hidden = false);
    }

    pub fn freeze_selected(&mut self) {
        let ids = self.scene.selected();
        self.set_flag("Freeze Selected", ids, |o| (o.frozen, o.selected) = (true, false));
    }

    pub fn unfreeze_all(&mut self) {
        let ids = self.scene.objects.iter().filter(|(_, o)| o.frozen).map(|x| x.0).collect();
        self.set_flag("Unfreeze All", ids, |o| o.frozen = false);
    }

    /// Display -> Hide by Category: hides (or unhides) every object of `cat`.
    pub fn hide_category(&mut self, cat: ObjCategory, hide: bool) {
        let ids = self.scene.objects.iter().filter(|(_, o)| cat.accepts(o) && o.hidden != hide).map(|x| x.0).collect();
        let label = format!("{} {}", if hide { "Hide" } else { "Unhide" }, cat.label());
        self.set_flag(&label, ids, |o| {
            o.hidden = hide;
            o.selected &= !hide;
        });
    }

    pub fn toggle_see_through(&mut self) {
        let ids = self.scene.selected();
        self.set_flag("See-Through", ids, |o| o.see_through = !o.see_through);
    }

    /// Select From Scene: replaces (or extends) the selection with `ids`.
    pub fn select_ids(&mut self, ids: &[ObjId], add: bool) {
        if !add {
            self.scene.select_only(None);
        }
        for &id in ids {
            if let Some(o) = self.scene.objects.get_mut(id) {
                o.selected = !o.hidden && !o.frozen;
            }
        }
        self.sub = SubObject::Object;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::PrimitiveKind;
    use crate::render::LightKind;

    fn scene() -> Editor {
        let mut ed = Editor::default();
        ed.create_primitive(PrimitiveKind::Box, Vec3::ZERO);
        ed.create_primitive(PrimitiveKind::Sphere, Vec3::new(3.0, 0.0, 0.0));
        ed.create_light_default(LightKind::Omni);
        ed
    }

    #[test]
    fn categories_and_hide_by_category() {
        let mut ed = scene();
        let cats: Vec<_> = ed.scene.objects.values().map(ObjCategory::of).collect();
        assert_eq!(cats.iter().filter(|c| **c == ObjCategory::Geometry).count(), 2);
        assert!(cats.contains(&ObjCategory::Lights));
        ed.hide_category(ObjCategory::Lights, true);
        assert!(ed.scene.objects.values().all(|o| o.hidden == (ObjCategory::of(o) == ObjCategory::Lights)));
        ed.undo();
        assert!(ed.scene.objects.values().all(|o| !o.hidden), "one undo step");
        ed.hide_category(ObjCategory::All, true);
        ed.unhide_all();
        assert!(ed.scene.objects.values().all(|o| !o.hidden));
    }

    #[test]
    fn hide_freeze_and_select_by_name() {
        let mut ed = scene();
        let ids: Vec<_> = ed.scene.objects.keys().collect();
        ed.select_ids(&ids[..1], false);
        assert_eq!(ed.scene.selected(), vec![ids[0]]);
        ed.hide_unselected();
        assert_eq!(ed.scene.objects.values().filter(|o| o.hidden).count(), ids.len() - 1);
        ed.unhide_all();
        ed.freeze_selected();
        assert!(ed.scene.objects[ids[0]].frozen && ed.scene.selected().is_empty());
        ed.select_ids(&ids[..1], false);
        assert!(ed.scene.selected().is_empty(), "frozen objects cannot be selected");
        ed.unfreeze_all();
        ed.select_ids(&ids, true);
        assert_eq!(ed.scene.selected().len(), ids.len());
    }

    #[test]
    fn nav_buttons_toggle_and_cancel_tools() {
        let mut ed = scene();
        ed.tool = Tool::Create(PrimitiveKind::Box);
        ed.toggle_nav_mode(NavMode::Orbit);
        assert_eq!((ed.iface.nav_mode, ed.tool.clone()), (Some(NavMode::Orbit), Tool::None));
        ed.toggle_nav_mode(NavMode::Orbit);
        assert_eq!(ed.iface.nav_mode, None);
        let before: Vec<_> = ed.viewports.views.iter().map(|v| v.camera).collect();
        ed.nav_drag(0, NavMode::Zoom, Vec2::new(0.0, -50.0), 400.0);
        assert_ne!(ed.viewports.views[0].camera, before[0], "zoom changes the dragged view");
        assert_eq!(ed.viewports.views[1].camera, before[1], "zoom leaves other views alone");
        ed.nav_drag(0, NavMode::ZoomAll, Vec2::new(0.0, -50.0), 400.0);
        assert_ne!(ed.viewports.views[1].camera, before[1], "Zoom All changes every view");
        let fov = ed.viewports.views[3].camera.fov;
        ed.nav_drag(3, NavMode::Fov, Vec2::new(0.0, 100.0), 400.0);
        assert!(ed.viewports.views[3].camera.fov > fov);
    }
}
