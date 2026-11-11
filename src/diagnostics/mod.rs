//! Black-box flight recorder, automatic crash reporter and emergency scene
//! recovery.
//!
//! - [`breadcrumbs`]: the last 50 actions, hotkeys, tools, sub-object
//!   changes, modifier edits and dialogs.
//! - [`state`]: the scene/selection summary and scene snapshot kept fresh
//!   by the UI each frame.
//! - [`report`]: the Markdown crash report.
//! - [`recovery`]: the crashes folder, the emergency `.mmax` and next-launch
//!   detection.
//! - [`hook`]: the panic hook and fatal-signal handlers.

pub mod breadcrumbs;
pub mod hook;
pub mod recovery;
pub mod report;
pub mod state;

#[cfg(test)]
mod tests;
