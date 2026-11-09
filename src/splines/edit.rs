//! Editable Spline operations: Vertex (type, Fillet, Chamfer, Weld, Refine,
//! Make First, Delete), Segment (Divide, Delete) and Spline (Outline,
//! Mirror, Reverse, Close, Delete), plus sub-object selection helpers.

use super::{Knot, KnotType, Shape, Spline};
use bevy::math::{Mat4, Vec3};

/// Spline sub-object level (keys 1 / 2 / 3).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SplineLevel {
    Vertex,
    Segment,
    Spline,
}

impl Shape {
    /// Indices `(spline, knot)` of selected knots (by level).
    pub fn selected_knots(&self, level: SplineLevel) -> Vec<(usize, usize)> {
        let mut v = vec![];
        for (si, s) in self.splines.iter().enumerate() {
            let n = s.knots.len();
            for ki in 0..n {
                let on = match level {
                    SplineLevel::Vertex => s.knots[ki].selected,
                    SplineLevel::Segment => {
                        s.knots[ki].seg_selected && ki < s.segment_count()
                            || ki > 0 && s.knots[ki - 1].seg_selected
                            || s.closed && ki == 0 && s.knots[n - 1].seg_selected
                    }
                    SplineLevel::Spline => s.selected,
                };
                if on {
                    v.push((si, ki));
                }
            }
        }
        v
    }

    pub fn transform_selected(&mut self, level: SplineLevel, m: Mat4) {
        for (si, ki) in self.selected_knots(level) {
            self.splines[si].knots[ki].transform(m);
        }
    }

    pub fn clear_selection(&mut self) {
        for s in &mut self.splines {
            s.selected = false;
            for k in &mut s.knots {
                k.selected = false;
                k.seg_selected = false;
            }
        }
    }

    pub fn set_type_selected(&mut self, kind: KnotType) {
        for (si, ki) in self.selected_knots(SplineLevel::Vertex) {
            self.splines[si].set_type(ki, kind);
        }
    }

    /// Fillet / Chamfer every selected vertex.
    pub fn chamfer_selected(&mut self, dist: f32, fillet: bool) -> usize {
        let mut count = 0;
        for s in &mut self.splines {
            let sel: Vec<usize> = (0..s.knots.len()).filter(|&i| s.knots[i].selected).collect();
            for &i in sel.iter().rev() {
                count += s.chamfer_knot(i, dist, fillet) as usize;
            }
        }
        count
    }

    /// Delete at the given level. Deleting segments splits splines.
    pub fn delete_selected(&mut self, level: SplineLevel) {
        let mut out = vec![];
        for s in std::mem::take(&mut self.splines) {
            match level {
                SplineLevel::Spline if s.selected => {}
                SplineLevel::Vertex => {
                    let mut s = s;
                    s.knots.retain(|k| !k.selected);
                    if s.knots.len() >= 2 {
                        s.closed &= s.knots.len() >= 3;
                        out.push(s);
                    }
                }
                SplineLevel::Segment if s.knots.iter().take(s.segment_count()).any(|k| k.seg_selected) => {
                    out.extend(split_at_segments(&s));
                }
                _ => out.push(s),
            }
        }
        self.splines = out;
    }

    /// Divide: inserts `n` evenly spaced knots on every selected segment.
    pub fn divide_selected(&mut self, n: u32) {
        for s in &mut self.splines {
            let mut i = s.segment_count();
            while i > 0 {
                i -= 1;
                if s.knots[i].seg_selected {
                    for k in (1..=n).rev() {
                        let idx = s.refine(i, k as f32 / (k + 1) as f32);
                        s.knots[idx].seg_selected = true;
                    }
                }
            }
        }
    }

    /// Weld: joins selected open ends closer than `threshold` (closing a
    /// spline onto itself, or merging two splines).
    pub fn weld_selected(&mut self, threshold: f32) -> usize {
        let mut welds = 0;
        'outer: loop {
            let ends: Vec<(usize, bool, Vec3)> = (0..self.splines.len())
                .filter(|&i| !self.splines[i].closed && self.splines[i].knots.len() >= 2)
                .flat_map(|i| {
                    let s = &self.splines[i];
                    let (f, l) = (&s.knots[0], s.knots.last().unwrap());
                    [(f.selected).then_some((i, false, f.pos)), (l.selected).then_some((i, true, l.pos))]
                })
                .flatten()
                .collect();
            for a in 0..ends.len() {
                for b in a + 1..ends.len() {
                    let ((si, sl, sp), (ti, tl, tp)) = (ends[a], ends[b]);
                    if sp.distance(tp) > threshold {
                        continue;
                    }
                    if si == ti {
                        let s = &mut self.splines[si];
                        let last = s.knots.pop().unwrap();
                        s.knots[0].in_h = last.in_h;
                        s.closed = true;
                    } else {
                        let mut t = self.splines[ti].clone();
                        let mut s = self.splines[si].clone();
                        if !sl {
                            s.reverse();
                        }
                        if tl {
                            t.reverse();
                        }
                        let first = t.knots.remove(0);
                        s.knots.last_mut().unwrap().out_h = first.out_h;
                        s.knots.extend(t.knots);
                        self.splines[si] = s;
                        self.splines.remove(ti);
                    }
                    welds += 1;
                    continue 'outer;
                }
            }
            break;
        }
        welds
    }

    /// Outline: adds an offset copy of each selected spline (open splines
    /// become one closed outline).
    pub fn outline_selected(&mut self, amount: f32) {
        let mut added = vec![];
        for s in self.splines.iter_mut().filter(|s| s.selected) {
            let o = s.offset(amount);
            if s.closed {
                added.push(o);
            } else {
                let mut back = o.clone();
                back.reverse();
                s.knots.extend(back.knots);
                s.closed = true;
            }
        }
        self.splines.extend(added);
    }

    /// Mirror selected splines about their own center (axis 0 = X, 1 = Y, 2 = both).
    pub fn mirror_selected(&mut self, axis: usize, copy: bool) {
        let mut added = vec![];
        for s in self.splines.iter_mut().filter(|s| s.selected) {
            let Some(b) = crate::mesh::Aabb::from_points(s.knots.iter().map(|k| k.pos)) else { continue };
            let c = b.center();
            let sc = match axis {
                0 => Vec3::new(-1.0, 1.0, 1.0),
                1 => Vec3::new(1.0, -1.0, 1.0),
                _ => Vec3::new(-1.0, -1.0, 1.0),
            };
            let m = Mat4::from_translation(c) * Mat4::from_scale(sc) * Mat4::from_translation(-c);
            let mut t = s.clone();
            t.knots.iter_mut().for_each(|k| k.transform(m));
            if axis < 2 {
                t.reverse();
            }
            if copy { added.push(t) } else { *s = t }
        }
        self.splines.extend(added);
    }
}

fn split_at_segments(s: &Spline) -> Vec<Spline> {
    let n = s.knots.len();
    let plain = |i: usize| Knot { seg_selected: false, ..s.knots[i % n].clone() };
    let (start, segs): (usize, Vec<usize>) = if s.closed {
        let Some(s0) = (0..n).find(|&i| s.knots[i].seg_selected) else { return vec![s.clone()] };
        (s0 + 1, (1..n).map(|k| (s0 + k) % n).collect())
    } else {
        (0, (0..n - 1).collect())
    };
    let mut out = vec![];
    let mut cur = vec![plain(start)];
    for j in segs {
        if s.knots[j].seg_selected {
            out.push(std::mem::take(&mut cur));
            cur = vec![plain(j + 1)];
        } else {
            cur.push(plain(j + 1));
        }
    }
    out.push(cur);
    out.into_iter().filter(|k| k.len() >= 2).map(|knots| Spline { knots, closed: false, selected: false }).collect()
}
