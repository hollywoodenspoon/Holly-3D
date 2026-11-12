use super::bake::{bake, View};
use super::delight::{luminance, to_linear};
use super::frames::analyse;
use super::mask::{landmark_mask, Mask};
use super::math::Similarity;
use super::render::{self, head_view};
use crate::character::anime::head_sphere;
use crate::character::body::{body_mask, morph};
use crate::character::makehuman::library;
use crate::character::params::CharacterParams;
use crate::materials::texture::Texture;
use bevy::math::{Quat, Vec3};

/// The exact camera of a render view, in the cloner's image frame.
fn cam_of(v: &render::View) -> Similarity {
    let rot = Quat::from_mat3(&v.rot());
    let h = v.size as f32 * 0.5;
    Similarity { scale: v.scale, rot, offset: Vec3::new(h, -h, 0.0) - rot * v.center * v.scale }
}

fn truth_skin() -> Texture {
    let mm = library::material("skins/young_caucasian_female/young_caucasian_female.mhmat").unwrap();
    (*library::texture(&format!("skins/young_caucasian_female/{}", mm.diffuse_texture.unwrap())).unwrap().tex).clone()
}

fn lin(p: [u8; 4]) -> Vec3 {
    Vec3::new(to_linear(p[0]), to_linear(p[1]), to_linear(p[2]))
}

#[test]
fn a_render_projects_onto_the_head_through_its_camera() {
    let (coords, _) = morph(&CharacterParams::default());
    let (v, _) = head_view(&coords, 0.4);
    let cam = cam_of(&v);
    for i in [100usize, 5000, 9000] {
        let p = coords[i];
        let (a, c) = (v.project(p), cam.apply(p));
        assert!((a.x - c.x).abs() < 1e-3 && (a.y + c.y).abs() < 1e-3, "{a} {c}");
    }
}

#[test]
fn landmark_mask_covers_the_face_but_not_the_eyes() {
    let (coords, _) = morph(&CharacterParams::default());
    let s = analyse("front", &head_view(&coords, 0.0).1).unwrap();
    let m = landmark_mask(&s);
    let at = |i: usize| m.at(bevy::math::Vec2::new(s.point(i).x, s.point(i).y));
    assert!(at(4) > 0.9 && at(151) > 0.9 && at(50) > 0.9, "nose, forehead, cheek: {} {} {}", at(4), at(151), at(50));
    assert!(at(468) < 0.1 && at(473) < 0.1, "iris centres are masked out");
    let far = m.at(bevy::math::Vec2::new(5.0, 5.0));
    assert_eq!(far, 0.0);
    assert!(!m.sam);
}

#[test]
fn baked_skin_matches_the_rendered_skin_and_tints_the_rest() {
    let p = CharacterParams::default();
    let (coords, _) = morph(&p);
    let head = head_sphere(&coords, body_mask());
    let shots: Vec<_> = [0.0f32, 0.6, -0.6].iter().map(|&y| head_view(&coords, y)).collect();
    let analysed: Vec<_> = shots.iter().map(|(_, img)| analyse("v", img).unwrap()).collect();
    let masks: Vec<Mask> = analysed.iter().map(landmark_mask).collect();
    let views: Vec<View> = shots.iter().zip(&masks).map(|((v, img), m)| View { cam: cam_of(v), img, mask: m, weight: 1.0 }).collect();
    let truth = truth_skin();
    let grey = Texture::new(truth.width, truth.height, [128, 128, 128, 255]);
    let (out, rep) = bake(&coords, head, &views, &grey);
    eprintln!("{rep:?}");
    assert!(rep.covered > 2000.0, "the face should be covered ({})", rep.covered);
    // Where the photos were used, the bake should follow the true skin
    // (up to exposure: brightness is unknown from photos).
    let tinted = out.data[0];
    let region: Vec<usize> = (0..out.data.len()).filter(|&i| out.data[i] != tinted && truth.data[i][3] > 200).collect();
    let (sb, st): (f32, f32) = region.iter().fold((0.0, 0.0), |(a, b), &i| (a + luminance(lin(out.data[i])), b + luminance(lin(truth.data[i]))));
    let g = st / sb;
    let err = region.iter().map(|&i| (lin(out.data[i]) * g - lin(truth.data[i])).abs().element_sum() / 3.0).sum::<f32>() / region.len() as f32;
    let flat = region.iter().map(|&i| (Vec3::splat(st / region.len() as f32) - lin(truth.data[i])).abs().element_sum() / 3.0).sum::<f32>() / region.len() as f32;
    eprintln!("{} texels, error {err:.4} vs flat {flat:.4}, gain {g:.2}", region.len());
    assert!(err < 0.06, "baked face texels should be close to the true skin ({err})");
    // The rest of the body took the face's tone: warmer than the grey it was.
    let t = lin(tinted);
    assert!(t.x > t.y && t.y > t.z, "tinted toward skin: {t}");
}

/// Runs only where MobileSAM is installed (it is an optional download).
#[test]
fn mobilesam_mask_keeps_the_face_skin_when_installed() {
    if !super::sam::installed() {
        eprintln!("MobileSAM not installed: skipped");
        return;
    }
    let (coords, _) = morph(&CharacterParams::default());
    for yaw in [0.0f32, 0.6] {
        let img = head_view(&coords, yaw).1;
        let s = analyse("v", &img).unwrap();
        let t = std::time::Instant::now();
        let m = super::mask::skin_mask(&s, &img);
        let lm = landmark_mask(&s);
        let kept = m.data.iter().sum::<f32>() / lm.data.iter().sum::<f32>();
        eprintln!("yaw {yaw}: sam {} kept {kept:.2} in {:?}", m.sam, t.elapsed());
        assert!(m.sam && kept > 0.6, "MobileSAM should keep most of a clean face ({kept})");
    }
}

/// A dark fringe over the forehead is cut out of the skin mask.
#[test]
fn a_fringe_is_cut_off_the_forehead() {
    let (coords, _) = morph(&CharacterParams::default());
    let mut img = head_view(&coords, 0.0).1;
    let s = analyse("v", &img).unwrap();
    let (top, brow) = (s.point(10), s.point(9));
    let (x0, x1) = (s.point(54).x as u32, s.point(284).x as u32);
    for y in (top.y as u32).saturating_sub(40)..(top.y + (brow.y - top.y) * 0.7) as u32 {
        for x in x0..x1 {
            img.put_pixel(x, y, image::Rgb([52, 34, 22]));
        }
    }
    let m = super::mask::skin_mask(&s, &img);
    let lm = landmark_mask(&s);
    let fringe = |mk: &Mask| {
        let y = (top.y + (brow.y - top.y) * 0.45) as u32;
        (x0 + 10..x1 - 10).map(|x| mk.data[(y * mk.w + x) as usize]).sum::<f32>() / (x1 - x0 - 20) as f32
    };
    let cheek = m.at(bevy::math::Vec2::new(s.point(50).x, s.point(50).y));
    eprintln!("fringe: landmarks {:.2}, sam {:.2}; cheek {cheek:.2}", fringe(&lm), fringe(&m));
    assert!(fringe(&m) < 0.3 * fringe(&lm) && cheek > 0.8);
    assert_eq!(m.sam, super::sam::installed());
}

/// Photos on disk -> clone -> the character wears the photo skin on ID 1;
/// switching it off gives the character's own skin back.
#[test]
fn a_clone_from_files_puts_the_photo_skin_on_the_character_pack() {
    use crate::character::wardrobe::materials::CLONED_SKIN;
    use crate::character::params::ids;
    use crate::editor::Editor;
    let dir = std::env::temp_dir().join(format!("ilex-face-photos-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (coords, _) = morph(&CharacterParams::default());
    let shots: Vec<_> = [0.0f32, 0.6, -0.6]
        .iter()
        .enumerate()
        .map(|(i, &y)| {
            let img = head_view(&coords, y).1;
            let path = dir.join(format!("shot{i}.png"));
            img.save(&path).unwrap();
            let mut s = analyse(&path.display().to_string(), &img).unwrap();
            s.used = true;
            s
        })
        .collect();
    let mut ed = Editor::default();
    let id = ed.create_character(CharacterParams::default(), Vec3::ZERO);
    let (_, p) = ed.character().unwrap();
    let (q, rep) = super::clone::clone_face(&p, &shots, &super::clone::Options::default()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    let k = rep.skin.expect("the skin was baked");
    assert_eq!(k.photos, 3);
    assert!(q.face.texture.is_some());
    ed.set_character(id, q.clone());
    let skin_name = |ed: &Editor| {
        let m = &ed.scene.materials.mats[ed.scene.objects[id].material.unwrap()];
        m.multi.as_ref().unwrap().subs.iter().find(|s| s.id == ids::SKIN).unwrap().name.clone()
    };
    assert_eq!(skin_name(&ed), CLONED_SKIN);
    let mut off = q;
    off.face.use_texture = false;
    ed.set_character(id, off);
    assert_ne!(skin_name(&ed), CLONED_SKIN);
}
