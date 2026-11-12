//! What the crash reporter knows about the running session: a short
//! Markdown summary of the scene and selection, the latest scene snapshot
//! for the emergency save, and the GPU name. The UI refreshes them every
//! frame through [`Observer`], which also turns tool, sub-object level and
//! dialog changes into breadcrumbs.

use super::breadcrumbs::record;
use crate::editor::Editor;
use crate::scene::{BaseObject, Scene};
use std::sync::{Mutex, OnceLock};

/// How often (seconds) the scene snapshot is refreshed while edits happen.
pub const SNAPSHOT_EVERY: f64 = 5.0;
/// How often (seconds) the selection summary is refreshed.
pub const SUMMARY_EVERY: f64 = 0.5;

#[derive(Default)]
struct Shared {
    summary: String,
    scene: Option<Scene>,
    gpu: String,
}

fn shared() -> &'static Mutex<Shared> {
    static S: OnceLock<Mutex<Shared>> = OnceLock::new();
    S.get_or_init(Default::default)
}

fn with<R>(f: impl FnOnce(&mut Shared) -> R) -> R {
    f(&mut shared().lock().unwrap_or_else(|p| p.into_inner()))
}

/// For the panic hook: never waits on a held lock.
fn try_with<R>(f: impl FnOnce(&mut Shared) -> R) -> Option<R> {
    match shared().try_lock() {
        Ok(mut s) => Some(f(&mut s)),
        Err(std::sync::TryLockError::Poisoned(p)) => Some(f(&mut p.into_inner())),
        Err(std::sync::TryLockError::WouldBlock) => None,
    }
}

pub fn set_gpu(name: impl Into<String>) {
    let name = name.into();
    with(|s| s.gpu = name);
}

pub fn gpu() -> String {
    try_with(|s| s.gpu.clone()).filter(|g| !g.is_empty()).unwrap_or_else(|| "unknown".into())
}

pub fn summary() -> Option<String> {
    try_with(|s| s.summary.clone()).filter(|s| !s.is_empty())
}

/// Runs `f` on the last scene snapshot (for the emergency save).
pub fn with_scene<R>(f: impl FnOnce(&Scene) -> R) -> Option<R> {
    try_with(|s| s.scene.as_ref().map(f)).flatten()
}

pub fn type_name(b: &BaseObject) -> &'static str {
    match b {
        BaseObject::Primitive(_) => "Primitive",
        BaseObject::EditablePoly(_) => "Editable Poly",
        BaseObject::Light(_) => "Light",
        BaseObject::Target => "Target",
        BaseObject::Camera(_) => "Camera",
        BaseObject::Shape(_) => "Shape",
        BaseObject::GeoGen(_) => "GeoGen",
        BaseObject::Mech(_) => "Mech",
        BaseObject::World(_) => "World",
        BaseObject::Character(_) => "Character",
    }
}

/// A tool's name without its (possibly long) data.
pub fn short_debug(v: &impl std::fmt::Debug) -> String {
    let s = format!("{v:?}");
    let cut = s.find([' ', '{', '(']).unwrap_or(s.len());
    s[..cut].to_string()
}

/// The "Active Scene & Selection State" section body.
pub fn describe(ed: &Editor) -> String {
    let mut out = String::new();
    let mut line = |k: &str, v: String| out.push_str(&format!("- **{k}:** {v}\n"));
    line("Objects in scene", ed.scene.objects.len().to_string());
    let sel = ed.scene.selected();
    let cur = ed.current_obj();
    line(
        "Selected",
        match (cur, sel.len()) {
            (_, 0) | (None, _) => "nothing".into(),
            (Some(o), 1) => format!("`{}` ({})", o.name, type_name(&o.base)),
            (Some(o), n) => format!("{n} objects; current `{}` ({})", o.name, type_name(&o.base)),
        },
    );
    line("Sub-object level", format!("{:?}", ed.sub));
    line("Active tool", short_debug(&ed.tool));
    line("Open caddy", ed.caddy.as_ref().map_or("none".into(), |c| c.op.name().to_string()));
    line("Command panel tab", format!("{:?}", ed.tab));
    if let Some(o) = cur {
        let stack: Vec<String> = o
            .modifiers
            .iter()
            .enumerate()
            .map(|(i, m)| {
                let on = if m.enabled_in_viewport { "" } else { " (off)" };
                let act = if o.active_level == Some(i) { " ◀ active" } else { "" };
                format!("{}{on}{act}", m.name)
            })
            .collect();
        let base = if o.active_level.is_none() { " ◀ active" } else { "" };
        line("Modifier stack (bottom → top)", format!("{}{base}{}", type_name(&o.base), stack.iter().map(|s| format!(" → {s}")).collect::<String>()));
        let (v, e, f) = o.evaluated_mesh.counts();
        line("Active object mesh", format!("{v} verts, {e} edges, {f} polys"));
    }
    line("Last undo step", ed.history.last_label().map_or("none".into(), |l| format!("`{l}`")));
    line("Scene file", ed.scene_path.as_ref().map_or("unsaved".into(), |p| p.display().to_string()));
    out
}

/// Spots changes between frames and refreshes the shared state.
#[derive(Default)]
pub struct Observer {
    tool: String,
    sub: String,
    caddy: Option<String>,
    dialogs: Vec<bool>,
    pushes: Option<u64>,
    snap_at: f64,
    sum_at: f64,
    primed: bool,
}

/// Dialog name and whether it is open.
fn dialogs(ed: &Editor) -> [(&'static str, bool); 9] {
    [
        ("Material Editor", ed.mat_editor.open),
        ("Render Setup", ed.rfw.setup_open),
        ("Veach Rendered Frame Window", ed.rfw.rfw_open),
        ("Environment and Effects", ed.env_open),
        ("Preferences", ed.iface.prefs_open),
        ("Help", ed.iface.help_open),
        ("Array", ed.array_dialog.is_some()),
        ("Align", ed.align_dialog.is_some()),
        ("Crash Reports", ed.iface.crash_open),
    ]
}

impl Observer {
    /// Breadcrumbs for what changed since the last call.
    pub fn changes(&mut self, ed: &Editor) -> Vec<(&'static str, String)> {
        let mut v = vec![];
        let tool = short_debug(&ed.tool);
        if tool != self.tool {
            v.push(("Tool", tool.clone()));
            self.tool = tool;
        }
        let sub = format!("{:?}", ed.sub);
        if sub != self.sub {
            v.push(("Sub-Object", sub.clone()));
            self.sub = sub;
        }
        let caddy = ed.caddy.as_ref().map(|c| c.op.name().to_string());
        if caddy != self.caddy {
            v.push(("Caddy", caddy.clone().map_or("closed".into(), |c| format!("{c} opened"))));
            self.caddy = caddy;
        }
        let d = dialogs(ed);
        for (i, (name, open)) in d.iter().enumerate() {
            if *open && !self.dialogs.get(i).copied().unwrap_or(false) {
                v.push(("Dialog", format!("{name} opened")));
            }
        }
        self.dialogs = d.iter().map(|x| x.1).collect();
        // The first look is the starting state, not a change.
        if !std::mem::replace(&mut self.primed, true) {
            v.clear();
        }
        v
    }

    /// Called once per frame by the UI.
    pub fn tick(&mut self, ed: &Editor, now: f64) {
        for (k, t) in self.changes(ed) {
            record(k, t);
        }
        let pushes = ed.history.pushes();
        let edited = self.pushes != Some(pushes);
        // A new undo step, or the periodic refresh that catches drags between
        // checkpoints. Meshes are shared (Arc), so the clone is cheap.
        let scene = (edited || now - self.snap_at >= SNAPSHOT_EVERY).then(|| ed.scene.clone());
        if scene.is_some() || now - self.sum_at >= SUMMARY_EVERY {
            let summary = describe(ed);
            with(|s| {
                s.summary = summary;
                if let Some(sc) = scene {
                    s.scene = Some(sc);
                    self.snap_at = now;
                }
            });
            self.sum_at = now;
        }
        self.pushes = Some(pushes);
    }
}

/// The UI's observer (one per process).
pub fn tick(ed: &Editor, now: f64) {
    static OBS: OnceLock<Mutex<Observer>> = OnceLock::new();
    let obs = OBS.get_or_init(Default::default);
    obs.lock().unwrap_or_else(|p| p.into_inner()).tick(ed, now);
}
