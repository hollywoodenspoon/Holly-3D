//! The CPU smoke solver (the reference the GPU solver is checked against).
//! One substep: emit at the source, add buoyancy, wind and swirl, burn
//! fuel, make the velocity divergence-free (Jacobi pressure solve), then
//! move everything along the flow (semi-Lagrangian for velocity,
//! MacCormack with clamping for smoke, heat and fuel, which keeps fine
//! tendrils sharp) and let smoke and heat fade.
//!
//! Velocities are in m/s on a collocated grid of cell size `h`. The floor
//! is closed, the sides and top are open (gas leaves the domain).

use super::SimParams;
use super::field::{Field, par_fill};
use bevy::math::Vec3;
use noise::{NoiseFn, Perlin};

/// Scales the Swirl setting so 0..1 spans gentle to strongly curled.
const SWIRL_GAIN: f32 = 4.0;

#[derive(Clone, Debug)]
pub struct SimState {
    pub h: f32,
    pub n: [usize; 3],
    pub vel: [Field; 3],
    pub density: Field,
    pub temperature: Field,
    pub fuel: Field,
    /// Frames simulated so far and the time they cover (s).
    pub frame: u32,
    pub time: f32,
}

impl SimState {
    pub fn new(p: &SimParams) -> Self {
        let (h, n) = p.grid();
        let f = || Field::new(n);
        Self { h, n, vel: [f(), f(), f()], density: f(), temperature: f(), fuel: f(), frame: 0, time: 0.0 }
    }

    /// Simulates one frame (several substeps when the gas moves fast).
    pub fn step_frame(&mut self, p: &SimParams) {
        let dt = 1.0 / p.fps.clamp(1.0, 240.0);
        let speed = self.vel.iter().map(|f| f.d.iter().fold(0.0f32, |m, v| m.max(v.abs()))).chain([p.source_speed.abs(), p.wind[0].abs(), p.wind[1].abs()]).fold(0.0f32, f32::max);
        let steps = ((speed * dt * 3f32.sqrt() / (self.h * 1.5)).ceil() as u32).clamp(1, 6);
        for _ in 0..steps {
            self.substep(p, dt / steps as f32);
        }
        self.frame += 1;
    }

    pub fn substep(&mut self, p: &SimParams, dt: f32) {
        self.emit(p, dt);
        self.forces(p, dt);
        self.swirl(p.swirl, dt);
        self.burn(p, dt);
        self.project(p.pressure_iters.clamp(1, 200));
        self.advect(dt);
        let (fd, fc) = ((-p.dissipation.max(0.0) * dt).exp(), (-p.cooling.max(0.0) * dt).exp());
        self.density.d.iter_mut().for_each(|v| *v *= fd);
        self.temperature.d.iter_mut().for_each(|v| *v *= fc);
        self.time += dt;
    }

    /// Source centre (cell coordinates) and radius (cells).
    pub fn source(&self, p: &SimParams) -> (Vec3, f32) {
        let c = Vec3::new((self.n[0] as f32 - 1.0) * 0.5, (self.n[1] as f32 - 1.0) * 0.5, p.source_height / self.h - 0.5);
        (c, (p.source_radius / self.h).max(1.0))
    }

    fn emit(&mut self, p: &SimParams, dt: f32) {
        let (c, r) = self.source(p);
        let noise = Perlin::new(p.seed);
        let (t, turb) = (f64::from(self.time) * 1.7, p.turbulence.clamp(0.0, 2.0));
        let lo = (c - r - 1.0).max(Vec3::ZERO);
        let hi = (c + r + 1.0).min(Vec3::new(self.n[0] as f32 - 1.0, self.n[1] as f32 - 1.0, self.n[2] as f32 - 1.0));
        for z in lo.z as usize..=hi.z.max(0.0) as usize {
            for y in lo.y as usize..=hi.y as usize {
                for x in lo.x as usize..=hi.x as usize {
                    let q = Vec3::new(x as f32, y as f32, z as f32);
                    let s = (1.0 - (q - c).length_squared() / (r * r)).max(0.0);
                    if s <= 0.0 {
                        continue;
                    }
                    let m = q * (0.35 * self.h.recip().min(4.0) / 4.0).max(0.05);
                    let nz = noise.get([f64::from(m.x), f64::from(m.y), f64::from(m.z), t]) as f32 * turb;
                    let i = self.density.idx(x, y, z);
                    let s = s.sqrt();
                    self.density.d[i] = self.density.d[i].max(p.smoke.max(0.0) * s * (1.0 + nz).max(0.0));
                    self.temperature.d[i] = self.temperature.d[i].max(p.heat.max(0.0) * s);
                    self.fuel.d[i] += p.fuel.max(0.0) * s * dt;
                    let w = p.source_speed * (1.0 + 0.5 * nz);
                    self.vel[2].d[i] += (w - self.vel[2].d[i]) * s;
                }
            }
        }
    }

    fn forces(&mut self, p: &SimParams, dt: f32) {
        let k = (0.5 * dt).min(1.0);
        for (i, w) in self.vel[2].d.iter_mut().enumerate() {
            *w += dt * (p.buoyancy * self.temperature.d[i] - p.weight * self.density.d[i]);
        }
        for (a, wind) in [(0, p.wind[0]), (1, p.wind[1])] {
            if wind != 0.0 {
                self.vel[a].d.iter_mut().for_each(|v| *v += (wind - *v) * k);
            }
        }
    }

    /// Curl of the velocity at a cell (1/s).
    fn curl(&self, x: usize, y: usize, z: usize) -> Vec3 {
        let (x, y, z) = (x as i64, y as i64, z as i64);
        let d = |f: &Field, dx: i64, dy: i64, dz: i64| (f.at(x + dx, y + dy, z + dz) - f.at(x - dx, y - dy, z - dz)) / (2.0 * self.h);
        let [u, v, w] = &self.vel;
        Vec3::new(d(w, 0, 1, 0) - d(v, 0, 0, 1), d(u, 0, 0, 1) - d(w, 1, 0, 0), d(v, 1, 0, 0) - d(u, 0, 1, 0))
    }

    /// Vorticity confinement: pushes the flow around its own eddies.
    fn swirl(&mut self, eps: f32, dt: f32) {
        if eps <= 0.0 {
            return;
        }
        let mut mag = Field::new(self.n);
        par_fill(&mut mag, |x, y, z| self.curl(x, y, z).length());
        let mut out = [Field::new(self.n), Field::new(self.n), Field::new(self.n)];
        for (a, o) in out.iter_mut().enumerate() {
            par_fill(o, |x, y, z| {
                let (xi, yi, zi) = (x as i64, y as i64, z as i64);
                let g = Vec3::new(mag.at(xi + 1, yi, zi) - mag.at(xi - 1, yi, zi), mag.at(xi, yi + 1, zi) - mag.at(xi, yi - 1, zi), mag.at(xi, yi, zi + 1) - mag.at(xi, yi, zi - 1));
                let f = g.normalize_or_zero().cross(self.curl(x, y, z)) * (eps * SWIRL_GAIN * self.h);
                self.vel[a].get(x, y, z) + dt * f[a]
            });
        }
        self.vel = out;
    }

    fn burn(&mut self, p: &SimParams, dt: f32) {
        let most = p.burn_rate.max(0.0) * dt;
        for i in 0..self.fuel.d.len() {
            let f = self.fuel.d[i];
            if f > 0.0 && self.temperature.d[i] >= p.ignition {
                let b = f.min(most);
                self.fuel.d[i] -= b;
                self.temperature.d[i] += p.heat_release * b;
                self.density.d[i] += p.soot * b;
            }
        }
    }

    /// Velocity divergence (1/s), with a closed floor and open sides / top.
    pub fn divergence(&self) -> Field {
        let mut div = Field::new(self.n);
        let [u, v, w] = &self.vel;
        par_fill(&mut div, |x, y, z| {
            let (xi, yi, zi) = (x as i64, y as i64, z as i64);
            let below = if z == 0 { -w.get(x, y, 0) } else { w.at(xi, yi, zi - 1) };
            (u.at(xi + 1, yi, zi) - u.at(xi - 1, yi, zi) + v.at(xi, yi + 1, zi) - v.at(xi, yi - 1, zi) + w.at(xi, yi, zi + 1) - below) / (2.0 * self.h)
        });
        div
    }

    pub fn project(&mut self, iters: u32) {
        let div = self.divergence();
        let n = self.n;
        let (mut p, mut q) = (Field::new(n), Field::new(n));
        let h2 = self.h * self.h;
        // Pressure outside: 0 past the open sides and top, mirrored below the floor.
        let pv = |f: &Field, x: i64, y: i64, z: i64| {
            if x < 0 || y < 0 || x >= n[0] as i64 || y >= n[1] as i64 || z >= n[2] as i64 {
                0.0
            } else {
                f.get(x as usize, y as usize, z.max(0) as usize)
            }
        };
        for _ in 0..iters {
            par_fill(&mut q, |x, y, z| {
                let (x, y, z) = (x as i64, y as i64, z as i64);
                let s = pv(&p, x + 1, y, z) + pv(&p, x - 1, y, z) + pv(&p, x, y + 1, z) + pv(&p, x, y - 1, z) + pv(&p, x, y, z + 1) + pv(&p, x, y, z - 1);
                (s - h2 * div.get(x as usize, y as usize, z as usize)) / 6.0
            });
            std::mem::swap(&mut p, &mut q);
        }
        let mut out = [Field::new(n), Field::new(n), Field::new(n)];
        for (a, o) in out.iter_mut().enumerate() {
            par_fill(o, |x, y, z| {
                let (xi, yi, zi) = (x as i64, y as i64, z as i64);
                let e = [[1, 0, 0], [0, 1, 0], [0, 0, 1]][a];
                let g = (pv(&p, xi + e[0], yi + e[1], zi + e[2]) - pv(&p, xi - e[0], yi - e[1], zi - e[2])) / (2.0 * self.h);
                let v = self.vel[a].get(x, y, z) - g;
                if a == 2 && z == 0 { v.max(0.0) } else { v }
            });
        }
        self.vel = out;
    }

    /// Velocity at a point in cell coordinates, in cells per second.
    fn cells_per_s(&self, q: Vec3) -> Vec3 {
        Vec3::new(self.vel[0].sample(q), self.vel[1].sample(q), self.vel[2].sample(q)) / self.h
    }

    /// Where the gas now at cell `c` was `dt` seconds ago (RK2).
    fn back(&self, c: Vec3, dt: f32) -> Vec3 {
        let mid = c - 0.5 * dt * self.cells_per_s(c);
        c - dt * self.cells_per_s(mid)
    }

    fn advect(&mut self, dt: f32) {
        let cell = |x: usize, y: usize, z: usize| Vec3::new(x as f32, y as f32, z as f32);
        let mut scalars = [Field::new(self.n), Field::new(self.n), Field::new(self.n)];
        for (k, out) in scalars.iter_mut().enumerate() {
            let f = [&self.density, &self.temperature, &self.fuel][k];
            let mut fwd = Field::new(self.n);
            par_fill(&mut fwd, |x, y, z| f.sample(self.back(cell(x, y, z), dt)));
            let mut rev = Field::new(self.n);
            par_fill(&mut rev, |x, y, z| fwd.sample(self.back(cell(x, y, z), -dt)));
            par_fill(out, |x, y, z| {
                let src = self.back(cell(x, y, z), dt);
                let (lo, hi) = f.range(src);
                let i = f.idx(x, y, z);
                (fwd.d[i] + 0.5 * (f.d[i] - rev.d[i])).clamp(lo, hi).max(0.0)
            });
        }
        let mut vel = [Field::new(self.n), Field::new(self.n), Field::new(self.n)];
        for (a, out) in vel.iter_mut().enumerate() {
            par_fill(out, |x, y, z| self.vel[a].sample(self.back(cell(x, y, z), dt)));
        }
        let [d, t, f] = scalars;
        (self.density, self.temperature, self.fuel, self.vel) = (d, t, f, vel);
    }
}
