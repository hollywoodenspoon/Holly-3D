//! Background render jobs: the raytracer runs on worker threads pulling
//! rows from a shared counter and streams finished rows to the UI; the
//! Cycles bridge runs the external CLI and streams back the whole image.

use super::cycles;
use super::framebuffer::FrameBuffer;
use super::outline;
use super::scene_build::RenderScene;
use super::settings::RenderSettings;
use super::trace::{GSample, Pixel, Tracer};
use bevy::math::Vec4;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Pixel rectangle `[x0, y0, x1, y1)` inside the output image.
pub type Region = [u32; 4];

pub enum Msg {
    /// Rows `y0..` of the region, `w` pixels wide starting at `x0`.
    Block {
        x0: u32,
        y0: u32,
        w: u32,
        display: Vec<Vec4>,
        hdr: Vec<Vec4>,
    },
    /// Cycles: a whole new image.
    Image(FrameBuffer),
    Done(Result<String, String>),
}

pub struct RenderJob {
    rx: Mutex<Receiver<Msg>>,
    cancel: Arc<AtomicBool>,
    pub started: Instant,
    pub rows_done: u32,
    pub rows_total: u32,
}

impl Drop for RenderJob {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}

/// Clamps a region to the image (full image when `None` or empty).
pub fn clamp_region(r: Option<Region>, w: u32, h: u32) -> Region {
    match r {
        Some([x0, y0, x1, y1]) if x1.min(w) > x0 && y1.min(h) > y0 => [x0, y0, x1.min(w), y1.min(h)],
        _ => [0, 0, w, h],
    }
}

/// Renders a region synchronously (tests and the job worker).
pub fn raytrace_region(
    rs: &RenderScene,
    cfg: &RenderSettings,
    region: Region,
    cancel: &AtomicBool,
    mut on_row: impl FnMut(u32, &[Pixel]),
) -> Vec<Pixel> {
    let [x0, y0, x1, y1] = region;
    let (rw, rh) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let tracer = Tracer::new(rs, cfg);
    let next = AtomicUsize::new(0);
    let (tx, rx) = channel::<(usize, Vec<Pixel>)>();
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get()).max(1);
    let mut out = vec![Pixel { display: Vec4::ZERO, hdr: Vec4::ZERO, g: GSample::EMPTY }; rw * rh];
    std::thread::scope(|s| {
        for _ in 0..workers {
            let (tx, tracer, next) = (tx.clone(), &tracer, &next);
            s.spawn(move || {
                loop {
                    let r = next.fetch_add(1, Ordering::Relaxed);
                    if r >= rh || cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let y = y0 + r as u32;
                    let row: Vec<Pixel> = (x0..x1).map(|x| tracer.pixel(x, y, cfg.width, cfg.height)).collect();
                    if tx.send((r, row)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);
        for (r, row) in rx {
            on_row(y0 + r as u32, &row);
            out[r * rw..(r + 1) * rw].copy_from_slice(&row);
        }
    });
    if cfg.outlines && !cancel.load(Ordering::Relaxed) {
        let g: Vec<GSample> = out.iter().map(|p| p.g).collect();
        let (mut d, mut h): (Vec<Vec4>, Vec<Vec4>) = out.iter().map(|p| (p.display, p.hdr)).unzip();
        outline::apply(&g, rw, rh, &rs.materials, &mut d, &mut h);
        for (p, (d, h)) in out.iter_mut().zip(d.into_iter().zip(h)) {
            (p.display, p.hdr) = (d, h);
        }
    }
    out
}

fn split(px: &[Pixel]) -> (Vec<Vec4>, Vec<Vec4>) {
    px.iter().map(|p| (p.display, p.hdr)).unzip()
}

impl RenderJob {
    fn new(rows_total: u32) -> (Self, Sender<Msg>, Arc<AtomicBool>) {
        let (tx, rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let job = Self { rx: Mutex::new(rx), cancel: cancel.clone(), started: Instant::now(), rows_done: 0, rows_total };
        (job, tx, cancel)
    }

    /// Starts the built-in raytracer on a background thread.
    pub fn raytrace(rs: RenderScene, cfg: RenderSettings, region: Region) -> Self {
        let (job, tx, cancel) = Self::new(region[3] - region[1]);
        std::thread::spawn(move || {
            let t0 = Instant::now();
            let w = region[2] - region[0];
            let px = raytrace_region(&rs, &cfg, region, &cancel, |y, row| {
                let (display, hdr) = split(row);
                let _ = tx.send(Msg::Block { x0: region[0], y0: y, w, display, hdr });
            });
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(Msg::Done(Err("Render cancelled".into())));
                return;
            }
            let (display, hdr) = split(&px);
            let _ = tx.send(Msg::Block { x0: region[0], y0: region[1], w, display, hdr });
            let _ = tx.send(Msg::Done(Ok(format!("Rendered {}x{} in {:.1}s", cfg.width, cfg.height, t0.elapsed().as_secs_f32()))));
        });
        job
    }

    /// Exports the scene and runs the Cycles CLI on a background thread.
    pub fn cycles(rs: RenderScene, cfg: RenderSettings) -> Self {
        let (job, tx, cancel) = Self::new(cfg.height);
        std::thread::spawn(move || {
            let r = cycles::run(&rs, &cfg, &cancel).map(|fb| {
                let _ = tx.send(Msg::Image(fb));
                format!("Cycles finished ({} samples)", cfg.cycles_samples)
            });
            let _ = tx.send(Msg::Done(r));
        });
        job
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Applies streamed rows to `fb`; returns the final status once done.
    pub fn poll(&mut self, fb: &mut FrameBuffer) -> Option<Result<String, String>> {
        let rx = self.rx.lock().ok()?;
        let mut done = None;
        while let Ok(m) = rx.try_recv() {
            match m {
                Msg::Block { x0, y0, w, display, hdr } => {
                    if w > 0 && display.len() == w as usize {
                        self.rows_done += 1;
                    }
                    fb.put_block(x0, y0, w, &display, &hdr);
                }
                Msg::Image(img) => {
                    let rev = fb.rev + 1;
                    *fb = img;
                    fb.rev = rev;
                    self.rows_done = self.rows_total;
                }
                Msg::Done(r) => done = Some(r),
            }
        }
        done
    }

    pub fn progress(&self) -> f32 {
        self.rows_done.min(self.rows_total) as f32 / self.rows_total.max(1) as f32
    }
}
