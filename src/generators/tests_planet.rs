//! Planet Studio tests: equal-angle sphere topology, surface preset IDs,
//! per-layer Material IDs (5 clouds, 6 atmosphere, 7 rings) and toggles.

use super::planet_layers::{CLOUD_SHELL, ID_ATMOSPHERE, ID_CLOUDS, ID_RINGS};
use super::planet_surface::{ID_HIGH, ID_ICE, ID_LAND, ID_OCEAN};
use super::sphere::annulus;
use super::*;
use crate::mesh::EditablePoly;

fn ids(m: &EditablePoly) -> std::collections::BTreeSet<u16> {
    m.polygons.values().map(|p| p.material_id).collect()
}

fn bare(preset: SurfacePreset) -> PlanetParams {
    let mut p = PlanetParams { subdivisions: 12, ..PlanetParams::default() }.with_preset(preset);
    (p.clouds.enabled, p.atmosphere.enabled, p.rings.enabled) = (false, false, false);
    p
}

#[test]
fn equal_angle_sphere_is_closed_unit_quad_sphere() {
    for n in [1u32, 4, 9] {
        let m = equal_angle_sphere(n);
        let n = n as usize;
        assert_eq!(m.polygons.len(), 6 * n * n);
        assert_eq!(m.vertices.len(), 6 * n * n + 2);
        assert_eq!(m.edges.len(), 12 * n * n);
        m.validate().unwrap();
        assert!(m.is_closed());
        assert_eq!(m.euler(), 2);
        assert!(m.signed_volume() > 0.0, "outward winding");
        assert!(m.vertices.values().all(|v| (v.position.length() - 1.0).abs() < 1e-4));
        assert!(m.polygons.values().all(|p| p.corners.len() == 4));
    }
}

#[test]
fn sphere_uvs_are_seam_safe() {
    let m = equal_angle_sphere(8);
    for p in m.polygons.values() {
        let us: Vec<f32> = p.corners.iter().map(|c| c.uv.x).collect();
        let span = us.iter().cloned().fold(f32::MIN, f32::max) - us.iter().cloned().fold(f32::MAX, f32::min);
        assert!(span < 0.5, "no polygon stretches across the seam ({span})");
        assert!(p.corners.iter().all(|c| (0.0..=2.0).contains(&c.uv.x) && (0.0..=1.0).contains(&c.uv.y)));
    }
}

#[test]
fn equal_angle_quads_are_near_uniform() {
    let m = equal_angle_sphere(16);
    let areas: Vec<f32> = m.polygons.keys().map(|p| m.poly_normal_raw(p).length()).collect();
    let (lo, hi) = areas.iter().fold((f32::MAX, 0.0f32), |(lo, hi), &a| (lo.min(a), hi.max(a)));
    assert!(hi / lo < 1.6, "equal-angle warp keeps quads even ({})", hi / lo);
}

#[test]
fn surface_presets_use_ids_one_to_four() {
    for s in SurfacePreset::ALL {
        let m = bare(s).build();
        assert_eq!(m.polygons.len(), 6 * 12 * 12, "{s:?}");
        m.validate().unwrap();
        let set = ids(&m);
        assert!(set.iter().all(|i| (ID_OCEAN..=ID_ICE).contains(i)), "{s:?} {set:?}");
        assert!(m.vertices.values().all(|v| v.position.is_finite() && v.color.is_finite()));
    }
    let terran = ids(&bare(SurfacePreset::Terran).build());
    assert!(terran.contains(&ID_OCEAN) && terran.contains(&ID_LAND), "terran has seas and land {terran:?}");
    let ice = ids(&bare(SurfacePreset::Ice).build());
    assert!(ice.contains(&ID_ICE), "{ice:?}");
    let moon = bare(SurfacePreset::Moon).build();
    assert!(moon.vertices.values().any(|v| (v.position.length() - 50.0).abs() > 0.05), "moon craters");
    let _ = ID_HIGH;
}

#[test]
fn layers_toggle_and_carry_their_ids() {
    let p = PlanetParams { subdivisions: 8, ..PlanetParams::default() };
    let mut all = p.clone();
    (all.clouds.enabled, all.atmosphere.enabled, all.rings.enabled) = (true, true, true);
    let full = all.build();
    let set = ids(&full);
    for id in [ID_CLOUDS, ID_ATMOSPHERE, ID_RINGS] {
        assert!(set.contains(&id), "layer {id} present {set:?}");
    }
    full.validate().unwrap();
    // Each toggle removes exactly its own ID.
    let mut no_rings = all.clone();
    no_rings.rings.enabled = false;
    assert!(!ids(&no_rings.build()).contains(&ID_RINGS));
    let mut no_clouds = all.clone();
    no_clouds.clouds.enabled = false;
    assert!(!ids(&no_clouds.build()).contains(&ID_CLOUDS));
    let mut no_air = all.clone();
    no_air.atmosphere.enabled = false;
    assert!(!ids(&no_air.build()).contains(&ID_ATMOSPHERE));
    let mut no_surface = all.clone();
    no_surface.show_surface = false;
    assert!(ids(&no_surface.build()).iter().all(|&i| i >= ID_CLOUDS));
    // Surface + clouds polygon count: two quad spheres.
    let mut two = all;
    (two.atmosphere.enabled, two.rings.enabled) = (false, false);
    assert_eq!(two.build().polygons.len(), 2 * 6 * 8 * 8);
}

#[test]
fn cloud_shell_radius_and_alpha() {
    let p = PlanetParams { subdivisions: 10, radius: 10.0, ..PlanetParams::default() };
    let shell = p.clouds.build(p.seed, 10);
    assert!(shell.vertices.values().all(|v| (v.position.length() - CLOUD_SHELL).abs() < 1e-4));
    let alphas: Vec<f32> = shell.vertices.values().map(|v| v.color.w).collect();
    assert!(alphas.iter().all(|a| (0.0..=1.0).contains(a)));
    assert!(alphas.iter().any(|&a| a > 0.3) && alphas.iter().any(|&a| a < 0.1), "clouds and clear sky");
    let clear = CloudShell { coverage: 0.0, ..p.clouds.clone() }.build(1, 10);
    let mean = |m: &EditablePoly| m.vertices.values().map(|v| v.color.w).sum::<f32>() / m.vertices.len() as f32;
    assert!(mean(&clear) < mean(&shell));
}

#[test]
fn atmosphere_shells_and_glow() {
    let a = Atmosphere { enabled: true, ..Atmosphere::default() };
    let m = a.build(8, bevy::math::Vec3::X);
    assert_eq!(m.polygons.len(), Atmosphere::SHELLS.len() * 6 * 8 * 8);
    assert!(m.polygons.values().all(|p| p.material_id == ID_ATMOSPHERE));
    assert!(m.vertices.values().all(|v| v.color.w > 0.0 && v.color.w <= 1.0));
}

#[test]
fn rings_face_up_tilt_and_stay_in_radius() {
    let (ring, ids_) = annulus(1.0, 2.0, 3, 16);
    assert_eq!(ring.polygons.len(), 3 * 16);
    assert_eq!(ids_.len(), 4 * 16);
    assert!(ring.polygons.keys().all(|p| ring.poly_normal(p).z > 0.99), "annulus faces +Z");
    let r = Rings { enabled: true, tilt: 0.0, ..Rings::default() };
    let m = r.build(1);
    assert!(m.polygons.values().all(|p| p.material_id == ID_RINGS));
    for v in m.vertices.values() {
        let d = v.position.length();
        assert!(d >= r.inner - 1e-3 && d <= r.outer + 1e-3, "ring radius {d}");
        assert!(v.position.z.abs() < 1e-4);
    }
    let tilted = Rings { tilt: 30.0, ..r }.build(1);
    assert!(tilted.vertices.values().any(|v| v.position.z.abs() > 0.1));
}

#[test]
fn custom_surface_is_plain_id_one() {
    let p = PlanetParams { custom_surface: Some("earth.png".into()), ..bare(SurfacePreset::Terran) };
    let m = p.surface_mesh();
    assert!(m.polygons.values().all(|p| p.material_id == 1));
    assert!(m.vertices.values().all(|v| (v.position.length() - 1.0).abs() < 1e-4), "no relief under a texture");
}

#[test]
fn presets_set_layers_and_seed_is_deterministic() {
    let gas = PlanetParams::default().with_preset(SurfacePreset::GasGiant);
    assert!(gas.rings.enabled && !gas.clouds.enabled && gas.relief == 0.0);
    let moon = PlanetParams::default().with_preset(SurfacePreset::Moon);
    assert!(!moon.atmosphere.enabled && !moon.clouds.enabled);
    let a = bare(SurfacePreset::Terran);
    let pos = |m: &EditablePoly| m.vertices.values().map(|v| v.position).collect::<Vec<_>>();
    assert_eq!(pos(&a.build()), pos(&a.build()));
    assert_ne!(pos(&a.build()), pos(&PlanetParams { seed: 9, ..a.clone() }.build()));
    let sun = PlanetParams { sun_azimuth: 0.0, sun_elevation: 90.0, ..a }.sun_dir();
    assert!((sun.z - 1.0).abs() < 1e-5);
}
