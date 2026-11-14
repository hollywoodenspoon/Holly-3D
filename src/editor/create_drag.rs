//! Click-and-drag creation. Drag out the base (corner to corner or
//! a radius), release, then move the mouse and click for height and any
//! further step. Right-click cancels a half-made object; a plain click
//! still places a default-size object. The tool stays active afterwards.

use super::*;
use crate::generators::{WorldGen, WorldKind};
use crate::mech::{MechKind, MechPart};
use crate::modifiers::GeoMode;
use crate::primitives::Primitive;
use crate::scene::BaseObject;
use bevy::math::Quat;

/// Pointer travel (pixels) before a press becomes a drag (above egui's
/// 6 px click distance, so a click never also starts an object).
pub const DRAG_START_PX: f32 = 7.0;
const MIN: f32 = 0.01;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CreateKind {
    Prim(PrimitiveKind),
    Geo(GeoMode),
    World(WorldKind),
    Mech(MechKind),
    /// Drag sets the height (uniform scale).
    Character(super::characters::CharacterPick),
}

/// Creation Method rollout (Create tab).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CreateMethod {
    /// Box: drag a cube out from its centre instead of base + height.
    pub cube: bool,
    /// Round objects: the drag spans the diameter (Edge) instead of the radius (Center).
    pub edge: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CreateDrag {
    pub view: usize,
    pub kind: CreateKind,
    /// Created once the pointer has moved [`DRAG_START_PX`].
    pub id: Option<ObjId>,
    pub step: u8,
    pub press: Vec2,
    pub start: Vec3,
    pub rot: Quat,
    pub center: Vec3,
    /// Rect: width, length, height. Round: radius, height, second radius.
    pub vals: [f32; 3],
    pub anchor: Vec2,
    /// World units per pixel at the anchor (height steps).
    pub per_px: f32,
    pub tab: CommandTab,
}

impl CreateKind {
    pub fn of(tool: &Tool) -> Option<Self> {
        match *tool {
            Tool::Create(k) => Some(Self::Prim(k)),
            Tool::CreateGeoGen(m) => Some(Self::Geo(m)),
            Tool::CreateWorld(k) => Some(Self::World(k)),
            Tool::CreateMech(MechKind::Gear) => Some(Self::Mech(MechKind::Gear)),
            Tool::CreateCharacter(c) => Some(Self::Character(c)),
            _ => None,
        }
    }

    /// Steps: 1 = the drag alone, 2 = + height, 3 = + a second radius.
    pub fn steps(self, m: CreateMethod) -> u8 {
        match self {
            Self::Prim(PrimitiveKind::Box) if m.cube => 1,
            Self::Prim(PrimitiveKind::Box | PrimitiveKind::Cylinder | PrimitiveKind::Torus) => 2,
            Self::Prim(PrimitiveKind::Cone) => 3,
            Self::Geo(_) | Self::World(WorldKind::Terrain | WorldKind::Ocean | WorldKind::Cloud(_)) => 2,
            _ => 1,
        }
    }

    /// The first step drags a rectangle (else a radius).
    pub fn is_rect(self) -> bool {
        matches!(
            self,
            Self::Prim(PrimitiveKind::Box | PrimitiveKind::Plane) | Self::World(WorldKind::Terrain | WorldKind::Ocean | WorldKind::Settlement(_) | WorldKind::Building(_) | WorldKind::CloudField)
        )
    }

    /// Status-bar prompt for step `step`.
    pub fn prompt(self, step: u8) -> &'static str {
        match (step, self.is_rect(), self) {
            (0, _, Self::Character(_)) => "Drag to set the character's height, right-click cancels",
            (0, true, _) => "Drag out the base (corner to corner), right-click cancels",
            (0, false, _) => "Drag out the radius, right-click cancels",
            (1, _, Self::Prim(PrimitiveKind::Torus)) => "Move to set the tube thickness, click to finish",
            (1, _, Self::World(WorldKind::Ocean)) => "Move to set the wave height, click to finish",
            (1, _, Self::Prim(PrimitiveKind::Cone)) => "Move to set the height, click for the top radius",
            (1, _, _) => "Move to set the height, click to finish",
            _ => "Move to set the top radius, click to finish",
        }
    }
}

impl Editor {
    /// Left press in a viewport while a Create tool is active. Returns true when used.
    pub fn create_press(&mut self, view: usize, cursor: Vec2) -> bool {
        if let Some(d) = &self.iface.create_drag
            && d.step > 0
        {
            self.create_next_step(cursor);
            return true;
        }
        let Some(kind) = CreateKind::of(&self.tool) else { return false };
        let start = self.snapped_point_pub(view, cursor);
        let rot = self.view_plane_rotation(view);
        self.iface.create_drag = Some(CreateDrag {
            view,
            kind,
            id: None,
            step: 0,
            press: cursor,
            start,
            rot,
            center: start,
            vals: [MIN; 3],
            anchor: cursor,
            per_px: 0.01,
            tab: self.tab,
        });
        true
    }

    /// Pointer moved (button down during the first step, hovering after it).
    pub fn create_move(&mut self, cursor: Vec2) {
        let Some(mut d) = self.iface.create_drag.clone() else { return };
        let p = self.snapped_point_pub(d.view, cursor);
        if d.step == 0 {
            if d.id.is_none() {
                if cursor.distance(d.press) < DRAG_START_PX {
                    return;
                }
                d.id = Some(self.create_kind_at(d.kind, d.start, d.rot));
                self.tab = d.tab;
            }
            let local = d.rot.inverse() * (p - d.start);
            if self.iface.create_method.cube && d.kind == CreateKind::Prim(PrimitiveKind::Box) {
                let s = (2.0 * local.x.abs().max(local.y.abs())).max(MIN);
                d.vals = [s, s, s];
                d.center = d.start;
            } else if d.kind.is_rect() {
                d.vals[0] = local.x.abs().max(MIN);
                d.vals[1] = local.y.abs().max(MIN);
                d.center = d.start + d.rot * Vec3::new(local.x, local.y, 0.0) * 0.5;
            } else if self.iface.create_method.edge && !matches!(d.kind, CreateKind::Character(_)) {
                d.vals[0] = (local.length() * 0.5).max(MIN);
                d.center = d.start + d.rot * local * 0.5;
            } else {
                d.vals[0] = local.length().max(MIN);
                d.center = d.start;
            }
        } else {
            let up = (d.anchor.y - cursor.y) * d.per_px;
            let round = !d.kind.is_rect();
            match (d.step, d.kind) {
                (1, CreateKind::Prim(PrimitiveKind::Torus)) => d.vals[1] = ((p - d.center).length() - d.vals[0]).abs().max(MIN),
                (1, _) if round => d.vals[1] = up.max(MIN),
                (1, _) => d.vals[2] = up.max(MIN),
                _ => d.vals[2] = (p - d.center).length(),
            }
        }
        self.apply_create_drag(&d);
        self.iface.create_drag = Some(d);
    }

    /// Left release: ends the first step. A press that never moved is left
    /// to the plain click (default-size object).
    pub fn create_release(&mut self, cursor: Vec2) {
        let Some(d) = &mut self.iface.create_drag else { return };
        if d.step > 0 {
            return;
        }
        if d.id.is_none() {
            self.iface.create_drag = None;
            return;
        }
        let steps = d.kind.steps(self.iface.create_method);
        if steps <= 1 {
            self.finish_create();
            return;
        }
        d.step = 1;
        d.anchor = cursor;
        let v = &self.viewports.views[d.view];
        d.per_px = v.construction_point(cursor + Vec2::Y).distance(v.construction_point(cursor)).clamp(1e-4, 10.0);
        if d.kind == CreateKind::Prim(PrimitiveKind::Cone) {
            d.vals[2] = 0.0;
        }
        self.status = d.kind.prompt(1).into();
    }

    fn create_next_step(&mut self, cursor: Vec2) {
        let Some(d) = &mut self.iface.create_drag else { return };
        if d.step + 1 >= d.kind.steps(self.iface.create_method) {
            self.finish_create();
            return;
        }
        d.step += 1;
        d.anchor = cursor;
        self.status = d.kind.prompt(d.step).into();
    }

    fn finish_create(&mut self) {
        let Some(d) = self.iface.create_drag.take() else { return };
        if let Some(id) = d.id {
            self.status = format!("Created {}", self.scene.objects[id].name);
        }
    }

    /// Right-click while creating: removes the half-made object. True if one was cancelled.
    pub fn cancel_create(&mut self) -> bool {
        let Some(d) = self.iface.create_drag.take() else { return false };
        if d.id.is_some()
            && let Some(s) = self.history.discard_last()
        {
            self.restore(s);
            self.status = "Creation cancelled".into();
        }
        d.id.is_some()
    }

    fn create_kind_at(&mut self, kind: CreateKind, at: Vec3, rot: Quat) -> ObjId {
        match kind {
            CreateKind::Prim(k) => self.create_primitive(k, at),
            CreateKind::Geo(m) => self.create_geogen(m, at),
            CreateKind::World(k) => self.create_world(k, at),
            CreateKind::Mech(k) => self.create_mech(k, at),
            CreateKind::Character(c) => {
                self.create_pick(c, at);
            }
        }
        let id = self.current().expect("just created");
        self.scene.objects[id].transform.rotation = rot;
        id
    }

    fn apply_create_drag(&mut self, d: &CreateDrag) {
        let Some(id) = d.id else { return };
        let [a, b, c] = d.vals;
        let o = &mut self.scene.objects[id];
        o.transform.translation = d.center;
        if let BaseObject::Character(_) = &o.base {
            // Only the transform changes: no rebuild while dragging.
            let h0 = o.evaluated_mesh.bounds().map_or(1.0, |b| (b.max.z - b.min.z).max(1e-3));
            o.transform.scale = Vec3::splat(a.max(0.1) / h0);
            return;
        }
        match &mut o.base {
            BaseObject::Primitive(p) => match p {
                Primitive::Box { length, width, height, .. } => (*width, *length, *height) = (a, b, c),
                Primitive::Plane { length, width, .. } => (*width, *length) = (a, b),
                Primitive::Sphere { radius, .. } => *radius = a,
                Primitive::Cylinder { radius, height, .. } => (*radius, *height) = (a, b),
                Primitive::Cone { radius1, radius2, height, .. } => (*radius1, *height, *radius2) = (a, b, c),
                Primitive::Torus { radius1, radius2, .. } => (*radius1, *radius2) = (a, b.min(a)),
            },
            BaseObject::GeoGen(g) => {
                let ratio = if g.max_height > 0.0 { g.min_height / g.max_height } else { 0.5 };
                g.base_radius = a;
                if d.step > 0 {
                    (g.max_height, g.min_height) = (b, b * ratio);
                }
            }
            BaseObject::World(w) => match w {
                WorldGen::Terrain(t) => {
                    (t.width, t.length) = (a, b);
                    if d.step > 0 {
                        t.max_elevation = c;
                    }
                }
                WorldGen::Ocean(t) => {
                    (t.width, t.length) = (a, b);
                    if d.step > 0 {
                        t.wave_height = c;
                    }
                }
                WorldGen::Planet(p) => p.radius = a,
                WorldGen::Settlement(s) => (s.width, s.length) = (a.max(4.0), b.max(4.0)),
                WorldGen::Building(s) => (s.width, s.length) = (a.max(1.0), b.max(1.0)),
                WorldGen::Cloud(s) => {
                    s.size = (2.0 * a).max(1.0);
                    if d.step > 0 {
                        s.height = c.max(1.0);
                    }
                }
                WorldGen::CloudField(s) => (s.width, s.depth) = (a.max(10.0), b.max(10.0)),
                WorldGen::Trail(s) => (s.length, s.dissipation) = ((2.0 * a).max(0.5), (2.0 * a).max(0.5)),
                WorldGen::Pyro(s) => s.size = a.max(0.5),
                WorldGen::Smoke(s) => {
                    s.base_radius = a.max(0.1);
                    if d.step > 0 {
                        s.height = c.max(1.0);
                    }
                }
                WorldGen::Vdb(_) => {}
            },
            BaseObject::Mech(MechPart::Gear(g)) => g.module = (2.0 * a / g.teeth.max(1) as f32).max(MIN),
            _ => {}
        }
        o.evaluate();
        self.scene.propagate_from(id);
    }
}

#[cfg(test)]
#[path = "create_drag_tests.rs"]
mod tests;
