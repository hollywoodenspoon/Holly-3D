//! Works out which imported part is which: a contact graph (parts whose
//! surfaces touch) with the hull as the hub. Removing the hull splits the
//! graph into four leg chains (ring -> thigh -> shin -> foot, the foot
//! being the chain's free end) and the turret group, where the gun is the
//! most elongated part and the turret is the part it sits on.

use super::geom::{Cloud, contact, pca, surface_samples};
use crate::mesh::{Aabb, EditablePoly};
use bevy::math::Vec3;
use std::collections::{BTreeSet, VecDeque};

/// One imported part in world space.
pub struct Part {
    pub mesh: EditablePoly,
    pub bb: Aabb,
    pub samples: Vec<Vec3>,
    pub cloud: Cloud,
}

impl Part {
    pub fn new(mesh: EditablePoly, step: f32) -> Self {
        let bb = Aabb::from_points(mesh.vertices.values().map(|v| v.position)).unwrap_or(Aabb { min: Vec3::ZERO, max: Vec3::ZERO });
        let samples = surface_samples(&mesh, step);
        let cloud = Cloud::new(&samples, step);
        Self { mesh, bb, samples, cloud }
    }
}

/// Indices (into the part list) of one leg, hull to foot.
#[derive(Clone, Copy, Debug)]
pub struct LegParts {
    pub ring: usize,
    pub thigh: usize,
    pub shin: usize,
    pub foot: usize,
}

#[derive(Clone, Debug)]
pub struct Roles {
    pub hull: usize,
    pub legs: Vec<LegParts>,
    pub turret: usize,
    pub gun: usize,
    /// Extra parts welded to another: (part, parent).
    pub welded: Vec<(usize, usize)>,
}

/// Contact graph: `adj[i]` lists the parts touching part `i`.
pub fn contact_graph(parts: &[Part], tol: f32) -> Vec<BTreeSet<usize>> {
    let mut adj = vec![BTreeSet::new(); parts.len()];
    for i in 0..parts.len() {
        for j in i + 1..parts.len() {
            let (a, b) = (&parts[i], &parts[j]);
            if (a.bb.min - tol).cmpgt(b.bb.max).any() || (b.bb.min - tol).cmpgt(a.bb.max).any() {
                continue;
            }
            let (small, big) = if a.samples.len() < b.samples.len() { (a, b) } else { (b, a) };
            if contact(&small.samples, &big.cloud).len() >= 3 {
                adj[i].insert(j);
                adj[j].insert(i);
            }
        }
    }
    adj
}

/// BFS distances from `from`, staying inside `within`.
fn distances(adj: &[BTreeSet<usize>], from: usize, within: &[usize]) -> Vec<Option<usize>> {
    let mut d = vec![None; adj.len()];
    d[from] = Some(0);
    let mut q = VecDeque::from([from]);
    while let Some(n) = q.pop_front() {
        for &m in &adj[n] {
            if d[m].is_none() && within.contains(&m) {
                d[m] = Some(d[n].unwrap() + 1);
                q.push_back(m);
            }
        }
    }
    d
}

fn diag(p: &Part) -> f32 {
    p.bb.size().length()
}

/// Walks one leg up from its foot: shin and thigh are the biggest new
/// neighbour at each step, the hip ring the smallest one above the thigh.
fn walk_leg(parts: &[Part], adj: &[BTreeSet<usize>], foot: usize, taken: &BTreeSet<usize>) -> Option<LegParts> {
    let mut seen = taken.clone();
    seen.insert(foot);
    let step = |from: usize, big: bool, seen: &mut BTreeSet<usize>| {
        let next = adj[from].iter().copied().filter(|n| !seen.contains(n));
        let pick = if big { next.max_by(|&a, &b| diag(&parts[a]).total_cmp(&diag(&parts[b]))) } else { next.min_by(|&a, &b| diag(&parts[a]).total_cmp(&diag(&parts[b]))) };
        if let Some(p) = pick {
            seen.insert(p);
        }
        pick
    };
    let shin = step(foot, true, &mut seen)?;
    let thigh = step(shin, true, &mut seen)?;
    let ring = step(thigh, false, &mut seen)?;
    Some(LegParts { ring, thigh, shin, foot })
}

/// Assigns every part a role, or explains what is missing.
pub fn classify(parts: &[Part], tol: f32) -> Result<Roles, String> {
    let adj = contact_graph(parts, tol);
    let lo = parts.iter().map(|p| p.bb.min.z).fold(f32::MAX, f32::min);
    let hi = parts.iter().map(|p| p.bb.max.z).fold(f32::MIN, f32::max);
    // Feet: free ends (one contact) standing near the ground.
    let feet: Vec<usize> = (0..parts.len()).filter(|&i| adj[i].len() == 1 && parts[i].bb.min.z < lo + 0.3 * (hi - lo)).collect();
    let mut taken = BTreeSet::new();
    let mut legs = vec![];
    for &f in &feet {
        if let Some(l) = walk_leg(parts, &adj, f, &taken) {
            taken.extend([l.ring, l.thigh, l.shin, l.foot]);
            legs.push(l);
        }
    }
    if legs.len() != 4 {
        return Err(format!(
            "Found {} leg(s), expected 4. Select one whole spider: hull, 4 legs (hip ring, thigh, shin, foot), turret and gun.",
            legs.len()
        ));
    }
    let rings: Vec<usize> = legs.iter().map(|l| l.ring).collect();
    let body: Vec<usize> = (0..parts.len()).filter(|i| !taken.contains(i)).collect();
    let hull = body
        .iter()
        .copied()
        .filter(|&b| rings.iter().filter(|r| adj[b].contains(r)).count() >= 2)
        .max_by(|&a, &b| diag(&parts[a]).total_cmp(&diag(&parts[b])))
        .ok_or("No hull found: nothing holds the hip rings")?;
    let elongation = |i: usize| {
        let pts: Vec<Vec3> = parts[i].mesh.vertices.values().map(|v| v.position).collect();
        let (_, _, s) = pca(&pts);
        s.z / s.y.max(1e-6)
    };
    let others: Vec<usize> = body.iter().copied().filter(|&n| n != hull).collect();
    let gun = others.iter().copied().max_by(|&a, &b| elongation(a).total_cmp(&elongation(b))).ok_or("No turret or gun found above the hull")?;
    let turret = others
        .iter()
        .copied()
        .filter(|&n| n != gun && adj[gun].contains(&n))
        .max_by(|&a, &b| diag(&parts[a]).total_cmp(&diag(&parts[b])))
        .ok_or("The gun does not touch a turret")?;
    // Other parts weld to the hull when they reach it without the turret,
    // else to the leg part or turret they touch.
    let free: Vec<usize> = others.iter().copied().filter(|&n| n != gun && n != turret).collect();
    let mut via = free.clone();
    via.push(hull);
    let reach = distances(&adj, hull, &via);
    let welded = free
        .into_iter()
        .map(|n| {
            let p = if reach[n].is_some() { hull } else { adj[n].iter().copied().find(|m| taken.contains(m)).unwrap_or(turret) };
            (n, p)
        })
        .collect();
    Ok(Roles { hull, legs, turret, gun, welded })
}
