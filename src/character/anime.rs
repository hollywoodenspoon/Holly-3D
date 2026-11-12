//! Anime Ratio: blends a MakeHuman body toward stylised anime proportions
//! (bigger eyes, smaller nose and mouth, V-shaped jaw, larger head) and
//! "spherized" toon normals on the head so cel shading reads as clean
//! flat planes instead of noisy facial detail.

use super::params::CharacterParams;
use bevy::math::Vec3;

/// Micro slider recipe at Anime Ratio 1.
const RECIPE: &[(&str, f32)] = &[
    ("eyes/r-eye-scale-decr|incr", 1.0),
    ("eyes/r-eye-height2-decr|incr", 0.8),
    ("nose/nose-scale-horiz-decr|incr", -0.8),
    ("nose/nose-scale-vert-decr|incr", -0.6),
    ("nose/nose-scale-depth-decr|incr", -0.7),
    ("mouth/mouth-scale-horiz-decr|incr", -0.6),
    ("chin/chin-width-decr|incr", -0.9),
    ("chin/chin-prominent-decr|incr", 0.4),
    ("head/head-invertedtriangular", 0.8),
];

/// Adds the anime micro offsets (on top of the user's own values).
pub fn micro(p: &CharacterParams) -> CharacterParams {
    let mut q = p.clone();
    if p.anime <= 0.0 {
        return q;
    }
    for (k, v) in RECIPE {
        let cur = q.micro.get(*k).copied().unwrap_or(0.0);
        q.set_micro(k, (cur + v * p.anime).clamp(-1.0, 1.0));
    }
    q
}

/// Head centre and radius in hm08 coordinates (top ~12% of the height).
pub fn head_sphere(coords: &[Vec3], mask: &[bool]) -> (Vec3, f32) {
    let ys = coords.iter().zip(mask).filter(|(_, m)| **m).map(|(c, _)| c.y);
    let (lo, hi) = ys.fold((f32::MAX, f32::MIN), |(a, b), y| (a.min(y), b.max(y)));
    let cut = hi - (hi - lo) * 0.12;
    let (mut sum, mut n) = (Vec3::ZERO, 0.0);
    for (c, _) in coords.iter().zip(mask).filter(|(c, m)| **m && c.y > cut) {
        sum += *c;
        n += 1.0;
    }
    let c = if n > 0.0 { sum / n } else { Vec3::new(0.0, hi, 0.0) };
    (c, (hi - c.y).max(0.1))
}

/// Grows the head about its centre (Anime Ratio 1 = +22%), fading out
/// over the neck so the seam stays smooth.
pub fn grow_head(coords: &mut [Vec3], head: (Vec3, f32), anime: f32) {
    if anime <= 0.0 {
        return;
    }
    let (c, r) = head;
    let s = 1.0 + 0.22 * anime;
    let neck = c.y - r * 1.15;
    for p in coords.iter_mut() {
        let t = ((p.y - neck) / (r * 0.5)).clamp(0.0, 1.0);
        let k = 1.0 + (s - 1.0) * t * t * (3.0 - 2.0 * t);
        if k > 1.0 {
            *p = c + (*p - c) * k + Vec3::Y * (k - 1.0) * r * 0.6;
        }
    }
}

/// Toon normal at `p` (head region only): the surface normal `n` pulled
/// toward the head-sphere normal by `anime`.
pub fn toon_normal(p: Vec3, n: Vec3, head: (Vec3, f32), anime: f32) -> Option<Vec3> {
    let (c, r) = head;
    let d = p - c;
    if anime <= 0.0 || d.y < -r * 0.9 || d.length() > r * 1.8 {
        return None;
    }
    let w = (0.75 * anime).min(0.9);
    Some(n.lerp(d.normalize_or(n), w).normalize_or(n))
}
