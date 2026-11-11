//! Strokes on a paint layer (or on a fill layer's mask). Every stamp is
//! projected onto the surface along the brush's tangent plane and
//! evaluated per texel in world space, so strokes cross UV seams cleanly,
//! skip faces that look away, and write colour, roughness, metallic,
//! height and emission together.

use super::alphas::spin;
use crate::paint::layers::{LayerKind, PaintLayer, Source};
use crate::paint::raster::{PaintTri, px, raster_tri};
use crate::paint::texel::{Sample, Texel, Tiles};
use crate::paint::{Brush, BrushTool};
use bevy::math::{Mat2, Quat, Vec2, Vec3};

/// Where and how one stamp lands.
#[derive(Clone, Copy, Debug)]
pub struct Stamp {
    /// World-space centre on the surface.
    pub center: Vec3,
    /// Surface normal there (`ZERO` floods: no plane, no facing test).
    pub normal: Vec3,
    /// Stroke direction (any vector off the normal).
    pub tangent: Vec3,
    /// Faces looking away from this direction are skipped (`ZERO` = paint
    /// both sides).
    pub view_dir: Vec3,
}

pub struct LayerStroke {
    tris: Vec<PaintTri>,
    /// The layer when the stroke (or the current stamp) began.
    pre: LayerKind,
    /// Strongest coverage each texel got in this stroke, so going back and
    /// forth never builds past the brush opacity.
    cov: Vec<f32>,
    size: u32,
    seed: u32,
    count: u32,
}

fn byte(x: f32) -> u8 {
    (x.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

/// `smp` over `pre` at strength `w`, channel by channel.
fn over(pre: Texel, smp: &Sample, w: f32) -> Texel {
    let mut out = pre;
    let wa = smp.a * w;
    if wa > 0.0 {
        let pa = pre.rgba[3] as f32 / 255.0;
        let a = wa + pa * (1.0 - wa);
        for k in 0..3 {
            out.rgba[k] = byte((smp.color[k] * wa + pre.rgba[k] as f32 / 255.0 * pa * (1.0 - wa)) / a);
        }
        out.rgba[3] = byte(a);
    }
    for k in 0..4 {
        let wk = smp.cov[k] * w;
        if wk > 0.0 {
            let pc = pre.cov[k] as f32 / 255.0;
            let c = wk + pc * (1.0 - wk);
            out.pbr[k] = byte((smp.pbr[k] * wk + pre.pbr[k] as f32 / 255.0 * pc * (1.0 - wk)) / c);
            out.cov[k] = byte(c);
        }
    }
    out
}

fn erase(pre: Texel, w: f32) -> Texel {
    let k = 1.0 - w;
    let f = |c: u8| (c as f32 * k + 0.5) as u8;
    let out = Texel { rgba: [pre.rgba[0], pre.rgba[1], pre.rgba[2], f(pre.rgba[3])], pbr: pre.pbr, cov: pre.cov.map(f) };
    if out.rgba[3] == 0 && out.cov == [0; 4] { Texel::default() } else { out }
}

/// Coverage-weighted mean of the 3 x 3 texels around (x, y).
fn blurred(t: &Tiles<Texel>, x: i64, y: i64) -> Texel {
    let (mut rgb, mut a, mut pbr, mut cov) = ([0.0f32; 3], 0.0f32, [0.0f32; 4], [0.0f32; 4]);
    for dy in -1..=1 {
        for dx in -1..=1 {
            let s = t.get(x + dx, y + dy);
            let sa = s.rgba[3] as f32;
            (0..3).for_each(|k| rgb[k] += s.rgba[k] as f32 * sa);
            a += sa;
            for k in 0..4 {
                pbr[k] += s.pbr[k] as f32 * s.cov[k] as f32;
                cov[k] += s.cov[k] as f32;
            }
        }
    }
    let mut out = Texel::default();
    if a > 0.0 {
        (0..3).for_each(|k| out.rgba[k] = (rgb[k] / a + 0.5) as u8);
        out.rgba[3] = (a / 9.0 + 0.5) as u8;
    }
    for k in 0..4 {
        if cov[k] > 0.0 {
            out.pbr[k] = (pbr[k] / cov[k] + 0.5) as u8;
            out.cov[k] = (cov[k] / 9.0 + 0.5) as u8;
        }
    }
    out
}

fn mix_texel(a: Texel, b: Texel, w: f32) -> Texel {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * w + 0.5) as u8;
    // Unpainted texels have no colour of their own to blend from.
    let from = |x: [u8; 4], y: [u8; 4], xa: u8| if xa == 0 { y } else { x };
    let (ac, ap) = (from(a.rgba, b.rgba, a.rgba[3]), a.pbr);
    Texel {
        rgba: [m(ac[0], b.rgba[0]), m(ac[1], b.rgba[1]), m(ac[2], b.rgba[2]), m(a.rgba[3], b.rgba[3])],
        pbr: std::array::from_fn(|k| if a.cov[k] == 0 { b.pbr[k] } else { m(ap[k], b.pbr[k]) }),
        cov: std::array::from_fn(|k| m(a.cov[k], b.cov[k])),
    }
}

impl LayerStroke {
    /// Starts a stroke on `layer`. Erasing a fill layer that has no mask
    /// yet gives it a full one first.
    pub fn new(tris: Vec<PaintTri>, layer: &mut PaintLayer, stack_size: u32, seed: u32) -> Self {
        if let LayerKind::Fill { mask, .. } = &mut layer.kind
            && mask.is_none()
        {
            let mut m = Tiles::new(stack_size);
            m.fill_with(|_, _| 255u8);
            *mask = Some(m);
        }
        let size = match &layer.kind {
            LayerKind::Paint(t) => t.size(),
            LayerKind::Fill { mask, .. } => mask.as_ref().map_or(stack_size, |m| m.size()),
        };
        Self { tris, pre: layer.kind.clone(), cov: vec![0.0; (size * size) as usize], size, seed, count: 0 }
    }

    /// One stamp. Returns the number of texels it reached.
    pub fn stamp(&mut self, layer: &mut PaintLayer, at: &Stamp, brush: &Brush, source: &Source) -> usize {
        // Decal stamps are committed one by one, so they can overlap.
        if brush.alpha.is_stamp() && self.count > 0 {
            self.pre = layer.kind.clone();
            self.cov.iter_mut().for_each(|c| *c = 0.0);
        }
        self.count += 1;
        let (size, r) = (self.size, brush.size.max(1e-6));
        let flood = at.normal == Vec3::ZERO;
        let n = at.normal.normalize_or(Vec3::Z);
        let mut t = (at.tangent - n * n.dot(at.tangent)).normalize_or(n.any_orthonormal_vector());
        if brush.alpha.spins() {
            t = Quat::from_axis_angle(n, spin(self.count, self.seed)) * t;
        }
        let b = n.cross(t);
        let seed = self.seed.wrapping_add(self.count.wrapping_mul(7919));
        let opacity = brush.opacity.clamp(0.0, 1.0);
        let mut writes: Vec<(usize, f32, Vec3, Option<crate::paint::layers::Flat>)> = vec![];
        for tri in &self.tris {
            if !flood && ((at.view_dir != Vec3::ZERO && tri.normal.dot(at.view_dir) > 0.0) || tri.normal.dot(n) <= 0.0) {
                continue;
            }
            let (lo, hi) = (tri.p[0].min(tri.p[1]).min(tri.p[2]), tri.p[0].max(tri.p[1]).max(tri.p[2]));
            if at.center.clamp(lo, hi).distance(at.center) > r * 1.5 {
                continue;
            }
            // Affine world -> pixel map on the triangle's plane: the pixel
            // box the brush can reach.
            let [pa, pb, pc] = tri.uv.map(|u| px(size, u));
            let (e1, e2) = (tri.p[1] - tri.p[0], tri.p[2] - tri.p[0]);
            let g = Mat2::from_cols_array(&[e1.dot(e1), e1.dot(e2), e1.dot(e2), e2.dot(e2)]);
            if g.determinant().abs() < 1e-12 {
                continue;
            }
            let gi = g.inverse();
            let (d1, d2) = (pb - pa, pc - pa);
            let rel = at.center - tri.p[0];
            let st = gi * Vec2::new(e1.dot(rel), e2.dot(rel));
            let cpx = pa + d1 * st.x + d2 * st.y;
            let (kx, ky) = (gi * Vec2::new(d1.x, d2.x), gi * Vec2::new(d1.y, d2.y));
            let ext = Vec2::new((e1 * kx.x + e2 * kx.y).length(), (e1 * ky.x + e2 * ky.y).length()) * r * 1.5 + Vec2::ONE;
            raster_tri(tri, size, Some((cpx - ext, cpx + ext)), |i, w, _| {
                let p = tri.p[0] * w[0] + tri.p[1] * w[1] + tri.p[2] * w[2];
                let d = p - at.center;
                // Tangent projection: only the surface near the brush plane.
                if !flood && d.dot(n).abs() > r {
                    return;
                }
                let q = if flood { Vec2::ZERO } else { Vec2::new(d.dot(t), d.dot(b)) / r };
                let dab = brush.alpha.eval(q, brush.falloff, seed);
                let wt = dab.cov * opacity;
                if wt > 0.0 {
                    writes.push((i, wt, p, dab.look));
                }
            });
        }
        let n_px = size as usize;
        let at_px = |i: usize| ((i % n_px) as i64, (i / n_px) as i64);
        let uv_of = |i: usize| Vec2::new(((i % n_px) as f32 + 0.5) / size as f32, 1.0 - ((i / n_px) as f32 + 0.5) / size as f32);
        match (&mut layer.kind, &self.pre) {
            (LayerKind::Paint(tiles), LayerKind::Paint(pre)) => match brush.tool {
                BrushTool::Paint | BrushTool::Erase => {
                    for (i, wt, p, look) in &writes {
                        if *wt <= self.cov[*i] {
                            continue;
                        }
                        self.cov[*i] = *wt;
                        let (x, y) = at_px(*i);
                        let before = pre.get(x, y);
                        let texel = if brush.tool == BrushTool::Erase {
                            erase(before, *wt)
                        } else {
                            let smp = look.as_ref().map_or_else(|| source.sample(uv_of(*i), *p), |f| f.sample());
                            over(before, &smp, *wt)
                        };
                        tiles.set(x, y, texel);
                    }
                }
                BrushTool::Blur => {
                    let soft: Vec<(i64, i64, Texel)> = writes
                        .iter()
                        .map(|(i, wt, ..)| {
                            let (x, y) = at_px(*i);
                            (x, y, mix_texel(tiles.get(x, y), blurred(tiles, x, y), wt * 0.5))
                        })
                        .collect();
                    soft.into_iter().for_each(|(x, y, t)| tiles.set(x, y, t));
                }
            },
            (LayerKind::Fill { mask: Some(mask), .. }, LayerKind::Fill { mask: Some(pre), .. }) => {
                for (i, wt, ..) in &writes {
                    if *wt <= self.cov[*i] {
                        continue;
                    }
                    self.cov[*i] = *wt;
                    let (x, y) = at_px(*i);
                    let before = pre.get(x, y) as f32;
                    let v = match brush.tool {
                        BrushTool::Erase => before * (1.0 - wt),
                        _ => before + (255.0 - before) * wt,
                    };
                    mask.set(x, y, (v + 0.5) as u8);
                }
            }
            _ => {}
        }
        writes.len()
    }
}
