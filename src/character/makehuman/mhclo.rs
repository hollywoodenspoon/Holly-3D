//! MakeHuman `.mhclo` proxy fitting. Every proxy vertex is bound to a base
//! mesh triangle: `p = w1*v1 + w2*v2 + w3*v3 + offset * scale`, where the
//! per-axis scale follows the distance between two reference vertices, so
//! eyes, clothes and hair follow every morph. Exact single-vertex
//! bindings (`verts` lines with one index) are supported too.

use bevy::math::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Binding {
    pub v: [u32; 3],
    pub w: [f32; 3],
    pub offset: Vec3,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Proxy {
    pub name: String,
    pub obj_file: String,
    pub bindings: Vec<Binding>,
    /// (v1, v2, reference distance) for x, y, z.
    pub scale: [Option<(u32, u32, f32)>; 3],
    pub delete_verts: Vec<u32>,
}

impl Proxy {
    pub fn parse(src: &str) -> Self {
        let mut p = Proxy::default();
        let mut in_verts = false;
        let mut in_delete = false;
        for line in src.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
            let f: Vec<&str> = line.split_ascii_whitespace().collect();
            let num = f[0].parse::<f32>().is_ok();
            // Only section keywords switch sections: other keys (such as
            // `material`) may follow `verts 0` before the binding lines.
            if !num && matches!(f[0], "verts" | "delete_verts" | "weights") {
                in_verts = f[0] == "verts";
                in_delete = f[0] == "delete_verts";
            }
            match f[0] {
                "name" => p.name = f[1..].join(" "),
                "obj_file" => p.obj_file = f.get(1).unwrap_or(&"").to_string(),
                "x_scale" | "y_scale" | "z_scale" if f.len() >= 4 => {
                    let axis = (f[0].as_bytes()[0] - b'x') as usize;
                    p.scale[axis] = Some((f[1].parse().unwrap_or(0), f[2].parse().unwrap_or(0), f[3].parse().unwrap_or(1.0)));
                }
                _ if num && in_verts => {
                    let n: Vec<f32> = f.iter().filter_map(|s| s.parse().ok()).collect();
                    let b = if n.len() >= 9 {
                        Binding { v: [n[0] as u32, n[1] as u32, n[2] as u32], w: [n[3], n[4], n[5]], offset: Vec3::new(n[6], n[7], n[8]) }
                    } else {
                        Binding { v: [n[0] as u32; 3], w: [1.0, 0.0, 0.0], offset: Vec3::ZERO }
                    };
                    p.bindings.push(b);
                }
                _ if in_delete => {
                    // Single indices and `a - b` ranges.
                    let mut k = 0;
                    while k < f.len() {
                        if let Ok(a) = f[k].parse::<u32>() {
                            match (f.get(k + 1), f.get(k + 2).and_then(|s| s.parse::<u32>().ok())) {
                                (Some(&"-"), Some(b)) if b >= a => {
                                    p.delete_verts.extend(a..=b);
                                    k += 2;
                                }
                                _ => p.delete_verts.push(a),
                            }
                        }
                        k += 1;
                    }
                }
                _ => {}
            }
        }
        p
    }

    /// Per-axis offset scale on the (morphed) base `coords`.
    pub fn scales(&self, coords: &[Vec3]) -> Vec3 {
        let mut s = Vec3::ONE;
        for (axis, sc) in self.scale.iter().enumerate() {
            if let Some((a, b, d)) = *sc {
                let (pa, pb) = (coords.get(a as usize), coords.get(b as usize));
                if let (Some(pa), Some(pb)) = (pa, pb) {
                    s[axis] = (pa[axis] - pb[axis]).abs() / d.max(1e-6);
                }
            }
        }
        s
    }

    /// Fitted proxy vertex positions (base-mesh coordinates).
    pub fn fit(&self, coords: &[Vec3]) -> Vec<Vec3> {
        let s = self.scales(coords);
        let at = |i: u32| coords.get(i as usize).copied().unwrap_or(Vec3::ZERO);
        self.bindings.iter().map(|b| at(b.v[0]) * b.w[0] + at(b.v[1]) * b.w[1] + at(b.v[2]) * b.w[2] + b.offset * s).collect()
    }
}

/// Binds an arbitrary point to the three nearest `candidates` base
/// vertices (inverse-distance weights + offset), the way procedural hair
/// and outfit pieces ride on the morphed body.
pub fn bind_point(coords: &[Vec3], candidates: &[u32], p: Vec3) -> Binding {
    let mut best = [(f32::MAX, 0u32); 3];
    for &i in candidates {
        let d = coords[i as usize].distance_squared(p);
        if d < best[2].0 {
            best[2] = (d, i);
            best.sort_by(|a, b| a.0.total_cmp(&b.0));
        }
    }
    let inv: Vec<f32> = best.iter().map(|(d, _)| 1.0 / (d.sqrt() + 1e-3)).collect();
    let sum: f32 = inv.iter().sum();
    let w = [inv[0] / sum, inv[1] / sum, inv[2] / sum];
    let v = [best[0].1, best[1].1, best[2].1];
    let anchor: Vec3 = (0..3).map(|k| coords[v[k] as usize] * w[k]).sum();
    Binding { v, w, offset: p - anchor }
}

/// Position of a binding on morphed `coords` (unscaled offset).
pub fn eval(b: &Binding, coords: &[Vec3]) -> Vec3 {
    (0..3).map(|k| coords[b.v[k] as usize] * b.w[k]).sum::<Vec3>() + b.offset
}
