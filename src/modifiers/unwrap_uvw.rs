//! Unwrap UVW: per-face-corner UV overrides edited in the 2D UV Editor
//! (Move / Rotate / Scale / Relax / Stitch) plus Smart UV Unwrap.

use crate::mesh::{EditablePoly, VertexId};
use bevy::math::Vec2;
use std::collections::{BTreeSet, HashMap, HashSet};

/// A face corner addressed by (polygon order index, corner index).
pub type CornerRef = (usize, usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum UvSel {
    #[default]
    Vertex,
    Edge,
    Face,
}

#[derive(Clone, Debug)]
pub struct UnwrapMod {
    /// UVs per input polygon (slotmap order), per corner.
    pub uvs: Vec<Vec<Vec2>>,
    /// Selected corners (2D selection; coincident UV vertices move together).
    pub selected: BTreeSet<CornerRef>,
    pub sel_mode: UvSel,
    pub angle_limit: f32,
    pub island_margin: f32,
    pub correct_aspect: bool,
}

impl Default for UnwrapMod {
    fn default() -> Self {
        Self {
            uvs: vec![],
            selected: BTreeSet::new(),
            sel_mode: UvSel::Vertex,
            angle_limit: 66.0,
            island_margin: 0.02,
            correct_aspect: true,
        }
    }
}

/// Polygon corner counts used to detect topology changes below the modifier.
fn layout(m: &EditablePoly) -> Vec<usize> {
    m.polygons.values().map(|p| p.corners.len()).collect()
}

/// Corner -> 3D vertex table for an input mesh (slotmap order).
pub fn corner_vertices(m: &EditablePoly) -> Vec<Vec<VertexId>> {
    m.polygons.values().map(|p| p.vertex_ids().collect()).collect()
}

fn uv_key(v: VertexId, uv: Vec2) -> (VertexId, i64, i64) {
    (v, (uv.x * 1e5).round() as i64, (uv.y * 1e5).round() as i64)
}

impl UnwrapMod {
    /// Captures the incoming UVs (on add / Reset).
    pub fn capture(&mut self, input: &EditablePoly) {
        self.uvs = input.polygons.values().map(|p| p.corners.iter().map(|c| c.uv).collect()).collect();
        self.selected.clear();
    }

    pub fn matches(&self, input: &EditablePoly) -> bool {
        self.uvs.iter().map(Vec::len).eq(layout(input))
    }

    pub fn apply(&self, mesh: &mut EditablePoly) {
        if !self.matches(mesh) {
            return;
        }
        for (p, uvs) in mesh.polygons.values_mut().zip(&self.uvs) {
            for (c, uv) in p.corners.iter_mut().zip(uvs) {
                c.uv = *uv;
            }
        }
    }

    pub fn uv(&self, c: CornerRef) -> Vec2 {
        self.uvs[c.0][c.1]
    }

    /// Every corner that shares a UV vertex with `c` (same 3D vertex, same UV).
    pub fn welded(&self, cv: &[Vec<VertexId>], c: CornerRef) -> Vec<CornerRef> {
        let k = uv_key(cv[c.0][c.1], self.uv(c));
        self.all_corners().filter(|&o| uv_key(cv[o.0][o.1], self.uv(o)) == k).collect()
    }

    pub fn all_corners(&self) -> impl Iterator<Item = CornerRef> + '_ {
        self.uvs.iter().enumerate().flat_map(|(p, cs)| (0..cs.len()).map(move |c| (p, c)))
    }

    /// Adds coincident corners so welded UV vertices stay welded when moved.
    pub fn expand_selection(&mut self, cv: &[Vec<VertexId>]) {
        let keys: HashSet<_> = self.selected.iter().map(|&c| uv_key(cv[c.0][c.1], self.uv(c))).collect();
        let extra: Vec<CornerRef> = self.all_corners().filter(|&c| keys.contains(&uv_key(cv[c.0][c.1], self.uv(c)))).collect();
        self.selected.extend(extra);
    }

    pub fn selection_center(&self) -> Option<Vec2> {
        let n = self.selected.len();
        (n > 0).then(|| self.selected.iter().map(|&c| self.uv(c)).sum::<Vec2>() / n as f32)
    }

    fn map_selected(&mut self, f: impl Fn(Vec2) -> Vec2) {
        let sel: Vec<CornerRef> = self.selected.iter().copied().collect();
        for (p, c) in sel {
            if let Some(uv) = self.uvs.get_mut(p).and_then(|cs| cs.get_mut(c)) {
                *uv = f(*uv);
            }
        }
    }

    pub fn move_selected(&mut self, d: Vec2) {
        self.map_selected(|uv| uv + d);
    }

    pub fn rotate_selected(&mut self, radians: f32) {
        let Some(c) = self.selection_center() else { return };
        let r = Vec2::from_angle(radians);
        self.map_selected(|uv| c + r.rotate(uv - c));
    }

    pub fn scale_selected(&mut self, s: Vec2) {
        let Some(c) = self.selection_center() else { return };
        self.map_selected(|uv| c + (uv - c) * s);
    }

    /// UV-space neighbours of each UV vertex, plus which ones lie on an island border.
    fn uv_graph(&self, cv: &[Vec<VertexId>]) -> UvGraph {
        let mut id_of: HashMap<(VertexId, i64, i64), usize> = HashMap::new();
        let mut corner_ids: Vec<Vec<usize>> = vec![];
        for (p, cs) in self.uvs.iter().enumerate() {
            corner_ids.push(
                (0..cs.len())
                    .map(|c| {
                        let n = id_of.len();
                        *id_of.entry(uv_key(cv[p][c], cs[c])).or_insert(n)
                    })
                    .collect(),
            );
        }
        let n = id_of.len();
        let mut nbrs = vec![BTreeSet::new(); n];
        let mut edge_use: HashMap<(usize, usize), u32> = HashMap::new();
        for ids in &corner_ids {
            for i in 0..ids.len() {
                let (a, b) = (ids[i], ids[(i + 1) % ids.len()]);
                nbrs[a].insert(b);
                nbrs[b].insert(a);
                *edge_use.entry((a.min(b), a.max(b))).or_default() += 1;
            }
        }
        let mut border = vec![false; n];
        for (&(a, b), &k) in &edge_use {
            if k == 1 {
                border[a] = true;
                border[b] = true;
            }
        }
        UvGraph { corner_ids, nbrs, border }
    }

    /// Relax: Laplacian smoothing of UV vertices with island borders pinned.
    /// Affects the selection, or everything when nothing is selected.
    pub fn relax(&mut self, cv: &[Vec<VertexId>], iterations: u32, amount: f32) {
        let g = self.uv_graph(cv);
        let n = g.nbrs.len();
        let mut pos = vec![Vec2::ZERO; n];
        let mut affected = vec![self.selected.is_empty(); n];
        for (p, ids) in g.corner_ids.iter().enumerate() {
            for (c, &id) in ids.iter().enumerate() {
                pos[id] = self.uvs[p][c];
                if self.selected.contains(&(p, c)) {
                    affected[id] = true;
                }
            }
        }
        for _ in 0..iterations {
            let prev = pos.clone();
            for i in (0..n).filter(|&i| affected[i] && !g.border[i] && !g.nbrs[i].is_empty()) {
                let avg = g.nbrs[i].iter().map(|&j| prev[j]).sum::<Vec2>() / g.nbrs[i].len() as f32;
                pos[i] = prev[i].lerp(avg, amount);
            }
        }
        for (p, ids) in g.corner_ids.iter().enumerate() {
            for (c, &id) in ids.iter().enumerate() {
                self.uvs[p][c] = pos[id];
            }
        }
    }

    /// Stitch: selected corners that share a 3D vertex snap to one UV
    /// (the average), welding split UV seams back together.
    pub fn stitch(&mut self, cv: &[Vec<VertexId>]) -> usize {
        let mut groups: HashMap<VertexId, Vec<CornerRef>> = HashMap::new();
        for &c in &self.selected {
            groups.entry(cv[c.0][c.1]).or_default().push(c);
        }
        let mut welded = 0;
        for cs in groups.values() {
            let distinct: HashSet<_> = cs.iter().map(|&c| uv_key(cv[c.0][c.1], self.uv(c))).collect();
            if distinct.len() < 2 {
                continue;
            }
            let avg = cs.iter().map(|&c| self.uv(c)).sum::<Vec2>() / cs.len() as f32;
            for &(p, c) in cs {
                self.uvs[p][c] = avg;
            }
            welded += 1;
        }
        welded
    }

    /// Smart UV Unwrap (angle-based charts, packed into 0..1).
    pub fn smart_unwrap(&mut self, input: &EditablePoly) -> Result<usize, String> {
        let (uvs, charts) = super::smart_uv::smart_uv_project(input, self.angle_limit, self.island_margin, self.correct_aspect)?;
        self.uvs = uvs;
        self.selected.clear();
        Ok(charts)
    }
}

struct UvGraph {
    corner_ids: Vec<Vec<usize>>,
    nbrs: Vec<BTreeSet<usize>>,
    border: Vec<bool>,
}
