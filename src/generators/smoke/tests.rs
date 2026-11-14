use super::*;
use std::collections::HashSet;

fn ids(m: &EditablePoly) -> HashSet<u16> {
    m.polygons.values().map(|p| p.material_id).collect()
}

#[test]
fn plume_is_valid_deterministic_and_tall() {
    let p = SmokeParams::default();
    let m = p.build();
    m.validate().unwrap();
    assert!(m.polygons.len() > 500, "{}", m.polygons.len());
    assert_eq!(p.build().counts(), m.counts(), "deterministic");
    assert!(m.vertices.values().all(|v| v.position.is_finite() && v.color.is_finite()));
    assert!(m.polygons.values().all(|q| q.corners.iter().all(|c| c.uv.is_finite())));
    let b = m.bounds().unwrap();
    assert!(b.max.z > p.height * 0.9 && b.min.z > -p.base_radius * 2.0, "{b:?}");
    // Wider at the top than at the source.
    let width = |lo: f32, hi: f32| {
        let xs: Vec<f32> = m.vertices.values().filter(|v| (lo..hi).contains(&v.position.z)).map(|v| v.position.y).collect();
        xs.iter().cloned().fold(f32::MIN, f32::max) - xs.iter().cloned().fold(f32::MAX, f32::min)
    };
    assert!(width(p.height * 0.75, p.height * 1.2) > width(0.0, p.height * 0.15) * 1.5);
}

#[test]
fn fire_base_glows_and_smoke_is_translucent() {
    let m = SmokeParams { fire: 80.0, ..Default::default() }.build();
    assert_eq!(ids(&m), HashSet::from([1, 2, 3]));
    let low = |id: u16| m.polygons.values().filter(|p| p.material_id == id).map(|p| m.pos(p.corners[0].vertex_id).z).fold(f32::MAX, f32::min);
    assert!(low(1) < low(2) + 1.0, "fire at the base");
    // Density lives in the vertex alpha: below 1 in the smoke, 0 on the card rims.
    let alphas: Vec<f32> = m.vertices.values().map(|v| v.color.w).collect();
    assert!(alphas.contains(&0.0) && alphas.iter().any(|&a| a > 0.3 && a < 1.0));
    let none = SmokeParams { fire: 0.0, ..Default::default() }.build();
    assert!(!ids(&none).contains(&1), "Fire Base 0 % = smoke only");
}

#[test]
fn parameters_shape_the_plume() {
    let base = SmokeParams::default();
    let top = |p: &SmokeParams| p.build().bounds().unwrap();
    assert!(top(&SmokeParams { height: 60.0, ..base.clone() }).max.z > top(&base).max.z + 20.0);
    let calm = SmokeParams { wind_lean: 0.0, ..base.clone() };
    let windy = SmokeParams { wind_lean: 45.0, wind_dir: 90.0, ..base.clone() };
    assert!(top(&windy).max.y > top(&calm).max.y + 10.0, "leans toward +Y");
    assert!(SmokeParams { cards: false, ..base.clone() }.build().polygons.len() < base.build().polygons.len());
    assert!(SmokeParams { mushroom: false, ..base.clone() }.build().polygons.len() < base.build().polygons.len());
    assert_ne!(SmokeParams { phase: 0.5, ..base.clone() }.build().bounds(), base.build().bounds(), "rise phase moves the puffs");
    // Styles differ in tint and density.
    let mean = |s: SmokeStyle| {
        let m = SmokeParams { style: s, fire: 0.0, ..base.clone() }.build();
        m.vertices.values().map(|v| v.color.x).sum::<f32>() / m.vertices.len() as f32
    };
    assert!(mean(SmokeStyle::Battlefield) < mean(SmokeStyle::Chimney) && mean(SmokeStyle::Chimney) < mean(SmokeStyle::Steam));
}

#[test]
fn town_plumes_use_the_ghosttown_ids() {
    let mut m = EditablePoly::new();
    spawn(&mut m, Vec3::new(5.0, 0.0, 2.0), &SmokeParams::default());
    let set = ids(&m);
    assert!(set.iter().all(|&i| i == crate::generators::kit::ids::EMBERS || i == crate::generators::kit::ids::SMOKE), "{set:?}");
    assert!(m.bounds().unwrap().min.z > 0.0);
}

#[test]
fn plume_puffs_feed_the_volume_renderers() {
    let p = SmokeParams { fire: 80.0, ..Default::default() };
    assert_eq!(p.look.render_as, crate::generators::clouds::RenderAs::Volume, "new plumes are volumes");
    let puffs = p.puffs();
    assert_eq!(puffs, p.puffs(), "deterministic");
    let (fire, smoke): (Vec<_>, Vec<_>) = puffs.iter().partition(|q| q.fire);
    assert!(!fire.is_empty() && smoke.len() >= p.lobes as usize, "flame tongues and every lobe");
    let low = |v: &[&PlumePuff]| v.iter().map(|q| q.c.z).fold(f32::MAX, f32::min);
    assert!(low(&fire) < low(&smoke) + 1.0, "fire at the base");
    assert!(puffs.iter().all(|q| q.r > 0.0 && q.sq > 0.0 && (0.05..=1.0).contains(&q.w) && q.c.is_finite()));
    // Thinner as it spreads.
    let top = smoke.iter().max_by(|a, b| a.c.z.total_cmp(&b.c.z)).unwrap();
    assert!(top.w < smoke.iter().map(|q| q.w).fold(0.0, f32::max));
    assert!(p.fire_mat().unwrap().emission > 0.0);
    assert!(SmokeParams { fire: 0.0, ..p.clone() }.fire_mat().is_none());
    assert!(SmokeParams { fire: 0.0, ..p.clone() }.puffs().iter().all(|q| !q.fire));
    // Darker styles absorb more.
    let alb = |s: SmokeStyle| SmokeParams { style: s, ..p.clone() }.smoke_mat().albedo().x;
    assert!(alb(SmokeStyle::Battlefield) < alb(SmokeStyle::Chimney) && alb(SmokeStyle::Chimney) < alb(SmokeStyle::Steam));
}

#[test]
fn plumes_saved_before_render_as_keep_their_mesh() {
    let old: SmokeParams = serde_json::from_str(r#"{"style":"Chimney","height":40.0}"#).unwrap();
    assert_eq!(old.look.render_as, crate::generators::clouds::RenderAs::Mesh);
    assert_eq!(old.height, 40.0);
    let p = SmokeParams::default();
    let back: SmokeParams = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(back, p, "round trip keeps Volume");
}
