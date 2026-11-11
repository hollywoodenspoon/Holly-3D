//! Background cooking: the UI asks whether a material's procedural maps
//! are ready; if not, a worker thread cooks them into the shared cache
//! while the viewport and sample slots keep showing the previous look.

use super::cache;
use super::surface::{COOK_SIZE, ProcMaps, ProcSurface, cook};
use crate::materials::Material;
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock, PoisonError};

/// Cooks in flight at once; further requests wait for a later frame, by
/// which time the spinner has usually moved on (latest wins).
const MAX_JOBS: usize = 2;

fn pending() -> std::sync::MutexGuard<'static, HashSet<u64>> {
    static P: OnceLock<Mutex<HashSet<u64>>> = OnceLock::new();
    P.get_or_init(Default::default).lock().unwrap_or_else(PoisonError::into_inner)
}

/// True when the recipe's maps are cached. Otherwise starts cooking them
/// on a worker thread (if a worker is free) and returns false.
pub fn request(p: &ProcSurface, rough: f32, metal: f32) -> bool {
    if !p.active() {
        return true;
    }
    let key = p.key(rough, metal, COOK_SIZE);
    if cache::peek::<ProcMaps>(key).is_some() {
        return true;
    }
    let mut jobs = pending();
    if jobs.len() < MAX_JOBS && jobs.insert(key) {
        let p = p.clone();
        std::thread::spawn(move || {
            cache::cooked(key, || cook(&p, rough, metal, COOK_SIZE));
            pending().remove(&key);
        });
    }
    false
}

/// [`request`] for a material and every sub-material of a Multi/Sub-Object.
pub fn ready(m: &Material) -> bool {
    let mut ok = request(&m.proc, m.roughness, m.metallic);
    if let Some(multi) = &m.multi {
        for s in &multi.subs {
            ok &= ready(&s.mat);
        }
    }
    ok
}

/// Cooks in flight (for the "cooking..." hint).
pub fn busy() -> usize {
    pending().len()
}
