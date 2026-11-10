//! Hydraulic Piston parts: a housing (cylinder barrel with a gland) or a
//! rod, each with an eyelet at its pivot (pin axis along local Y) and its
//! body along local +Z, so a Piston joint can aim the two at each other.

use crate::mesh::EditablePoly;
use crate::primitives::Primitive;
use bevy::math::{Mat4, Quat, Vec3};

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PistonPart {
    /// Rod (thin, slides in the housing) instead of the housing.
    pub rod: bool,
    pub radius: f32,
    pub length: f32,
    pub eyelet: f32,
    pub sides: u32,
}

impl PistonPart {
    pub fn housing() -> Self {
        Self { rod: false, radius: 0.15, length: 1.2, eyelet: 0.12, sides: 18 }
    }

    pub fn rod() -> Self {
        Self { rod: true, radius: 0.06, length: 1.2, eyelet: 0.1, sides: 14 }
    }

    pub fn build(&self) -> EditablePoly {
        let sides = self.sides.clamp(6, 64);
        let e = self.eyelet.max(0.01);
        let cyl = |radius: f32, height: f32| {
            Primitive::Cylinder { radius: radius.max(0.005), height: height.max(0.01), height_segs: 1, cap_segs: 1, sides }
                .to_editable_poly()
        };
        let mut m = EditablePoly::new();
        let ring = Primitive::Torus { radius1: e, radius2: e * 0.35, segments: sides, sides: 8 }.to_editable_poly();
        m.append(&ring, Mat4::from_quat(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)));
        let base = e * 1.2;
        m.append(&cyl(self.radius, self.length), Mat4::from_translation(Vec3::Z * base));
        if !self.rod {
            // Gland ring at the open end of the barrel.
            let gland = (self.length * 0.08).max(0.01);
            let at = Vec3::Z * (base + self.length - gland);
            m.append(&cyl(self.radius * 1.15, gland), Mat4::from_translation(at));
        }
        m.rebuild_edges();
        m
    }
}
