//! Every command in Ilex as one `Action`: its label, tooltip and what it
//! does. The toolbar, menus, ribbon, navigation buttons and hotkeys all
//! go through this table, and a test checks that every action has a visible
//! button or menu entry ("a button for everything").

pub mod keymap;
pub mod keys;
pub mod layout;
mod run;
#[cfg(test)]
mod tests;

use crate::editor::NavMode;
use crate::mesh::SubObject;
use crate::render::sky_presets::SkyPreset;
use crate::splines::constraints::SplineConstraint;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    // File
    NewScene,
    OpenScene,
    SaveScene,
    SaveSceneAs,
    Import,
    ExportAll,
    ExportSelected,
    // Edit
    Undo,
    Redo,
    Clone,
    Delete,
    SelectAll,
    SelectNone,
    SelectInvert,
    SelectByName,
    LockSelection,
    WindowCrossing,
    Cancel,
    RepeatLast,
    // Transform and snaps
    SelectObject,
    Move,
    Rotate,
    Scale,
    Snap,
    AngleSnap,
    PercentSnap,
    SnapSettings,
    // Sub-objects
    Sub(SubObject),
    SelectRing,
    SelectLoop,
    RemoveSubObjects,
    // Tools
    Array,
    Mirror,
    Align,
    Isolate,
    JointTool,
    ContactSets,
    Paint,
    ConvertToPoly,
    ConvertToSpline,
    Measure,
    /// One-shot Editable Spline sketch constraint.
    Constrain(SplineConstraint),
    // Display
    HideSelected,
    HideUnselected,
    UnhideAll,
    FreezeSelected,
    UnfreezeAll,
    SeeThrough,
    ShadowCatcher,
    // Views
    ToggleGrid,
    Wireframe,
    EdgedFaces,
    ZoomExtents,
    ZoomExtentsAll,
    Maximize,
    SceneExplorer,
    CameraView,
    Ribbon,
    Nav(NavMode),
    // Rendering
    Render,
    RenderSetup,
    FrameWindow,
    ColorGrading,
    LightMix,
    Turntable,
    BuildingsBrowser,
    CharacterStudio,
    FaceCloner,
    MaterialEditor,
    MaterialLibrary,
    Environment,
    DefaultLights,
    Sky(SkyPreset),
    // Help
    Help,
    Tour,
    HotkeyList,
    About,
    CrashReports,
    Preferences,
}

const SUBS: [SubObject; 6] =
    [SubObject::Vertex, SubObject::Edge, SubObject::Border, SubObject::Polygon, SubObject::Element, SubObject::Object];
const NAVS: [NavMode; 5] = [NavMode::Zoom, NavMode::ZoomAll, NavMode::Fov, NavMode::Pan, NavMode::Orbit];

impl Action {
    /// Every action, in Hotkey Editor order.
    pub fn all() -> Vec<Action> {
        use Action::*;
        let mut v = vec![
            NewScene, OpenScene, SaveScene, SaveSceneAs, Import, ExportAll, ExportSelected, Undo, Redo, Clone, Delete, SelectAll,
            SelectNone, SelectInvert, SelectByName, LockSelection, WindowCrossing, Cancel, RepeatLast, SelectObject, Move, Rotate,
            Scale, Snap, AngleSnap, PercentSnap, SnapSettings,
        ];
        v.extend(SUBS.map(Sub));
        v.extend([
            SelectRing, SelectLoop, RemoveSubObjects, Array, Mirror, Align, Isolate, JointTool, ContactSets, Paint, BuildingsBrowser, CharacterStudio, FaceCloner, ConvertToPoly,
            ConvertToSpline, Measure, HideSelected, HideUnselected, UnhideAll, FreezeSelected, UnfreezeAll, SeeThrough, ShadowCatcher, ToggleGrid,
            Wireframe, EdgedFaces, ZoomExtents, ZoomExtentsAll, Maximize, SceneExplorer, CameraView, Ribbon,
        ]);
        v.extend(SplineConstraint::ALL.map(Constrain));
        v.extend(NAVS.map(Nav));
        v.extend([Render, RenderSetup, FrameWindow, ColorGrading, LightMix, Turntable, MaterialEditor, MaterialLibrary, Environment, DefaultLights]);
        v.extend(SkyPreset::ALL.map(Sky));
        v.extend([Help, Tour, HotkeyList, About, CrashReports, Preferences]);
        v
    }

    /// Short button / menu label.
    pub fn label(self) -> String {
        use Action::*;
        let s = match self {
            NewScene => "New Scene",
            OpenScene => "Open...",
            SaveScene => "Save",
            SaveSceneAs => "Save As...",
            Import => "Import...",
            ExportAll => "Export All...",
            ExportSelected => "Export Selected...",
            Undo => "Undo",
            Redo => "Redo",
            Clone => "Clone",
            Delete => "Delete",
            SelectAll => "Select All",
            SelectNone => "Select None",
            SelectInvert => "Select Invert",
            SelectByName => "Select by Name...",
            LockSelection => "Lock Selection",
            WindowCrossing => "Window / Crossing",
            Cancel => "Cancel Tool",
            RepeatLast => "Repeat Last",
            SelectObject => "Select Object",
            Move => "Select and Move",
            Rotate => "Select and Rotate",
            Scale => "Select and Scale",
            Snap => "3D Snaps",
            AngleSnap => "Angle Snap",
            PercentSnap => "Percent Snap",
            SnapSettings => "Grid and Snap Settings...",
            Sub(SubObject::Object) => "Top Level (Object)",
            Sub(s) => return s.label().to_string(),
            SelectRing => "Ring",
            SelectLoop => "Loop",
            RemoveSubObjects => "Remove",
            Array => "Array...",
            Mirror => "Mirror...",
            Align => "Align...",
            Isolate => "Isolate Selection",
            JointTool => "Joint Tool",
            ContactSets => "Contact Sets",
            Paint => "Viewport Paint",
            ConvertToPoly => "Convert to Editable Poly",
            ConvertToSpline => "Convert to Editable Spline",
            Measure => "Measure...",
            Constrain(c) => return c.label().to_string(),
            HideSelected => "Hide Selected",
            HideUnselected => "Hide Unselected",
            UnhideAll => "Unhide All",
            FreezeSelected => "Freeze Selected",
            UnfreezeAll => "Unfreeze All",
            SeeThrough => "See-Through",
            ShadowCatcher => "Shadow & Reflection Catcher",
            ToggleGrid => "Show Grid",
            Wireframe => "Wireframe",
            EdgedFaces => "Edged Faces",
            ZoomExtents => "Zoom Extents",
            ZoomExtentsAll => "Zoom Extents All",
            Maximize => "Maximize Viewport",
            SceneExplorer => "Scene Explorer",
            CameraView => "Camera View",
            Ribbon => "Modeling Ribbon",
            Nav(NavMode::Zoom) => "Zoom",
            Nav(NavMode::ZoomAll) => "Zoom All",
            Nav(NavMode::Fov) => "Field of View",
            Nav(NavMode::Pan) => "Pan View",
            Nav(NavMode::Orbit) => "Orbit",
            Render => "Render",
            RenderSetup => "Render Setup...",
            FrameWindow => "Veach Rendered Frame Window",
            ColorGrading => "Color Grading & Camera RAW",
            LightMix => "Luxon LightMix (Relight Render)",
            Turntable => "Render Turntable / Sprite Sheet...",
            BuildingsBrowser => "Demiurge Buildings & Props Browser...",
            CharacterStudio => "Character & Creature Studio...",
            FaceCloner => "Face Cloner (Photos / Frames)...",
            MaterialEditor => "Material Editor",
            MaterialLibrary => "Material Library...",
            Environment => "Environment and Effects...",
            DefaultLights => "Default Lights",
            Sky(p) => return p.label().to_string(),
            Help => "Help...",
            Tour => "Getting Started Tour",
            HotkeyList => "Hotkey List",
            About => "About Ilex 3D",
            CrashReports => "View Crash Reports...",
            Preferences => "Preferences...",
        };
        s.to_string()
    }

    /// Compact toolbar caption (an icon plus a word or two).
    pub fn short(self) -> String {
        use Action::*;
        let s = match self {
            Undo => "↺",
            Redo => "↻",
            SelectObject => "▣ Q",
            Move => "✚ W",
            Rotate => "⟲ E",
            Scale => "⬈ R",
            Snap => "⌖ S",
            AngleSnap => "◔ A",
            PercentSnap => "% Snap",
            SnapSettings => "⚙",
            SelectByName => "☰ Name",
            LockSelection => "🔒",
            WindowCrossing => "◑",
            Mirror => "⬌ Mirror",
            Align => "⬍ Align",
            Array => "🗐 Array",
            Clone => "⎘ Clone",
            Paint => "🖊 Paint",
            ContactSets => "Contact",
            BuildingsBrowser => "🏠 Buildings",
            CharacterStudio => "♀♂ Characters",
            FaceCloner => "📸 Face Cloner",
            Isolate => "◎",
            SceneExplorer => "🗄",
            Ribbon => "🛠 Ribbon",
            DefaultLights => "💡 Default",
            Environment => "☀ Environment",
            MaterialEditor => "🎨 Material",
            MaterialLibrary => "📚 Library",
            RenderSetup => "⚙ Setup",
            FrameWindow => "🖼",
            Render => "▶ Render",
            Help => "❓ Help",
            Tour => "▶ Tour",
            HotkeyList => "⌨",
            About => "ℹ",
            CrashReports => "Crash Reports",
            Preferences => "⚙",
            Nav(NavMode::Zoom) => "🔍",
            Nav(NavMode::ZoomAll) => "🔍⁴",
            Nav(NavMode::Fov) => "FOV",
            Nav(NavMode::Pan) => "✋",
            Nav(NavMode::Orbit) => "⟳",
            ZoomExtents => "⛶",
            ZoomExtentsAll => "⛶⁴",
            Maximize => "⊞",
            Sky(p) => return format!("{} {}", p.icon(), p.label()),
            _ => return self.label(),
        };
        s.to_string()
    }

    /// One-line explanation shown on hover.
    pub fn tip(self) -> &'static str {
        use Action::*;
        match self {
            Preferences => "Theme, hotkeys, viewport colours, autosave, units, render defaults and asset packs",
            SelectByName => "Select From Scene: pick objects from a searchable list",
            LockSelection => "Lock Selection: clicks and marquees no longer change what is selected",
            WindowCrossing => "Window (fully inside) or Crossing (touching) region selection",
            PercentSnap => "Scale in 10 % steps",
            SnapSettings => "Snap types, grid spacing and snap strength",
            Isolate => "Show only the selection (press again to show everything)",
            JointTool => "Click two parts to link them with a joint",
            ContactSets => "Contact Member parts push each other along their hinges and sliders",
            Paint => "Viewport Paint: VertexPaint / texture projection (PaintBox)",
            Measure => "Size, centre and counts of the selection",
            Constrain(c) => c.tip(),
            SeeThrough => "Draw the selected objects semi-transparent",
            ShadowCatcher => "Render the selected ground transparent except for the shadows, AO and reflections cast on it",
            CameraView => "Look through the selected (or first) camera",
            Ribbon => "Big buttons for every Edit Poly tool under the toolbar",
            Nav(m) => m.label(),
            DefaultLights => "Light the viewports with the 2-light headlight rig, even with scene lights",
            Environment => "Background, HDRI, sky, deep space, ambient light and fog",
            Sky(_) => "One-click sky (opens the Environment window to fine-tune it)",
            MaterialLibrary => "Browse ~280 ready-made materials (ground, roads, walls, metal, glass...) and apply one with a click",
            FaceCloner => "Fit the selected character's head to photos or exported video frames of a person",
            CharacterStudio => "MakeHuman-based RPG characters (Elf, Dwarf, Orc, anime...) with hair, outfits, weapons, plus creatures",
            BuildingsBrowser => "Visual browser of the 43 standalone buildings, bridges and street props: click a tile, then drag in a viewport",
            Turntable => "Render a 360° turntable PNG sequence or a 4 / 8-direction isometric sprite atlas + JSON",
            LightMix => "Relight the finished F9 render per light group (sun, sky, emissive, each light) without re-rendering",
            ColorGrading => "Grade the F9 render live: tone, white balance, colour wheels, vignette, grain",
            Render => "Render the active viewport into the Veach Rendered Frame Window",
            RemoveSubObjects => "Remove the selected vertices or edges",
            RepeatLast => "Repeat the previous Edit Poly operation",
            Cancel => "Leave the current tool, caddy or menu",
            Help => "How Ilex works, with screenshots (F1 opens the page for what is under the mouse)",
            Tour => "A short step-by-step tour of the screen",
            CrashReports => "Crash reports Ilex wrote after a crash: copy one for Claude, restore the unsaved scene, or simulate a test report",
            _ => "",
        }
    }
}
