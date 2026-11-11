//! The painted material flattened into textures for the GPU viewport and
//! for texture export: base colour, occlusion / roughness / metallic, a
//! tangent-space normal map and emission. Updates are incremental: a
//! stroke re-composites only the tiles it touched, found by comparing
//! tile pointers with the stack seen last time.

use super::blend::tilt;
use super::{LayerKind, PaintStack};
use crate::materials::bake::uv_point;
use crate::materials::relief::base_tangent_normal;
use crate::materials::shading::{base_surface_at, linear_to_srgb, material_height};
use crate::materials::{Material, Texture};
use crate::paint::bake::GeoMap;
use crate::paint::texel::TILE;
use bevy::math::{Vec2, Vec3, Vec4};

/// What an update changed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Dirty {
    None,
    /// Tile indices (row-major, `size / TILE` per row).
    Tiles(Vec<usize>),
    All,
}

pub struct Composite {
    pub size: u32,
    /// sRGB base colour + alpha.
    pub color: Texture,
    /// R = cavity AO, G = roughness, B = metallic (linear).
    pub orm: Texture,
    /// Tangent-space normal map (+Y up the V axis).
    pub normal: Texture,
    /// sRGB emission divided by `peak`.
    pub emissive: Texture,
    pub peak: f32,
    seen: Option<(Material, u64)>,
}

fn byte(x: f32) -> u8 {
    (x.clamp(0.0, 1.0) * 255.0 + 0.5) as u8
}

fn without_paint(m: &Material) -> Material {
    let mut b = m.clone();
    b.paint = None;
    b
}

/// True when every layer setting that is not texel data matches.
fn same_settings(a: &PaintStack, b: &PaintStack) -> bool {
    a.size == b.size
        && a.layers.len() == b.layers.len()
        && a.layers.iter().zip(&b.layers).all(|(x, y)| {
            x.blend == y.blend
                && x.opacity == y.opacity
                && x.visible == y.visible
                && match (&x.kind, &y.kind) {
                    (LayerKind::Paint(p), LayerKind::Paint(q)) => p.size() == q.size(),
                    (LayerKind::Fill { source: s, mask: m }, LayerKind::Fill { source: t, mask: n }) => {
                        s == t && m.as_ref().map(|m| m.size()) == n.as_ref().map(|n| n.size())
                    }
                    _ => false,
                }
        })
}

/// Stack tiles covered by tile `t` of a layer image `layer_size` wide.
fn covered(t: usize, layer_size: u32, stack_size: u32, out: &mut Vec<usize>) {
    let (lp, sp) = (layer_size / TILE, stack_size / TILE);
    let (tx, ty) = (t as u32 % lp, t as u32 / lp);
    let span = |v: u32| ((v * sp) / lp, ((v + 1) * sp).div_ceil(lp).max((v * sp) / lp + 1));
    let ((x0, x1), (y0, y1)) = (span(tx), span(ty));
    for y in y0..y1.min(sp) {
        for x in x0..x1.min(sp) {
            out.push((y * sp + x) as usize);
        }
    }
}

/// Runs `f` over `items` on all cores, keeping the order.
fn par<T: Send>(items: &[usize], f: impl Fn(usize) -> T + Sync) -> Vec<T> {
    let threads = std::thread::available_parallelism().map_or(1, |t| t.get()).min(16);
    if threads <= 1 || items.len() < 2 {
        return items.iter().map(|&i| f(i)).collect();
    }
    let chunk = items.len().div_ceil(threads);
    let f = &f;
    std::thread::scope(|s| {
        let jobs: Vec<_> = items.chunks(chunk).map(|c| s.spawn(move || c.iter().map(|&i| f(i)).collect::<Vec<T>>())).collect();
        jobs.into_iter().flat_map(|j| j.join().expect("composite thread")).collect()
    })
}

type TileOut = Vec<[[u8; 4]; 4]>;

impl Composite {
    pub fn new(size: u32) -> Self {
        let t = |fill| Texture::new(size, size, fill);
        Self { size, color: t([0, 0, 0, 255]), orm: t([255, 128, 0, 255]), normal: t([128, 128, 255, 255]), emissive: t([0, 0, 0, 255]), peak: 1.0, seen: None }
    }

    /// The whole composite of `m` (which must have a paint stack).
    pub fn build(m: &Material, geo: Option<&GeoMap>) -> Composite {
        let mut c = Composite::new(m.paint.as_ref().map_or(TILE, |p| p.size));
        c.update(m, geo, 0);
        c
    }

    /// Brings the textures up to date with `m`. `geo` is the painted
    /// object's geometry map (needed by Multi/Sub-Object and Triplanar
    /// bases) and `geo_key` changes whenever it does.
    pub fn update(&mut self, m: &Material, geo: Option<&GeoMap>, geo_key: u64) -> Dirty {
        let Some(stack) = &m.paint else { return Dirty::None };
        let per_row = (stack.size / TILE) as usize;
        let all: Vec<usize> = (0..per_row * per_row).collect();
        let dirty = match &self.seen {
            Some((old, key)) if *key == geo_key && self.size == stack.size => {
                let old_stack = old.paint.as_ref().expect("seen materials are painted");
                if old_stack == stack {
                    return Dirty::None;
                }
                if !same_settings(old_stack, stack) || without_paint(old) != without_paint(m) {
                    None
                } else {
                    let mut tiles = vec![];
                    for (a, b) in stack.layers.iter().zip(&old_stack.layers) {
                        match (&a.kind, &b.kind) {
                            (LayerKind::Paint(p), LayerKind::Paint(q)) => p.changed_tiles(q).into_iter().for_each(|t| covered(t, p.size(), stack.size, &mut tiles)),
                            (LayerKind::Fill { mask: Some(p), .. }, LayerKind::Fill { mask: Some(q), .. }) => {
                                p.changed_tiles(q).into_iter().for_each(|t| covered(t, p.size(), stack.size, &mut tiles))
                            }
                            _ => {}
                        }
                    }
                    // Normals read one texel across tile borders.
                    let mut grown = tiles.clone();
                    for &t in &tiles {
                        let (x, y) = (t % per_row, t / per_row);
                        for (dx, dy) in [(1, 0), (per_row - 1, 0), (0, 1), (0, per_row - 1)] {
                            grown.push(((y + dy) % per_row) * per_row + (x + dx) % per_row);
                        }
                    }
                    grown.sort_unstable();
                    grown.dedup();
                    Some(grown)
                }
            }
            _ => None,
        };
        if self.size != stack.size {
            *self = Composite::new(stack.size);
        }
        self.peak = m.multi.as_ref().map_or(m.emissive_strength, |ms| ms.subs.iter().map(|s| s.mat.emissive_strength).fold(0.0, f32::max)).max(super::source::EMIT_GAIN);
        let list = dirty.as_deref().unwrap_or(&all);
        let peak = self.peak;
        let done = par(list, |t| Self::tile(m, stack, geo, t, peak));
        let n = stack.size as usize;
        for (&t, texels) in list.iter().zip(&done) {
            let (ox, oy) = ((t % per_row) * TILE as usize, (t / per_row) * TILE as usize);
            for (k, px) in texels.iter().enumerate() {
                let i = (oy + k / TILE as usize) * n + ox + k % TILE as usize;
                (self.color.data[i], self.orm.data[i], self.normal.data[i], self.emissive.data[i]) = (px[0], px[1], px[2], px[3]);
            }
        }
        self.seen = Some((m.clone(), geo_key));
        match dirty {
            Some(d) if d.is_empty() => Dirty::None,
            Some(d) => Dirty::Tiles(d),
            None => Dirty::All,
        }
    }

    /// Composites one tile: colour, ORM, normal and emission per texel.
    fn tile(m: &Material, stack: &PaintStack, geo: Option<&GeoMap>, t: usize, peak: f32) -> TileOut {
        let (n, per_row) = (stack.size as usize, (stack.size / TILE) as usize);
        let (ox, oy) = ((t % per_row) * TILE as usize, (t / per_row) * TILE as usize);
        let e = 1.0 / n as f32;
        (0..(TILE * TILE) as usize)
            .map(|k| {
                let (x, y) = (ox + k % TILE as usize, oy + k / TILE as usize);
                let i = y * n + x;
                let uv = Vec2::new((x as f32 + 0.5) * e, 1.0 - (y as f32 + 0.5) * e);
                let geo = geo.filter(|g| g.size == stack.size);
                let sub = match (&m.multi, geo) {
                    (Some(ms), Some(g)) => ms.resolve(g.ids[i].max(1)),
                    (Some(ms), None) => ms.resolve(1),
                    _ => m,
                };
                let space = geo.and_then(|g| g.space(i)).filter(|_| sub.projection.on());
                let (p, mut s, base_n) = match space {
                    Some((tp, tn)) => (tp, base_surface_at(sub, uv, tp, tn, Vec4::ONE), None),
                    None => {
                        let p = uv_point(uv);
                        (p, crate::materials::shading::base_surface(sub, uv, p, Vec4::ONE), base_tangent_normal(sub, uv, p, Vec3::X * 2.0, Vec3::Y * 2.0))
                    }
                };
                let relief = stack.apply(uv, p, &mut s, || material_height(sub, uv, p).unwrap_or(0.5));
                let h = |du: f32, dv: f32| stack.relief(uv + Vec2::new(du, dv), p).height;
                let (dx, dy) = ((h(e, 0.0) - h(-e, 0.0)) / (2.0 * e), (h(0.0, e) - h(0.0, -e)) / (2.0 * e));
                let tn = tilt(base_n, relief.cover, dx, dy).unwrap_or(Vec3::Z);
                let srgb = |c: Vec3, a: f32| [byte(linear_to_srgb(c.x)), byte(linear_to_srgb(c.y)), byte(linear_to_srgb(c.z)), byte(a)];
                [
                    srgb(s.base, s.alpha),
                    [byte(s.ao), byte(s.roughness), byte(s.metallic), 255],
                    [byte(tn.x * 0.5 + 0.5), byte(tn.y * 0.5 + 0.5), byte(tn.z * 0.5 + 0.5), 255],
                    srgb(s.emissive / peak, 1.0),
                ]
            })
            .collect()
    }

    /// The four textures with their export names.
    pub fn images(&self) -> [(&'static str, &Texture); 4] {
        [("basecolor", &self.color), ("orm", &self.orm), ("normal", &self.normal), ("emissive", &self.emissive)]
    }
}
