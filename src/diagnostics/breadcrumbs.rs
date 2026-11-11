//! The "black-box flight recorder": a thread-safe ring buffer of the last
//! [`CAPACITY`] user actions, tool clicks, hotkeys, sub-object changes,
//! modifier evaluations and dialog openings. A crash report lists them in
//! order so the steps that led up to the crash are visible.
//!
//! Repeats of the same entry (a spinner drag re-evaluating a modifier every
//! frame) fold into one entry with a count, so they do not push the
//! interesting history out.

use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

pub const CAPACITY: usize = 50;

#[derive(Clone, Debug, PartialEq)]
pub struct Crumb {
    /// Seconds since the program started.
    pub t: f64,
    /// "Action", "Hotkey", "Tool", "Sub-Object", "Modifier", "Dialog", "Op"...
    pub kind: &'static str,
    pub text: String,
    /// How many times in a row this entry happened (1 = once).
    pub repeat: u32,
}

impl Crumb {
    /// One Markdown list line, e.g. `+12.40s  **Action** Move ×3`.
    pub fn line(&self) -> String {
        let rep = if self.repeat > 1 { format!(" ×{}", self.repeat) } else { String::new() };
        format!("`+{:>8.2}s` **{}** {}{}", self.t, self.kind, self.text, rep)
    }
}

/// A fixed-size ring of crumbs, oldest first.
#[derive(Clone, Debug)]
pub struct Ring {
    buf: VecDeque<Crumb>,
    cap: usize,
    /// Entries ever recorded (folded repeats count once).
    pub total: u64,
}

impl Ring {
    pub fn new(cap: usize) -> Self {
        Self { buf: VecDeque::with_capacity(cap), cap: cap.max(1), total: 0 }
    }

    pub fn push(&mut self, t: f64, kind: &'static str, text: impl Into<String>) {
        let text = text.into();
        if let Some(last) = self.buf.back_mut()
            && last.kind == kind
            && last.text == text
        {
            last.repeat += 1;
            last.t = t;
            return;
        }
        if self.buf.len() == self.cap {
            self.buf.pop_front();
        }
        self.buf.push_back(Crumb { t, kind, text, repeat: 1 });
        self.total += 1;
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    /// Oldest first.
    pub fn crumbs(&self) -> Vec<Crumb> {
        self.buf.iter().cloned().collect()
    }
}

fn start() -> Instant {
    static START: OnceLock<Instant> = OnceLock::new();
    *START.get_or_init(Instant::now)
}

fn global() -> &'static Mutex<Ring> {
    static RING: OnceLock<Mutex<Ring>> = OnceLock::new();
    RING.get_or_init(|| Mutex::new(Ring::new(CAPACITY)))
}

/// Seconds since the program started.
pub fn now() -> f64 {
    start().elapsed().as_secs_f64()
}

/// Records one breadcrumb. Never blocks for long and never panics.
pub fn record(kind: &'static str, text: impl Into<String>) {
    let t = now();
    let mut ring = global().lock().unwrap_or_else(|p| p.into_inner());
    ring.push(t, kind, text);
}

/// The recorded crumbs, oldest first. Used from the panic hook, so it does
/// not wait: if the buffer is locked (the crash happened mid-record) it
/// returns `None`.
pub fn try_snapshot() -> Option<Vec<Crumb>> {
    match global().try_lock() {
        Ok(r) => Some(r.crumbs()),
        Err(std::sync::TryLockError::Poisoned(p)) => Some(p.into_inner().crumbs()),
        Err(std::sync::TryLockError::WouldBlock) => None,
    }
}
