//! Every command in Ilex as one `Action`: its label, tooltip and what it
//! does. The toolbar, menus, ribbon, navigation buttons and hotkeys all
//! go through this table, and a test checks that every action has a visible
//! button or menu entry ("a button for everything").

pub mod keymap;
pub mod keys;
pub mod layout;
mod run;
mod tips;
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
    ImportVdb,
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
    OrientationCube,
    ShowGizmo,
    GizmoSmaller,
    GizmoBigger,
    Wireframe,
    EdgedFaces,
    PreviewVolumes,
    ZoomExtents,
    ZoomExtentsAll,
    Maximize,
    SceneExplorer,
    CameraView,
    CameraFromView,
    Orthographic,
    Ribbon,
    Nav(NavMode),
    // Rendering
    Render,
    RenderLast,
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
            NewScene, OpenScene, SaveScene, SaveSceneAs, Import, ImportVdb, ExportAll, ExportSelected, Undo, Redo, Clone, Delete, SelectAll,
            SelectNone, SelectInvert, SelectByName, LockSelection, WindowCrossing, Cancel, RepeatLast, SelectObject, Move, Rotate,
            Scale, Snap, AngleSnap, PercentSnap, SnapSettings,
        ];
        v.extend(SUBS.map(Sub));
        v.extend([
            SelectRing, SelectLoop, RemoveSubObjects, Array, Mirror, Align, Isolate, JointTool, ContactSets, Paint, BuildingsBrowser, CharacterStudio, FaceCloner, ConvertToPoly,
            ConvertToSpline, Measure, HideSelected, HideUnselected, UnhideAll, FreezeSelected, UnfreezeAll, SeeThrough, ShadowCatcher, ToggleGrid, OrientationCube, ShowGizmo, GizmoSmaller, GizmoBigger,
            Wireframe, EdgedFaces, PreviewVolumes, ZoomExtents, ZoomExtentsAll, Maximize, SceneExplorer, CameraView, CameraFromView, Orthographic, Ribbon,
        ]);
        v.extend(SplineConstraint::ALL.map(Constrain));
        v.extend(NAVS.map(Nav));
        v.extend([Render, RenderLast, RenderSetup, FrameWindow, ColorGrading, LightMix, Turntable, MaterialEditor, MaterialLibrary, Environment, DefaultLights]);
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
            ImportVdb => "Import VDB Volume...",
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
            OrientationCube => "Orientation Cube",
            ShowGizmo => "Show Transform Gizmo",
            GizmoSmaller => "Gizmo Size −",
            GizmoBigger => "Gizmo Size +",
            Wireframe => "Wireframe",
            EdgedFaces => "Edged Faces",
            PreviewVolumes => "Preview Volumes",
            ZoomExtents => "Zoom Extents",
            ZoomExtentsAll => "Zoom Extents All",
            Maximize => "Maximize Viewport",
            SceneExplorer => "Scene Explorer",
            CameraView => "Camera View",
            CameraFromView => "Create Camera From View",
            Orthographic => "Orthographic / Perspective",
            Ribbon => "Modeling Ribbon",
            Nav(NavMode::Zoom) => "Zoom",
            Nav(NavMode::ZoomAll) => "Zoom All",
            Nav(NavMode::Fov) => "Field of View",
            Nav(NavMode::Pan) => "Pan View",
            Nav(NavMode::Orbit) => "Orbit",
            Render => "Render",
            RenderLast => "Render Last",
            RenderSetup => "Render Setup...",
            FrameWindow => "Veach Rendered Frame Window",
            ColorGrading => "Color Grading & Camera RAW",
            LightMix => "Luxon LightMix (Relight Render)",
            Turntable => "Render Turntable / Sprite Sheet...",
            BuildingsBrowser => "Prefab Library...",
            CharacterStudio => "MakeHuman...",
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
            BuildingsBrowser => "🏠 Prefabs",
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
}
