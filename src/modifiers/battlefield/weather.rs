//! Surface weathering after the damage: soot darkens vertex colours by the
//! blast influence (charred rafters go almost black), and in Ancient Ruins
//! mode moss (Material ID 8 plus green vertex colour) covers up-facing
//! surfaces while ivy tints the feet of the walls.

use super::DestructionMode as Mode;
use super::field::{Field, wobble};
use crate::generators::kit::ids;
use crate::mesh::EditablePoly;
use bevy::math::Vec4;

pub const MOSS: Vec4 = Vec4::new(0.32, 0.5, 0.2, 1.0);
const IVY: Vec4 = Vec4::new(0.42, 0.62, 0.3, 1.0);

pub fn run(m: &mut EditablePoly, f: &Field) {
    let soot = f.p.soot.clamp(0.0, 1.0);
    if soot > 0.0 && f.mode != Mode::AncientRuins {
        for v in m.vertices.values_mut() {
            let k = 1.0 - soot * f.influence(v.position) * 0.85;
            v.color = (v.color.truncate() * k).extend(v.color.w);
        }
    }
    let charred: Vec<_> = m.polygons.values().filter(|p| p.material_id == ids::CHARRED).flat_map(|p| p.vertex_ids().collect::<Vec<_>>()).collect();
    for v in charred {
        m.vertices[v].color = Vec4::new(0.12, 0.1, 0.09, 1.0);
    }
    if f.mode == Mode::AncientRuins {
        overgrow(m, f);
    }
}

fn overgrow(m: &mut EditablePoly, f: &Field) {
    let ground: std::collections::HashSet<_> = f.elements.iter().filter(|e| e.ground).flat_map(|e| e.polys.iter().copied()).collect();
    let ids_: Vec<_> = m.polygons.keys().filter(|p| !ground.contains(p)).collect();
    for pid in ids_ {
        let c = m.poly_centroid(pid);
        let n = m.poly_normal(pid);
        let infl = f.influence(c);
        let noise = wobble(f.p.seed ^ 0x3055, c.truncate() * 0.8 + c.z * 0.3);
        let moss = n.z > 0.6 && noise + infl > 0.15;
        let ivy = n.z.abs() < 0.3 && c.z - f.ground_at(c.truncate()) < 1.0 + 3.0 * infl && noise > -0.2;
        if moss {
            m.polygons[pid].material_id = ids::FOLIAGE;
        }
        if moss || ivy {
            let tint = if moss { MOSS } else { IVY };
            let vs: Vec<_> = m.polygons[pid].vertex_ids().collect();
            for v in vs {
                m.vertices[v].color = tint;
            }
        }
    }
}
