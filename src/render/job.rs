//! Background render jobs: the raytracer runs on worker threads pulling
//! 32x32 tiles from a shared counter and streams finished tiles to the UI; the
//! Cycles bridge runs the external CLI and streams back the whole image.

use super::cycles;
use super::framebuffer::FrameBuffer;
use super::post::PixelLayers;
use super::scene_build::RenderScene;
use super::settings::RenderSettings;
use super::trace::{Pixel, Tracer};
use bevy::math::Vec4;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Pixel rectangle `[x0, y0, x1, y1)` inside the output image.
pub type Region = [u32; 4];

pub enum Msg {
    /// A block of pixels `w` wide at (`x0`, `y0`) (a tile, or the final image).
    Block {
        x0: u32,
        y0: u32,
        w: u32,
        display: Vec<Vec4>,
        hdr: Vec<Vec4>,
        layers: Vec<PixelLayers>,
    },
    /// Cycles: a whole new image.
    Image(FrameBuffer),
    Done(Result<String, String>),
}

pub struct RenderJob {
    rx: Mutex<Receiver<Msg>>,
    cancel: Arc<AtomicBool>,
    pub started: Instant,
    pub px_done: u64,
    pub px_total: u64,
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

/// Tile edge in pixels: workers pull 32x32 tiles from a shared counter, so
/// every core stays busy and each tile's rays stay coherent in the cache.
pub const TILE: u32 = 32;

/// A finished tile: `[x0, y0, w, h]` in image pixels and its pixels.
pub type Tile = ([u32; 4], Vec<Pixel>);

/// Tiles covering `region`, row by row.
pub fn tiles(region: Region) -> Vec<[u32; 4]> {
    let [x0, y0, x1, y1] = region;
    let mut out = vec![];
    for ty in (y0..y1).step_by(TILE as usize) {
        for tx in (x0..x1).step_by(TILE as usize) {
            out.push([tx, ty, TILE.min(x1 - tx), TILE.min(y1 - ty)]);
        }
    }
    out
}

/// Renders a region synchronously (tests and the job worker); `on_tile`
/// sees each tile as it finishes.
pub fn raytrace_region(
    rs: &RenderScene,
    cfg: &RenderSettings,
    region: Region,
    cancel: &AtomicBool,
    mut on_tile: impl FnMut([u32; 4], &[Pixel]),
) -> Vec<Pixel> {
    let [x0, y0, x1, y1] = region;
    let (rw, rh) = ((x1 - x0) as usize, (y1 - y0) as usize);
    let tracer = Tracer::new(rs, cfg);
    let todo = tiles(region);
    let next = AtomicUsize::new(0);
    let (tx, rx) = channel::<Tile>();
    let workers = std::thread::available_parallelism().map_or(4, |n| n.get()).clamp(1, todo.len().max(1));
    let mut out = vec![Pixel::EMPTY; rw * rh];
    std::thread::scope(|s| {
        for _ in 0..workers {
            let (tx, tracer, next, todo) = (tx.clone(), &tracer, &next, &todo);
            s.spawn(move || {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(&[tx0, ty0, w, h]) = todo.get(i) else { break };
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    let px = (ty0..ty0 + h)
                        .flat_map(|y| (tx0..tx0 + w).map(move |x| (x, y)))
                        .map(|(x, y)| tracer.pixel(x, y, cfg.width, cfg.height))
                        .collect();
                    if tx.send(([tx0, ty0, w, h], px)).is_err() {
                        break;
                    }
                }
            });
        }
        drop(tx);
        for (t, px) in rx {
            on_tile(t, &px);
            let [tx0, ty0, w, _] = t;
            for (r, row) in px.chunks(w as usize).enumerate() {
                let at = (ty0 - y0) as usize + r;
                let c = (tx0 - x0) as usize;
                out[at * rw + c..at * rw + c + w as usize].copy_from_slice(row);
            }
        }
    });
    if !cancel.load(Ordering::Relaxed) {
        super::postfx::run(&mut out, rw, rh, rs, cfg);
    }
    out
}

fn block(x0: u32, y0: u32, w: u32, px: &[Pixel]) -> Msg {
    let (display, hdr) = px.iter().map(|p| (p.display, p.hdr)).unzip();
    Msg::Block { x0, y0, w, display, hdr, layers: px.iter().map(|p| p.layers).collect() }
}

impl RenderJob {
    fn new(px_total: u64) -> (Self, Sender<Msg>, Arc<AtomicBool>) {
        let (tx, rx) = channel();
        let cancel = Arc::new(AtomicBool::new(false));
        let job = Self { rx: Mutex::new(rx), cancel: cancel.clone(), started: Instant::now(), px_done: 0, px_total };
        (job, tx, cancel)
    }

    /// Starts the built-in raytracer on a background thread.
    pub fn raytrace(rs: RenderScene, cfg: RenderSettings, region: Region) -> Self {
        let (job, tx, cancel) = Self::new(u64::from(region[2] - region[0]) * u64::from(region[3] - region[1]));
        std::thread::spawn(move || {
            let t0 = Instant::now();
            let w = region[2] - region[0];
            let px = raytrace_region(&rs, &cfg, region, &cancel, |[x0, y0, tw, _], tile| {
                let _ = tx.send(block(x0, y0, tw, tile));
            });
            if cancel.load(Ordering::Relaxed) {
                let _ = tx.send(Msg::Done(Err("Render cancelled".into())));
                return;
            }
            let _ = tx.send(block(region[0], region[1], w, &px));
            let _ = tx.send(Msg::Done(Ok(format!("Rendered {}x{} in {:.1}s", cfg.width, cfg.height, t0.elapsed().as_secs_f32()))));
        });
        job
    }

    /// Exports the scene and runs the Cycles CLI on a background thread.
    pub fn cycles(rs: RenderScene, cfg: RenderSettings) -> Self {
        let (job, tx, cancel) = Self::new(u64::from(cfg.width) * u64::from(cfg.height));
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

    /// Applies streamed tiles to `fb`; returns the final status once done.
    pub fn poll(&mut self, fb: &mut FrameBuffer) -> Option<Result<String, String>> {
        let rx = self.rx.lock().ok()?;
        let mut done = None;
        while let Ok(m) = rx.try_recv() {
            match m {
                Msg::Block { x0, y0, w, display, hdr, layers } => {
                    self.px_done += display.len() as u64;
                    fb.put_block(x0, y0, w, &display, &hdr);
                    fb.put_layers(x0, y0, w, &layers);
                }
                Msg::Image(img) => {
                    let rev = fb.rev + 1;
                    *fb = img;
                    fb.rev = rev;
                    self.px_done = self.px_total;
                }
                Msg::Done(r) => done = Some(r),
            }
        }
        done
    }

    pub fn progress(&self) -> f32 {
        (self.px_done.min(self.px_total) as f64 / self.px_total.max(1) as f64) as f32
    }
}
