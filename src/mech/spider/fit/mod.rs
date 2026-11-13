//! Rig Selection as Spider Platform: rigs the owner's own imported spider
//! meshes (one mech, selected) with the same joints as the built-in
//! Railgun prefab. It finds which part is which from how they touch,
//! puts each pivot on the part's bolt, cuts the fixed strut out of each
//! shin and adds the Option A carriage + piston in its place. Nothing of
//! the imported geometry is stored anywhere but the scene.

pub mod build;
pub mod classify;
pub mod geom;
pub mod pins;
pub mod strut;
#[cfg(test)]
mod tests;

use crate::mesh::Aabb;
use crate::scene::{ObjId, Scene};
use classify::Part;

/// What the fit did.
pub struct Fitted {
    pub rig: super::SpiderRig,
    /// Strut polygons cut from each shin.
    pub cut: Vec<usize>,
    /// Hinges (of 12) placed on a found bolt.
    pub bolts: usize,
}

impl Fitted {
    pub fn summary(&self) -> String {
        let cut: usize = self.cut.iter().sum();
        format!(
            "Rigged as Spider Platform: {} joints on bolts (of 12), {} strut polygons replaced by knee pistons. Drag a foot with W to bend its leg, turn joints with E.",
            self.bolts, cut
        )
    }
}

/// Rigs objects `ids` (one whole spider) in place.
pub fn rig_selection(s: &mut Scene, ids: &[ObjId]) -> Result<Fitted, String> {
    if ids.len() < 18 {
        return Err(format!("Select one whole spider (at least 18 parts: hull, turret, gun and 4 legs of 4 parts); {} selected", ids.len()));
    }
    let a = &s.assembly;
    if ids.iter().any(|&i| a.joint_of(i).is_some() || a.joints.iter().any(|j| j.parent == i)) {
        return Err("Some selected parts already have joints; Break their joints first".into());
    }
    let meshes: Vec<_> = ids.iter().map(|&i| geom::world_mesh(s, i)).collect();
    let bb = meshes
        .iter()
        .filter_map(|m| Aabb::from_points(m.vertices.values().map(|v| v.position)))
        .reduce(|a, b| a.union(&b))
        .ok_or("The selection has no geometry")?;
    let tol = (bb.size().length() * 0.01).max(1e-3);
    let parts: Vec<Part> = meshes.into_iter().map(|m| Part::new(m, tol)).collect();
    let roles = classify::classify(&parts, tol)?;
    let hub = parts[roles.hull].bb.center();
    let fits = roles.legs.iter().map(|l| pins::fit_leg(&parts, l, hub, tol * 10.0)).collect::<Result<Vec<_>, _>>()?;
    let bolts = fits.iter().map(|f| f.bolts).sum();
    let built = build::build(s, ids, parts, &roles, &fits);
    Ok(Fitted { rig: built.rig, cut: built.cut, bolts })
}
