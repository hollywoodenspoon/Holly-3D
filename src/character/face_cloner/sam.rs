//! MobileSAM (Chaoning Zhang et al., Apache-2.0), run by EffectCraft's
//! pure-Rust port: an optional download (`mobile_sam.pt`, about 41 MB)
//! that cuts the face skin out of each photo, without hair, background or
//! anything in front of the face. The weights live in
//! `~/.local/share/ilex/models/`, are checked against their SHA-256, and
//! can be removed again. Without them the cloner masks with the landmarks.

use effectcraft_segment::{load, notice, Loaded, MaskModel, Prompt, MOBILE_SAM};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

/// Download size in megabytes (for the button).
pub fn size_mb() -> f32 {
    MOBILE_SAM.size as f32 / 1_000_000.0
}

pub fn dir() -> PathBuf {
    crate::prefs::data_dir().join("models")
}

pub fn path() -> PathBuf {
    dir().join(MOBILE_SAM.file_name)
}

pub fn installed() -> bool {
    std::fs::metadata(path()).is_ok_and(|m| m.len() == MOBILE_SAM.size)
}

static MODEL: Mutex<Option<Arc<dyn MaskModel>>> = Mutex::new(None);

/// The loaded model (read from disk on first use), if installed.
pub fn model() -> Option<Arc<dyn MaskModel>> {
    let mut m = MODEL.lock().ok()?;
    if m.is_none() && installed() {
        let bytes = std::fs::read(path()).ok()?;
        if let Ok(Loaded::Mask(net)) = load(MOBILE_SAM.id, &bytes) {
            *m = Some(net);
        }
    }
    m.clone()
}

/// Checks `bytes` and writes them (plus the licence notice) into place.
pub fn install(bytes: &[u8]) -> Result<(), String> {
    match load(MOBILE_SAM.id, bytes)? {
        Loaded::Mask(net) => {
            std::fs::create_dir_all(dir()).map_err(|e| e.to_string())?;
            std::fs::write(path(), bytes).map_err(|e| e.to_string())?;
            std::fs::write(dir().join("mobile_sam.NOTICE.txt"), notice(&MOBILE_SAM)).map_err(|e| e.to_string())?;
            if let Ok(mut m) = MODEL.lock() {
                *m = Some(net);
            }
            Ok(())
        }
        Loaded::Face(_) => Err("not a mask model".into()),
    }
}

/// Downloads and installs the weights; `done` counts kilobytes.
pub fn download(cancel: &AtomicBool, done: &AtomicU32) -> Result<(), String> {
    let agent = crate::assets::http::agent(MOBILE_SAM.url);
    let mut got = 0u64;
    let bytes = crate::assets::http::get(&agent, MOBILE_SAM.url, cancel, &mut |n| {
        got += n;
        done.store((got / 1024) as u32, Ordering::Relaxed);
    })?;
    install(&bytes)
}

pub fn remove() -> Result<(), String> {
    if let Ok(mut m) = MODEL.lock() {
        *m = None;
    }
    for f in [path(), dir().join("mobile_sam.NOTICE.txt")] {
        if f.exists() {
            std::fs::remove_file(&f).map_err(|e| format!("{}: {e}", f.display()))?;
        }
    }
    Ok(())
}

/// Foreground probability of every pixel of `img` (row-major), for the
/// face given by skin points (`fg`), points that are not skin (`bg`) and a
/// box around the face.
pub fn segment(net: &dyn MaskModel, img: &image::RgbImage, fg: &[[f32; 2]], bg: &[[f32; 2]], bbox: [f32; 4]) -> Option<Vec<f32>> {
    let rgb: Vec<[f32; 3]> = img.pixels().map(|p| p.0.map(|c| c as f32 / 255.0)).collect();
    let points = fg.iter().map(|&p| (p, true)).chain(bg.iter().map(|&p| (p, false))).collect();
    net.segment(&rgb, img.width() as usize, img.height() as usize, &Prompt { points, bbox: Some(bbox), mask: None }).ok()
}
