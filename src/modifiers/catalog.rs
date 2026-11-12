//! The Modifier List catalog: every modifier under a 3ds Max-style group
//! heading, with a one-line description and whether it fits an object.

use crate::scene::SceneObject;

pub struct ModifierInfo {
    pub name: &'static str,
    pub tip: &'static str,
}

const fn m(name: &'static str, tip: &'static str) -> ModifierInfo {
    ModifierInfo { name, tip }
}

/// Group headings and their modifiers, in Modifier List order.
pub const GROUPS: &[(&str, &[ModifierInfo])] = &[
    (
        "Mesh Editing",
        &[
            m("Edit Poly", "Vertex / Edge / Border / Polygon / Element editing on top of the stack"),
            m("Chamfer", "Bevels edges: all, selected, by smoothing group or by angle"),
            m("Symmetry", "Mirrors one half of the mesh onto the other and welds the seam"),
            m("Shell", "Gives surfaces thickness (inner and outer amount)"),
            m("Panel Groove", "Cuts recessed panel lines along the selected edges"),
            m("Edit Normals", "Break, unify and set explicit vertex normals"),
            m("ProOptimizer", "Reduces the polygon count while keeping the shape"),
        ],
    ),
    ("Subdivision", &[m("TurboSmooth", "Catmull-Clark subdivision for smooth surfaces")]),
    (
        "Free-Form Deformers",
        &[
            m("FFD 2x2x2", "Warp the mesh with a 2x2x2 lattice of control points"),
            m("FFD 3x3x3", "Warp the mesh with a 3x3x3 lattice of control points"),
            m("FFD 4x4x4", "Warp the mesh with a 4x4x4 lattice of control points"),
        ],
    ),
    (
        "Parametric Deformers",
        &[
            m("Bend", "Bends the object around an axis"),
            m("Twist", "Twists the object around an axis"),
            m("Taper", "Scales one end of the object down or up"),
            m("Push", "Inflates or shrinks along the vertex normals"),
        ],
    ),
    (
        "UV Coordinates",
        &[
            m("UVW Map", "Planar, box, cylindrical or spherical texture projection"),
            m("Unwrap UVW", "UV editor and one-click Smart UV Unwrap"),
        ],
    ),
    ("Paint", &[m("VertexPaint", "Paint colours straight onto the vertices (no UVs needed)")]),
    (
        "Spline → Mesh",
        &[
            m("Lathe", "Spins a spline profile around an axis (vases, pillars)"),
            m("Extrude", "Pushes a closed spline into a solid"),
            m("Loft", "Skins picked cross-section shapes along this path spline, with twist and scale"),
        ],
    ),
    ("Booleans", &[m("Boolean", "Union, subtract or intersect with other objects")]),
    (
        "Generators",
        &[
            m("GeoGen", "Grows crystals, rocks or stalagmites on the surface"),
            m("Greeble", "Sci-fi panels and widgets over the surface"),
        ],
    ),
    (
        "Demiurge Worlds",
        &[
            m("Demiurge Terrain", "Noise-driven landscape with auto Material IDs"),
            m("Demiurge Ocean", "Gerstner waves with foam"),
        ],
    ),
    (
        "Demiurge Settlements & Destruction",
        &[
            m("Demiurge GhostTown", "A whole settlement on a plane or inside a closed spline"),
            m("Demiurge Medieval Architect", "Medieval buildings on a plane or along a spline"),
            m("Demiurge Battlefield & Ruin", "Craters, rubble and damage"),
        ],
    ),
];

/// Pre-Demiurge modifier names (saved modifier-button sets, scripts) to
/// their current names; any other name passes through.
pub fn current_name(name: &str) -> &str {
    match name {
        "Terrain" => "Demiurge Terrain",
        "Ocean" => "Demiurge Ocean",
        "GhostTown" => "Demiurge GhostTown",
        "Medieval Architect" => "Demiurge Medieval Architect",
        "Battlefield / Ruin" => "Demiurge Battlefield & Ruin",
        n => n,
    }
}

/// Whether a modifier fits the object it would be added to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    Yes,
    /// Allowed but probably not what you want (drawn dimmed).
    Dim(&'static str),
    /// Cannot be added (drawn dimmed and disabled).
    No(&'static str),
}

/// Modifiers that build a mesh from a spline flowing up the stack.
const SHAPE_ONLY: [&str; 3] = ["Lathe", "Extrude", "Loft"];
/// Modifiers that accept either a spline or a mesh.
const SHAPE_OR_MESH: [&str; 2] = ["Demiurge GhostTown", "Demiurge Medieval Architect"];

pub fn fit(name: &str, o: Option<&SceneObject>) -> Fit {
    let name = current_name(name);
    let Some(o) = o else { return Fit::No("Select an object first") };
    if o.light().is_some() || o.camera().is_some() || o.is_helper() {
        return Fit::No("Lights, cameras and helpers take no modifiers");
    }
    let spline = o.shape.is_some();
    if SHAPE_ONLY.contains(&name) {
        return if spline { Fit::Yes } else { Fit::No("Needs a spline (Create → Shapes)") };
    }
    if spline && !SHAPE_OR_MESH.contains(&name) {
        return Fit::Dim("Works on the spline's renderable mesh only: add Lathe, Extrude or Loft first");
    }
    Fit::Yes
}

pub fn info(name: &str) -> Option<&'static ModifierInfo> {
    GROUPS.iter().flat_map(|(_, list)| list.iter()).find(|i| i.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modifiers::{MODIFIER_LIST, make_modifier};

    #[test]
    fn every_modifier_is_in_exactly_one_group_and_builds() {
        let all: Vec<_> = GROUPS.iter().flat_map(|(_, l)| l.iter().map(|i| i.name)).collect();
        for item in MODIFIER_LIST {
            assert_eq!(all.iter().filter(|n| **n == item.name).count(), 1, "{} must be in exactly one group", item.name);
        }
        assert_eq!(all.len(), MODIFIER_LIST.len(), "no group entry outside the Modifier List");
        for name in all {
            assert!(make_modifier(name).is_some(), "{name} does not build a modifier");
            assert!(!info(name).unwrap().tip.is_empty());
        }
    }

    #[test]
    fn fit_follows_the_object_type() {
        use crate::editor::Editor;
        use crate::primitives::PrimitiveKind;
        use crate::render::LightKind;
        use crate::splines::ShapeKind;
        let mut ed = Editor::default();
        ed.create_primitive(PrimitiveKind::Box, bevy::math::Vec3::ZERO);
        ed.create_shape(ShapeKind::Circle, 0, bevy::math::Vec3::X * 5.0);
        ed.create_light_default(LightKind::Omni);
        let kind = |o: &SceneObject| (o.light().is_some(), o.shape.is_some());
        for o in ed.scene.objects.values() {
            match kind(o) {
                (true, _) => assert!(matches!(fit("Bend", Some(o)), Fit::No(_))),
                (false, true) => {
                    assert_eq!(fit("Lathe", Some(o)), Fit::Yes);
                    assert_eq!(fit("Demiurge GhostTown", Some(o)), Fit::Yes);
                    assert!(matches!(fit("Bend", Some(o)), Fit::Dim(_)));
                }
                (false, false) => {
                    assert_eq!(fit("Bend", Some(o)), Fit::Yes);
                    assert!(matches!(fit("Lathe", Some(o)), Fit::No(_)));
                }
            }
        }
        assert!(matches!(fit("Bend", None), Fit::No(_)));
    }
}
