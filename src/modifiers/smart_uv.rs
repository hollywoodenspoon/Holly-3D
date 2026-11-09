//! Blender-style Smart UV Project powered by `uvunwrap`: charts are grown by
//! normal angle, flattened (LSCM / planar) and packed into 0..1.

use crate::mesh::EditablePoly;
use crate::mesh::triangulate::triangulate_polygon;
use bevy::math::{Vec2, Vec3};

const PAGE: u32 = 512;

/// Returns per-polygon, per-corner UVs plus the chart count.
pub fn smart_uv_project(m: &EditablePoly, angle_limit: f32, margin: f32, correct_aspect: bool) -> Result<(Vec<Vec<Vec2>>, usize), String> {
    // One input vertex per face corner: uvunwrap welds identical positions,
    // and per-polygon ids keep every corner of a polygon on one chart.
    let mut positions: Vec<[f32; 3]> = vec![];
    let mut indices: Vec<u32> = vec![];
    let mut poly_ids: Vec<u32> = vec![];
    let mut tri_owner: Vec<(usize, [usize; 3])> = vec![];
    let mut corner_counts = vec![];
    for (pi, p) in m.polygons.values().enumerate() {
        let base = positions.len() as u32;
        let pts: Vec<Vec3> = p.vertex_ids().map(|v| m.pos(v)).collect();
        positions.extend(pts.iter().map(|q| q.to_array()));
        corner_counts.push(pts.len());
        for t in triangulate_polygon(&pts) {
            indices.extend(t.iter().map(|&c| base + c as u32));
            poly_ids.push(pi as u32);
            tri_owner.push((pi, t));
        }
    }
    if indices.is_empty() {
        return Err("Nothing to unwrap".into());
    }
    let mut out: Vec<Vec<Vec2>> = corner_counts.iter().map(|&n| vec![Vec2::ZERO; n]).collect();
    let input = uvunwrap::MeshInput { polygons: Some(&poly_ids), ..uvunwrap::MeshInput::new(&positions, &indices) };
    let padding = ((margin.clamp(0.0, 0.25) * PAGE as f32) * 0.5).round() as u32;
    let opts = uvunwrap::Options {
        max_chart_angle: angle_limit.clamp(1.0, 89.0),
        density: uvunwrap::Density::Auto { page_size: PAGE },
        padding: padding.max(1),
        packing: uvunwrap::Packing::Fast,
        ..Default::default()
    };
    let atlas = uvunwrap::generate(&[input], &opts).map_err(|e| format!("Smart UV Unwrap failed: {e}"))?;
    let mesh = &atlas.meshes[0];
    // Page coordinates are normalized per axis; Correct Aspect keeps texels square.
    let (w, h) = (atlas.width.max(1) as f32, atlas.height.max(1) as f32);
    let v_scale = if correct_aspect { h / w } else { 1.0 };
    for (t, &(pi, corners)) in tri_owner.iter().enumerate() {
        for k in 0..3 {
            let ov = &mesh.vertices[mesh.indices[t * 3 + k] as usize];
            // uvunwrap's v grows downward (image rows); 3ds Max UV v points up.
            out[pi][corners[k]] = Vec2::new(ov.uv[0], 1.0 - ov.uv[1] * v_scale);
        }
    }
    Ok((out, atlas.chart_count as usize))
}
