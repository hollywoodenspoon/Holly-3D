//! Voronoi generators: warped cracks, crackle, rounded stones, flat rock
//! plates, cells and asphalt aggregate.
//!
//! Ported from MaterialCraft (https://github.com/storytold/materialcraft:
//! `cracks` in `weathering.rs`, `crackle` and `stones` in
//! `noises_more.rs`, before the old procedural library was removed),
//! copyright (c) 2026 ArtCraft Team and the MaterialCraft contributors,
//! MIT OR Apache-2.0. Widths in pixels became fractions of a cell, since
//! Ilex evaluates per point.

use super::generators::Px;
use super::lattice::{cellular, fbm, smoothstep};

/// Cracks: 1 on a crack. Broad bends plus fine jaggedness warp a Voronoi
/// diagram; `branches` adds finer cracks that only survive in some areas.
pub fn cracks(u: f32, v: f32, s: f32, width: f32, warp: f32, branches: f32, seed: u32) -> f32 {
    let k = warp / s;
    let bend = |a: u32, b: u32| (fbm(u, v, 4.0, 4, 0.5, seed ^ a) - 0.5) * 2.0 * k + (fbm(u, v, s * 4.0, 3, 0.5, seed ^ b) - 0.5) * 0.3 * k;
    let (wu, wv) = (u + bend(11, 14), v + bend(12, 15));
    let line = |m: f32, sd: u32, half: f32| {
        let c = cellular(wu * s * m, wv * s * m, (s * m) as i32, 1.0, sd);
        1.0 - smoothstep(half, half + 0.02, (c.f2 - c.f1) * 0.5 / m)
    };
    let main = line(1.0, seed, width * 0.5);
    if branches <= 0.0 {
        return main;
    }
    let keep = smoothstep(1.0 - branches, 1.0 - branches + 0.2, fbm(u, v, s, 3, 0.5, seed ^ 13));
    main.max(line(3.0, seed ^ 7, width * 0.3) * keep * 0.8)
}

/// Crackle: 1 on a crack; `amount` < 1 hides the cracks in patches.
pub fn crackle(u: f32, v: f32, sc: f32, width: f32, disorder: f32, amount: f32, seed: u32) -> f32 {
    let width = width.max(1e-3);
    let qx = (fbm(u, v, sc, 4, 0.5, seed ^ 1) - 0.5) * disorder / sc;
    let qy = (fbm(u, v, sc, 4, 0.5, seed ^ 2) - 0.5) * disorder / sc;
    let (wu, wv) = (u + qx, v + qy);
    let c = cellular(wu * sc, wv * sc, sc as i32, 1.0, seed);
    let main = 1.0 - smoothstep(0.0, width, c.f2 - c.f1);
    let n2 = sc * 3.0;
    let c2 = cellular(wu * n2, wv * n2, n2 as i32, 1.0, seed ^ 5);
    // Finer cracks only in some areas, so they break off instead of closing into cells.
    let fine = (1.0 - smoothstep(0.0, width * 1.5, c2.f2 - c2.f1)) * smoothstep(0.55, 0.65, fbm(u, v, sc * 2.0, 3, 0.5, seed ^ 11)) * 0.6;
    let vis = smoothstep(0.9 - amount, 1.1 - amount, fbm(u, v, 2.0, 3, 0.5, seed ^ 3) * 0.999);
    (main.max(fine) * if amount >= 1.0 { 1.0 } else { vis }).clamp(0.0, 1.0)
}

/// Rounded stones (cobbles): domed Voronoi cells with a gap between them,
/// each a different height, with a little surface detail.
pub fn stones(u: f32, v: f32, sc: f32, gap: f32, bevel: f32, height_random: f32, seed: u32) -> Px {
    let c = cellular(u * sc, v * sc, sc as i32, 0.8, seed);
    let e = (c.f2 - c.f1) * 0.5 - gap * 0.5;
    if e <= 0.0 {
        return Px { h: 0.0, id: 0.0 };
    }
    let dome = smoothstep(0.0, bevel.max(1e-3), e).sqrt();
    let d = fbm(u, v, sc * 2.0, 4, 0.5, seed ^ 3);
    Px { h: dome * (1.0 - height_random * c.id) * (0.85 + 0.15 * d), id: c.id }
}

/// Flat rock plates / flagstones: warped Voronoi cells with flat tops at
/// different heights and a narrow chamfer into the joints.
pub fn plates(u: f32, v: f32, sc: f32, gap: f32, warp: f32, height_random: f32, seed: u32) -> Px {
    let k = warp / sc * 0.5;
    let wu = u + (fbm(u, v, sc * 2.0, 3, 0.5, seed ^ 21) - 0.5) * k;
    let wv = v + (fbm(u, v, sc * 2.0, 3, 0.5, seed ^ 22) - 0.5) * k;
    let c = cellular(wu * sc, wv * sc, sc as i32, 1.0, seed);
    let e = (c.f2 - c.f1) * 0.5 - gap * 0.5;
    if e <= 0.0 {
        return Px { h: 0.0, id: 0.0 };
    }
    let top = 1.0 - height_random * c.id * 0.6;
    Px { h: smoothstep(0.0, 0.05, e) * top * (0.94 + 0.06 * fbm(u, v, sc * 4.0, 3, 0.5, seed ^ 23)), id: c.id }
}

/// Plain cells: a cone to the cell border, flattened by `flatness`.
pub fn cells(u: f32, v: f32, sc: f32, gap: f32, flatness: f32, jitter: f32, seed: u32) -> Px {
    let c = cellular(u * sc, v * sc, sc as i32, jitter.clamp(0.0, 1.0), seed);
    let e = (c.f2 - c.f1) * 0.5 - gap * 0.5;
    if e <= 0.0 {
        return Px { h: 0.0, id: 0.0 };
    }
    Px { h: (e / (1.0 - flatness.clamp(0.0, 0.99)) * 2.0).min(1.0) * (0.75 + 0.25 * c.id), id: c.id }
}

/// Asphalt: fine stone aggregate bound in tar, with scattered pits.
/// `stone` sets how far the stones stand proud, `rough` the fine noise.
pub fn asphalt(u: f32, v: f32, sc: f32, pits: f32, stone: f32, rough: f32, seed: u32) -> f32 {
    let c = cellular(u * sc, v * sc, sc as i32, 1.0, seed);
    let grain = smoothstep(0.0, 0.35, (c.f2 - c.f1) * 0.5) * (0.4 + 0.6 * c.id);
    let fine = fbm(u, v, sc * 2.0, 3, 0.6, seed ^ 31);
    let pit_noise = fbm(u, v, (sc * 0.25).max(2.0), 4, 0.55, seed ^ 32);
    let pit = smoothstep(1.0 - pits * 0.5, 1.0 - pits * 0.5 + 0.08, pit_noise) * 0.45;
    (0.5 + grain * stone * 2.0 + (fine - 0.5) * rough * 0.5 - pit).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cover(f: impl Fn(f32, f32) -> f32) -> usize {
        (0..4000).filter(|i| f(*i as f32 * 0.618 % 1.0, *i as f32 * 0.137 % 1.0) > 0.5).count()
    }

    #[test]
    fn wider_cracks_cover_more_and_branches_add_cracks() {
        let wide = cover(|u, v| cracks(u, v, 5.0, 0.2, 0.5, 0.0, 1));
        let thin = cover(|u, v| cracks(u, v, 5.0, 0.04, 0.5, 0.0, 1));
        assert!(wide > thin && thin > 0, "{wide} vs {thin}");
        let any = |b: f32| (0..4000).filter(|i| cracks(*i as f32 * 0.618 % 1.0, *i as f32 * 0.137 % 1.0, 5.0, 0.05, 0.5, b, 1) > 0.1).count();
        assert!(any(1.0) > any(0.0));
        // Less crackle amount hides cracks.
        assert!(cover(|u, v| crackle(u, v, 6.0, 0.06, 0.5, 1.0, 1)) > cover(|u, v| crackle(u, v, 6.0, 0.06, 0.5, 0.3, 1)));
    }

    #[test]
    fn stones_and_plates_have_gaps_and_different_heights() {
        for f in [stones as fn(f32, f32, f32, f32, f32, f32, u32) -> Px, plates] {
            let px: Vec<Px> = (0..3000).map(|i| f(i as f32 * 0.618 % 1.0, i as f32 * 0.137 % 1.0, 6.0, 0.12, 0.2, 0.5, 2)).collect();
            let gaps = px.iter().filter(|p| p.h == 0.0).count();
            assert!(gaps > 100 && gaps < 2000, "{gaps} gap samples");
            let ids: std::collections::HashSet<u32> = px.iter().map(|p| (p.id * 1e6) as u32).collect();
            assert!(ids.len() > 15);
        }
        // Plates are flat on top: most of a plate sits near its top height.
        let tops = (0..3000).map(|i| plates(i as f32 * 0.618 % 1.0, i as f32 * 0.137 % 1.0, 5.0, 0.06, 0.4, 0.0, 2).h).filter(|h| *h > 0.9).count();
        assert!(tops > 1500, "{tops}");
    }
}
