//! Help page content, part 1: page types and the first topics.
//! Marker positions are pixels in the 1600×920 screenshots.

use crate::editor::CommandTab;
use crate::ui::actions::Action;

/// A numbered circle on a screenshot, explained under it.
#[derive(Clone, Copy, Debug)]
pub struct Mark {
    /// Position as a fraction of the image size.
    pub at: [f32; 2],
    pub label: &'static str,
}

pub const fn m(x: f32, y: f32, label: &'static str) -> Mark {
    Mark { at: [x / 1600.0, y / 920.0], label }
}

/// What a "Show me" button opens.
#[derive(Clone, Copy, Debug)]
pub enum Show {
    Act(Action),
    Tab(CommandTab),
}

#[derive(Clone, Copy, Debug)]
pub enum Block {
    H(&'static str),
    P(&'static str),
    Steps(&'static [&'static str]),
    Tip(&'static str),
    Shot(&'static str, &'static [Mark]),
    ShowMe(&'static str, Show),
    /// The live hotkey cheat sheet.
    Keys,
}

#[derive(Clone, Copy, Debug)]
pub struct Page {
    pub id: &'static str,
    pub title: &'static str,
    pub blocks: &'static [Block],
}

use Block::*;

pub const BASICS: Page = Page {
    id: "basics",
    title: "1. Getting Started",
    blocks: &[
        P("Ilex works like 3ds Max. The screen has five fixed zones, and every tool has a button: hotkeys are only shortcuts."),
        Shot(
            "layout",
            &[
                m(40.0, 13.0, "Menu bar: File, Edit, Tools, Views, Create, Modifiers, Rendering, Help"),
                m(555.0, 42.0, "Main Toolbar: select, move, rotate, scale, snaps, Mirror, Align, Array"),
                m(100.0, 170.0, "Scene Explorer: every object, with hide and freeze toggles (H hides the panel)"),
                m(825.0, 511.0, "Viewport label menus: view type and shading"),
                m(1420.0, 116.0, "Command Panel: Create, Modify, Hierarchy, Display, Utilities"),
                m(1215.0, 858.0, "Navigation buttons: zoom, pan, orbit, zoom extents, maximize"),
                m(330.0, 906.0, "Prompt line: tells you the next step of the current tool"),
            ],
        ),
        H("Moving around the viewports"),
        Steps(&[
            "Middle mouse drag: pan.",
            "Alt + middle mouse drag: orbit around the selection.",
            "Mouse wheel (or Ctrl + Alt + middle drag): zoom.",
            "Z: zoom to the selection. Shift+Z: zoom to everything.",
            "Alt+W: maximize the active viewport, or go back to four views.",
            "F3: wireframe or shaded. F4: edged faces. G: grid.",
        ]),
        Tip("No middle mouse button? Click Pan, Orbit or Zoom at the bottom right, then left-drag in a viewport. Right-click stops."),
        ShowMe("Show me the Create tab", Show::Tab(CommandTab::Create)),
    ],
};

pub const CREATE: Page = Page {
    id: "create",
    title: "2. Creating objects",
    blocks: &[
        P("Pick a type on the Create tab (or in the Create menu), then draw it in a viewport, just like 3ds Max."),
        Shot(
            "create",
            &[
                m(1352.0, 117.0, "The Create tab"),
                m(1370.0, 195.0, "Creation Method: Box or Cube, Center or Edge"),
                m(1369.0, 287.0, "Object types: click one, then draw in a viewport"),
                m(1015.0, 700.0, "Drag here to draw the base"),
            ],
        ),
        Steps(&[
            "Press and drag to draw the base (a rectangle for boxes and planes, a radius for round objects).",
            "Release, move the mouse up or down for the height, then click.",
            "Cones ask for a top radius and tori for a tube thickness: move and click once more.",
            "Right-click cancels a half-made object. Ctrl+Z removes a finished one.",
            "The tool stays on, so you can draw several in a row. Press Esc or click the button again to stop.",
        ]),
        Tip("A plain click places a default-size object. Right-click a Create button to place one at the origin."),
        H("What you can create"),
        P("Standard Primitives (Box, Sphere, Cylinder, Plane, Torus, Cone), GeoGen rocks, crystals, cliffs and stalagmites, Worlds (Terrain, Ocean, Planet), GhostTown settlements, Mech parts (Gear, Piston, Hose), Shapes / Splines, Lights and Cameras."),
        P("Objects drawn in Top or Perspective sit on the ground grid. In Front or Left they stand on that view's plane. Snaps (S) work while drawing."),
    ],
};

pub const TRANSFORM: Page = Page {
    id: "transform",
    title: "3. Selecting and transforming",
    blocks: &[
        Shot(
            "transform",
            &[
                m(190.0, 42.0, "Q Select, and the selection region shape"),
                m(555.0, 42.0, "W Move"),
                m(600.0, 42.0, "E Rotate"),
                m(643.0, 42.0, "R Scale"),
                m(700.0, 42.0, "Reference coordinate system"),
                m(905.0, 42.0, "S Snaps (right-click the toolbar's settings button for snap types)"),
                m(1015.0, 690.0, "Gizmo: drag an axis or a plane handle"),
                m(1345.0, 906.0, "Type-in: exact X / Y / Z (right-click a spinner to zero it)"),
            ],
        ),
        Steps(&[
            "Click to select, Ctrl+click to add, Alt+click to remove. Drag for a region.",
            "Q cycles the region shape: rectangle, circle, fence, lasso.",
            "Shift + drag a gizmo clones the object (Copy, Instance or Reference).",
            "A turns on angle snap (5° steps).",
        ]),
        ShowMe("Open Array", Show::Act(Action::Array)),
        ShowMe("Open Mirror", Show::Act(Action::Mirror)),
        ShowMe("Pick an object to Align to", Show::Act(Action::Align)),
    ],
};

pub const EDIT_POLY: Page = Page {
    id: "editpoly",
    title: "4. Editable Poly modelling",
    blocks: &[
        P("Right-click an object and choose Convert to Editable Poly (or add an Edit Poly modifier). Then pick a sub-object level."),
        Shot(
            "editpoly",
            &[
                m(1456.0, 226.0, "The stack: Editable Poly"),
                m(1482.0, 474.0, "Sub-object levels 1–5: Vertex, Edge, Border, Polygon, Element"),
                m(1490.0, 702.0, "Tool buttons: click to apply"),
                m(1556.0, 702.0, "☐ opens the caddy: live settings with OK, Apply and Cancel"),
                m(1320.0, 680.0, "The selected polygon is red"),
            ],
        ),
        Steps(&[
            "1 Vertex, 2 Edge, 3 Border, 4 Polygon, 5 Element. Press the same number again to go back to the object.",
            "Edges: double-click (or Alt+L) for a loop, Alt+R for a ring.",
            "Shift + drag edges in Edge or Border mode extrudes new polygons.",
            "Repeat Last runs the previous tool again with one click.",
        ]),
        H("The Modeling Ribbon"),
        Shot("ribbon", &[m(75.0, 75.0, "Ribbon on / off"), m(700.0, 112.0, "Big buttons for every tool at this level")]),
        Tip("On the ribbon, clicking a caddy tool uses its last settings; right-click opens its caddy."),
        ShowMe("Show the Modeling Ribbon", Show::Act(Action::Ribbon)),
    ],
};

pub const MODIFIERS: Page = Page {
    id: "modifiers",
    title: "5. Modifiers",
    blocks: &[
        P("Modifiers stack on top of an object without changing it. The bottom is applied first."),
        Shot(
            "modifiers",
            &[
                m(1398.0, 188.0, "Modifier List: scroll through every modifier, grouped"),
                m(1533.0, 188.0, "Buttons: a grid of your favourite modifiers"),
                m(1452.0, 230.0, "The stack: click a row to edit it; the eye turns it off"),
                m(1450.0, 375.0, "Pin, Show End Result, Make Unique, Delete, Collapse"),
            ],
        ),
        Steps(&[
            "Select an object and open the Modify tab.",
            "Open the Modifier List and click a modifier. It goes on top of the highlighted row.",
            "Modifiers that don't fit the selection are dimmed and say why.",
            "Right-click the Buttons header to choose which modifiers it shows.",
            "Expand a row (⏵) for its sub-objects: Gizmo, Center, Control Points, Mirror.",
        ]),
        P("Included: Edit Poly, FFD 2×2×2 / 3×3×3 / 4×4×4, UVW Map, Unwrap UVW, TurboSmooth, Symmetry, Shell, Chamfer, Bend, Twist, Taper, Push, Lathe, Extrude, Greeble, GeoGen, Boolean, ProOptimizer, Edit Normals, VertexPaint, Panel Groove, Terrain, Ocean and Battlefield & Ruin."),
        ShowMe("Show the Modify tab", Show::Tab(CommandTab::Modify)),
    ],
};

pub const SPLINES: Page = Page {
    id: "splines",
    title: "6. Splines, Lathe and Extrude",
    blocks: &[
        Shot(
            "splines",
            &[m(1454.0, 224.0, "Extrude on top of the Star shape"), m(1340.0, 470.0, "Extrude amount, segments and caps")],
        ),
        Steps(&[
            "Create > Shapes: Line, Circle, Arc, NGon, Rectangle, Ellipse, Star, Helix.",
            "Line: click for corners, drag for Bezier curves, right-click to finish. Click the first point to close it.",
            "Add Lathe to spin a profile into a vase or a pillar, or Extrude to give a closed shape depth.",
            "Rendering rollout: turn any spline into a pipe or bar with Enable in Viewport.",
            "Editable Spline sub-objects: 1 Vertex (Fillet, Chamfer, Weld, Refine), 2 Segment (Divide), 3 Spline (Outline, Boolean, Mirror).",
        ]),
    ],
};

pub const MATERIALS: Page = Page {
    id: "materials",
    title: "7. Materials",
    blocks: &[
        Shot(
            "materials",
            &[
                m(553.0, 75.0, "Material Editor (M)"),
                m(500.0, 230.0, "24 sample slots: click one to edit it"),
                m(348.0, 376.0, "Assign to Selection, Show in Viewport, Reset, Copy, Pick"),
                m(528.0, 520.0, "Quick Preset: one-click materials"),
            ],
        ),
        Steps(&[
            "Press M or click Material.",
            "Pick a slot, then a Quick Preset or a Shading model (PBR, Toon, Glass, Unlit).",
            "Select objects and click Assign.",
            "Every material has an Ink / Toon Outline rollout.",
            "Principled BSDF rollout: Subsurface Scattering (skin, wax, jade), Anisotropy (brushed metal), Clearcoat (car paint), Cloth Sheen, Absorption and Dispersion for glass. Try the one-click looks.",
            "Maps rollout: image or procedural maps for colour, metal, roughness, bump, opacity and glow.",
            "Procedural maps include Bricks / Tiles, Stripes (road lines), Cracks and Grunge (dirt, rust, scratches).",
            "Material Library (📚 Library): over 300 ready-made materials in 16 categories. Click a swatch to load it into the active slot; ☆ marks a favourite.",
            "Library filter: Procedural, Photo or Both. Photo materials appear after you download the Photo Textures pack (Preferences > Assets).",
            "Mapping rollout: World / Triplanar projects the material from three sides, so it tiles on objects without UVs (GhostTown, Terrain, rocks).",
            "Preserve Texture Appearance (under Emission Multiplier): the material still lights the scene, but the camera sees its own colours at Direct Camera Brightness.",
        ]),
        H("Procedural Pattern & Weathering"),
        Shot(
            "procedural",
            &[
                m(336.0, 140.0, "The sample slot shows the result"),
                m(530.0, 636.0, "Pattern and Style: bricks, cobbles, rock, wood, weave, metal, asphalt, terrain"),
                m(480.0, 755.0, "Tiling, Seed, Gap, Bevel, Color Variation, Surface Relief (scroll down for Live Weathering)"),
                m(1017.0, 684.0, "Colour, normals, roughness and AO follow the pattern in the viewport"),
            ],
        ),
        Steps(&[
            "In the Material Editor (Compact mode), open the Procedural Pattern & Weathering rollout.",
            "Pick a Pattern, then a Style. One click sets its colours, gap, roughness and metalness.",
            "Tune Tiling / Scale, Seed (🎲 for a new one), Gap / Mortar Width, Gap / Bevel Depth, Color Variation and Surface Relief.",
            "Drag the Live Weathering sliders: Edge Wear, Cavity Dirt, Moss, Scorch / Rust and Wetness / Puddles.",
            "Leave Auto-Derive Normal, Roughness & Cavity AO ticked: the normal map, roughness variation and ambient occlusion follow the pattern in the viewport and in F9 renders.",
            "Every pattern tiles seamlessly, so it repeats across any surface with no seams.",
        ]),
        Tip("Maps cook on background threads. While you drag a spinner the old look stays on screen and updates a moment later."),
        Tip("In the Maps rollout (or the Slate view) any channel can also take a Tileable Pattern, Levels (with a histogram), Height Blend, Slope Blur or Normal From Height map."),
        Shot(
            "library",
            &[
                m(957.0, 107.0, "Material Library (📚 Library)"),
                m(787.0, 141.0, "Search by name or category"),
                m(760.0, 420.0, "Categories and ★ Favourites"),
                m(1107.0, 400.0, "Click a swatch to load it into the active slot"),
                m(640.0, 379.0, "Library button in the Material Editor"),
            ],
        ),
        ShowMe("Open the Material Editor", Show::Act(Action::MaterialEditor)),
        ShowMe("Open the Material Library", Show::Act(Action::MaterialLibrary)),
    ],
};

pub const PAINT: Page = Page {
    id: "paint",
    title: "8. Painting",
    blocks: &[
        Shot(
            "paint",
            &[
                m(1543.0, 41.0, "Viewport Paint on / off"),
                m(1013.0, 700.0, "Paint straight on the model"),
                m(400.0, 713.0, "Layers: your material stays at the bottom, untouched"),
                m(440.0, 475.0, "Damage stamps"),
            ],
        ),
        Steps(&[
            "Select an object and click Paint.",
            "PaintBox: brush size, opacity, falloff, colour, Fill Object and Blur.",
            "Vertex mode needs no UVs. Texture mode paints in layers on top of the object's material.",
            "Right-click in the viewport to stop painting.",
        ]),
        H("Texture Paint layers"),
        Steps(&[
            "Your material stays as Layer 0: Base Material. Painting never changes it, and your UVs are kept. Only an object without UVs is unwrapped for you.",
            "Paint on a Paint Layer. The Erase tool removes paint, so the base material shows again.",
            "Add Paint Layers and Fill Layers. Each has a 👁 toggle, a blend mode (Normal, Multiply, Add / Glow, Screen, Height-Blend) and an opacity.",
            "Paint with: a colour, or any Material Library preset. One stroke paints colour, relief, roughness, metallic and glow together.",
            "Damage Stamps: Bullet Hole, Blast Star, Blood Splatter, Burn Patch, Ember Cracks, Scratch, Rivet. Click to place one, drag to scatter.",
            "🔥 Bake Mesh Maps works out Curvature, Cavity AO, World Normal and Z-Height. The ✨ Smart buttons use them: Edge Wear, Metal Chipping, Cavity Dirt, Rust, Dust, Snow, Moss, Soot, Mud Splash.",
            "💾 Export Textures writes base colour, ORM, normal and emission PNGs.",
            "Ctrl+Z undoes one stroke. Only the 64 x 64 tiles a stroke touched are copied, so undo stays light.",
        ]),
        ShowMe("Start painting", Show::Act(Action::Paint)),
    ],
};
