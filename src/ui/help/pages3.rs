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
        Shot(
            "characters",
            &[
                m(960.0, 660.0, "An Elf in the Wave pose"),
                m(1070.0, 790.0, "A Dire Wolf creature"),
                m(560.0, 160.0, "Studio tiles: click one, then click or drag in a viewport"),
            ],
        ),
        H("Making a character"),
        Steps(&[
            "Click ♀♂ Characters on the toolbar (or Tools > Character & Creature Studio..., or Create > Geometry — Characters & Creatures).",
            "Click an archetype (Human, Elf, Dwarf, Goblin, Orc, Chibi, Anime Hero or Anime Heroine), then click in a viewport to place it, feet on the ground, or drag to set its height like a 3ds Max Biped. The tool stays armed; Place ... at Origin drops one at 0,0,0.",
            "Body (MakeHuman Macros): Gender, Age, Muscle, Weight, Height, Proportions, Breast size and firmness, and the African / Asian / Caucasian mix.",
            "Anime Ratio grows the head and eyes and slims the body toward a toon look; 0 keeps realistic proportions.",
            "Eyes, Hair (Bald, Crop, Long, Ponytail, Spiky, Bun, Beard), Outfit (Bodysuit, Tunic, Knight, Ranger, Robe) and Weapon (Sword, Axe, Staff, Shield).",
            "Detail Sliders: every MakeHuman face and body target (nose, ears, chin, torso, legs...). Search filters them; Mirror L/R moves both sides together. Right-click a slider to reset it.",
            "Creatures: Wolf, Dragon, Spider, Slime, Golem and Serpent, with a Size slider and a Seed.",
            "Material IDs: 1 Skin, 2 Eyes, 3 Hair, 4 Cloth, 5 Metal, 6 Leather, then 7 Eyebrows, 8 Eyelashes, 9 Teeth, 10 Tongue, 11 MakeHuman Hair and 12 upwards for each MakeHuman clothing piece. The Character Pack Multi/Sub-Object material (with skin subsurface scattering) is assigned for you.",
            "Every change is one undo step; Convert to Editable Poly freezes the character for hand modelling.",
        ]),
        H("MakeHuman hair, clothes and skins (CC0)"),
        Shot(
            "wardrobe",
            &[
                m(975.0, 680.0, "MakeHuman ponytail, casual suit, shoes and skin"),
                m(1065.0, 700.0, "Short hair, another suit and a middle-aged African male skin"),
                m(1430.0, 500.0, "Scroll the Modify tab down to Character — MakeHuman Assets"),
            ],
        ),
        Steps(&[
            "Select a humanoid and open Character — MakeHuman Assets (CC0) on the Modify tab.",
            "Pick Eyebrows (12), Eyelashes (4), Teeth (6), Tongue, Hair (10), a Skin (23) and an Eye Colour (9) from the drop-downs, or None.",
            "Tick any of the 20 clothing pieces in the Clothes list; several can be worn at once (up to 12). Body faces under a piece are hidden so the skin does not poke through.",
            "Body Topology swaps the base body for a low-poly MakeHuman proxy (proxy741, female1605 or male1591).",
            "Every piece follows the body sliders and the pose, and gets its own textured sub-material in the Character Pack; edits you make in the Material Editor are kept.",
            "Face Kit puts on the default eyebrows, eyelashes, teeth and tongue (new Humans start with it); Take Off All Clothes and Remove All Assets clear them again.",
            "Picking MakeHuman hair replaces the procedural hair and beard.",
        ]),
        H("Easy Pose (the rig)"),
        Steps(&[
            "Every humanoid carries the MakeHuman default skeleton (163 bones), fitted to its own body shape, with MakeHuman's skin weights.",
            "Character — Easy Pose: click a preset (A-Pose, T-Pose, Relaxed, Walk, Run, Sit, Kneel, Wave, Fighting Stance, Victory, Spell Cast). Presets shape the hands too: fists for Fighting Stance and Victory, a loose grip for Run.",
            "Or pick a Joint (Whole Body, Spine, Chest, Neck, Head, Jaw, Clavicle, Upper Arm, Elbow, Wrist, Thigh, Knee, Foot) and drag Bend, Twist and Side. Left and right move together unless 'Pose this side only' is on.",
            "Hands: Open, Relaxed, Grip, Fist or Point curl every finger (both hands, or only the picked side with 'Pose this side only').",
            "IK Move: pick Hand or Foot and a side, then click Up, Down, Forward, Back, Out or In to move it 5 cm; the elbow or knee bends to follow (two-bone IK).",
            "Copy L to R, Copy R to L, Mirror Pose, Reset Joint and Reset Pose. Save Pose... writes a .ilexpose file; Load Pose... opens .ilexpose and older .mmaxpose files.",
            "The body and eyes follow the pose and the feet stay on the ground. Every pose change is one undo step.",
        ]),
        H("Clone a face from photos or video frames"),
        Steps(&[
            "Select a humanoid, then click Load Photos / Frame Folder... in the 📸 Clone Face from Photos / Videos rollout (or Tools > Face Cloner (Photos / Frames)..., or 📸 Face Cloner on the toolbar).",
            "Load Photos... takes one or more photos of the same person; Load Frame Folder... reads every picture in a folder (every n-th one if you set the stride). Videos are not read directly: export the clip to frames first. A slow head turn from one side to the other gives every view.",
            "MediaPipe finds 478 face landmarks with depth on every picture. Blurry frames, near-duplicates and (with Neutral expression only) open mouths are dropped, and the best frames per view are kept: profile, three-quarter and front, each side.",
            "Untick a picture to leave it out, or click Pins... to drag a landmark where it belongs (right-click a pinned dot to unpin it).",
            "Click ✨ Clone Face onto <name>. The MakeHuman head sliders are fitted across all views, the fit is re-checked on renders of the new head, then a fine warp adds what the sliders cannot reach. It fades out above the neck. One undo step.",
            "Warp Strength (right-click resets to 1) scales the fine warp; Reset Cloned Face takes the clone off. The Detail Sliders refine the result.",
        ]),
        Tip("The clone reshapes the existing hm08 head, so topology, UVs, the rig and every MakeHuman asset still fit; the head keeps the body's scale. Ears and the back of the head are guessed. Accuracy is limited by the landmark detector: on test renders it closes about a quarter to two fifths of the true 3D shape difference, so treat the result as a strong starting point. Not done yet: hair and background masks, a 360° skin texture from the photos, skin-tone match and saved face presets."),
        Tip("Not done yet: the MakeHuman normal, AO, bump and specular maps and litspheres (only the colour and alpha textures are imported); clothes follow the body binding, not their own skin weights; no extra MakeHuman community packs, expressions or extra rigs; no asset thumbnails; and height / proportion targets for ages other than young adult. Posing is forward kinematics plus two-bone IK nudges for the hands and feet (no IK goals that stay pinned while you pose the body, fingers only through the five hand shapes, no face bones, no animation), and the procedural hair, armour and weapons are placed from the posed joints but are not bent with the limbs."),
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
