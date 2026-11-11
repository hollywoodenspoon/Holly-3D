//! Where crash reports live and how the next launch finds them:
//! `~/.local/share/minimax/crashes/crash_<stamp>.md`, the emergency scene
//! `crash_recovery.mmax`, and a `.pending` marker naming the newest report
//! until the user dismisses the startup dialog.

use crate::scene::Scene;
use std::path::{Path, PathBuf};

pub const RECOVERY_FILE: &str = "crash_recovery.mmax";
pub const PENDING_FILE: &str = ".pending";

pub fn crash_dir() -> PathBuf {
    crate::prefs::data_dir().join("crashes")
}

/// What was written for one crash.
#[derive(Clone, Debug, PartialEq)]
pub struct Bundle {
    pub report: PathBuf,
    pub recovery: Option<PathBuf>,
}

/// Writes the report, the emergency scene (when there is one) and the
/// pending marker. The report path gets a `-2`, `-3`... suffix rather than
/// overwrite an earlier crash in the same second.
pub fn write_bundle(dir: &Path, stamp: &str, report: &str, scene: Option<&Scene>) -> std::io::Result<Bundle> {
    std::fs::create_dir_all(dir)?;
    let mut path = dir.join(format!("crash_{stamp}.md"));
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("crash_{stamp}-{n}.md"));
        n += 1;
    }
    std::fs::write(&path, report)?;
    let recovery = scene.and_then(|s| {
        let p = dir.join(RECOVERY_FILE);
        crate::io::scene_file::save(s, &p).ok().map(|_| p)
    });
    std::fs::write(dir.join(PENDING_FILE), path.file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default())?;
    Ok(Bundle { report: path, recovery })
}

/// A crash the user has not seen yet.
#[derive(Clone, Debug, PartialEq)]
pub struct Pending {
    pub report: PathBuf,
    pub text: String,
    pub recovery: Option<PathBuf>,
}

/// The crash from the last run, if the startup dialog should show.
pub fn detect_pending(dir: &Path) -> Option<Pending> {
    let name = std::fs::read_to_string(dir.join(PENDING_FILE)).ok()?;
    let name = name.trim();
    let report = if name.is_empty() { list_reports(dir).into_iter().next()? } else { dir.join(name) };
    let text = std::fs::read_to_string(&report).ok()?;
    let recovery = Some(dir.join(RECOVERY_FILE)).filter(|p| p.is_file());
    Some(Pending { report, text, recovery })
}

/// Crash reports, newest first.
pub fn list_reports(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| {
            rd.filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "md") && p.file_name().is_some_and(|f| f.to_string_lossy().starts_with("crash_")))
                .collect()
        })
        .unwrap_or_default();
    // The stamp sorts by time; the -N suffix sorts after the plain name.
    v.sort_by_key(|p| std::cmp::Reverse(p.file_name().map(|f| f.to_os_string())));
    v
}

/// Hides the startup dialog until the next crash; keeps the files.
pub fn dismiss(dir: &Path) {
    let _ = std::fs::remove_file(dir.join(PENDING_FILE));
}

/// Deletes every report, the emergency scene and the marker. Returns how
/// many files went.
pub fn clear(dir: &Path) -> usize {
    let mut n = list_reports(dir).into_iter().filter(|p| std::fs::remove_file(p).is_ok()).count();
    n += usize::from(std::fs::remove_file(dir.join(RECOVERY_FILE)).is_ok());
    dismiss(dir);
    n
}

/// Opens the folder in the desktop file manager.
pub fn open_folder(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let opener = if cfg!(target_os = "macos") { "open" } else if cfg!(windows) { "explorer" } else { "xdg-open" };
    std::process::Command::new(opener).arg(dir).spawn().map(|_| ()).map_err(|e| format!("{opener}: {e}"))
}
