//! Help page content, part 2, and the full page list.

use super::pages::*;
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
            "Deep Space turns the sky black with stars and nebulae; add a Planet from Create > Worlds.",
        ]),
        H("Lights and cameras"),
        Steps(&[
            "Create > Lights: Omni, Target / Free Spot, Target / Free Directional, Skylight.",
            "Target lights and cameras take two clicks: the light, then its target.",
            "With no lights, a 3ds Max-style headlight rig lights the scene. Ctrl+L forces it on.",
            "C looks through the selected camera.",
        ]),
        ShowMe("Open Environment and Effects", Show::Act(Action::Environment)),
    ],
};

pub const RENDER: Page = Page {
    id: "render",
    title: "10. Rendering",
    blocks: &[
        Shot(
            "render",
            &[m(627.0, 75.0, "Render Setup (F10)"), m(680.0, 75.0, "Rendered Frame Window"), m(735.0, 75.0, "Render (F9)")],
        ),
        Steps(&[
            "F10 Render Setup: resolution presets, alpha background, camera, anti-aliasing, AO, GI, denoiser, bloom, depth of field.",
            "Pick the renderer: the built-in raytracer, or Cycles through Blender (set Blender's path in Preferences > Rendering).",
            "F9 renders the active view into the Rendered Frame Window.",
            "In the Frame Window: save PNG / EXR / JPG, copy, clone the window to compare, and view R / G / B / A channels.",
        ]),
        ShowMe("Open Render Setup", Show::Act(Action::RenderSetup)),
    ],
};

pub const GHOSTTOWN: Page = Page {
    id: "ghosttown",
    title: "11. GhostTown, Medieval Architect and destruction",
    blocks: &[
        Shot(
            "ghosttown",
            &[
                m(1356.0, 464.0, "Settlement preset: downtown, suburbs, walled city, village"),
                m(1486.0, 518.0, "New Seed: a different town"),
                m(1396.0, 864.0, "Assign the GhostTown Material Pack"),
            ],
        ),
        Steps(&[
            "Create > Settlement, then drag out the town area in a viewport.",
            "Change the seed, the block and street sizes, floors and density on the Modify tab.",
            "Damaged / Battlefield Mode, or the Battlefield & Ruin modifier, wrecks the buildings. Blast Zones aim the damage.",
        ]),
        ShowMe("Show the Create tab", Show::Tab(CommandTab::Create)),
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

pub const FILES: Page = Page {
    id: "files",
    title: "13. Files",
    blocks: &[Steps(&[
        "File > New, Open and Save use MiniMax scenes (.mmax).",
        "Import OBJ, glTF / GLB and FBX. Export everything or just the selection.",
        "Autosave keeps a backup every few minutes (Preferences > General).",
        "You can also open a file from the terminal: minimax scene.mmax",
    ])],
};

pub const PREFS: Page = Page {
    id: "prefs",
    title: "14. Preferences and hotkeys",
    blocks: &[
        Shot("prefs", &[m(836.0, 75.0, "Preferences (⚙)"), m(418.0, 68.0, "Tabs: Theme, Hotkeys, Viewports, General, Rendering, Assets")]),
        Steps(&[
            "Theme: pick a preset or make your own colours. Text is checked for readability.",
            "Hotkeys: click a key and press the new one. Choose the 3ds Max key set if you prefer it.",
            "Changes save as soon as you let go of the mouse.",
        ]),
        ShowMe("Open Preferences", Show::Act(Action::Preferences)),
    ],
};

pub const HOTKEYS: Page = Page {
    id: "hotkeys",
    title: "Hotkey cheat sheet",
    blocks: &[P("Your current keys (change them in Preferences > Hotkeys)."), Keys],
};

pub const CREDITS: Page = Page {
    id: "credits",
    title: "Credits",
    blocks: &[
        P("MiniMax is built with Bevy, egui, transform-gizmo, slotmap, boolmesh, meshopt, uvunwrap, noise and image."),
        P("Free CC0 assets: Poly Haven (photo textures and realistic models), Kenney and Quaternius (low-poly models). Thank you!"),
    ],
};

/// Every page, in topic order.
pub const ALL: [Page; 16] =
    [BASICS, CREATE, TRANSFORM, EDIT_POLY, MODIFIERS, SPLINES, MATERIALS, PAINT, SKY, RENDER, GHOSTTOWN, MECH, FILES, PREFS, HOTKEYS, CREDITS];

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
