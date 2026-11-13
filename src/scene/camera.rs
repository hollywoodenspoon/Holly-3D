//! Target / Free Camera objects. A camera looks down its
//! local -Z axis, `fov` is horizontal and the lens is measured on a 36 mm
//! film back. Lights and cameras share the Target helper bookkeeping here.

use super::{BaseObject, ObjId, Scene, SceneObject};
use crate::render::LightKind;
use crate::render::lighting::{aim, emit_dir};
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::{Quat, Vec3};

/// Film / aperture width used for the Lens (mm) <-> FOV conversion.
pub const FILM_MM: f32 = 36.0;
/// Stock lenses offered as one-click buttons.
pub const STOCK_LENSES: [f32; 9] = [15.0, 20.0, 24.0, 28.0, 35.0, 50.0, 85.0, 135.0, 200.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CameraKind {
    Target,
    Free,
}

impl CameraKind {
    pub const ALL: [CameraKind; 2] = [CameraKind::Target, CameraKind::Free];
    pub fn label(self) -> &'static str {
        match self {
            CameraKind::Target => "Target Camera",
            CameraKind::Free => "Free Camera",
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Camera {
    pub kind: CameraKind,
    /// Horizontal field of view in degrees.
    pub fov: f32,
    /// Clip Manually: use `near` / `far` instead of the automatic planes.
    pub clip_manually: bool,
    pub near: f32,
    pub far: f32,
    /// Free camera: distance of its (invisible) target, used for the cone
    /// and as the orbit pivot.
    pub target_distance: f32,
    /// Show Cone: draws the view frustum even when not selected.
    pub show_cone: bool,
    /// Target helper (Target Camera only).
    pub target: Option<ObjId>,
}

impl Camera {
    pub fn new(kind: CameraKind) -> Self {
        Self { kind, fov: 45.0, clip_manually: false, near: 1.0, far: 1000.0, target_distance: 10.0, show_cone: false, target: None }
    }

    pub fn lens_mm(&self) -> f32 {
        FILM_MM * 0.5 / (self.fov.to_radians() * 0.5).tan()
    }

    pub fn set_lens_mm(&mut self, mm: f32) {
        self.fov = (2.0 * (FILM_MM * 0.5 / mm.max(0.1)).atan()).to_degrees();
        self.clamp();
    }

    pub fn clamp(&mut self) {
        self.fov = self.fov.clamp(0.5, 175.0);
        self.near = self.near.max(0.001);
        self.far = self.far.max(self.near + 0.001);
        self.target_distance = self.target_distance.max(0.01);
    }

    /// Active clipping planes (automatic: tiny near, no far).
    pub fn clip(&self) -> (f32, f32) {
        if self.clip_manually { (self.near, self.far) } else { (0.02, f32::INFINITY) }
    }

    /// What the camera at `eye` with rotation `rot` sees, aimed at `aim_at`
    /// (its target) or `target_distance` straight ahead.
    pub fn view(&self, eye: Vec3, rot: Quat, aim_at: Option<Vec3>) -> ViewCamera {
        let fwd = aim_at.map_or(emit_dir(rot), |a| (a - eye).normalize_or(emit_dir(rot)));
        let dist = aim_at.map_or(self.target_distance, |a| a.distance(eye)).max(0.01);
        let mut v = ViewCamera::new(ViewKind::Camera);
        // Heading: from the forward vector, or the camera's up when looking straight down / up.
        let up = rot * Vec3::Y;
        let h = if fwd.truncate().length() > 1e-4 { fwd.truncate() } else { up.truncate() * -fwd.z.signum() };
        v.yaw = h.y.atan2(h.x);
        v.pitch = fwd.z.clamp(-1.0, 1.0).asin();
        v.target = eye + fwd * dist;
        v.distance = dist;
        v.fov = self.fov.to_radians();
        v.hfov = true;
        (v.near, v.far) = self.clip();
        v
    }
}

impl SceneObject {
    pub fn camera(&self) -> Option<&Camera> {
        match &self.base {
            BaseObject::Camera(c) => Some(c),
            _ => None,
        }
    }

    pub fn camera_mut(&mut self) -> Option<&mut Camera> {
        match &mut self.base {
            BaseObject::Camera(c) => Some(c),
            _ => None,
        }
    }

    /// Target helper of a target light or Target Camera.
    pub fn target_id(&self) -> Option<ObjId> {
        match &self.base {
            BaseObject::Light(l) => l.target,
            BaseObject::Camera(c) => c.target,
            _ => None,
        }
    }

    pub fn set_target_id(&mut self, t: Option<ObjId>) {
        match &mut self.base {
            BaseObject::Light(l) => l.target = t,
            BaseObject::Camera(c) => c.target = t,
            _ => {}
        }
    }

    /// Its target was deleted: becomes the Free variant.
    pub fn lose_target(&mut self) {
        self.set_target_id(None);
        match &mut self.base {
            BaseObject::Light(l) => l.kind = if l.kind == LightKind::TargetSpot { LightKind::FreeSpot } else { LightKind::FreeDirect },
            BaseObject::Camera(c) => c.kind = CameraKind::Free,
            _ => {}
        }
    }
}

impl Scene {
    /// Adds `owner` aimed at `aim_at` plus, when `with_target`, its Target helper.
    pub(super) fn insert_aimed(&mut self, mut o: SceneObject, at: Vec3, aim_at: Vec3, with_target: bool) -> ObjId {
        o.set_target_id(None);
        o.transform.translation = at;
        o.transform.rotation = aim(at, aim_at);
        let name = o.name.clone();
        let wire = o.wire_color;
        let id = self.objects.insert(o);
        if with_target {
            let mut t = SceneObject::new(format!("{name}.Target"), BaseObject::Target);
            t.transform.translation = aim_at;
            t.wire_color = wire;
            let tid = self.objects.insert(t);
            self.objects[id].set_target_id(Some(tid));
        }
        id
    }

    /// Adds a camera at `at` looking at `aim_at` (Target Cameras get a target there).
    pub fn add_camera(&mut self, mut cam: Camera, at: Vec3, aim_at: Vec3) -> ObjId {
        let with_target = cam.kind == CameraKind::Target;
        if !with_target {
            cam.target_distance = at.distance(aim_at).max(0.01);
        }
        let mut o = SceneObject::new(self.unique_name("Camera"), BaseObject::Camera(cam));
        o.wire_color = [0.35, 0.75, 0.95];
        self.insert_aimed(o, at, aim_at, with_target)
    }

    /// The view seen through camera object `id`.
    pub fn camera_view(&self, id: ObjId) -> Option<ViewCamera> {
        let o = self.objects.get(id)?;
        let aim_at = o.target_id().and_then(|t| self.objects.get(t)).map(|t| t.transform.translation);
        Some(o.camera()?.view(o.transform.translation, o.transform.rotation, aim_at))
    }

    /// Camera objects in creation order.
    pub fn cameras(&self) -> Vec<ObjId> {
        self.objects.iter().filter(|(_, o)| o.camera().is_some()).map(|(k, _)| k).collect()
    }

    /// Points every target light / Target Camera at its target. True if any moved.
    pub fn aim_targets(&mut self) -> bool {
        let aims: Vec<(ObjId, Quat)> = self
            .objects
            .iter()
            .filter_map(|(k, o)| {
                let t = self.objects.get(o.target_id()?)?;
                let q = aim(o.transform.translation, t.transform.translation);
                (q.dot(o.transform.rotation).abs() < 1.0 - 1e-7).then_some((k, q))
            })
            .collect();
        for (k, q) in &aims {
            self.objects[*k].transform.rotation = *q;
        }
        !aims.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lens_and_fov_follow_36mm_film() {
        let mut c = Camera::new(CameraKind::Free);
        assert!((c.lens_mm() - 43.456).abs() < 0.01, "45° = 43.456 mm");
        c.set_lens_mm(50.0);
        assert!((c.fov - 39.598).abs() < 0.01);
        assert_eq!(c.clip().1, f32::INFINITY);
        c.clip_manually = true;
        assert_eq!(c.clip(), (1.0, 1000.0));
    }

    #[test]
    fn target_camera_looks_at_target_and_follows_it() {
        let mut s = Scene::default();
        let id = s.add_camera(Camera::new(CameraKind::Target), Vec3::new(0.0, -10.0, 5.0), Vec3::ZERO);
        let t = s.objects[id].target_id().unwrap();
        assert!(s.objects[t].is_helper() && s.objects[id].is_helper());
        let v = s.camera_view(id).unwrap();
        assert!((v.eye() - Vec3::new(0.0, -10.0, 5.0)).length() < 1e-3);
        assert!((v.target - Vec3::ZERO).length() < 1e-3);
        assert!(v.project(Vec3::ZERO, 1.5).unwrap().length() < 1e-4);
        s.objects[t].transform.translation = Vec3::new(3.0, 0.0, 0.0);
        assert!(s.aim_targets());
        assert!(s.camera_view(id).unwrap().project(Vec3::new(3.0, 0.0, 0.0), 1.5).unwrap().length() < 1e-3);
        // deleting the target turns it into a Free Camera
        s.delete(t);
        assert_eq!(s.objects[id].camera().unwrap().kind, CameraKind::Free);
        assert_eq!(s.objects.len(), 1);
    }

    #[test]
    fn free_camera_looking_straight_down_keeps_heading() {
        let mut s = Scene::default();
        let id = s.add_camera(Camera::new(CameraKind::Free), Vec3::new(0.0, 0.0, 8.0), Vec3::ZERO);
        let v = s.camera_view(id).unwrap();
        assert!((v.forward() - Vec3::NEG_Z).length() < 1e-4);
        assert!((v.target - Vec3::ZERO).length() < 1e-3, "target distance from creation");
        assert!(v.project(Vec3::ZERO, 1.0).unwrap().length() < 1e-4);
        assert!(s.objects[id].target_id().is_none());
    }
}
