//! PR 8: viewport precipitation preview. Falling rain streaks, snowflakes
//! or ash / glowing cinders drawn as overlay lines in a box around the
//! scene, animated with the clock, while the Dynamic Sky's weather has
//! precipitation and `Show in Perspective Viewports` is on.

use super::lines::OverlayLines;
use crate::editor::Editor;
use crate::render::sky::SkyMode;
use crate::render::weather::{Precip, Weather};
use bevy::prelude::*;
use rand::{Rng, SeedableRng};

/// Particle positions in a unit box (0..1)³, fixed per seed.
fn cloud(wx: &Weather, n: usize) -> Vec<Vec3> {
    let mut r = rand_chacha::ChaCha8Rng::seed_from_u64(0x7A1 ^ wx.seed as u64);
    (0..n).map(|_| Vec3::new(r.gen_range(0.0..1.0), r.gen_range(0.0..1.0), r.gen_range(0.0..1.0))).collect()
}

/// Line segments (start, end, color index 0 = main, 1 = cinder) for the
/// weather at time `t` (seconds) in the box `lo..lo+size`.
pub fn segments(wx: &Weather, lo: Vec3, size: Vec3, t: f32) -> Vec<(Vec3, Vec3, u8)> {
    let (count, speed, len) = match wx.precip {
        Precip::None => return vec![],
        Precip::Rain => (700, 9.0, 0.7),
        Precip::Snow => (500, 0.9, 0.08),
        Precip::Ash => (350, 0.5, 0.08),
    };
    let n = (count as f32 * wx.amount.clamp(0.0, 1.0)) as usize;
    let s = wx.slant.clamp(-60.0, 60.0).to_radians();
    let fall = Vec3::new(s.sin(), 0.0, -s.cos());
    cloud(wx, n)
        .into_iter()
        .enumerate()
        .map(|(i, u)| {
            let drop = (u.z - t * speed / size.z.max(1e-3)).rem_euclid(1.0);
            let mut p = lo + Vec3::new(u.x, u.y, drop) * size;
            if wx.precip != Precip::Rain {
                // Flakes and ash flutter sideways.
                p.x += (t * 1.3 + i as f32).sin() * 0.25;
            }
            let kind = u8::from(wx.precip == Precip::Ash && i % 6 == 0);
            match wx.precip {
                Precip::Rain => (p, p + fall * len, 0),
                _ => (p - Vec3::X * len * 0.5, p + Vec3::X * len * 0.5, kind),
            }
        })
        .collect()
}

pub fn draw(ed: Res<Editor>, time: Res<Time>, mut g: Gizmos<OverlayLines>) {
    let sky = &ed.scene.environment.sky;
    if sky.mode != SkyMode::Earth || !sky.show_in_viewport || sky.weather.precip == Precip::None {
        return;
    }
    let (mut lo, mut hi) = (Vec3::splat(-15.0), Vec3::splat(15.0));
    for o in ed.scene.objects.values().filter(|o| !o.hidden) {
        lo = lo.min(o.transform.translation - Vec3::splat(15.0));
        hi = hi.max(o.transform.translation + Vec3::splat(15.0));
    }
    lo.z = lo.z.max(-2.0);
    let size = (hi - lo).min(Vec3::splat(200.0));
    let (main, cinder) = match sky.weather.precip {
        Precip::Rain => (Color::srgba(0.7, 0.78, 0.9, 0.55), Color::NONE),
        Precip::Snow => (Color::srgba(0.95, 0.97, 1.0, 0.9), Color::NONE),
        _ => (Color::srgba(0.25, 0.24, 0.23, 0.8), Color::srgb(1.0, 0.5, 0.1)),
    };
    for (a, b, k) in segments(&sky.weather, lo, size, time.elapsed_secs()) {
        g.line(a, b, if k == 1 { cinder } else { main });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn viewport_precipitation_falls_inside_the_box() {
        let wx = Weather { precip: Precip::Rain, amount: 1.0, ..Default::default() };
        let (lo, size) = (Vec3::splat(-5.0), Vec3::splat(10.0));
        let a = segments(&wx, lo, size, 0.0);
        let b = segments(&wx, lo, size, 0.1);
        assert_eq!(a.len(), 700);
        assert!(a.iter().all(|(p, _, _)| p.cmpge(lo - 0.01).all() && p.cmple(lo + size + 0.01).all()));
        // Rain falls: most streaks moved down between frames.
        assert!(a.iter().zip(&b).filter(|(x, y)| y.0.z < x.0.z).count() > 600);
        assert!(a.iter().all(|(p, q, _)| q.z < p.z), "streaks point down");
        let ash = segments(&Weather { precip: Precip::Ash, amount: 1.0, ..Default::default() }, lo, size, 0.0);
        assert!(ash.iter().any(|s| s.2 == 1), "glowing cinders");
        assert!(segments(&Weather::default(), lo, size, 0.0).is_empty());
        let half = segments(&Weather { amount: 0.5, ..wx }, lo, size, 0.0);
        assert_eq!(half.len(), 350);
    }
}
