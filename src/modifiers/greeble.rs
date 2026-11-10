//! Greeble: sci-fi hull detail. Each target polygon (selected, or all) is
//! split into random rectangular panels that are raised with a tapered top,
//! and small widgets (boxes, cylinders, ribs) are scattered on the tops.

use super::geogen::{range, rng};
use crate::mesh::{EditablePoly, PolyId};
use bevy::math::{Vec2, Vec3};
use rand_chacha::ChaCha8Rng;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct GreebleMod {
    pub seed: u32,
    pub min_height: f32,
    pub max_height: f32,
    /// Inward bevel of panel tops, 0..1.
    pub taper: f32,
    pub select_tops: bool,
    pub panels: bool,
    /// Each level splits every panel in two (1..=3).
    pub levels: u32,
    pub widget_density: f32,
    pub min_widget: f32,
    pub max_widget: f32,
    pub boxes: bool,
    pub cylinders: bool,
    pub ribs: bool,
    pub panel_mat: u16,
    pub widget_mat: u16,
}

impl Default for GreebleMod {
    fn default() -> Self {
        Self {
            seed: 1,
            min_height: 0.02,
            max_height: 0.1,
            taper: 0.15,
            select_tops: false,
            panels: true,
            levels: 2,
            widget_density: 0.5,
            min_widget: 0.03,
            max_widget: 0.08,
            boxes: true,
            cylinders: true,
            ribs: true,
            panel_mat: 1,
            widget_mat: 2,
        }
    }
}

/// Never generate more panels than this.
pub const MAX_PANELS: usize = 4000;

/// Closed prism over the CCW `base` footprint: bottom, top, sides.
/// The top is scaled toward its center by `1 - taper`. Returns the top polygon.
pub fn prism(m: &mut EditablePoly, base: &[Vec3], n: Vec3, h: f32, taper: f32, mat: u16) -> PolyId {
    let c = base.iter().sum::<Vec3>() / base.len() as f32;
    let k = 1.0 - taper.clamp(0.0, 0.95);
    let sink = n * h * 0.01;
    let b: Vec<_> = base.iter().map(|&p| m.add_vertex(p - sink)).collect();
    let t: Vec<_> = base.iter().map(|&p| m.add_vertex(c + (p - c) * k + n * h)).collect();
    let len = base.len();
    let uv = |i: usize, v: f32| Vec2::new(i as f32 / len as f32, v);
    let top = m.add_polygon_uv(&t, &(0..len).map(|i| uv(i, 1.0)).collect::<Vec<_>>());
    let rb: Vec<_> = b.iter().rev().copied().collect();
    let bottom = m.add_polygon_uv(&rb, &(0..len).map(|i| uv(i, 0.0)).collect::<Vec<_>>());
    let mut ids = vec![top, bottom];
    for i in 0..len {
        let j = (i + 1) % len;
        ids.push(m.add_polygon_uv(&[b[i], b[j], t[j], t[i]], &[uv(i, 0.0), uv(i + 1, 0.0), uv(i + 1, 1.0), uv(i, 1.0)]));
    }
    for id in ids {
        m.polygons[id].material_id = mat;
        m.polygons[id].smoothing_group = 0;
    }
    top
}

/// Random kd-split of the unit square, `levels` times (sizes measured in world units).
fn split_rects(r: &mut ChaCha8Rng, levels: u32, su: f32, sv: f32) -> Vec<[f32; 4]> {
    let mut rects = vec![[0.0, 0.0, 1.0, 1.0]];
    for _ in 0..levels.clamp(1, 3) {
        rects = rects
            .into_iter()
            .flat_map(|[u0, v0, u1, v1]| {
                let t = range(r, 0.3, 0.7);
                if (u1 - u0) * su >= (v1 - v0) * sv {
                    let u = u0 + (u1 - u0) * t;
                    [[u0, v0, u, v1], [u, v0, u1, v1]]
                } else {
                    let v = v0 + (v1 - v0) * t;
                    [[u0, v0, u1, v], [u0, v, u1, v1]]
                }
            })
            .collect();
    }
    rects
}

impl GreebleMod {
    /// Panel footprints (CCW, on the surface) for one polygon.
    fn footprints(&self, m: &EditablePoly, p: PolyId, r: &mut ChaCha8Rng) -> Vec<Vec<Vec3>> {
        let pts: Vec<Vec3> = m.polygons[p].vertex_ids().map(|v| m.pos(v)).collect();
        let shrink = |f: Vec<Vec3>| {
            let c = f.iter().sum::<Vec3>() / f.len() as f32;
            f.into_iter().map(|q| c + (q - c) * 0.9).collect::<Vec<_>>()
        };
        if pts.len() != 4 {
            return vec![shrink(pts)];
        }
        let at = |s: f32, t: f32| pts[0].lerp(pts[1], s).lerp(pts[3].lerp(pts[2], s), t);
        split_rects(r, self.levels, pts[0].distance(pts[1]), pts[0].distance(pts[3]))
            .into_iter()
            .map(|[u0, v0, u1, v1]| shrink(vec![at(u0, v0), at(u1, v0), at(u1, v1), at(u0, v1)]))
            .collect()
    }

    fn widget(&self, out: &mut EditablePoly, top: &[Vec3], n: Vec3, r: &mut ChaCha8Rng) {
        let kinds: Vec<u8> = [(self.boxes, 0), (self.cylinders, 1), (self.ribs, 2)].iter().filter(|k| k.0).map(|k| k.1).collect();
        if kinds.is_empty() {
            return;
        }
        let kind = kinds[(range(r, 0.0, kinds.len() as f32) as usize).min(kinds.len() - 1)];
        let c = top.iter().sum::<Vec3>() / top.len() as f32;
        let room = top.iter().map(|&q| q.distance(c)).fold(f32::MAX, f32::min);
        let s = range(r, self.min_widget, self.max_widget.max(self.min_widget)).min(room * 0.8);
        if s <= 1e-4 {
            return;
        }
        let tu = (top[1] - top[0]).normalize_or(n.any_orthonormal_vector());
        let tv = n.cross(tu);
        let at = c + tu * range(r, -0.3, 0.3) * (room - s * 0.5).max(0.0) + tv * range(r, -0.3, 0.3) * (room - s * 0.5).max(0.0);
        let rect = |a: f32, b: f32| vec![at - tu * a - tv * b, at + tu * a - tv * b, at + tu * a + tv * b, at - tu * a + tv * b];
        let (base, h) = match kind {
            0 => (rect(s * 0.5, s * 0.5), s * 0.6),
            1 => {
                let ring =
                    (0..8).map(|i| i as f32 / 8.0 * std::f32::consts::TAU).map(|a| at + (tu * a.cos() + tv * a.sin()) * s * 0.5).collect();
                (ring, s * 0.8)
            }
            _ => (rect(s * 0.9, s * 0.15), s * 0.3),
        };
        prism(out, &base, n, h, 0.0, self.widget_mat);
    }

    pub fn apply(&self, m: &mut EditablePoly) {
        let selected = m.selected_polygons();
        let targets = if selected.is_empty() { m.polygons.keys().collect() } else { selected };
        let mut r = rng(self.seed, 7);
        let mut out = EditablePoly::new();
        let mut tops = vec![];
        let mut count = 0;
        for &p in &targets {
            let n = m.poly_normal(p);
            let feet =
                if self.panels { self.footprints(m, p, &mut r) } else { vec![m.polygons[p].vertex_ids().map(|v| m.pos(v)).collect()] };
            for f in feet {
                if count >= MAX_PANELS {
                    break;
                }
                count += 1;
                let top: Vec<Vec3> = if self.panels {
                    let h = range(&mut r, self.min_height, self.max_height.max(self.min_height));
                    let id = prism(&mut out, &f, n, h, self.taper, self.panel_mat);
                    tops.push(id);
                    out.polygons[id].vertex_ids().map(|v| out.pos(v)).collect()
                } else {
                    f
                };
                if range(&mut r, 0.0, 1.0) < self.widget_density {
                    self.widget(&mut out, &top, n, &mut r);
                }
            }
        }
        out.finalize();
        if self.select_tops {
            m.polygons.values_mut().for_each(|p| p.selected = false);
            out.polygons.values_mut().for_each(|p| p.selected = false);
            for t in tops {
                out.polygons[t].selected = true;
            }
        }
        m.append(&out, bevy::math::Mat4::IDENTITY);
        m.finalize();
    }
}
