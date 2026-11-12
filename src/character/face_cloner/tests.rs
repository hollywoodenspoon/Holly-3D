use super::correspond;
use super::math::{jacobi, similarity, solve_ridge};
use super::model;
use crate::character::makehuman::obj::base;
use bevy::math::{Quat, Vec3};

#[test]
fn solvers_recover_known_answers() {
    let (vals, vecs) = jacobi(vec![2.0, 1.0, 1.0, 2.0], 2);
    let mut v = vals.clone();
    v.sort_by(f64::total_cmp);
    assert!((v[0] - 1.0).abs() < 1e-9 && (v[1] - 3.0).abs() < 1e-9);
    assert!((vecs[0].abs() - 0.5f64.sqrt()).abs() < 1e-9);
    let rot = Quat::from_euler(bevy::math::EulerRot::YXZ, 0.4, -0.2, 0.1);
    let src: Vec<Vec3> = (0..12).map(|i| Vec3::new((i as f32 * 1.3).sin(), (i as f32 * 0.7).cos(), i as f32 * 0.1)).collect();
    let dst: Vec<Vec3> = src.iter().map(|&p| rot * p * 2.5 + Vec3::new(1.0, -2.0, 0.5)).collect();
    let s = similarity(&src, &dst, &[1.0; 12], None);
    assert!((s.scale - 2.5).abs() < 1e-4, "{s:?}");
    assert!(src.iter().zip(&dst).all(|(&a, &b)| s.apply(a).distance(b) < 1e-4));
    // x + y = 3, x - y = 1 -> (2, 1)
    let x = solve_ridge(vec![2.0, 0.0, 0.0, 2.0], vec![4.0, 2.0], 2, 0.0).unwrap();
    assert!((x[0] - 2.0).abs() < 1e-12 && (x[1] - 1.0).abs() < 1e-12);
}

#[test]
fn bundled_face_model_loads_with_its_canonical_mesh() {
    assert!(model::available());
    let c = model::canonical();
    assert_eq!(c.len(), model::MESH_POINTS);
    // Subject's left eye corner is +x, the nose tip is the most forward point.
    assert!(c[263].x > 0.0 && c[33].x < 0.0);
    let front = (0..c.len()).max_by(|&a, &b| c[a].z.total_cmp(&c[b].z)).unwrap();
    assert!(c[front].distance(c[correspond::NOSE_TIP]) < 0.5);
}

#[test]
fn canonical_face_lands_on_the_hm08_head() {
    let b = base();
    let cor = correspond::get().expect("correspondence");
    assert_eq!(cor.verts.len(), model::MESH_POINTS);
    let mean_gap = cor.gap.iter().sum::<f32>() / cor.gap.len() as f32;
    assert!(mean_gap < 0.06, "canonical face should hug the head (mean gap {mean_gap} dm)");
    let p = |i: usize| b.coords[cor.verts[i] as usize];
    // Left/right pairs mirror across x = 0 at about the same height.
    for (l, r) in [(263, 33), (362, 133), (291, 61), (454, 234)] {
        let (a, c) = (p(l), p(r));
        assert!(a.x > 0.0 && c.x < 0.0, "{l}/{r}");
        assert!((a.x + c.x).abs() < 0.08 && (a.y - c.y).abs() < 0.08, "{l}/{r}: {a} {c}");
    }
    // Top to bottom: forehead, eyes, nose tip, mouth, chin.
    let ys = [10, 168, 4, 13, 152].map(|i| p(i).y);
    assert!(ys.windows(2).all(|w| w[0] > w[1]), "{ys:?}");
    let distinct: std::collections::HashSet<_> = cor.verts.iter().collect();
    assert!(distinct.len() > 380, "most points should get their own vertex ({})", distinct.len());
}

/// A front view of the textured base head, centred between the eyes.
pub fn head_view(yaw: f32) -> (super::render::View, image::RgbImage) {
    use crate::character::makehuman::library;
    let b = base();
    let eyes = correspond::group_verts(b, "helper-l-eye").into_iter().chain(correspond::group_verts(b, "helper-r-eye"));
    let (sum, n) = eyes.fold((Vec3::ZERO, 0.0), |(s, n), v| (s + b.coords[v as usize], n + 1.0));
    let view = super::render::View { center: sum / n - Vec3::Y * 0.25, yaw, pitch: 0.0, scale: 170.0, size: 512 };
    let mm = library::material("skins/young_caucasian_female/young_caucasian_female.mhmat").unwrap_or_default();
    let tex = mm.diffuse_texture.and_then(|t| library::texture(&format!("skins/young_caucasian_female/{t}")));
    let img = super::render::render(b, &b.coords, &view, tex.as_ref().map(|t| &*t.tex));
    (view, img)
}

#[test]
fn detector_finds_the_rendered_base_head_where_the_correspondence_says() {
    let cor = correspond::get().unwrap();
    let b = base();
    let (view, img) = head_view(0.0);
    if let Ok(dir) = std::env::var("FACE_DEBUG_DIR") {
        let _ = img.save(format!("{dir}/head_front.png"));
    }
    let det = model::detect_image(&img).expect("MediaPipe should find the rendered MakeHuman face");
    assert_eq!(det.points.len(), model::POINTS);
    let iod = det.points[33].truncate().distance(det.points[263].truncate());
    let err: f32 = (0..model::MESH_POINTS).map(|i| view.project(b.coords[cor.verts[i] as usize]).truncate().distance(det.points[i].truncate())).sum::<f32>() / model::MESH_POINTS as f32;
    eprintln!("score {} interocular {iod:.1}px mean error {err:.2}px ({:.1}% IOD)", det.score, err / iod * 100.0);
    assert!(err / iod < 0.12, "detected points should sit on the corresponding vertices ({err} px, IOD {iod})");
}
