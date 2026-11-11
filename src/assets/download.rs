//! Pack downloads on a worker thread, with progress the Preferences
//! window polls. Files are written straight into the pack folder; a
//! marker file is added only when everything arrived, so an interrupted
//! download shows as partial and resumes where it stopped.

use super::polyhaven::{self, Fetch, Pick};
use super::{Installed, Pack, Sources, http, mark_installed, safe_rel, zip};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Phase {
    Running,
    Done,
    Cancelled,
    Failed(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Progress {
    pub phase: Phase,
    pub files_done: u32,
    pub files_total: u32,
    pub bytes: u64,
    /// What is being fetched right now.
    pub note: String,
    /// Optional files the host did not have.
    pub skipped: u32,
}

impl Progress {
    /// 0..1 for the progress bar.
    pub fn fraction(&self) -> f32 {
        if self.files_total == 0 { 0.0 } else { self.files_done as f32 / self.files_total as f32 }
    }
}

/// A running (or finished) download.
#[derive(Clone)]
pub struct Job {
    pub progress: Arc<Mutex<Progress>>,
    cancel: Arc<AtomicBool>,
}

impl Job {
    pub fn snapshot(&self) -> Progress {
        self.progress.lock().map(|p| p.clone()).unwrap_or_else(|e| e.into_inner().clone())
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    pub fn running(&self) -> bool {
        self.snapshot().phase == Phase::Running
    }
}

/// `[ Download ]`: starts fetching `pack` into `folder` in the background.
pub fn start(pack: Pack, folder: String, sources: Sources) -> Job {
    let job = Job {
        progress: Arc::new(Mutex::new(Progress { phase: Phase::Running, files_done: 0, files_total: 0, bytes: 0, note: "Connecting...".into(), skipped: 0 })),
        cancel: Arc::new(AtomicBool::new(false)),
    };
    let j = job.clone();
    std::thread::spawn(move || {
        let result = run(pack, &folder, &sources, &j);
        let mut p = j.progress.lock().unwrap_or_else(|e| e.into_inner());
        p.phase = match result {
            Ok(_) => Phase::Done,
            Err(_) if j.cancel.load(Ordering::Relaxed) => Phase::Cancelled,
            Err(e) => Phase::Failed(e),
        };
    });
    job
}

fn set(job: &Job, f: impl FnOnce(&mut Progress)) {
    f(&mut job.progress.lock().unwrap_or_else(|e| e.into_inner()));
}

/// Fetches every file of `list` that is not on disk yet.
fn fetch_all(agent: &ureq::Agent, dir: &std::path::Path, list: &[Fetch], job: &Job) -> Result<(), String> {
    set(job, |p| p.files_total += list.len() as u32);
    for f in list {
        let rel = safe_rel(&f.path).ok_or_else(|| format!("unsafe path {}", f.path))?;
        let dest = dir.join(rel);
        if dest.metadata().is_ok_and(|m| m.len() > 0) {
            set(job, |p| p.files_done += 1);
            continue;
        }
        set(job, |p| p.note = f.path.clone());
        match http::get(agent, &f.url, &job.cancel, &mut |n| set(job, |p| p.bytes += n)) {
            Ok(bytes) => {
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
                }
                // Write under a temporary name, so a crash leaves no half file.
                let tmp = dest.with_extension("part");
                std::fs::write(&tmp, bytes).and_then(|()| std::fs::rename(&tmp, &dest)).map_err(|e| format!("{}: {e}", dest.display()))?;
            }
            Err(e) if !f.required && !job.cancel.load(Ordering::Relaxed) && e.contains("HTTP 404") => set(job, |p| p.skipped += 1),
            Err(e) => return Err(e),
        }
        set(job, |p| p.files_done += 1);
    }
    Ok(())
}

/// Downloads `pack` (blocking). Used by [`start`] and by the tests.
pub fn run(pack: Pack, folder: &str, sources: &Sources, job: &Job) -> Result<Installed, String> {
    let dir = pack.path(folder);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let api = sources.api.trim_end_matches('/');
    let res = if sources.resolution == "2k" { "2k" } else { "1k" };
    match pack {
        Pack::Photos => {
            let agent = http::agent(api);
            let listing = http::get_json(&agent, &format!("{api}/assets?t=textures"), &job.cancel)?;
            let picks: Vec<Pick> = polyhaven::pick_textures(&listing, sources.photos_per_group);
            if picks.is_empty() {
                return Err("the texture list came back empty".into());
            }
            let mut kept = vec![];
            for pick in picks {
                set(job, |p| p.note = format!("{} (file list)", pick.name));
                let files = http::get_json(&agent, &format!("{api}/files/{}", pick.slug), &job.cancel)?;
                let list = polyhaven::texture_files(&pick.slug, &files, res);
                if list.iter().any(|f| f.required) {
                    fetch_all(&agent, &dir, &list, job)?;
                    kept.push(pick);
                }
            }
            // The index the Material Library reads its photo shelf from.
            let text = ron::to_string(&kept).map_err(|e| e.to_string())?;
            std::fs::write(dir.join(super::photo::INDEX), text).map_err(|e| e.to_string())?;
        }
        Pack::Realistic => {
            let agent = http::agent(api);
            let listing = http::get_json(&agent, &format!("{api}/assets?t=models"), &job.cancel)?;
            let slugs = polyhaven::pick_models(&listing, sources.model_count);
            if slugs.is_empty() {
                return Err("the model list came back empty".into());
            }
            for slug in slugs {
                set(job, |p| p.note = format!("{slug} (file list)"));
                let files = http::get_json(&agent, &format!("{api}/files/{slug}"), &job.cancel)?;
                fetch_all(&agent, &dir, &polyhaven::model_files(&slug, &files, res), job)?;
            }
        }
        Pack::LowPoly => {
            set(job, |p| p.files_total = sources.lowpoly_zips.len() as u32);
            for (k, url) in sources.lowpoly_zips.iter().enumerate() {
                let agent = http::agent(url);
                set(job, |p| p.note = url.rsplit('/').nth(4).unwrap_or(url).to_string());
                let bytes = http::get(&agent, url, &job.cancel, &mut |n| set(job, |p| p.bytes += n))?;
                // Keep the models (and the textures beside them), flat per kit.
                let kit = dir.join(format!("kit{:02}", k + 1));
                std::fs::create_dir_all(&kit).map_err(|e| e.to_string())?;
                for e in zip::entries(&bytes)? {
                    let lower = e.name.to_lowercase();
                    let model = [".glb", ".gltf", ".bin", ".obj", ".mtl"].iter().any(|x| lower.ends_with(x));
                    let texture = lower.contains("/models/") && [".png", ".jpg"].iter().any(|x| lower.ends_with(x));
                    let Some(file) = safe_rel(&e.name).and_then(|p| p.file_name().map(|f| f.to_owned())) else { continue };
                    if model || texture {
                        std::fs::write(kit.join(file), e.read()?).map_err(|e| e.to_string())?;
                    }
                }
                set(job, |p| p.files_done += 1);
            }
        }
    }
    let done = job.snapshot();
    let installed = Installed { files: super::count_files(&dir), bytes: done.bytes };
    mark_installed(pack, folder, installed)?;
    Ok(installed)
}

/// A job for calling [`run`] directly.
pub fn blocking_job() -> Job {
    Job {
        progress: Arc::new(Mutex::new(Progress { phase: Phase::Running, files_done: 0, files_total: 0, bytes: 0, note: String::new(), skipped: 0 })),
        cancel: Arc::new(AtomicBool::new(false)),
    }
}
