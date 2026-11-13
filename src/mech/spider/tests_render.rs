//! Check renders for markup (not part of `cargo test`):
//! `SPIDER_SHOTS=/some/dir cargo test --lib spider_check_renders -- --ignored`
//! writes front / side / rear / top / crouch PNGs with a 1.8 m figure.

use super::kit::{Kit, block, cyl, mat};
use super::spawn_railgun;
use crate::render::RenderSettings;
use crate::render::job::raytrace_region;
use crate::render::scene_build::RenderScene;
use crate::scene::{BaseObject, Scene, Shared};
use crate::viewport::camera::{ViewCamera, ViewKind};
use bevy::math::Vec3;
use std::sync::atomic::AtomicBool;

/// A plain 1.8 m figure for scale.
fn figure() -> crate::mesh::EditablePoly {
    let mut k = Kit::default();
    for s in [-1.0, 1.0] {
        k.add(block(Vec3::new(0.16, 0.2, 0.85), Vec3::new(s * 0.11, 0.0, 0.425)), mat::HULL);
        k.add(block(Vec3::new(0.1, 0.12, 0.6), Vec3::new(s * 0.28, 0.0, 1.17)), mat::HULL);
    }
    k.add(block(Vec3::new(0.46, 0.26, 0.62), Vec3::new(0.0, 0.0, 1.16)), mat::HULL);
    k.add(cyl(0.11, 0.26, 12, Vec3::Z * 1.54, Vec3::Z), mat::HULL);
    k.finish()
}

pub(super) fn shot(s: &Scene, name: &str, target: Vec3, dist: f32, yaw: f32, pitch: f32) {
    let Ok(dir) = std::env::var("SPIDER_SHOTS") else { return };
    let mut cam = ViewCamera::new(ViewKind::Perspective);
    (cam.target, cam.distance, cam.yaw, cam.pitch) = (target, dist, yaw.to_radians(), pitch.to_radians());
    let cfg = RenderSettings { width: 1200, height: 800, aa: 2, ..Default::default() };
    let rs = RenderScene::build(s, cam, true);
    let px = raytrace_region(&rs, &cfg, [0, 0, cfg.width, cfg.height], &AtomicBool::new(false), |_, _| {});
    let mut img = image::RgbImage::new(cfg.width, cfg.height);
    for (i, p) in px.iter().enumerate() {
        let c = p.display.truncate().clamp(Vec3::ZERO, Vec3::ONE) * 255.0;
        img.put_pixel(i as u32 % cfg.width, i as u32 / cfg.width, image::Rgb([c.x as u8, c.y as u8, c.z as u8]));
    }
    img.save(format!("{dir}/{name}.png")).expect("write the check render");
}

fn pose(s: &mut Scene, id: crate::scene::ObjId, v: f32) {
    let i = s.assembly.joint_of(id).unwrap();
    s.assembly.joints[i].kind.set_value(v);
}

#[test]
#[ignore]
fn spider_check_renders() {
    let mut s = Scene::default();
    let ground = s.add(BaseObject::EditablePoly(Shared::new(block(Vec3::new(80.0, 80.0, 0.04), Vec3::Z * -0.02))), Vec3::ZERO);
    s.objects[ground].wire_color = [0.42, 0.43, 0.45];
    s.objects[ground].evaluate();
    let man = s.add(BaseObject::EditablePoly(Shared::new(figure())), Vec3::new(4.6, -4.2, 0.0));
    s.objects[man].evaluate();
    let rig = spawn_railgun(&mut s, Vec3::ZERO);
    let mid = Vec3::new(0.0, -1.0, 2.2);
    shot(&s, "01_front_three_quarter", mid, 17.0, 60.0, -14.0);
    shot(&s, "02_side", mid, 17.0, 180.0, -6.0);
    shot(&s, "03_rear_three_quarter", mid, 17.0, -120.0, -18.0);
    shot(&s, "04_top", Vec3::new(0.0, -1.5, 0.0), 20.0, 75.0, -78.0);
    // Crouch: the hull drops 0.5 m and each foot is dragged back to its
    // planted spot (IK), the turret turns and the gun lifts.
    let feet: Vec<_> = rig.legs.iter().map(|l| (l.foot, s.objects[l.foot].transform.translation)).collect();
    s.objects[rig.hull].transform.translation.z -= 0.5;
    s.solve_assembly();
    for _ in 0..3 {
        for &(f, at) in &feet {
            s.objects[f].transform.translation = at;
            s.solve_assembly();
        }
    }
    pose(&mut s, rig.turret, 30.0);
    pose(&mut s, rig.gun, 40.0);
    for _ in 0..6 {
        s.solve_assembly();
    }
    shot(&s, "05_crouch_gun_up", mid, 18.0, 45.0, -10.0);
}
