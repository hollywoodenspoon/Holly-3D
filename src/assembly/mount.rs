//! Mount points for the J tool (a vertex, a face / cap center or an edge
//! loop center, with an outward normal) and the snap that mates part A's
//! mount onto part B's.

use super::{Frame, Joint, JointKind};
use crate::mesh::adjacency::Adjacency;
use crate::mesh::{EditablePoly, PolyId, VertexId};
use crate::scene::ObjId;
use bevy::math::{Mat4, Quat, Vec3};

/// A picked mount: world point and outward normal on object `obj`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mount {
    pub obj: ObjId,
    pub point: Vec3,
    pub normal: Vec3,
}

/// What a click landed on, in mesh space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MountAt {
    Vertex(VertexId),
    /// An edge (its two vertices): the center of its closed edge loop.
    Edge(VertexId, VertexId),
    Face,
}

fn world_normal(world: Mat4, n: Vec3) -> Vec3 {
    world.inverse().transpose().transform_vector3(n).normalize_or(Vec3::Z)
}

/// The closed edge loop through edge `a`-`b` (quad loops and open borders).
pub fn closed_loop(m: &EditablePoly, a: VertexId, b: VertexId) -> Option<Vec<VertexId>> {
    let adj = Adjacency::build(m);
    let mut out = vec![a, b];
    let (mut from, mut cur) = (a, b);
    for _ in 0..m.vertices.len() {
        let n = m.loop_next(&adj, from, cur)?;
        if n == a {
            return Some(out);
        }
        if out.contains(&n) {
            return None;
        }
        out.push(n);
        (from, cur) = (cur, n);
    }
    None
}

/// Newell normal of a closed point loop (unnormalized area vector).
fn newell(pts: &[Vec3]) -> Vec3 {
    (0..pts.len()).map(|i| pts[i].cross(pts[(i + 1) % pts.len()])).sum()
}

/// Mount point and normal (world space) for a click on polygon `poly`.
/// Loop normals have no inside / outside, so they face `toward` (the viewer).
pub fn mount_point(m: &EditablePoly, world: Mat4, poly: PolyId, at: MountAt, toward: Vec3) -> (Vec3, Vec3) {
    let face_n = world_normal(world, m.poly_normal(poly));
    let corners: Vec<Vec3> = m.polygons[poly].corners.iter().map(|c| m.pos(c.vertex_id)).collect();
    let center = corners.iter().sum::<Vec3>() / corners.len().max(1) as f32;
    match at {
        MountAt::Vertex(v) => (world.transform_point3(m.pos(v)), face_n),
        MountAt::Edge(a, b) => match closed_loop(m, a, b) {
            Some(lp) if lp.len() >= 3 => {
                let pts: Vec<Vec3> = lp.iter().map(|&v| world.transform_point3(m.pos(v))).collect();
                let c = pts.iter().sum::<Vec3>() / pts.len() as f32;
                let n = newell(&pts).normalize_or(face_n);
                (c, if n.dot(toward) < 0.0 { -n } else { n })
            }
            _ => (world.transform_point3((m.pos(a) + m.pos(b)) * 0.5), face_n),
        },
        MountAt::Face => (world.transform_point3(center), face_n),
    }
}

/// New rigid pose of the child so its mount sits on the parent's mount,
/// face to face (normals opposed).
pub fn mate(child: Frame, a: &Mount, b: &Mount) -> Frame {
    let q = Quat::from_rotation_arc(a.normal.normalize_or(Vec3::Z), -b.normal.normalize_or(Vec3::Z));
    Frame::new(b.point + q * (child.pos - a.point), (q * child.rot).normalize())
}

/// Joint between an already-mated child (pose `child`) and parent (pose
/// `parent`), with its mount frame at `b` (Z along the parent's normal).
pub fn joint_at(kind: JointKind, child: (ObjId, Frame), parent: (ObjId, Frame), b: &Mount) -> Joint {
    let w = Frame::from_normal(b.point, b.normal);
    Joint { child: child.0, parent: parent.0, kind, parent_frame: parent.1.inverse() * w, child_frame: child.1.inverse() * w, posed: None }
}

/// Piston between a housing (mount `a`) and a rod (mount `b`): neither part
/// moves; each mount frame's Z axis points at the other mount.
pub fn piston_at(housing: (ObjId, Frame), rod: (ObjId, Frame), a: &Mount, b: &Mount) -> Joint {
    let d = b.point - a.point;
    Joint {
        child: housing.0,
        parent: rod.0,
        kind: JointKind::Piston,
        child_frame: housing.1.inverse() * Frame::from_normal(a.point, d),
        parent_frame: rod.1.inverse() * Frame::from_normal(b.point, -d),
        posed: None,
    }
}
