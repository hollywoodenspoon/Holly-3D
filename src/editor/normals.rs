//! Edit Normals in the viewport: normal-line data, click / marquee selection
//! (By Normal, Vertex, Edge, Face) and E-gizmo rotation of the selection.

use super::select::{EDGE_PICK_PX, SelectOp, VERTEX_PICK_PX};
use super::*;
use crate::mesh::EditablePoly;
use crate::modifiers::edit_normals::{CornerKey, corners, keys_at_edge, keys_at_vertex, keys_of_face};
use crate::modifiers::{EditNormalsMod, ModifierKind, NormalKind, NormalSelBy};
use crate::viewport::picking::{in_rect, pick_edge, pick_polygon, pick_vertex, point_segment};
use bevy::math::{Mat4, Quat};

/// Normals drawn per object at most (huge meshes stay responsive).
pub const MAX_NORMAL_LINES: usize = 40_000;

/// One normal line in world space.
pub struct NormalLine {
    pub key: CornerKey,
    pub base: Vec3,
    pub tip: Vec3,
    pub kind: NormalKind,
    pub selected: bool,
}

impl Editor {
    /// The object and its active Edit Normals modifier. Its `level_mesh`
    /// (the modifier's output) shares the input's topology and positions.
    fn edit_normals(&self) -> Option<(&SceneObject, &EditNormalsMod)> {
        let o = self.current_obj()?;
        match o.active_modifier()? {
            ModifierKind::EditNormals(e) => Some((o, e)),
            _ => None,
        }
    }

    /// Normal lines of the active Edit Normals modifier (world space).
    pub fn normal_lines(&self) -> Vec<NormalLine> {
        let Some((o, e)) = self.edit_normals() else { return vec![] };
        let input = &o.level_mesh;
        let w = o.world_matrix();
        let cur = e.current(input);
        corners(input)
            .take(MAX_NORMAL_LINES)
            .map(|(key, v)| {
                let p = input.pos(v);
                let (n, kind) = cur[&key];
                NormalLine {
                    key,
                    base: w.transform_point3(p),
                    tip: w.transform_point3(p + n * e.display_length),
                    kind,
                    selected: e.selected.contains(&key),
                }
            })
            .collect()
    }

    fn set_normal_selection(&mut self, keys: Vec<CornerKey>, op: SelectOp) {
        let Some(id) = self.current() else { return };
        let o = &mut self.scene.objects[id];
        if let Some(ModifierKind::EditNormals(e)) = o.active_modifier_mut() {
            if op == SelectOp::Replace {
                e.selected.clear();
            }
            for k in keys {
                if op == SelectOp::Subtract {
                    e.selected.remove(&k);
                } else {
                    e.selected.insert(k);
                }
            }
        }
        o.selection_version = crate::scene::next_version();
    }

    fn normals_input(&self) -> Option<(EditablePoly, Mat4, NormalSelBy)> {
        let (o, e) = self.edit_normals()?;
        Some((o.level_mesh.clone(), o.world_matrix(), e.select_by))
    }

    pub(crate) fn normals_click(&mut self, view: usize, cursor: Vec2, op: SelectOp) {
        let Some((input, w, by)) = self.normals_input() else { return };
        let v = self.viewports.views[view].clone();
        let project = |p: Vec3| v.to_screen(p);
        let keys = match by {
            NormalSelBy::Normal => self
                .normal_lines()
                .into_iter()
                .filter_map(|l| Some((l.key, point_segment(cursor, v.to_screen(l.base)?, v.to_screen(l.tip)?).0)))
                .filter(|x| x.1 <= VERTEX_PICK_PX)
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|x| vec![x.0])
                .unwrap_or_default(),
            NormalSelBy::Vertex => {
                pick_vertex(&input, w, &project, cursor, VERTEX_PICK_PX, &|_| true).map_or(vec![], |vid| keys_at_vertex(&input, vid))
            }
            NormalSelBy::Edge => pick_edge(&input, w, &project, cursor, EDGE_PICK_PX, &|_| true)
                .map_or(vec![], |(e, _)| keys_at_edge(&input, input.edges[e].v0, input.edges[e].v1)),
            NormalSelBy::Face => pick_polygon(&input, w, &v.ray(cursor), false).map_or(vec![], |h| keys_of_face(&input, h.poly)),
        };
        self.set_normal_selection(keys, op);
    }

    pub(crate) fn normals_marquee(&mut self, view: usize, a: Vec2, b: Vec2, op: SelectOp) {
        let Some((input, w, by)) = self.normals_input() else { return };
        let v = self.viewports.views[view].clone();
        let (min, max) = (a.min(b), a.max(b));
        let inside = |p: Vec3| v.to_screen(w.transform_point3(p)).is_some_and(|s| in_rect(s, min, max));
        let keys: Vec<CornerKey> = match by {
            NormalSelBy::Normal => self
                .normal_lines()
                .into_iter()
                .filter(|l| v.to_screen(l.tip).is_some_and(|s| in_rect(s, min, max)))
                .map(|l| l.key)
                .collect(),
            NormalSelBy::Vertex => corners(&input).filter(|c| inside(input.pos(c.1))).map(|c| c.0).collect(),
            NormalSelBy::Edge => input
                .edges
                .values()
                .filter(|e| inside(input.pos(e.v0)) && inside(input.pos(e.v1)))
                .flat_map(|e| keys_at_edge(&input, e.v0, e.v1))
                .collect(),
            NormalSelBy::Face => {
                input.polygons.keys().filter(|&p| inside(input.poly_centroid(p))).flat_map(|p| keys_of_face(&input, p)).collect()
            }
        };
        self.set_normal_selection(keys, op);
    }

    /// Mesh-space center of the selected normals' vertices (gizmo position).
    pub(crate) fn normals_center(&self) -> Option<Vec3> {
        let (o, e) = self.edit_normals()?;
        let input = &o.level_mesh;
        let pts: Vec<Vec3> = e
            .selected
            .iter()
            .filter_map(|&(p, i)| input.polygons.get(p).and_then(|poly| poly.corners.get(i)).map(|c| input.pos(c.vertex_id)))
            .collect();
        (!pts.is_empty()).then(|| pts.iter().sum::<Vec3>() / pts.len() as f32)
    }
}

/// Rotation part of a mesh-space drag (E gizmo) applied to the selected normals.
pub(crate) fn rotate_normals(e: &mut EditNormalsMod, input: &EditablePoly, local: Mat4) {
    let (_, q, _) = local.to_scale_rotation_translation();
    if q.angle_between(Quat::IDENTITY) > 1e-6 {
        e.rotate_selected(input, q);
    }
}
