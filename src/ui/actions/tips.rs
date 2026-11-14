//! Hover tips for actions.

use super::Action;

impl Action {
    /// One-line explanation shown on hover.
    pub fn tip(self) -> &'static str {
        use Action::*;
        match self {
            ImportVdb => "Simulated smoke and fire from an OpenVDB (.vdb) file, as a volume",
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
            ShowGizmo => "Show or hide the Move / Rotate / Scale gizmo",
            GizmoSmaller | GizmoBigger => "Make the transform gizmo smaller or larger",
            PreviewVolumes => "Draw clouds as soft volumes in Perspective views (off: the solid preview puffs)",
            ShadowCatcher => "Render the selected ground transparent except for the shadows, AO and reflections cast on it",
            CameraFromView => "Turn the active Perspective view into a camera and look through it",
            Orthographic => "Switch the active view between perspective and orthographic (no depth distortion), keeping the angle",
            CameraView => "Look through the selected (or first) camera",
            Ribbon => "Big buttons for every Edit Poly tool under the toolbar",
            Nav(m) => m.label(),
            DefaultLights => "Light the viewports with the 2-light headlight rig, even with scene lights",
            Environment => "Background, HDRI, sky, deep space, ambient light and fog",
            Sky(_) => "One-click sky (opens the Environment window to fine-tune it)",
            MaterialLibrary => "Browse ~280 ready-made materials (ground, roads, walls, metal, glass...) and apply one with a click",
            FaceCloner => "Fit the selected character's head to photos or exported video frames of a person",
            CharacterStudio => "The MakeHuman window: characters (Human, Elf, Dwarf, anime...) with hair, eyes, clothes, skins, poses and Randomise, plus creatures and an asset Library",
            BuildingsBrowser => "Prefab Library: thumbnails of every building, bridge and street prop, with favourites; click a tile, then click in a viewport to drop it",
            Turntable => "Render a 360° turntable PNG sequence or a 4 / 8-direction isometric sprite atlas + JSON",
            LightMix => "Relight the finished F9 render per light group (sun, sky, emissive, each light) without re-rendering",
            ColorGrading => "Grade the F9 render live: tone, white balance, colour wheels, vignette, grain",
            Render => "Render the active viewport (or the Render Setup camera) into the Veach Rendered Frame Window",
            RenderLast => "Render the same view or camera as the previous render again",
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
