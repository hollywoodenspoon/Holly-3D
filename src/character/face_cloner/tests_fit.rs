use super::clone::{clone_face, Options};
use super::correspond;
use super::frames::{analyse, from_points, Shot};
use super::math::Similarity;
use super::render::head_view;
use crate::character::body::morph;
use crate::character::params::CharacterParams;
use bevy::math::{EulerRot, Quat, Vec3};

/// A character with a clearly different face.
fn someone() -> CharacterParams {
    with(&[("nose/nose-scale-horiz-decr|incr", 0.7), ("chin/chin-width-decr|incr", -0.6), ("mouth/mouth-scale-horiz-decr|incr", 0.6), ("eyes/r-eye-scale-decr|incr", 0.5), ("head/head-oval", 0.6)])
}

/// Another, different in other ways.
fn someone_else() -> CharacterParams {
    with(&[("nose/nose-scale-vert-decr|incr", -0.6), ("nose/nose-trans-backward|forward", 0.5), ("chin/chin-prominent-decr|incr", 0.6), ("eyes/r-eye-trans-down|up", 0.5), ("head/head-square", 0.7), ("mouth/mouth-upperlip-volume-decr|incr", 0.6)])
}

fn with(sliders: &[(&str, f32)]) -> CharacterParams {
    let mut p = CharacterParams::default();
    for (k, v) in sliders {
        p.set_micro(k, *v);
    }
    p
}

fn face_points(p: &CharacterParams) -> Vec<Vec3> {
    let (c, _) = morph(p);
    correspond::get().unwrap().verts.iter().map(|&v| c[v as usize]).collect()
}

/// RMS distance of two point sets after the best similarity alignment.
fn shape_gap(a: &[Vec3], b: &[Vec3]) -> f32 {
    let s = super::math::similarity(a, b, &vec![1.0; a.len()], None);
    (a.iter().zip(b).map(|(&p, &q)| (s.apply(p) - q).length_squared()).sum::<f32>() / a.len() as f32).sqrt()
}

fn exact_shot(pts: &[Vec3], yaw_deg: f32) -> Shot {
    let cam = Similarity { scale: 300.0, rot: Quat::from_euler(EulerRot::YXZ, yaw_deg.to_radians(), 0.05, 0.0), offset: Vec3::new(400.0, -300.0, 0.0) };
    let mut img: Vec<Vec3> = pts.iter().map(|&p| super::frames::to_canonical_frame(cam.apply(p))).collect();
    img.resize(super::model::POINTS, img[0]);
    from_points(&format!("yaw{yaw_deg}"), (800, 600), img, 1.0)
}

#[test]
fn exact_projections_of_a_known_face_are_recovered() {
    let truth = face_points(&someone());
    let base = face_points(&CharacterParams::default());
    let shots: Vec<Shot> = [0.0, 35.0, -35.0, 70.0].iter().map(|&y| exact_shot(&truth, y)).collect();
    assert!(shots[1].yaw > 25.0 && shots[2].yaw < -25.0, "{} {}", shots[1].yaw, shots[2].yaw);
    let opt = Options { calibrate: false, ..Options::default() };
    let (q, rep) = clone_face(&CharacterParams::default(), &shots, &opt).unwrap();
    let got = face_points(&q);
    let (before, after) = (shape_gap(&base, &truth), shape_gap(&got, &truth));
    eprintln!("{rep:?}\nshape gap before {before:.4} dm, after {after:.4} dm");
    assert!(after < before * 0.25, "fit should recover the face ({before} -> {after})");
    assert!(rep.warp_mm <= rep.sliders_mm + 1e-3 && rep.sliders_mm < rep.before_mm * 0.5, "{rep:?}");
    assert_eq!(q.face.views, 4);
    // Only face sliders were touched, and the warp stops at the neck.
    assert!(q.micro.keys().all(|k| super::fit::GROUPS.iter().any(|g| k.starts_with(&format!("{g}/")))));
    let (c0, _) = morph(&CharacterParams::default());
    let (c, r) = crate::character::anime::head_sphere(&c0, crate::character::body::body_mask());
    assert!(!q.face.is_empty() && q.face.deltas.iter().all(|(i, _)| c0[*i as usize].y > c.y - r * 1.75));
}

#[test]
fn rendered_default_head_clones_to_almost_itself() {
    let base = face_points(&CharacterParams::default());
    let shots: Vec<Shot> = [0.0f32, 0.6, -0.6]
        .iter()
        .map(|&y| {
            let mut s = analyse("view", &head_view(&morph(&CharacterParams::default()).0, y).1).unwrap();
            s.used = true;
            s
        })
        .collect();
    let (q, rep) = clone_face(&CharacterParams::default(), &shots, &Options::default()).unwrap();
    let gap = shape_gap(&face_points(&q), &base);
    let worst = q.micro.values().fold(0.0f32, |m, v| m.max(v.abs()));
    eprintln!("{rep:?}\nself-clone shape gap {gap:.4} dm, largest slider {worst:.2}, {} warp verts", q.face.deltas.len());
    assert!(gap < 0.03, "the unchanged head should stay put ({gap} dm)");
}

#[test]
fn rendered_known_faces_are_moved_toward_the_truth() {
    let base = face_points(&CharacterParams::default());
    for p in [someone(), someone_else()] {
        let (coords, _) = morph(&p);
        let truth = face_points(&p);
        let shots: Vec<Shot> = [0.0f32, 0.6, -0.6]
            .iter()
            .map(|&y| {
                let mut s = analyse("view", &head_view(&coords, y).1).expect("face found on the morphed head");
                s.used = true;
                s
            })
            .collect();
        let (q, rep) = clone_face(&CharacterParams::default(), &shots, &Options::default()).unwrap();
        let (before, after) = (shape_gap(&base, &truth), shape_gap(&face_points(&q), &truth));
        eprintln!("{:?}\nrendered: shape gap before {before:.4} dm, after {after:.4} dm", (rep.before_mm, rep.sliders_mm, rep.warp_mm));
        // MediaPipe follows real shape changes only partly (about 0.5-0.7x,
        // correlation 0.8), so a clone from renders closes part of the gap.
        assert!(after < before * 0.8, "the clone should be nearer the truth than the base head ({before} -> {after})");
        assert!(rep.warp_mm < rep.before_mm * 0.6, "{rep:?}");
    }
}

#[test]
fn old_characters_without_a_face_still_load() {
    let mut v = serde_json::to_value(CharacterParams::default()).unwrap();
    v.as_object_mut().unwrap().remove("face");
    let p: CharacterParams = serde_json::from_value(v).unwrap();
    assert!(p.face.is_empty() && p.face.strength == 1.0);
}
