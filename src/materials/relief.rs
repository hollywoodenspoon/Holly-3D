//! Surface relief: the tangent-space normal a material asks for at a point
//! (from its procedural surface and its Bump / Normal map), and how to
//! apply it to a shading normal. Shared by the raytracer, the sample-slot
//! previews and the viewport normal-map bake.

use super::maps::{MapChannel, MapNode};
use super::proc_engine::{filters::unpack_normal, surface};
use super::Material;
use bevy::math::{Vec2, Vec3};

/// Height-map slope to normal tilt: a 0..1 height map at Amount 100 gives
/// the same normals as a Normal From Height node at intensity 1.
const BUMP_SCALE: f32 = 1.0 / 32.0;

/// The tangent-space normal at `uv` (`None` = smooth). `dpdu` / `dpdv` say
/// how the object-space point moves with U and V, so 3D bump maps work.
pub fn tangent_normal(m: &Material, uv: Vec2, p: Vec3, dpdu: Vec3, dpdv: Vec3) -> Option<Vec3> {
    let base = base_tangent_normal(m, uv, p, dpdu, dpdv);
    match &m.paint {
        Some(paint) => paint.tangent_normal(uv, p, base),
        None => base,
    }
}

/// [`tangent_normal`] of the material alone, without its paint layers.
pub fn base_tangent_normal(m: &Material, uv: Vec2, p: Vec3, dpdu: Vec3, dpdv: Vec3) -> Option<Vec3> {
    let mut n = None;
    if m.proc.auto_pbr && m.proc.relief > 0.0 {
        n = surface::maps(&m.proc, m.roughness, m.metallic).map(|pm| unpack_normal(pm.normal.sample(uv)));
    }
    if let Some((node, amount)) = m.active_map(MapChannel::Bump) {
        let t = match node {
            // Already a normal map.
            MapNode::NormalFromHeight { .. } => unpack_normal(node.sample(uv, p)),
            MapNode::Bitmap { path, .. } if is_normal_map(path) => unpack_normal(node.sample(uv, p)),
            // A height map: central differences.
            _ => {
                let e = 1.0 / 512.0;
                let h = |du: f32, dv: f32| node.sample_scalar(uv + Vec2::new(du, dv), p + dpdu * du + dpdv * dv);
                let (dx, dy) = ((h(e, 0.0) - h(-e, 0.0)) / (2.0 * e), (h(0.0, e) - h(0.0, -e)) / (2.0 * e));
                Vec3::new(-dx * BUMP_SCALE, -dy * BUMP_SCALE, 1.0).normalize()
            }
        };
        let t = Vec3::Z.lerp(t, amount).normalize_or(Vec3::Z);
        n = Some(add_tilt(n, t));
    }
    if m.brush.enabled {
        n = Some(add_tilt(n, m.brush.tangent_normal(uv)));
    }
    n
}

/// Detail normals add their tilt to the base normal.
fn add_tilt(base: Option<Vec3>, t: Vec3) -> Vec3 {
    base.map_or(t, |b| Vec3::new(b.x + t.x, b.y + t.y, b.z).normalize_or(Vec3::Z))
}

/// True when a bitmap's file name says it is a normal map ("nor.jpg",
/// "wall_nor_gl_1k.jpg", "bricks_normal.png", "x_nrm.png"), not a height map.
pub fn is_normal_map(path: &str) -> bool {
    let name = path.rsplit(['/', '\\']).next().unwrap_or(path).to_lowercase();
    let stem = name.rsplit_once('.').map_or(name.as_str(), |(s, _)| s);
    stem.split(['_', '-', ' ', '.']).any(|w| matches!(w, "nor" | "nrm" | "normal" | "normalgl" | "normals"))
}

/// Tilts shading normal `n` by tangent-space normal `tn` in the frame
/// spanned by `dpdu` and `dpdv`.
pub fn perturb(n: Vec3, dpdu: Vec3, dpdv: Vec3, tn: Vec3) -> Vec3 {
    let t = (dpdu - n * n.dot(dpdu)).normalize_or_zero();
    if t == Vec3::ZERO {
        return n;
    }
    let b = (dpdv - n * n.dot(dpdv) - t * t.dot(dpdv)).normalize_or(n.cross(t));
    (t * tn.x + b * tn.y + n * tn.z).normalize_or(n)
}

/// Shading normal of a World / Triplanar material: the relief of each
/// projection tilts `n` in that projection's frame (turned to world space
/// by `rot`), blended like the colours. `tp` / `tn` are the point and
/// normal in projection space.
pub fn triplanar_normal(m: &Material, tp: Vec3, tn: Vec3, n: Vec3, rot: bevy::math::Quat) -> Vec3 {
    let frames = m.projection.frames();
    m.projection
        .blend(tp, tn, |uv, k| {
            let (du, dv) = frames[k];
            base_tangent_normal(m, uv, tp, du, dv).map_or(n, |t| perturb(n, rot * du, rot * dv, t))
        })
        .normalize_or(n)
}

/// How a triangle's points move with U and V (`None` without usable UVs).
pub fn uv_frame(p: [Vec3; 3], uv: [Vec2; 3]) -> Option<(Vec3, Vec3)> {
    let (e1, e2) = (p[1] - p[0], p[2] - p[0]);
    let (d1, d2) = (uv[1] - uv[0], uv[2] - uv[0]);
    let det = d1.x * d2.y - d2.x * d1.y;
    (det.abs() > 1e-12).then(|| ((e1 * d2.y - e2 * d1.y) / det, (e2 * d1.x - e1 * d2.x) / det))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::materials::proc_engine::{Pattern, ProcSurface, TileKind};

    #[test]
    fn frames_follow_the_uvs_and_perturbing_tilts_towards_the_tangent() {
        // A quad in XZ whose U runs along X and V along -Z.
        let (dpdu, dpdv) = uv_frame([Vec3::ZERO, Vec3::X * 2.0, Vec3::NEG_Z * 4.0], [Vec2::ZERO, Vec2::X, Vec2::Y]).unwrap();
        assert!((dpdu - Vec3::X * 2.0).length() < 1e-5 && (dpdv - Vec3::NEG_Z * 4.0).length() < 1e-5);
        assert!(uv_frame([Vec3::ZERO, Vec3::X, Vec3::Y], [Vec2::ZERO; 3]).is_none(), "no UVs, no frame");
        let n = perturb(Vec3::Y, dpdu, dpdv, Vec3::new(0.6, 0.0, 0.8));
        assert!((n - Vec3::new(0.6, 0.8, 0.0)).length() < 1e-5, "{n}");
        assert_eq!(perturb(Vec3::Y, dpdu, dpdv, Vec3::Z), Vec3::Y, "a flat normal map changes nothing");
        assert_eq!(perturb(Vec3::Y, Vec3::ZERO, dpdv, Vec3::new(0.6, 0.0, 0.8)), Vec3::Y);
    }

    #[test]
    fn plain_materials_are_smooth_and_bump_maps_tilt_the_normal() {
        let (du, dv) = (Vec3::X * 2.0, Vec3::Y * 2.0);
        let mut m = Material::default();
        assert!(tangent_normal(&m, Vec2::splat(0.3), Vec3::ZERO, du, dv).is_none());
        // A brick height map: flat on a brick face, tilted on its bevel.
        m.map_mut(MapChannel::Bump).node = Some(MapNode::tile(TileKind::Bricks, [0.0; 3], [1.0; 3]));
        let face = tangent_normal(&m, Vec2::new(0.125, 0.0625), Vec3::ZERO, du, dv).unwrap();
        assert!(face.z > 0.999, "{face}");
        let bevel = tangent_normal(&m, Vec2::new(0.125, 0.0625 - 0.047), Vec3::ZERO, du, dv).unwrap();
        assert!(bevel.z < 0.95 && bevel.y < -0.2, "height rising towards +V tips the normal to -V: {bevel}");
        // Less Amount, less tilt.
        m.map_mut(MapChannel::Bump).amount = 20.0;
        assert!(tangent_normal(&m, Vec2::new(0.125, 0.0625 - 0.047), Vec3::ZERO, du, dv).unwrap().z > bevel.z);
        // A Normal From Height node on the slot is used as a normal map.
        m.map_mut(MapChannel::Bump).amount = 100.0;
        m.map_mut(MapChannel::Bump).node = Some(MapNode::new_kind(14, None));
        assert!(tangent_normal(&m, Vec2::new(0.125, 0.0625 - 0.047), Vec3::ZERO, du, dv).unwrap().z < 0.99);
    }

    #[test]
    fn procedural_surfaces_derive_their_normals_only_when_asked() {
        let mut m = Material { proc: ProcSurface::styled(Pattern::Bricks, 0), ..Default::default() };
        let tilted = (0..200).filter_map(|i| tangent_normal(&m, Vec2::new(i as f32 * 0.618 % 1.0, i as f32 * 0.137 % 1.0), Vec3::ZERO, Vec3::X, Vec3::Y)).filter(|n| n.z < 0.999).count();
        assert!(tilted > 10, "{tilted} samples on bevels");
        m.proc.auto_pbr = false;
        assert!(tangent_normal(&m, Vec2::splat(0.3), Vec3::ZERO, Vec3::X, Vec3::Y).is_none());
    }
}
