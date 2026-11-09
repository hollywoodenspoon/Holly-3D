use super::texture::fill_texture;
use super::vertex::fill;
use super::*;
use crate::materials::Texture;
use crate::mesh::EditablePoly;
use crate::modifiers::VertexPaintMod;
use crate::primitives::{Primitive, PrimitiveKind};
use bevy::math::{Mat4, Vec2, Vec3, Vec4};
use std::collections::HashMap;

fn plane() -> EditablePoly {
    // 2x2 plane (4x4 quads), UVs 0..1, Z up.
    Primitive::default_of(PrimitiveKind::Plane).to_editable_poly()
}

#[test]
fn falloff_is_hard_inside_and_zero_outside() {
    assert_eq!(falloff_weight(0.0, 1.0, 0.5), 1.0);
    assert_eq!(falloff_weight(0.49, 1.0, 0.5), 1.0);
    let mid = falloff_weight(0.75, 1.0, 0.5);
    assert!(mid > 0.0 && mid < 1.0);
    assert_eq!(falloff_weight(1.0, 1.0, 0.5), 0.0);
    assert_eq!(falloff_weight(0.99, 1.0, 0.0), 1.0, "falloff 0 is a hard brush");
}

#[test]
fn vertex_stroke_paints_only_inside_brush_and_respects_opacity() {
    let m = plane();
    let (v, e, p) = m.counts();
    let mut layer = HashMap::new();
    let mut s = VertexStroke::new(&m, Mat4::IDENTITY, &layer, false);
    let center = m.vertices.values().map(|v| v.position).fold(Vec3::splat(f32::MAX), |a, b| a.min(b));
    let brush = Brush { size: 0.1, opacity: 0.5, falloff: 0.0, color: [1.0, 0.0, 0.0], tool: BrushTool::Paint };
    // Two stamps at the same point: per-stroke opacity does not accumulate.
    assert_eq!(s.stamp(&mut layer, center, &brush), 1);
    s.stamp(&mut layer, center, &brush);
    assert_eq!(layer.len(), 1);
    let l = *layer.values().next().unwrap();
    assert!((l.w - 0.5).abs() < 1e-5 && (l.x - 1.0).abs() < 1e-5);
    // Modifier composite: half way between white and red, topology intact.
    let mut out = m.clone();
    VertexPaintMod { layer: layer.clone(), layer_opacity: 1.0 }.apply(&mut out);
    assert_eq!(out.counts(), (v, e, p));
    let painted = out.vertices.values().find(|x| x.color != Vec4::ONE).unwrap();
    assert!((painted.color - Vec4::new(1.0, 0.5, 0.5, 1.0)).length() < 1e-4);
    // Erase removes the paint again.
    let mut s = VertexStroke::new(&m, Mat4::IDENTITY, &layer, false);
    s.stamp(&mut layer, center, &Brush { tool: BrushTool::Erase, opacity: 1.0, ..brush });
    assert!(layer.is_empty());
}

#[test]
fn blur_and_fill_and_mask() {
    let mut m = plane();
    let mut layer = HashMap::new();
    assert_eq!(fill(&m, &mut layer, [0.0, 0.0, 1.0], 1.0, false), m.vertices.len());
    assert!(layer.values().all(|l| (l.w - 1.0).abs() < 1e-5 && l.z > 0.99));
    // Blur a single black vertex among blue neighbours: it moves towards blue.
    let k = *layer.keys().next().unwrap();
    layer.insert(k, Vec4::new(0.0, 0.0, 0.0, 1.0));
    let pos = m.vertices[k].position;
    let mut s = VertexStroke::new(&m, Mat4::IDENTITY, &layer, false);
    s.stamp(&mut layer, pos, &Brush { size: 0.05, falloff: 0.0, tool: BrushTool::Blur, ..Brush::default() });
    assert!(layer[&k].z > 0.2);
    // Selected Faces Only: fill touches only that polygon's corners.
    let pid = m.polygons.keys().next().unwrap();
    m.polygons[pid].selected = true;
    let mut masked = HashMap::new();
    assert_eq!(fill(&m, &mut masked, [1.0; 3], 1.0, true), m.polygons[pid].corners.len());
}

#[test]
fn uv_overlap_detects_stacked_box_faces() {
    let plane_overlap = uv_overlap(&plane());
    assert!(plane_overlap < 0.02, "plane UVs are a clean 0..1 layout ({plane_overlap})");
    let boxm = Primitive::default_of(PrimitiveKind::Box).to_editable_poly();
    assert!(uv_overlap(&boxm) > 0.5, "box faces all map to 0..1");
    let mut flat = plane();
    for p in flat.polygons.values_mut() {
        p.corners.iter_mut().for_each(|c| c.uv = Vec2::ZERO);
    }
    assert_eq!(uv_overlap(&flat), 1.0, "no UVs at all");
}

#[test]
fn texture_stamp_projects_through_uvs() {
    let m = plane();
    let tris = paint_tris(&m, Mat4::IDENTITY, false);
    assert_eq!(tris.len(), m.polygons.len() * 2);
    let mut tex = Texture::new(64, 64, [255; 4]);
    let mut s = TextureStroke::new(tris, &tex);
    let b = m.bounds().unwrap();
    let brush = Brush { size: b.size().x * 0.1, opacity: 1.0, falloff: 0.0, color: [0.0, 0.0, 0.0], tool: BrushTool::Paint };
    let hit = s.stamp(&mut tex, b.center(), Vec3::NEG_Z, &brush);
    assert!(hit > 0);
    // Plane center maps to UV (0.5, 0.5) -> texel (32, 32) is black.
    assert_eq!(tex.texel(32, 32), [0, 0, 0, 255]);
    assert_eq!(tex.texel(2, 2), [255; 4], "far texels untouched");
    // Back-facing view paints nothing.
    let mut t2 = Texture::new(64, 64, [255; 4]);
    let mut s2 = TextureStroke::new(paint_tris(&m, Mat4::IDENTITY, false), &t2);
    assert_eq!(s2.stamp(&mut t2, b.center(), Vec3::Z, &brush), 0);
    fill_texture(&mut t2, [0.0, 1.0, 0.0], 1.0);
    assert_eq!(t2.texel(5, 9), [0, 255, 0, 255]);
}
