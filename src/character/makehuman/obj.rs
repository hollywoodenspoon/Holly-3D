//! hm08 base mesh parser. Keeps the raw MakeHuman coordinates (Y-up,
//! decimetres) so `.target` deltas and `.mhclo` bindings apply directly;
//! [`to_scene`] converts to the editor's Z-up metres.

use super::assets;
use bevy::math::{Vec2, Vec3};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug)]
pub struct Face {
    pub v: [u32; 4],
    pub vt: [u32; 4],
    /// 3 or 4 corners.
    pub n: u8,
    pub group: u16,
}

impl Face {
    pub fn verts(&self) -> &[u32] {
        &self.v[..self.n as usize]
    }
    pub fn uvs(&self) -> &[u32] {
        &self.vt[..self.n as usize]
    }
}

#[derive(Debug, Default)]
pub struct BaseMesh {
    pub coords: Vec<Vec3>,
    pub uvs: Vec<Vec2>,
    pub faces: Vec<Face>,
    pub groups: Vec<String>,
}

/// Which part of the base mesh a face group is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Body,
    /// `helper-tights` / `helper-skirt`: clothing fitting shells.
    Helper,
    /// The rig's joint cubes and the other proxy-fitting helpers.
    Hidden,
}

pub fn part_of(group: &str) -> Part {
    match group {
        "body" => Part::Body,
        "helper-tights" | "helper-skirt" => Part::Helper,
        _ => Part::Hidden,
    }
}

impl BaseMesh {
    pub fn parse(src: &str) -> Self {
        let mut m = BaseMesh::default();
        let mut group = 0u16;
        for line in src.lines() {
            let mut it = line.split_ascii_whitespace();
            match it.next() {
                Some("v") => {
                    let f: Vec<f32> = it.filter_map(|s| s.parse().ok()).collect();
                    m.coords.push(Vec3::new(f[0], f[1], f[2]));
                }
                Some("vt") => {
                    let f: Vec<f32> = it.filter_map(|s| s.parse().ok()).collect();
                    m.uvs.push(Vec2::new(f[0], f[1]));
                }
                Some("g") => {
                    m.groups.push(it.next().unwrap_or("").to_string());
                    group = (m.groups.len() - 1) as u16;
                }
                Some("f") => {
                    let mut f = Face { v: [0; 4], vt: [0; 4], n: 0, group };
                    for (i, c) in it.take(4).enumerate() {
                        let mut p = c.split('/');
                        f.v[i] = p.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(1) - 1;
                        f.vt[i] = p.next().and_then(|s| s.parse::<u32>().ok()).unwrap_or(1) - 1;
                        f.n = i as u8 + 1;
                    }
                    if f.n >= 3 {
                        m.faces.push(f);
                    }
                }
                _ => {}
            }
        }
        m
    }

    pub fn part(&self, f: &Face) -> Part {
        part_of(&self.groups[f.group as usize])
    }

    /// Per-vertex mask: true for vertices used by a face of `part`.
    pub fn vertex_mask(&self, part: Part) -> Vec<bool> {
        let mut mask = vec![false; self.coords.len()];
        for f in self.faces.iter().filter(|f| self.part(f) == part) {
            for &v in f.verts() {
                mask[v as usize] = true;
            }
        }
        mask
    }
}

/// The parsed hm08 base mesh (cached).
pub fn base() -> &'static BaseMesh {
    static CELL: OnceLock<BaseMesh> = OnceLock::new();
    CELL.get_or_init(|| BaseMesh::parse(assets::text(assets::base_obj())))
}

/// MakeHuman (x, y-up, z, decimetres) -> scene (x, -z, z-up, metres).
pub fn to_scene(p: Vec3) -> Vec3 {
    Vec3::new(p.x, -p.z, p.y) * 0.1
}
