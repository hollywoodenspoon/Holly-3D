//! Help page content, part 2, and the full page list.

use super::pages::*;
use super::pages3::{CHARACTERS, NAMES};
use crate::editor::CommandTab;
use crate::ui::actions::Action;
use Block::*;

pub const SKY: Page = Page {
    id: "sky",
    title: "9. Lights, cameras, sky and deep space",
    blocks: &[
        Shot(
            "sky",
            &[
                m(260.0, 75.0, "Environment and Effects (8)"),
                m(405.0, 75.0, "Sky menu: day, sunset, overcast, night, deep space, nebula in one click"),
                m(175.0, 75.0, "Default Lights (Ctrl+L)"),
            ],
        ),
        H("Making a sky or deep space"),
        Steps(&[
            "Click the Sky menu on the toolbar and pick a preset; that is all it takes.",
            "For more control press 8: time of day, sun, clouds, fog, stars, nebula and HDRI images.",
            "Deep Space turns the sky black with stars and nebulae; add a Planet from Create > Demiurge Worlds.",
            "Weather & Storm Clouds (in the 8 dialog): pick a Weather Preset, from Clear Sky to Thunderstorm, Ash Haze, Heavy Fog, Rainstorm or Blizzard.",
            "Cloud Thickness darkens storm cloud bases. Overcast Sun Dimming dims the linked sun and softens its shadows. Lightning Flash adds a bolt.",
            "Precipitation (Rain, Snow, or Ash with glowing cinders) falls in the viewport and in F9, hidden behind nearer objects.",
        ]),
        H("Lights and cameras"),
        Steps(&[
            "Create > Lights: Omni, Target / Free Spot, Target / Free Directional, Skylight, and Rect / Disc / Sphere / Tube area lights.",
            "Area lights cast soft shadows that grow with their size; tick Visible to Camera to see the softbox or neon bar in F9.",
            "Spot and area lights take an IES Light Profile (wall scallop, streetlamp, flashlight, high-bay, stage beam).",
            "Target lights and cameras take two clicks: the light, then its target.",
            "With no lights, a headlight rig lights the scene. Ctrl+L forces it on.",
            "C looks through the selected camera.",
        ]),
        H("Godrays and mist"),
        Steps(&[
            "Press 8 and open Luxon Volumetric Godrays & Mist. Tick Enable, or click Cathedral Shafts, Valley Mist or Dusty Hangar.",
            "Mist Base Height and Height Falloff keep mist low in streets and valleys; Wisps add patchy 3D noise that drifts with Wind.",
        ]),
        ShowMe("Open Environment and Effects", Show::Act(Action::Environment)),
    ],
};

pub const RENDER: Page = Page {
    id: "render",
    title: "10. Rendering",
    blocks: &[
        Shot("render", &[m(627.0, 75.0, "Render Setup (F10)"), m(680.0, 75.0, "Veach Rendered Frame Window"), m(735.0, 75.0, "Render (F9)")]),
        Steps(&[
            "F10 Render Setup: resolution presets, alpha background, camera, anti-aliasing, AO, GI, denoiser, bloom, depth of field.",
            "Pick the renderer: the Veach Path Tracer (default: multi-bounce light, MIS, Russian Roulette) or the fast Classic Raytracer.",
            "Compute Device: Auto uses the GPU (wgpu Vulkan compute) and falls back to the CPU; HDRI skies, opacity, toon and texture maps render on the CPU.",
            "Progressive Preview streams full-frame passes (1, 2, 4 ... samples). Veach MIS & Adaptive Sampling stops pixels whose noise is below the Noise Threshold.",
            "F9 renders the active view into the Veach Rendered Frame Window.",
            "In the Frame Window: save PNG / EXR / JPG, copy, clone the window to compare, and view R / G / B / A channels.",
        ]),
        H("Color Grading & Camera RAW"),
        Shot(
            "grading",
            &[
                m(950.0, 153.0, "🎨 Grading: show or hide the sidebar"),
                m(975.0, 273.0, "Color Grading sidebar: every slider re-grades at once, no re-render"),
            ],
        ),
        Steps(&[
            "Click 🎨 Grading on the Frame Window toolbar (or Rendering > Color Grading & Camera RAW).",
            "Tone & Presence: Highlights, Shadows, Whites, Blacks, Clarity, Vibrance and Saturation.",
            "White Balance: Temperature in Kelvin (higher is warmer) and Tint (green to magenta). Drag in the Shadows / Midtones / Highlights wheels to tint each range.",
            "Lens & Film FX: Vignette with Midpoint, Chromatic Aberration and Film Grain (🎲 New Grain changes the pattern).",
            "Right-click any slider or wheel to reset it. Saved PNG / JPG images include the grade; EXR keeps the ungraded linear light.",
        ]),
        H("Luxon LightMix, Veach denoiser and render passes"),
        Steps(&[
            "Render with the Veach Path Tracer, then click 💡 Luxon LightMix: every group (Sun & Sky, Environment, Emissive, Godrays, each light) has On, Solo, Intensity and Tint. The image re-mixes at once, no re-render.",
            "💡 Apply Luxon LightMix to Scene Lights bakes the faders into the lights (Ctrl + Z undoes it).",
            "The Veach À-Trous Wavelet Denoiser divides out the surface albedo, smooths only the lighting with a 5-pass A-Trous filter (normal, depth, roughness and object aware), then puts the crisp texture back.",
            "The pass list next to R / G / B / A shows Direct, Indirect GI, Volumetric, Specular, Subsurface, Albedo, World Normal, Linear Depth, Cavity / AO and the adaptive sample heatmap.",
        ]),
        H("Lens effects, ink and catchers"),
        Steps(&[
            "F10 > Luxon Lens FX (Bokeh, Anamorphic & Glare): 5 to 9 aperture blades (with rotation) shape the depth-of-field bokeh; Cat's-Eye squeezes it near the frame edges.",
            "Anamorphic Streak Flare and a 4 / 6 / 8 point Starburst add glare to highlights above the Flare Threshold.",
            "Material Editor > Ink / Toon Outline > Interior Pen Lines draws soft folds and Panel Groove seams. Toon materials have Manga Halftone Shadows.",
            "Material Editor > Alpha: Blend shows texture alpha as see-through; Cut-out makes it solid or a hole (hair cards, leaves, fences), with no sorting problems and no haze. Holes let light and AO through.",
            "Display tab > Shadow & Reflection Catcher: the ground renders transparent except for shadows, AO and reflections, ready for a photo backplate.",
            "🎞 Turntable / Sprites (F9 toolbar or F10): a 360° PNG sequence, or an N / NE / E ... sprite atlas PNG + .json at a 30°, 35.264° or 45° pitch.",
        ]),
        ShowMe("Open Luxon LightMix", Show::Act(Action::LightMix)),
        ShowMe("Render Turntable / Sprite Sheet", Show::Act(Action::Turntable)),
        ShowMe("Open Color Grading", Show::Act(Action::ColorGrading)),
        ShowMe("Open Render Setup", Show::Act(Action::RenderSetup)),
    ],
};

pub const GHOSTTOWN: Page = Page {
    id: "ghosttown",
    title: "11. Demiurge: Settlement, Medieval Architect and destruction",
    blocks: &[
        Shot(
            "ghosttown",
            &[
                m(1356.0, 464.0, "Settlement preset: 14 towns, from downtown to star fort"),
                m(1486.0, 518.0, "New Seed: a different town"),
                m(1396.0, 864.0, "Assign the Settlement Material Pack"),
            ],
        ),
        Steps(&[
            "Create > Demiurge Settlements, then drag out the town area in a viewport.",
            "Change the seed, the block and street sizes, floors and density on the Modify tab.",
            "New presets: River City, Harbour Town, Industrial, Old Town + Modern Sprawl, Square Walled City, River Town, Castle & Town, Star Fort, Crossroads Hamlet and Monastery.",
            "Buildings: uncheck any type to leave it out. Building Parts, Town Features (river, bridges, walls, moat, rail...), Emissive Lights and Prop Style (Generated / Low-poly / Realistic) shape the town.",
            "Damaged / Battlefield Mode, or the Demiurge Battlefield & Ruin modifier, wrecks the buildings. Blast Zones aim the damage.",
            "Prefab Library (Tools menu, the Demiurge tab or the 🏠 Prefabs button): thumbnails of every house, building, bridge and street prop, with categories (Residential has cottages to towers, each with Lot checkboxes: Garden, Driveway, Garage, Fence; Commercial & Office has shops, cafés, a supermarket, mall, hotel, motel, bank, cinema, car dealership, drive-through and offices; Industrial & Utilities has factories, depots, silos, a refinery, tank farm, recycling yard, power plant, wind turbine, solar farm, pump station, substation, pylon and cell tower), search and Favourites (click the star on a tile). Click a tile, then click in a viewport: the prefab lands on the surface under the cursor (a roof, terrain, a table) standing upright. The mouse wheel turns it, [ and ] scale it, Shift-click stamps more copies and right-click ends. The bar on top has the same Rotation and Scale with buttons, Vary Seed (each copy looks different), Drop on Surfaces, and Drag to Size (drag out the footprint instead).",
            "Every prefab has Detail: Standard (light) or Mid (window frames, sills, mullions and trims), and the Destruction rollout from Intact to Rubble.",
            "Destruction Level (on a building, the town and the Battlefield modifier): Intact, Light Damage, Heavy Damage, Warzone Collapse, Apocalyptic Ruin or Pure Rubble. Each building in a town varies by one step.",
            "Rubble Density, Era (Modern concrete and rebar, Medieval stone and charred timber, Ancient stone and moss), Glowing Embers with Ember Glow (ID 25) and Gore / Blood Smears (ID 28) dress the rubble mounds.",
            "The Rubble Pile type places a mound on its own: Width and Length set the spread, Pile Height its height.",
            "Smoke Plume (Create > Demiurge Worlds): drag the source radius, then the height. Style (black battlefield, grey chimney, white steam), Wind Lean, Turbulence, Rise Phase and Fire Base Intensity shape it.",
            "Spawn Rising Smoke Plumes (damaged town or Battlefield modifier) raises plumes from the tallest wrecks and the blast zones.",
        ]),
        ShowMe("Show the Create tab", Show::Tab(CommandTab::Create)),
        ShowMe("Open the Prefab Library", Show::Act(Action::BuildingsBrowser)),
    ],
};

pub const MECH: Page = Page {
    id: "mech",
    title: "12. Mech assembly",
    blocks: &[
        Shot("mech", &[m(1467.0, 116.0, "Hierarchy tab: joints, contact sets and motion links"), m(1376.0, 440.0, "Piston parameters")]),
        Steps(&[
            "Create > Mech: Gear, Hydraulic Piston, Hose.",
            "J (Assemble Joint): click a mount point on the child part, then one on the parent.",
            "Drag the end of a chain to pose it (IK); rotate joints for FK.",
            "Motion Links make gears turn together; a hose follows the two parts it is bound to.",
        ]),
        ShowMe("Start the Joint tool", Show::Act(Action::JointTool)),
    ],
};

pub const CAD: Page = Page {
    id: "cad",
    title: "13. CAD tools: fasteners, holes, Loft, constraints",
    blocks: &[
        Shot("cad", &[m(1433.0, 600.0, "Fastener parameters: standard, size, lengths"), m(1020.0, 600.0, "Real helical threads")]),
        H("Bolts, nuts and threaded holes"),
        P(
            "Create > Geometry > Mech has Bolt, Nut and Threaded Hole. Pick ISO Metric (M3 to M24) or Imperial UNC (#10 to 1\"), and the head, nut and thread sizes follow the standard tables. A nut fits the bolt of the same size. Untick Model Threads for light plain cylinders.",
        ),
        H("Hole, Counterbore and Countersink"),
        Steps(&[
            "Edit Poly or Editable Poly, Polygon level (4): select the faces to drill.",
            "Edit Polygons > the settings button next to Hole: choose Simple Hole, Counterbore or Countersink.",
            "Set Diameter and Depth, or tick Through All; counterbores and countersinks also take a Top Diameter.",
            "OK drills one hole in the centre of every selected polygon, straight in along its normal.",
        ]),
        Tip("The cut is a Boolean, so the mesh must be closed. If it is not, nothing changes and the status bar says why."),
        H("Multi-profile Loft"),
        Steps(&[
            "Select the path spline and add the Loft modifier (Modifier List > Spline > Mesh).",
            "Set Next Section At % and click Pick Shape, then click a shape in a viewport. Repeat for more sections.",
            "Sections blend smoothly between their path percentages. Twist, Scale Start / End and Scale Curve shape the skin; Cap Start / End close it.",
        ]),
        H("Spline constraints"),
        P(
            "Select an Editable Spline and pick a level. Segment level (2) has Parallel, Perpendicular and Equal Length. Vertex level (1) has Tangent, which makes the curve leave a vertex in the direction it arrived. Spline level (3) has Concentric. Each one runs once: the first selected item in spline order is the reference, and the other selected items move to match it. The constraints are also in Tools > Spline Constraints.",
        ),
        ShowMe("Open the Create tab", Show::Tab(CommandTab::Create)),
    ],
};

pub const FILES: Page = Page {
    id: "files",
    title: "14. Files",
    blocks: &[Steps(&[
        "File > New, Open and Save use Ilex scenes (.ilex). Open also reads scenes saved before the rename (.mmax); saving one writes a .ilex beside it.",
        "Import OBJ, glTF / GLB and FBX. Export everything or just the selection.",
        "Autosave keeps a backup every few minutes (Preferences > General).",
        "You can also open a file from the terminal: ilex scene.ilex (the old minimax command still works)",
    ])],
};

pub const PREFS: Page = Page {
    id: "prefs",
    title: "15. Preferences and hotkeys",
    blocks: &[
        Shot("prefs", &[m(836.0, 75.0, "Preferences (⚙)"), m(418.0, 68.0, "Tabs: Theme, Hotkeys, Viewports, General, Rendering, Assets")]),
        Steps(&[
            "Theme: pick a preset or make your own colours. Text is checked for readability.",
            "Hotkeys: click a key and press the new one. Choose the Classic key set if you prefer it.",
            "Changes save as soon as you let go of the mouse.",
        ]),
        ShowMe("Open Preferences", Show::Act(Action::Preferences)),
    ],
};

pub const CRASH: Page = Page {
    id: "crash",
    title: "16. Crash reports and recovery",
    blocks: &[
        P(
            "If Ilex ever crashes, it writes a report of what happened and saves your scene first. Next time it starts, this dialog appears.",
        ),
        Shot(
            "crash",
            &[
                m(657.0, 473.0, "Copy the report, then paste it into Claude Code"),
                m(713.0, 503.0, "Open the scene as it was (unsaved)"),
                m(651.0, 533.0, "Open the crashes folder"),
                m(616.0, 563.0, "Close; the reports are kept"),
            ],
        ),
        Steps(&[
            "Copy Crash Report for Claude copies the whole Markdown report. Paste it into Claude Code and ask it to fix the bug.",
            "The report says where it crashed, your last 50 actions (tools, hotkeys, modifiers, dialogs), the scene and selection, and the Ilex functions that were running.",
            "Restore Unsaved Scene opens crash_recovery.ilex. It opens unsaved, so use File > Save As to keep it.",
            "Reports live in ~/.local/share/ilex/crashes.",
        ]),
        Tip(
            "Help > View Crash Reports... lists every report. Simulate Test Crash Report writes a sample one without crashing, so you can check it works.",
        ),
        ShowMe("View Crash Reports", Show::Act(Action::CrashReports)),
    ],
};

pub const HOTKEYS: Page =
    Page { id: "hotkeys", title: "Hotkey cheat sheet", blocks: &[P("Your current keys (change them in Preferences > Hotkeys)."), Keys] };

pub const CREDITS: Page = Page {
    id: "credits",
    title: "Credits",
    blocks: &[
        P("Ilex is free software under the GNU General Public License, version 3 or later (see the LICENSE file)."),
        P("Ilex is built with Bevy, egui, transform-gizmo, slotmap, boolmesh, meshopt, uvunwrap, noise and image."),
        P("Free CC0 assets: Poly Haven (photo textures and realistic models), Kenney and Quaternius (low-poly models). Thank you!"),
        H("MaterialCraft"),
        P(
            "The procedural texture engine (tileable lattice noise, brick, Voronoi crack, wood, weave and scratch generators, Levels, Height Blend, Slope Blur, Normal From Height, cavity AO, weathering masks and the content-addressed cache) is ported from MaterialCraft by the ArtCraft Team and the MaterialCraft contributors: github.com/storytold/materialcraft. Licensed MIT / Apache-2.0.",
        ),
        H("TextureCraft"),
        P(
            "The texture painter follows the design of TextureCraft by the ArtCraft Team and the TextureCraft contributors: github.com/storytold/texturecraft. That covers the 64 x 64 copy-on-write texel tiles, the layer stack with blend modes and masks, mesh map baking, Smart Masks and the brush stamps. Licensed MIT / Apache-2.0.",
        ),
        H("LightCraft"),
        P(
            "The F9 Color Grading & Camera RAW sidebar follows the develop pipeline of LightCraft by the ArtCraft Team and the LightCraft contributors: github.com/storytold/lightcraft. That covers the tone and presence controls, Kelvin white balance, split-toning wheels, vignette, chromatic aberration and film grain. Licensed MIT / Apache-2.0.",
        ),
        H("SolidCraft"),
        P(
            "The CAD tools follow SolidCraft by the SolidCraft contributors: github.com/tangency-labs/solidcraft. That covers the ISO metric / UNC bolts, nuts and threaded holes, the Hole / Counterbore / Countersink cutter, the multi-profile Loft and the Parallel, Perpendicular, Tangent, Concentric and Equal Length sketch constraints. Licensed MIT / Apache-2.0; no SolidCraft brand or interface material is used.",
        ),
        P(
            "The path tracer (CPU and the wgpu GPU kernel) follows the design of SolidCraft's GPU path tracer: its GGX / visible-normal sampling, lobe mix, power-heuristic MIS and storage-buffer BVH layout, plus the Principled BSDF lobes (random-walk SSS, anisotropic GGX, clearcoat, sheen, dispersion). MIT / Apache-2.0.",
        ),
        H("MakeHuman"),
        P(
            "The MakeHuman window uses the MakeHuman hm08 base mesh, its morph targets, modifiers, eyes, skeleton and system assets (hair, clothes, eyebrows, eyelashes, teeth, tongue, body proxies, skins and eye colours) by the MakeHuman Team (makehumancommunity.org). These assets are CC0 1.0 (public domain); see assets/makehuman/LICENSE.ASSETS.md.",
        ),
        H("Strolle"),
        P(
            "The albedo-demodulated A-Trous denoiser follows the SVGF / A-Trous wavelet filter in Strolle by Patryk Wychowaniec and the Strolle contributors: github.com/Patryk27/strolle. Licensed MIT.",
        ),
        H("EffectCraft and MediaPipe"),
        P(
            "The lens effects (polygonal aperture bokeh, cat's-eye vignetting, anamorphic streaks and starburst glare) follow EffectCraft by the ArtCraft Team and the EffectCraft contributors, and the Face Cloner runs face landmark detection through EffectCraft's segment crate. Licensed MIT / Apache-2.0.",
        ),
        P(
            "The Face Cloner's landmark model (assets/face/face_landmarker.task, 478 points with depth) is the MediaPipe Face Landmarker by Google: github.com/google-ai-edge/mediapipe. Licensed Apache-2.0.",
        ),
        P(
            "The Face Cloner's optional hair and background masks use MobileSAM by Chaoning Zhang et al. (github.com/ChaoningZhang/MobileSAM), run by EffectCraft's pure-Rust port. Licensed Apache-2.0. The weights are not shipped: you download them from the Face Cloner window.",
        ),
        H("Asset packs"),
        P(
            "The optional downloads in Preferences > Assets are CC0 (public domain): photo textures and models from Poly Haven (polyhaven.com), low-poly models from Kenney (kenney.nl).",
        ),
        H("Font"),
        P(
            "The interface font is Fira Code by The Fira Code Project Authors: github.com/tonsky/FiraCode. It is licensed under the SIL Open Font License 1.1.",
        ),
        P("The licence texts ship with Ilex in the licenses folder."),
    ],
};

/// Every page, in topic order.
pub const ALL: [Page; 20] = [
    BASICS, CREATE, TRANSFORM, EDIT_POLY, MODIFIERS, SPLINES, MATERIALS, PAINT, SKY, RENDER, GHOSTTOWN, MECH, CAD, FILES, PREFS, CRASH,
    CHARACTERS, NAMES, HOTKEYS, CREDITS,
];

pub fn index_of(id: &str) -> usize {
    ALL.iter().position(|p| p.id == id).unwrap_or(0)
}

/// All the words on a page, lower-case (for search).
pub fn text_of(p: &Page) -> String {
    let mut s = p.title.to_lowercase();
    for b in p.blocks {
        let mut add = |t: &str| {
            s.push(' ');
            s.push_str(&t.to_lowercase());
        };
        match b {
            H(t) | P(t) | Tip(t) | ShowMe(t, _) => add(t),
            Steps(v) => v.iter().for_each(|t| add(t)),
            Shot(_, marks) => marks.iter().for_each(|mk| add(mk.label)),
            Keys => {}
        }
    }
    s
}
