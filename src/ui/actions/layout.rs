//! Where every action lives: the top menus, the main toolbar, the
//! bottom-right navigation buttons and the Modeling Ribbon. The rule test
//! walks these tables to prove every action has a visible button.

use super::Action;
use super::Action::*;
use crate::editor::NavMode;
use crate::mesh::SubObject;
use crate::render::sky_presets::SkyPreset;

/// One menu row.
pub enum Entry {
    A(Action),
    Sep,
    /// A cascading sub-menu.
    Sub(&'static str, &'static [Entry]),
}

use Entry::{A, Sep};

const TRANSFORM: &[Entry] = &[A(SelectObject), A(Move), A(Rotate), A(Scale), Sep, A(Snap), A(AngleSnap), A(PercentSnap), A(SnapSettings)];

const SUB_LEVELS: &[Entry] = &[
    A(Sub(SubObject::Vertex)),
    A(Sub(SubObject::Edge)),
    A(Sub(SubObject::Border)),
    A(Sub(SubObject::Polygon)),
    A(Sub(SubObject::Element)),
    Sep,
    A(Sub(SubObject::Object)),
    Sep,
    A(SelectRing),
    A(SelectLoop),
    A(RemoveSubObjects),
    A(RepeatLast),
];

const DISPLAY: &[Entry] =
    &[A(HideSelected), A(HideUnselected), A(UnhideAll), Sep, A(FreezeSelected), A(UnfreezeAll), Sep, A(SeeThrough), A(Isolate)];

const NAVIGATION: &[Entry] = &[
    A(Nav(NavMode::Zoom)),
    A(Nav(NavMode::ZoomAll)),
    A(Nav(NavMode::Fov)),
    A(Nav(NavMode::Pan)),
    A(Nav(NavMode::Orbit)),
];

const SKIES: &[Entry] = &[
    A(Sky(SkyPreset::ClearDay)),
    A(Sky(SkyPreset::Sunset)),
    A(Sky(SkyPreset::Overcast)),
    A(Sky(SkyPreset::Night)),
    A(Sky(SkyPreset::DeepSpace)),
    A(Sky(SkyPreset::Nebula)),
    Sep,
    A(Sky(SkyPreset::Off)),
];

/// The menu bar. "Create" and "Modifiers" are generated from the Create tab
/// and the Modifier List and are drawn between "Views" and "Rendering".
pub const MENUS: &[(&str, &[Entry])] = &[
    (
        "File",
        &[A(NewScene), A(OpenScene), Sep, A(SaveScene), A(SaveSceneAs), Sep, A(Import), A(ExportAll), A(ExportSelected)],
    ),
    (
        "Edit",
        &[
            A(Undo),
            A(Redo),
            Sep,
            A(Clone),
            A(Delete),
            Sep,
            A(SelectAll),
            A(SelectNone),
            A(SelectInvert),
            A(SelectByName),
            Sep,
            A(LockSelection),
            A(WindowCrossing),
            Entry::Sub("Transform", TRANSFORM),
            Sep,
            A(Cancel),
            Sep,
            A(Preferences),
        ],
    ),
    (
        "Tools",
        &[
            A(Mirror),
            A(Array),
            A(Align),
            Sep,
            A(Isolate),
            A(Measure),
            Sep,
            A(Paint),
            A(JointTool),
            A(ContactSets),
            Sep,
            Entry::Sub("Sub-Object Level", SUB_LEVELS),
            A(ConvertToPoly),
            A(ConvertToSpline),
        ],
    ),
    (
        "Views",
        &[
            A(ToggleGrid),
            A(Wireframe),
            A(EdgedFaces),
            A(SeeThrough),
            Sep,
            A(ZoomExtents),
            A(ZoomExtentsAll),
            A(Maximize),
            Entry::Sub("Navigation", NAVIGATION),
            Sep,
            A(CameraView),
            Sep,
            A(SceneExplorer),
            A(Ribbon),
            Entry::Sub("Display", DISPLAY),
        ],
    ),
    (
        "Rendering",
        &[
            A(Render),
            A(RenderSetup),
            A(FrameWindow),
            Sep,
            A(MaterialEditor),
            A(MaterialLibrary),
            Sep,
            A(Environment),
            Entry::Sub("Sky", SKIES),
            A(DefaultLights),
        ],
    ),
    ("Help", &[A(Help), A(Tour), A(HotkeyList), Sep, A(About)]),
];

/// One slot of the main toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    A(Action),
    Sep,
    /// Selection region shape dropdown.
    Region,
    /// Selection Filter dropdown.
    Filter,
    /// Reference Coordinate System dropdown.
    Coord,
    /// Use Center (pivot) dropdown.
    Pivot,
    /// One-click sky presets.
    SkyMenu,
}

/// Left-aligned toolbar slots, in 3ds Max order.
pub const TOOLBAR_LEFT: &[Slot] = &[
    Slot::A(Undo),
    Slot::A(Redo),
    Slot::Sep,
    Slot::Filter,
    Slot::A(SelectObject),
    Slot::A(SelectByName),
    Slot::Region,
    Slot::A(WindowCrossing),
    Slot::Sep,
    Slot::A(Move),
    Slot::A(Rotate),
    Slot::A(Scale),
    Slot::Coord,
    Slot::Pivot,
    Slot::Sep,
    Slot::A(Snap),
    Slot::A(AngleSnap),
    Slot::A(PercentSnap),
    Slot::A(SnapSettings),
    Slot::Sep,
    Slot::A(Mirror),
    Slot::A(Align),
    Slot::A(Array),
    Slot::A(Clone),
    Slot::A(Isolate),
    Slot::Sep,
    Slot::A(Paint),
    Slot::A(ContactSets),
];

/// Right-aligned toolbar slots, listed left to right.
pub const TOOLBAR_RIGHT: &[Slot] = &[
    Slot::A(SceneExplorer),
    Slot::A(Ribbon),
    Slot::Sep,
    Slot::A(DefaultLights),
    Slot::A(Environment),
    Slot::SkyMenu,
    Slot::Sep,
    Slot::A(MaterialEditor),
    Slot::A(MaterialLibrary),
    Slot::A(RenderSetup),
    Slot::A(FrameWindow),
    Slot::A(Render),
    Slot::Sep,
    Slot::A(HotkeyList),
    Slot::A(Preferences),
    Slot::A(Help),
];

/// Bottom-right viewport navigation buttons (2 rows, like 3ds Max).
pub const NAV_BUTTONS: [[Action; 4]; 2] = [
    [Nav(NavMode::Zoom), Nav(NavMode::ZoomAll), ZoomExtents, ZoomExtentsAll],
    [Nav(NavMode::Fov), Nav(NavMode::Pan), Nav(NavMode::Orbit), Maximize],
];

/// Every action reachable from a menu, the toolbar or the nav buttons.
pub fn placed_actions() -> Vec<Action> {
    fn walk(entries: &[Entry], out: &mut Vec<Action>) {
        for e in entries {
            match e {
                Entry::A(a) => out.push(*a),
                Entry::Sep => {}
                Entry::Sub(_, sub) => walk(sub, out),
            }
        }
    }
    let mut out = Vec::new();
    for (_, entries) in MENUS {
        walk(entries, &mut out);
    }
    for s in TOOLBAR_LEFT.iter().chain(TOOLBAR_RIGHT) {
        if let Slot::A(a) = s {
            out.push(*a);
        }
    }
    out.extend(NAV_BUTTONS.iter().flatten());
    out
}
