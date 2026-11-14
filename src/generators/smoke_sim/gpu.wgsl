// The smoke solver on the GPU (matches `solver.rs` step for step; the
// host side is `gpu.rs`). Every field is a flat array of cells indexed
// `(z * ny + y) * nx + x`; vectors and the three scalars are stored as
// three such arrays back to back.

struct Params {
    n: vec3<u32>,
    cells: u32,
    h: f32,
    dt: f32,
    buoyancy: f32,
    weight: f32,
    wind: vec2<f32>,
    wind_k: f32,
    // Swirl strength times gain times h (0 = off).
    eps: f32,
    most: f32,
    ignition: f32,
    heat_release: f32,
    soot: f32,
    fade: f32,
    cool: f32,
    emits: u32,
    pad: u32,
}

@group(0) @binding(0) var<uniform> P: Params;
// Velocity x, y, z and its next value.
@group(0) @binding(1) var<storage, read_write> vel: array<f32>;
@group(0) @binding(2) var<storage, read_write> vel_out: array<f32>;
// Smoke density, temperature, fuel and their next values.
@group(0) @binding(3) var<storage, read_write> sca: array<f32>;
@group(0) @binding(4) var<storage, read_write> sca_out: array<f32>;
// Scratch: 0 swirl strength or divergence, 1 / 2 pressure, 3 / 4 MacCormack.
@group(0) @binding(5) var<storage, read_write> tmp: array<f32>;
@group(0) @binding(6) var<storage, read_write> emit_list: array<f32>;
@group(0) @binding(7) var<storage, read_write> stats: array<atomic<u32>>;

// Which scalar (advection) or pressure slot (gradient) a pipeline uses.
override FIELD: u32 = 0u;

fn ni() -> vec3<i32> { return vec3<i32>(P.n); }

fn id(c: vec3<i32>) -> u32 {
    let n = ni();
    return u32((c.z * n.y + c.y) * n.x + c.x);
}

// Index of a cell that may lie outside (clamped to the edge).
fn at(c: vec3<i32>) -> u32 { return id(clamp(c, vec3<i32>(0), ni() - 1)); }

fn lerp(a: f32, b: f32, t: f32) -> f32 { return a + (b - a) * t; }

// Trilinear value of the field starting at `base` (in `vel`, `sca` or
// `tmp`, by `src`) at cell coordinates `p`.
fn fetch(src: u32, i: u32) -> f32 {
    if (src == 0u) { return vel[i]; }
    if (src == 1u) { return sca[i]; }
    return tmp[i];
}

struct Corners { a: vec3<i32>, b: vec3<i32>, f: vec3<f32> }

fn corners(p: vec3<f32>) -> Corners {
    let hi = vec3<f32>(P.n) - 1.0;
    let q = clamp(p, vec3<f32>(0.0), hi);
    let a = vec3<i32>(q);
    let b = min(a + 1, ni() - 1);
    return Corners(a, b, q - vec3<f32>(a));
}

fn sample(src: u32, base: u32, p: vec3<f32>) -> f32 {
    let c = corners(p);
    let x00 = lerp(fetch(src, base + id(vec3<i32>(c.a.x, c.a.y, c.a.z))), fetch(src, base + id(vec3<i32>(c.b.x, c.a.y, c.a.z))), c.f.x);
    let x10 = lerp(fetch(src, base + id(vec3<i32>(c.a.x, c.b.y, c.a.z))), fetch(src, base + id(vec3<i32>(c.b.x, c.b.y, c.a.z))), c.f.x);
    let x01 = lerp(fetch(src, base + id(vec3<i32>(c.a.x, c.a.y, c.b.z))), fetch(src, base + id(vec3<i32>(c.b.x, c.a.y, c.b.z))), c.f.x);
    let x11 = lerp(fetch(src, base + id(vec3<i32>(c.a.x, c.b.y, c.b.z))), fetch(src, base + id(vec3<i32>(c.b.x, c.b.y, c.b.z))), c.f.x);
    return lerp(lerp(x00, x10, c.f.y), lerp(x01, x11, c.f.y), c.f.z);
}

fn cell(g: vec3<u32>) -> bool { return all(g < P.n); }

// Linear kernels: index of a 2D dispatch of 64-wide groups.
fn lin(g: vec3<u32>, groups: vec3<u32>) -> u32 { return g.x + g.y * groups.x * 64u; }

@compute @workgroup_size(64)
fn emit(@builtin(global_invocation_id) g: vec3<u32>, @builtin(num_workgroups) w: vec3<u32>) {
    let k = lin(g, w);
    if (k >= P.emits) { return; }
    let e = k * 8u;
    let i = bitcast<u32>(emit_list[e]);
    let s = emit_list[e + 1u];
    let c = P.cells;
    sca[i] = max(sca[i], emit_list[e + 2u]);
    sca[c + i] = max(sca[c + i], emit_list[e + 3u]);
    sca[2u * c + i] += emit_list[e + 4u];
    vel[2u * c + i] += (emit_list[e + 5u] - vel[2u * c + i]) * s;
    vel[i] += emit_list[e + 6u];
    vel[c + i] += emit_list[e + 7u];
}

@compute @workgroup_size(4, 4, 4)
fn forces(@builtin(global_invocation_id) g: vec3<u32>) {
    if (!cell(g)) { return; }
    let i = id(vec3<i32>(g));
    let c = P.cells;
    vel[2u * c + i] += P.dt * (P.buoyancy * sca[c + i] - P.weight * sca[i]);
    if (P.wind.x != 0.0) { vel[i] += (P.wind.x - vel[i]) * P.wind_k; }
    if (P.wind.y != 0.0) { vel[c + i] += (P.wind.y - vel[c + i]) * P.wind_k; }
}

fn curl(p: vec3<i32>) -> vec3<f32> {
    let c = P.cells;
    let s = 2.0 * P.h;
    let ex = vec3<i32>(1, 0, 0);
    let ey = vec3<i32>(0, 1, 0);
    let ez = vec3<i32>(0, 0, 1);
    let dwy = (vel[2u * c + at(p + ey)] - vel[2u * c + at(p - ey)]) / s;
    let dvz = (vel[c + at(p + ez)] - vel[c + at(p - ez)]) / s;
    let duz = (vel[at(p + ez)] - vel[at(p - ez)]) / s;
    let dwx = (vel[2u * c + at(p + ex)] - vel[2u * c + at(p - ex)]) / s;
    let dvx = (vel[c + at(p + ex)] - vel[c + at(p - ex)]) / s;
    let duy = (vel[at(p + ey)] - vel[at(p - ey)]) / s;
    return vec3<f32>(dwy - dvz, duz - dwx, dvx - duy);
}

@compute @workgroup_size(4, 4, 4)
fn curl_mag(@builtin(global_invocation_id) g: vec3<u32>) {
    if (!cell(g)) { return; }
    tmp[id(vec3<i32>(g))] = length(curl(vec3<i32>(g)));
}

@compute @workgroup_size(4, 4, 4)
fn swirl(@builtin(global_invocation_id) g: vec3<u32>) {
    if (!cell(g)) { return; }
    let p = vec3<i32>(g);
    let i = id(p);
    let ex = vec3<i32>(1, 0, 0);
    let ey = vec3<i32>(0, 1, 0);
    let ez = vec3<i32>(0, 0, 1);
    let gr = vec3<f32>(tmp[at(p + ex)] - tmp[at(p - ex)], tmp[at(p + ey)] - tmp[at(p - ey)], tmp[at(p + ez)] - tmp[at(p - ez)]);
    let l = length(gr);
    let nrm = select(vec3<f32>(0.0), gr / l, l > 0.0);
    let f = cross(nrm, curl(p)) * P.eps;
    let c = P.cells;
    vel_out[i] = vel[i] + P.dt * f.x;
    vel_out[c + i] = vel[c + i] + P.dt * f.y;
    vel_out[2u * c + i] = vel[2u * c + i] + P.dt * f.z;
}

@compute @workgroup_size(4, 4, 4)
fn burn(@builtin(global_invocation_id) g: vec3<u32>) {
    if (!cell(g)) { return; }
    let i = id(vec3<i32>(g));
    let c = P.cells;
    let f = sca[2u * c + i];
    if (f > 0.0 && sca[c + i] >= P.ignition) {
        let b = min(f, P.most);
        sca[2u * c + i] = f - b;
        sca[c + i] += P.heat_release * b;
        sca[i] += P.soot * b;
    }
}

@compute @workgroup_size(4, 4, 4)
fn divergence(@builtin(global_invocation_id) g: vec3<u32>) {
    if (!cell(g)) { return; }
    let p = vec3<i32>(g);
    let c = P.cells;
    let w = 2u * c;
    var below = vel[w + at(p - vec3<i32>(0, 0, 1))];
    if (p.z == 0) { below = -vel[w + id(p)]; }
    let d = vel[at(p + vec3<i32>(1, 0, 0))] - vel[at(p - vec3<i32>(1, 0, 0))] + vel[c + at(p + vec3<i32>(0, 1, 0))] - vel[c + at(p - vec3<i32>(0, 1, 0))] + vel[w + at(p + vec3<i32>(0, 0, 1))] - below;
    tmp[id(p)] = d / (2.0 * P.h);
}

// Pressure in slot `slot` (0 outside the open sides and top, mirrored
// below the floor).
fn pv(slot: u32, p: vec3<i32>) -> f32 {
    let n = ni();
    if (p.x < 0 || p.y < 0 || p.x >= n.x || p.y >= n.y || p.z >= n.z) { return 0.0; }
    return tmp[slot * P.cells + id(vec3<i32>(p.x, p.y, max(p.z, 0)))];
}

fn jacobi(g: vec3<u32>, src: u32, dst: u32) {
    if (!cell(g)) { return; }
    let p = vec3<i32>(g);
    let s = pv(src, p + vec3<i32>(1, 0, 0)) + pv(src, p - vec3<i32>(1, 0, 0)) + pv(src, p + vec3<i32>(0, 1, 0)) + pv(src, p - vec3<i32>(0, 1, 0)) + pv(src, p + vec3<i32>(0, 0, 1)) + pv(src, p - vec3<i32>(0, 0, 1));
    let i = id(p);
    tmp[dst * P.cells + i] = (s - P.h * P.h * tmp[i]) / 6.0;
}

@compute @workgroup_size(4, 4, 4)
fn jacobi_pq(@builtin(global_invocation_id) g: vec3<u32>) { jacobi(g, 1u, 2u); }

@compute @workgroup_size(4, 4, 4)
fn jacobi_qp(@builtin(global_invocation_id) g: vec3<u32>) { jacobi(g, 2u, 1u); }

@compute @workgroup_size(4, 4, 4)
fn gradient(@builtin(global_invocation_id) g: vec3<u32>) {
    if (!cell(g)) { return; }
    let p = vec3<i32>(g);
    let i = id(p);
    let c = P.cells;
    let s = 2.0 * P.h;
    let ex = vec3<i32>(1, 0, 0);
    let ey = vec3<i32>(0, 1, 0);
    let ez = vec3<i32>(0, 0, 1);
    vel_out[i] = vel[i] - (pv(FIELD, p + ex) - pv(FIELD, p - ex)) / s;
    vel_out[c + i] = vel[c + i] - (pv(FIELD, p + ey) - pv(FIELD, p - ey)) / s;
    var w = vel[2u * c + i] - (pv(FIELD, p + ez) - pv(FIELD, p - ez)) / s;
    if (p.z == 0) { w = max(w, 0.0); }
    vel_out[2u * c + i] = w;
}

// Velocity at cell coordinates `q`, in cells per second.
fn cells_per_s(q: vec3<f32>) -> vec3<f32> {
    let c = P.cells;
    return vec3<f32>(sample(0u, 0u, q), sample(0u, c, q), sample(0u, 2u * c, q)) / P.h;
}

// Where the gas now at `c` was `dt` seconds ago (RK2).
fn back(c: vec3<f32>, dt: f32) -> vec3<f32> {
    let mid = c - 0.5 * dt * cells_per_s(c);
    return c - dt * cells_per_s(mid);
}

@compute @workgroup_size(4, 4, 4)
fn adv_fwd(@builtin(global_invocation_id) g: vec3<u32>) {
    if (!cell(g)) { return; }
    let i = id(vec3<i32>(g));
    tmp[3u * P.cells + i] = sample(1u, FIELD * P.cells, back(vec3<f32>(g), P.dt));
}

@compute @workgroup_size(4, 4, 4)
fn adv_rev(@builtin(global_invocation_id) g: vec3<u32>) {
    if (!cell(g)) { return; }
    let i = id(vec3<i32>(g));
    tmp[4u * P.cells + i] = sample(2u, 3u * P.cells, back(vec3<f32>(g), -P.dt));
}

@compute @workgroup_size(4, 4, 4)
fn adv_fin(@builtin(global_invocation_id) g: vec3<u32>) {
    if (!cell(g)) { return; }
    let i = id(vec3<i32>(g));
    let base = FIELD * P.cells;
    let k = corners(back(vec3<f32>(g), P.dt));
    var lo = 3.4e38;
    var hi = -3.4e38;
    for (var j = 0u; j < 8u; j++) {
        let q = select(k.a, k.b, vec3<bool>((j & 1u) != 0u, (j & 2u) != 0u, (j & 4u) != 0u));
        let v = sca[base + id(q)];
        lo = min(lo, v);
        hi = max(hi, v);
    }
    let c = P.cells;
    let v = tmp[3u * c + i] + 0.5 * (sca[base + i] - tmp[4u * c + i]);
    sca_out[base + i] = max(clamp(v, lo, hi), 0.0);
}

@compute @workgroup_size(4, 4, 4)
fn adv_vel(@builtin(global_invocation_id) g: vec3<u32>) {
    if (!cell(g)) { return; }
    let i = id(vec3<i32>(g));
    let q = back(vec3<f32>(g), P.dt);
    let c = P.cells;
    vel_out[i] = sample(0u, 0u, q);
    vel_out[c + i] = sample(0u, c, q);
    vel_out[2u * c + i] = sample(0u, 2u * c, q);
}

@compute @workgroup_size(4, 4, 4)
fn fade(@builtin(global_invocation_id) g: vec3<u32>) {
    if (!cell(g)) { return; }
    let i = id(vec3<i32>(g));
    sca[i] *= P.fade;
    sca[P.cells + i] *= P.cool;
}

@compute @workgroup_size(64)
fn max_speed(@builtin(global_invocation_id) g: vec3<u32>, @builtin(num_workgroups) w: vec3<u32>) {
    let k = lin(g, w);
    if (k >= 3u * P.cells) { return; }
    atomicMax(&stats[0], bitcast<u32>(abs(vel[k])));
}
