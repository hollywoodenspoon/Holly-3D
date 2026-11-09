//! Scene graph: objects with base object + modifier stack + transform.

mod clone;
mod object;
#[cfg(test)]
mod tests;

pub use clone::CloneKind;

use crate::mesh::EditablePoly;
use crate::modifiers::ModifierEntry;
use crate::primitives::Primitive;
use bevy::math::{Mat4, Quat, Vec3};
use slotmap::{SlotMap, new_key_type};
use std::collections::HashMap;

new_key_type! {
    pub struct ObjId;
}

static VERSION: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

/// Globally unique, monotonic version stamp (snapshots restored by undo or
/// drag previews still get fresh versions, so the renderer always re-uploads).
pub fn next_version() -> u64 {
    VERSION.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

#[derive(Clone, Debug)]
pub enum BaseObject {
    Primitive(Primitive),
    EditablePoly(EditablePoly),
    /// Light object (no geometry; drawn as a viewport icon).
    Light(crate::render::Light),
    /// Target helper of a Target Spot / Target Direct light.
    LightTarget,
}

impl BaseObject {
    pub fn to_editable_poly(&self) -> EditablePoly {
        match self {
            BaseObject::Primitive(p) => p.to_editable_poly(),
            BaseObject::EditablePoly(m) => m.clone(),
            BaseObject::Light(_) | BaseObject::LightTarget => EditablePoly::new(),
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            BaseObject::Primitive(p) => p.name(),
            BaseObject::EditablePoly(_) => "Editable Poly",
            BaseObject::Light(l) => l.kind.label(),
            BaseObject::LightTarget => "Target",
        }
    }
}

/// Object-level transform (3ds Max node TM) — Z up.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Xform {
    pub translation: Vec3,
    pub rotation: Quat,
    pub scale: Vec3,
}

impl Default for Xform {
    fn default() -> Self {
        Self { translation: Vec3::ZERO, rotation: Quat::IDENTITY, scale: Vec3::ONE }
    }
}

impl Xform {
    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
    pub fn from_matrix(m: Mat4) -> Self {
        let (scale, rotation, translation) = m.to_scale_rotation_translation();
        Self { translation, rotation, scale }
    }
}

#[derive(Clone, Debug)]
pub struct SceneObject {
    pub name: String,
    pub base: BaseObject,
    /// Bottom-to-top modifier stack.
    pub modifiers: Vec<ModifierEntry>,
    /// Cached fold of the whole (enabled) stack.
    pub evaluated_mesh: EditablePoly,
    /// Mesh at the active stack level (what sub-object tools edit and pick).
    pub level_mesh: EditablePoly,
    /// `None` = base object, `Some(i)` = modifier `i`.
    pub active_level: Option<usize>,
    pub transform: Xform,
    /// Pivot offset (3ds Max "object offset"): mesh space -> pivot space.
    pub offset: Mat4,
    pub wire_color: [f32; 3],
    pub hidden: bool,
    pub frozen: bool,
    pub see_through: bool,
    /// Show the UV checker texture in shaded viewports.
    pub checker: bool,
    /// Assigned material (instances share the id, like 3ds Max).
    pub material: Option<crate::materials::MatId>,
    pub selected: bool,
    pub instance_group: Option<u32>,
    pub reference_parent: Option<ObjId>,
    /// Number of bottom modifiers inherited from `reference_parent`.
    pub reference_shared: usize,
    /// Bumped whenever geometry changes; the renderer re-uploads on change.
    pub version: u64,
    pub selection_version: u64,
}

impl SceneObject {
    pub fn new(name: String, base: BaseObject) -> Self {
        let mut o = Self {
            name,
            base,
            modifiers: vec![],
            evaluated_mesh: EditablePoly::new(),
            level_mesh: EditablePoly::new(),
            active_level: None,
            transform: Xform::default(),
            offset: Mat4::IDENTITY,
            wire_color: [0.55, 0.78, 0.35],
            hidden: false,
            frozen: false,
            see_through: false,
            checker: false,
            material: None,
            selected: false,
            instance_group: None,
            reference_parent: None,
            reference_shared: 0,
            version: 0,
            selection_version: 0,
        };
        o.evaluate();
        o
    }

    pub fn light(&self) -> Option<&crate::render::Light> {
        match &self.base {
            BaseObject::Light(l) => Some(l),
            _ => None,
        }
    }

    pub fn light_mut(&mut self) -> Option<&mut crate::render::Light> {
        match &mut self.base {
            BaseObject::Light(l) => Some(l),
            _ => None,
        }
    }

    /// Lights and targets: no geometry, picked and framed by their pivot.
    pub fn is_helper(&self) -> bool {
        matches!(self.base, BaseObject::Light(_) | BaseObject::LightTarget)
    }

    /// World matrix applied to the evaluated mesh.
    pub fn world_matrix(&self) -> Mat4 {
        self.transform.matrix() * self.offset
    }
}

#[derive(Clone, Debug, Default)]
pub struct Scene {
    pub objects: SlotMap<ObjId, SceneObject>,
    /// Material library + the 24 Material Editor sample slots.
    pub materials: crate::materials::MaterialLib,
    /// Environment & Effects (`8`): background, ambient, fog, lighting preset.
    pub environment: crate::render::Environment,
    name_counters: HashMap<String, u32>,
    next_instance_group: u32,
}

/// Pleasant 3ds Max-like random wireframe colors.
const WIRE_COLORS: [[f32; 3]; 8] = [
    [0.55, 0.78, 0.35],
    [0.33, 0.62, 0.89],
    [0.88, 0.55, 0.30],
    [0.74, 0.45, 0.85],
    [0.35, 0.80, 0.75],
    [0.90, 0.75, 0.30],
    [0.86, 0.40, 0.48],
    [0.60, 0.68, 0.90],
];

impl Scene {
    pub fn unique_name(&mut self, base: &str) -> String {
        let base = base.trim_end_matches(|c: char| c.is_ascii_digit()).replace(' ', "");
        let c = self.name_counters.entry(base.clone()).or_insert(0);
        *c += 1;
        format!("{base}{:03}", *c)
    }

    pub fn add(&mut self, base: BaseObject, at: Vec3) -> ObjId {
        let name = self.unique_name(base.label());
        let mut o = SceneObject::new(name, base);
        o.transform.translation = at;
        o.wire_color = WIRE_COLORS[self.objects.len() % WIRE_COLORS.len()];
        self.objects.insert(o)
    }

    pub fn selected(&self) -> Vec<ObjId> {
        self.objects.iter().filter(|(_, o)| o.selected).map(|(k, _)| k).collect()
    }

    pub fn select_only(&mut self, id: Option<ObjId>) {
        for (k, o) in self.objects.iter_mut() {
            o.selected = Some(k) == id;
        }
    }

    pub fn delete(&mut self, id: ObjId) {
        let Some(o) = self.objects.remove(id) else { return };
        // A light takes its target along; a deleted target frees its light.
        if let Some(t) = o.light().and_then(|l| l.target) {
            self.objects.remove(t);
        }
        for o in self.objects.values_mut() {
            if o.reference_parent == Some(id) {
                o.reference_parent = None;
                o.reference_shared = 0;
            }
            if let Some(l) = o.light_mut()
                && l.target == Some(id)
            {
                l.target = None;
                l.kind = match l.kind {
                    crate::render::LightKind::TargetSpot => crate::render::LightKind::FreeSpot,
                    _ => crate::render::LightKind::FreeDirect,
                };
            }
        }
    }

    /// Adds a light (plus its target helper for target lights) aimed at `aim`.
    pub fn add_light(&mut self, mut light: crate::render::Light, name: Option<&str>, at: Vec3, aim: Vec3) -> ObjId {
        let has_target = light.kind.has_target();
        light.target = None;
        let wire = [0.98, 0.85, 0.3];
        let name = match name {
            Some(n) => self.unique_name(n),
            None => self.unique_name(light.kind.label()),
        };
        let mut o = SceneObject::new(name.clone(), BaseObject::Light(light));
        o.transform.translation = at;
        o.transform.rotation = crate::render::lighting::aim(at, aim);
        o.wire_color = wire;
        let id = self.objects.insert(o);
        if has_target {
            let mut t = SceneObject::new(format!("{name}.Target"), BaseObject::LightTarget);
            t.transform.translation = aim;
            t.wire_color = wire;
            let tid = self.objects.insert(t);
            if let Some(l) = self.objects[id].light_mut() {
                l.target = Some(tid);
            }
        }
        id
    }

    /// Points every target light at its target. Returns true if any moved.
    pub fn aim_target_lights(&mut self) -> bool {
        let aims: Vec<(ObjId, Quat)> = self
            .objects
            .iter()
            .filter_map(|(k, o)| {
                let t = self.objects.get(o.light()?.target?)?;
                let q = crate::render::lighting::aim(o.transform.translation, t.transform.translation);
                (q.dot(o.transform.rotation).abs() < 1.0 - 1e-7).then_some((k, q))
            })
            .collect();
        for (k, q) in &aims {
            self.objects[*k].transform.rotation = *q;
        }
        !aims.is_empty()
    }

    /// Lights that are switched on (what decides Scene vs Default lighting).
    pub fn active_lights(&self) -> impl Iterator<Item = (ObjId, &SceneObject, &crate::render::Light)> {
        self.objects.iter().filter_map(|(k, o)| o.light().filter(|l| l.on && !o.hidden).map(|l| (k, o, l)))
    }

    pub fn new_instance_group(&mut self) -> u32 {
        self.next_instance_group += 1;
        self.next_instance_group
    }

    /// Totals for the status bar: (verts, edges, polys).
    pub fn stats(&self) -> (usize, usize, usize) {
        self.objects.values().filter(|o| !o.hidden).fold((0, 0, 0), |a, o| {
            let c = o.evaluated_mesh.counts();
            (a.0 + c.0, a.1 + c.1, a.2 + c.2)
        })
    }
}
