//! Step D4: the GPU solver against the CPU reference, frame by frame.

use super::gpu::GpuSim;
use super::{Field, SimParams, SimState};

/// Relative L1 difference of two fields.
fn rel(a: &Field, b: &Field) -> f32 {
    let (d, s) = a.d.iter().zip(&b.d).fold((0.0f64, 0.0f64), |(d, s), (x, y)| (d + f64::from((x - y).abs()), s + f64::from(x.abs())));
    (d / s.max(1e-9)) as f32
}

/// The GPU and CPU solvers, the same settings, `frames` frames.
fn both(p: &SimParams, frames: u32) -> Option<Vec<(SimState, SimState)>> {
    let mut g = match GpuSim::new(p) {
        Ok(g) => g,
        Err(e) if std::env::var_os("MINIMAX_REQUIRE_GPU").is_none() => {
            eprintln!("skipping GPU smoke parity: {e}");
            return None;
        }
        Err(e) => panic!("{e}"),
    };
    let mut c = SimState::new(p);
    let mut out = Vec::new();
    for _ in 0..frames {
        g.step_frame(p).expect("GPU frame");
        c.step_frame(p);
        out.push((g.state.clone(), c.clone()));
    }
    Some(out)
}

fn fire() -> SimParams {
    SimParams { size: [1.5, 1.5, 3.0], resolution: 24, source_radius: 0.3, source_height: 0.35, fuel: 3.0, smoke: 0.3, heat: 1.2, buoyancy: 2.0, swirl: 0.5, wind: [0.4, 0.0], pressure_iters: 31, frames: 12, ..SimParams::default() }
}

#[test]
fn gpu_solver_matches_the_cpu_frame_by_frame() {
    let p = fire();
    let Some(frames) = both(&p, 12) else { return };
    for (k, (g, c)) in frames.iter().enumerate() {
        assert_eq!((g.frame, g.n), (c.frame, c.n));
        assert!((g.time - c.time).abs() < 1e-5, "same substeps: {} vs {}", g.time, c.time);
        let (d, t, w) = (rel(&g.density, &c.density), rel(&g.temperature, &c.temperature), rel(&g.vel[2], &c.vel[2]));
        eprintln!("frame {}: density {d:.2e}, temperature {t:.2e}, upward speed {w:.2e}", k + 1);
        // Same arithmetic; only float rounding (fused multiply-adds on
        // some GPUs) may differ, growing slowly with time.
        let tol = 1e-3;
        assert!(d < tol && t < tol && w < tol, "frame {}: {d} {t} {w}", k + 1);
    }
    let (g, c) = frames.last().unwrap();
    assert!(g.density.max() > 0.1 && g.fuel.max() > 0.0, "the GPU fire burns");
    assert!((g.density.max() - c.density.max()).abs() < 1e-3 * c.density.max());
}

#[test]
fn gpu_solver_handles_an_even_pressure_count_and_no_swirl() {
    let p = SimParams { pressure_iters: 20, swirl: 0.0, fuel: 0.0, wind: [0.0, -0.3], ..fire() };
    let Some(frames) = both(&p, 4) else { return };
    let (g, c) = frames.last().unwrap();
    assert!(rel(&g.density, &c.density) < 1e-3, "{}", rel(&g.density, &c.density));
    assert!(rel(&g.vel[1], &c.vel[1]) < 1e-3);
}
