//! Rendering a range of simulation frames: before each frame the render
//! job shows that frame on every Smoke & Fire Simulation in its copy of
//! the scene, waits until the background bake has it, and rebuilds the
//! scene's volumes.

use crate::generators::WorldGen;
use crate::scene::{BaseObject, Scene};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// Every simulation's settings in the scene (visible ones only).
fn sims(scene: &Scene) -> impl Iterator<Item = &super::SmokeSimParams> {
    scene.objects.values().filter(|o| !o.hidden).filter_map(|o| match &o.base {
        BaseObject::World(WorldGen::SmokeSim(p)) => Some(&**p),
        _ => None,
    })
}

/// Frames in the longest simulation (0 when the scene has none).
pub fn longest(scene: &Scene) -> u32 {
    sims(scene).map(|p| p.sim.frames.clamp(1, 2000)).max().unwrap_or(0)
}

/// Shows frame `k` (0-based) on every simulation and waits until each has
/// baked it (or its last frame, if it is shorter). False when cancelled.
pub fn show_frame(scene: &mut Scene, k: u32, cancel: &AtomicBool) -> bool {
    for o in scene.objects.values_mut() {
        if let BaseObject::World(WorldGen::SmokeSim(p)) = &mut o.base {
            p.frame = k;
        }
    }
    loop {
        if cancel.load(Ordering::Relaxed) {
            return false;
        }
        // `bake()` restarts a bake that was dropped meanwhile.
        if sims(scene).all(|p| {
            let b = p.bake();
            b.ready() > k.min(b.total - 1)
        }) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}
