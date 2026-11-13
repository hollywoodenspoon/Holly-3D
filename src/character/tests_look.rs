//! Check renders of the head (not part of `cargo test`):
//! `LOOK_SHOTS=/some/dir cargo test --lib character_look_renders -- --ignored`
//! writes close-ups of a human's hair, brows, lashes and eyes.

use super::params::{CharacterParams, Eyes, ids};
use super::wardrobe::{self, MhAssets};
use crate::materials::{Material, MultiPreset};
use crate::render::RenderSettings;
use crate::render::job::raytrace_region;
use crate::render::scene_build::RenderScene;
use crate::scene::{BaseObject, ObjId, Scene};
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::Vec3;
use std::sync::atomic::AtomicBool;

fn shot(s: &Scene, dir: &str, name: &str, target: Vec3, dist: f32, yaw: f32, pitch: f32) {
    let mut cam = ViewCamera::new(ViewKind::Perspective);
    (cam.target, cam.distance, cam.yaw, cam.pitch) = (target, dist, yaw.to_radians(), pitch.to_radians());
    let cfg = RenderSettings { width: 640, height: 640, aa: 2, ..Default::default() };
    let rs = RenderScene::build(s, cam, true);
    let px = raytrace_region(&rs, &cfg, [0, 0, cfg.width, cfg.height], &AtomicBool::new(false), |_, _| {});
    let mut img = image::RgbImage::new(cfg.width, cfg.height);
    for (i, p) in px.iter().enumerate() {
        let c = p.display.truncate().clamp(Vec3::ZERO, Vec3::ONE) * 255.0;
        img.put_pixel(i as u32 % cfg.width, i as u32 / cfg.width, image::Rgb([c.x as u8, c.y as u8, c.z as u8]));
    }
    img.save(format!("{dir}/{name}.png")).expect("write the check render");
}

/// A human the way the Studio makes one, with Mat IDs and the pack material.
pub(crate) fn human(s: &mut Scene, p: CharacterParams) -> ObjId {
    let id = s.add(BaseObject::Character(Box::new(p)), Vec3::ZERO);
    let mut m = Material::named("Skin");
    MultiPreset::Character.apply(&mut m);
    if let (Some(ms), BaseObject::Character(c)) = (m.multi.as_mut(), &s.objects[id].base) {
        wardrobe::materials::sync(&c.mh, c.face.skin(), ms);
    }
    s.objects[id].material = Some(s.materials.mats.insert(m));
    s.objects[id].evaluate();
    id
}

/// Centre of the faces with Mat ID `mid`.
fn centre(s: &Scene, id: ObjId, mid: u16) -> Vec3 {
    let m = &s.objects[id].evaluated_mesh;
    let pts: Vec<Vec3> = m.polygons.values().filter(|p| p.material_id == mid).flat_map(|p| p.corners.iter().map(|c| m.vertices[c.vertex_id].position)).collect();
    pts.iter().copied().sum::<Vec3>() / pts.len().max(1) as f32
}

#[test]
#[ignore]
fn character_look_renders() {
    let Ok(dir) = std::env::var("LOOK_SHOTS") else { return };
    let tag = std::env::var("LOOK_TAG").unwrap_or_else(|_| "now".into());
    // LOOK_DYE="0.8,0.4,0.18" renders with that hair colour.
    let dye = std::env::var("LOOK_DYE").ok().and_then(|d| {
        let v: Vec<f32> = d.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        (v.len() == 3).then(|| [v[0], v[1], v[2]])
    });
    for (who, hair, gender) in [("f", "ponytail01", 0.0), ("m", "short02", 1.0)] {
        let mut s = Scene::default();
        let mh = MhAssets { hair: Some(hair.into()), eye_color: Some("brown".into()), hair_color: dye, ..MhAssets::face_kit() };
        let mut p = CharacterParams { eyes: Eyes::High, mh, ..Default::default() };
        (p.hair, p.outfit) = (Default::default(), Default::default());
        p.macros.gender = gender;
        let id = human(&mut s, p);
        let m = s.objects[id].evaluated_mesh.clone();
        let top = m.vertices.values().map(|v| v.position.z).fold(0.0, f32::max);
        let head = Vec3::new(0.0, 0.0, top - 0.13);
        let eye = centre(&s, id, ids::EYES);
        shot(&s, &dir, &format!("{tag}_{who}_1front"), head, 0.6, 90.0, 0.0);
        shot(&s, &dir, &format!("{tag}_{who}_2three_quarter"), head, 0.6, 45.0, -5.0);
        shot(&s, &dir, &format!("{tag}_{who}_3back"), head, 0.6, -60.0, -10.0);
        shot(&s, &dir, &format!("{tag}_{who}_4eyes"), eye + Vec3::new(0.0, -0.03, 0.0), 0.16, 90.0, 0.0);
    }
}
