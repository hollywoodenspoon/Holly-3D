//! 4-wide BVH: the binary BVH collapsed so each node holds four child
//! boxes in SoA form (`Vec4` per coordinate). One traversal step tests all
//! four slabs at once with SIMD (glam's `Vec4` is SSE / NEON backed).

use super::bvh::Tri;
use super::bvh::{Hit, Node, intersect};
use bevy::math::{Vec3, Vec4};

/// One child slot: an inner wide node (`count == 0`, `index` = node) or a
/// leaf (`index` = first triangle, `count` triangles).
#[derive(Clone, Copy, Debug, Default)]
struct Slot {
    index: u32,
    count: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Node4 {
    lo: [Vec4; 3],
    hi: [Vec4; 3],
    slots: [Slot; 4],
    /// Number of used slots (1..=4).
    n: u32,
}

/// Collapses the binary tree rooted at node 0 into wide nodes.
pub fn collapse(bin: &[Node]) -> Vec<Node4> {
    let mut out = vec![];
    if !bin.is_empty() {
        build(bin, 0, &mut out);
    }
    out
}

fn area(n: &Node) -> f32 {
    let e = (n.hi - n.lo).max(Vec3::ZERO);
    e.x * e.y + e.y * e.z + e.z * e.x
}

fn children(bin: &[Node], i: usize) -> [usize; 2] {
    [i + 1, bin[i].start as usize]
}

fn build(bin: &[Node], root: usize, out: &mut Vec<Node4>) -> u32 {
    // Gather up to four descendants, opening the largest inner node first.
    let mut kids: Vec<usize> = if bin[root].count > 0 { vec![root] } else { children(bin, root).to_vec() };
    while kids.len() < 4 {
        let open =
            kids.iter().enumerate().filter(|(_, k)| bin[**k].count == 0).max_by(|a, b| area(&bin[*a.1]).total_cmp(&area(&bin[*b.1])));
        let Some((at, &k)) = open else { break };
        kids.splice(at..=at, children(bin, k));
    }
    let me = out.len();
    out.push(Node4 { lo: [Vec4::ZERO; 3], hi: [Vec4::ZERO; 3], slots: [Slot::default(); 4], n: kids.len() as u32 });
    for (lane, &k) in kids.iter().enumerate() {
        let n = bin[k];
        let slot = if n.count > 0 { Slot { index: n.start, count: n.count } } else { Slot { index: build(bin, k, out), count: 0 } };
        let node = &mut out[me];
        for a in 0..3 {
            node.lo[a][lane] = n.lo[a];
            node.hi[a][lane] = n.hi[a];
        }
        node.slots[lane] = slot;
    }
    me as u32
}

/// Entry / exit distances of the ray against the node's four boxes.
#[inline]
fn slab4(n: &Node4, o: Vec3, inv: Vec3, limit: f32) -> (Vec4, u32) {
    let mut t0 = Vec4::ZERO;
    let mut t1 = Vec4::splat(limit);
    for a in 0..3 {
        let (oa, ia) = (Vec4::splat(o[a]), Vec4::splat(inv[a]));
        let p = (n.lo[a] - oa) * ia;
        let q = (n.hi[a] - oa) * ia;
        t0 = t0.max(p.min(q));
        t1 = t1.min(p.max(q));
    }
    (t0, t0.cmple(t1).bitmask() & ((1 << n.n) - 1))
}

/// Closest (or any, for shadow rays) hit along `o + t d`, `t` in (eps, tmax).
pub fn walk(nodes: &[Node4], tris: &[Tri], o: Vec3, d: Vec3, tmax: f32, any: bool) -> Option<Hit> {
    if nodes.is_empty() {
        return None;
    }
    // No exact zero components: a ray running along a box face with its
    // origin on that face would get 0 * inf = NaN and miss the box.
    let safe = |v: f32| if v.abs() < 1e-20 { 1e-20f32.copysign(v) } else { v };
    let inv = Vec3::ONE / Vec3::new(safe(d.x), safe(d.y), safe(d.z));
    let mut best: Option<Hit> = None;
    let mut limit = tmax;
    // (entry distance, slot): popped nearest first.
    let mut stack = [(0.0f32, Slot::default()); 96];
    stack[0] = (0.0, Slot { index: 0, count: 0 });
    let mut sp = 1;
    while sp > 0 {
        sp -= 1;
        let (tin, slot) = stack[sp];
        if tin > limit {
            continue;
        }
        if slot.count > 0 {
            let leaf = tris.iter().enumerate().skip(slot.index as usize).take(slot.count as usize);
            for (i, tri) in leaf {
                if let Some((t, u, v)) = intersect(tri, o, d)
                    && t > 1e-4
                    && t < limit
                {
                    limit = t;
                    best = Some(Hit { t, tri: i, u, v });
                    if any {
                        return best;
                    }
                }
            }
            continue;
        }
        let n = &nodes[slot.index as usize];
        let (t0, mask) = slab4(n, o, inv, limit);
        // Push hit children far-to-near so the nearest is processed first.
        let mut hits: [(f32, Slot); 4] = [(0.0, Slot::default()); 4];
        let mut k = 0;
        for lane in 0..4 {
            if mask & (1 << lane) != 0 {
                hits[k] = (t0[lane], n.slots[lane]);
                k += 1;
            }
        }
        hits[..k].sort_unstable_by(|a, b| b.0.total_cmp(&a.0));
        for h in &hits[..k] {
            if sp < stack.len() {
                stack[sp] = *h;
                sp += 1;
            }
        }
    }
    best
}
