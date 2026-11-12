//! Autodesk FBX export: ASCII FBX 7.4, Z-up like 3ds Max, 1 unit = 1 metre.
//! Geometry keeps its polygons (n-gons), per-polygon smoothing-group
//! bitmasks (the 3ds Max convention), UVs, vertex colours and material IDs.

use super::{MatInfo, MeshItem};
use crate::mesh::VertexId;
use bevy::math::EulerRot;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;

pub fn export(path: &Path, items: &[MeshItem]) -> Result<(), String> {
    std::fs::write(path, write_string(items)).map_err(|e| e.to_string())
}

fn array<T: std::fmt::Display>(o: &mut String, indent: &str, name: &str, data: &[T]) {
    let joined: Vec<String> = data.iter().map(|v| v.to_string()).collect();
    let _ = writeln!(o, "{indent}{name}: *{} {{\n{indent}\ta: {}\n{indent}}}", data.len(), joined.join(","));
}

fn layer_element(o: &mut String, kind: &str, version: u32, name: &str, mapping: &str, reference: &str, body: impl FnOnce(&mut String)) {
    let _ = writeln!(o, "\t\t{kind}: 0 {{\n\t\t\tVersion: {version}\n\t\t\tName: \"{name}\"");
    let _ = writeln!(o, "\t\t\tMappingInformationType: \"{mapping}\"\n\t\t\tReferenceInformationType: \"{reference}\"");
    body(o);
    o.push_str("\t\t}\n");
}

fn geometry(o: &mut String, id: u64, item: &MeshItem, mat_index: &HashMap<u16, usize>) {
    let m = &item.mesh;
    let index: HashMap<VertexId, i64> = m.vertices.keys().enumerate().map(|(i, k)| (k, i as i64)).collect();
    let verts: Vec<f64> = m.vertices.values().flat_map(|v| v.position.to_array().map(f64::from)).collect();
    let (mut pvi, mut uv, mut col, mut sg, mut mats) = (vec![], vec![], vec![], vec![], vec![]);
    for p in m.polygons.values() {
        let n = p.corners.len();
        for (i, c) in p.corners.iter().enumerate() {
            let vi = index[&c.vertex_id];
            pvi.push(if i + 1 == n { -vi - 1 } else { vi });
            uv.extend([c.uv.x as f64, c.uv.y as f64]);
            col.extend(m.vertices[c.vertex_id].color.to_array().map(f64::from));
        }
        sg.push(p.smoothing_group as i64);
        mats.push(mat_index.get(&p.material_id.max(1)).copied().unwrap_or(0) as i64);
    }
    let corner_idx: Vec<i64> = (0..pvi.len() as i64).collect();
    let _ = writeln!(o, "\tGeometry: {id}, \"Geometry::{}\", \"Mesh\" {{", item.name);
    array(o, "\t\t", "Vertices", &verts);
    array(o, "\t\t", "PolygonVertexIndex", &pvi);
    o.push_str("\t\tGeometryVersion: 124\n");
    layer_element(o, "LayerElementSmoothing", 102, "", "ByPolygon", "Direct", |o| array(o, "\t\t\t", "Smoothing", &sg));
    layer_element(o, "LayerElementColor", 101, "Col", "ByPolygonVertex", "IndexToDirect", |o| {
        array(o, "\t\t\t", "Colors", &col);
        array(o, "\t\t\t", "ColorIndex", &corner_idx);
    });
    layer_element(o, "LayerElementUV", 101, "UVChannel_1", "ByPolygonVertex", "IndexToDirect", |o| {
        array(o, "\t\t\t", "UV", &uv);
        array(o, "\t\t\t", "UVIndex", &corner_idx);
    });
    layer_element(o, "LayerElementMaterial", 101, "", "ByPolygon", "IndexToDirect", |o| array(o, "\t\t\t", "Materials", &mats));
    o.push_str("\t\tLayer: 0 {\n\t\t\tVersion: 100\n");
    for kind in ["LayerElementSmoothing", "LayerElementColor", "LayerElementUV", "LayerElementMaterial"] {
        let _ = writeln!(o, "\t\t\tLayerElement:  {{\n\t\t\t\tType: \"{kind}\"\n\t\t\t\tTypedIndex: 0\n\t\t\t}}");
    }
    o.push_str("\t\t}\n\t}\n");
}

fn model(o: &mut String, id: u64, item: &MeshItem) {
    let (s, r, t) = item.transform.to_scale_rotation_translation();
    let (rx, ry, rz) = r.to_euler(EulerRot::XYZEx);
    let _ = writeln!(o, "\tModel: {id}, \"Model::{}\", \"Mesh\" {{\n\t\tVersion: 232\n\t\tProperties70:  {{", item.name);
    let _ = writeln!(o, "\t\t\tP: \"Lcl Translation\", \"Lcl Translation\", \"\", \"A\",{},{},{}", t.x, t.y, t.z);
    let _ =
        writeln!(o, "\t\t\tP: \"Lcl Rotation\", \"Lcl Rotation\", \"\", \"A\",{},{},{}", rx.to_degrees(), ry.to_degrees(), rz.to_degrees());
    let _ = writeln!(o, "\t\t\tP: \"Lcl Scaling\", \"Lcl Scaling\", \"\", \"A\",{},{},{}", s.x, s.y, s.z);
    o.push_str("\t\t}\n\t\tShading: T\n\t\tCulling: \"CullingOff\"\n\t}\n");
}

fn material(o: &mut String, id: u64, name: &str, m: &MatInfo) {
    let [r, g, b] = m.base_color;
    let [er, eg, eb] = m.emissive;
    let _ =
        writeln!(o, "\tMaterial: {id}, \"Material::{name}\", \"\" {{\n\t\tVersion: 102\n\t\tShadingModel: \"phong\"\n\t\tMultiLayer: 0");
    o.push_str("\t\tProperties70:  {\n");
    let _ = writeln!(o, "\t\t\tP: \"DiffuseColor\", \"Color\", \"\", \"A\",{r},{g},{b}");
    let _ = writeln!(o, "\t\t\tP: \"DiffuseFactor\", \"Number\", \"\", \"A\",1");
    let _ = writeln!(o, "\t\t\tP: \"EmissiveColor\", \"Color\", \"\", \"A\",{er},{eg},{eb}");
    let _ = writeln!(o, "\t\t\tP: \"EmissiveFactor\", \"Number\", \"\", \"A\",1");
    let _ = writeln!(o, "\t\t\tP: \"TransparencyFactor\", \"Number\", \"\", \"A\",{}", 1.0 - m.opacity);
    let _ = writeln!(o, "\t\t\tP: \"Opacity\", \"double\", \"Number\", \"\",{}", m.opacity);
    let shininess = ((1.0 - m.roughness).powi(2) * 100.0).max(2.0);
    let _ = writeln!(o, "\t\t\tP: \"Shininess\", \"double\", \"Number\", \"\",{shininess}");
    let _ = writeln!(o, "\t\t\tP: \"ReflectionFactor\", \"Number\", \"\", \"A\",{}", m.metallic);
    o.push_str("\t\t}\n\t}\n");
}

/// The whole ASCII FBX document.
pub fn write_string(items: &[MeshItem]) -> String {
    let mut o = String::from("; FBX 7.4.0 project file\n; Created by Ilex\n");
    o.push_str("FBXHeaderExtension:  {\n\tFBXHeaderVersion: 1003\n\tFBXVersion: 7400\n\tCreator: \"Ilex\"\n}\n");
    o.push_str("GlobalSettings:  {\n\tVersion: 1000\n\tProperties70:  {\n");
    for (k, v) in [("UpAxis", 2), ("UpAxisSign", 1), ("FrontAxis", 1), ("FrontAxisSign", -1), ("CoordAxis", 0), ("CoordAxisSign", 1)] {
        let _ = writeln!(o, "\t\tP: \"{k}\", \"int\", \"Integer\", \"\",{v}");
    }
    o.push_str("\t\tP: \"UnitScaleFactor\", \"double\", \"Number\", \"\",100\n\t}\n}\n");
    let mats: usize = items.iter().map(|i| i.material_ids().len()).sum();
    let _ = writeln!(
        o,
        "Definitions:  {{\n\tVersion: 100\n\tCount: {}\n\tObjectType: \"Model\" {{\n\t\tCount: {}\n\t}}\n\tObjectType: \"Geometry\" {{\n\t\tCount: {}\n\t}}\n\tObjectType: \"Material\" {{\n\t\tCount: {mats}\n\t}}\n}}",
        items.len() * 2 + mats,
        items.len(),
        items.len()
    );
    o.push_str("Objects:  {\n");
    let mut conns = String::from("Connections:  {\n");
    let mut next = 1_000_000u64;
    let mut id = || {
        next += 1;
        next
    };
    for item in items {
        let (gid, mid) = (id(), id());
        let ids = item.material_ids();
        let mat_index: HashMap<u16, usize> = ids.iter().enumerate().map(|(i, &m)| (m, i)).collect();
        geometry(&mut o, gid, item, &mat_index);
        model(&mut o, mid, item);
        let _ = writeln!(conns, "\tC: \"OO\",{mid},0\n\tC: \"OO\",{gid},{mid}");
        for &mat_id in &ids {
            let matid = id();
            material(&mut o, matid, &item.material_name(mat_id), &item.material_for(mat_id));
            let _ = writeln!(conns, "\tC: \"OO\",{matid},{mid}");
        }
    }
    o.push_str("}\n");
    conns.push_str("}\n");
    o + &conns
}
