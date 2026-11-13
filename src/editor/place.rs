//! Prefab Library place mode: click drops a prefab onto the surface under
//! the cursor (any object, else the grid), standing upright. The mouse
//! wheel turns it, `[` / `]` scale it, Shift-click stamps more copies and
//! "Vary Seed" gives each copy its own look.

use super::*;
use crate::generators::buildings::BuildingKind;
use crate::generators::{WorldGen, WorldKind};
use crate::scene::BaseObject;
use crate::viewport::picking::pick_polygon;
use bevy::math::Quat;

/// Place-mode options (the Prefab Library's top bar).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlaceOpts {
    /// Turn about Z, degrees.
    pub yaw_deg: f32,
    /// Uniform scale.
    pub scale: f32,
    /// Each placement gets a new seed.
    pub vary_seed: bool,
    /// Drop onto the object under the cursor (else the grid / snap point).
    pub on_surface: bool,
    /// Copies placed so far (feeds the varied seeds).
    pub count: u32,
}

impl Default for PlaceOpts {
    fn default() -> Self {
        Self { yaw_deg: 0.0, scale: 1.0, vary_seed: true, on_surface: true, count: 0 }
    }
}

/// Wheel step, degrees.
pub const ROTATE_STEP: f32 = 15.0;
/// `[` / `]` scale factor.
pub const SCALE_STEP: f32 = 1.1;

impl PlaceOpts {
    /// Turns by `steps` wheel notches, kept in 0..360.
    pub fn rotate(&mut self, steps: f32) {
        self.yaw_deg = (self.yaw_deg + steps * ROTATE_STEP).rem_euclid(360.0);
    }
    /// Scales up (`up`) or down by one step, within 0.05..20.
    pub fn rescale(&mut self, up: bool) {
        let f = if up { SCALE_STEP } else { 1.0 / SCALE_STEP };
        self.scale = (self.scale * f).clamp(0.05, 20.0);
    }
    /// The seed of the next copy.
    pub fn next_seed(&self) -> u32 {
        1 + self.count.wrapping_mul(7919) % 99_991
    }
}

impl Editor {
    /// Where a prefab lands: the closest object surface under the cursor
    /// (when Drop on Surfaces is on), else the snapped construction point.
    pub fn place_point(&self, view: usize, cursor: Vec2) -> Vec3 {
        if self.iface.place.on_surface {
            let ray = self.viewports.views[view].ray(cursor);
            let hit = self
                .selectable()
                .filter_map(|(_, o)| pick_polygon(&o.evaluated_mesh, o.world_matrix(), &ray, true))
                .min_by(|a, b| a.distance.total_cmp(&b.distance));
            if let Some(h) = hit {
                return h.point;
            }
        }
        self.snapped_point_pub(view, cursor)
    }

    /// Drops `kind` under the cursor with the place options; `keep` leaves
    /// place mode on (Shift-click stamping).
    pub fn place_prefab(&mut self, kind: BuildingKind, view: usize, cursor: Vec2, keep: bool) {
        let at = self.place_point(view, cursor);
        self.create_world(WorldKind::Building(kind), at);
        let opts = self.iface.place;
        if let Some(id) = self.current() {
            let o = &mut self.scene.objects[id];
            o.transform.rotation = Quat::from_rotation_z(opts.yaw_deg.to_radians());
            o.transform.scale = Vec3::splat(opts.scale);
            if opts.vary_seed
                && let BaseObject::World(WorldGen::Building(p)) = &mut o.base
            {
                p.seed = opts.next_seed();
                o.evaluate();
            }
            self.scene.propagate_from(id);
        }
        self.iface.place.count = opts.count.wrapping_add(1);
        self.tool = if keep { Tool::PlacePrefab(kind) } else { Tool::None };
        if keep {
            self.status = format!("Stamped {}: Shift-click for another, right-click ends", kind.label());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn place_opts_rotate_wrap_and_scale_clamps() {
        let mut o = PlaceOpts::default();
        o.rotate(-1.0);
        assert_eq!(o.yaw_deg, 345.0);
        o.rotate(25.0);
        assert_eq!(o.yaw_deg, 0.0);
        for _ in 0..100 {
            o.rescale(true);
        }
        assert_eq!(o.scale, 20.0);
        let a = o.next_seed();
        o.count += 1;
        assert_ne!(a, o.next_seed());
    }

    #[test]
    fn placing_lands_on_top_of_an_object_and_stamps() {
        let mut ed = Editor::default();
        ed.viewports.maximized = true;
        let v = ed.viewports.active;
        // Lay out a viewport and look straight down from above.
        ed.viewports.layout(crate::viewport::layout::ScreenRect { min: Vec2::ZERO, max: Vec2::new(800.0, 600.0) }, 0.0);
        let mid = Vec2::new(400.0, 300.0);
        // A kiosk at the origin, which the default view looks at.
        ed.create_world(WorldKind::Building(BuildingKind::Kiosk), Vec3::ZERO);
        let p = ed.place_point(v, mid);
        assert!(p.z > 0.1, "landed on the kiosk, not the grid: {p}");
        ed.iface.place.on_surface = false;
        assert!(ed.place_point(v, mid).z.abs() < 1e-3, "grid when Drop on Surfaces is off");
        ed.iface.place.on_surface = true;
        ed.iface.place.yaw_deg = 90.0;
        let n = ed.scene.objects.len();
        ed.place_prefab(BuildingKind::Bench, v, mid, true);
        assert_eq!(ed.scene.objects.len(), n + 1);
        assert_eq!(ed.tool, Tool::PlacePrefab(BuildingKind::Bench));
        let id = ed.current().unwrap();
        assert!((ed.scene.objects[id].transform.rotation.to_euler(bevy::math::EulerRot::ZYX).0 - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
        ed.place_prefab(BuildingKind::Bench, v, mid, false);
        assert_eq!(ed.tool, Tool::None);
        let seeds: Vec<u32> = ed
            .scene
            .objects
            .values()
            .filter_map(|o| match &o.base {
                BaseObject::World(WorldGen::Building(p)) if p.kind == BuildingKind::Bench => Some(p.seed),
                _ => None,
            })
            .collect();
        assert_eq!(seeds.len(), 2);
        assert_ne!(seeds[0], seeds[1], "Vary Seed");
    }
}
