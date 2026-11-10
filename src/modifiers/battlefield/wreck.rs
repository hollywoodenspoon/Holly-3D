//! Structure damage: per element, openings shatter, roofs burn (charred
//! rafters) or cave in, lamp posts bend, pillars snap into fallen drums,
//! and everything above the mode's cut field is knocked off (small pieces
//! and merlons become rubble) or sheared flat (big walls, with bent rebar
//! and pancaked slabs in Modern mode, talus piles in Medieval Siege).

use super::DestructionMode as Mode;
use super::debris;
use super::field::{Element, Field};
use crate::generators::kit::{Kit, ids, rng};
use crate::mesh::{PolyId, VertexId};
use bevy::math::{Quat, Vec2, Vec3};
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;

pub fn run(kit: &mut Kit, f: &Field) {
    let mut r = rng(f.p.seed, 0x3EC);
    for e in f.elements.iter().filter(|e| !e.ground) {
        let infl = f.influence(e.center()).max(f.influence(e.min)).max(f.influence(e.max));
        if infl <= 0.0 {
            continue;
        }
        if e.tall_thin() && pole(kit, f, e, infl, &mut r) {
            continue;
        }
        surfaces(kit, f, e, &mut r);
        cut(kit, f, e, infl, &mut r);
    }
}

fn verts(kit: &Kit, polys: &[PolyId]) -> Vec<VertexId> {
    let mut seen = HashSet::new();
    polys.iter().filter_map(|&p| kit.m.polygons.get(p)).flat_map(|p| p.vertex_ids()).filter(|v| seen.insert(*v)).collect()
}

/// Lamp posts bend (Modern); pillars snap and drop their drums (Ancient).
/// Returns true when the element was handled.
fn pole(kit: &mut Kit, f: &Field, e: &Element, infl: f32, r: &mut ChaCha8Rng) -> bool {
    let s = e.size();
    let pivot = e.center().truncate().extend(e.min.z);
    let a = r.gen_range(0.0..std::f32::consts::TAU);
    let dir = Vec2::new(a.cos(), a.sin());
    match f.mode {
        Mode::Modern if infl > 0.12 => {
            let axis = Vec3::new(-dir.y, dir.x, 0.0);
            let bend = infl * r.gen_range(0.6..1.3);
            for v in verts(kit, &e.polys) {
                let p = kit.m.pos(v);
                let t = ((p.z - e.min.z) / s.z).clamp(0.0, 1.0);
                kit.m.vertices[v].position = pivot + Quat::from_axis_angle(axis, bend * t * t) * (p - pivot);
            }
            true
        }
        Mode::AncientRuins if infl > 0.2 && r.gen_bool((0.3 + infl as f64).min(1.0)) => {
            let zb = e.min.z + s.z * r.gen_range(0.25..0.7);
            for v in verts(kit, &e.polys) {
                let p = &mut kit.m.vertices[v].position;
                p.z = p.z.min(zb + (p.x * 7.0 + p.y * 3.0).sin() * 0.05);
            }
            let rad = s.x.max(s.y) * 0.45;
            let ground = f.ground_at(pivot.truncate());
            debris::drums(kit, pivot.truncate().extend(ground), dir, rad, e.max.z - zb);
            true
        }
        _ => false,
    }
}

/// Windows and roofs of one element.
fn surfaces(kit: &mut Kit, f: &Field, e: &Element, r: &mut ChaCha8Rng) {
    for &pid in &e.polys {
        let Some(poly) = kit.m.polygons.get(pid) else { continue };
        let id = poly.material_id;
        let c = kit.m.poly_centroid(pid);
        let n = kit.m.poly_normal(pid);
        let infl = f.influence(c);
        if infl <= 0.0 {
            continue;
        }
        let corners: Vec<Vec3> = poly.vertex_ids().map(|v| kit.m.pos(v)).collect();
        match id {
            ids::GLASS | ids::LIT if r.gen_bool((0.3 + infl as f64).min(1.0)) => {
                kit.m.polygons.remove(pid);
                if f.mode == Mode::Modern {
                    debris::shards(kit, &corners, r);
                }
            }
            ids::ROOF if f.mode == Mode::MedievalSiege && infl > 0.12 => {
                kit.m.polygons.remove(pid);
                if n.z > 0.05 {
                    debris::rafters(kit, &corners, n);
                }
            }
            ids::ROOF if f.mode == Mode::AncientRuins && infl > 0.2 && r.gen_bool((0.4 + infl as f64).min(1.0)) => {
                kit.m.polygons.remove(pid);
                if n.z > 0.05 && r.gen_bool(0.3) {
                    let g = f.ground_at(c.truncate());
                    debris::rubble(kit, r, c.truncate().extend(g), 1.0, 2, ids::ROOF, false);
                }
            }
            _ => {}
        }
    }
}

/// Knocks off or shears what stands above the cut field.
fn cut(kit: &mut Kit, f: &Field, e: &Element, infl: f32, r: &mut ChaCha8Rng) {
    let vs = verts(kit, &e.polys);
    if vs.is_empty() {
        return;
    }
    let cuts: Vec<f32> = vs.iter().map(|&v| f.cut(kit.m.pos(v))).collect();
    let above = vs.iter().zip(&cuts).filter(|(v, c)| kit.m.pos(**v).z > **c).count();
    if above == 0 {
        return;
    }
    let s = e.size();
    let ground = f.ground_at(e.center().truncate());
    let base = e.center().truncate().extend(ground);
    let rebar = f.mode == Mode::Modern;
    if s.max_element() < 2.5 || above == vs.len() {
        e.polys.iter().for_each(|&p| {
            kit.m.polygons.remove(p);
        });
        let big = s.max_element() > 0.6;
        let chance = (f.p.rubble * if big { 0.6 } else { 0.08 }).clamp(0.0, 1.0) as f64;
        if r.gen_bool(chance) {
            let n = if big { r.gen_range(2..5) } else { 1 };
            let mat = if f.mode == Mode::Modern || e.mat == ids::GLASS { ids::FACADE } else { e.mat };
            debris::rubble(kit, r, base, (s.x.max(s.y) * 0.6).max(1.0), n, mat, rebar);
        }
        return;
    }
    // Shear: pull every vertex above the cut down onto it.
    let mut top = f32::MIN;
    for (&v, &c) in vs.iter().zip(&cuts) {
        let p = &mut kit.m.vertices[v].position;
        if p.z > c {
            p.z = c.max(e.min.z + 0.05);
        }
        top = top.max(p.z);
    }
    let half = Vec2::new(s.x, s.y) * 0.5;
    match f.mode {
        Mode::Modern => {
            debris::rebar_tuft(kit, r, e.center().truncate().extend(top), half, (2.0 + infl * 6.0) as usize);
            if half.min_element() > 1.5 && r.gen_bool((f.p.rubble as f64).clamp(0.0, 1.0)) {
                let n = r.gen_range(2..5);
                debris::slabs(kit, r, base, half * 0.8, n);
            }
            debris::rubble(kit, r, base, half.max_element() + 1.0, (f.p.rubble * 6.0) as usize, ids::FACADE, true);
        }
        Mode::MedievalSiege => {
            let at = (e.center().truncate() + Vec2::new(r.gen_range(-1.0..1.0), r.gen_range(-1.0..1.0)) * half).extend(ground);
            debris::talus(kit, r, at, (s.z * 0.25).clamp(0.6, 3.0), f.p.rubble);
        }
        Mode::AncientRuins => {
            debris::rubble(kit, r, base, half.max_element() + 0.5, (f.p.rubble * 4.0) as usize, e.mat, false);
        }
    }
}
