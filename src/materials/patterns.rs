//! Procedural pattern maps: bricks / tiles / planks / cobbles, stripes
//! (road lines, corrugated metal), cracks / Voronoi (flagstones, cracked
//! earth) and grunge / wear (dirt, rust, scratches, stains).

use bevy::math::{Vec2, Vec3};
use noise::{NoiseFn, Perlin};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum BrickKind {
    /// Running bond: every other row shifted by half a brick.
    #[default]
    Running,
    /// Stack bond / square tiles.
    Stack,
    /// Long boards with random joints.
    Planks,
    /// Rounded stones in offset rows.
    Cobbles,
}

impl BrickKind {
    pub const ALL: [BrickKind; 4] = [BrickKind::Running, BrickKind::Stack, BrickKind::Planks, BrickKind::Cobbles];
    pub fn label(self) -> &'static str {
        match self {
            BrickKind::Running => "Running Bond",
            BrickKind::Stack => "Stack / Tiles",
            BrickKind::Planks => "Planks",
            BrickKind::Cobbles => "Cobbles",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum GrungeKind {
    #[default]
    Dirt,
    Rust,
    Scratches,
    Stains,
}

impl GrungeKind {
    pub const ALL: [GrungeKind; 4] = [GrungeKind::Dirt, GrungeKind::Rust, GrungeKind::Scratches, GrungeKind::Stains];
    pub fn label(self) -> &'static str {
        match self {
            GrungeKind::Dirt => "Dirt",
            GrungeKind::Rust => "Rust",
            GrungeKind::Scratches => "Scratches",
            GrungeKind::Stains => "Stains",
        }
    }
}

/// Integer hash to 0..1.
pub fn hash(x: i32, y: i32, z: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x8da6_b343) ^ (y as u32).wrapping_mul(0xd816_3841) ^ (z as u32).wrapping_mul(0xcb1a_b31f);
    h ^= seed.wrapping_mul(0x2c1b_3c6d);
    h = (h ^ (h >> 15)).wrapping_mul(0x2c1b_3c6d);
    h = (h ^ (h >> 12)).wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    (h & 0x00ff_ffff) as f32 / 16_777_215.0
}

fn perlin() -> &'static Perlin {
    static P: OnceLock<Perlin> = OnceLock::new();
    P.get_or_init(|| Perlin::new(29))
}

/// 4-octave fractal noise, about 0..1.
pub fn fbm(p: Vec3) -> f32 {
    let (mut sum, mut amp, mut q) = (0.0, 0.5, p);
    for _ in 0..4 {
        sum += perlin().get(q.as_dvec3().to_array()) as f32 * amp;
        q = q * 2.03 + Vec3::splat(5.1);
        amp *= 0.5;
    }
    (sum + 0.5).clamp(0.0, 1.0)
}

fn smooth(a: f32, b: f32, x: f32) -> f32 {
    let t = ((x - a) / (b - a)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn varied(c: Vec3, amount: f32, h: f32) -> Vec3 {
    (c * (1.0 + (h - 0.5) * amount)).clamp(Vec3::ZERO, Vec3::ONE)
}

/// Bricks / tiles: `cols` × `rows` units across the UV square; `gap` is the
/// mortar width in row heights, `vary` the per-brick colour variation.
#[allow(clippy::too_many_arguments)]
pub fn bricks(kind: BrickKind, cols: f32, rows: f32, gap: f32, vary: f32, seed: u32, brick: Vec3, mortar: Vec3, uv: Vec2) -> Vec3 {
    let (cols, rows) = (cols.max(0.5), rows.max(0.5));
    let q = uv * Vec2::new(cols, rows);
    let row = q.y.floor();
    let shift = match kind {
        BrickKind::Running | BrickKind::Cobbles => (row.rem_euclid(2.0)) * 0.5,
        BrickKind::Stack => 0.0,
        BrickKind::Planks => hash(row as i32, 0, 1, seed),
    };
    let x = q.x + shift;
    let cell = (x.floor() as i32, row as i32);
    let f = Vec2::new(x.fract(), q.y - row);
    // Distances to the cell edges in row heights.
    let aspect = rows / cols;
    let dx = f.x.min(1.0 - f.x) / aspect;
    let dy = f.y.min(1.0 - f.y);
    let d = if kind == BrickKind::Cobbles {
        // Rounded-rectangle distance with a little wobble.
        let r = 0.3;
        let cx = (r - dx).max(0.0);
        let cy = (r - dy).max(0.0);
        r - (cx * cx + cy * cy).sqrt() - fbm(Vec3::new(q.x * 3.0, q.y * 3.0, seed as f32)) * 0.12
    } else {
        dx.min(dy)
    };
    let t = smooth(gap * 0.5, gap * 0.5 + 0.04, d);
    let h = hash(cell.0, cell.1, 0, seed);
    let grain = fbm(Vec3::new(q.x * 6.0, q.y * 6.0, h * 9.0)) - 0.5;
    let b = varied(brick, vary, h) * (1.0 + grain * 0.25);
    mortar.lerp(b.clamp(Vec3::ZERO, Vec3::ONE), t)
}

/// Stripes: `count` lines across the UV square, `width` as a fraction of
/// the spacing; `dash` > 0 breaks them into dashes; `soft` gives a
/// corrugated sine profile.
#[allow(clippy::too_many_arguments)]
pub fn stripes(count: f32, width: f32, dash: f32, soft: bool, along_u: bool, line: Vec3, ground: Vec3, uv: Vec2) -> Vec3 {
    let (across, along) = if along_u { (uv.y, uv.x) } else { (uv.x, uv.y) };
    let f = (across * count.max(0.01)).fract();
    if soft {
        let t = 0.5 + 0.5 * (f * std::f32::consts::TAU).cos();
        return ground.lerp(line, t);
    }
    let w = width.clamp(0.0, 1.0) * 0.5;
    let edge = (f - 0.5).abs();
    let mut t = 1.0 - smooth(w - 0.01, w + 0.01, edge);
    if dash > 0.0 && (along * dash).fract() > 0.5 {
        t = 0.0;
    }
    ground.lerp(line, t)
}

/// Distances to the nearest and second-nearest Voronoi points, and the
/// nearest cell's hash.
pub fn voronoi(p: Vec3, seed: u32) -> (f32, f32, f32) {
    let c = p.floor();
    let (mut f1, mut f2, mut id) = (9.0f32, 9.0f32, 0.0);
    for z in -1..=1 {
        for y in -1..=1 {
            for x in -1..=1 {
                let (ix, iy, iz) = (c.x as i32 + x, c.y as i32 + y, c.z as i32 + z);
                let o = Vec3::new(hash(ix, iy, iz, seed), hash(ix, iy, iz, seed + 1), hash(ix, iy, iz, seed + 2));
                let d = (Vec3::new(ix as f32, iy as f32, iz as f32) + o - p).length();
                if d < f1 {
                    (f2, f1, id) = (f1, d, hash(ix, iy, iz, seed + 3));
                } else if d < f2 {
                    f2 = d;
                }
            }
        }
    }
    (f1, f2, id)
}

/// Cracks between Voronoi cells (cracked earth, flagstones, crazing).
#[allow(clippy::too_many_arguments)]
pub fn cracks(scale: f32, width: f32, vary: f32, seed: u32, surface: Vec3, crack: Vec3, p: Vec3) -> Vec3 {
    let q = p * scale.max(0.01);
    let warp = Vec3::new(fbm(q * 0.7), fbm(q * 0.7 + Vec3::splat(3.3)), 0.0) * 0.6;
    let (f1, f2, id) = voronoi(q + warp, seed);
    let t = smooth(width * 0.5, width * 0.5 + 0.05, f2 - f1);
    let s = varied(surface, vary, id) * (0.9 + fbm(q * 4.0) * 0.2);
    crack.lerp(s.clamp(Vec3::ZERO, Vec3::ONE), t)
}

/// Grunge / wear layer: `amount` 0..1 is how much of the surface is dirty.
pub fn grunge(kind: GrungeKind, scale: f32, amount: f32, seed: u32, clean: Vec3, dirt: Vec3, p: Vec3) -> Vec3 {
    let q = p * scale.max(0.01) + Vec3::splat(seed as f32 * 7.31);
    let a = amount.clamp(0.0, 1.0);
    let t = match kind {
        GrungeKind::Dirt => smooth(0.75 - a * 0.6, 0.85 - a * 0.6, fbm(q)),
        GrungeKind::Rust => {
            let (f1, _, _) = voronoi(q * 2.0, seed);
            let n = fbm(q) * 0.7 + (1.0 - f1.min(1.0)) * 0.3;
            smooth(0.72 - a * 0.55, 0.8 - a * 0.55, n) * (0.75 + fbm(q * 8.0) * 0.25)
        }
        GrungeKind::Scratches => {
            let mut s = 0.0f32;
            for (i, dir) in [Vec3::new(1.0, 0.3, 0.1), Vec3::new(-0.4, 1.0, 0.2), Vec3::new(0.6, -0.7, 0.3)].into_iter().enumerate() {
                let d = dir.normalize();
                let along = q.dot(d);
                let across = (q - d * along) * 40.0;
                let line = perlin().get((across + Vec3::splat(i as f32 * 11.0)).as_dvec3().to_array()) as f32;
                let mask = fbm(q * 0.8 + Vec3::splat(i as f32 * 3.0));
                s = s.max((1.0 - line.abs() * 12.0).max(0.0) * smooth(0.65 - a * 0.4, 0.75 - a * 0.4, mask));
            }
            s
        }
        GrungeKind::Stains => {
            let n = fbm(q * 0.5);
            let ring = 1.0 - ((n - (0.7 - a * 0.4)).abs() * 14.0).min(1.0);
            ring * 0.7 + smooth(0.72 - a * 0.4, 0.9 - a * 0.4, n) * 0.4
        }
    };
    clean.lerp(dirt, t.clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in01(c: Vec3) -> bool {
        c.is_finite() && c.min_element() >= 0.0 && c.max_element() <= 1.0 + 1e-4
    }

    #[test]
    fn bricks_have_mortar_lines_and_brick_faces() {
        let (b, m) = (Vec3::new(0.7, 0.3, 0.2), Vec3::splat(0.8));
        for k in BrickKind::ALL {
            let mut saw = (false, false);
            for i in 0..400 {
                let uv = Vec2::new((i % 20) as f32 / 20.0 + 0.013, (i / 20) as f32 / 20.0 + 0.007);
                let c = bricks(k, 4.0, 8.0, 0.12, 0.3, 1, b, m, uv);
                assert!(in01(c), "{k:?} {c}");
                saw.0 |= c.distance(m) < 0.02;
                saw.1 |= c.distance(m) > 0.2;
            }
            assert!(saw.0 && saw.1, "{k:?} draws both mortar and bricks");
        }
    }

    #[test]
    fn stripes_dash_and_corrugate() {
        let (l, g) = (Vec3::ONE, Vec3::ZERO);
        assert_eq!(stripes(1.0, 0.1, 0.0, false, false, l, g, Vec2::new(0.5, 0.2)), l, "centre line");
        assert_eq!(stripes(1.0, 0.1, 0.0, false, false, l, g, Vec2::new(0.2, 0.2)), g);
        assert_eq!(stripes(1.0, 0.1, 4.0, false, false, l, g, Vec2::new(0.5, 0.1)), l, "dash on");
        assert_eq!(stripes(1.0, 0.1, 4.0, false, false, l, g, Vec2::new(0.5, 0.2)), g, "gap between dashes");
        let s = stripes(10.0, 0.5, 0.0, true, true, l, g, Vec2::new(0.3, 0.025));
        assert!(s.x > 0.0 && s.x < 1.0, "corrugated is smooth: {s}");
    }

    #[test]
    fn voronoi_cracks_and_grunge_stay_in_range() {
        let (a, b) = (Vec3::new(0.6, 0.5, 0.4), Vec3::splat(0.05));
        let mut cracked = 0;
        for i in 0..300 {
            let p = Vec3::new(i as f32 * 0.037, (i * 7 % 13) as f32 * 0.11, 0.3);
            let (f1, f2, id) = voronoi(p * 3.0, 4);
            assert!(f1 <= f2 && (0.0..=1.0).contains(&id));
            let c = cracks(3.0, 0.08, 0.2, 4, a, b, p);
            assert!(in01(c));
            cracked += (c.distance(b) < 0.05) as usize;
            for k in GrungeKind::ALL {
                assert!(in01(grunge(k, 2.0, 0.5, 3, a, b, p)), "{k:?}");
            }
        }
        assert!(cracked > 3 && cracked < 200, "cracks are thin lines: {cracked}");
        // More amount means more dirt.
        let dirt = |amt| (0..400).filter(|&i| grunge(GrungeKind::Dirt, 2.0, amt, 1, a, b, Vec3::new(i as f32 * 0.05, 0.1, 0.2)).distance(b) < 0.2).count();
        assert!(dirt(0.9) > dirt(0.1));
    }
}
