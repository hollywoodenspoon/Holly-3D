//! Scene graph: objects with base object + modifier stack + transform.

pub mod camera;
mod clone;
mod object;
pub mod shared;
pub use shared::Shared;
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum BaseObject {
    Primitive(Primitive),
    EditablePoly(Shared<EditablePoly>),
    /// Light object (no geometry; drawn as a viewport icon).
    Light(crate::render::Light),
    /// Target helper of a target light or Target Camera.
    Target,
    /// Target / Free Camera.
    Camera(camera::Camera),
    /// Spline shape (parametric or Editable Spline).
    Shape(crate::splines::ShapeObject),
    /// GeoGen procedural rock / crystal / cliff / stalagmite.
    GeoGen(crate::modifiers::GeoGenParams),
    /// Mech primitive: Gear, Hydraulic Piston part or Hose.
    Mech(crate::mech::MechPart),
    /// Procedural world: Terrain, Ocean or Planet.
    World(crate::generators::WorldGen),
    /// Phase 10A Character Studio humanoid (MakeHuman hm08) or creature.
    Character(Box<crate::character::CharacterParams>),
}

impl BaseObject {
    pub fn to_editable_poly(&self) -> EditablePoly {
        match self {
            BaseObject::Primitive(p) => p.to_editable_poly(),
            BaseObject::EditablePoly(m) => EditablePoly::clone(m),
            BaseObject::Light(_) | BaseObject::Target | BaseObject::Camera(_) => EditablePoly::new(),
            BaseObject::Shape(s) => s.viewport_mesh(),
            BaseObject::GeoGen(g) => g.build(),
            BaseObject::Mech(m) => m.build(),
            BaseObject::World(w) => w.build(),
            BaseObject::Character(c) => c.build(),
        }
    }
    /// The shape flowing into the stack (shapes only).
    pub fn to_shape(&self) -> Option<crate::splines::Shape> {
        match self {
            BaseObject::Shape(s) => Some(s.to_shape()),
            _ => None,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            BaseObject::Primitive(p) => p.name(),
            BaseObject::EditablePoly(_) => "Editable Poly",
            BaseObject::Light(l) => l.kind.label(),
            BaseObject::Target => "Target",
            BaseObject::Camera(c) => c.kind.label(),
            BaseObject::Shape(s) => s.label(),
            BaseObject::GeoGen(g) => g.mode.short(),
            BaseObject::Mech(m) => m.name(),
            BaseObject::World(w) => w.short(),
            BaseObject::Character(c) => c.short(),
        }
    }
}

/// Object-level transform (node TM) — Z up.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SceneObject {
    pub name: String,
    pub base: BaseObject,
    /// Bottom-to-top modifier stack.
    pub modifiers: Vec<ModifierEntry>,
    /// Cached fold of the whole (enabled) stack.
    #[serde(skip)]
    pub evaluated_mesh: Shared<EditablePoly>,
    /// Mesh at the active stack level (what sub-object tools edit and pick).
    #[serde(skip)]
    pub level_mesh: Shared<EditablePoly>,
    /// Shape still flowing out of the stack (drawn as splines); `None` once a
    /// Lathe / Extrude turned it into a mesh, and for non-shape objects.
    #[serde(skip)]
    pub shape: Option<crate::splines::Shape>,
    /// `None` = base object, `Some(i)` = modifier `i`.
    pub active_level: Option<usize>,
    pub transform: Xform,
    /// Pivot offset ("object offset"): mesh space -> pivot space.
    pub offset: Mat4,
    pub wire_color: [f32; 3],
    pub hidden: bool,
    pub frozen: bool,
    pub see_through: bool,
    /// Display property: Shadow & Reflection Catcher (alpha ground plane).
    #[serde(default)]
    pub shadow_catcher: bool,
    /// Show the UV checker texture in shaded viewports.
    pub checker: bool,
    /// Assigned material (instances share the id).
    pub material: Option<crate::materials::MatId>,
    pub selected: bool,
    pub instance_group: Option<u32>,
    pub reference_parent: Option<ObjId>,
    /// Number of bottom modifiers inherited from `reference_parent`.
    pub reference_shared: usize,
    /// Bumped whenever geometry changes; the renderer re-uploads on change.
    #[serde(skip)]
    pub version: u64,
    #[serde(skip)]
    pub selection_version: u64,
}

impl SceneObject {
    pub fn new(name: String, base: BaseObject) -> Self {
        let mut o = Self {
            name,
            base,
            modifiers: vec![],
            evaluated_mesh: Default::default(),
            level_mesh: Default::default(),
            shape: None,
            active_level: None,
            transform: Xform::default(),
            offset: Mat4::IDENTITY,
            wire_color: [0.55, 0.78, 0.35],
            hidden: false,
            frozen: false,
            see_through: false,
            shadow_catcher: false,
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

    /// Lights, cameras and targets: no geometry, picked and framed by their pivot.
    pub fn is_helper(&self) -> bool {
        matches!(self.base, BaseObject::Light(_) | BaseObject::Target | BaseObject::Camera(_))
    }

    /// A placed 3D cloud (hidden by Environment -> Clouds: Off / 2D).
    pub fn is_cloud(&self) -> bool {
        use crate::generators::WorldGen;
        matches!(self.base, BaseObject::World(WorldGen::Cloud(_) | WorldGen::CloudField(_)))
    }

    /// A Trail, Pyro, Smoke Plume or VDB Volume effect (rendered as volume
    /// media).
    pub fn is_fx(&self) -> bool {
        use crate::generators::WorldGen;
        matches!(self.base, BaseObject::World(WorldGen::Trail(_) | WorldGen::Pyro(_) | WorldGen::Smoke(_) | WorldGen::Vdb(_)))
    }

    /// World matrix applied to the evaluated mesh.
    pub fn world_matrix(&self) -> Mat4 {
        self.transform.matrix() * self.offset
    }
}

#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Scene {
    pub objects: SlotMap<ObjId, SceneObject>,
    /// Material library + the 24 Material Editor sample slots.
    pub materials: crate::materials::MaterialLib,
    /// Environment & Effects (`8`): background, ambient, fog, lighting preset.
    pub environment: crate::render::Environment,
    /// Mech joints between parts (J tool).
    #[serde(default)]
    pub assembly: crate::assembly::Assembly,
    name_counters: HashMap<String, u32>,
    next_instance_group: u32,
}

/// Pleasant random wireframe colors.
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

    /// Name counters, saved by live-preview dialogs so rebuilt previews
    /// reuse the same names (Box002, Box003...) instead of skipping ahead.
    pub fn name_state(&self) -> HashMap<String, u32> {
        self.name_counters.clone()
    }
    pub fn restore_names(&mut self, state: HashMap<String, u32>) {
        self.name_counters = state;
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
        // Lights / cameras take their target along; a deleted target frees its owner.
        if let Some(t) = o.target_id() {
            self.objects.remove(t);
        }
        for o in self.objects.values_mut() {
            if o.reference_parent == Some(id) {
                o.reference_parent = None;
                o.reference_shared = 0;
            }
            if o.target_id() == Some(id) {
                o.lose_target();
            }
        }
    }

    /// Adds a light (plus its target helper for target lights) aimed at `aim`.
    pub fn add_light(&mut self, light: crate::render::Light, name: Option<&str>, at: Vec3, aim: Vec3) -> ObjId {
        let has_target = light.kind.has_target();
        let name = self.unique_name(name.unwrap_or(light.kind.label()));
        let mut o = SceneObject::new(name, BaseObject::Light(light));
        o.wire_color = [0.98, 0.85, 0.3];
        self.insert_aimed(o, at, aim, has_target)
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
