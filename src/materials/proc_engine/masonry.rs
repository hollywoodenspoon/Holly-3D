//! Masonry generators: bricks and tiles (running and stack bond),
//! herringbone, and coursed rows with random joints (planks, medieval
//! rubble). Each returns the bevelled height and a random id per tile.
//!
//! Ported from MaterialCraft (https://github.com/storytold/materialcraft:
//! `bricks` in `patterns.rs`, `herringbone` and `planks` in
//! `patterns_more.rs`, before the old procedural library was removed),
//! copyright (c) 2026 ArtCraft Team and the MaterialCraft contributors,
//! MIT OR Apache-2.0.

use super::generators::Px;
use super::lattice::rand2;

const GAP: Px = Px { h: 0.0, id: 0.0 };

/// A bevelled tile from the distance `e` to its edge (in tile heights):
/// `gap` eats the edge, `bevel` ramps up to the face, whose height varies a
/// little per tile.
#[inline]
fn tile(e: f32, gap: f32, bevel: f32, id: f32) -> Px {
    let e = e - gap * 0.5;
    if e <= 0.0 { GAP } else { Px { h: (e / bevel.max(1e-4)).min(1.0) * (1.0 - 0.2 * id), id } }
}

/// Bricks: `bx` × `by` across the tile. `running` shifts every other row
/// by half a brick (an odd row count would break the V seam, so it stacks).
#[allow(clippy::too_many_arguments)]
pub fn bricks(u: f32, v: f32, bx: f32, by: f32, gap: f32, bevel: f32, running: bool, seed: u32) -> Px {
    let row = (v * by).floor();
    let off = if running && by as i32 % 2 == 0 { 0.5 } else { 0.0 };
    let x = u * bx + off * row;
    let col = x.floor();
    let (fx, fy) = (x - col, v * by - row);
    let (ci, ri) = ((col as i32).rem_euclid(bx as i32), (row as i32).rem_euclid(by as i32));
    // Distance to the brick's edge, in brick-height units.
    let e = (fx.min(1.0 - fx) * by / bx).min(fy.min(1.0 - fy));
    tile(e, gap, bevel * 0.5, rand2(ci, ri, seed))
}

/// Herringbone of 1 × `len` planks, `t` repeats across the tile.
pub fn herringbone(u: f32, v: f32, t: i32, len: i32, gap: f32, bevel: f32, seed: u32) -> Px {
    let lf = len as f32;
    let span = (t * 2 * len) as f32;
    // 1 × L planks: a horizontal plank [0, L)×[0, 1) and a vertical one [L, L+1)×[1-L, 1)
    // repeat on the lattice m·(1, 1) + n·(L, -L), which has period 2L on both axes.
    let (x, y) = (u * span, v * span);
    let (m0, n0) = (((x + y) * 0.5).floor() as i32, ((x - y) / (2.0 * lf)).floor() as i32);
    for dm in -(len / 2 + 2)..=1 {
        for dn in -2..=1 {
            let (m, nn) = (m0 + dm, n0 + dn);
            let (qx, qy) = (x - (m + nn * len) as f32, y - (m - nn * len) as f32);
            let hit = if (0.0..lf).contains(&qx) && (0.0..1.0).contains(&qy) {
                Some((qx.min(lf - qx).min(qy).min(1.0 - qy), 0))
            } else if (lf..lf + 1.0).contains(&qx) && (1.0 - lf..1.0).contains(&qy) {
                Some(((qx - lf).min(lf + 1.0 - qx).min(qy - 1.0 + lf).min(1.0 - qy), 1))
            } else {
                None
            };
            if let Some((e, kind)) = hit {
                let k = nn.div_euclid(t);
                let (cm, cn) = ((m - k * len * t).rem_euclid(2 * len * t), nn - k * t);
                return tile(e, gap, bevel, rand2(cm * 2 + kind, cn, seed));
            }
        }
    }
    GAP
}

/// Rows of tiles along U whose end joints are jittered by `jitter` (0..1):
/// boards at 0.4, medieval rubble courses at 0.8. Each row starts at a
/// random place, so joints never line up.
#[allow(clippy::too_many_arguments)]
pub fn coursed(u: f32, v: f32, count: f32, rows: f32, gap: f32, bevel: f32, jitter: f32, seed: u32) -> Px {
    let (ci, ri) = (count as i32, rows as i32);
    let row = ((v * rows).floor() as i32).rem_euclid(ri);
    let fy = (v * rows).rem_euclid(1.0);
    let t = u * count + rand2(row, 0, seed ^ 0x11) * count;
    // Tile ends: one per whole step of `t`, jittered; indices wrap so each row tiles.
    let cut = |j: i32| j as f32 + (rand2(j.rem_euclid(ci), row, seed) - 0.5) * jitter.clamp(0.0, 0.95);
    let j = t.floor() as i32;
    let j = if t < cut(j) {
        j - 1
    } else if t >= cut(j + 1) {
        j + 1
    } else {
        j
    };
    let along = (t - cut(j)).min(cut(j + 1) - t) / count * rows;
    tile(along.min(fy).min(1.0 - fy), gap, bevel, rand2(j.rem_euclid(ci), row, seed ^ 0x22))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bricks_have_mortar_joints_and_running_bond_shifts_rows() {
        // Row joints sit at whole multiples of 1 / rows.
        assert_eq!(bricks(0.3, 0.001, 4.0, 8.0, 0.1, 0.15, true, 0).h, 0.0);
        assert!(bricks(0.3, 0.0625, 4.0, 8.0, 0.1, 0.15, true, 0).h > 0.5);
        // A vertical joint at u = 0 in row 0 is mid-brick in row 1 of a running bond...
        assert_eq!(bricks(0.001, 0.06, 4.0, 8.0, 0.1, 0.15, true, 0).h, 0.0);
        assert!(bricks(0.001, 0.19, 4.0, 8.0, 0.1, 0.15, true, 0).h > 0.5);
        // ...and a joint again in a stack bond.
        assert_eq!(bricks(0.001, 0.19, 4.0, 8.0, 0.1, 0.15, false, 0).h, 0.0);
    }

    #[test]
    fn herringbone_and_courses_cover_the_tile_with_many_tiles() {
        let mut ids = std::collections::HashSet::new();
        let (mut filled, n) = (0, 64);
        for i in 0..n * n {
            let (u, v) = ((i % n) as f32 / n as f32, (i / n) as f32 / n as f32);
            let p = herringbone(u, v, 2, 3, 0.08, 0.15, 1);
            filled += (p.h > 0.0) as i32;
            ids.insert((p.id * 1e6) as u32);
        }
        assert!(filled > n * n * 6 / 10, "{filled}");
        assert!(ids.len() > 10, "{} planks", ids.len());
        assert_eq!(coursed(0.4, 0.0005, 2.0, 8.0, 0.04, 0.08, 0.4, 1).h, 0.0);
        // More jitter moves the joints: the two layouts differ.
        let differs = (0..200).any(|i| {
            let u = i as f32 / 200.0;
            (coursed(u, 0.06, 3.0, 6.0, 0.1, 0.15, 0.1, 1).h > 0.0) != (coursed(u, 0.06, 3.0, 6.0, 0.1, 0.15, 0.8, 1).h > 0.0)
        });
        assert!(differs);
    }
}
