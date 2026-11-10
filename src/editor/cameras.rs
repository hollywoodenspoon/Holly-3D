//! Cameras: Create -> Cameras tool, camera parameters, looking through a
//! camera in a viewport (`C`) and camera-view navigation, which moves the
//! camera itself like 3ds Max (dolly / orbit / truck).

use super::*;
use crate::render::lighting::aim;
use crate::scene::camera::{Camera, CameraKind};
use crate::viewport::camera::ViewKind;

/// Height cameras are lifted to when placed on the ground grid.
const EYE_HEIGHT: f32 = 1.6;

impl Editor {
    /// Creates a camera at `at` looking at `aim_at` and selects it.
    pub fn create_camera(&mut self, kind: CameraKind, at: Vec3, aim_at: Vec3) -> ObjId {
        self.checkpoint("Create Camera");
        let id = self.scene.add_camera(Camera::new(kind), at, aim_at);
        self.scene.select_only(Some(id));
        self.sub = SubObject::Object;
        self.tab = CommandTab::Modify;
        self.status = format!("Created {} — press C to look through it", self.scene.objects[id].name);
        id
    }

    pub(super) fn create_camera_click(&mut self, view: usize, cursor: Vec2, kind: CameraKind, first: Option<Vec3>) {
        let v = &self.viewports.views[view];
        let fwd = v.camera.forward();
        let p = self.snapped_point_pub(view, cursor);
        let ground = !matches!(v.camera.kind, ViewKind::Front | ViewKind::Left);
        let at = if ground { p + Vec3::Z * EYE_HEIGHT } else { p };
        match (kind, first) {
            (CameraKind::Target, None) => {
                self.tool = Tool::CreateCamera { kind, first: Some(at) };
                return;
            }
            (CameraKind::Target, Some(eye)) => {
                self.create_camera(kind, eye, p);
            }
            // Free cameras look the way the viewport looks.
            (CameraKind::Free, _) => {
                self.create_camera(kind, at, at + fwd * 10.0);
            }
        }
        self.tool = Tool::CreateCamera { kind, first: None };
    }

    /// Edits the current camera with an undo step when `label` is given.
    pub fn edit_camera(&mut self, label: Option<&str>, f: impl FnOnce(&mut Camera)) {
        let Some(id) = self.current().filter(|&id| self.scene.objects[id].camera().is_some()) else { return };
        if let Some(l) = label {
            self.checkpoint(l);
        }
        if let Some(c) = self.scene.objects[id].camera_mut() {
            f(c);
            c.clamp();
        }
    }

    /// Type dropdown: Target <-> Free (adds or removes the target helper).
    pub fn set_camera_kind(&mut self, kind: CameraKind) {
        let Some(id) = self.current() else { return };
        let Some(view) = self.scene.camera_view(id) else { return };
        if self.scene.objects[id].camera().is_some_and(|c| c.kind == kind) {
            return;
        }
        self.checkpoint("Camera Type");
        let o = &mut self.scene.objects[id];
        let old_target = o.target_id();
        let (eye, name, wire) = (o.transform.translation, o.name.clone(), o.wire_color);
        if let Some(c) = o.camera_mut() {
            c.kind = kind;
            c.target_distance = view.distance;
            c.target = None;
        }
        if let Some(t) = old_target {
            self.scene.objects.remove(t);
        }
        if kind == CameraKind::Target {
            let mut t = SceneObject::new(format!("{name}.Target"), crate::scene::BaseObject::Target);
            t.transform.translation = view.target;
            t.wire_color = wire;
            let tid = self.scene.objects.insert(t);
            let o = &mut self.scene.objects[id];
            o.set_target_id(Some(tid));
            o.transform.rotation = aim(eye, view.target);
        }
    }

    /// Makes viewport `vi` look through camera `id` (`None`: back to Perspective).
    pub fn look_through(&mut self, vi: usize, id: Option<ObjId>) {
        let v = &mut self.viewports.views[vi];
        match id.and_then(|id| self.scene.camera_view(id).map(|c| (id, c))) {
            Some((id, c)) => {
                v.camera = c;
                v.cam_obj = Some(id);
                v.shading = crate::viewport::layout::Shading::Default;
            }
            None => {
                let keep = (v.camera.target, v.camera.distance, v.camera.yaw, v.camera.pitch);
                v.camera = crate::viewport::camera::ViewCamera::new(ViewKind::Perspective);
                (v.camera.target, v.camera.distance, v.camera.yaw, v.camera.pitch) = keep;
                v.cam_obj = None;
            }
        }
    }

    /// `C`: the active viewport looks through the selected camera (or the
    /// camera of a selected target); otherwise cycles through the cameras.
    pub fn camera_hotkey(&mut self) {
        let cams = self.scene.cameras();
        if cams.is_empty() {
            self.status = "No cameras in the scene — create one from Create -> Cameras".into();
            return;
        }
        let picked = cams.iter().copied().find(|&c| {
            let o = &self.scene.objects[c];
            o.selected || o.target_id().is_some_and(|t| self.scene.objects[t].selected)
        });
        let vi = self.viewports.active;
        let id = picked.unwrap_or_else(|| {
            let cur = self.viewports.views[vi].cam_obj.and_then(|c| cams.iter().position(|&k| k == c));
            cams[cur.map_or(0, |i| (i + 1) % cams.len())]
        });
        self.look_through(vi, Some(id));
        self.status = format!("Viewing through {}", self.scene.objects[id].name);
    }

    /// Camera viewports follow their camera objects (unbinds deleted ones).
    pub fn sync_camera_views(&mut self) {
        for vi in 0..self.viewports.views.len() {
            let Some(id) = self.viewports.views[vi].cam_obj else { continue };
            match self.scene.camera_view(id) {
                Some(c) => self.viewports.views[vi].camera = c,
                None => self.look_through(vi, None),
            }
        }
    }

    /// After navigating a camera viewport: moves the camera (and its target).
    pub fn push_view_to_camera(&mut self, vi: usize) {
        let v = self.viewports.views[vi].camera;
        let Some(id) = self.viewports.views[vi].cam_obj else { return };
        let Some(o) = self.scene.objects.get_mut(id) else { return };
        let eye = v.eye();
        o.transform.translation = eye;
        o.transform.rotation = aim(eye, v.target);
        let target = o.target_id();
        if let Some(c) = o.camera_mut() {
            c.target_distance = v.distance;
        }
        if let Some(t) = target.and_then(|t| self.scene.objects.get_mut(t)) {
            t.transform.translation = v.target;
        }
    }

    /// Viewport / camera label used by the render pickers and the RFW.
    pub fn view_label(&self, vi: usize) -> String {
        let v = &self.viewports.views[vi];
        match v.cam_obj.and_then(|c| self.scene.objects.get(c)) {
            Some(o) => o.name.clone(),
            None => v.camera.kind.label().to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_look_through_navigate_and_delete() {
        let mut ed = Editor::default();
        assert!(ed.scene.cameras().is_empty());
        ed.camera_hotkey();
        assert!(ed.status.starts_with("No cameras"));
        let id = ed.create_camera(CameraKind::Target, Vec3::new(0.0, -8.0, 3.0), Vec3::ZERO);
        let tid = ed.scene.objects[id].target_id().unwrap();
        ed.camera_hotkey();
        let vi = ed.viewports.active;
        assert_eq!(ed.viewports.views[vi].cam_obj, Some(id));
        assert_eq!(ed.view_label(vi), "Camera001");
        // orbit the camera view: the camera moves, the target stays the pivot
        let pivot = ed.viewports.views[vi].camera.target;
        ed.viewports.views[vi].camera.orbit(Vec2::new(60.0, 0.0), pivot);
        ed.push_view_to_camera(vi);
        let o = &ed.scene.objects[id];
        assert!((o.transform.translation - Vec3::new(0.0, -8.0, 3.0)).length() > 0.5);
        assert!((ed.scene.objects[tid].transform.translation).length() < 1e-3);
        assert!((o.transform.translation.length() - Vec3::new(0.0, -8.0, 3.0).length()).abs() < 1e-3);
        // Target -> Free -> Target keeps the view
        let before = ed.scene.camera_view(id).unwrap();
        ed.set_camera_kind(CameraKind::Free);
        assert!(ed.scene.objects[id].target_id().is_none() && !ed.scene.objects.contains_key(tid));
        ed.set_camera_kind(CameraKind::Target);
        let after = ed.scene.camera_view(id).unwrap();
        assert!((before.eye() - after.eye()).length() < 1e-3 && (before.target - after.target).length() < 1e-3);
        // deleting the camera returns the view to Perspective
        ed.scene.delete(id);
        ed.sync_camera_views();
        assert_eq!(ed.viewports.views[vi].camera.kind, ViewKind::Perspective);
        assert_eq!(ed.viewports.views[vi].cam_obj, None);
    }

    #[test]
    fn free_camera_click_looks_along_view_and_lens_edit_undoes() {
        let mut ed = Editor::default();
        ed.viewports.layout(crate::viewport::layout::ScreenRect { min: Vec2::ZERO, max: Vec2::new(800.0, 600.0) }, 0.0);
        ed.create_camera_click(0, Vec2::new(200.0, 150.0), CameraKind::Free, None);
        let id = ed.current().unwrap();
        let v = ed.scene.camera_view(id).unwrap();
        assert!((v.forward() - Vec3::NEG_Z).length() < 1e-3, "Top view: looks down");
        ed.edit_camera(Some("Lens"), |c| c.set_lens_mm(85.0));
        assert!((ed.scene.objects[id].camera().unwrap().lens_mm() - 85.0).abs() < 1e-3);
        ed.undo();
        assert!((ed.scene.objects[id].camera().unwrap().fov - 45.0).abs() < 1e-4);
    }
}
