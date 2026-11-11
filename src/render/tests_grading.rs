//! Color Grading & Camera RAW (F9) tests.

use super::FrameBuffer;
use super::grading::*;
use super::post::Tone;
use bevy::math::{Vec3, Vec4};

fn gray_ramp(w: usize, h: usize) -> Vec<Vec4> {
    (0..w * h).map(|i| Vec3::splat((i % w) as f32 / (w - 1) as f32).extend(1.0)).collect()
}

fn graded(g: Grading) -> Vec<Vec4> {
    let mut img = gray_ramp(32, 16);
    grade(&mut img, 32, 16, &g);
    img
}

#[test]
fn neutral_grading_leaves_the_image_alone() {
    assert!(Grading::default().is_neutral());
    assert_eq!(graded(Grading::default()), gray_ramp(32, 16));
    let off = Grading { enabled: false, saturation: 50.0, ..Default::default() };
    assert!(off.is_neutral());
    assert!(!Grading { shadows: 10.0, ..Default::default() }.is_neutral());
}

#[test]
fn shadows_lift_darks_and_highlights_pull_brights() {
    let base = gray_ramp(32, 16);
    let s = graded(Grading { shadows: 100.0, ..Default::default() });
    assert!(s[4].x > base[4].x + 0.05, "darks lifted");
    assert!((s[31].x - base[31].x).abs() < 0.02, "white stays");
    let h = graded(Grading { highlights: -100.0, ..Default::default() });
    assert!(h[26].x < base[26].x - 0.05, "brights pulled down");
    assert!((h[2].x - base[2].x).abs() < 0.02, "darks stay");
}

#[test]
fn whites_and_blacks_move_the_end_points() {
    let b = graded(Grading { blacks: -100.0, ..Default::default() });
    assert_eq!(b[2].x, 0.0, "crushed blacks");
    let w = graded(Grading { whites: 100.0, ..Default::default() });
    assert_eq!(w[29].x, 1.0, "clipped whites");
}

#[test]
fn temperature_warms_and_cools() {
    let warm = white_balance(9000.0, 0.0);
    let cool = white_balance(4000.0, 0.0);
    assert!(warm.x > warm.z, "warm: more red than blue {warm}");
    assert!(cool.z > cool.x, "cool: more blue than red {cool}");
    let n = white_balance(NEUTRAL_K, 0.0);
    assert!((n - Vec3::ONE).abs().max_element() < 1e-3, "6500 K is neutral {n}");
    let magenta = white_balance(NEUTRAL_K, 60.0);
    assert!(magenta.y < magenta.x && magenta.y < magenta.z, "tint + is magenta");
    let img = graded(Grading { temperature: 9000.0, ..Default::default() });
    assert!(img[16].x > img[16].z, "gray turns warm");
}

#[test]
fn saturation_and_vibrance() {
    let color = |g: Grading| {
        let mut img = vec![Vec4::new(0.6, 0.4, 0.3, 1.0)];
        grade(&mut img, 1, 1, &g);
        img[0]
    };
    let spread = |c: Vec4| c.truncate().max_element() - c.truncate().min_element();
    let base = spread(Vec4::new(0.6, 0.4, 0.3, 1.0));
    assert!(spread(color(Grading { saturation: -100.0, ..Default::default() })) < 1e-4, "-100 is gray");
    assert!(spread(color(Grading { saturation: 50.0, ..Default::default() })) > base);
    assert!(spread(color(Grading { vibrance: 50.0, ..Default::default() })) > base);
}

#[test]
fn split_toning_tints_shadows_and_highlights_separately() {
    let g = Grading { shadow_wheel: Wheel { hue: 220.0, amount: 1.0 }, highlight_wheel: Wheel { hue: 30.0, amount: 1.0 }, ..Default::default() };
    let img = graded(g);
    assert!(img[4].z > img[4].x, "blue shadows");
    assert!(img[28].x > img[28].z, "orange highlights");
}

#[test]
fn vignette_darkens_corners_not_the_centre() {
    let mut img = vec![Vec4::new(0.5, 0.5, 0.5, 1.0); 33 * 33];
    grade(&mut img, 33, 33, &Grading { vignette: -100.0, ..Default::default() });
    assert!((img[16 * 33 + 16].x - 0.5).abs() < 1e-4, "centre untouched");
    assert!(img[0].x < 0.1, "corner dark {}", img[0].x);
}

#[test]
fn chromatic_aberration_fringes_edges() {
    // A white square in the middle: its corner edges pick up colour.
    let (w, h) = (64, 64);
    let mut img: Vec<Vec4> = (0..w * h).map(|i| if (16..48).contains(&(i % w)) && (16..48).contains(&(i / w)) { Vec4::ONE } else { Vec4::W }).collect();
    let before = img.clone();
    grade(&mut img, w, h, &Grading { chromatic_aberration: 100.0, ..Default::default() });
    let fringed = img.iter().filter(|p| (p.x - p.z).abs() > 0.05).count();
    assert!(fringed > 0, "fringes appear");
    assert_eq!(img[32 * w + 32], before[32 * w + 32], "centre stays white");
}

#[test]
fn film_grain_is_deterministic_and_changes_with_seed() {
    let g = Grading { grain: 60.0, ..Default::default() };
    let (a, b) = (graded(g), graded(g));
    assert_eq!(a, b, "same seed, same grain");
    assert_ne!(a, gray_ramp(32, 16));
    assert_ne!(graded(Grading { grain_seed: 9, ..g }), a);
}

#[test]
fn clarity_adds_local_contrast() {
    // A soft step: clarity steepens it around the edge.
    let (w, h) = (64, 8);
    let step: Vec<Vec4> = (0..w * h).map(|i| Vec3::splat(if i % w < 32 { 0.4 } else { 0.6 }).extend(1.0)).collect();
    let mut img = step.clone();
    grade(&mut img, w, h, &Grading { clarity: 100.0, ..Default::default() });
    assert!(img[4 * w + 30].x < 0.4 && img[4 * w + 33].x > 0.6, "edge contrast up");
}

#[test]
fn frame_buffer_regrades_without_losing_the_render() {
    let mut fb = FrameBuffer::new(8, 1);
    let row: Vec<Vec4> = (0..8).map(|x| Vec3::splat(x as f32 / 7.0).extend(1.0)).collect();
    fb.put_block(0, 0, 8, &row, &row);
    let g = Grading { saturation: -50.0, shadows: 80.0, ..Default::default() };
    fb.regrade(&Tone::default(), &g);
    assert!(fb.display[2].x > row[2].x, "graded");
    fb.regrade(&Tone::default(), &g);
    let once = fb.display.clone();
    fb.regrade(&Tone::default(), &g);
    assert_eq!(fb.display, once, "grading again does not stack");
    fb.regrade(&Tone::default(), &Grading::default());
    assert_eq!(fb.display, row, "back to the render");
}

#[test]
fn grading_is_saved_with_the_scene_settings() {
    let mut p = super::post::PostFx::default();
    p.grade.temperature = 5000.0;
    p.grade.shadow_wheel = Wheel { hue: 200.0, amount: 0.4 };
    let s = ron::to_string(&p).unwrap();
    let back: super::post::PostFx = ron::from_str(&s).unwrap();
    assert_eq!(back.grade, p.grade);
    let old: super::post::PostFx = ron::from_str("(bloom: true)").unwrap();
    assert!(old.grade.is_neutral(), "older files load neutral");
}
