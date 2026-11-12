//! Help page content, part 3: Phase 10A / 10B characters and the Ilex names.

use super::pages::*;
use crate::editor::CommandTab;
use crate::ui::actions::Action;
use Block::*;

pub const CHARACTERS: Page = Page {
    id: "characters",
    title: "17. Characters and creatures (MakeHuman)",
    blocks: &[
        P("The Character & Creature Studio builds RPG characters on the MakeHuman hm08 base body (CC0), so every human, elf, dwarf or anime hero starts from a real, clean-topology body."),
        H("Making a character"),
        Steps(&[
            "Click ♀♂ Characters on the toolbar (or Tools > Character & Creature Studio..., or Create > Geometry — Characters & Creatures).",
            "Click an archetype (Human, Elf, Dwarf, Goblin, Orc, Chibi, Anime Hero or Anime Heroine), then click in a viewport to place it, feet on the ground, or drag to set its height like a 3ds Max Biped. The tool stays armed; Place ... at Origin drops one at 0,0,0.",
            "Body (MakeHuman Macros): Gender, Age, Muscle, Weight, Height, Proportions, Breast size and firmness, and the African / Asian / Caucasian mix.",
            "Anime Ratio grows the head and eyes and slims the body toward a toon look; 0 keeps realistic proportions.",
            "Eyes, Hair (Bald, Crop, Long, Ponytail, Spiky, Bun, Beard), Outfit (Bodysuit, Tunic, Knight, Ranger, Robe) and Weapon (Sword, Axe, Staff, Shield).",
            "Detail Sliders: every MakeHuman face and body target (nose, ears, chin, torso, legs...). Search filters them; Mirror L/R moves both sides together. Right-click a slider to reset it.",
            "Creatures: Wolf, Dragon, Spider, Slime, Golem and Serpent, with a Size slider and a Seed.",
            "Material IDs: 1 Skin, 2 Eyes, 3 Hair, 4 Cloth, 5 Metal, 6 Leather. The Character Pack Multi/Sub-Object material (with skin subsurface scattering) is assigned for you.",
            "Every change is one undo step; Convert to Editable Poly freezes the character for hand modelling.",
        ]),
        H("Easy Pose (the rig)"),
        Steps(&[
            "Every humanoid carries the MakeHuman default skeleton (163 bones), fitted to its own body shape, with MakeHuman's skin weights.",
            "Character — Easy Pose: click a preset (A-Pose, T-Pose, Relaxed, Walk, Run, Sit, Kneel, Wave, Fighting Stance, Victory, Spell Cast).",
            "Or pick a Joint (Whole Body, Spine, Chest, Neck, Head, Jaw, Clavicle, Upper Arm, Elbow, Wrist, Thigh, Knee, Foot) and drag Bend, Twist and Side. Left and right move together unless 'Pose this side only' is on.",
            "Copy L to R, Copy R to L, Mirror Pose, Reset Joint and Reset Pose. Save Pose... writes a .ilexpose file; Load Pose... opens .ilexpose and older .mmaxpose files.",
            "The body and eyes follow the pose and the feet stay on the ground. Every pose change is one undo step.",
        ]),
        Tip("Not done yet: the photo face cloner, real MakeHuman hair and clothes assets (hair and outfits are built procedurally), and height / proportion targets for ages other than young adult. Posing is forward kinematics only (no IK, no fingers or face bones in Easy Pose, no animation), and the procedural hair, armour and weapons are placed from the posed joints but are not bent with the limbs."),
        ShowMe("Open the Character & Creature Studio", Show::Act(Action::CharacterStudio)),
        ShowMe("Show the Create tab", Show::Tab(CommandTab::Create)),
    ],
};

pub const NAMES: Page = Page {
    id: "names",
    title: "18. Ilex, Veach, Luxon and Demiurge",
    blocks: &[
        P("MiniMax is now Ilex 3D, named after the Common Holly (Ilex aquifolium) in its logo. Nothing you made is lost."),
        H("Your old files"),
        Steps(&[
            "Open reads Ilex scenes (.ilex) and scenes saved before the rename (.mmax) side by side. Saving an old scene writes a .ilex beside it and leaves the .mmax alone.",
            "Poses use .ilexpose; older .mmaxpose files are recognised too.",
            "The old minimax command still starts the app, and preferences, hotkeys, assets and crash reports in ~/.config/minimax and ~/.local/share/minimax keep working until an ilex folder exists.",
            "Old modifier names (Terrain, Ocean, GhostTown, Medieval Architect, Battlefield / Ruin) in saved Show Buttons sets still add the right modifier.",
        ]),
        H("The family names"),
        Steps(&[
            "Veach: the path tracer in F10 (Veach Path Tracer), its Veach MIS & Adaptive Sampling settings, the Veach À-Trous Wavelet Denoiser, and the F9 Veach Rendered Frame Window.",
            "Luxon: light and optics. Luxon LightMix in F9, Luxon Photometric & Area Lights on a light's Modify panel, Luxon Volumetric Godrays & Mist in the 8 dialog, and Luxon Lens FX (Bokeh, Anamorphic & Glare) in F10.",
            "Demiurge: world building. Demiurge Worlds (Terrain, Ocean, Planet, Smoke Plume) and Demiurge Settlements (GhostTown, Buildings & Props) on the Create tab, and the Demiurge Terrain, Ocean, GhostTown, Medieval Architect and Battlefield & Ruin modifiers.",
        ]),
        ShowMe("Open Render Setup", Show::Act(Action::RenderSetup)),
    ],
};
