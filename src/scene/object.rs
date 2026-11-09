//! Stack evaluation, Edit Poly op routing and pivot tools for `SceneObject`.

use super::*;
use crate::mesh::ops::PolyOp;
use crate::modifiers::{ModifierEntry, ModifierKind};

impl SceneObject {
    /// Folds base + modifiers (bottom to top); also refreshes `level_mesh`.
    pub fn evaluate(&mut self) {
        let (m, shape, level) = self.fold(self.modifiers.len(), self.active_level);
        self.level_mesh = match self.active_level {
            None => self.base.to_editable_poly(),
            Some(_) => level.unwrap_or_else(|| m.clone()),
        };
        self.evaluated_mesh = m;
        self.shape = shape;
        self.version = next_version();
        self.selection_version = next_version();
    }

    /// True when sub-object modes (1–5) are available at the active level.
    pub fn sub_object_editable(&self) -> bool {
        match self.active_level {
            None => matches!(self.base, BaseObject::EditablePoly(_)),
            Some(i) => self.modifiers.get(i).is_some_and(|m| m.kind.supports_sub_objects()),
        }
    }

    pub fn is_editable_poly(&self) -> bool {
        matches!(self.base, BaseObject::EditablePoly(_))
    }

    /// Runs an Edit Poly op on the active level. On an Editable Poly base it
    /// edits the mesh directly; on an Edit Poly modifier it is recorded.
    pub fn apply_op(&mut self, op: PolyOp) -> bool {
        let mut m = self.level_mesh.clone();
        let sel = m.selection_sets();
        if !op.apply(&mut m) {
            return false;
        }
        match self.active_level {
            None => match &mut self.base {
                BaseObject::EditablePoly(base) => *base = m,
                _ => return false,
            },
            Some(i) => match self.modifiers.get_mut(i).map(|md| &mut md.kind) {
                Some(ModifierKind::EditPoly(ep)) => {
                    ep.ops.push((sel, op));
                    ep.selection = m.selection_sets();
                }
                _ => return false,
            },
        }
        self.evaluate();
        true
    }

    /// Stores selection edits made on `level_mesh` into the level's owner.
    pub fn commit_selection(&mut self) {
        let sets = self.level_mesh.selection_sets();
        match self.active_level {
            None => {
                if let BaseObject::EditablePoly(base) = &mut self.base {
                    base.apply_selection_sets(&sets);
                }
            }
            Some(i) => {
                if let Some(ModifierKind::EditPoly(ep)) = self.modifiers.get_mut(i).map(|m| &mut m.kind) {
                    ep.selection = sets;
                }
            }
        }
        self.evaluate();
    }

    /// Convert to Editable Poly / Collapse To: bakes the whole stack.
    pub fn collapse_to_editable_poly(&mut self) {
        let m = self.evaluated_mesh.clone();
        self.base = BaseObject::EditablePoly(m);
        self.modifiers.clear();
        self.active_level = None;
        self.reference_shared = 0;
        self.evaluate();
    }

    /// Adds a modifier above the active level (3ds Max inserts above the
    /// highlighted entry) and makes it active.
    pub fn add_modifier(&mut self, mut md: ModifierEntry) {
        let at = self.active_level.map(|i| i + 1).unwrap_or(0).min(self.modifiers.len());
        md.kind.init(&self.input_of(at));
        self.modifiers.insert(at, md);
        self.active_level = Some(at);
        self.evaluate();
    }

    /// Mesh flowing into modifier `i` (base + enabled modifiers below it).
    pub fn input_of(&self, i: usize) -> crate::mesh::EditablePoly {
        self.fold(i, None).0
    }

    /// Shape flowing into modifier `i` (if no Lathe / Extrude consumed it below).
    pub fn shape_input_of(&self, i: usize) -> Option<crate::splines::Shape> {
        self.fold(i, None).1
    }

    /// Folds the base through the first `n` modifiers. A shape travels up the
    /// stack until a shape modifier (Lathe / Extrude) turns it into a mesh.
    /// Also returns the mesh right after modifier `level` (if asked).
    fn fold(
        &self,
        n: usize,
        level: Option<usize>,
    ) -> (crate::mesh::EditablePoly, Option<crate::splines::Shape>, Option<crate::mesh::EditablePoly>) {
        let mut m = self.base.to_editable_poly();
        let mut shape = self.base.to_shape();
        let mut at_level = None;
        for (i, md) in self.modifiers.iter().enumerate().take(n) {
            match &shape {
                Some(sh) if md.consumes_shape() => {
                    if md.enabled_in_viewport {
                        m = md.build_from_shape(sh);
                        shape = None;
                    }
                }
                _ => md.apply(&mut m),
            }
            if level == Some(i) {
                at_level = Some(m.clone());
            }
        }
        (m, shape, at_level)
    }

    /// Local-space bounds of everything drawn (mesh and live splines).
    pub fn bounds(&self) -> Option<crate::mesh::Aabb> {
        let mesh = self.evaluated_mesh.bounds();
        let shape = self.shape.as_ref().and_then(|s| s.bounds());
        match (mesh, shape) {
            (Some(a), Some(b)) => Some(a.union(&b)),
            (a, b) => a.or(b),
        }
    }

    pub fn shape_object(&self) -> Option<&crate::splines::ShapeObject> {
        match &self.base {
            BaseObject::Shape(s) => Some(s),
            _ => None,
        }
    }

    pub fn shape_object_mut(&mut self) -> Option<&mut crate::splines::ShapeObject> {
        match &mut self.base {
            BaseObject::Shape(s) => Some(s),
            _ => None,
        }
    }

    /// Mesh the renderer uses: Enable In Renderer sweeps a spline that is
    /// not shown as a mesh in the viewport.
    pub fn render_mesh(&self) -> std::borrow::Cow<'_, crate::mesh::EditablePoly> {
        if let (Some(so), Some(sh)) = (self.shape_object(), &self.shape)
            && so.render.renderer
            && !so.render.viewport
        {
            return std::borrow::Cow::Owned(so.render.sweep(sh));
        }
        std::borrow::Cow::Borrowed(&self.evaluated_mesh)
    }

    /// The active modifier, if the stack highlight is on one.
    pub fn active_modifier(&self) -> Option<&ModifierKind> {
        self.active_level.and_then(|i| self.modifiers.get(i)).map(|m| &m.kind)
    }

    pub fn active_modifier_mut(&mut self) -> Option<&mut ModifierKind> {
        let i = self.active_level?;
        self.modifiers.get_mut(i).map(|m| &mut m.kind)
    }

    pub fn delete_active_modifier(&mut self) {
        if let Some(i) = self.active_level {
            if i < self.modifiers.len() {
                self.modifiers.remove(i);
            }
            self.active_level = if i == 0 { None } else { Some(i - 1) };
            self.evaluate();
        }
    }

    pub fn set_active_level(&mut self, level: Option<usize>) {
        self.active_level = level.filter(|&i| i < self.modifiers.len());
        self.evaluate();
    }

    /// Edits a primitive's parameters in place (live dimension spinners).
    pub fn primitive_mut(&mut self) -> Option<&mut Primitive> {
        match &mut self.base {
            BaseObject::Primitive(p) => Some(p),
            _ => None,
        }
    }

    pub fn primitive_mut_ref(&self) -> Option<&Primitive> {
        match &self.base {
            BaseObject::Primitive(p) => Some(p),
            _ => None,
        }
    }

    pub fn geogen_mut(&mut self) -> Option<&mut crate::modifiers::GeoGenParams> {
        match &mut self.base {
            BaseObject::GeoGen(g) => Some(g),
            _ => None,
        }
    }

    // ---- Pivot tab ---------------------------------------------------------

    fn set_pivot(&mut self, new_node: Mat4) {
        let world = self.world_matrix();
        self.offset = new_node.inverse() * world;
        self.transform = Xform::from_matrix(new_node);
    }

    pub fn bounds_in_pivot_space(&self) -> Option<crate::mesh::Aabb> {
        crate::mesh::Aabb::from_points(self.evaluated_mesh.vertices.values().map(|v| self.offset.transform_point3(v.position)))
    }

    pub fn center_pivot(&mut self) {
        if let Some(b) = self.bounds_in_pivot_space() {
            let node = self.transform.matrix() * Mat4::from_translation(b.center());
            self.set_pivot(node);
        }
    }

    pub fn bottom_center_pivot(&mut self) {
        if let Some(b) = self.bounds_in_pivot_space() {
            let c = b.center();
            let node = self.transform.matrix() * Mat4::from_translation(Vec3::new(c.x, c.y, b.min.z));
            self.set_pivot(node);
        }
    }

    pub fn align_pivot_to_world(&mut self) {
        let node = Mat4::from_scale_rotation_translation(self.transform.scale, Quat::IDENTITY, self.transform.translation);
        self.set_pivot(node);
    }

    /// Reset XForm: bakes rotation & scale into the object offset.
    pub fn reset_xform(&mut self) {
        let node = Mat4::from_translation(self.transform.translation);
        self.set_pivot(node);
    }

    /// Moves the pivot by `delta` (world) without moving the geometry (Affect Pivot Only).
    pub fn move_pivot(&mut self, delta: Vec3) {
        let node = Mat4::from_translation(delta) * self.transform.matrix();
        self.set_pivot(node);
    }

    /// Mesh-space (pre-offset) matrix converting world points into this object's mesh space.
    pub fn world_to_mesh(&self) -> Mat4 {
        self.world_matrix().inverse()
    }
}
