//! Wavefront OBJ + MTL. Written Y-up like 3ds Max's "Flip YZ-axis" default;
//! n-gons stay n-gons, vertex colours use the common `v x y z r g b` form,
//! and smoothing groups go out as `s N` (plus `#sg <mask>` for faces in
//! several groups, which this importer reads back).

use super::{MatInfo, MeshItem, y_to_z_up, z_to_y_up};
use crate::mesh::{EditablePoly, FaceCorner, Polygon, VertexId};
use bevy::math::{Mat4, Vec2, Vec3, Vec4};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::Path;

pub fn export(path: &Path, items: &[MeshItem]) -> Result<(), String> {
    let mtl_path = path.with_extension("mtl");
    let mtl_name = mtl_path.file_name().and_then(|s| s.to_str()).unwrap_or("scene.mtl").to_string();
    let (obj, mtl) = write_strings(items, &mtl_name);
    std::fs::write(path, obj).map_err(|e| e.to_string())?;
    std::fs::write(&mtl_path, mtl).map_err(|e| e.to_string())
}

/// OBJ and MTL text for `items`.
pub fn write_strings(items: &[MeshItem], mtl_name: &str) -> (String, String) {
    let mut o = format!("# MiniMax OBJ export\nmtllib {mtl_name}\n");
    let mut mtl = String::from("# MiniMax MTL export\n");
    let mut written: Vec<String> = vec![];
    let (mut nv, mut nt) = (0usize, 0usize);
    let yup = z_to_y_up();
    for item in items {
        let m = item.world_mesh();
        let _ = writeln!(o, "o {}", item.name.replace(' ', "_"));
        let colored = m.vertices.values().any(|v| v.color != Vec4::ONE);
        let mut index: HashMap<VertexId, usize> = HashMap::new();
        for (k, v) in &m.vertices {
            let p = yup.transform_point3(v.position);
            if colored {
                let c = v.color;
                let _ = writeln!(o, "v {} {} {} {} {} {}", p.x, p.y, p.z, c.x, c.y, c.z);
            } else {
                let _ = writeln!(o, "v {} {} {}", p.x, p.y, p.z);
            }
            nv += 1;
            index.insert(k, nv);
        }
        for p in m.polygons.values() {
            for c in &p.corners {
                let _ = writeln!(o, "vt {} {}", c.uv.x, c.uv.y);
            }
        }
        for id in item.material_ids() {
            let name = item.material_name(id).replace(' ', "_");
            let _ = writeln!(o, "usemtl {name}");
            if !written.contains(&name) {
                write_mtl(&mut mtl, &name, &item.material_for(id));
                written.push(name);
            }
            let mut last_sg = None;
            let mut t = nt;
            for p in m.polygons.values() {
                let n = p.corners.len();
                if p.material_id.max(1) != id {
                    t += n;
                    continue;
                }
                if last_sg != Some(p.smoothing_group) {
                    write_sg(&mut o, p.smoothing_group);
                    last_sg = Some(p.smoothing_group);
                }
                o.push('f');
                for (i, c) in p.corners.iter().enumerate() {
                    let _ = write!(o, " {}/{}", index[&c.vertex_id], t + i + 1);
                }
                o.push('\n');
                t += n;
            }
        }
        nt += m.polygons.values().map(|p| p.corners.len()).sum::<usize>();
    }
    (o, mtl)
}

fn write_sg(o: &mut String, sg: u32) {
    if sg == 0 {
        o.push_str("s off\n");
        return;
    }
    if sg.count_ones() > 1 {
        let _ = writeln!(o, "#sg {sg}");
    }
    let _ = writeln!(o, "s {}", sg.trailing_zeros() + 1);
}

fn write_mtl(mtl: &mut String, name: &str, m: &MatInfo) {
    let [r, g, b] = m.base_color;
    let [er, eg, eb] = m.emissive;
    let ns = ((1.0 - m.roughness).powi(2) * 1000.0).max(1.0);
    let _ = writeln!(mtl, "\nnewmtl {name}\nKd {r} {g} {b}\nKa 0 0 0\nKs 0.5 0.5 0.5\nNs {ns}\nd {}", m.opacity);
    let _ = writeln!(mtl, "Ke {er} {eg} {eb}\nPr {}\nPm {}\nillum 2", m.roughness, m.metallic);
}

pub fn import(path: &Path) -> Result<Vec<MeshItem>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut mats = HashMap::new();
    for line in text.lines() {
        if let Some(lib) = line.trim().strip_prefix("mtllib ") {
            let p = path.with_file_name(lib.trim());
            if let Ok(s) = std::fs::read_to_string(p) {
                mats.extend(parse_mtl(&s));
            }
        }
    }
    Ok(parse(&text, &mats))
}

/// Parses MTL text into named materials.
pub fn parse_mtl(s: &str) -> HashMap<String, MatInfo> {
    let mut out = HashMap::new();
    let mut cur: Option<MatInfo> = None;
    let f = |w: &[&str], i: usize| w.get(i).and_then(|x| x.parse::<f32>().ok()).unwrap_or(0.0);
    for line in s.lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        match w.first().copied() {
            Some("newmtl") => {
                if let Some(m) = cur.take() {
                    out.insert(m.name.clone(), m);
                }
                cur = Some(MatInfo { name: w[1..].join(" "), ..Default::default() });
            }
            Some(key) => {
                let Some(m) = cur.as_mut() else { continue };
                match key {
                    "Kd" => m.base_color = [f(&w, 1), f(&w, 2), f(&w, 3)],
                    "Ke" => m.emissive = [f(&w, 1), f(&w, 2), f(&w, 3)],
                    "d" => m.opacity = f(&w, 1),
                    "Tr" => m.opacity = 1.0 - f(&w, 1),
                    "Pr" => m.roughness = f(&w, 1),
                    "Pm" => m.metallic = f(&w, 1),
                    "Ns" => m.roughness = 1.0 - (f(&w, 1) / 1000.0).clamp(0.0, 1.0).sqrt(),
                    _ => {}
                }
            }
            None => {}
        }
    }
    if let Some(m) = cur {
        out.insert(m.name.clone(), m);
    }
    out
}

#[derive(Default)]
struct Building {
    name: String,
    mesh: EditablePoly,
    remap: HashMap<usize, VertexId>,
    mats: Vec<String>,
}

/// Parses OBJ text (Y-up) into Z-up mesh items.
pub fn parse(text: &str, mats: &HashMap<String, MatInfo>) -> Vec<MeshItem> {
    let zup = y_to_z_up();
    let (mut pos, mut cols, mut uvs): (Vec<Vec3>, Vec<Vec4>, Vec<Vec2>) = (vec![], vec![], vec![]);
    let mut done: Vec<Building> = vec![];
    let mut cur = Building { name: "Object".into(), ..Default::default() };
    let (mut sg, mut pending_mask, mut mat_id) = (0u32, None::<u32>, 1u16);
    for line in text.lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        let f = |i: usize| w.get(i).and_then(|x| x.parse::<f32>().ok()).unwrap_or(0.0);
        match w.first().copied() {
            Some("v") => {
                pos.push(zup.transform_point3(Vec3::new(f(1), f(2), f(3))));
                cols.push(if w.len() >= 7 { Vec4::new(f(4), f(5), f(6), 1.0) } else { Vec4::ONE });
            }
            Some("vt") => uvs.push(Vec2::new(f(1), f(2))),
            Some("o") | Some("g") if w.len() > 1 => {
                let name = w[1..].join(" ");
                if !cur.mesh.polygons.is_empty() {
                    done.push(std::mem::take(&mut cur));
                    mat_id = 1;
                } else if w[0] == "g" && cur.name != "Object" {
                    continue;
                }
                cur.name = name;
            }
            Some("usemtl") => {
                let name = w[1..].join(" ");
                let i = cur.mats.iter().position(|m| *m == name).unwrap_or_else(|| {
                    cur.mats.push(name);
                    cur.mats.len() - 1
                });
                mat_id = i as u16 + 1;
            }
            Some("#sg") => pending_mask = w.get(1).and_then(|x| x.parse().ok()),
            Some("s") => {
                sg = match w.get(1).copied() {
                    None | Some("off") | Some("0") => 0,
                    Some(n) => pending_mask.take().unwrap_or_else(|| 1u32 << (n.parse::<u32>().unwrap_or(1).clamp(1, 32) - 1)),
                };
            }
            Some("f") => {
                let mut corners = vec![];
                for tok in &w[1..] {
                    let mut parts = tok.split('/');
                    let idx = |s: Option<&str>, len: usize| {
                        let i: i64 = s.filter(|s| !s.is_empty())?.parse().ok()?;
                        let i = if i < 0 { len as i64 + i } else { i - 1 };
                        (0..len as i64).contains(&i).then_some(i as usize)
                    };
                    let Some(vi) = idx(parts.next(), pos.len()) else { continue };
                    let uv = idx(parts.next(), uvs.len()).map_or(Vec2::ZERO, |t| uvs[t]);
                    let vid = *cur.remap.entry(vi).or_insert_with(|| {
                        let id = cur.mesh.add_vertex(pos[vi]);
                        cur.mesh.vertices[id].color = cols[vi];
                        id
                    });
                    corners.push(FaceCorner::new(vid, uv));
                }
                if corners.len() >= 3 {
                    let mut t = Polygon::new(vec![]);
                    t.smoothing_group = sg;
                    t.material_id = mat_id;
                    cur.mesh.add_polygon_corners(corners, &t);
                }
            }
            _ => {}
        }
    }
    done.push(cur);
    done.into_iter()
        .filter(|b| !b.mesh.polygons.is_empty())
        .map(|mut b| {
            b.mesh.finalize();
            let materials =
                b.mats.iter().map(|n| mats.get(n).cloned().unwrap_or(MatInfo { name: n.clone(), ..Default::default() })).collect();
            MeshItem { name: b.name, mesh: b.mesh, transform: Mat4::IDENTITY, materials }
        })
        .collect()
}
