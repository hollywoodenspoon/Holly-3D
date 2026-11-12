//! Spider Platform mech line (Create -> Geometry -> Mech): a 4-leg base
//! that turrets and mission modules mount on, rebuilt from the owner's
//! old renders. Bare geometry (Material IDs 1-10 and Smart UVs, no
//! materials) posed with bone-free joints:
//! hip swing (hinge, vertical) -> hip lift (hinge) -> knee (hinge) ->
//! ankle (ball socket, 30° cone), with the Option A knee support (a
//! carriage sliding on the shin, Motion-Linked to the knee, and a piston
//! from the thigh to the carriage). Turret turn and gun lift are hinges.
//! The hull is Grounded, so dragging a foot with `W` bends its leg (IK).

pub mod hull;
pub mod kit;
pub mod leg;
pub mod railgun;
mod rig;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_render;

use bevy::math::Vec2;
pub use rig::{LegIds, SpiderRig, spawn_railgun};

/// Hull plate underside above the ground at rest (scene units = metres).
pub const HULL_Z: f32 = 2.0;
/// Base plate thickness.
pub const PLATE_H: f32 = 0.45;
/// Turret ring height above the plate underside.
pub const PEDESTAL_TOP: f32 = 1.35;
/// Hip drum centres in the hull's XY plane: FL, FR, RL, RR (front = -Y,
/// the mech's left = +X).
pub const DRUM_AT: [Vec2; 4] = [Vec2::new(1.85, -2.3), Vec2::new(-1.85, -2.3), Vec2::new(1.85, 2.3), Vec2::new(-1.85, 2.3)];
/// Leg names, in `DRUM_AT` order.
pub const LEG_NAMES: [&str; 4] = ["FL", "FR", "RL", "RR"];

/// Joint stops (degrees, or metres for the carriage).
pub mod limits {
    pub const HIP_SWING: f32 = 45.0;
    pub const HIP_LIFT: (f32, f32) = (-30.0, 60.0);
    pub const KNEE: (f32, f32) = (-40.0, 40.0);
    pub const ANKLE_CONE: f32 = 30.0;
    pub const ANKLE_TWIST: f32 = 15.0;
    pub const CARRIAGE: f32 = 0.5;
    pub const GUN_LIFT: (f32, f32) = (-5.0, 85.0);
    /// Carriage travel (m) per degree of knee.
    pub const CARRIAGE_RATIO: f32 = -0.012;
}
