# Ilex: what it has and what's planned

*As of 14 November 2026. Everything below "What Ilex has" is merged to `main` (up to PR #36). Details for each feature are in `PROJECT_STATUS.md`, by section number (§).*

**Size:** 689 `.rs` files, about 103,000 lines, none over 350 lines; 796 headless tests; clippy clean. Licence GPL-3.0-or-later. Built on Bevy 0.20 and bevy_egui 0.43.1.

---

## What Ilex has

### Interface (§7, §15–18, §39)
- **Layout:**
  - one header row: menus, Undo/Redo, Q W E R, coordinate system, pivot, snaps, a `»` overflow menu, and Materials / MakeHuman / Setup / Render buttons;
  - 4-way or maximised rounded viewports with pill labels;
  - a right-hand Properties column: the Scene Explorer on top, and vertical tabs below (Create, Modify, Hierarchy, Display, Utilities, Render, Environment, Demiurge, Characters);
  - a status bar with an X/Y/Z type-in.
- **Look:** the Graphite & Teal theme (others in Preferences), rounded cards, soft shadows, Fira Code, and right-click-to-reset spinners everywhere.
- **Every feature has a button**, enforced by a test; hotkeys are shortcuts. There is an optional ribbon and a quad menu on right-click.
- **Windows:** an Orientation Cube in each viewport. Dialogs pop out as real OS windows (Wayland app ids for niri). Floating caddies for Edit Poly operations.
- **Preferences** (theme, keymap, viewport, render, layout), **Help** with screenshots and a guided tour, and **click-and-drag creation** of objects.
- **Crash safety:** a crash reporter, a black-box breadcrumb recorder and emergency scene recovery (§24).

### Viewport and transforms (§3)
- Pan, orbit and zoom; Zoom Extents (`Z`); 4-view (`Alt+W`); wireframe and edged faces (`F3` / `F4`); See-Through; grid.
- Ilex's own Move, Rotate and Scale gizmos (arrows and plane squares; rings, view ring and trackball; axes, plane strips and uniform triangle; yellow active handle), plus Shift-drag clone (Copy / Instance / Reference) (§79).
- Snaps: grid, pivot, vertex, edge, midpoint, face and angle. Selection regions: rectangle, circle, fence and lasso.
- Align dialog, Mirror, Array dialog (1D / 2D / 3D with preview), undo and redo for everything.

### Modelling
- **Primitives:** Box, Sphere, Cylinder, Plane, Cone and Torus with live parameters. Mech parts: Gear, Hydraulic Piston, Hose.
- **Editable Poly / Edit Poly, sub-object modes 1–5:**
  - extrude, chamfer, bevel, inset, outline, connect, bridge, weld and target weld, break, remove, insert vertex;
  - Cut, Slice Plane, QuickSlice, cap, hinge, extrude along spline, create shape, attach and detach;
  - Make Planar X/Y/Z, MSmooth and tessellate, Material IDs, 32 smoothing groups;
  - soft selection, ring and loop, Repeat Last;
  - Panel Groove.
- **Modifier stack:** Edit Poly, FFD 2/3/4, UVW Map, Unwrap UVW (UV editor + Smart UV), Symmetry, Shell, TurboSmooth, Chamfer, Bend / Twist / Taper / Push, Edit Normals, Boolean, ProOptimizer, Greeble, GeoGen, Lathe, Spline Extrude, VertexPaint. Also Show End Result, Pin Stack and Collapse.
- **Splines:** Line, Circle, Arc, NGon, Rectangle, Ellipse, Star, Helix. Vertex / segment / spline editing, fillet, chamfer, outline, refine, spline booleans, and renderable thickness.
- **SolidCraft CAD tools** (§23).
- **File I/O:** `.ilex` / `.mmax` scenes, and OBJ, glTF/GLB and FBX import and export.

### Materials and painting (§5D, §19–21)
- Compact Material Editor (`M`): 24 slots, Standard/PBR, Toon, Crystal/Glass, Unlit and Volume shading, quick presets, and an ink outline on every material.
- Multi/Sub-Object materials, a Material Library of 281 presets, and procedural pattern maps.
- MaterialCraft: a procedural node engine, a Pattern & Weathering rollout, a layer painter, a mesh map baker, damage stamps, Triplanar mapping and an asset downloader.
- Brush Strokes (Painted Normals): a material checkbox that repaints the lighting as brush marks (Gouache / Oil / Concept Art looks).
- Viewport painting: VertexPaint (no UVs needed) and texture projection paint with auto-setup.

### Rendering (§10F, §22, §25–27, §56–64)
- **Render Setup (`F10`) and Rendered Frame Window (`F9`):** channels, region, clone window, PNG / EXR / JPG output.
- **Native path tracer on the CPU or the GPU (wgpu):**
  - MIS, adaptive sampling, progressive rendering and a denoiser;
  - the GPU matches the CPU for sky, HDRI, maps, Principled lobes, godrays, mist, area lights, IES, prefab materials, LightMix, see-through surfaces, the shadow catcher, toon shading and volumes.
- **Principled BSDF:** subsurface scattering, anisotropy, clearcoat, sheen, absorption and dispersion. Area lights (rect, disc, sphere, tube) and IES profiles.
- **Luxon:** LightMix, À-Trous denoiser, AOV passes, lens effects, NPR ink, shadow catcher, colour grading and camera RAW.
- **Lighting:** a default headlight rig, Omni, Spot, Directional and Skylight, light parameters, studio rig presets, and the Environment dialog (`8`).
- **Turntable / sprite sheet / simulation-frame sequences** saved as PNGs.

### Worlds and effects: Demiurge (§12–13, §28–30, §65–78)
- **Terrain** with erosion, a **Gerstner Ocean** with foam, and **Planet Studio**.
- **Sky:** a dynamic sky with time of day, deep space, weather presets, storm darkening and precipitation.
- **Settlements:** four eras, the Medieval Architect, and Battlefield & Ruin destruction (5 damage states, rubble, blast zones).
- **Volumes:**
  - **clouds** as true volumes: Cumulus, Congestus, Cumulonimbus, Altocumulus, Mammatus, Lenticular, Cloud Bank and Cloud Field, with a viewport preview;
  - **trails** (contrails, vortices, afterburner, vapor cone, damage smoke, missile);
  - **explosions** and the mushroom cloud;
  - **smoke plumes**;
  - **VDB import**;
  - **Smoke & Fire Simulation**, solved on the GPU or the CPU and baked in the background.
- **GeoGen** rocks, crystals and stalagmites, and **Greeble**.

### Prefab Library (§42–55)
- A library browser with thumbnails, place mode, Detail (Standard / Mid) and a Destruction slider.
- **Buildings:** residential, commercial and office, industrial and utilities, services and transport, parks and leisure, garden and lot details, rural and farm, plus the medieval set.
- **Furniture, kitchen & bath, lighting & decor, appliances & electronics, room shells.**
- **Road cars:** the first vehicle wave, H4a.

### Characters (§31, §33–36, §40–41)
- **MakeHuman Character & Creature Studio:** body and face sliders, an anime ratio, CC0 hair, clothes, face kit and skins, hair colour, and imports.
- **Rig and Easy Pose.**
- **Face Cloner:** builds a MakeHuman head from photos or video frames, with a photo skin texture and saved presets.

### Mechs (§10, §37–38)
- Bone-free assembly: joints, FK and IK drag, Contact Sets, Motion Links, Mirror Mech Limb.
- Spider Platform base and Railgun prefab. Your own meshes can be rigged locally; they stay private and are never committed.

### Packaging (§14)
- A pacman package and a local repository. Installs `ilex`, the desktop entry, the icon and file types.

---

## What's planned

In rough order. Each piece is committed and pushed as it's finished, with tests, help and a PROJECT_STATUS section.

### 1. After the weekly reset (Holly picks which goes first)
- **Animation & Timeline** — plan: https://claude.ai/artifact/2oVatNyZTTXMx3m1aqkkFS. Roughly 3–4 days in total; working animation after the first day.
  1. **T1:** keys, tracks and interpolation, saved in the scene, with undo.
  2. **T2:** time slider, track bar, playback, Auto Key / Set Key and Time Configuration.
  3. **T3:** render the timeline to image sequences; simulations follow the timeline.
  4. **T4:** keys on modifier, primitive, material, light and environment settings.
  5. **T5:** Curve Editor and Dope Sheet.
  6. **T6:** animated mech joints and IK, for walk cycles.
- **Liquids:** the next part of the Clouds / Smoke / Pyro plan (https://claude.ai/artifact/XUnpQpB8qDCb5bTCPGQ51x).

### 2. Prefab Library, remaining waves (vehicles H4 paused until Holly says go)
- **H4b:** bus, lorry, tractor, forklift, motorbike, bicycle, scooter.
- **H4c:** train carriage, locomotive, boat, small plane, helicopter.
- **Bridges (about 10):** beam, suspension, cable-stayed, covered wooden, rope, viaduct, footbridge, bascule, pontoon, stepping stones.
- **Outdoor and street (about 20):** trees, bushes, fences and walls along a spline, traffic lights, signs, hydrant, bins, picnic table, parasol, market stalls.
- **The "Lego" workflow:** snap sockets, one-click Sets (living room, kitchen, office, street corner …), Group / Ungroup, My Prefabs.
- **Smaller follow-ups:** a cleaner furniture material pack; ceiling options and floor plans for rooms.

### 3. Known gaps worth closing (from the "Not done yet" lists)
- **Simulations:**
  - collisions and mesh emitters;
  - saving bakes with the scene;
  - keeping GPU data between frames;
  - animated VDB sequences.
- **Rendering:**
  - AOV passes and the À-Trous denoiser on the GPU;
  - some live map combinations still use the CPU;
  - video output (MP4/WebM) and motion blur (with animation);
  - timing on Holly's RX 9060 XT and Ryzen 7 5700X, since everything here ran on a software GPU.
- **Volumes and effects:**
  - trails that follow a moving object;
  - an explosion light in the viewport;
  - Mammatus under an existing anvil;
  - terrain-following lenticular clouds;
  - fire-base light for plumes;
  - volumetric plumes in auto-spawned settlements.
- **Clouds:**
  - tuning lighting contrast;
  - fixing gaps in tall towers;
  - a 2D coverage map for Cloud Fields.

### 4. Ideas kept for later (not scheduled)
- Character poses and a CC0 animation library (Mesh2Motion), retargeted onto the MakeHuman skeleton.
- Path and look-at constraints; animating vertices with Edit Poly.
- Herringbone and planetary gears, catenary cables, SVG path import, and 3D text from font files.
- A Geometry Graph (node-based modelling built on Ilex's own operations).
