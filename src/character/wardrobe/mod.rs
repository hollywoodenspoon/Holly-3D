//! MakeHuman system assets on a character: eyebrows, eyelashes, teeth,
//! tongue, hair, clothes and an optional low-poly body topology, each
//! fitted to the morphed (and posed) hm08 body through its `.mhclo`
//! binding, plus skins and eye colours for the Character Pack. Clothes
//! hide the body polygons they cover (`delete_verts`), like MakeHuman.

pub mod materials;

#[cfg(test)]
mod tests;

use super::body::Place;
use super::makehuman::library::{self, Category, Item};
use super::params::ids;
use crate::mesh::EditablePoly;
use bevy::math::{Vec2, Vec3};

/// The picked MakeHuman assets (item names from the bundled pack).
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MhAssets {
    pub eyebrows: Option<String>,
    pub eyelashes: Option<String>,
    pub teeth: Option<String>,
    pub tongue: Option<String>,
    /// Replaces the procedural hair when set.
    pub hair: Option<String>,
    pub clothes: Vec<String>,
    pub skin: Option<String>,
    pub eye_color: Option<String>,
    /// A low-poly body proxy in place of the full hm08 body.
    pub topology: Option<String>,
}

/// The most clothes one character wears (Material IDs 12..=23).
pub const MAX_CLOTHES: usize = 12;

impl MhAssets {
    /// What new Studio characters start with: brows, lashes, teeth, tongue.
    pub fn face_kit() -> Self {
        let s = |v: &str| Some(v.to_string());
        Self { eyebrows: s("eyebrow001"), eyelashes: s("eyelashes01"), teeth: s("teeth_base"), tongue: s("tongue01"), ..Default::default() }
    }

    /// The single-choice slot of `cat` (None for Clothes).
    pub fn slot_mut(&mut self, cat: Category) -> Option<&mut Option<String>> {
        Some(match cat {
            Category::Eyebrows => &mut self.eyebrows,
            Category::Eyelashes => &mut self.eyelashes,
            Category::Teeth => &mut self.teeth,
            Category::Tongue => &mut self.tongue,
            Category::Hair => &mut self.hair,
            Category::Proxy => &mut self.topology,
            Category::Clothes => return None,
        })
    }

    pub fn slot(&self, cat: Category) -> Option<&str> {
        match cat {
            Category::Eyebrows => self.eyebrows.as_deref(),
            Category::Eyelashes => self.eyelashes.as_deref(),
            Category::Teeth => self.teeth.as_deref(),
            Category::Tongue => self.tongue.as_deref(),
            Category::Hair => self.hair.as_deref(),
            Category::Proxy => self.topology.as_deref(),
            Category::Clothes => None,
        }
    }

    /// Wears or takes off a clothing item.
    pub fn toggle_clothes(&mut self, name: &str) {
        if let Some(i) = self.clothes.iter().position(|c| c == name) {
            self.clothes.remove(i);
        } else if self.clothes.len() < MAX_CLOTHES {
            self.clothes.push(name.to_string());
        }
    }

    /// Every fitted piece: (category, item name, Material ID).
    pub fn pieces(&self) -> Vec<(Category, &str, u16)> {
        let mut out = vec![];
        let single = [
            (Category::Eyebrows, ids::EYEBROWS),
            (Category::Eyelashes, ids::EYELASHES),
            (Category::Teeth, ids::TEETH),
            (Category::Tongue, ids::TONGUE),
            (Category::Hair, ids::MH_HAIR),
        ];
        for (cat, id) in single {
            if let Some(n) = self.slot(cat) {
                out.push((cat, n, id));
            }
        }
        for (k, c) in self.clothes.iter().take(MAX_CLOTHES).enumerate() {
            out.push((Category::Clothes, c.as_str(), ids::CLOTHES + k as u16));
        }
        out
    }

    /// Per-base-vertex mask of the body hidden under the clothes.
    pub fn hidden_body(&self, n: usize) -> Vec<bool> {
        let mut mask = vec![false; n];
        for c in &self.clothes {
            if let Some(it) = library::item(Category::Clothes, c) {
                for &v in &it.proxy.delete_verts {
                    if let Some(m) = mask.get_mut(v as usize) {
                        *m = true;
                    }
                }
            }
        }
        mask
    }
}

/// Adds a fitted proxy mesh with Material ID `mat`; returns its polygon count.
pub fn add_item(m: &mut EditablePoly, it: &Item, coords: &[Vec3], place: &Place, mat: u16) -> usize {
    let fitted = it.proxy.fit(coords);
    let ids: Vec<_> = fitted.iter().map(|p| m.add_vertex(place.at(*p))).collect();
    let mut n = 0;
    for f in &it.mesh.faces {
        let v: Vec<_> = f.verts().iter().filter_map(|&i| ids.get(i as usize).copied()).collect();
        if v.len() != f.verts().len() {
            continue;
        }
        let uvs: Vec<Vec2> = f.uvs().iter().map(|&t| it.mesh.uvs.get(t as usize).copied().unwrap_or(Vec2::ZERO)).collect();
        let pid = m.add_polygon_uv(&v, &uvs);
        (m.polygons[pid].material_id, m.polygons[pid].smoothing_group) = (mat, 1);
        n += 1;
    }
    n
}

/// Adds every picked piece (not the body topology) to the character.
pub fn add_pieces(m: &mut EditablePoly, mh: &MhAssets, coords: &[Vec3], place: &Place) {
    for (cat, name, mat) in mh.pieces() {
        if let Some(it) = library::item(cat, name) {
            add_item(m, &it, coords, place, mat);
        }
    }
}
