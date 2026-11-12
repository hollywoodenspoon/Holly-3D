use super::presets::PosePreset;
use super::*;
use crate::character::body::body_mask;
use crate::character::makehuman::obj::base;
use crate::character::CharacterParams;

fn joint_after(pose: &Pose, bone: &str) -> (Vec3, Vec3) {
    let rest = base().coords.clone();
    let mut posed = rest.clone();
    apply(pose, &mut posed);
    let s = skeleton();
    let b = s.index[bone];
    (s.head(b, &rest), s.head(b, &posed))
}

#[test]
fn skeleton_is_ordered_and_weights_are_normalised() {
    let s = skeleton();
    assert_eq!(s.bones.len(), 163);
    assert_eq!(s.bones[0].name, "root");
    for (i, b) in s.bones.iter().enumerate() {
        assert!(b.parent.is_none_or(|p| p < i), "{} comes after its parent", b.name);
        assert!(!b.head.is_empty() && !b.tail.is_empty(), "{} has joints", b.name);
    }
    let w = weights();
    assert_eq!(w.len(), base().coords.len());
    for (slot, body) in w.iter().zip(body_mask()) {
        if *body {
            let sum: f32 = slot.iter().map(|x| x.1).sum();
            assert!((sum - 1.0).abs() < 1e-4, "every body vertex is fully weighted ({sum})");
        }
    }
    // The fitted rig follows the morph: a taller body has a higher head bone.
    let mut tall = CharacterParams::default();
    tall.macros.height = 1.0;
    let (rest, _) = crate::character::body::morph(&CharacterParams::default());
    let (morphed, _) = crate::character::body::morph(&tall);
    let h = s.index["head"];
    assert!(s.head(h, &morphed).y > s.head(h, &rest).y + 0.2);
}

#[test]
fn rest_pose_is_a_no_op_and_joints_move_the_right_way() {
    let mut c = base().coords.clone();
    apply(&Pose::default(), &mut c);
    assert_eq!(c, base().coords, "the rest pose leaves the body alone");
    // T-Pose: the left wrist rises to about shoulder height.
    let t = PosePreset::TPose.pose();
    let (rest, posed) = joint_after(&t, "wrist.L");
    assert!(posed.y > rest.y + 2.0 && posed.x > rest.x, "{rest} -> {posed}");
    let (r_rest, r_posed) = joint_after(&t, "wrist.R");
    assert!((posed.y - r_posed.y).abs() < 0.05 && (posed.x + r_posed.x).abs() < 0.05, "mirrored: {posed} vs {r_posed}");
    assert!(r_posed.y > r_rest.y + 2.0);
    // Bending the left knee swings the ankle back and leaves the rest still.
    let mut knee = Pose::default();
    knee.set("lowerleg01.L", [90.0, 0.0, 0.0]);
    let (a0, a1) = joint_after(&knee, "foot.L");
    assert!(a1.z < a0.z - 2.0 && a1.y > a0.y + 1.0, "ankle {a0} -> {a1}");
    for still in ["foot.R", "head", "wrist.L"] {
        let (p0, p1) = joint_after(&knee, still);
        assert!((p0 - p1).length() < 1e-3, "{still} stays put");
    }
    // Children inherit: bending the chest forward moves the head forward.
    let mut chest = Pose::default();
    chest.set("spine02", [30.0, 0.0, 0.0]);
    let (h0, h1) = joint_after(&chest, "head");
    assert!(h1.z > h0.z + 1.0 && h1.y < h0.y);
}

#[test]
fn mirroring_and_copying_sides() {
    let walk = PosePreset::Walk.pose();
    let m = walk.mirrored();
    assert_eq!(m.get("upperleg01.R"), pose::mirror(walk.get("upperleg01.L")));
    assert_eq!(m.mirrored(), walk, "mirroring twice is the identity");
    let mut p = Pose::default();
    p.set("upperarm01.L", [10.0, 20.0, 30.0]);
    p.copy_side(true);
    assert_eq!(p.get("upperarm01.R"), [10.0, -20.0, -30.0]);
    p.set_mirrored("upperarm01.L", [0.0; 3]);
    assert!(p.is_rest(), "zero rotations are removed");
    assert_eq!(pose::twin("foot.L").as_deref(), Some("foot.R"));
    assert_eq!(pose::twin("head"), None);
    for (_, l, r) in pose::CONTROLS {
        assert!(skeleton().index.contains_key(l), "{l}");
        if let Some(r) = r {
            assert!(skeleton().index.contains_key(r) && pose::twin(l).as_deref() == Some(r));
        }
    }
}

#[test]
fn posed_characters_are_valid_and_stand_on_the_ground() {
    for preset in [PosePreset::TPose, PosePreset::Sit, PosePreset::Fight] {
        let p = CharacterParams { pose: preset.pose(), ..Default::default() };
        let m = p.build();
        let rest = CharacterParams::default().build();
        assert_eq!(m.counts(), rest.counts(), "{preset:?}: posing keeps the topology");
        let low = |m: &crate::mesh::EditablePoly| m.vertices.values().map(|v| v.position.z).fold(f32::MAX, f32::min);
        // The outfit sits a hair outside the skin, so compare with the rest build.
        assert!((low(&m) - low(&rest)).abs() < 0.01, "{preset:?} stands on the ground ({} vs {})", low(&m), low(&rest));
        assert!(m.vertices.values().all(|v| v.position.is_finite()));
        assert_ne!(m.vertices.values().map(|v| v.position).collect::<Vec<_>>(), rest.vertices.values().map(|v| v.position).collect::<Vec<_>>());
    }
    assert_ne!(PosePreset::Fight.pose().get("finger3-2.R"), [0.0; 3], "the fighting stance makes fists");
    for preset in PosePreset::ALL {
        assert!(preset == PosePreset::APose || !preset.pose().is_rest(), "{preset:?} poses something");
    }
}

#[test]
fn pose_files_round_trip_with_legacy_headers() {
    let pose = PosePreset::Wave.pose();
    let s = pose_file::to_string(&pose).unwrap();
    assert!(s.contains("format: \"Ilex\""));
    assert_eq!(pose_file::from_str(&s).unwrap(), pose);
    assert_eq!(pose_file::from_str(&s.replacen("\"Ilex\"", "\"MiniMax\"", 1)).unwrap(), pose, "old MiniMax header");
    assert!(pose_file::from_str(&s.replacen("\"Ilex\"", "\"Other\"", 1)).is_err());
    let dir = std::env::temp_dir().join(format!("ilex-pose-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let saved = pose_file::save(&pose, &dir.join("wave")).unwrap();
    assert_eq!(saved.extension().unwrap(), "ilexpose");
    let legacy = dir.join("wave.mmaxpose");
    std::fs::write(&legacy, s.replacen("\"Ilex\"", "\"MiniMax\"", 1)).unwrap();
    assert!(crate::io::scene_file::is_pose_path(&legacy));
    assert_eq!(pose_file::load(&legacy).unwrap(), pose);
    assert_eq!(pose_file::load(&saved).unwrap(), pose);
}

#[test]
fn hand_shapes_curl_the_fingers_into_the_palm() {
    use super::hands::{set_hand, HandShape};
    let tip = |pose: &Pose, f: &str| {
        let rest = base().coords.clone();
        let mut posed = rest.clone();
        apply(pose, &mut posed);
        let s = skeleton();
        let (b, w) = (s.index[f], s.index[&f.replace("finger3-3", "wrist").replace("finger2-3", "wrist")]);
        (s.tail(b, &rest).distance(s.head(w, &rest)), s.tail(b, &posed).distance(s.head(w, &posed)), s.tail(b, &rest), s.tail(b, &posed))
    };
    let mut fist = Pose::default();
    set_hand(&mut fist, HandShape::Fist, true, true);
    let (d0, d1, l0, l) = tip(&fist, "finger3-3.L");
    assert!(d1 < d0 * 0.75, "the middle finger closes toward the wrist ({d0} -> {d1})");
    // Into the palm, not backwards: measured across the finger direction.
    let (c, s) = (&base().coords, skeleton());
    let d = (s.tail(s.index["finger3-3.L"], c) - s.head(s.index["finger3-1.L"], c)).normalize();
    let n = super::hands::palm_normal(c);
    let across = (n - d * d.dot(n)).normalize();
    let mut relaxed = Pose::default();
    set_hand(&mut relaxed, HandShape::Relaxed, true, false);
    let (.., l_relaxed) = tip(&relaxed, "finger3-3.L");
    assert!((l_relaxed - l0).dot(across) > 0.15, "it curls into the palm ({})", (l_relaxed - l0).dot(across));
    assert!((l - l0).dot(across) > 0.0);
    // The thumb folds toward the little finger.
    let (pinky, thumb) = (s.head(s.index["finger5-1.L"], c), s.index["finger1-3.L"]);
    let mut posed = c.clone();
    apply(&fist, &mut posed);
    assert!(s.tail(thumb, &posed).distance(pinky) < s.tail(thumb, c).distance(pinky) - 0.1, "thumb across the palm");
    let (_, _, _, r) = tip(&fist, "finger3-3.R");
    assert!((l.x + r.x).abs() < 0.05 && (l.y - r.y).abs() < 0.05, "both hands mirror: {l} vs {r}");
    // Point keeps the index finger straight while the others curl.
    let mut point = Pose::default();
    set_hand(&mut point, HandShape::Point, true, false);
    let (i0, i1, ..) = tip(&point, "finger2-3.L");
    assert!((i0 - i1).abs() < 0.05, "index stays out ({i0} -> {i1})");
    assert!(point.joints.keys().all(|k| k.ends_with(".L")), "left hand only");
    set_hand(&mut fist, HandShape::Open, true, true);
    assert!(fist.is_rest(), "Open clears the finger rotations");
}


#[test]
fn two_bone_ik_reaches_the_target_and_bends_the_right_way() {
    use super::ik::{joints, nudge, solve, Limb};
    let (c, s) = (&base().coords, skeleton());
    for (limb, right) in [(Limb::Arm, false), (Limb::Arm, true), (Limb::Leg, false), (Limb::Leg, true)] {
        let mut pose = PosePreset::Relaxed.pose();
        let chain = limb.chain(right);
        let [sh, _, end] = joints(s, c, &pose, &chain).unwrap();
        let target = sh + (end - sh) * 0.8 + Vec3::Z * 1.5;
        assert!(solve(&mut pose, c, limb, right, target));
        let [sh2, mid, end2] = joints(s, c, &pose, &chain).unwrap();
        assert!(sh.distance(sh2) < 1e-3, "{limb:?}: the shoulder / hip stays put");
        assert!(end2.distance(target) < 0.05, "{limb:?} right={right}: reaches {target} (got {end2})");
        // Knees bend forward, elbows point back.
        let side = (mid - (sh2 + end2) * 0.5).z;
        assert!(if limb == Limb::Leg { side > 0.0 } else { side < 0.2 }, "{limb:?} bends the natural way ({side})");
    }
    // Nudging the left hand up by 5 cm (0.5 hm08 units) raises the wrist.
    let mut pose = Pose::default();
    let before = joints(s, c, &pose, &Limb::Arm.chain(false)).unwrap()[2];
    assert!(nudge(&mut pose, c, Limb::Arm, false, Vec3::Y * 0.5));
    let after = joints(s, c, &pose, &Limb::Arm.chain(false)).unwrap()[2];
    assert!((after - before - Vec3::Y * 0.5).length() < 0.05, "{before} -> {after}");
    assert!(pose.joints.keys().all(|k| k.ends_with(".L")), "one side only");
}
