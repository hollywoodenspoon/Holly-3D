//! Content-addressed cache of cooked grids. A key is a hash of everything
//! that decides the result (a map node's kind, parameters and inputs, or a
//! surface recipe), so equal work is done once, an edit simply makes a new
//! key, and old results fall out of the byte-bounded LRU.
//!
//! The design follows MaterialCraft's `crates/eval` (`cache.rs`,
//! `cache_key.rs`): https://github.com/storytold/materialcraft, copyright
//! (c) 2026 ArtCraft Team and the MaterialCraft contributors, MIT OR
//! Apache-2.0.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

/// What the cache stores: anything that knows its size in bytes.
pub trait Cooked: Send + Sync + 'static {
    fn bytes(&self) -> usize;
}

impl Cooked for super::grid::Grid {
    fn bytes(&self) -> usize {
        super::grid::Grid::bytes(self)
    }
}

type Entry = (Arc<dyn std::any::Any + Send + Sync>, usize, u64);

/// LRU cache of cooked results, bounded by bytes.
pub struct Cache {
    entries: HashMap<u64, Entry>,
    bytes: usize,
    cap: usize,
    tick: u64,
}

impl Cache {
    pub fn new(cap_bytes: usize) -> Cache {
        Cache { entries: HashMap::new(), bytes: 0, cap: cap_bytes, tick: 0 }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn bytes(&self) -> usize {
        self.bytes
    }

    pub fn get<T: Cooked>(&mut self, key: u64) -> Option<Arc<T>> {
        self.tick += 1;
        let t = self.tick;
        self.entries.get_mut(&key).and_then(|e| {
            e.2 = t;
            e.0.clone().downcast::<T>().ok()
        })
    }

    pub fn put<T: Cooked>(&mut self, key: u64, v: Arc<T>) {
        let size = v.bytes();
        self.tick += 1;
        if let Some(old) = self.entries.insert(key, (v, size, self.tick)) {
            self.bytes = self.bytes.saturating_sub(old.1);
        }
        self.bytes += size;
        while self.bytes > self.cap && self.entries.len() > 1 {
            let Some((&k, _)) = self.entries.iter().min_by_key(|(_, e)| e.2) else { break };
            if let Some(e) = self.entries.remove(&k) {
                self.bytes = self.bytes.saturating_sub(e.1);
            }
        }
    }
}

/// FNV-1a over the words fed to it: the content key builder.
pub struct Key(pub u64);

impl Default for Key {
    fn default() -> Self {
        Key(0xcbf2_9ce4_8422_2325)
    }
}

impl Key {
    pub fn u(&mut self, v: u64) -> &mut Self {
        self.0 = (self.0 ^ v).wrapping_mul(0x0000_0100_0000_01b3);
        self
    }
    pub fn f(&mut self, vs: &[f32]) -> &mut Self {
        vs.iter().for_each(|v| _ = self.u(v.to_bits() as u64));
        self
    }
}

const CAP_BYTES: usize = 256 << 20;
const RECENT: usize = 8;

fn shared() -> &'static Mutex<Cache> {
    static C: OnceLock<Mutex<Cache>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(Cache::new(CAP_BYTES)))
}

fn lock() -> std::sync::MutexGuard<'static, Cache> {
    shared().lock().unwrap_or_else(PoisonError::into_inner)
}

static HITS: AtomicU64 = AtomicU64::new(0);
static COOKS: AtomicU64 = AtomicU64::new(0);

thread_local!(static LAST: RefCell<Vec<(u64, Arc<dyn std::any::Any + Send + Sync>)>> = const { RefCell::new(Vec::new()) });

fn remember<T: Cooked>(key: u64, v: &Arc<T>) {
    LAST.with(|r| {
        let mut r = r.borrow_mut();
        r.insert(0, (key, v.clone()));
        r.truncate(RECENT);
    });
}

/// The cooked result under `key` if it is already there. Each thread keeps
/// its last few results, so render threads rarely touch the shared lock.
pub fn peek<T: Cooked>(key: u64) -> Option<Arc<T>> {
    if let Some(v) = LAST.with(|r| r.borrow().iter().find(|e| e.0 == key).and_then(|e| e.1.clone().downcast::<T>().ok())) {
        return Some(v);
    }
    let v = lock().get::<T>(key)?;
    HITS.fetch_add(1, Ordering::Relaxed);
    remember(key, &v);
    Some(v)
}

/// The result under `key`, cooked with `make` on a miss.
pub fn cooked<T: Cooked>(key: u64, make: impl FnOnce() -> T) -> Arc<T> {
    if let Some(v) = peek(key) {
        return v;
    }
    // Cooked outside the lock: an input's grid is fetched the same way.
    let v = Arc::new(make());
    COOKS.fetch_add(1, Ordering::Relaxed);
    lock().put(key, v.clone());
    remember(key, &v);
    v
}

/// (shared-cache hits, cooks) so far, for tests and diagnostics.
pub fn stats() -> (u64, u64) {
    (HITS.load(Ordering::Relaxed), COOKS.load(Ordering::Relaxed))
}

#[cfg(test)]
mod tests {
    use super::super::grid::Grid;
    use super::*;

    fn grid(v: f32) -> Arc<Grid> {
        Arc::new(Grid::gray_from(4, &[v; 16]))
    }

    #[test]
    fn the_lru_keeps_recent_entries_within_its_byte_budget() {
        // Each 4 × 4 grid is 256 bytes; room for three.
        let mut c = Cache::new(800);
        for k in 0..3 {
            c.put(k, grid(k as f32));
        }
        assert_eq!((c.len(), c.bytes()), (3, 768));
        assert!(c.get::<Grid>(0).is_some(), "touching 0 makes 1 the oldest");
        c.put(3, grid(3.0));
        assert_eq!(c.len(), 3);
        assert!(c.get::<Grid>(1).is_none() && c.get::<Grid>(0).is_some() && c.get::<Grid>(3).is_some());
        // Replacing a key does not count its bytes twice.
        c.put(3, grid(9.0));
        assert_eq!(c.bytes(), 768);
        assert_eq!(c.get::<Grid>(3).unwrap().data[0][0], 9.0);
    }

    #[test]
    fn equal_keys_cook_once_and_different_keys_cook_again() {
        let key = |v: f32| Key::default().u(0x7e57).f(&[v, 2.0]).0;
        assert_eq!(key(1.0), key(1.0));
        assert_ne!(key(1.0), key(1.5));
        let calls = std::cell::Cell::new(0);
        let make = |v: f32| {
            calls.set(calls.get() + 1);
            Grid::gray_from(2, &[v; 4])
        };
        let a = cooked(key(1.0), || make(1.0));
        let b = cooked(key(1.0), || make(1.0));
        assert!(Arc::ptr_eq(&a, &b) && calls.get() == 1, "the second request is a cache hit");
        let c = cooked(key(1.5), || make(1.5));
        assert!(calls.get() == 2 && c.data[0][0] == 1.5);
        assert!(peek::<Grid>(key(1.0)).is_some() && peek::<Grid>(key(77.0)).is_none());
        // Another thread finds it in the shared cache.
        let hits = stats().0;
        let k = key(1.0);
        assert!(std::thread::spawn(move || peek::<Grid>(k).is_some()).join().unwrap());
        assert!(stats().0 > hits);
    }
}
