# Third-party licences

MiniMax includes work from these projects. Their licence texts are in this folder.

| What | Where it is used | Licence |
|---|---|---|
| **MaterialCraft** (ArtCraft Team and the MaterialCraft contributors), https://github.com/storytold/materialcraft | The procedural texture engine in `src/materials/proc_engine/`: tileable lattice noise, the brick, herringbone, plank, Voronoi crack, stone, wood grain, tread plate, weave and scratch generators, Levels, Height Blend, Slope Blur, Normal From Height, cavity AO, the weathering masks and the content-addressed cache design. Ported to MiniMax's per-point map evaluation; each file's header says what it took. | MIT OR Apache-2.0, used here under MIT (`MaterialCraft-LICENSE-MIT.txt`, `MaterialCraft-NOTICE.txt`) |
| **Fira Code** (The Fira Code Project Authors), https://github.com/tonsky/FiraCode | The default interface font: `assets/fonts/FiraCode-Regular.ttf` and `FiraCode-Bold.ttf` (release 5.2), built into the program. | SIL Open Font License 1.1 (`FiraCode-OFL.txt`) |
| **LightCraft** (ArtCraft Team and the LightCraft contributors), https://github.com/storytold/lightcraft | The F9 Color Grading & Camera RAW pipeline in `src/render/grading.rs`: tone and presence curves, Kelvin white balance and tint, split-toning colour wheels, clarity, vignette, chromatic aberration and film grain. Written for MiniMax from LightCraft's published design; no source files were copied. | MIT OR Apache-2.0 (`LightCraft-LICENSE-MIT.txt`, `LightCraft-NOTICE.txt`) |
| **TextureCraft** (ArtCraft Team and the TextureCraft contributors), https://github.com/storytold/texturecraft | The design of the texture painter in `src/paint/`: 64 x 64 copy-on-write texel tiles (`texel/`), the layer stack with blend modes and masks (`layers/`), mesh map baking and Smart Masks (`bake/`), brush alphas and stamps (`brush/`). The code was written for MiniMax from TextureCraft's published design; no source files were copied. | MIT OR Apache-2.0 (`TextureCraft-LICENSE-MIT.txt`, `TextureCraft-NOTICE.txt`) |

The optional asset packs (Preferences → Assets) are downloaded by the user, not shipped: Poly Haven textures and models and Kenney models, all CC0.
