//! Background work for the Face Cloner window: reading and analysing a
//! batch of photos / frames, running the fit, and downloading MobileSAM,
//! off the UI thread.

use super::clone::{clone_face, Options, Report};
use super::frames::{analyse, Shot};
use crate::character::params::CharacterParams;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};

/// Picture files the cloner reads.
pub const IMAGE_EXTS: [&str; 7] = ["png", "jpg", "jpeg", "bmp", "webp", "tif", "tiff"];

pub enum Msg {
    /// One file analysed: its shot, or why it has none.
    Shot(Result<Shot, String>),
    Fitted(Box<Result<(CharacterParams, Report), String>>),
    /// MobileSAM was downloaded (or installed from a file) and checked.
    Sam(Result<(), String>),
}

pub struct Job {
    cancel: Arc<AtomicBool>,
    done: Arc<AtomicU32>,
    pub total: u32,
    rx: Mutex<Receiver<Msg>>,
    finished: Arc<AtomicBool>,
}

impl Job {
    pub fn progress(&self) -> (u32, u32) {
        (self.done.load(Ordering::Relaxed), self.total)
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// True once the worker has stopped and every message was taken.
    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Acquire)
    }

    /// The messages that arrived since the last call.
    pub fn poll(&self) -> Vec<Msg> {
        self.rx.lock().map(|rx| rx.try_iter().collect()).unwrap_or_default()
    }
}

fn spawn(total: u32, work: impl FnOnce(&dyn Fn(Msg), &AtomicBool, &AtomicU32) + Send + 'static) -> Job {
    let (cancel, done, finished) = (Arc::new(AtomicBool::new(false)), Arc::new(AtomicU32::new(0)), Arc::new(AtomicBool::new(false)));
    let (tx, rx) = channel();
    let (c, d, f) = (cancel.clone(), done.clone(), finished.clone());
    std::thread::spawn(move || {
        work(&|m| drop(tx.send(m)), &c, &d);
        f.store(true, Ordering::Release);
    });
    Job { cancel, done, total, rx: Mutex::new(rx), finished }
}

/// The pictures in `dir` (sorted by name, every `stride`-th one).
pub fn image_files(dir: &Path, stride: usize) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()).is_some_and(|e| IMAGE_EXTS.contains(&e.to_lowercase().as_str())))
        .collect();
    v.sort();
    v.into_iter().step_by(stride.max(1)).collect()
}

/// Reads and analyses `paths`; each becomes a `Msg::Shot` named by its path.
pub fn load(paths: Vec<PathBuf>) -> Job {
    spawn(paths.len() as u32, move |send, cancel, done| {
        for p in paths {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let name = p.display().to_string();
            let r = match image::open(&p) {
                Ok(img) => analyse(&name, &img.to_rgb8()).ok_or_else(|| format!("{}: no face found", file_name(&name))),
                Err(e) => Err(format!("{}: {e}", file_name(&name))),
            };
            send(Msg::Shot(r));
            done.fetch_add(1, Ordering::Relaxed);
        }
    })
}

/// Runs the fit of `p` to the used `shots`.
pub fn fit(p: CharacterParams, shots: Vec<Shot>, opt: Options) -> Job {
    spawn(1, move |send, _, done| {
        send(Msg::Fitted(Box::new(clone_face(&p, &shots, &opt))));
        done.store(1, Ordering::Relaxed);
    })
}

/// Downloads MobileSAM; progress counts kilobytes.
pub fn download_sam() -> Job {
    spawn((effectcraft_segment::MOBILE_SAM.size / 1024) as u32, |send, cancel, done| send(Msg::Sam(super::sam::download(cancel, done))))
}

/// Checks and installs MobileSAM weights from a file.
pub fn install_sam(path: PathBuf) -> Job {
    spawn(1, move |send, _, done| {
        send(Msg::Sam(std::fs::read(&path).map_err(|e| format!("{}: {e}", path.display())).and_then(|b| super::sam::install(&b))));
        done.store(1, Ordering::Relaxed);
    })
}

/// The file name part of a shot's path.
pub fn file_name(path: &str) -> &str {
    Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or(path)
}
