//! The 4-wide SIMD BVH and the 32x32 tile scheduler.

use super::bvh::{Bvh, Tri};
use super::job::{TILE, tiles};
use bevy::math::{Vec2, Vec3, Vec4};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn soup(n: usize, rng: &mut ChaCha8Rng) -> Vec<Tri> {
    let mut r = || Vec3::new(rng.gen_range(-5.0..5.0), rng.gen_range(-5.0..5.0), rng.gen_range(-5.0..5.0));
    (0..n)
        .map(|i| {
            let c = r();
            let p = [c, c + r() * 0.15, c + r() * 0.15];
            Tri { p, n: [Vec3::Z; 3], uv: [Vec2::ZERO; 3], col: [Vec4::ONE; 3], mat: 0, obj: i as u32 }
        })
        .collect()
}

#[test]
fn wide_bvh_matches_binary_and_brute_force() {
    let mut rng = ChaCha8Rng::seed_from_u64(7);
    for n in [1, 3, 5, 17, 300, 2000] {
        let bvh = Bvh::build(soup(n, &mut rng));
        let mut hits = 0;
        for _ in 0..400 {
            let o = Vec3::new(rng.gen_range(-8.0..8.0), rng.gen_range(-8.0..8.0), rng.gen_range(-8.0..8.0));
            let mut d = Vec3::new(rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0), rng.gen_range(-1.0..1.0));
            if rng.gen_bool(0.2) {
                d = Vec3::new(d.x, 0.0, 0.0); // axis-aligned rays (infinite inverses)
            }
            let d = d.normalize_or(Vec3::X);
            let wide = bvh.closest(o, d, 100.0).map(|h| h.t);
            let bin = bvh.walk_binary(o, d, 100.0, false).map(|h| h.t);
            let brute = bvh.tris.iter().filter_map(|t| t.ray(o, d)).filter(|t| *t > 1e-4 && *t < 100.0).min_by(f32::total_cmp);
            assert_eq!(wide.map(|t| (t * 1e4).round()), brute.map(|t| (t * 1e4).round()), "n={n} vs brute force");
            assert_eq!(wide, bin, "n={n} vs binary");
            assert_eq!(bvh.occluded(o, d, 100.0), brute.is_some());
            hits += wide.is_some() as usize;
        }
        assert!(n < 300 || hits > 5, "rays actually hit things ({hits})");
    }
    assert!(Bvh::build(vec![]).closest(Vec3::ZERO, Vec3::X, 1.0).is_none());
}

#[test]
fn tiles_cover_the_region_exactly_once() {
    for region in [[0, 0, 100, 70], [5, 9, 37, 41], [0, 0, 32, 32], [3, 3, 4, 4]] {
        let [x0, y0, x1, y1] = region;
        let mut seen = vec![0u8; ((x1 - x0) * (y1 - y0)) as usize];
        for [tx, ty, w, h] in tiles(region) {
            assert!(w <= TILE && h <= TILE && w > 0 && h > 0);
            for y in ty..ty + h {
                for x in tx..tx + w {
                    seen[((y - y0) * (x1 - x0) + x - x0) as usize] += 1;
                }
            }
        }
        assert!(seen.iter().all(|&c| c == 1), "{region:?}");
    }
    assert_eq!(tiles([0, 0, 100, 70]).len(), 4 * 3);
}
