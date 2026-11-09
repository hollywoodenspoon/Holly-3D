//! FBX import through `ufbx` (binary and ASCII, any version). The scene is
//! converted to Z-up metres; polygons stay n-gons. 3ds Max smoothing-group
//! bitmasks come from [`super::fbx_smoothing`] (ufbx only reports a smooth /
//! faceted flag per face).

use super::{MatInfo, MeshItem};
use crate::mesh::{EditablePoly, FaceCorner, Polygon, VertexId};
use bevy::math::{DMat4, DVec4, Mat4, Vec2, Vec3, Vec4};
use std::collections::HashMap;
use std::path::Path;

fn mat4(m: &ufbx::Matrix) -> DMat4 {
    DMat4::from_cols(
        DVec4::new(m.m00, m.m10, m.m20, 0.0),
        DVec4::new(m.m01, m.m11, m.m21, 0.0),
        DVec4::new(m.m02, m.m12, m.m22, 0.0),
        DVec4::new(m.m03, m.m13, m.m23, 1.0),
    )
}

fn material_info(m: &ufbx::Material) -> MatInfo {
    let f = &m.fbx;
    let c = f.diffuse_color.value_vec4;
    let e = f.emission_color.value_vec4;
    let opacity = if m.pbr.opacity.has_value { m.pbr.opacity.value_vec4.x } else { 1.0 - f.transparency_factor.value_vec4.x };
    MatInfo {
        name: m.element.name.to_string(),
        base_color: [c.x as f32, c.y as f32, c.z as f32],
        opacity: opacity.clamp(0.0, 1.0) as f32,
        metallic: m.pbr.metalness.value_vec4.x.clamp(0.0, 1.0) as f32,
        roughness: if m.pbr.roughness.has_value { m.pbr.roughness.value_vec4.x.clamp(0.0, 1.0) as f32 } else { 0.5 },
        emissive: [e.x as f32, e.y as f32, e.z as f32],
    }
}

pub fn import(path: &Path) -> Result<Vec<MeshItem>, String> {
    let opts = ufbx::LoadOpts {
        target_axes: ufbx::CoordinateAxes::right_handed_z_up(),
        target_unit_meters: 1.0,
        space_conversion: ufbx::SpaceConversion::AdjustTransforms,
        ..Default::default()
    };
    let masks = std::fs::read(path).map(|b| super::fbx_smoothing::read(&b)).unwrap_or_default();
    let scene = ufbx::load_file(path.to_str().ok_or("Invalid path")?, opts).map_err(|e| format!("FBX: {}", e.description))?;
    let mut out = vec![];
    for node in scene.nodes.iter() {
        let Some(mesh) = node.mesh.as_ref() else { continue };
        let to_world = mat4(&node.node_to_world);
        let geo = to_world.inverse() * mat4(&node.geometry_to_world);
        let sg = masks.get(mesh.element.name.as_ref()).filter(|m| m.len() == mesh.num_faces);
        let (mesh_out, materials) = read_mesh(mesh, geo.as_mat4(), sg);
        if !mesh_out.polygons.is_empty() {
            out.push(MeshItem { name: node.element.name.to_string(), mesh: mesh_out, transform: to_world.as_mat4(), materials });
        }
    }
    Ok(out)
}

fn read_mesh(mesh: &ufbx::Mesh, geo: Mat4, masks: Option<&Vec<u32>>) -> (EditablePoly, Vec<MatInfo>) {
    let mut out = EditablePoly::new();
    let ids: Vec<VertexId> =
        mesh.vertices.iter().map(|p| out.add_vertex(geo.transform_point3(Vec3::new(p.x as f32, p.y as f32, p.z as f32)))).collect();
    let (uv, col) = (&mesh.vertex_uv, &mesh.vertex_color);
    let mut colored: HashMap<usize, Vec4> = HashMap::new();
    for (f, face) in mesh.faces.iter().enumerate() {
        let mut corners = vec![];
        for i in face.index_begin..face.index_begin + face.num_indices {
            let i = i as usize;
            let vi = mesh.vertex_indices[i] as usize;
            let t = if uv.exists { uv.values[uv.indices[i] as usize] } else { Default::default() };
            if col.exists {
                let c = col.values[col.indices[i] as usize];
                colored.insert(vi, Vec4::new(c.x as f32, c.y as f32, c.z as f32, c.w as f32));
            }
            corners.push(FaceCorner::new(ids[vi], Vec2::new(t.x as f32, t.y as f32)));
        }
        if corners.len() < 3 {
            continue;
        }
        let mut t = Polygon::new(vec![]);
        t.material_id = mesh.face_material.get(f).map_or(1, |&m| m as u16 + 1);
        t.smoothing_group = match masks {
            Some(m) => m[f],
            None => mesh.face_smoothing.get(f).map_or(1, |&s| s as u32),
        };
        out.add_polygon_corners(corners, &t);
    }
    for (vi, c) in colored {
        out.vertices[ids[vi]].color = c;
    }
    out.remove_isolated_vertices();
    out.finalize();
    let materials = mesh.materials.iter().map(|m| material_info(m)).collect();
    (out, materials)
}
