# MiniMax (Holly-3D): Project Status Report

_Status as of 10 November 2026, measured against the spec in `CLAUDE.md`._

## At a glance

| | |
|---|---|
| Build phases | **All done and merged to `main`**: Phases 1–5 from `CLAUDE.md` (PRs #1–#3), Phase 6 (gap closure and hardware optimisation) and Phase 7 (mech assembly) (PRs #4–#5), **Phase 8A** (Multi/Sub-Object materials, renderer overhaul, dynamic sky; PR #6, tested by hand in the running app), **Phase 8B** (§11 polish, Terrain, Gerstner Ocean, Planet Studio; PR #7, status update in PR #8, unit-tested, not yet tested by hand), and **Phase 8C** (GhostTown settlements, Medieval Architect, Battlefield & Ruin; PR #9, unit-tested, not yet tested by hand) |
| Source | 287 `.rs` files, about 44,100 lines; the largest is 347 lines (limit 350) |
| Tests | 320 headless tests, all passing (1 ignored: needs a real Blender) (`cargo test --lib`) |
| Lint | `cargo clippy --all-targets` reports no warnings |
| Packaging | `packaging/`: a pacman PKGBUILD (version `0.<commits>.g<hash>`, `znver3`) that installs the binary, desktop entry, icon and `.mmax` file type, plus `publish-repo.sh` for a local `[minimax]` repository (see README) |
| Stack | Bevy 0.20, bevy_egui 0.43.1, transform-gizmo-egui, slotmap, boolmesh, meshopt, uvunwrap, noise, rand/rand_chacha, image |

**Phases 6 and 7 merged in PR #4.** **Phase 6 done:** all §9 gaps are closed (✅ marks below), plus copy-on-write undo, a tiled renderer and a SIMD BVH. **Phase 7 done:** the bone-free mech assembly system, mech primitives and Panel Groove (§10). File I/O is done: `.mmax` scenes plus OBJ, glTF/GLB and FBX import and export. **Phase 8A done** (§11): Multi/Sub-Object materials with emissive area lights, AO / 1-bounce GI / denoiser / softbox / bloom / DOF / live tonemapping, and a time-of-day sky with clouds plus a deep-space mode. Show End Result now works. **Phase 8B done** (§12): the four §11 issues are fixed, plus procedural worlds under `Create -> Geometry -> Worlds`: biome Terrain with erosion and slope / altitude Material IDs, a Gerstner Ocean with crest foam, and the layered Planet Studio. **Phase 8C done** (§13): GhostTown settlements in four eras, the scale-aware Medieval Architect, and period-accurate Battlefield & Ruin destruction with movable Blast Zones.

Legend: ✅ done · 🟡 partly done · ❌ not done

---

## 1. Core rules (§1)

| Rule | Status | Notes |
|---|---|---|
| 3ds Max conventions only | ✅ | Q/W/E/R, 1–5, M, F9, F10, Shift+Drag clone; no Blender-style modal keys |
| Clean five-zone UI | ✅ | Slate `#222327` with amber `#E59C24` |
| No `.rs` file over 350 lines | ✅ | Largest: `src/mesh/ops/chamfer.rs` at 347 lines |
| No `Rc<RefCell>` / `Arc<Mutex>` in topology | ✅ | `slotmap` arenas |
| Headless tests for every mesh operation and modifier | ✅ | Counts plus normal and UV validity |

## 2. Tech stack (§2)

| Crate | Status |
|---|---|
| bevy 0.20, bevy_egui 0.43.1 | ✅ |
| transform-gizmo-egui | ✅ |
| slotmap, glam | ✅ |
| noise, rand, rand_chacha | ✅ |
| boolmesh 0.2.2 | ✅ |
| meshopt (meshoptimizer) | ✅ |
| uvunwrap 0.2.0 | ✅ |
| image (PNG, JPEG, EXR, HDR) | ✅ |
| `gltf`, `ufbx`, OBJ (own reader/writer) | ✅ Import and export of OBJ+MTL, glTF/GLB and FBX |
| `serde`, `ron`, `rfd` | ✅ `.mmax` scene files and native file dialogs |

## 3. Controls and hotkeys (§3)

| Control | Status | Notes |
|---|---|---|
| MMB pan, Alt+MMB orbit, wheel or Ctrl+Alt+MMB zoom | ✅ | Orbits around the selection |
| `Z` zoom extents selected (`Shift+Z` for all views) | ✅ | Works in sub-object modes |
| `Alt+W`, `G`, `F3`, `F4`, `Alt+X` | ✅ | |
| `Alt+Q` isolate | ✅ | |
| `Q/W/E/R` and the gizmo | ✅ | |
| `Q` cycles selection-region shapes | ✅ | Rectangle, Circle, Fence, Lasso; the Q button shows the current shape (right-click to pick one) |
| Shift+Drag clone (Copy / Instance / Reference, number of copies) | ✅ | |
| Shift+Drag border edges to extrude | ✅ | |
| `S` snap / `A` angle snap | ✅ | 3D Snaps: Grid Points, Pivot, Vertex, Edge/Segment, Midpoint, Face Center; right-click `S` for settings; cyan crosshair at the snap point. Angle snap 5° |
| `Ctrl+Z` / `Ctrl+Y` | ✅ | Covers transforms, mesh, modifier, material and paint edits |
| `Delete` | ✅ | Objects or sub-objects |
| `M`, `F9`, `F10`, `8`, `Ctrl+L` | ✅ | |
| `1–5` sub-object modes; pressing the same key again exits | ✅ | |
| `Alt+R` ring, `Alt+L` loop, double-click for loop | ✅ | |
| `H` toggles the Scene Explorer | ✅ | |

## 4. Mesh and scene architecture (§4)

| Item | Status |
|---|---|
| Face-corner `EditablePoly` (Vertex, FaceCorner, Polygon, Edge) with the spec's fields | ✅ |
| Base objects: primitives, splines, GeoGen, EditablePoly | ✅ |
| Modifier stack: enable toggles, cached evaluation, rebuild only when dirty | ✅ |
| Instances and references (shared base object, Make Unique) | ✅ |

---

## 5. Priority 1 features (§5)

### A. Editable Poly / Edit Poly modifier

| Rollout | Done | Not done |
|---|---|---|
| Selection | ✅ Five mode buttons, By Vertex, Ignore Backfacing, Grow/Shrink, Ring/Loop with ◀ ▶ Ring/Loop shift arrows | |
| Soft Selection | ✅ Falloff, Pinch, Bubble, rainbow heat colours | |
| Edit Vertices | ✅ Remove, Break, Extrude ☐, Weld ☐, Chamfer ☐, Target Weld, Connect, Remove Isolated Vertices, Remove Unused Map Verts | |
| Edit Edges / Borders | ✅ Insert Vertex, Remove (Ctrl also removes orphaned vertices), Split, Extrude, Weld, Target Weld, Chamfer ☐ (amount, segments, tension, open), Connect ☐ (segments, pinch, slide), Bridge, Cap, Create Shape from Selection (Smooth / Linear) | |
| Edit Polygons / Elements | ✅ Insert Vertex, Extrude ☐, Outline ☐, Bevel ☐, Inset ☐, Bridge, Flip, Hinge From Edge ☐ (angle, segments), Extrude Along Spline ☐ (segments, taper, twist, align) | |
| Edit Geometry | ✅ Repeat Last, Collapse, Attach/Detach, Slice Plane + Slice + Split + Reset Plane, QuickSlice, Cut, MSmooth, Tessellate ☐, Make Planar + X/Y/Z, Create (click existing vertices or empty space) | |
| Material IDs and Smoothing Groups | ✅ Set ID / Select ID, the 4×8 grid of 32 groups, Clear All, Auto Smooth (45°) | |
| Floating caddies (☐) | ✅ Live preview with OK, Apply & Continue, Cancel | |

### B. FFD modifiers
✅ FFD 2×2×2, 3×3×3 and 4×4×4 with Control Points and Lattice sub-objects, box selection plus W/E/R, Conform to Shape, Reset, All X/Y/Z.

### C. UVW Map and Unwrap UVW
- ✅ **UVW Map:**
  - All seven mapping types, including Cylindrical Cap.
  - Length, width, height, U/V/W tile and flip settings, and the X/Y/Z alignment.
  - Fit, Center, Normal Align, View Align and Reset.
  - A gizmo sub-object.
- ✅ **Bitmap Fit:** pick an image; the gizmo keeps its width and takes the image's aspect ratio.
- ✅ **Unwrap UVW:**
  - A floating UV editor with vertex, edge and polygon selection.
  - Move, rotate, scale, Relax and Stitch.
  - A checker pattern in the viewport.
  - Smart UV Unwrap (Angle Limit, Island Margin, Correct Aspect).

### D. Materials (`M`)
✅ All of it:
- Compact Material Editor with 24 sample slots and the toolbar: Assign, Show in Viewport, Reset, Copy, Eyedropper.
- Four shading models and all seven presets.
- The Universal Ink Outline on every material (width, colour, crease lines).
- The Maps rollout and Import PBR Texture Folder.
- A Slate node view with Noise, Checker, Gradient Ramp and Color Correction nodes.

### E. Direct 3D painting
✅ All of it:
- VertexPaint modifier with the PaintBox floater: size, opacity, falloff, colour, all or selected faces only, Fill Object, Blur, Layer Opacity.
- Texture projection paint at 2048 or 4096, with Auto-Setup Paint Canvas.

### F. Rendering (`F9` / `F10`)
- ✅ **Render Setup:**
  - Resolution presets, aspect lock, transparent background.
  - Choice of the built-in raytracer (32x32 parallel tiles, 4-wide SIMD BVH, soft shadows, reflection and refraction, anti-aliasing, tonemapping, outline pass) or the Cycles bridge. By default the bridge runs `blender --background -E CYCLES` with a Python loader; it can also write Cycles XML for the standalone `cycles` tool.
- ✅ **Rendered Frame Window:**
  - Render, a viewport/camera picker, Region.
  - Save as PNG, EXR or JPG; Copy to Clipboard; Clone Render Window.
  - R, G, B, A and Mono channel buttons.
- ✅ **Cameras:** Target and Free Camera objects; Render Setup and the Frame Window list them next to the viewports.
- ✅ Cycles bridge, verified against a real Blender 4.0 install. F10 defaults to Blender at `/usr/bin/blender`, the HIP device with HIP-RT, and OpenImageDenoise. A missing GPU device falls back to the CPU, and a Blender build without OIDN renders undenoised.

### G. Lighting
- ✅ The default two-light headlight rig, with `Ctrl+L` and automatic switching to scene lights.
- ✅ Light types: Omni, Target Spot, Free Spot, Target Directional, Free Directional, Skylight.
- ✅ Light Parameters: on/off, type, shadows, multiplier, colour plus Kelvin presets, decay/range, Show Attenuation, hotspot and falloff, shadow bias, softness and colour.
- ✅ Hotspot and falloff cone gizmos.
- ✅ Near and far attenuation (Use Near / Near Start / Near End plus the far range), each drawn as a pair of wire spheres when Show Attenuation is on.
- ✅ The Environment dialog (`8`): five lighting rig presets, background colour, ambient tint, exponential fog.
- ✅ **HDRI environment map** (`.hdr` / `.exr`, equirectangular): Strength, Rotation, Use for Lighting, Show in Perspective Viewports. It is the F9 background and reflection source, the Cycles world texture, and a skybox in perspective viewports.

---

## 6. Priority 2 features (§6)

### A. Splines, Lathe and Extrude
- ✅ **Shapes:**
  - Line: click for corner points, drag for Bezier handles, right-click to finish, offer to close.
  - Circle, Arc, NGon, Rectangle, Ellipse, Star, Helix.
- ✅ **Rendering rollout:** Enable In Renderer and Enable In Viewport, with Radial or Rectangular profiles.
- ✅ **Vertex level:** Bezier Corner, Bezier, Corner and Smooth types; Fillet, Chamfer, Weld, Make First, Delete.
- ✅ **Segment level:** Divide, Delete.
- ✅ **Spline level:** Outline, Mirror (with Copy), Reverse, Close, Delete.
- ✅ Convert to Editable Spline.
- ✅ **Refine:** click on a segment at Vertex level to insert a knot there (the curve is kept).
- ✅ **Spline Boolean:** Union, Subtraction and Intersection at Spline level; select one closed spline, then click the other.
- ✅ **Lathe:** degrees, segments, Weld Core, Flip Normals, Cap Start/End, X/Y/Z direction, Min/Center/Max alignment, axis sub-object.
- ✅ **Spline Extrude:** amount, segments, Cap Start/End.

### B. Array dialog
✅ Incremental and Totals sections for move, rotate and scale, with Re-Orient and Uniform; Copy, Instance or Reference; 1D, 2D and 3D; live Preview. A **Mirror** dialog was also added.

### C. Procedural generators and other modifiers

| Modifier | Status | Notes |
|---|---|---|
| GeoGen (crystal, rock, cliff, stalagmite) | ✅ | Works as an object and as a modifier; all spec parameters |
| Greeble | ✅ | Panels and widgets rollouts, Select Tops, separate material IDs |
| Chamfer modifier | ✅ | All, Selected, Smoothing-Group or Angle filters |
| Symmetry | ✅ | All parameters work. The **Mirror** sub-object (`1`) moves and rotates the plane with W / E |
| Shell | ✅ | Straighten Corners, Override Edge Material ID, select inner/outer/edge faces |
| TurboSmooth | ✅ | Iterations 0–4, Isoline Display, Separate by Smoothing Groups or Materials |
| Bend, Twist, Taper, Push | ✅ | All parameters and limits work. **Gizmo** (`1`, W/E/R) and **Center** (`2`, W) sub-objects, Reset Gizmo |
| Edit Normals | ✅ | Cyan, blue and green lines; select by Normal, Vertex, Edge or Face; Break, Unify, Reset, Make Explicit; `E` rotates the selected normals (tried in the app, and tested on a rotated object) |
| Boolean (boolmesh) | ✅ | Union, A−B, B−A, Intersect; Pick Operand B; an operand list with Remove, Extract and on/off toggles; Merge Coplanar Faces. Open meshes are skipped with a warning |
| ProOptimizer (meshopt) | ✅ | Calculate, linked Vertex % and Vertex Count, keeps material, UV, colour and normal boundaries, Protect Borders |
| Terrain, Ocean (Phase 8B) | ✅ | Objects under `Create -> Geometry -> Worlds` and stack modifiers; see §12. Planet Studio is an object only |
| GhostTown, Medieval Architect, Battlefield / Ruin (Phase 8C) | ✅ | Settlement objects under `Create -> Geometry -> Settlement` and stack modifiers; see §13 |

---

## 7. UI and visual design (§7)

| Item | Status | Notes |
|---|---|---|
| Theme colours, spacing, corner radius | ✅ | |
| Right-click resets any spinner (`max_spinner`) | ✅ | |
| Top toolbar: File, Edit, Tools menus | ✅ | File: New, Open (`Ctrl+O`), Save (`Ctrl+S`), Save As (`Ctrl+Shift+S`), Import, Export All / Selected all work. Tools has Array, Mirror, Align (`Alt+A`) and Isolate |
| Toolbar centre: Q/W/E/R, View/World/Local, the three pivot modes, snaps, Mirror, Array, Paint | ✅ | |
| Toolbar right: Material Editor, Render Setup, Quick Render | ✅ | |
| Scene Explorer: visibility, freeze, type icons, search filter | ✅ | |
| Viewports: `[+] [Perspective] [Shading]` labels, gold border on the active view, axis tripod | ✅ | |
| Command panel: Create, Modify and Pivot tabs | ✅ | |
| Create tab: geometry, GeoGen, shapes, lights, cameras | ✅ | Target and Free Camera |
| Modify tab: name and wireframe colour, searchable modifier list, stack with sub-object expanders | ✅ | Every modifier in the list is available |
| Stack action row | ✅ | Make Unique, Delete and Collapse work. **Pin Stack** keeps the Modify panel on the pinned object whatever is selected. **Show End Result** toggles between the whole stack and the result up to the current modifier |
| Pivot tab: Affect Pivot Only, Center to Object, Align to World, Bottom-Center, Reset XForm | ✅ | |
| Status bar: mesh statistics, Abs/Offset X/Y/Z type-in | ✅ | |
| Quad menu: Tools 1, Transform, Display, Convert To | ✅ | |

---

## 8. Phase history

| Phase | Contents | PR |
|---|---|---|
| 1 | Viewports, camera, theme, five-zone layout, gizmos, clone dialog, primitives | #1 ✅ merged |
| 2 | Editable Poly, the full Edit Poly toolkit, caddies, quad menu | #1 ✅ merged |
| 3 | FFD, UVW Map, Unwrap UVW, UV editor, Smart UV | #2 ✅ merged |
| 4 | Materials, painting, lighting, Render Setup, Frame Window, raytracer, Cycles bridge | #3 ✅ merged |
| 5 | Splines, Lathe, Extrude, Array, Mirror, GeoGen, Greeble, Chamfer, Symmetry, Shell, TurboSmooth, deformers, Edit Normals, Boolean, ProOptimizer | #3 ✅ merged |
| 6 | File I/O (`.mmax`, OBJ, glTF/GLB, FBX), cameras, Align, Edit Poly gap tools, modifier gizmos, Spline Boolean / Refine, 3D Snaps, selection regions, HDRI, Cycles through Blender, copy-on-write undo, tiled renderer, SIMD BVH (§9) | #4 ✅ merged |
| 7 | Bone-free mech assembly: joints and the J tool, FK / IK drag, Mirror Mech Limb, Contact Sets, Motion Links, Gear / Piston / Hose primitives, Panel Groove (§10) | #4 ✅ merged |
| 8A | Show End Result, Multi/Sub-Object materials, emissive area lights, GI & Post-FX, dynamic sky / clouds / deep space (§11) | #6 ✅ merged |
| 8B | §11 polish, procedural Terrain (6 biomes, erosion, auto Material IDs), Gerstner Ocean with crest foam, Planet Studio (surface, clouds, atmosphere, rings) (§12) | #7 ✅ merged |
| 8C | GhostTown settlements (4 eras), Medieval Architect (5 presets, scale-aware), Battlefield & Ruin (3 period-accurate modes, Blast Zones) (§13) | #9 ✅ merged |

---

## 9. Phase 6: remaining work, by impact (all done)

1. ✅ **File input and output.** `src/io/` + `src/editor/file.rs` + `src/ui/file_menu.rs`.
   - Native `.mmax` scene (RON): objects with their base objects and full modifier stacks, transforms and pivots, lights, materials including painted textures (embedded PNG), environment and name counters. Caches are rebuilt on load.
   - Import / Export (All or Selected): OBJ + MTL, glTF / GLB and ASCII FBX 7.4 (FBX import reads binary and ASCII through `ufbx`). Quads and n-gons, smoothing groups 1..32 (FBX masks read straight from the file), UVs, vertex colours and material IDs survive round trips.
   - A scene or mesh path given on the command line is opened at start-up.
   - Tests: round trips for every format, editor export-selected → import, save → open.
2. ✅ **Target Camera and Free Camera.** `src/scene/camera.rs`, `src/editor/cameras.rs`, `src/ui/command/camera_params.rs`, `src/viewport/camera_icons.rs`.
   - Create -> Cameras: Target Camera takes two clicks (camera, then target); Free Camera takes one and looks the way the viewport looks. Right-click a button to create one at a default spot.
   - Parameters: Lens (mm) and FOV linked on a 36 mm film back, stock lenses 15–200 mm, Show Cone, Type (Target / Free, adds or removes the target), Target Distance, Clip Manually with Near / Far Clip.
   - Viewport: camera icon, target line, view cone and clip planes when selected.
   - `C` looks through the selected camera (or cycles through the cameras); the `[ View ]` label menu has a Cameras list. Navigating a camera view moves the camera, and Alt+MMB orbits around its target.
   - Render Setup and the Frame Window list the cameras, and F9 renders through them. The raytracer and Cycles honour the lens; near clip applies in the raytracer and viewports. In the viewports, the far clip only culls whole objects, because Bevy's projection is infinite.
   - Tests: lens/FOV maths, target aiming, deleting the target, looking straight down, look-through, orbit write-back, Target <-> Free, and undoing a lens change.
3. ✅ **Align Selection dialog** (Tools → Align, `Alt+A`). `src/editor/align.rs`, `src/ui/align_dialog.rs`.
   - Pick the target object with a click (Esc cancels). Targets of lights and cameras are never moved.
   - Align Position (World): X / Y / Z, with Minimum / Center / Pivot Point / Maximum for the current and the target object (world bounding boxes).
   - Align Orientation (Local) X / Y / Z: one axis turns that local axis onto the target's; two or more copy the target's rotation. Match Scale X / Y / Z.
   - Live preview in every viewport. Apply keeps the result and starts a fresh alignment; OK / Enter confirms; Cancel / Esc restores the originals. Each step is one undo step.
   - Tests: Min→Max on X only, Center→Center, undo, orientation, scale, cancel, and picking the selection itself.
4. ✅ **Edit Poly tools.** `src/mesh/ops/sweep.rs`, `src/mesh/selection_shift.rs`, `src/editor/poly_tools.rs`.
   - Create: click existing vertices or empty space (snapped); click the first point or right-click to build the polygon, wound to face the viewer.
   - Create Shape from Selection: selected edges are joined into chains and become a new Editable Spline (Smooth or Linear knots).
   - Hinge From Edge ☐: pick the hinge edge; Angle and Segments. The region always swings out along its normal and the hinge edge stays shared.
   - Extrude Along Spline ☐: pick a shape; Segments, Taper Amount, Twist and Align to Face Normal, with parallel-transported frames along the path.
   - Ring / Loop shift: ◀ ▶ arrows next to Ring and Loop move every selected edge one step along its ring or loop.
   - Tests: hinge counts (1 and 3 segments), straight / bent / tapered sweeps, Create from existing and new points, edge chains (open and closed), ring and loop shift on a cylinder.
5. ✅ **Modifier gizmos.** `src/modifiers/deformers.rs`, `src/modifiers/symmetry.rs`, `src/editor/modsub.rs`, `src/viewport/mod_gizmos.rs`.
   - Bend / Twist / Taper: a Gizmo frame (move / rotate / scale with W/E/R) and a Center inside it (move with W). The gizmo box is fitted to the input and travels with the gizmo (orange while active, center cross). Reset Gizmo puts both back on the pivot.
   - Symmetry: a Mirror sub-object; W moves and E rotates the mirror plane (kept rigid), with slicing and seam welding on any tilt.
   - Older scenes load unchanged (new fields default to identity).
   - Tests: moving the gizmo equals moving the center, Twist is unchanged by turning its gizmo about the axis, tilting Bend changes the result, the Mirror move equals Mirror Offset, a rotated Mirror equals another axis, a tilted plane welds closed, and editor drags for Gizmo, Center and Mirror.
6. ✅ **Spline Boolean and Refine.** `src/splines/boolean.rs`, `src/editor/spline_tools.rs`, `src/ui/rollouts/spline_tools.rs`.
   - Refine (Vertex level): click a segment to insert a knot at that point; the Bezier shape is kept (de Casteljau split). Undoable.
   - Boolean (Spline level): pick Union / Subtraction / Intersection, select a closed spline, then click the other. Both are cut at their crossings without losing curvature, each segment is kept or dropped by side, and the kept pieces are joined into closed splines (holes come out as separate splines). Open splines are refused unchanged.
   - Tests: square union / subtraction / intersection areas and knot counts (either operand order, either winding), a circle lens within 2% of the exact area, disjoint and nested shapes, open splines refused, and the editor click flow with undo.
7. ✅ **3D Snaps.** `src/editor/snap.rs`, `src/ui/snaps.rs`.
   - Snap types: Grid Points, Pivot, Vertex (mesh vertices and spline knots), Edge/Segment (perspective-correct nearest point), Midpoint and Face Center. Point snaps win over edges, edges over the grid.
   - Right-click the `S` button for Grid and Snap Settings: the six types, Grid Spacing (also the Move gizmo increment), Snap Strength in pixels, Clear All / Defaults.
   - A cyan crosshair with the snap type follows the cursor while `S` is on.
   - Used by every creation click (primitives, shapes, Line, lights, cameras, Create polygon) and by the Move gizmo, which drops the pivot or sub-object center onto the snap point. The selection being moved is ignored, so it never snaps to itself.
   - Tests: vertex, midpoint, edge, face-center, pivot and grid hits on a box in perspective; nothing with `S` off; a selected object is skipped while moving but snapped to by creation tools.
8. ✅ **Selection-region shapes.** `src/viewport/region.rs`, `src/ui/viewport_ui/region.rs`, `src/ui/toolbar.rs`.
   - `Q` switches to Select Object; pressed again in Select mode it cycles Rectangular → Circular → Fence → Lasso. The Q button draws the current shape and a right-click menu picks one.
   - Rectangle drags corner to corner, Circle drags from the center, Lasso drags a free-hand outline. Fence drags its first side, then each click adds a corner; click the first corner, double-click or right-click to close it.
   - Every window selection uses the region: objects, Edit Poly sub-objects, spline vertices / segments / splines, FFD control points and Edit Normals.
   - Tests: shape cycling, containment for each shape (an L-shaped lasso excludes its notch, degenerate regions are refused), and Circle / Fence / Lasso selecting the right boxes in a Top view.
9. ✅ **Smaller items.**
   - HDRI environment map: `src/render/hdri.rs`, `src/viewport/skybox.rs`, the `8` dialog. The image path is saved with the scene and the image reloaded on open. In F9, camera misses and reflections sample it, and Use for Lighting adds its solid-angle average to the ambient. Perspective viewports get a cubemap skybox resampled from the equirect image.
   - Near attenuation: Use Near / Near Start / Near End on Omni and Spot lights. The raytracer ramps light up from Near Start to Near End and down over the far range. Show Attenuation draws both pairs of spheres.
   - Bitmap Fit in UVW Map; Pin Stack in the stack action row.
   - Tests: equirect sampling, rotation, average and cube faces; HDRI background and image-based ambient in F9; HDRI save / open round trip; near and far attenuation ramps; Bitmap Fit aspect; Pin Stack.
10. ✅ **Checks and the Cycles bridge.** `src/render/blender.rs`, `src/render/blender_loader.py`, `src/render/cycles.rs`, `src/ui/render_setup.rs`.
   - Edit Normals `E` rotation tried in the app: the gizmo sits on the selected normals and the ring drag turns them. A test drives the same drag path on a rotated object (world X rotation, top normals end up at world -Y).
   - Cycles through Blender (the default): the scene is exported as JSON (one mesh per object and material, with corner normals; Principled materials; point, spot and sun lights; camera; color or HDRI world). A bundled Python loader rebuilds it in `blender --background --factory-startup -E CYCLES` and writes a PNG. A real Blender 4.0 render of a box under a spot light came back into the frame buffer correctly.
   - F10 Cycles options: Run Through (Blender / Standalone cycles), executable path, Device (HIP, OptiX, CUDA, oneAPI, Metal, CPU; default HIP), HIP-RT (default on), Denoise with OpenImageDenoise (default on), Samples, Max Bounces. The standalone route passes `--device` too.
   - Tests: defaults, including old scene files without the new fields; the JSON (triangle count, camera axes, spot light, HIP-RT only on HIP); the loader reads every exported key; a round trip with a stand-in `blender` script; readable errors for a missing executable. `real_blender_renders_the_scene` (ignored by default) renders through a real Blender.
11. ✅ **Hardware optimisation.** `src/scene/shared.rs`, `src/render/job.rs`, `src/render/bvh_wide.rs`.
   - Copy-on-write undo: Editable Poly base meshes and the cached evaluated / level meshes are held in `Shared<T>` (an `Arc` that copies on the first write). An undo checkpoint clones pointers, not meshes; an edit copies only the mesh it changes, and every untouched object stays shared with the snapshots.
   - F9 raytracer: worker threads (one per core) pull 32x32 tiles from an atomic counter and stream each finished tile to the Frame Window. Progress is counted in pixels.
   - 4-wide SIMD BVH: the median-split binary BVH is collapsed into nodes with four child boxes stored as SoA `Vec4`s, so one step tests four slabs at once (SSE / NEON through glam). The nearest children are visited first, and shadow rays stop at the first hit.
   - Tests: snapshots share every mesh and an edit copies only the edited one (undo still restores it); random triangle soups (1 to 2000 triangles, including axis-aligned rays) give the same hits through the wide BVH, the binary BVH and brute force; tiles cover any region exactly once.

---

## 10. Phase 7: mech assembly (bone-free)

1. ✅ **Joints and the J tool.** `src/assembly/` (frame, solve, mount), `src/editor/joints.rs`, `src/ui/command/assembly.rs`, `src/viewport/joint_gizmos.rs`.
   - `J` or `[Assemble Joint]` in the Pivot tab's Assembly rollout: click a mount on part A (the child), then on part B (the parent). A mount is a vertex, an edge (the center of its closed edge loop, e.g. a cylinder ring) or a face / cap center. A is snapped face to face onto B and linked by the chosen joint.
   - Joints: Rigid (welded), Revolute / Hinge (X/Y/Z axis, angle, min/max stops), Ball Socket (swing cone and twist limit), Slider / Rail (axis, travel, min/max), Hydraulic Piston (the two parts turn about their mounts to aim at each other; neither is moved).
   - Transforms stay in world space. Each frame the solver poses children from their parents, parents first. A direct move or turn of a child is projected onto the joint's free axes and clamped at its stops, so `W` / `E` on a jointed part only moves it the way the joint allows. Loops are refused, deleting a part drops its joints, and joints are saved with the scene (older scenes still load).
   - The rollout edits the selected part's joint type, axis, angle / travel, limits, cone and twist (one undo step per drag), and offers Select Parent and Break Joint. The viewport draws mounts, the hinge axis with its yellow limit arc and a cyan angle tick, the ball cone, the slider rail and the piston line.
   - Tests: welded child follows its parent; hinge projection and clamping, angle unwrap past 180°; ball cone and twist clamp; slider projection and stops; chain order, loop refusal and cleanup on delete; piston aim; face, cap and edge-loop mounts and the face-to-face mate; RON round trip; two viewport clicks build a joint, undo removes it, and parameter edits re-pose the child.
2. ✅ **FK, IK drag and Mirror Mech Limb.** `src/assembly/ik.rs`, `src/assembly/mirror.rs`, `src/editor/joints.rs`.
   - FK on `E`: a turn of a jointed part is kept only about its joint's free axes and clamped at the stops; everything it carries (and pistons) follows.
   - IK on `W`: when a part is moved further than its own joint allows, cyclic coordinate descent re-poses the hinges, ball sockets and sliders above it so its pivot reaches the dragged point, up to the root part or the first part marked **Grounded**. Stops are kept. `[✓] IK Drag` in the Assembly rollout turns it off.
   - `Mirror Mech Limb [X] [Y] [Z]`: copies the selected part and every part it carries across the plane through its parent's pivot (mirrored, negative-scale copies), and rebuilds their joints, with the copy hanging off the same parent. Hinge angles and stops come out mirrored, so equal and opposite angles keep the two limbs mirror images.
   - Tests: a hinge turns to reach a dragged point; a two-link arm bends to reach, stops at a Grounded part (closest reach), and is unchanged with IK off; IK keeps stops; a mirrored limb sits at the mirrored positions with negated angle and swapped stops, already satisfies its joints, stays mirrored under opposite angles, and undoes in one step.
3. ✅ **Contact Sets and Motion Links.** `src/assembly/contact.rs`, `src/assembly/links.rs`, `src/editor/mech.rs`.
   - `[Contact]` toolbar toggle and a `[✓] Contact Member` checkbox per part: overlapping members push each other along their own hinge or slider (the smallest change that clears, found by stepping and then bisecting, so parts end up touching). The dragged (selected) part pushes; a part that cannot give way (welded, or at a stop) blocks the dragged part instead. Parts jointed to each other are mated by design and never collide.
   - Motion Links in the Assembly rollout: `Driven by [part]` links the current part's hinge or slider to another's (driven = offset + Ratio × driver, the offset taken when linking so nothing jumps). `Ratio` spinner (right-click resets to 1) and `[Auto-Ratio from Gear Teeth / Radius]`: meshing gears turn opposite ways at the inverse ratio of their radii; a pinion drives a rack by arc length. Loops of links are refused, chains of links settle, and links to deleted parts are dropped.
   - Tests: gear pair (−r₁/r₂) and rack (arc length) follow their driver; loop and no-hinge refusals; a new ratio keeps the driven part in place; links drop with their driver; a slider member is pushed until touching; Contact Sets off lets parts pass; a welded member blocks the dragged one; rollout flow with undo.
4. ✅ **Mech primitives: Gear, Hydraulic Piston and Hose.** `src/mech/`, `src/editor/mech.rs`, `src/ui/rollouts/mech.rs`.
   - Create -> Geometry -> Mech: `[Gear]` `[Piston]` `[Hose]`. Click a type, then click in a viewport to place it; right-click a type to create at the origin. Every parameter stays live in the Modify panel, and Convert to Editable Poly collapses the part.
   - **Involute Gear:** Type (External Spur, Internal Ring or Linear Rack), Teeth 6..200, Module, Pressure Angle (20°), Thickness, Bore Radius and Helix Angle. True involute flanks from the base circle, addendum 1 m and dedendum 1.25 m; ring gears have their teeth inside a rim, racks have straight-sided teeth at the pressure angle. Gear pitch radii drive the Motion Link Auto-Ratio (a ring gear turns the same way as its pinion).
   - **Hydraulic Piston:** a housing (barrel with a gland) and a rod, each with an eyelet at its pivot, created in line and joined by a Hydraulic Piston joint so they keep aiming at each other. Radius, Length, Eyelet Radius and Sides per part.
   - **Hose / Mecha Cable:** Bezier or Catmull-Rom path with Tension, Slack, Gravity Sag, Diameter, Sides and Segments, plus Corrugated Rings (Ring Count, Ring Depth, Round or Square). Free ends have Start / End spinners. `Pick Top / Bottom End Object` binds an end to a part: the hose leaves the top object down its −Z axis, enters the bottom object up its +Z, and flexes live as either part moves. `[Free]` unbinds an end; deleting an end object leaves that end where it was.
   - Tests: spur gear vertex / edge / face counts, closed and outward, addendum and dedendum radii, bore radius (genus 1); ring teeth point inward; rack length is teeth × circular pitch, and the helix twists the top face; piston parts; hose sag, end tangents, square rings and watertight caps; the Create tool builds a piston as two parts and a joint (one undo step); a bound hose follows its end objects and is freed when one is deleted.
5. ✅ **Panel Groove (Panel Line / Seam Groove).** `src/mesh/ops/groove.rs`, `src/modifiers/panel_groove.rs`.
   - Edit Poly, Edge mode: `[Panel Groove] [□]` with a caddy for Groove Width, Groove Depth, Bevel Lip Width and Inner Groove Material ID (recorded in the Edit Poly modifier and available to Repeat Last).
   - `Panel Groove` modifier with the same parameters plus the Chamfer modifier's Selection Filter (Selected Edges by default, so an Edit Poly below picks the seams; or All Edges, Smoothing Group Boundaries, By Angle).
   - The edges are chamfered into a strip `width + 2 × lip` wide, a 45° bevel lip steps down and in, then the channel floor sinks to the full depth along mitered normals, so the depth stays exact across hard edges and box corners. Walls and floor take the inner material ID and are faceted; the surface around keeps its material; the incoming polygon selection flows up the stack.
   - Tests: a groove line across a plane (counts, depth, floor width, material IDs); a loop around a box stays watertight with the floor at height − depth; all 12 cube edges through the modifier (counts, closed, Euler 2, material and smoothing); no-op cases (nothing selected, zero width).

## 11. Phase 8A: Multi/Sub-Object materials, renderer overhaul, dynamic sky

0. ✅ **Show End Result.** The Modify panel's toggle shows the stack evaluated only up to the current modifier (viewport and sub-object display), like 3ds Max.
1. ✅ **Multi/Sub-Object materials and emissive area lights.** `src/materials/multi.rs`, `src/ui/material_editor/multi.rs`, `src/render/emitters.rs`, `src/viewport/multi_sync.rs`.
   - Any of the 24 slots switches between Standard and **Multi/Sub-Object**. A scrollable list of IDs 1..N with `[Set Number]`, ID #, name, colour swatch and `[On/Off]`; click a row to edit that sub-material with the full parameter rollouts (PBR, Toon, Crystal, Unlit, Ink Outline per ID).
   - Polygons render with the sub-material their `material_id` picks (3ds Max wrap-around; switched-off IDs use the default grey) in the viewport, the F9 raytracer and both Cycles paths.
   - Presets: *Medieval Castle Pack*, *Modern City Pack*, *Alpine Terrain Pack*.
   - Every material has **Luminance / Emission Colour** and **Emission Multiplier (0..100)**. Polygons with a multiplier above 0 are area-light emitters: F9 samples them with next-event estimation; Cycles and Blender get real emission shaders.
2. ✅ **Built-in renderer upgrades.** `src/render/gi.rs`, `postfx.rs`, `post.rs`, `sample.rs`, `src/ui/render_fx.rs`.
   - F10 **GI & Post-FX** tab. *GI & AO:* Ambient Occlusion (Samples 4..64, Radius, Strength); 1-bounce indirect GI (cosine-weighted bounce rays with next-event estimation on scene lights and emitters); Emitter Samples; an edge-preserving bilateral denoiser (normal, depth and object aware); a procedural **Studio Softbox** reflection dome when no HDRI is loaded.
   - *Post-FX:* Bloom / Glare (Threshold, Intensity, Radius), Depth of Field (Focus Distance, `[Pick Focus Object]`, f-stop), tonemapping (AgX, ACES Filmic, Reinhard, Standard sRGB) with Exposure (EV) and Contrast.
   - The F9 window keeps the render's layers, so the tone operator, EV and contrast in its toolbar update the image live without re-rendering. The Blender bridge maps DOF and the view transform / exposure / look.
3. ✅ **Dynamic time-of-day sky, clouds and deep space** (`8` dialog). `src/render/sky.rs`, `src/editor/sky.rs`, `src/ui/sky_dialog.rs`.
   - Sky Mode: Off, **Dynamic Earth Sky**, **Deep Space**. Earth: Time of Day slider (00:00..24:00) moves the sun and moon; the analytic Rayleigh / Mie look gives a blue noon, golden-hour sunsets and an indigo starlit night. Sun path azimuth and sky intensity.
   - `[✓] Link Main Directional Sun to Sky Time` aims the first directional light (creating a "Sun" if needed) along the sun, with warm golden-hour colour, and switches to cool moonlight at night.
   - **Procedural Clouds** rollout: Coverage, Density, Altitude, Wind Drift Speed (clouds drift with the time of day), tinted by sunset and moonlight.
   - **Deep Space:** high-contrast starfield (density, brightness), two-colour nebula bands with density, and one bright star (azimuth, elevation, colour) that lights the scene.
   - F9 evaluates the sky per ray, so stars and the sun stay sharp in the background and reflections. The sky is also baked to a 1024-wide equirect image, used for ambient light, the skybox in perspective viewports, and a temporary `.hdr` for Cycles / Blender.
   - Tests: noon blue zenith, sunset glow on the sun's side, dark night with stars and moonlight, cloud coverage and wind drift, deep-space stars / nebula / star, bake layout and cache key, `.hdr` round trip, an F9 render using the sky, the linked sun following the time, and the HH:MM clock label.

### Hands-on test (10 November 2026)

The app was run on a virtual display and driven with real clicks and hotkeys, using a test scene: floor, a Multi/Sub-Object box, an emissive blue panel, a wood sphere, Camera001 and the Earth sky.

| Feature | Result |
|---|---|
| Multi/Sub-Object box in the viewport | ✅ each face shows its sub-material |
| `M` editor: Set Number, ID list, sub-material editing, pack presets | ✅ |
| F10 GI & Post-FX tab | ✅ all controls present |
| F9 at 800×600 with GI, AO and bloom | ✅ about 9 s; the panel blooms and lights the floor blue |
| Live retone (tone operator, EV, Contrast) | ✅ updates without a re-render |
| `8` Dynamic Sky: time as HH:MM, clouds rollout | ✅ |
| Viewport skybox at 17:30 and 22:00 | ✅ sunset clouds; indigo night with moonlight on the scene |
| F9 night render through Camera001 | ✅ sharp starfield (after fix 1) |
| Deep Space mode | not tested by hand (covered by unit tests) |

**Fixed during the test (PR #6):**
1. F9 stars were blurry because the renderer read the 512-wide sky bake. It now evaluates the sky per ray.
2. The selected row label in the Multi/Sub-Object list was amber on amber. It is now dark bold text.
3. The `8` dialog was only about 400 px tall. It now uses a taller scroll area.

**Open minor issues (all fixed in Phase 8B, §12 step 0):**
- ✅ Viewport stars were softer than in F9. The skybox cubemap faces are now 512 px, built on six threads.
- ✅ Sparkle noise on the floor near emitters at low Emitter Samples. Fixed with a distance² floor of 0.05 and a firefly clamp on emitter NEE weights (solid angle bounded at 2π, weight cap 4·2π/N). This is a clamp, not full MIS.
- ✅ Sub-material name fields were cut off. The Name column is now 132 px (was 78) and the sub-material button is at least 100 px wide.
- ✅ At night the box rendered almost black under moonlight alone. The night sky now gives a little more skylight (a night ambient term plus a brighter zenith and horizon), and the default Moonlight multiplier is 0.22 (was 0.12).

## 12. Phase 8B: §11 polish, procedural Terrain, Gerstner Ocean and Planet Studio

Merged to `main` in PR #7. Covered by headless tests only so far; a hands-on test in the running app (Create -> Geometry -> Worlds, the Planet rollouts and an F9 render of a planet) is still to do.


0. ✅ **§11 polish:** the four open issues above. `src/viewport/skybox.rs`, `src/render/hdri.rs`, `src/render/gi.rs`, `src/render/sky.rs`, `src/ui/material_editor/multi.rs`. The F9 raytracer's shadow rays also pass through translucent surfaces (opacity × vertex alpha, up to 8 layers), so cloud shells and glass cast soft, partial shadows.
1. ✅ **Terrain** (`Create -> Geometry -> Worlds -> Terrain`, and a **Terrain** stack modifier). `src/generators/terrain.rs`, `terrain_ids.rs`, `erosion.rs`.
   - Biome presets: Plains, Rolling Hills, Jagged Mountains (ridged multifractal plus erosion), Mesa (terraces and canyon cuts), Savannah (riverbeds and termite mounds), Desert Dunes (wind-sheared Worley crescents with a Wind Direction).
   - Parameters: Width, Length, Segments, Random Seed with `[🎲 New Seed]`, Max Elevation, Octaves, Roughness, Terrace Steps, Erosion Strength (droplet hydraulic erosion followed by thermal talus slumping).
   - `[✓] Auto-Assign Material IDs by Slope & Altitude` with Cliff Angle, Snow Line and Sand Line: ID 1 Grass, 2 Cliff Rock, 3 Snow, 4 Sand / Riverbed. A new Terrain gets its own copy of the **Alpine Terrain Pack**, now reordered to match (1 Alpine Grass, 2 Granite Cliff, 3 Snow, 4 Riverbed Sand, then Pine Needles, Glacier Ice, Lake Water, Dirt Path).
   - The modifier raises any input mesh along local Z over its XY bounds.
2. ✅ **Gerstner Ocean** (base object and **Ocean** modifier). `src/generators/ocean.rs`.
   - Six-wave trochoidal spectrum around the wind direction. Parameters: Width, Length, Segments, Wave Height, Choppiness (per-wave steepness so crests never loop), Wavelength, Wind Direction, and a Phase / Time scrubber.
   - `[✓] Auto-Crest Foam` with Foam Threshold: foam from crest height and the Jacobian pinch whitens `Vertex.color` and sets Material ID 2. A new Ocean gets the new **Ocean Water Pack** (1 Ocean Water, 2 Wave Foam).
3. ✅ **Planet Studio** (`Create -> Geometry -> Worlds -> Planet`). `src/generators/planet.rs`, `planet_surface.rs`, `planet_layers.rs`, `sphere.rs`, `src/editor/worlds.rs`, `src/ui/rollouts/planet.rs`.
   - Equal-angle (tangent-warped) quad sphere: even quads with no pole pinching, shared seam vertices, and lat-long UVs that are unwrapped across the seam. Radius, Subdivisions (4..128), Random Seed.
   - **1. Planet Surface:** presets Terran, Moon (seeded craters), Desert, Ice, Gas Giant (bands and storm ovals); Sea Level and Mountain Relief; IDs 1 Ocean / Lowland, 2 Land, 3 Highland, 4 Ice. `[Load Custom Surface Texture]` turns the surface into a plain ID 1 sphere and puts the image on the ID 1 sub-material; `[✕ Procedural]` restores it.
   - **2. Weather & Cloud Shell** (radius × 1.015, ID 5): Coverage, Swirl Intensity (Coriolis spirals around storm eyes, counter-clockwise in the north), Storm Eye Count, Opacity, latitude weather bands, and an optional custom cloud map (colour and opacity). The cloud alpha casts soft shadows on the surface in F9.
   - **3. Atmosphere Rim Halo** (ID 6): three stacked see-through shells, so the halo glows brightest at the limb. Colour, Density, Sunset Terminator Glow, and Sun Azimuth / Elevation.
   - **4. Planetary Rings** (ID 7): Inner and Outer Radius, Tilt Angle, Radial Band Noise (with a Cassini-style gap), Opacity.
   - Each layer has its own toggle. Presets set sensible layers: gas giants get rings, and moons have no air or clouds. A new Planet gets the **Planet Studio Pack**.
   - Tests (`src/generators/tests.rs`, `tests_planet.rs`, `src/editor/worlds.rs`): grid topology `(n+1)²` vertices / `2n(n+1)` edges / `n²` polygons; IDs 1–4 for every biome (mountains have cliffs and snow, savannah has riverbeds, dunes are mostly sand); seed determinism; erosion lowers the steepest step; both modifiers keep topology; ocean foam IDs and white foam colours, with no foam when it is off; the quad sphere is closed (6n² quads, Euler 2, outward, seam-safe UVs, near-even areas); surface IDs 1–4 per preset; layer IDs 5 / 6 / 7 come and go with their toggles; clouds sit at 1.015 R; rings face +Z inside their radii; a custom surface is plain ID 1; each world gets its matching pack; loading a custom texture and going back to procedural.

---

## 13. Phase 8C: GhostTown settlements, Medieval Architect and Battlefield destruction

Merged to `main` in PR #9. Covered by headless tests only; a hands-on test in the running app is still to do.

0. ✅ **Building kit** (`src/generators/kit/`): a shared mesh builder (blocks, cuboids, beams, prisms, frustums, tubes, strips, gable / hipped roofs, arched walls and panels, ground grids) with seeded RNG and the GhostTown Material IDs. All three features build on it.
1. ✅ **GhostTown** (`Create -> Geometry -> Settlement`, and a **GhostTown** stack modifier that fills a flat Plane / Editable Poly or a closed spline's outline). `src/generators/settlement/`.
   - Presets: **Modern Downtown** (setback towers with lit window bays, rooftop AC units and helipads, streetlamps), **Modern Suburbs** (houses with wings, garages, porches, fenced yards), **Medieval Walled City** (cobbled plaza with a well and lindens, radial and ring lanes, curtain wall with towers and gates, half-timbered townhouses), **Medieval Village** (hilltop church, cottages, crop fields, fences, wells).
   - Parameters: Seed with `[🎲]`, Area Width / Length (object only), Block Size, Street Width, Sidewalk Width, Building Density, Min / Max Floors, Roof Pitch, Tree / Prop Density, and `[✓] Damaged / Battlefield Mode` with the Battlefield parameters.
   - New **GhostTown Settlement Pack** (`[ Assign GhostTown Material Pack ]`): 1 Road / Cobblestone, 2 Lot / Dirt, 3 Facade, 4 Timber / Brick Trim, 5 Roof, 6 Glass, 7 Lit Window, 8 Foliage, 9 Charred Wood, 10 Iron, 11 Plaster, 12 Rusted Rebar.
2. ✅ **Medieval Architect** modifier. `src/modifiers/medieval/`.
   - Presets: Watchtower, Curtain Wall / Gatehouse, Half-Timbered House, Bridge / Aqueduct, Gothic Cathedral (vaulted nave, flying buttresses, portal and rose window). Plan shape Auto / Round / Square (Auto detects cylinders); a closed spline input builds a curtain wall along it.
   - Toggles: 3D Stone Bricks, Crenellations, Machicolations, Arrow Slits / Windows, Door / Portcullis, Buttresses, Roof / Spire. Spinners: Master Brick Scale, Floor Height, Wall Thickness, Merlon / Crenel Ratio, Window Spacing, Spline Wall Height, Seed.
   - **Scale-aware:** bricks, merlons, floors, windows and bridge arches are recounted from the base size, so resizing the base adds detail instead of stretching it.
3. ✅ **Battlefield / Ruin** modifier, also run by the Damaged toggles. `src/modifiers/battlefield/`, `src/editor/blast_zones.rs`.
   - **Modern / Mecha Warfare:** tilted shear planes, pancaked slabs, bent rebar (ID 12), shattered glass shards, bomb and mech-footprint craters, buckled roads, bent streetlamps.
   - **Medieval Siege:** V-breaches, knocked-off merlons, burned roofs with charred rafters (ID 9), impact pits, siege ruts, talus piles, carts, barrels and splintered beams. Never rebar or concrete.
   - **Ancient Overgrown Ruins:** eroded wall tops, caved roofs, snapped pillars with fallen drums, sinkholes, moss (ID 8 with green vertex colour) and ivy at wall feet.
   - Spinners: Intensity 0–100 %, Seed, Crater Count, Crater Radius, Crater Depth, Rubble Density, Soot Darkening.
   - **Blast Zones** sub-object (`1`): `[➕ Add Zone]` / `[🗑 Remove Zone]` and a zone list with radius spinners; zones draw as three yellow great circles (orange when selected), are picked by click or marquee, move with W and scale with R. With no zones, seeded blasts hit the structures.
4. Tests (`src/generators/kit/tests.rs`, `src/generators/settlement/tests.rs`, `src/modifiers/medieval/tests.rs`, `src/modifiers/battlefield/tests.rs`, `src/modifiers/tests_gizmo.rs`, `src/editor/worlds.rs`): every era is valid, deterministic, seed-dependent, stays in its area and only uses IDs 1–12; era signatures (downtown towers with lit glass, low suburban houses, timber and plaster medieval towns with no concrete); density and floor counts; the modifier on a plane and a closed spline; damaged medieval towns have charred wood and no rebar, damaged modern towns have rebar, all have craters; every Architect preset is valid, deterministic and sits on its base; toggles remove their details; resizing keeps brick size and recounts details; round / square detection; spline curtain walls; each destruction mode's signature (rebar and shards, charred rafters and no rebar, moss and erosion); zero intensity changes nothing; a user zone only damages its area; zone select / move / scale in the editor; assigning the GhostTown pack with undo.
