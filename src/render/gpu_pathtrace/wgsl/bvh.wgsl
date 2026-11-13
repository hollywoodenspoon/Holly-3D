// Ray / triangle and BVH traversal (matches src/render/bvh).

struct Hit { t: f32, u: f32, v: f32, tri: u32 }

fn intersect(i: u32, o: vec3<f32>, d: vec3<f32>, best: ptr<function, Hit>) {
    let t = tris[i];
    let e1 = t.p1.xyz - t.p0.xyz;
    let e2 = t.p2.xyz - t.p0.xyz;
    let pv = cross(d, e2);
    let det = dot(e1, pv);
    if (abs(det) < 1e-12) { return; }
    let inv = 1.0 / det;
    let s = o - t.p0.xyz;
    let uu = dot(s, pv) * inv;
    if (uu < 0.0 || uu > 1.0) { return; }
    let q = cross(s, e1);
    let vv = dot(d, q) * inv;
    if (vv < 0.0 || uu + vv > 1.0) { return; }
    let tt = dot(e2, q) * inv;
    if (tt > 0.0 && tt < (*best).t) {
        // PR 4: alpha-tested see-through surfaces skip the hit here.
        if (FEAT_OPACITY && !opaque_at(i, uu, vv)) { return; }
        *best = Hit(tt, uu, vv, i);
    }
}

fn slab(n: Node, o: vec3<f32>, inv: vec3<f32>, tmax: f32) -> bool {
    let a = (n.lo.xyz - o) * inv;
    let b = (n.hi.xyz - o) * inv;
    let t0 = max(max(min(a.x, b.x), min(a.y, b.y)), max(min(a.z, b.z), 0.0));
    let t1 = min(min(max(a.x, b.x), max(a.y, b.y)), min(max(a.z, b.z), tmax));
    return t0 <= t1;
}

fn closest(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> Hit {
    var best = Hit(tmax, 0.0, 0.0, 0xffffffffu);
    if (arrayLength(&nodes) == 0u || arrayLength(&tris) == 0u) { return best; }
    let inv = 1.0 / select(d, vec3<f32>(1e-20), abs(d) < vec3<f32>(1e-20));
    var stack: array<u32, 64>;
    var sp = 1;
    stack[0] = 0u;
    while (sp > 0) {
        sp -= 1;
        let ni = stack[sp];
        let n = nodes[ni];
        if (!slab(n, o, inv, best.t)) { continue; }
        let start = bitcast<u32>(n.lo.w);
        let count = bitcast<u32>(n.hi.w);
        if (count > 0u) {
            for (var k = 0u; k < count; k++) { intersect(start + k, o, d, &best); }
        } else if (sp < 62) {
            // The left child follows its parent; `start` is the right child.
            stack[sp] = start;
            stack[sp + 1] = ni + 1u;
            sp += 2;
        }
    }
    return best;
}

fn occluded(o: vec3<f32>, d: vec3<f32>, tmax: f32) -> bool {
    return closest(o, d, tmax).tri != 0xffffffffu;
}

// Hook for alpha testing (PR 4); every surface is opaque until then.
fn opaque_at(i: u32, u: f32, v: f32) -> bool {
    return true;
}
