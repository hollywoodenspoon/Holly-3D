//! Lights: Create -> Lights tool, Default Lights (Ctrl+L), light type
//! changes and the Environment lighting presets (`8`).

use super::*;
use crate::render::environment::apply_preset;
use crate::render::{Light, LightKind, LightingPreset};
use crate::viewport::camera::ViewKind;

/// Height free lights are lifted to when placed on the ground grid.
const LIFT: f32 = 3.0;

impl Editor {
    /// Default Modeling Lights are used when Ctrl+L forces them, or when no
    /// scene light is on (unless the environment is Pitch Black).
    pub fn uses_default_lights(&self) -> bool {
        self.default_lights || (self.scene.environment.auto_default_lights && self.scene.active_lights().next().is_none())
    }

    pub fn toggle_default_lights(&mut self) {
        self.default_lights = !self.default_lights;
        self.status = if self.default_lights {
            "Default Lights: on (Ctrl+L)".into()
        } else if self.uses_default_lights() {
            "Default Lights: automatic — no scene lights yet".into()
        } else {
            "Scene Lights".into()
        };
    }

    /// Creates a light at `at` aimed at `aim` and selects it.
    pub fn create_light(&mut self, kind: LightKind, at: Vec3, aim: Vec3) -> ObjId {
        self.checkpoint("Create Light");
        let id = self.scene.add_light(Light::new(kind), None, at, aim);
        self.scene.select_only(Some(id));
        self.sub = SubObject::Object;
        self.tab = CommandTab::Modify;
        self.status = format!("Created {}", self.scene.objects[id].name);
        id
    }

    /// Right-click on a light type: a light above and in front of the origin.
    pub fn create_light_default(&mut self, kind: LightKind) {
        let at = if kind.is_directional() || kind == LightKind::Skylight { Vec3::new(-4.0, -5.0, 7.0) } else { Vec3::new(-3.0, -4.0, 4.0) };
        self.create_light(kind, at, Vec3::ZERO);
    }

    pub(super) fn create_light_click(&mut self, view: usize, cursor: Vec2, kind: LightKind, first: Option<Vec3>) {
        let v = &self.viewports.views[view];
        let mut p = v.construction_point(cursor);
        if self.snap {
            p = (p * 4.0).round() / 4.0;
        }
        // Ground-plane views place lights above the grid, like a ceiling rig.
        let ground = !matches!(v.camera.kind, ViewKind::Front | ViewKind::Left);
        match (kind.has_target(), first) {
            (true, None) => {
                let at = if ground { p + Vec3::Z * LIFT } else { p };
                self.tool = Tool::CreateLight { kind, first: Some(at) };
                return;
            }
            (true, Some(at)) => {
                self.create_light(kind, at, p);
            }
            (false, _) => {
                let at = if ground { p + Vec3::Z * LIFT } else { p };
                // Free spots / directs point straight down (local -Z).
                let id = self.create_light(kind, at, at - Vec3::Z);
                if kind == LightKind::Omni {
                    self.scene.objects[id].transform.rotation = bevy::math::Quat::IDENTITY;
                }
            }
        }
        self.tool = Tool::CreateLight { kind, first: None };
    }

    /// Edits the current light with an undo step when `label` is given.
    pub fn edit_light(&mut self, label: Option<&str>, f: impl FnOnce(&mut Light)) {
        let Some(id) = self.current() else { return };
        if self.scene.objects[id].light().is_none() {
            return;
        }
        if let Some(l) = label {
            self.checkpoint(l);
        }
        if let Some(l) = self.scene.objects[id].light_mut() {
            f(l);
            l.clamp_cone();
        }
    }

    /// Type dropdown: switching to a target type adds a target in front of
    /// the light; switching away removes it.
    pub fn set_light_kind(&mut self, kind: LightKind) {
        let Some(id) = self.current() else { return };
        let Some(old) = self.scene.objects[id].light().cloned() else { return };
        if old.kind == kind {
            return;
        }
        self.checkpoint("Light Type");
        let o = &self.scene.objects[id];
        let (at, rot, name, sel) = (o.transform.translation, o.transform.rotation, o.name.clone(), o.selected);
        let aim = old
            .target
            .and_then(|t| self.scene.objects.get(t))
            .map_or(at + crate::render::lighting::emit_dir(rot) * 5.0, |t| t.transform.translation);
        if let Some(t) = old.target {
            self.scene.objects.remove(t);
        }
        let mut l = old.clone();
        l.kind = kind;
        // Re-create through add_light so the target bookkeeping is shared.
        self.scene.objects.remove(id);
        let nid = self.scene.add_light(l, Some(&name), at, aim);
        let o = &mut self.scene.objects[nid];
        o.name = name.clone();
        o.selected = sel;
        if !kind.has_target() {
            o.transform.rotation = rot;
        }
        if let Some(t) = o.light().and_then(|l| l.target) {
            self.scene.objects[t].name = format!("{name}.Target");
        }
    }

    /// 1-click lighting preset: replaces the previous preset's lights.
    pub fn apply_lighting_preset(&mut self, p: LightingPreset) {
        self.checkpoint("Lighting Preset");
        let old: Vec<ObjId> = self.scene.objects.iter().filter(|(_, o)| o.light().is_some_and(|l| l.from_preset)).map(|(k, _)| k).collect();
        for k in old {
            self.scene.delete(k);
        }
        let geo: Vec<Vec3> = self
            .scene
            .objects
            .values()
            .filter(|o| !o.is_helper() && !o.hidden)
            .flat_map(|o| {
                let w = o.world_matrix();
                o.evaluated_mesh.bounds().map(|b| [w.transform_point3(b.min), w.transform_point3(b.max)])
            })
            .flatten()
            .collect();
        let (c, r) = crate::mesh::Aabb::from_points(geo).map_or((Vec3::ZERO, 2.0), |b| (b.center(), b.size().length() * 0.5));
        for pl in apply_preset(&mut self.scene.environment, p, c, r) {
            self.scene.add_light(pl.light, Some(pl.name), pl.position, pl.aim);
        }
        self.default_lights = false;
        self.status = format!("Lighting preset: {}", p.label());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_lights_bring_a_target_and_scene_lights_replace_defaults() {
        let mut ed = Editor::default();
        assert!(ed.uses_default_lights(), "empty scene uses the headlight rig");
        let id = ed.create_light(LightKind::TargetSpot, Vec3::new(0.0, -4.0, 4.0), Vec3::ZERO);
        let t = ed.scene.objects[id].light().unwrap().target.unwrap();
        assert!(ed.scene.objects[t].name.ends_with(".Target"));
        assert!(!ed.uses_default_lights(), "a scene light switches to Scene Lights");
        ed.toggle_default_lights();
        assert!(ed.uses_default_lights(), "Ctrl+L forces the rig");
        ed.toggle_default_lights();
        // Moving the target re-aims the light.
        ed.scene.objects[t].transform.translation = Vec3::new(3.0, 0.0, 0.0);
        assert!(ed.scene.aim_targets());
        let o = &ed.scene.objects[id];
        let d = crate::render::lighting::emit_dir(o.transform.rotation);
        assert!(d.distance((Vec3::new(3.0, 0.0, 0.0) - o.transform.translation).normalize()) < 1e-4);
        assert!(!ed.scene.aim_targets(), "already aimed");
        // Type change: Free Spot drops the target, keeping the direction.
        ed.set_light_kind(LightKind::FreeSpot);
        let id = ed.current().unwrap();
        assert!(ed.scene.objects[id].light().unwrap().target.is_none());
        assert!(!ed.scene.objects.contains_key(t));
        assert_eq!(ed.scene.objects.len(), 1);
        ed.set_light_kind(LightKind::TargetDirect);
        assert_eq!(ed.scene.objects.len(), 2);
        // Deleting the light deletes its target; undo restores both.
        ed.delete_selection();
        assert!(ed.scene.objects.is_empty());
        ed.undo();
        assert_eq!(ed.scene.objects.len(), 2);
    }

    #[test]
    fn deleting_a_target_frees_the_light_and_clones_copy_targets() {
        let mut ed = Editor::default();
        let id = ed.create_light(LightKind::TargetDirect, Vec3::new(0.0, 0.0, 5.0), Vec3::ZERO);
        let t = ed.scene.objects[id].light().unwrap().target.unwrap();
        let mut xf = ed.scene.objects[id].transform;
        xf.translation.x += 2.0;
        let c = ed.scene.clone_object(id, crate::scene::CloneKind::Instance, xf).unwrap();
        let ct = ed.scene.objects[c].light().unwrap().target.unwrap();
        assert_ne!(ct, t, "clone owns its own target");
        assert!((ed.scene.objects[ct].transform.translation.x - 2.0).abs() < 1e-5);
        assert!(ed.scene.objects[c].instance_group.is_none(), "lights clone as copies");
        ed.scene.delete(t);
        let l = ed.scene.objects[id].light().unwrap();
        assert_eq!((l.kind, l.target), (LightKind::FreeDirect, None));
    }

    #[test]
    fn presets_replace_their_own_lights_only() {
        let mut ed = Editor::default();
        ed.create_primitive(crate::primitives::PrimitiveKind::Box, Vec3::ZERO);
        let mine = ed.create_light(LightKind::Omni, Vec3::new(0.0, 0.0, 3.0), Vec3::ZERO);
        ed.apply_lighting_preset(LightingPreset::ThreePoint);
        let count = |ed: &Editor| ed.scene.objects.values().filter(|o| o.light().is_some()).count();
        assert_eq!(count(&ed), 4);
        ed.apply_lighting_preset(LightingPreset::Daylight);
        assert_eq!(count(&ed), 3, "3-point lights replaced by sun + sky");
        assert!(ed.scene.objects.contains_key(mine), "user lights survive presets");
        ed.apply_lighting_preset(LightingPreset::PitchBlack);
        ed.scene.objects[mine].light_mut().unwrap().on = false;
        assert!(!ed.uses_default_lights(), "pitch black never falls back to the rig");
    }
}
