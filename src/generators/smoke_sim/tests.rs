use super::*;
use bevy::math::Vec3;

fn small() -> SimParams {
    SimParams { size: [2.0, 2.0, 4.0], resolution: 24, source_radius: 0.35, source_height: 0.4, pressure_iters: 30, gpu: false, ..Default::default() }
}

fn run(p: &SimParams, frames: u32) -> SimState {
    let mut s = SimState::new(p);
    for _ in 0..frames {
        s.step_frame(p);
    }
    s
}

/// Density-weighted mean height (m).
fn centroid_z(s: &SimState) -> f32 {
    let (mut m, mut z) = (0.0, 0.0);
    for (i, d) in s.density.d.iter().enumerate() {
        m += d;
        z += d * (i / (s.n[0] * s.n[1])) as f32;
    }
    z / m.max(1e-6) * s.h
}

fn centroid_x(s: &SimState) -> f32 {
    let (mut m, mut x) = (0.0, 0.0);
    for (i, d) in s.density.d.iter().enumerate() {
        m += d;
        x += d * (i % s.n[0]) as f32;
    }
    x / m.max(1e-6) - (s.n[0] as f32 - 1.0) * 0.5
}

fn finite(s: &SimState) -> bool {
    [&s.density, &s.temperature, &s.fuel, &s.vel[0], &s.vel[1], &s.vel[2]].iter().all(|f| f.d.iter().all(|v| v.is_finite()))
}

#[test]
fn grid_follows_size_and_resolution() {
    let (h, n) = small().grid();
    assert!((h - 4.0 / 24.0).abs() < 1e-6);
    assert_eq!(n, [12, 12, 24]);
}

#[test]
fn projection_removes_most_divergence() {
    let p = small();
    let mut s = SimState::new(&p);
    // A smooth blob of outward flow in the middle of the domain.
    let c = Vec3::new(s.n[0] as f32 - 1.0, s.n[1] as f32 - 1.0, s.n[2] as f32 - 1.0) * 0.5;
    for z in 0..s.n[2] {
        for y in 0..s.n[1] {
            for x in 0..s.n[0] {
                let d = Vec3::new(x as f32, y as f32, z as f32) - c;
                let v = d * (-d.length_squared() / 18.0).exp();
                let i = s.density.idx(x, y, z);
                (s.vel[0].d[i], s.vel[1].d[i], s.vel[2].d[i]) = (v.x, v.y, v.z);
            }
        }
    }
    let norm = |s: &SimState| s.divergence().d.iter().map(|v| v * v).sum::<f32>().sqrt();
    let before = norm(&s);
    s.project(200);
    assert!(norm(&s) * 3.0 < before, "{} -> {}", before, norm(&s));
}

#[test]
fn hot_smoke_rises_higher_than_cold_smoke() {
    let hot = run(&small(), 20);
    let cold = run(&SimParams { buoyancy: 0.0, source_speed: 0.3, ..small() }, 20);
    assert!(finite(&hot) && finite(&cold));
    assert!(hot.density.max() > 0.1);
    assert!(centroid_z(&hot) > centroid_z(&cold) + 0.1, "{} vs {}", centroid_z(&hot), centroid_z(&cold));
}

#[test]
fn same_seed_same_result_new_seed_differs() {
    let a = run(&small(), 8);
    let b = run(&small(), 8);
    assert_eq!(a.density, b.density);
    let c = run(&SimParams { seed: 7, ..small() }, 8);
    assert_ne!(a.density, c.density);
}

#[test]
fn fuel_burns_into_heat_and_soot() {
    let p = SimParams { fuel: 4.0, smoke: 0.0, ..small() };
    let s = run(&p, 16);
    assert!(finite(&s));
    assert!(s.temperature.max() > p.heat * 1.05, "peak {}", s.temperature.max());
    let cold = run(&SimParams { ignition: 100.0, ..p.clone() }, 16);
    let total = |f: &Field| f.d.iter().sum::<f32>();
    assert!(total(&s.fuel) < total(&cold.fuel) * 0.8);
    assert!(total(&s.density) > total(&cold.density));
}

#[test]
fn wind_carries_smoke_downwind() {
    let s = run(&SimParams { wind: [1.5, 0.0], ..small() }, 20);
    assert!(centroid_x(&s) > 0.5, "{}", centroid_x(&s));
}

#[test]
fn swirl_concentrates_vorticity_into_eddies() {
    // Confinement spins up the existing eddies: the strongest one and the
    // squared total (enstrophy) grow with Swirl.
    let measure = |sw: f32| {
        let s = run(&SimParams { swirl: sw, ..small() }, 16);
        let (mut sq, mut peak) = (0.0f32, 0.0f32);
        for z in 1..s.n[2] - 1 {
            for y in 1..s.n[1] - 1 {
                for x in 1..s.n[0] - 1 {
                    let w = s.curl_at(x, y, z).length();
                    (sq, peak) = (sq + w * w, peak.max(w));
                }
            }
        }
        (sq, peak)
    };
    let (calm, swirly) = (measure(0.0), measure(1.0));
    assert!(swirly.0 > calm.0 * 1.1 && swirly.1 > calm.1 * 1.2, "{calm:?} vs {swirly:?}");
}

#[test]
fn values_stay_bounded_over_a_long_run() {
    let s = run(&SimParams { fuel: 2.0, swirl: 1.0, turbulence: 1.5, ..small() }, 48);
    assert!(finite(&s));
    assert!(s.density.max() < 50.0 && s.temperature.max() < 50.0);
    assert!(s.vel.iter().all(|f| f.d.iter().all(|v| v.abs() < 100.0)));
    assert_eq!(s.frame, 48);
}

#[test]
fn bake_fills_frames_the_object_renders() {
    let sim = SimParams { frames: 4, resolution: 16, ..small() };
    let p = SmokeSimParams { sim: sim.clone(), frame: 3, ..Default::default() };
    let b = p.bake();
    let t0 = std::time::Instant::now();
    while !b.done() && t0.elapsed().as_secs() < 60 {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(b.ready(), 4);
    assert_eq!(p.stamp(), 4);
    let direct = run(&sim, 4);
    let (_, f) = b.frame(3).unwrap();
    assert_eq!(f.smoke.data, direct.density.d);
    let parts = p.parts(None);
    assert_eq!(parts.len(), 1, "smoke only without fuel");
    let (lo, hi) = p.bounds();
    assert!(lo.z == 0.0 && (hi.x + lo.x).abs() < 1e-5 && hi.z > 3.9);
    assert_eq!(parts[0].lo, lo);
    // A later frame not baked shows the last ready one.
    assert_eq!(b.frame(99).map(|(k, _)| k), Some(3));
}

#[test]
fn fire_preset_renders_smoke_and_fire_and_saves() {
    let p = SmokeSimParams::fire_preset();
    assert!(p.smoke_mat().is_some() && p.fire_mat().is_some());
    let w = crate::generators::WorldGen::SmokeSim(Box::new(p));
    assert_eq!(w.kind(), crate::generators::WorldKind::FireSim);
    let s = serde_json::to_string(&w).unwrap();
    assert_eq!(serde_json::from_str::<crate::generators::WorldGen>(&s).unwrap(), w);
    let m = w.build();
    m.validate().unwrap();
    assert_eq!(m.polygons.len(), 12 * 6);
}

#[test]
fn simulation_frames_render_to_numbered_pngs() {
    use crate::generators::WorldGen;
    use crate::render::scene_build::RenderScene;
    use crate::render::turntable::{SheetMode, SheetSettings, render_sheet, views};
    use crate::scene::{BaseObject, Scene};
    use crate::viewport::camera::{ViewCamera, ViewKind};
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    let dir = std::env::temp_dir().join(format!("ilex_simseq_{}", std::process::id()));
    let sim = SimParams { resolution: 12, frames: 4, seed: 4242, ..small() };
    let mut scene = Scene::default();
    let id = scene.add(BaseObject::World(WorldGen::SmokeSim(Box::new(SmokeSimParams { sim, frame: 0, ..Default::default() }))), Vec3::ZERO);
    scene.objects[id].evaluate();
    assert_eq!(super::sequence::longest(&scene), 4);
    let mut cam = ViewCamera::new(ViewKind::Perspective);
    cam.frame(Vec3::new(-1.0, -1.0, 0.0), Vec3::new(1.0, 1.0, 4.0));
    let s = SheetSettings { mode: SheetMode::Simulation, start: 2, end: 3, folder: dir.to_string_lossy().into(), name: "sim".into(), ..Default::default() };
    let v = views(&s, Vec3::ZERO, Vec3::ONE, &cam);
    assert_eq!(v.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), ["sim_0002", "sim_0003"]);
    let mut cfg = crate::render::settings::RenderSettings { aa: 1, width: 24, height: 32, ..Default::default() };
    cfg.path.integrator = crate::render::pathtrace::settings::Integrator::Classic;
    let (cancel, done) = (AtomicBool::new(false), AtomicU32::new(0));
    let mut shown = Vec::new();
    let rs = RenderScene::build(&scene, cam, true);
    let prep = |i: usize, rs: &mut RenderScene| {
        let k = s.start - 1 + i as u32;
        let ok = super::sequence::show_frame(&mut scene, k, &cancel);
        rs.volumes = crate::render::volumes::Volumes::build(&scene);
        if let BaseObject::World(WorldGen::SmokeSim(p)) = &scene.objects[id].base {
            shown.push(p.bake().frame(p.frame).map(|(f, _)| f));
        }
        ok
    };
    render_sheet(rs, &cfg, &s, v, prep, &cancel, &done).unwrap();
    assert_eq!(done.load(Ordering::Relaxed), 2);
    assert_eq!(shown, [Some(1), Some(2)], "each render waits for its own baked frame");
    for n in ["sim_0002", "sim_0003"] {
        let img = image::open(dir.join(format!("{n}.png"))).unwrap();
        assert_eq!((img.width(), img.height()), (24, 32));
    }
    let _ = std::fs::remove_dir_all(&dir);
}
