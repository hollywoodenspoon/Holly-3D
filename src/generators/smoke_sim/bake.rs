//! Background baking of Smoke Simulation frames. Each distinct set of
//! simulation settings gets one bake: a worker thread steps the solver and
//! stores every frame as smoke and fire voxel grids, so the viewport and
//! the renderers can show any frame that is ready while the rest is still
//! being worked out. A few recent bakes are kept; older ones are cancelled
//! and dropped. `shutdown()` stops and joins every worker (at exit).

use super::{SimParams, SimState};
use crate::render::volumes::density::Voxels;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// Bakes kept at once (each up to a few hundred MB at high resolution).
const KEEP: usize = 3;

/// One baked frame: smoke density and fire (0..1).
#[derive(Debug)]
pub struct Frame {
    pub smoke: Arc<Voxels>,
    pub fire: Arc<Voxels>,
}

#[derive(Debug)]
pub struct Bake {
    key: String,
    pub total: u32,
    frames: Mutex<Vec<Arc<Frame>>>,
    cancel: AtomicBool,
}

impl Bake {
    /// Frames ready so far.
    pub fn ready(&self) -> u32 {
        self.lock().len() as u32
    }

    pub fn done(&self) -> bool {
        self.ready() >= self.total
    }

    /// Frame `i`, or the latest ready one before it.
    pub fn frame(&self, i: u32) -> Option<(u32, Arc<Frame>)> {
        let f = self.lock();
        let k = (i as usize).min(f.len().checked_sub(1)?);
        Some((k as u32, f[k].clone()))
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Vec<Arc<Frame>>> {
        self.frames.lock().unwrap_or_else(|e| e.into_inner())
    }
}

static BAKES: Mutex<Vec<Arc<Bake>>> = Mutex::new(Vec::new());
static WORKERS: Mutex<Vec<JoinHandle<()>>> = Mutex::new(Vec::new());

fn key_of(p: &SimParams) -> String {
    serde_json::to_string(p).unwrap_or_default()
}

/// Fire from the temperature: only where fuel is burning hot enough to
/// glow, softly saturating so a hot core doesn't blow out.
fn fire_of(p: &SimParams, s: &SimState) -> Vec<f32> {
    if p.fuel <= 0.0 {
        return vec![0.0; s.temperature.d.len()];
    }
    let t0 = p.ignition.max(0.05) + 0.5 * p.heat.max(0.0);
    s.temperature.d.iter().map(|t| 1.0 - (-(t - t0).max(0.0) * 1.5).exp()).collect()
}

fn snapshot(p: &SimParams, s: &SimState) -> Frame {
    let dims = s.n;
    Frame { smoke: Arc::new(Voxels { dims, data: s.density.d.clone() }), fire: Arc::new(Voxels { dims, data: fire_of(p, s) }) }
}

fn run(p: SimParams, b: Arc<Bake>) {
    let mut s = SimState::new(&p);
    for _ in 0..b.total {
        if b.cancel.load(Ordering::Relaxed) {
            return;
        }
        s.step_frame(&p);
        let f = Arc::new(snapshot(&p, &s));
        b.lock().push(f);
    }
}

/// The bake of these settings, started if it isn't running yet.
pub fn bake_of(p: &SimParams) -> Arc<Bake> {
    let key = key_of(p);
    let mut all = BAKES.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(i) = all.iter().position(|b| b.key == key) {
        let b = all.remove(i);
        all.push(b.clone());
        return b;
    }
    let b = Arc::new(Bake { key, total: p.frames.clamp(1, 2000), frames: Mutex::new(Vec::new()), cancel: AtomicBool::new(false) });
    while all.len() >= KEEP {
        all.remove(0).cancel.store(true, Ordering::Relaxed);
    }
    all.push(b.clone());
    let (p, w) = (p.clone(), b.clone());
    let mut workers = WORKERS.lock().unwrap_or_else(|e| e.into_inner());
    workers.retain(|h| !h.is_finished());
    workers.push(std::thread::spawn(move || run(p, w)));
    b
}

/// Drops the bake of these settings so the next request starts afresh.
pub fn rebake(p: &SimParams) {
    let key = key_of(p);
    let mut all = BAKES.lock().unwrap_or_else(|e| e.into_inner());
    all.retain(|b| {
        let keep = b.key != key;
        if !keep {
            b.cancel.store(true, Ordering::Relaxed);
        }
        keep
    });
}

/// Cancels every bake and waits for the workers (call before exiting).
pub fn shutdown() {
    for b in BAKES.lock().unwrap_or_else(|e| e.into_inner()).drain(..) {
        b.cancel.store(true, Ordering::Relaxed);
    }
    let workers = std::mem::take(&mut *WORKERS.lock().unwrap_or_else(|e| e.into_inner()));
    for h in workers {
        let _ = h.join();
    }
}
