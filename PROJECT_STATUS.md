# Ilex 3D (formerly MiniMax, Holly-3D): Project Status Report

_Status as of 12 November 2026, measured against the spec in `CLAUDE.md`._

## At a glance

| | |
|---|---|
| Build phases | **All done and merged to `main`**: Phases 1–5 from `CLAUDE.md` (PRs #1–#3), Phase 6 (gap closure and hardware optimisation) and Phase 7 (mech assembly) (PRs #4–#5), **Phase 8A** (Multi/Sub-Object materials, renderer overhaul, dynamic sky; PR #6, tested by hand in the running app), **Phase 8B** (§11 polish, Terrain, Gerstner Ocean, Planet Studio; PR #7, status update in PR #8, unit-tested, not yet tested by hand), **Phase 8C** (Demiurge settlements, Medieval Architect, Battlefield & Ruin; PR #9, unit-tested, not yet tested by hand), **Phase 9 PR 1** (a button for everything; see §15) **Phase 9 PR 2** (Preferences; see §16) and **Phase 9 PR 3** (click-and-drag creation; see §17) |
| Source | 507 `.rs` files, about 76,300 lines; the largest is 350 lines (limit 350) |
| Tests | 581 headless tests, all passing (`cargo test --lib`) |
| Lint | `cargo clippy --all-targets` reports no warnings |
| Packaging | `packaging/`: a pacman PKGBUILD (version `0.<commits>.g<hash>`, `znver3`) that installs `/usr/bin/ilex` (with a `/usr/bin/minimax` symlink), the desktop entry, the white Holly tree icon and the `.ilex` / `.mmax` / `.ilexpose` file types (§32), plus `publish-repo.sh` for a local `[minimax]` repository (see README and §14). Built and published end to end with makepkg; not yet installed through pacman on CachyOS |
| Licence | GPL-3.0-or-later (`LICENSE`, Cargo.toml, PKGBUILD, README, Help → Credits). Third-party code and assets in `licenses/` are MIT, Apache-2.0, OFL-1.1 or CC0, all GPLv3-compatible; the Rust dependencies were checked too (MIT / Apache-2.0 / BSD / Zlib / ISC / Unicode / MPL-2.0 / CDLA-Permissive-2.0) |
| Stack | Bevy 0.20, bevy_egui 0.43.1, transform-gizmo-egui, slotmap, boolmesh, meshopt, uvunwrap, noise, rand/rand_chacha, image |

**Phases 6 and 7 merged in PR #4.** **Phase 6 done:** all §9 gaps are closed (✅ marks below), plus copy-on-write undo, a tiled renderer and a SIMD BVH. **Phase 7 done:** the bone-free mech assembly system, mech primitives and Panel Groove (§10). File I/O is done: `.mmax` scenes plus OBJ, glTF/GLB and FBX import and export. **Phase 8A done** (§11): Multi/Sub-Object materials with emissive area lights, AO / 1-bounce GI / denoiser / softbox / bloom / DOF / live tonemapping, and a time-of-day sky with clouds plus a deep-space mode. Show End Result now works. **Phase 8B done** (§12): the four §11 issues are fixed, plus procedural worlds under `Create -> Geometry -> Worlds`: biome Terrain with erosion and slope / altitude Material IDs, a Gerstner Ocean with crest foam, and the layered Planet Studio. **Phase 8C done** (§13): Demiurge settlements in four eras, the scale-aware Medieval Architect, and period-accurate Battlefield & Ruin destruction with movable Blast Zones. **Packaging done** (§14): a pacman package and a local `[minimax]` repository, plus a fix for release builds, which did not compile before.

Legend: ✅ done · 🟡 partly done · ❌ not done

---

## 1. Core rules (§1)

| Rule | Status | Notes |
|---|---|---|
| conventions only | ✅ | Q/W/E/R, 1–5, M, F9, F10, Shift+Drag clone; no modal keys |
| Clean five-zone UI | ✅ | Graphite & Teal (`#242424` with `#26B8B0`); Slate & Amber still selectable |
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
- A Node Graph view with Noise, Checker, Gradient Ramp and Color Correction nodes.

### E. Direct 3D painting
✅ All of it:
- VertexPaint modifier with the PaintBox floater: size, opacity, falloff, colour, all or selected faces only, Fill Object, Blur, Layer Opacity.
- Texture projection paint at 2048 or 4096, with Auto-Setup Paint Canvas.

### F. Rendering (`F9` / `F10`)
- ✅ **Render Setup:**
  - Resolution presets, aspect lock, transparent background.
  - Choice of the built-in raytracer (32x32 parallel tiles, 4-wide SIMD BVH, soft shadows, reflection and refraction, anti-aliasing, tonemapping, outline pass) or the external renderer bridge. By default the bridge runs `blender --background -E CYCLES` with a Python loader; it can also write the external renderer XML for the standalone `cycles` tool.
- ✅ **Rendered Frame Window:**
  - Render, a viewport/camera picker, Region.
  - Save as PNG, EXR or JPG; Copy to Clipboard; Clone Render Window.
  - R, G, B, A and Mono channel buttons.
- ✅ **Cameras:** Target and Free Camera objects; Render Setup and the Frame Window list them next to the viewports.
- ✅ external renderer bridge, verified against a real install of the external renderer. F10 defaulted to its executable, the HIP device with HIP-RT, and OpenImageDenoise. A missing GPU device falls back to the CPU, and a the external renderer build without OIDN renders undenoised.

### G. Lighting
- ✅ The default two-light headlight rig, with `Ctrl+L` and automatic switching to scene lights.
- ✅ Light types: Omni, Target Spot, Free Spot, Target Directional, Free Directional, Skylight.
- ✅ Light Parameters: on/off, type, shadows, multiplier, colour plus Kelvin presets, decay/range, Show Attenuation, hotspot and falloff, shadow bias, softness and colour.
- ✅ Hotspot and falloff cone gizmos.
- ✅ Near and far attenuation (Use Near / Near Start / Near End plus the far range), each drawn as a pair of wire spheres when Show Attenuation is on.
- ✅ The Environment dialog (`8`): five lighting rig presets, background colour, ambient tint, exponential fog.
- ✅ **HDRI environment map** (`.hdr` / `.exr`, equirectangular): Strength, Rotation, Use for Lighting, Show in Perspective Viewports. It is the F9 background and reflection source, the external renderer's world texture, and a skybox in perspective viewports.

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
| Settlement, Medieval Architect, Battlefield / Ruin (Phase 8C) | ✅ | Settlement objects under `Create -> Geometry -> Settlement` and stack modifiers; see §13 |

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
| 4 | Materials, painting, lighting, Render Setup, Frame Window, raytracer, external renderer bridge | #3 ✅ merged |
| 5 | Splines, Lathe, Extrude, Array, Mirror, GeoGen, Greeble, Chamfer, Symmetry, Shell, TurboSmooth, deformers, Edit Normals, Boolean, ProOptimizer | #3 ✅ merged |
| 6 | File I/O (`.mmax`, OBJ, glTF/GLB, FBX), cameras, Align, Edit Poly gap tools, modifier gizmos, Spline Boolean / Refine, 3D Snaps, selection regions, HDRI, the external renderer bridge, copy-on-write undo, tiled renderer, SIMD BVH (§9) | #4 ✅ merged |
| 7 | Bone-free mech assembly: joints and the J tool, FK / IK drag, Mirror Mech Limb, Contact Sets, Motion Links, Gear / Piston / Hose primitives, Panel Groove (§10) | #4 ✅ merged |
| 8A | Show End Result, Multi/Sub-Object materials, emissive area lights, GI & Post-FX, dynamic sky / clouds / deep space (§11) | #6 ✅ merged |
| 8B | §11 polish, procedural Terrain (6 biomes, erosion, auto Material IDs), Gerstner Ocean with crest foam, Planet Studio (surface, clouds, atmosphere, rings) (§12) | #7 ✅ merged |
| 8C | Demiurge settlements (4 eras), Medieval Architect (5 presets, scale-aware), Battlefield & Ruin (3 period-accurate modes, Blast Zones) (§13) | #9 ✅ merged |
| Packaging | pacman PKGBUILD, desktop entry, icon, `.mmax` file type, `publish-repo.sh` local repository, release-build fix (§14) | #11 (open) |

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
   - Render Setup and the Frame Window list the cameras, and F9 renders through them. The raytracer and the external renderer honour the lens; near clip applies in the raytracer and viewports. In the viewports, the far clip only culls whole objects, because Bevy's projection is infinite.
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
10. ✅ **Checks and the external renderer bridge.** `src/render/blender.rs`, `src/render/blender_loader.py`, `src/render/cycles.rs`, `src/ui/render_setup.rs`.
   - Edit Normals `E` rotation tried in the app: the gizmo sits on the selected normals and the ring drag turns them. A test drives the same drag path on a rotated object (world X rotation, top normals end up at world -Y).
   - The external renderer bridge (the default): the scene is exported as JSON (one mesh per object and material, with corner normals; Principled materials; point, spot and sun lights; camera; color or HDRI world). A bundled Python loader rebuilds it in `blender --background --factory-startup -E CYCLES` and writes a PNG. A real the external renderer 4.0 render of a box under a spot light came back into the frame buffer correctly.
   - F10 external-renderer options: Run Through (bridge / standalone), executable path, Device (HIP, OptiX, CUDA, oneAPI, Metal, CPU; default HIP), HIP-RT (default on), Denoise with OpenImageDenoise (default on), Samples, Max Bounces. The standalone route passes `--device` too.
   - Tests: defaults, including old scene files without the new fields; the JSON (triangle count, camera axes, spot light, HIP-RT only on HIP); the loader reads every exported key; a round trip with a stand-in `blender` script; readable errors for a missing executable. `real_blender_renders_the_scene` (ignored by default) renders through a real external renderer.
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

0. ✅ **Show End Result.** The Modify panel's toggle shows the stack evaluated only up to the current modifier (viewport and sub-object display).
1. ✅ **Multi/Sub-Object materials and emissive area lights.** `src/materials/multi.rs`, `src/ui/material_editor/multi.rs`, `src/render/emitters.rs`, `src/viewport/multi_sync.rs`.
   - Any of the 24 slots switches between Standard and **Multi/Sub-Object**. A scrollable list of IDs 1..N with `[Set Number]`, ID #, name, colour swatch and `[On/Off]`; click a row to edit that sub-material with the full parameter rollouts (PBR, Toon, Crystal, Unlit, Ink Outline per ID).
   - Polygons render with the sub-material their `material_id` picks (wrap-around; switched-off IDs use the default grey) in the viewport, the F9 raytracer and both external-renderer paths.
   - Presets: *Medieval Castle Pack*, *Modern City Pack*, *Alpine Terrain Pack*.
   - Every material has **Luminance / Emission Colour** and **Emission Multiplier (0..100)**. Polygons with a multiplier above 0 are area-light emitters: F9 samples them with next-event estimation; the external renderer and the external renderer get real emission shaders.
2. ✅ **Built-in renderer upgrades.** `src/render/gi.rs`, `postfx.rs`, `post.rs`, `sample.rs`, `src/ui/render_fx.rs`.
   - F10 **GI & Post-FX** tab. *GI & AO:* Ambient Occlusion (Samples 4..64, Radius, Strength); 1-bounce indirect GI (cosine-weighted bounce rays with next-event estimation on scene lights and emitters); Emitter Samples; an edge-preserving bilateral denoiser (normal, depth and object aware); a procedural **Studio Softbox** reflection dome when no HDRI is loaded.
   - *Post-FX:* Bloom / Glare (Threshold, Intensity, Radius), Depth of Field (Focus Distance, `[Pick Focus Object]`, f-stop), tonemapping (AgX, ACES Filmic, Reinhard, Standard sRGB) with Exposure (EV) and Contrast.
   - The F9 window keeps the render's layers, so the tone operator, EV and contrast in its toolbar update the image live without re-rendering. The external renderer bridge maps DOF and the view transform / exposure / look.
3. ✅ **Dynamic time-of-day sky, clouds and deep space** (`8` dialog). `src/render/sky.rs`, `src/editor/sky.rs`, `src/ui/sky_dialog.rs`.
   - Sky Mode: Off, **Dynamic Earth Sky**, **Deep Space**. Earth: Time of Day slider (00:00..24:00) moves the sun and moon; the analytic Rayleigh / Mie look gives a blue noon, golden-hour sunsets and an indigo starlit night. Sun path azimuth and sky intensity.
   - `[✓] Link Main Directional Sun to Sky Time` aims the first directional light (creating a "Sun" if needed) along the sun, with warm golden-hour colour, and switches to cool moonlight at night.
   - **Procedural Clouds** rollout: Coverage, Density, Altitude, Wind Drift Speed (clouds drift with the time of day), tinted by sunset and moonlight.
   - **Deep Space:** high-contrast starfield (density, brightness), two-colour nebula bands with density, and one bright star (azimuth, elevation, colour) that lights the scene.
   - F9 evaluates the sky per ray, so stars and the sun stay sharp in the background and reflections. The sky is also baked to a 1024-wide equirect image, used for ambient light, the skybox in perspective viewports, and a temporary `.hdr` for the external renderer / the external renderer.
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

## 13. Phase 8C: Demiurge settlements, Medieval Architect and Battlefield destruction

Merged to `main` in PR #9. Covered by headless tests only; a hands-on test in the running app is still to do.

0. ✅ **Building kit** (`src/generators/kit/`): a shared mesh builder (blocks, cuboids, beams, prisms, frustums, tubes, strips, gable / hipped roofs, arched walls and panels, ground grids) with seeded RNG and the Settlement Material IDs. All three features build on it.
1. ✅ **Settlement** (`Create -> Geometry -> Settlement`, and a **Settlement** stack modifier that fills a flat Plane / Editable Poly or a closed spline's outline). `src/generators/settlement/`.
   - Presets: **Modern Downtown** (setback towers with lit window bays, rooftop AC units and helipads, streetlamps), **Modern Suburbs** (houses with wings, garages, porches, fenced yards), **Medieval Walled City** (cobbled plaza with a well and lindens, radial and ring lanes, curtain wall with towers and gates, half-timbered townhouses), **Medieval Village** (hilltop church, cottages, crop fields, fences, wells).
   - Parameters: Seed with `[🎲]`, Area Width / Length (object only), Block Size, Street Width, Sidewalk Width, Building Density, Min / Max Floors, Roof Pitch, Tree / Prop Density, and `[✓] Damaged / Battlefield Mode` with the Battlefield parameters.
   - New **Settlement Pack** (`[ Assign Settlement Material Pack ]`): 1 Road / Cobblestone, 2 Lot / Dirt, 3 Facade, 4 Timber / Brick Trim, 5 Roof, 6 Glass, 7 Lit Window, 8 Foliage, 9 Charred Wood, 10 Iron, 11 Plaster, 12 Rusted Rebar.
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
4. Tests (`src/generators/kit/tests.rs`, `src/generators/settlement/tests.rs`, `src/modifiers/medieval/tests.rs`, `src/modifiers/battlefield/tests.rs`, `src/modifiers/tests_gizmo.rs`, `src/editor/worlds.rs`): every era is valid, deterministic, seed-dependent, stays in its area and only uses IDs 1–12; era signatures (downtown towers with lit glass, low suburban houses, timber and plaster medieval towns with no concrete); density and floor counts; the modifier on a plane and a closed spline; damaged medieval towns have charred wood and no rebar, damaged modern towns have rebar, all have craters; every Architect preset is valid, deterministic and sits on its base; toggles remove their details; resizing keeps brick size and recounts details; round / square detection; spline curtain walls; each destruction mode's signature (rebar and shards, charred rafters and no rebar, moss and erosion); zero intensity changes nothing; a user zone only damages its area; zone select / move / scale in the editor; assigning the Settlement pack with undo.

---

## 14. Packaging: pacman package and local repository

Everything lives in `packaging/`; the README's *As a pacman package* section has the user steps.

1. ✅ **`PKGBUILD`** (`minimax`):
   - **Version:** `pkgver()` gives `0.<commit count>.g<short hash>`, so every commit is an upgrade for `pacman -Syu`. It builds the committed state of the checked-out branch.
   - **Build:** `cargo build --release --locked` for `znver3`, or another CPU through `MINIMAX_TARGET_CPU`. `check()` runs `cargo test --lib`.
   - **Installs:**
     - `/usr/bin/minimax`
     - `minimax.desktop`
     - the Slate & Amber (`#222327` / `#E59C24`) SVG icon, also used as the file icon for scenes
     - `mime/packages/minimax.xml`, which registers `application/x-minimax-scene` by the `*.mmax` extension and by the `format: "MiniMax"` header
     - the README
2. ✅ **`publish-repo.sh`:**
   - Runs `makepkg -sf`, copies the package into `~/.local/share/minimax-repo/` (or `MINIMAX_REPO_DIR`) and runs `repo-add -R`, which also removes the old package.
   - Prints the three `/etc/pacman.conf` lines with the real path.
   - Restores the PKGBUILD that makepkg rewrites.
   - Warns when pacman 7's `alpm` download user cannot reach the repository folder.
3. ✅ **Desktop integration:**
   - The window class is now `minimax`, matching `StartupWMClass`.
   - `Exec=minimax %f` opens a double-clicked scene through the Phase 6 command-line loader.
4. ✅ **Fixes the package build turned up:**
   - Release builds did not compile, because egui's debug style option is now behind `cfg(debug_assertions)`.
   - Two tests wrote to fixed `/tmp` folders and failed for a second user. They now use per-process folders.
5. **Verified:**
   - A full `publish-repo.sh` run as a normal user (makepkg 6 on a non-Arch host) built version `0.59.g20937f2`, passed all 320 tests in `check()` and packaged the files above. It created `minimax.db` / `minimax.files` and left the checkout clean.
   - `desktop-file-validate` passes.
   - `update-mime-database` and gio detect `.mmax` files, both by extension and by header, and pick MiniMax as the default app.
   - **Still to do:** a real `pacman -Syu minimax` on CachyOS.

## 15. Phase 9 PR 1: a button for everything

The first of the seven PRs in `docs/PHASE_9_PLAN.md`. New rule in `CLAUDE.md` §1: every feature must be reachable from a visible button, dropdown or menu. Hotkeys are shortcuts only.

| Area | What's new |
|---|---|
| Action registry | `src/ui/actions/`: one `Action` enum with label, short glyph, tooltip, run, checked and enabled state. The default keymap is built from it, and toolbar, menus, quad menu and hotkeys all call the same code |
| Menu bar | File · Edit · Tools · Views · Create · Modifiers · Rendering · Help. Shortcuts show next to each entry and toggles have tick marks. Create lists every object type; Modifiers lists every modifier, grouped |
| Main Toolbar | Built from `actions::layout`. Adds Undo/Redo, Selection Filter, the region-shape dropdown, Window/Crossing, Lock Selection, Select From Scene, Mirror/Align/Array, a one-click Sky menu (day, sunset, overcast, night, deep space, nebula), Material Editor and Render. Wraps onto a second row on narrow windows instead of hiding buttons |
| Command Panel | Five tabs: Create, Modify, Hierarchy (was Pivot), Display (Hide by Category, Hide/Freeze, Display Properties) and Utilities (Reset XForm, Collapse, Measure and more) |
| Modifier List | One tall scrolling list of every modifier, grouped under headings with an optional filter. Modifiers that don't fit the selection are dimmed and explain why. Show Buttons gives a grid of favourite modifiers; right-click its header to edit the set |
| Modeling Ribbon | Toggle with 🛠 Ribbon. Big buttons for every Edit Poly tool, which change with the sub-object level. Caddy tools apply their last settings on click; right-click opens the caddy |
| Viewport navigation | Eight buttons at the bottom right: Zoom, Zoom All, Field of View, Pan, Orbit, Zoom Extents, Zoom Extents All, Maximize. Drag in a viewport to use one; right-click leaves it |
| Status bar | Prompt line (current tool and next step), Lock Selection, and `Grid = …` |
| Tool windows | Select From Scene, Measure, Grid and Snap Settings, Hotkey List, About |
| Tests | `every_action_has_a_button` enforces the rule. Other tests check that hotkeys are unique, that every action runs, and that every glyph used in the UI source exists in the UI fonts. The glyph test works around egui's `has_glyph`, which wrongly reports emoji-font glyphs as missing |


## 16. Phase 9 PR 2: Preferences

Opened with the toolbar ⚙ button or **Edit -> Preferences...**. Changes apply at once and are saved to `~/.config/minimax/preferences.ron` when the mouse button is released. A missing or partial file falls back to the defaults.

| Tab | What it does |
|---|---|
| 🎨 Theme | Six presets (Slate & Amber, Classic Gray, Midnight Blue, Light, High Contrast, Custom) with colour strips. Custom has ten colour pickers and warns when text would be hard to read (WCAG contrast check). UI size 90–150 % and font size 9–20 pt |
| ⌨ Hotkeys | Key set: Ilex default or Classic (H = Select by Name, Shift+Q = Render, Alt+Z = Zoom). Searchable list of every action: click a key, press a new one. A key that is already used asks Swap / Replace / Cancel. Per-action default and clear buttons, Reset, Export and Import (.ron) |
| ⊞ Viewports | Background (or the Environment colour), grid and axis colours, vertex and selected-vertex colours, invert zoom, zoom / orbit / pan speed |
| ⚙ General | Autosave every N minutes (0 = off) to `~/.local/share/minimax/autosave/`, undo steps (1–500), units (m / cm / ft for Measure and the grid readout), show the tour on startup |
| ▶ Rendering | Default renderer, external renderer path, resolution presets, and "Use these now" for Render Setup |
| 📦 Assets | Download folder and the three CC0 packs (Photo Textures 1K, Low-poly Models, Realistic Models). The download buttons come in PR 5 |

The Hotkey List window has a "⚙ Change hotkeys..." button. Tests cover the RON round trip, partial files, units, zoom, theme readability, keymap edit / swap / import / export, the key set, autosave and the undo limit.

## 17. Phase 9 PR 3: click-and-drag creation

Objects are drawn (`src/editor/create_drag.rs`). Pick a type on the Create tab or in the Create menu, then:

| Object | Drag | Then move and click |
|---|---|---|
| Box | Base, corner to corner | Height (Creation Method *Cube*: one drag from the centre) |
| Plane, Settlement | Rectangle; releasing finishes it | — |
| Sphere, Planet, Gear | Radius; releasing finishes it | — |
| Cylinder, GeoGen (rock, crystal, cliff, stalagmite) | Radius | Height |
| Cone | Bottom radius | Height, then top radius |
| Torus | Main radius | Tube thickness |
| Terrain, Ocean | Area rectangle | Height / wave height |

- **The view sets the plane:** Top and Perspective draw on the ground grid. Front and Left stand objects up on their own plane.
- **Snaps (`S`) work while dragging.**
- **Creation Method rollout:**
  - *Box / Cube* for the Box.
  - *Center / Edge* for round objects; Edge means the drag spans the diameter.
- **Cancel and undo:**
  - Right-click cancels a half-made object.
  - `Ctrl+Z` removes a finished one in one step.
- **The tool stays active**, so you can draw several objects in a row.
- **Default sizes:**
  - A plain click, with no drag, still places a default-size object.
  - Right-clicking a Create button places one at the origin.
  - The Piston and Hose are click-to-place only.
- **The status bar names the next step.** The Create panel stays open while you draw.
- **Tests:** seven tests simulate mouse presses and moves with no screen. They check:
  - each object's final size and position;
  - that one undo removes the object and that cancelling works;
  - that the Front view stands objects upright;
  - that a plain click still places a default object.
- **Checked in the running app:** dragging a Box in Perspective made a single box of the right size.

## 18. Phase 9 PR 4: Help with screenshots

- **Help window (`F1`, `❓ Help` on the toolbar, Help menu):**
  - 16 topics, in the order you learn them:
    1. Getting Started
    2. Creating objects
    3. Selecting and transforming
    4. Editable Poly modelling (plus the ribbon)
    5. Modifiers
    6. Splines, Lathe and Extrude
    7. Materials
    8. Painting
    9. Lights, cameras, sky and deep space
    10. Rendering
    11. Settlement, Medieval Architect and destruction
    12. Mech assembly
    13. Files
    14. Preferences and hotkeys
    - then a Hotkey cheat sheet and Credits.
  - Pages mix short text, numbered steps, tips and real screenshots. Amber numbered circles on each screenshot point at the parts the legend under it explains.
  - **"▶ Show me" buttons** open the window or tab the page is about (Material Editor, Render Setup, Environment, Preferences, the Create tab…).
  - **Search box** filters topics by any word on them.
  - **Previous / Next** buttons step through the pages.
  - **The cheat sheet is live:** it lists your current keys, so it follows changes made in Preferences → Hotkeys.
- **F1 opens the page for what is under the mouse:**
  - Material Editor → Materials
  - Render Setup or Frame Window → Rendering
  - PaintBox → Painting
  - Environment → Sky
  - caddy → Edit Poly
  - Array, Mirror or Align → Transforming
  - Command Panel → Create, Edit Poly or Modifiers, depending on the tab and selection
  - anything else → Getting Started
- **Getting Started tour:**
  - Eight screens with screenshots, Back / Next / Finish.
  - Shown at startup until you untick "Show this tour at startup" (also in Preferences → General).
  - Reopen it from Help → Getting Started Tour or the Help window.
- **Screenshots:**
  - 14 JPEGs (1200 px wide, about 1.3 MB in total) in `assets/help/`, built into the program.
  - They are captured from the real program: `minimax --help-shot <id>` sets up the scene and panels, and `tools/help_shots.sh [id …]` runs it under Xvfb and saves the images. Re-run it after UI changes.
- **Tests:**
  - every screenshot id sets up a scene, is built in and decodes;
  - every page and tour screenshot exists, and every marker lies inside its image;
  - search finds pages (any case), page ids are unique;
  - the Help action opens the window.
- **Checked in the running app:** the tour opens on first start, the Help window shows the topics and screenshots, and F1 over the Material Editor opens the Materials page.

## 19. Phase 9 PR 5a: Material Library and procedural pattern maps

- **Material Library (`📚 Library` on the toolbar, in the Material Editor and in Rendering → Material Library...):**
  - 281 ready-made materials in 15 categories:
    - Classic presets (7)
    - Ground & Terrain (30): grass, mud, sand, snow, ice, moss, lava crust, peat…
    - Roads & Urban (30): new / worn / cracked / wet asphalt, white and yellow centre lines (solid and dashed), edge lines, crosswalks, sidewalk slabs, curbs, cobbles, pavers, bike and bus lanes…
    - Walls & Facades (30)
    - Roofs (15)
    - Metals (30)
    - Wood (20)
    - Stone & Concrete (20)
    - Glass & Gems (20)
    - Plastic & Rubber (15)
    - Fabric & Leather (12)
    - Nature & Organic (15)
    - Glow / Emissive (15)
    - Toon & Stylised (12)
    - Sci-Fi (10)
  - The browser has a category list, a search box (name or category), and preview swatches. Swatches are rendered a few per frame and cached, so opening it never stalls.
  - **Click a swatch** to load it into the active slot (one undo step). The slot takes the material's name.
  - **☆ / ★ Favourites** are kept in Preferences (`favourite_materials`) and listed on their own shelf.
  - Buttons: **Assign to Selection** and **Editor**. The Material Editor's Quick Preset list also ends with "📚 More in the Material Library...".
- **New procedural map nodes** (Maps rollout and Node Graph). They work in the viewport, the raytracer and the previews alike:
  - **Bricks / Tiles:** running bond, stack bond, planks or cobbles. Settings: columns, rows, gap, colour variation and seed.
  - **Stripes:** count, width, dashes and soft edges, along U or V. This is what makes road markings.
  - **Cracks / Voronoi:** scale, width, variation and seed.
  - **Grunge / Wear:** dirt, rust, scratches or stains, with amount and seed. Library materials that use grunge also get matching roughness maps, and metallic maps when the material is metal.
  - Patterned library materials also get a bump map from their pattern.
- **Help:** the Materials page has a Material Library screenshot and a "Show me" button. F1 over the library opens the Materials page.
- **Tests (369 in total):**
  - the library holds at least 270 uniquely named materials, and every category has items;
  - every material is valid, and every map samples inside 0..1;
  - specs give the right shading: Ruby is crystal, Neon Pink glows, Toon Red has an outline, bricks get bump, rust gets metal and roughness maps, and the road line is white on asphalt;
  - pattern unit tests;
  - the browser filters by shelf, search and favourites;
  - loading an item is undoable, and favourites toggle.
- **Checked in the running app:** swatches render for every category, and the shelves, search and click-to-load work.
- **Next (PR 5b):** World / Triplanar mapping, and the CC0 asset downloader (Poly Haven 1K photo textures, low-poly and realistic model packs) in Preferences → Assets.

## 20. PR 5.5: MaterialCraft procedural engine, Pattern & Weathering rollout, Fira Code

- **Procedural engine (`src/materials/proc_engine/`), ported from MaterialCraft (MIT / Apache-2.0):**
  - **Tileable lattice noise** (`lattice.rs`): integer hashing, value, Perlin, fractal, ridged and cellular noise. Counts are whole numbers, so every pattern repeats seamlessly at any resolution.
  - **20 generators** (`generators.rs`, `masonry.rs`, `voronoi.rs`, `fibers.rs`), each with four knobs and a random id per tile:
    - masonry: running-bond bricks, stack-bond tiles, herringbone, medieval coursed stone, planks;
    - Voronoi: cobbles, rock plates / flagstones, warped cracks with branches, crackle, cells, asphalt aggregate;
    - directional: anisotropic wood grain, brushed metal, diamond plate, weave, chainmail, carbon-fibre twill, scratches;
    - fractal and ridged noise.
  - **Adjustments and filters** (`adjust.rs`, `filters.rs`): Levels, Height Blend, Slope Blur, Normal From Height (5 × 5 Sobel-like kernel) and cavity AO.
  - **Weathering** (`weather.rs`): edge wear (slope-blurred), cavity dirt and moss (height-blended into low places first), scorch (rust on metal), wetness with level puddles.
  - **Content-addressed cache** (`cache.rs`, `node_key.rs`): a key hashes everything that decides a result. Equal work is cooked once, an edit makes a new key, and old results leave a byte-bounded LRU (256 MB).
  - **Background worker** (`worker.rs`): the viewport and sample slots keep the previous look while a worker thread cooks the new maps; generators and filters run on all cores. F9 renders wait for the maps.
- **Compact Material Editor: "Procedural Pattern & Weathering" rollout** (first rollout, on every Standard material and every Multi/Sub-Object sub-material):
  - **Pattern:** None, Bricks & Masonry, Cobblestone & Flagstone, Voronoi Rock & Cracks, Wood Planks & Grain, Weave / Chainmail / Fabric, Brushed / Diamond Metal, Asphalt & Road Markings, Terrain Grass / Dirt / Moss.
  - **Style** within the pattern (23 in total, e.g. Herringbone Paving, Chainmail, Asphalt + Dashed Yellow Line). One click sets its colours, gap, bevel, roughness and metalness.
  - **Spinners:** Tiling / Scale, Seed with a 🎲 button, Gap / Mortar Width, Gap / Bevel Depth, Color Variation, Surface Relief, plus Surface and Gap colours.
  - **Live Weathering sliders (0–100 %):** Edge Wear / Chipping, Cavity Dirt / Grunge, Moss / Overgrowth, Scorch / Burn / Rust, Wetness / Puddles.
  - **Auto-Derive Normal, Roughness & Cavity AO** checkbox.
- **Normal maps and AO now render.** Before this PR the Bump / Normal slot had no effect anywhere.
  - **F9 raytracer and sample slots:** shading normals are tilted in the triangle's UV frame; AO darkens ambient light.
  - **Viewport:** baked normal and occlusion textures, with per-vertex tangents on every mesh.
  - **External renderer bridge:** base colour, roughness / metallic and normal maps are written as PNGs and wired into the Principled BSDF; meshes now export UVs. This covers every map type, not only the new ones. Not run against a real external renderer in this session (none installed); the JSON, the PNGs and the loader script are tested.
  - A height map on the Bump slot is turned into normals; a Normal From Height node on the slot is used as a normal map.
- **Maps rollout and Node Graph view:**
  - Five new map kinds on any channel: **Tileable Pattern** (all 20 generators), **Levels** (with a histogram of its input), **Height Blend**, **Slope Blur**, **Normal From Height**.
  - Nodes with inputs show each socket with its own map picker and parameters, in both Compact and Node Graph (the two views share one parameter editor).
  - The Node Graph draws and wires up to three input sockets per node. Inputs nested deeper than one level are edited in the inspector, not as separate canvas nodes.
- **Material Library:** new category **Weathered Surfaces (Procedural)** with 32 presets (Mossy Brick Wall, Wet Cobblestone Street, Rusty Diamond Plate, Charred Timber Planks...). The library now has 313 materials in 16 categories.
- **Fira Code is the default interface font** (proportional and monospace; bold for headings):
  - Built into the program from `assets/fonts/` (release 5.2). egui's fonts stay behind it as fallbacks, so emoji and icons still draw.
  - Preferences → Theme has a **Font** dropdown: Fira Code (default) or Classic (proportional). Font size scaling works as before.
  - The default font size went from 12.5 pt to 11.5 pt, since Fira Code is wider. Saved preferences keep their own size.
- **Credits and licences:** Help → Credits credits MaterialCraft (MIT / Apache-2.0) and Fira Code (OFL-1.1). The licence texts are in `licenses/`.
- **Help:** the Materials page has a Procedural Pattern & Weathering section with a screenshot. The other screenshots were taken before the font change.
- **Tests (402 in total, 33 new):**
  - every lattice noise and every generator tiles seamlessly and stays in 0..1;
  - normals lean away from rising height, flat heights give flat normals, cavities are occluded;
  - each weathering slider changes the surface its own way (moss fills gaps first, rust is not metal, puddles are glossy);
  - recipes and map nodes are content-addressed: equal keys cook once, any edit changes the key, filters share baked inputs;
  - the worker cooks in the background until the maps are ready;
  - Compact-mode edits: picking a pattern is one undo step, fills base colour / roughness / AO / normal, and weathering survives a style change;
  - tangents follow U and their sign follows V; the external-renderer export writes its PNGs;
  - Fira Code is first in every font family, is monospaced, and icons still draw.
- **Checked in the running app:** the rollout, the brick pattern with moss on a box in the viewport, the sample slot, and the Fira Code interface. A contact sheet of eight weathered presets showed relief, AO, puddles and rust as intended.
- **Not done:**
  - **World / Triplanar mapping and the CC0 asset downloader (PR 5b).** The Poly Haven hosts are blocked in this session's network, so the downloader could not be tested.
  - ~~The older Help screenshots were not re-captured with the new font.~~ Done in Phase 10B: all 20 screenshots were re-captured with the Ilex names and the current font (§33).

## 21. PR 5.6: layer painter, mesh map baker, damage stamps, Triplanar mapping, asset downloader, decoupled emission

- **Texture Paint now works in layers (`src/paint/layers/`), following TextureCraft's design (MIT / Apache-2.0):**
  - **Existing UVs are never overwritten.** UVs from a UVW Map, an Unwrap UVW or an imported / collapsed mesh are kept exactly. Only an object without a usable layout (a plain primitive, a generated mesh) gets a Smart UV Project.
  - **The object's material becomes "Layer 0: Base Material"** and is never written to. This holds for bitmap, procedural (PR 5 / PR 5.5) and Multi/Sub-Object materials. A transparent Paint Layer is added on top.
  - A material shared with other objects is copied first, so paint shows on one object only.
  - **Erase** removes paint from the active layer; the layers and the base below show again.
  - **Paint Layers and Fill Layers**, each with a 👁 toggle, an opacity and a blend mode: Normal, Multiply, Add / Glow, Screen, Height-Blend (fills low crevices first; the opacity is the fill level).
  - Layers can be added, deleted and moved up or down. Every command is one undo step.
- **Tiled copy-on-write texels (`src/paint/texel/`):**
  - Layers are stored in 64 × 64 tiles. Each texel holds colour, roughness, metallic, height and emission, each with its own coverage.
  - An undo snapshot copies pointers only; a stroke copies just the tiles it touches. Unpainted tiles use no memory.
  - Scene files store only painted tiles, compressed.
- **Brush engine (`src/paint/brush/`):**
  - Paint with a colour (pick which of Color / Rough / Metal / Height / Glow it writes) or with **any Material Library preset**: one stroke paints colour, relief, roughness, metallic and glow.
  - Stamps are projected on the brush's tangent plane and evaluated per texel in world space: strokes cross UV seams, skip faces that look away, and do not bleed onto surfaces behind.
  - **Nine brush alphas and damage stamps:** Soft Round, Hard Round, Bullet Hole / Crater Impact (pit, raised lip, soot), Shell Scorch / Blast Star, Blood Smear / Splatter, Charred Burn Patch, Glowing Ember Cracks (emissive), Scratch / Slash, Rivet / Weld Seam.
  - Painted height becomes a normal map in the viewport and tilts shading normals in F9.
- **Mesh map baking (`src/paint/bake/`):**
  - **🔥 Bake Mesh Maps** computes Curvature, Cavity AO (rays through the 4-wide SIMD BVH), World Normal and Z-Height in UV space.
  - **1-click Smart Mask layers:** Edge Wear, Metal Chipping, Cavity Dirt, Rust, Dust, Snow, Moss, Soot, Mud Splash. Each has Amount, Contrast, Breakup, Noise Scale and Seed.
  - **💾 Export Textures** writes base colour, ORM, normal and emission PNGs, plus the baked mesh maps.
- **Rendering of painted materials:**
  - **F9:** layers are folded over the base at every shading point, so the base keeps its full detail.
  - **Viewport:** one composite (colour, ORM, normal, emission). A stroke re-composites and re-uploads only the tiles it changed. A painted Multi/Sub-Object is drawn whole.
  - **External renderer bridge:** the composite is exported, with its emission map.
- **World / Triplanar mapping (PR 5b):**
  - New **Mapping** rollout on every material and sub-material: UV Channel or World / Triplanar, with Tile Size (scene units) and Edge Blend.
  - **F9:** three projections blended by the surface normal, including relief.
  - **Viewport:** box-mapped UVs (one side per vertex), since the GPU material has one UV set. Seam-free on flat faces; the exact blend shows in F9.
  - **External renderer bridge:** box projection driven by exported projection-space positions.
  - The projection space is the object's own space at world scale, so the texture stays put when the object moves or turns.
  - The Medieval Castle, Modern City, Alpine Terrain and Settlement packs now use it by default.
- **CC0 asset downloader (PR 5b), Preferences → Assets:**
  - Each pack has a checkbox, **Download / Resume / Update**, **Cancel**, **Remove** and a progress bar. Downloads run in the background into `~/.local/share/minimax/assets/`.
  - **📷 Photo Textures (1K or 2K):** the most downloaded Poly Haven textures per shelf (ground, roads, walls, roofs, wood, stone, metal), found through Poly Haven's public API.
  - **🏠 Low-poly Models:** Kenney's CC0 starter kits from GitHub (`.glb` models).
  - **🗿 Realistic Models:** the most downloaded Poly Haven models (glTF).
  - An interrupted download shows as partial and resumes without fetching files twice.
  - **Material Library:** installed photos appear as 📷 materials on their shelves, with a **[ Procedural | Photo | Both ]** filter.
  - A bitmap on the Bump slot whose file name says it is a normal map is now used as one (it was read as a height map before).
- **Decoupled emission, per material and per sub-material:**
  - Under Emission Multiplier: **[✓] Preserve Texture Appearance (Decouple Luminance)** and **Direct Camera Brightness (0–2)**.
  - Checked: the material still lights the scene at the full multiplier (GI, emitter sampling, reflections), but camera rays, the sample sphere and the viewport show its own colours at Direct Camera Brightness.
  - External renderer: a Light Path "Is Camera Ray" mix does the same.
- **Help and credits:** the Painting page covers layers, stamps, baking and Smart Masks, with a new screenshot. The Materials page covers Mapping, the library filter and the emission toggle. Credits list TextureCraft and the asset packs; licence texts are in `licenses/`.
- **Tests (440 in total, 38 new):**
  - tiles: nothing allocated until painted, snapshots share tiles, a write copies one tile, save / load round trip;
  - layers: every blend mode, opacity, visibility, masks, Height-Blend filling low places first, emission and relief;
  - the composite matches F9 shading and updates only changed tiles;
  - strokes: projection through UVs, back-face and off-surface culling, per-stroke opacity, erase reveals the base, material brushes, all nine alphas, overlapping stamps, mask painting, blur, flood fill;
  - baking: edges convex and faces flat on a box, occlusion next to a wall, smart masks follow the shape;
  - editor: Layer 0 kept, UVs kept, shared materials copied, layer commands undo, bake / smart layers / export;
  - Triplanar weights, seamless coordinates and box UVs;
  - the downloader against a local HTTP server: pick by popularity, download, skip a missing optional map, resume, unzip, failure and cancel;
  - the emission toggle.
- **Checked in the running app:** Texture Paint on a wood box showed the base material under bullet hole, ember, blood and blast stamps, with Metal Chipping on the edges and Snow on top; the PaintBox listed the layers above "Layer 0: Base Material".
- **Not verified, or limited:**
  - **The live asset hosts were not reached.** Poly Haven and Kenney's zip host are blocked in this session, so the downloader was tested only against a local stand-in. The Poly Haven API reply format is taken from its documentation as I remember it; if a field name differs, the photo and realistic packs will fail with a clear message and need a small fix. The Kenney repositories were confirmed to exist.
  - External renderer output (Triplanar box projection, painted emission, camera-ray mix) was not run against a real external renderer; the scene file and loader script are tested.
  - TextureCraft's source could not be opened here (only its README), so the painter is written from its published design, not ported file by file.
  - In the viewport, a painted material with a Triplanar base shows the base without its relief; F9 shows it.
  - Painted glow is visible and bounces light in F9, but is not sampled as an area light unless the material's Emission Multiplier is above 0.
  - Downloaded model packs are not yet used by any generator (that is PR 6's Prop Style selector).
- **Build note:** this repository's `.cargo/config.toml` targets `znver3`. On an Intel machine, set `RUSTFLAGS="-C target-cpu=x86-64-v3"` (as this session did), or the build crashes.

## 22. Phase 11 PR A: F9 Color Grading & Camera RAW (LightCraft)

The Rendered Frame Window has a collapsible **Color Grading & Camera RAW** sidebar (`🎨 Grading` on the RFW toolbar, or Rendering → Color Grading & Camera RAW). Every control re-grades the image at once without re-rendering, for the built-in raytracer and for the external renderer images alike.

- **Tone & Presence:** Highlights, Shadows, Whites, Blacks, Clarity (local contrast against a soft blur), Vibrance, Saturation.
- **White Balance & Split Toning:** Temperature in Kelvin (6500 K neutral, higher is warmer, as in Camera RAW), Tint (green ↔ magenta), and Shadows / Midtones / Highlights colour wheels (angle = hue, distance = amount).
- **Lens & Film FX:** Vignette with Midpoint, Chromatic Aberration (radial red / blue fringing), Film Grain with a `🎲 New Grain` seed.
- Right-click resets any slider or wheel; `Reset All` and `Enable Grading` sit at the top.

How it works:
- `src/render/grading.rs` holds `Grading` (saved with the scene in `RenderSettings.post.grade`; older files load neutral) and `grade()`, which runs on the display image in this order: chromatic aberration → white balance (linear light) → clarity and tone curve → saturation / vibrance → split toning → vignette → grain.
- `FrameBuffer.base` keeps the ungraded display image. `regrade()` re-tonemaps from the stored layers when it can, then grades a fresh copy of `base`, so grading never stacks. A finished render is graded automatically.
- Saved PNG / JPG files include the grade; EXR keeps the ungraded linear light.
- UI: `src/ui/render_grading.rs`.
- Help: the Rendering page has a new section with a screenshot (`grading`). Help → Credits and `licenses/` credit LightCraft (MIT / Apache-2.0).
- 12 headless tests are in `src/render/tests_grading.rs`.

Known limits:
- The code was written from LightCraft's published feature list; its repository was not inspected file by file, and no source files were copied.
- Grading is per pixel on the CPU. A 1920×1080 image re-grades in a fraction of a second, but very large renders with Clarity on will lag a little while you drag.

## 23. Phase 11 PR B: SolidCraft CAD tools

Hard-surface CAD tools after SolidCraft (MIT / Apache-2.0).

- **Threaded fasteners** (Create → Geometry → Mech → `Bolt`, `Nut`, `Threaded Hole`): ISO Metric M3–M24 and Imperial UNC #10–1" tables (`src/mech/fastener/table.rs`) set the pitch, hex across-flats, head and nut heights. Threads are real 60° helical profiles with crest and root flats, a run-out into the shank and a chamfered tip. A nut's internal thread fits the bolt of the same size. Every parameter stays live in the Fastener Parameters rollout: standard, size, length, thread length, block size, sides, thread detail, scale and `Model Threads` (off = plain, light cylinders). Bolts are closed solids (Euler 2); nuts and hole blocks are solid tori (Euler 0).
- **Hole / Counterbore / Countersink** (Edit Poly → Polygon → Edit Polygons → `Hole` settings button): drills one round hole into the centre of every selected polygon along its normal with a manifold Boolean subtract. The caddy sets the kind, `Diameter`, `Depth` or `Through All`, `Top Diameter`, `Bore Depth`, `Sink Angle` and `Sides`. A mesh that is not closed is left unchanged, and the status bar says why. Code: `src/modifiers/cad/hole.rs`, `PolyOp::Hole`.
- **Multi-profile Loft modifier** (Modifier List → Spline → Mesh → `Loft`): sits on the path spline. `Pick Shape` adds any shape object as a cross-section at `Next Section At %`. Sections are resampled to the same point count, lined up (winding and start point) and blended with smoothstep along the path on parallel-transport frames. Further controls are `Twist`, `Scale Start` / `Scale End` with `Scale Curve`, `Cap Start` / `Cap End`, `Flip Normals`, and `Path Steps` / `Shape Steps`. Each section's % can be edited in the list, and `Remove Section` deletes the highlighted one. With no sections, a 0.2 circle is used. Code: `src/modifiers/cad/loft.rs`, `src/editor/cad.rs`, `src/ui/rollouts/cad.rs`.
- **Spline constraints** (Editable Spline Geometry rollout, and Tools → Spline Constraints): `Parallel`, `Perpendicular` and `Equal Length` at Segment level; `Tangent` at Vertex level (the curve leaves the vertex in the direction it arrived, using Bezier Corner handles); `Concentric` at Spline level. They are one-shot actions. The first selected item in spline order is the reference, and the rest move once to match it. A segment that shares a vertex with the reference pivots on that vertex, so the reference never moves. Code: `src/splines/constraints.rs`, `Action::Constrain`.
- Help: there is a new page, "13. CAD tools", with a screenshot (`cad`). Help → Credits and `licenses/` (`SolidCraft-LICENSE-MIT.txt`, `SolidCraft-NOTICE.txt`) credit SolidCraft.
- Tests:
  - fasteners: 4 tests in `src/mech/tests.rs`;
  - Loft and Hole: 6 tests in `src/modifiers/cad/tests.rs`, covering counts, Euler characteristic, volume, a hole removing material, a through hole adding a handle, and the op failing cleanly with nothing selected;
  - constraints: 5 tests in `src/splines/tests_constraints.rs`.

Known limits:
- The tools were written from SolidCraft's published feature set; no source files were copied. SolidCraft's B-rep kernel and parametric timeline were not ported: these work on MiniMax's polygon meshes and splines.
- Constraints are one-shot, not a live solver. Moving a segment later does not keep it parallel.
- Holes drill straight along each polygon's normal from its centroid. There is no click-to-place point yet, and each hole is a separate Boolean (dozens of holes on a dense mesh take a moment).
- Loft sections are taken flat from each shape's own XY plane, about its pivot.

## 24. Phase 12: Black-box flight recorder, crash reporter and emergency scene recovery

The code is in `src/diagnostics/` and `src/ui/crash_dialog.rs`.

- **Readable backtraces in release:** `[profile.release]` now uses `debug = "line-tables-only"` and `strip = false`, so backtraces keep `file:line`. `build.rs` stamps the git commit (`MINIMAX_GIT_SHA`) into the build.
- **Flight recorder** (`breadcrumbs.rs`): a ring buffer of the last 50 events, each timed in seconds since start. It records:
  - every `Action` run from a button or menu;
  - every hotkey (chord and action);
  - every Edit Poly op;
  - every added modifier;
  - every stack evaluation, with identical repeats folded into one `×n` line.
  The UI's `Observer` (`state.rs`) adds tool changes, sub-object level changes, caddy opens and dialog opens. It also keeps a Markdown summary of the scene and selection current (every 0.5 s), and a scene snapshot for the emergency save (on each new undo step, or every 5 s). The snapshot is cheap because meshes are `Arc`-shared.
- **Crash triggers** (`hook.rs`):
  - **Panic hook:** captures the message, `file:line:col`, the thread name and a forced `Backtrace`. It then calls the previous hook, so the usual message still prints.
  - **Unix signal handlers** (`SIGSEGV`, `SIGBUS`, `SIGILL`, `SIGFPE`, `SIGABRT`): these write the same report from a fresh reporter thread. That thread is needed because a fault may arrive on an exhausted stack. The handler then restores the previous handler. A real fault simply happens again; a signal that was sent (by `kill` or `abort`) is raised again. Either way the process ends as it would have (exit 139 for a segfault).
  - Only the first crash is written.
  - Every lock is taken with `try_lock`, so a crash while the recorder is busy still produces a report, with that section marked as unavailable.
- **Report** (`report.rs`): written to `~/.local/share/minimax/crashes/crash_<YYYYMMDD-HHMMSS>.md`. A header gives the version, git sha, OS, kernel and GPU adapter (from `RenderAdapterInfo`). The sections are:
  1. Crash Summary & Location
  2. Last 50 User Actions
  3. Active Scene & Selection State: object count, selection, sub-object level, tool, caddy, tab, the modifier stack with the active level marked, mesh counts, the last undo step and the scene file
  4. MiniMax Stack Frames: only `minimax::` frames, excluding the reporter itself
  5. Full Backtrace
- **Emergency save and next-launch detection** (`recovery.rs`): writes `crash_recovery.mmax` next to the report, plus a `.pending` marker. On the next start, `detect_pending` shows the **MiniMax closed unexpectedly** dialog, with:
  - `Copy Crash Report for Claude`
  - `Restore Unsaved Scene (crash_recovery.mmax)` (it opens unsaved)
  - `Open Crash Reports Folder` (xdg-open)
  - `Dismiss / Clear` (this keeps the files)
- **Help → View Crash Reports...** (`Action::CrashReports`) opens a window listing the reports, newest first, with a preview. From there you can copy a report, restore the scene, open the folder, use `Simulate Test Crash Report` (writes a real report and emergency save without crashing) or `Delete All Reports`.
- **Help:** there is a new page, "16. Crash reports and recovery", with a screenshot (`crash`).
- **Tests:** 8 tests in `src/diagnostics/tests.rs`:
  - the ring buffer (cap of 50, oldest dropped, repeat folding);
  - UTC date formatting;
  - backtrace frame filtering;
  - the five report sections, in order and with balanced fences;
  - the bundle writing a loadable `.mmax` without overwriting a report from the same second;
  - pending detection, dismiss and clear;
  - the observer's change crumbs;
  - the scene summary.
- **Checked by hand:** I ran the app under Xvfb and sent `SIGSEGV`. The report and emergency scene were written, the process exited with 139, and the next launch showed the dialog.

Known limits:
- After a fatal signal (rather than a panic), the crashing thread's backtrace is not available. The report says so, and the breadcrumbs and scene state are still there.
- The emergency scene is the last snapshot. It can be up to 5 s behind an edit that is still being dragged.
- The signal handlers are Unix-only. On other systems only panics are recorded.

## 25. Phase 11.5A: Native GPU/CPU path tracer (external renderer bridge removed)

**Removed:** the external renderer bridge: `blender.rs`, `blender_loader.py`, `blender_tex.rs`, `cycles.rs`, its tests, the F10 external-renderer rollout, and the Preferences → Rendering external renderer path. Scenes saved with `engine: the external renderer` still load. Serde ignores the old fields, and they render with the native path tracer. A legacy-RON test covers this.

**F10 Render Setup:**
- `Renderer`: Path Tracer (default) or Classic Raytracer, which is the old fast tracer, unchanged.
- `Compute Device`: Auto (wgpu GPU with CPU fallback), GPU (wgpu Vulkan Compute) or CPU (SIMD Multi-Thread).
- `Stream Full-Frame Progressive Passes` (1 → 2 → 4 → … → final spp).
- Samples, `Max Bounces` (1..8), and separate Diffuse / Glossy / Transmission bounce limits.
- `Russian Roulette After` (default 2).
- `Adaptive Sampling` with `Noise Threshold` (0.002..0.10) and `Min Samples` (4..32). All spinners reset with a right-click.
- Preferences → Rendering sets the default Renderer and Compute Device. The Frame Window progress bar shows `Pass n/N · spp · % sampling`.

**CPU path tracer (`src/render/pathtrace/`):**
- `bsdf.rs`: Lambert + GGX with Heitz visible-normal sampling and separable Smith G, plus a smooth dielectric for glass and crystal. It uses SolidCraft's lobe mix, `0.5 (1 - metallic)`.
- `integrator.rs`:
  - Iterative paths with NEE on punctual lights.
  - One MIS-weighted sample per bounce on emissive polygons. Emission found by BSDF sampling gets the matching power-heuristic weight, so the classic tracer's firefly clamps are not needed here.
  - Stochastic opacity, fog, and Beer-Lambert crystal absorption.
- `adaptive.rs`: Welford statistics with a 3×3 worst-neighbour relative error.
- `progressive.rs`: row-parallel full-frame passes, with SplitMix-seeded streams.
- `src/render/mis.rs`: power and balance heuristics, plus area-to-solid-angle conversion.

**GPU path tracer (`src/render/gpu_pathtrace/`):**
- A WGSL compute port of the same integrator (`kernel.wgsl`).
- Triangles, the binary BVH, lights and the emitter CDF go up as storage buffers, with stack traversal.
- Each dispatch traces at most 4 spp, so the driver watchdog never fires. RGB/coverage and luminance sums are read back each pass for the adaptive mask.
- The G-buffer and background come from one CPU center ray, so outlines and the denoiser still work.
- Without an adapter, or for HDRI/procedural skies, opacity, toon, texture/procedural maps or decoupled emission, it falls back to the CPU and the status line says why.

**Fix:** hit points are now interpolated barycentrically instead of computed as `o + d t`. Orthographic rays start 2500 units out, and the lost precision made bounce rays re-hit their own surface. This helps the classic tracer too.

**Tests (now 483):**
- White furnace.
- More bounces add interreflected light.
- Russian Roulette leaves the mean unchanged.
- MIS agrees with pure BSDF sampling on an area light.
- Adaptive sampling stops early.
- BSDF sampled albedo equals the integrated albedo.
- Glass tint.
- MIS weights, Welford statistics and convergence.
- Legacy external-renderer scenes load.
- **GPU/CPU parity:** on llvmpipe Vulkan, image energy is 57.23 vs 57.26, with every 6×6 block matching. The test skips without an adapter; `MINIMAX_REQUIRE_GPU=1` makes that a failure.
- CPU fallback.

**Not done in 11.5A:**
- The hardware ray-query path. The kernel always uses software BVH traversal; wgpu's ray-query feature isn't used yet.
- Per-texel textures on the GPU. Textured scenes go to the CPU.

Credit: path-tracing design after SolidCraft (MIT / Apache-2.0).

## 26. Phase 11.5B: Volumetrics, Principled BSDF, area lights and IES

**1. Volumetric godrays and 3D height mist** (`src/render/volumetrics.rs`, `src/ui/volume_dialog.rs`, `src/viewport/volume_sync.rs`):
- The `8` Environment dialog has a **Volumetric Godrays & Mist** rollout:
  - Enable, Godray Intensity, Scattering Anisotropy g, Volumetric Steps and Fog Scatter Color.
  - Mist Base Height, Height Falloff, and 3D Noise Wisps (density, scale, wind drift).
  - Presets: Cathedral Shafts, Valley Mist, Dusty Hangar.
- F9 (both renderers) ray-marches the medium with Henyey-Greenstein single scattering and shadow rays to every light, so geometry blocks the shafts.
- The Perspective viewport shows live shafts through Bevy's `VolumetricFog` / `VolumetricLight` and a mist `FogVolume` fitted to the scene.

**2. Principled BSDF** (`src/materials/principled.rs`, `src/render/bsdf/`, `src/render/pathtrace/subsurface.rs`):
- A new **Principled BSDF** rollout on every Physical and Crystal material.
- Subsurface Scattering: Weight, Scatter Color, per-channel Radius and Scale. Rendered as a random walk: exponential free flights, isotropic scattering, van de Hulst albedo inversion and Russian Roulette.
- Anisotropic GGX with rotation and Heitz visible-normal sampling.
- Clearcoat with its own roughness, layered over the base with Fresnel energy loss.
- Cloth Sheen with Tint.
- Glass and crystal: Beer-Lambert Absorption Color and Density, and Dispersion as an Abbe number. Dispersion uses Cauchy per-channel IOR, and each path follows one channel.
- One-click looks: Skin, Marble / Jade, Car Paint, Brushed Metal, Velvet, Prism Glass.
- The viewport previews clearcoat, anisotropy and diffuse transmission through Bevy's StandardMaterial.

**3. Analytic area lights and IES profiles** (`src/render/area_lights.rs`):
- Create → Lights gains **Rect Light**, **Disc Light**, **Sphere Light** and **Tube Light**, with an Area Light Shape rollout and a **Visible to Camera** toggle.
- Each light sample picks a point on the facing part of the shape, so shadows soften with size.
- A multiplier-1 area light gives the same on-axis power as a multiplier-1 Omni.
- Spot and area lights take an **IES Light Profile**: None, Architectural Wall Scallop, Streetlamp Wide Throw, Double-Ring Tactical Flashlight, Industrial High-Bay, Stage Narrow Beam.
- The viewport approximates Rect / Disc as wide spots and Sphere / Tube as soft point lights.

**GPU fallback:** volumes, Principled lobes, area lights and IES profiles render on the CPU path tracer. The status line says why.

**Tests (now 502):**
- HG phase normalisation and sampling.
- Mist falloff and wisps; godray occlusion.
- Principled lobes sample their own density and never add energy.
- SSS moves diffuse energy into the walk; thin slabs pass light while thick ones return it.
- Anisotropy stretches the highlight; dispersion orders the channels; Beer-Lambert absorption.
- Area lights match an Omni far away; panels are one-sided; bigger lights cast softer shadows.
- Area shapes intersect rays; Visible to Camera works.
- IES profile shapes.
- The viewport previews Principled lobes.

**Not done in 11.5B:**
- The GPU kernel does not have these features yet.
- IES profiles are analytic curves. Loading `.ies` files is not supported.
- The viewport does not apply IES shaping.

## 27. Phase 11.5C: LightMix, A-Trous denoiser, AOVs, lens FX, NPR ink, shadow catcher, turntable

**1. LightMix: relight a finished render** (`src/render/lightmix.rs`, `src/editor/lightmix.rs`, `src/ui/render_lightmix.rs`):
- The CPU path tracer keeps one radiance buffer per light group:
  - Group 0: Sun & Dynamic Sky. Directional lights named "Sun…" also go here.
  - Group 1: Environment / HDRI.
  - Group 2: Emissive materials.
  - Group 3: Volumetric godrays & mist.
  - Groups 4–11: scene lights, one each. Lights after the eighth share group 11.
- The F9 **💡 LightMix** sidebar has On, Solo, Intensity (0–10×, log scale, right-click resets) and Tint for each group.
- Every change recomposites `Σ group × intensity × tint` and reruns the denoiser, outlines, bloom, tone and grade. There is no re-render.
- **Apply LightMix to Scene Lights** bakes the faders into each light's Multiplier and Color. It is undoable.

**2. Albedo-demodulated A-Trous denoiser + AOV passes** (`src/render/denoise_atrous.rs`, `src/render/aov.rs`, after Strolle, MIT):
- Radiance is divided by the camera-hit albedo.
- Five edge-avoiding B3-spline passes (1, 2, 4, 8, 16 px) are weighted by world normal, linear depth, roughness, object / material ID and variance-scaled luminance.
- The crisp albedo is then multiplied back, so textures and painted detail stay sharp at 4–8 spp.
- The Classic raytracer keeps the bilateral filter.
- The F9 toolbar has a pass selector next to R/G/B/A: Beauty, Direct, Indirect GI, Volumetric, Specular, Subsurface, Albedo, World Normal, Linear Depth, Cavity / AO, and the Adaptive Sample Count heatmap.

**3. Lens FX** (`src/render/lens_fx.rs`, F10 **Lens Effects** rollout, after EffectCraft, MIT / Apache-2.0):
- Aperture: Circular or 5–9 blades, with Blade Rotation. The blades shape the depth-of-field bokeh taps.
- Cat's-Eye Vignetting clips off-centre bokeh.
- Anamorphic Streak Flare: a horizontal, tinted streak.
- 4 / 6 / 8-point Starburst glare on highlights above the Flare Threshold.

**4. NPR ink and screentone** (`src/render/outline.rs`, `src/render/npr.rs`):
- **Interior Pen Lines** on the Ink Outline rollout draw:
  - soft-curvature folds, where normals two pixels apart turn by more than half the crease angle;
  - material seams inside one object, such as Panel Groove floors.
- Toon materials get **Manga Halftone Shadows**: the shadow band becomes a 45° dot grid that grows with darkness. **Screentone Cell** sets the dot size.

**5. Shadow & Reflection Catcher** (`src/render/pathtrace/catcher.rs`):
- Turn it on in the Display tab (per object; an Action) or in the Material Editor (per material).
- At a camera hit, the path tracer sets alpha from three things:
  - the shadow fraction: shadowed vs. open light from every lamp;
  - contact AO;
  - a Fresnel × gloss reflection trace.
- Reflections are premultiplied radiance, so the render composites over any backplate.

**6. Turntable & isometric sprite sheet** (`src/render/turntable.rs`, `src/editor/turntable.rs`, `src/ui/render_sheet.rs`):
- **🎞 Render Turntable / Sprite Sheet…** is in F10, on the F9 toolbar, and in Rendering ▸ menu.
- Turntable: 8–72 frames orbiting 360° around the selection (or the scene), saved as a numbered PNG sequence.
- Sprites: 4-Dir or 8-Dir (N, NE, E, SE, S, SW, W, NW), orthographic, at 30° / 35.264° / 45° pitch.
  - Transparent background and Ink Outline by default.
  - Packed into one atlas PNG plus a TexturePacker-style `.json`.
- It renders in the background with the F10 renderer and shows a progress bar and a Cancel button.

**GPU fallback:** shadow catchers render on the CPU path tracer. LightMix buffers and AOVs come from the CPU path tracer only. A GPU render shows no LightMix sidebar and no AOV passes.

**Tests (now 523):**
- LightMix: the identity mix matches the beauty render; faders relight; Apply bakes into the lights.
- AOVs: direct + indirect equals the beauty render; each pass shows its own channel.
- A-Trous: noise drops by more than 10×, checker edges keep their albedo, and nothing bleeds across objects.
- Bokeh: the polygon kernel has flat blades; cat's-eye clips off-centre bokeh.
- Glare: the anamorphic streak and the starburst ray counts.
- Ink: pen lines on folds and groove seams; halftone dot coverage.
- Shadow catcher: alpha in a scene; the alpha maths.
- Turntable / sprites: 8-direction layout and atlas JSON; turntable orbit; PNGs written to disk by the render and by the editor job.

**Not done in 11.5C:**
- The GPU kernel has no LightMix or AOV buffers.
- The turntable writes PNG sequences only, not GIF or WebM.
- Sprite sheets render one pose. Animation-cycle phases are not supported yet.
- The Classic raytracer renders catchers as normal surfaces.

## 28. PR 6: Buildings & Props spawner + Settlement variety

**1. Standalone Buildings & Props** (`src/generators/buildings/`, Create ▸ Geometry ▸ Buildings & Props, the Create menu, Tools ▸ **Buildings & Props Browser...**, the 🏠 toolbar button and the ribbon's *Populate* group):
- 43 types, each its own object. Click a type, then drag the footprint in a viewport. Right-click a type to create it at the origin.
  - Modern (14): Glass Skyscraper, Office Block, Brick Tenement, Apartment Block, Shop with Awning, Gas Station, Warehouse, Sawtooth Factory, Parking Garage, School, Hospital, Modern Church, Water Tower, Street Kiosk.
  - Medieval (12): Castle Keep, Guild Hall, Market Hall, Half-Timber Tavern, Stone Chapel, Windmill, Watermill, Blacksmith Forge, Stable, Granary, Gatehouse, Well House.
  - Bridges & infrastructure (5): Stone Arch Bridge, Steel Truss Bridge, Drawbridge, Quay / Pier, Crane.
  - Doodads & props (12): Streetlamp, Lantern Post, Car / Truck, Cart / Wagon, Market Stall, Barrel / Crate Cluster, Dumpster, Sandbag Wall, Tank Trap, Barbed Wire Coil, Park Bench, Fountain / Well.
- The browser window has category filters, a search box and a flat-shaded 3/4 thumbnail of every type.
- **Building Parameters** rollout on the Modify tab:
  - Type (grouped by category), Seed and 🎲, Width, Length, Floors (relabelled per type: Arches, Boom Extra, Courses).
  - Part toggles: Balconies, Fire Escapes, Awnings, Chimneys, Dormers, Shutters, Rooftop Clutter (AC units, tanks), Signs, Stairs & Porches.
  - `[✓] Emissive Lights`, Stairs (straight / L-shaped / spiral) and Material Style (Procedural | Photo). Changing the style assigns the matching pack.
- **Parametric architectural families** (`arch.rs`): walls along a path with mitred corners; door and window openings that frame themselves and leave their own hole (built as wall pieces around the opening, so no Boolean pass is needed); straight, L-shaped and spiral staircases; cross-gable roofs for L / T plans.
  - The prompt named BIMCraft as a source. **BIMCraft was not inspected.** These algorithms are written natively for this project, and no BIMCraft code is used.

**2. Settlement settlement expansion** (`src/generators/settlement/`):
- 14 presets. The original four (Grid Downtown, Suburbs, Round Walled City, Village) are joined by ten new ones:
  - River City: a river with quays, lamps and trees, alternating truss and arch bridges, and a modern grid.
  - Harbour Town: a sea band, quay wall, piers, cranes on rails, warehouse frontage and an inland grid.
  - Industrial District: sawtooth factories and warehouses, a rail yard band, tanks and brick chimneys.
  - Old Town + Modern Sprawl: a medieval core with a plaza, Market Hall and radial cobble lanes inside a ring road, with the modern grid around it.
  - Square Walled City: a square curtain wall with corner towers and four gatehouses, a cobble cross, a lane grid and a market square.
  - River Town: a river with embankments, stone arch bridges, watermills and landing stages, and bank lanes.
  - Castle & Town: a hill with the Castle Keep and a towered ring wall, a winding castle road and the town below.
  - Star Fort: five bastions, a moat, a gatehouse and drawbridge, a parade ground and barracks.
  - Crossroads Hamlet: two crossing roads with an inn, forge, stable, well house and granary on the corners, cottages, fenced crop fields and a windmill.
  - Monastery: a precinct wall, cloister garth and arcade, chapel, dormitory, granary, stable, kitchen gardens and an orchard.
- New toggles on the Settlement rollout (base object and modifier):
  - **Buildings**: one checkbox per palette type; unchecked types are left out.
  - **Building Parts**: the same part toggles as the standalone buildings.
  - **Town Features**: River / Sea, Bridges, Docks / Quays, Town Walls, Moat, Rail Yard, Market Square, Trees / Parks, Farm Plots.
  - `[✓] Emissive Lights` master toggle. When it is off, lit windows, lamps and neon become plain glass.
  - **Prop Style**: Generated (the preset's own props), Low-poly pack (box stand-ins) or Realistic pack (the detailed doodads along every street).
- **24-ID Settlement Material Pack** (`src/materials/ghost_pack.rs`): IDs 13–24 add Asphalt, Lane Lines, Sidewalk Slabs, Curbs, Cobbles, River Water, Brick, Stucco, Slate / Metal Roof, Rusted Metal, Broken Glass and Neon / Lantern Glow. They use PR 5.5 MaterialCraft textures with Triplanar mapping. **Assign Photo Pack** is the photo-style variant. The rollout shows a collapsible legend of all 24 IDs.

**Tests (now 535):**
- All 43 building types build valid, deterministic geometry that stays near its footprint, uses IDs 1–24 and has a browser thumbnail.
- Buildings: Emissive Lights off turns every light off; parameters drive the geometry; mitred walls share corners; openings cut framed holes; staircases climb their rise; Photo style picks the photo pack.
- The drag sets a building's footprint and assigns its pack.
- Every one of the 14 presets builds valid, deterministic geometry inside its area, with streets, roofs, walls, ground and trees.
- Emissive Lights off removes lit and neon IDs. River off removes water. Unchecked building types change the town. Low-poly and Realistic props add geometry. Town Walls off removes walls.
- Damaged mode stays period-accurate on all 14 presets.

**Not done in PR 6:**
- Prop Style "Realistic" reuses the standalone doodads; there is no imported asset pack.
- Castle & Town's houses on the hillside stand on flat footings, without stepped foundations.
- River and harbour water is a flat surface (Material ID 18), not the Ocean generator.

## 29. PR 7: 5-state building destruction, rubble mounds & battlefield textures

**1. Destruction levels** (`src/generators/ruin/`). Six states, offered on every building (Buildings & Props), on the Settlement settlement (base object and modifier) and in the Battlefield & Ruin modifier's new **Building Destruction** section:
- **Intact**: untouched.
- **Light Damage**: cracked or missing glass with shards below, soot above the windows, bullet-scar decals and a little debris. Lit windows go dark when their glass breaks.
- **Heavy Damage**: a blown-out corner (the mesh is sliced and the corner removed), shell holes, exposed slabs with rebar or joists, a caved-in roof (sagging slab or broken rafters), a slight lean, and barricades (sandbags and tank traps for Modern, carts and barrels for Medieval).
- **Warzone Collapse**: the failed storey gives way. The upper block either topples (it is detached, rotated about the edge it falls toward and laid on the ground) or pancakes into stacked slabs. A rubble mound forms underneath.
- **Apocalyptic Ruin**: a jagged stump of the lower floors (cut by three tilted planes), with broken rebar and I-beams, charred timbers or moss along the top, over a large mound.
- **Pure Rubble**: nothing stands; only the mound is left.

In a town or on the battlefield, each building gets the chosen level or one step either side (seeded per building), so a street has a mix of states. Bridges, quays and street props are never wrecked.

**2. Load-path stress model** (`ruin/stress.rs`). Blasts are scattered over the building's columns, and each blast takes capacity from the columns near it. Lower storeys carry more load. The lowest storey whose remaining capacity cannot carry the floors above it fails. The model also gives the building a lean or fall direction toward the side that lost the most support, and an asymmetry value that decides between toppling and pancaking. A block can only topple if it is taller than it is deep along the fall; otherwise it pancakes.
- The prompt named FieldCraft as a source. **FieldCraft was not inspected.** The stress, fracture and collapse math is written natively for this project, and no FieldCraft code is used. It is a simple heuristic, not a physics or finite-element solver.

**3. Rubble mounds** (`ruin/mound.rs`):
- A layered dust heap (ID 27) with a rough, noisy edge, plus ash patches (ID 26).
- Era debris on the surface:
  - **Modern**: concrete chunks, bricks, rubble clusters and girders.
  - **Medieval**: stone blocks, roof tiles and charred timbers.
  - **Ancient**: big weathered stones, column drums and moss.
  - Medieval and Ancient never get rebar or girders.
- `[✓] Glowing Embers` (ID 25) in the crevices, with an **Ember Glow** spinner and an `[ Apply Glow ]` button that sets the object's own ID 25 emission.
- `[✓] Gore / Blood Smears` (ID 28) with an **Amount** slider: blood pools and drag smears on the rubble and the ground.
- **Rubble Pile** is a new standalone Buildings & Props type (now 44 types). Width and Length set the spread, and Pile Height sets the height.

**4. Battlefield textures.** The Settlement Material Pack grows to **28 IDs**: 25 Glowing Embers (emissive), 26 Bullet Scars / Soot, 27 Rubble Dust / Ash, 28 Blood Smears. Objects that still carry an older 24-ID pack get ID 25 added when the Ember Glow is applied.

**5. Battlefield & Ruin modifier.** The modifier now finds each building in its input mesh (connected elements whose footprints overlap, ignoring flat ground) and wrecks each one with the Building Destruction settings, before its existing blast-zone pass.

**Tests (now 544):**
- Every level on five building types builds valid, deterministic meshes with UVs, using IDs within the pack.
- Intact leaves the building unchanged. The height drops with the level, Light adds scars and Heavy adds rubble.
- Debris is period-accurate, including the Ancient override.
- The Embers and Gore toggles add or remove their IDs.
- The Rubble Pile follows its spread, height and density.
- The stress model fails lower storeys harder.
- Wrecked towns are lower, have rubble and are deterministic, and walled towns get no rebar.
- The Battlefield modifier wrecks each building.

**Not done in PR 7:**
- Destruction is generated geometry; there is no rigid-body simulation or animation of the fall.
- Wrecked buildings are not split into separate objects; a town stays one mesh.
- Embers glow through the material's emission only; there are no particle sparks or smoke yet (smoke is planned for PR 8).

## 30. PR 8: Procedural smoke & fire plumes + dynamic weather & storm clouds

**1. Smoke Plume generator** (`src/generators/smoke/`; Create > Worlds > Smoke Plume, or the Worlds menu). Drag out the source radius, then the height.
- **Shape:** the plume is a stack of noise-displaced lobe shells (soft, metaball-like puffs). Alpha-eroded **Turbulence Cards** (fans whose rim fades to zero alpha) and a **Mushroom Cap** ring vortex at the top can be turned on or off. Density is stored in the vertex alpha and the tint in the vertex colour. The new **Battlefield Smoke** material (Settlement ID 29) and the **Smoke & Fire Pack** (Multi/Sub-Object preset) are translucent, so plumes cast soft partial shadows in F9.
- **Styles:** Oily Battlefield Smoke (black, thick), Chimney Smoke (grey) and Steam (white, thin).
- **Parameters:** Seed, Plume Height, Base Radius, Top Spread, Billow Turbulence, Wind Lean Angle and Direction, Lobes, Time / Rise Phase (scrubs the puffs up the column and loops) and **Fire Base Intensity %**. The fire setting adds glowing flame tongues and a burning base (emissive Fire Core / Embers) with an orange under-glow on the smoke above. At 0% the plume is smoke only.
- **Plume physics** (`smoke/column.rs`) is written natively from classic closed-form plume theory:
  - **Entrainment** (Morton-Taylor-Turner): the radius grows almost linearly with height.
  - **Buoyant slowdown:** vertical speed falls as z^(-1/3), so puffs bunch up into the cap (z ~ t^(3/4)).
  - **Briggs bent-over rise:** in wind the drift grows as z^(3/2).
  - **Karman vortex shedding:** a Strouhal-0.2 side-to-side sway that travels up with the Rise Phase.
- **FieldCraft was not inspected.** This is not a CFD, LBM or particle-simulation solver, and no external fluid code is used. The plume is static geometry shaped by those laws.

**2. Battlefield auto-spawn.** `[✓] Spawn Rising Smoke Plumes`, a **Smoke Plume Count** and a **Plume Look** sub-rollout were added to the Settlement Damaged / Battlefield mode and to the Battlefield & Ruin modifier. Plumes rise from the tallest wrecks (a coarse height grid) and from the user's blast zones, standing on the local ground. The defaults depend on the era: Modern uses black battlefield smoke; Medieval and Ancient use grey chimney smoke, with less fire for Ancient. The Settlement pack grows to **29 IDs**.

**3. Weather & Storm Clouds** (`src/render/weather.rs`; the `8` dialog, Dynamic Sky, then the **Weather & Storm Clouds** rollout):
- **Weather Preset** dropdown: Clear Sky, Partly Cloudy, Heavy Overcast, Dark Gathering Storm, Thunderstorm, Battlefield Ash & Smoke Haze, Heavy Fog / Mist, Rainstorm and Blizzard / Snow. A preset sets the cloud coverage, density and altitude, the thickness, the precipitation and the exponential fog in one click.
- **Cloud Thickness % (optical depth):** cloud bases darken by Beer-Lambert extinction, `T = exp(-tau * depth)`, toward a slate storm grey. Looking straight up at the base shows the most attenuation.
- **Overcast Sun Dimming:** the linked main sun's Multiplier is attenuated through the deck, and its shadow softness grows as the sun dims.
- **Lightning Flash:** a seeded, jagged bolt below the deck plus a broad flash that lights the surrounding clouds. It is baked into the sky, so it also lights reflections and the sky ambient.
- **Precipitation:** Rain Streaks, Snowflakes, or Ash & Glowing Cinders, with Amount, Wind Slant and Seed.
  - **Viewport:** animated overlay lines in a box around the scene (`src/viewport/weather_lines.rs`).
  - **F9:** a depth-aware overlay (`src/render/precip.rs`). Particles sit at seeded depths, nearer ones are bigger, and each is drawn only where it is in front of the geometry. Cinders are emissive, so Bloom picks them up.

**Tests (now 559):**
- **Plumes:** they are valid, deterministic and tall, and widen with height. Fire sits at the base, and the smoke is translucent with zero-alpha card rims. Height, wind, cards, cap, phase and style all change the plume. Town plumes use the Settlement IDs.
- **Plume physics:** the column widens, slows and bends downwind, and the vortex sway alternates and travels with the phase.
- **Battlefield:** the modifier spawns rising plumes.
- **Weather:**
  - Every preset applies.
  - Beer-Lambert darkening is monotonic, and storm skies are less than half as bright overhead.
  - Sun dimming lowers and softens the sun, and turning it off restores the clear sun.
  - Lightning brightens the storm.
  - Old scenes load without weather settings.
- **Precipitation:** rain, snow and ash are drawn but hidden behind near geometry. Rain brightens, snow whitens and ash darkens, cinders glow, and Amount scales the count. The viewport particles fall inside their box.

**Not done in PR 8:**
- Smoke is static meshes with vertex-alpha density, not a volumetric (voxel) medium. There is no fluid simulation.
- Precipitation in F9 is a 2D overlay at seeded depths. Drops do not hit, splash on or wet surfaces, and snow does not settle.
- Lightning is a single static bolt per seed and does not animate. The bolt is drawn in the sky and is not an actual light source in the scene.

## 31. Phase 10A: MakeHuman Character & Creature Studio

**Assets** (`assets/makehuman/`, CC0 1.0): the MakeHuman **hm08** base mesh, the full morph-target set (packed into one zlib-compressed `MHT1` file and decoded at 1/3000 precision), the modeling-modifier list, the high- and low-poly eyes with their `.mhclo` fitting, the default skeleton and skin weights. Everything is embedded with `include_bytes!` and decompressed once on first use, so characters work offline.

**Loader** (`src/character/makehuman/`):
- `obj.rs` parses the base mesh (19,158 vertices, 18,486 faces, 172 groups) and masks the body from the helper geometry (tights, skirt, hair and joint helpers).
- `target.rs` reads `.target` morphs (ASCII and the packed format) and applies them.
- `macros.rs` blends the macro targets (gender, age, muscle, weight, height, proportions, breast size and firmness, African / Asian / Caucasian) with MakeHuman's own weighting.
- `modifiers.rs` turns `modeling_modifiers.json` into the Detail Sliders (one- and two-sided).
- `mhclo.rs` fits `.mhclo` proxies (barycentric bindings, offsets, scales); the eyes use it.

**Character base object** (`BaseObject::Character`, `src/character/`):
- **Archetypes:** Human, Elf, Dwarf, Goblin, Orc, Chibi, Anime Hero and Anime Heroine. Each is a macro and micro recipe, for example pointed ears, a broad brow or a short stature.
- **Anime Ratio:** grows the head and eyes and slims the body; toon normals soften the face.
- **Built from Kit pieces:** procedural hair (Crop, Long, Ponytail, Spiky, Bun, Beard, with Hair Length), outfits (Bodysuit and Tunic use the MakeHuman tights and skirt helpers; Knight, Ranger and Robe add armour, belts, a cape or a robe), and weapons (Sword, Axe, Staff, Shield).
- **Creatures:** six creature presets (Wolf, Dragon, Spider, Slime, Golem, Serpent) with Size and Seed.
- **Materials:** IDs 1 Skin, 2 Eyes, 3 Hair, 4 Cloth, 5 Metal and 6 Leather, served by the new **Character Pack** Multi/Sub-Object preset (skin with subsurface scattering). Feet stand on Z = 0.

**UI:**
- **♀♂ Characters** toolbar button and Tools > **Character & Creature Studio...** (a gallery of archetypes and creatures, plus the parameters of the selected character).
- A Create-tab rollout and the Modify-panel rollouts: Archetype, Body (MakeHuman Macros), Eyes / Hair / Outfit / Weapon, and Detail Sliders with Search and Mirror L/R.
- Every change is one undo step.

**Tests:** 13 new (574 in all), all passing:
- base mesh counts and the body / helper split;
- packed and ASCII targets decode the same;
- targets apply, and macro weights sum correctly;
- slider keys are unique;
- the eyes fit into the head;
- every archetype builds a valid mesh standing on the ground;
- outfits and hair add Material IDs;
- creatures build;
- an undo test.

**Not done in 10A:**
- ~~The **face cloner** is not started.~~ Part done in §35 (PR A): load photos or a folder of frames exported from a video (video files are not read directly; export frames first), MediaPipe 478-point landmarks with depth, blur / duplicate / expression filtering and best-N frames per view, front / three-quarter / profile sorting, a 2D pin editor, a multi-view fit of the MakeHuman head sliders with render-and-check refinement, and an RBF fine warp that fades out above the neck. Hair and background masks, a de-lit 360° skin texture, skin-tone match and saved face presets are PR B.
- ~~**Real MakeHuman hair and clothes assets** are not bundled.~~ Done in §34: the CC0 system asset pack (hair, clothes, eyebrows, eyelashes, teeth, tongue, proxies, skins, eye colours) is bundled and fitted with `.mhclo`. The procedural Kit hair and outfits remain as an option.
- **Height and proportions** use only the young-adult targets, so they are approximate for child and old ages.
- ~~**Placement:** characters are placed at the origin; drag-to-create is not wired.~~ Done after 10B: a gallery tile arms a Create Character tool. Click in a viewport to place the character, or drag to set its height like a Biped (a uniform scale, with no mesh rebuild while dragging). **Place ... at Origin** keeps the one-click route.
- **Previews:** the Studio gallery shows text buttons, not rendered thumbnails.
- The skeleton and weights are bundled; rigging and Easy Pose arrived in Phase 10B (§33).
- **Not tested by hand:** none of this has been tried in the running app, only by unit tests.

## 32. Ilex / Veach / Luxon / Demiurge rebrand

**Application → Ilex.**
- **Names:** the crate and binary are now `ilex`. The window title is "Ilex", the About dialog says "Ilex 3D", and the menu bar, welcome status, tour, help, crash reports and exporter headers (OBJ, glTF, FBX) all say Ilex.
- **Logo:** a white Common Holly tree (*Ilex aquifolium*) with spiny leaf-edged tiers and cut-out berries, on a blue-green (`#127A72`) tile (`packaging/ilex.svg`).

**Backward compatibility.** Old files and setups keep working:
- **Scenes:** new scenes save as `.ilex` with a `format: "Ilex"` header. Open, the command line and autosave also accept `.mmax` files and the `"MiniMax"` header.
  - Saving a scene opened from a `.mmax` file writes a `.ilex` beside it and leaves the original alone.
  - The Open dialog lists both extensions together.
- **Poses:** `.ilexpose` and `.mmaxpose` are both recognised (`scene_file::is_pose_path`), ready for Easy Pose in 10B.
- **Folders:** preferences, assets, autosaves and crash reports move to `~/.config/ilex` and `~/.local/share/ilex`. An existing `minimax` folder is used until an `ilex` one exists.
- **Crash recovery:** both `crash_recovery.ilex` and the older `crash_recovery.mmax` are found.
- **glTF:** files write `extras.ilex` and still read `extras.minimax`.
- **Crash reports:** frames from both `ilex::` and `minimax::` count as the app's own.
- **Modifiers:** the old modifier names (Terrain, Ocean, Settlement, Medieval Architect, Battlefield / Ruin) still resolve, so saved Show Buttons sets work.
- **Keymap:** the "MiniMax" key set keeps its stored name and is now labelled "Ilex default".

**Packaging:**
- The PKGBUILD builds `pkgname=ilex` (`provides`, `conflicts` and `replaces` `minimax`, so an installed MiniMax upgrades in place).
- It installs `/usr/bin/ilex` plus a `/usr/bin/minimax -> ilex` symlink, `ilex.desktop` ("Ilex 3D") and `ilex.xml`. The MIME file registers `application/x-ilex-scene` (`*.ilex`), the legacy `application/x-minimax-scene` (`*.mmax`) and `application/x-ilex-pose` (`*.ilexpose`, `*.mmaxpose`).
- The local repository keeps its `[minimax]` name so an existing `pacman.conf` keeps working.
- `ILEX_SRC` and `ILEX_TARGET_CPU` are new; the `MINIMAX_*` variables still work.

**Veach** (the path tracer):
- **Veach Path Tracer** in the F10 renderer selector.
- **Veach MIS & Adaptive Sampling** settings.
- **Veach À-Trous Wavelet Denoiser**.
- The F9 **Veach Rendered Frame Window**.

**Luxon** (light and optics):
- **Luxon LightMix** (the F9 sidebar, its button and action).
- **Luxon Photometric & Area Lights**, plus **Luxon Photometric: IES Light Profile**, on the light Modify panel.
- **Luxon Volumetric Godrays & Mist** in the `8` dialog.
- **Luxon Lens FX (Bokeh, Anamorphic & Glare)** in F10.

**Demiurge** (world building):
- **Create tab:** the rollouts are now **Demiurge Worlds: Terrain, Ocean, Planet, Smoke Plume** and **Demiurge Settlements: Settlement, Buildings & Props**.
- **Create menu:** Demiurge Worlds, Demiurge Settlements and Demiurge Buildings & Props.
- **Modifier List:** new "Demiurge Worlds" and "Demiurge Settlements & Destruction" groups.
- **Modifiers:** **Demiurge Terrain**, **Demiurge Ocean**, **Demiurge Settlement**, **Demiurge Medieval Architect** and **Demiurge Battlefield & Ruin**.

**Help:** new pages "17. Characters and creatures (MakeHuman)" and "18. Ilex, Veach, Luxon and Demiurge" (20 pages in all), the renamed labels in the render, sky and Settlement pages, and a MakeHuman CC0 credit (also in `licenses/README.md`).

**Tests:**
- Scene extension and header detection, both new and legacy.
- A legacy `.mmax` round trip: it opens, and saving writes a `.ilex`.
- Legacy config folders are kept.
- Crash frames from both crate names.

**Not done in the rebrand:**
- ~~The window icon is not set~~ Done after the rebrand: the holly logo (`assets/icon/ilex-64.png`, rendered from `packaging/ilex.svg`) is set as the window / taskbar icon at start-up (`viewport/window_icon.rs`), checked on a live X11 window.
- The help screenshots in `assets/help/` still show the old labels and need regenerating with `tools/help_shots.sh`.
- Internal Rust identifiers (for example `KeySet::MiniMax`, `ModifierKind::Settlement`) keep their names so saved files still deserialize.
- Not tested by hand in the running app or through pacman.


## 33. Phase 10B: MakeHuman rig & Easy Pose

**Rig** (`src/character/rig/`):
- Every humanoid is bound to the bundled MakeHuman default skeleton: 163 bones, with parents placed before children.
- Bone heads are the centroids of the skeleton's joint vertex lists on the *morphed* body, so a tall orc and a chibi each get their own fitted rig.
- The bundled skin weights (139 weighted bones) are normalised per vertex.
- Posing is forward kinematics with linear blend skinning. Each bone rotates about its rest head, and its children inherit the motion.
- The pose is applied to the hm08 coordinates before the mesh is built:
  - The eyes (fitted with `.mhclo`) and the skin normals follow automatically.
  - The anime head sphere rides on the head bone.
  - The feet are put back on the ground.

**Easy Pose:**
- `CharacterParams::pose` holds per-bone Bend / Twist / Side angles. Scenes saved before 10B load in the rest A-pose.
- **11 presets:** A-Pose (Rest), T-Pose, Relaxed Stand, Walk Stride, Run, Sit, Kneel, Wave Hello, Fighting Stance, Victory and Spell Cast.
- **13 joint controls:** Whole Body, Lower Spine, Chest, Neck, Head, Jaw, Clavicle, Upper Arm, Elbow, Wrist, Thigh, Knee and Foot. Left and right are mirrored by default, with a "this side only" toggle.
- **Buttons:** Copy L to R, Copy R to L, Mirror Pose, Reset Joint and Reset Pose.
- **Pose files:** Save Pose... writes `.ilexpose` (RON, `format: "Ilex"`). Load Pose... also opens legacy `.mmaxpose` files and the `"MiniMax"` header.
- The **Character — Easy Pose (Rig)** rollout is on the Modify panel and in the Character & Creature Studio. Every pose change is one undo step.

**Tests:** 5 new (579 in all):
- skeleton order and joints, normalised weights, and the rig following the morph;
- the rest pose is a no-op, the T-Pose raises both wrists symmetrically, a knee bend moves only that leg, and children inherit a chest bend;
- mirroring and side copy;
- posed characters keep their topology and stand on the ground;
- `.ilexpose` / `.mmaxpose` round trips.

I also checked front and side silhouettes of all 11 presets.

**Not done in 10B:**
- ~~**No IK:**~~ Partly done: two-bone IK nudges for hands and feet (above). Still missing: there are no foot locks or hand targets.
- **No face bones in Easy Pose, and fingers only through the five hand shapes:** single finger joints have no sliders yet.
- **No animation:** there are no keyframes or timeline.
- **No viewport bone gizmo:** posing is done with sliders, not by dragging bones.
- **Angles** are measured in the rest-pose axes about each joint, not in a per-bone local frame, so big combined rotations can look odd.
- **Procedural Kit pieces:** hair, armour, capes and weapons are placed from the posed joints but are not bent with the limbs, so a weapon does not follow a raised hand.
- **Sit and Kneel** have no seat or ground contact solving.
- **Not tested by hand** in the running app, only by unit tests and the silhouette renders.
- **Hand shapes:** `character/rig/hands.rs`. Open, Relaxed, Grip, Fist and Point buttons curl all 15 finger bones per hand. Each segment turns about the axis across its own direction and the palm normal (the thumb folds toward the little finger), stored as ordinary per-bone angles, so mirroring and `.ilexpose` files work unchanged. The presets shape the hands too (Relaxed, Walk, Sit and Kneel relaxed; Run a loose grip; Fighting Stance and Victory fists). A test checks that the fist closes into the palm, the thumb crosses it, Point keeps the index straight and the two hands mirror.
- **Two-bone IK:** `character/rig/ik.rs`. The IK Move row (Hand or Foot, Left or Right, then Up, Down, Forward, Back, Out or In, 5 cm a click) solves analytically: it bends the elbow or knee to the needed reach, then swings the limb about the shoulder or hip. A straight limb bends the natural way (knees forward, elbows back). The result is stored as ordinary angles on the upper and lower limb bones. A test checks all four limbs reach a target within 5 mm, the shoulder or hip stays put, knees bend forward, and a nudge moves the wrist 5 cm. The solve uses the unmorphed base skeleton, so a very tall or short body can land a few millimetres off; the next click corrects it.
- **Help screenshots:** all 19 Help screenshots were re-captured (`tools/help_shots.sh`) so they show the Ilex names, the current font and toolbar. A new `characters` shot (an Elf in the Wave pose, a Dire Wolf and the Studio open) heads Help page 17.

## 34. MakeHuman system assets: hair, clothes, face kit, skins (CC0)

**Assets** (`assets/makehuman/`, CC0 1.0, from `makehuman_system_assets_cc0`):
- `system.mhp.z` (5.0 MB, zlib, `ILXPAK1` records): every `.obj`, `.mhclo`, `.proxy` and `.mhmat` for 12 eyebrows, 4 eyelashes, 6 teeth, 1 tongue, 10 hair styles, 20 clothing pieces, 3 body proxies (proxy741, female1605, male1591), 23 skins and 9 eye colours. Comments are stripped and coordinates rounded to 4 decimals.
- `textures.mhp` (7.7 MB): colour textures as JPEG (quality 86) with alpha as a separate grey PNG. Eyebrows and eyelashes 256 px, teeth and tongue 512, eye colours 512, hair, clothes and skins 1024. Textures that several items share are stored once (`.link` records).
- Packed by `tools/pack_makehuman.py`.

**Code:**
- `character/makehuman/pack.rs` reads the record format; `library.rs` lists the categories, parses items on first use and caches them and their decoded textures.
- `mhclo.rs`: only real section keywords switch sections (a `material` line after `verts 0` broke two items), and `delete_verts` expands `a - b` ranges.
- `character/wardrobe/`: `MhAssets` on `CharacterParams` (serde default, so older scenes load with no assets). Each piece is fitted to the morphed and posed body through its `.mhclo` binding; clothes hide the body faces they cover; a Body Topology proxy replaces the base mesh. MakeHuman hair replaces the procedural hair and beard.
- `wardrobe/materials.rs` keeps the Character Pack in step: a textured sub-material per piece (IDs 7 Eyebrows, 8 Eyelashes, 9 Teeth, 10 Tongue, 11 Hair, 12 upwards per clothing piece), and the picked skin and eye colour on IDs 1 and 2. Sub-materials whose name already matches are left alone so Material Editor edits survive.
- New Humans start with the Face Kit (default eyebrows, eyelashes, teeth and tongue).
- **UI:** the **Character — MakeHuman Assets (CC0)** rollout (Modify panel and Studio): a drop-down per category plus Skin and Eye Colour, a Clothes checklist, and Face Kit, Take Off All Clothes and Remove All Assets buttons.
- **Help:** a new section and `wardrobe` screenshot on page 17; the Credits entry now names the system assets.

**Tests:** 10 new in `character/wardrobe/tests.rs` (all passing): counts and labels, every item binds to the base mesh, the face kit, proxies follow the morphs, clothes hide the body and get their own IDs, the proxy741 topology, MH hair replaces procedural hair, texture decoding (alpha, links, 1024 px hair), material sync, and the serde round trip with old files.

**Not done:**
- Normal, AO, bump, specular and displacement maps and litspheres are not imported, only colour and alpha.
- Clothes follow the body binding; their own `vertexboneweights` are not used.
- No extra community packs (more hair, poses, expressions), extra rigs or per-age targets.
- No asset thumbnails in the pickers.
- Only checked in the running app through the help screenshot (two dressed characters at a distance); no close-up check of every item for clipping.

## 35. Face Cloner, part A: photos and frame folders → MakeHuman head

**What it does:** reshapes the selected humanoid's hm08 head to match photos of a person. The topology, UVs, rig and every `.mhclo` asset keep fitting, and the head keeps the body's scale.

**Pictures in:** one or more photos, or a folder of frames (sorted by name, every n-th with the stride). **Video files are not read directly**: export the clip to frames first with any frame-export tool. A slow head turn from one side to the other gives every view.

**Code** (`src/character/face_cloner/`):
- `frames.rs`: MediaPipe Face Landmarker (bundled `assets/face/face_landmarker.task`, Apache-2.0, run through `effectcraft-segment`) gives 478 points with depth per picture (downscaled to 1280 px). Each shot gets a sharpness score, head yaw and pitch, a mouth-open measure and a view bin (profile, three-quarter, front, each side; bins at ±15° and ±55°). `pick` drops blurry frames, near-duplicates and (optionally) open mouths, and keeps the sharpest N per view, with a note on every dropped frame.
- `fit.rs`: per-view similarity cameras (Horn) alternate with a clamped (0..1) active-set ridge least-squares solve over the head, forehead, eyebrows, eyes, nose, mouth, chin and cheek sliders (positive and negative targets per slider; right sliders include their left twin). Residuals are in head space along each camera's axes (depth weighted 0.3); points are weighted by how far they sit from their mesh vertex and by how squarely they face the camera, and views past 35° count less.
- `calibrate.rs`: the detector's habits are measured by rendering the default character at the shot's yaw (cached in 5° steps) and subtracted. Then 2 render-and-check passes re-measure them on the fitted head.
- `warp.rs`: a damped per-point offset solve, spread by a Gaussian RBF over the head and faded out between 1.7 and 1.3 head radii below its centre, so the neck seam stays put.
- `clone.rs` runs it all with the pose and anime ratio taken off for the fit (then restored); `job.rs` loads and fits on a background thread with progress and Cancel.
- `CharacterParams.face` (`FaceClone`: deltas, strength, views) is applied after the modifiers in `body::morph`; it has a serde default, so older scenes load unchanged.

**UI:**
- The **📸 Clone Face from Photos / Videos** rollout on a humanoid's Modify panel and in the Studio: Load Photos / Frame Folder..., Open Face Cloner..., status, Warp Strength (right-click resets to 1) and Reset Cloned Face.
- The **Face Cloner** window (Tools menu and the 📸 Face Cloner button) has three cards:
  1. pictures, with progress and Cancel;
  2. frame picking: per view, sharpness, neutral only, view counts and a per-picture use / Pins... list;
  3. the clone: fine warp, refine passes, caution and a report.
- The **pin editor** draws the picture with every landmark. Drag a dot to pin it, right-click to unpin. Pins override the detector in the fit.
- Help page 17 has a new section; the Credits and `licenses/README.md` name MediaPipe and EffectCraft.

**Tests:**
- Exact projections of a known face from four views are recovered to under a quarter of the shape gap, only face sliders move, and the warp stops at the neck.
- A rendered default head clones to almost itself.
- Rendered heads of two different faces move toward the truth.
- Old characters without a face still load.
- Head turns are measured and sorted into the right views.

**Not done (honest notes):**
- **Accuracy is detector-limited.** MediaPipe follows real shape changes only partly (gain about 0.5–0.7, correlation about 0.8). On renders, the landmark error drops by about 60–70%, but the true 3D shape gap closes only about 25–40%. The result is a strong starting point, not a likeness scan.
- **Ears and the back of the head are guessed** (MediaPipe has no landmarks there).
- Skin masks, the photo skin, skin-tone match and Cloned Face Presets came in part B (§36).
- Video files are not decoded.
- Real photos were not tried in the running app; only unit tests on renders and exact projections, plus `cargo check` of the UI.
- Strong expressions, glasses, heavy make-up and very wide-angle selfies are not handled beyond the neutral-expression filter.

## 36. Face Cloner, part B: photo skin, masks and Cloned Face Presets

**What it does:** the clone also bakes the photos into the character's skin texture (hm08 UVs), tints the rest of the body to match, and puts it on the Character Pack's Material ID 1. A cloned face can be saved under a name and loaded onto any humanoid.

**Code** (`src/character/face_cloner/`):
- `mask.rs`: a soft skin mask per photo. The MediaPipe face oval minus the eyes and mouth opening (grown 1.35x), feathered by 6% of the eye span, times a skin colour model learnt on the cheeks, nose and chin (chromaticity mean and spread, brightness 0.35–2.2x). With MobileSAM installed, it is prompted with those skin points, forehead and temple points of skin colour as foreground, the ones that are not (hair) and a point above the head as background, plus the face box; a SAM mask that drops more than 55% of the face is ignored.
- `sam.rs`: optional MobileSAM weights in `~/.local/share/ilex/models/` (download, install from file, SHA-256 check through `effectcraft-segment`, notice file, remove; loaded once per run).
- `delight.rs`: per photo, a 9-coefficient spherical-harmonic fit of skin brightness against the head normal is divided out (Ramamoorthi & Hanrahan), then each photo is scaled to the same mean; colour and fine detail stay.
- `bake.rs`: every head texel (within 1.9 head radii) is put on the fitted head, projected through each photo's fitted camera, z-buffer tested, weighted by mask x facing² x view weight, and blended in linear light. The mean photo colour against the base skin under it gives a per-channel tint (0.4–2.5x) for the whole base skin; the photo is feathered in by its coverage.
- `skin.rs`: the base is the character's MakeHuman skin texture, else a 1024² texture of the plain pack skin colour. Pictures are read again from their files at the analysed size.
- `preset.rs`: `FacePreset` (the cloner's own micro sliders plus `FaceClone`) as RON in `~/.local/share/ilex/faces/`; loading replaces only the face sliders and warp.
- `FaceClone` gained `texture` and `use_texture` (serde defaults, so older scenes load). `wardrobe::materials::sync` takes the photo skin: ID 1 becomes "Skin: Cloned Face"; a re-bake only swaps the bitmap, so Material Editor edits survive.
- `Cargo.toml`: the `image` crate now decodes BMP, WebP and TIFF too. Part A offered those files in the picker but could not read them.

**UI:** Face Cloner card 3 has "Bake the photos into the skin" and a skin line in the report. A new card 4, MobileSAM Masks, has Download (41 MB) with progress and Cancel, Install from File..., Remove and status. The character rollout has Use Photo Skin, Remove Photo Skin, and Cloned Face Presets (name, Save, a list, Load onto Character, Delete). Help page 17 and the Credits are updated, and `licenses/README.md` has a MobileSAM row.

**Tests:**
- A render projects onto the head through its camera.
- The landmark mask covers the face but not the eyes.
- A bake from three renders matches the true skin texture to 0.038 mean linear error, against 0.158 for a flat colour, and tints the rest toward skin.
- A painted dark fringe is cut off the forehead (0.98 -> 0.00) with the cheek kept, with and without MobileSAM.
- With MobileSAM installed, a clean face keeps 85–95% of its mask (1.3–2.5 s a photo on CPU). This test is skipped when MobileSAM is not installed.
- Lit, even samples come out even after de-lighting.
- Photos on disk -> clone -> ID 1 is "Skin: Cloned Face", and switching it off brings the MakeHuman skin back.
- A re-bake keeps Material Editor edits.
- A preset round-trips (texture included), loads onto another character without touching its other sliders, and deletes.

**Not done (honest notes):**
- **Not a full 360° scan.** The photo skin covers what the photos see (face, sides, neck). The back of the head and the body keep the base skin, tinted.
- **Absolute skin brightness is unknown from photos.** The bake follows the photos' exposure. Hard shadows, shine and coloured light that the low-order shading model cannot explain stay in the texture.
- Gaps between views are not inpainted; they fade to the tinted base skin.
- Only the colour map is made (no normal, roughness or SSS maps from the photos).
- Eyebrows and stubble in the photos are partly masked out by the colour model, so the MakeHuman eyebrow asset stays the brows.
- MobileSAM was only checked on renders here (with a painted fringe), not on real photos in the running app. Its download was not tried from inside the app, only through the same code in tests with the file already present.
- Presets store the texture inside the RON file (PNG-compressed bytes), so one preset is a few MB.

## 37. Mech PR1 (in progress): Spider Platform base + Railgun prefab

`Create -> Geometry -> Mech -> Spider Platform: Railgun` (also in the `Mech` menu)
spawns 31 Editable Poly parts joined by the assembly system (`src/mech/spider/`):

- **Hull**: a bevelled base plate, a tapered central box, a sloped front armour plate, vents, side hatches, a pedestal and a bearing ring. The hull is Grounded.
- **4 legs**: each has a hip drum (swing hinge ±45°), a thigh (lift hinge −30..60°), a shin (knee hinge ±40°) and a foot (ankle ball socket, 30° cone, 4 claws at 1.5×).
- **Option A knee support**: a carriage slides on the shin, Motion-Linked to the knee at −0.012 m/°. A piston runs from the thigh to the carriage and stays inside its stroke over the whole knee range.
- **Railgun turret**: a wedge turret (free turn hinge) with a camera box, hatches and a rear radiator. The gun has a lift hinge from −5 to 85°, a collar and a forked muzzle.
- **Surfaces**: Material IDs 1–10 follow the spec, with Smart UVs and auto smoothing at 35°.
- **Tests**: closed outward-facing shells, IDs, rest pose on the claws, knee → carriage → piston, turret and gun limits, and dragging a foot (IK) bends its leg.
- **Check renders**: the ignored test `spider_check_renders`.

Not done yet:

- The joints are verified by unit tests only. Nobody has posed the mech by hand in the app yet.
- Slat skirts are not built.
- The posing tools are not built: Foot Lock, Aim, Pose rollout, Ground Conform and Pose Library. The check-render crouch re-drags the feet by hand instead.
- The Missile/AA and Long Carrier variants and the swappable hardpoints are not built.
- The joint ranges and the claw size are the spec's proposed defaults and still wait for the owner's sign-off.
- The hull clevis is merged into the hip drum, not kept as a separate part.
- The top-down check render had to be tilted. With the camera at exactly −89° pitch, some up-facing faces dropped out. That is a possible raytracer edge case and has not been investigated.

## 38. Mech PR1b: Rig Selection as Spider Platform (the owner's own meshes)

`Create -> Geometry -> Mech -> Rig Selection as Spider Platform` (also in the `Mech` menu) rigs a spider you imported yourself. Select one whole mech (hull, turret, gun and 4 legs of hip ring, thigh, shin and foot) and click it. Nothing of the imported model is stored in the repository; the rig is built in the scene at runtime (`src/mech/spider/fit/`).

- **Which part is which**: a contact graph of which parts touch. Feet are the free ends near the ground; each leg is walked up from its foot (shin, thigh, then hip ring). The hull is the biggest part holding two or more rings. The gun is the most elongated remaining part, and the turret is the biggest part the gun touches. Every other part is welded (Rigid) to the hull, or to the leg part or turret it touches.
- **Pivots**: each hinge sits on the bolt (a thin round island) nearest where the two parts touch, with its axis along the bolt. Without a bolt it falls back to the contact centre and the leg-plane axis. The hip swing turns about the ring's vertical axis. The gun lift sits on the gun's long axis near the breech. The turret turns where it sits on the body.
- **Knee support (Option A)**: the fixed strut is cut out of each shin. A face is cut when every one of its corners lies more than 7.5% of the shin length towards the hip and within 9% of the leg mid-plane. A carriage collar sized to the shin section goes where the strut left the shin, on a slider. A piston runs from the strut's thigh end to the collar, and the carriage is Motion-Linked to the knee at 60% of the ratio that would hold the piston length.
- **In place**: every imported object keeps its world geometry. Its pivot moves onto its joint, its modifier stack is collapsed into the new Editable Poly, and it is renamed (`Spider_FL_Shin`, ...). Material IDs are set by role (1–10 as in §37); her UVs are kept. The rest pose is her pose, so joint angle 0 is the imported angle. Joint limits are the §37 ones, with the gun limits measured from its rest elevation. Undo with `Ctrl+Z`.
- **Tests**:
  - A synthetic imported spider checks classification, all 12 bolts found, the geometry left in place, the strut cut, carriage and piston stroke on a knee bend, and the gun lift and turret turn.
  - Partial selections are refused.
  - The ignored test `fits_reference_obj` (`REF_OBJ`, `REF_BOX`, `SPIDER_SHOTS`) runs on a local copy of the owner's OBJ. On the Railgun spider it found all 12 bolts and cut 194–199 strut faces per shin, in about 1 s.

Not done yet:

- It was run on the owner's Railgun spider only, in tests. It has not been used from the app's button.
- The hole the strut leaves in the shin is not capped; the collar covers the place it left.
- The piston housing eyelet floats at the strut's old thigh end; no mounting tab is added on the thigh.
- The Missile/AA spiders in the same file have not been tried.

## 39. UI redesign: look, layout, Orientation Cube, pop-out windows, rebrand

1. ✅ **Graphite & Teal** is the default theme (`src/prefs/palette.rs`): neutral greys with a `#26B8B0` accent. Slate & Amber and the other presets stay in Preferences → Theme. Old prefs migrate once (`look_version`).
2. ✅ **Soft look:** rounded controls, cards and windows, soft shadows, viewports as rounded cards with gaps and pill labels.
3. ✅ **Layout:** one header row (`src/ui/header.rs`), and a right **Properties** column (`src/ui/properties/`) with the Scene Explorer on top, a drag bar, and a vertical tab strip (Create, Modify, Hierarchy, Display, Utilities, Render, Environment, Demiurge, Characters). Width and Explorer height are saved.
4. ✅ **Orientation Cube** (`src/viewport/view_cube.rs`, `src/ui/viewport_ui/view_cube.rs`) top-right in every view: click a face, edge or corner to fly there (26 targets), drag to orbit, drag the ring to spin, Home, and a right-click menu (Set Home, Reset Home, Perspective / Orthographic, Fit to View on Change). Views → Orientation Cube hides it.
5. ✅ **Pop-out windows** (`src/ui/popout.rs`, `src/ui/popout_windows.rs`): Material Editor, Material Library, Rendered Frame Window, Edit UVWs, MakeHuman, Face Cloner, Buildings browser, PaintBox, Help and Preferences have a ↗ button that moves them into their own OS window, with app id `ilex-<name>` for window-manager rules. Each window has its own egui context with the theme, fonts and hotkeys. Closing it, or ↙, docks the dialog back. The popped set is saved in `prefs.layout.popouts`.
   - Checked under Xvfb (X11): pop out, previews drawn in the new window, M in the pop-out closes it, M in the main window reopens it popped, closing the OS window docks it back and updates prefs.
   - **Not checked:** Wayland / niri (app ids, tiling). The right-pane tabs cannot pop out yet.
6. ✅ **Rebrand:** source, help, README, CLAUDE.md and packaging no longer name other modelling apps or plugins. GhostTown is now **Demiurge Settlement** (old scenes and saved button sets still load; a modifier still carrying its old default name is renamed on load), the Slate view is the **Node Graph** view, the second key set is **Classic**, and the window title is "Ilex". Licence notices and credits keep the names they need. `src/brand_tests.rs` fails if an old name comes back outside the legacy-loading files.

## 40. MakeHuman look, part A: hair and eyes

- **Alpha: Blend / Cut-out** (`Material.alpha_cutout`, Material Editor combo). Cut-out makes alpha either solid or a hole at 50 %. The viewport draws it with MSAA alpha-to-coverage, so there is no per-mesh sorting. In F9, camera rays skip holes at the same depth, so what shows through keeps its full shading.
- **Alpha-aware occlusion:** sky occlusion, AO (raytracer and path tracer) now pass through holes and see-through surfaces instead of treating them as solid.
- **MakeHuman hair:** cut-out alpha; roughness 0.55 with a light sheen instead of the glossy look from `.mhmat` shininess; lifted 4 mm off the scalp so the skin no longer pokes through in patches. Brows and lashes keep soft blending (roughness 0.7). Hair and clothes saved with the old 0.999-opacity blending switch to cut-out on load.
- **Defaults:** new characters use the high-res eyes; new realistic archetypes get MakeHuman hair (short02 for men, ponytail01 for women, long01 for long hair). Anime archetypes and beards stay stylised.
- **Renderer fixes found on the way:** a ray landing exactly on a shared triangle edge could miss both triangles (black pin-holes), and a ray running along a BVH box face could get NaN in the slab test.
- **Eyes:** the iris colour matches the MakeHuman texture (the "brown" texture is a warm red-brown). The old blocky look came from the low-res eyes.
- Check renders: `LOOK_SHOTS=dir cargo test --lib character_look_renders -- --ignored`.
- Not done yet: hair and eye colour tint pickers (part B, MakeHuman window).

## 41. MakeHuman, part B1: the MakeHuman window, imports, hair colour

- **MakeHuman window** (header `MakeHuman`, Tools > MakeHuman..., can pop out). It replaces the Character & Creature Studio and has tabs: Main (archetype and creature tiles, Archetype/Creature, **Randomise**), Body, Face & Body Detail, Hair, Eyes, Clothes, Skin, Pose, Face Cloner, Stylised and Library. The Modify tab shows the same sections as rollouts.
- **Hair tab:** style tiles. **Hair Colour** has Natural, 12 dye swatches and a colour button. The dye recolours the texture by brightness, keeping roots and tips, rolls highlights off to white, and also colours the brows (`MhAssets.hair_color`, `wardrobe::materials::recolor`). It is only re-baked when the colour changes.
- **Eyes tab:** Low/High mesh buttons, eye colour tiles, teeth and tongue.
- **Stylised parts hidden:** the procedural low-poly hair, outfit and weapon now live only under Stylised (and a collapsed Modify rollout), behind "Show stylised low-poly parts". Characters that already use them always show them. Picking an archetype in the window also gives it MakeHuman hair.
- **Randomise** (`character::randomise`): Everything, Body only or Face only, Keep gender, a face-variation amount and a seed. It is deterministic (tested).
- **Library tab, user imports** (`makehuman::user_lib`): import a folder, a `.zip` (own small reader on `miniz_oxide`, `makehuman::unzip`) or a `.mhclo`. Assets are copied flat to `~/.local/share/ilex/makehuman/<category>/<name>/`.
  - The category is auto-detected from folder names (with an "Import as" override).
  - Names never clash with bundled ones.
  - Textures may use another extension than the `.mhmat` names.
  - Imports appear next to the bundled items in every picker, fitted the same way. Remove (click twice) deletes Ilex's copy.
- Not done yet (part B2): bundled extra CC0 community assets, asset thumbnails, eye-colour tint, the Mesh2Motion animation library.

## 42. Prefab Library, part H1: library browser, place mode, Detail

- **Prefab Library** (it was the Buildings & Props Browser). Open it from Tools, the Demiurge tab or the 🏠 Prefabs button.
  - It has categories, search and **Favourites**: click the star on a tile. Favourites are kept in Preferences (`layout.prefab_favourites`).
  - The thumbnail cache grows with the number of prefab kinds, so new kinds need no code change there.
- **Place mode** (`Tool::PlacePrefab`):
  - **Placing:**
    - Click a tile, then click in a viewport. The prefab drops onto the closest object surface under the cursor, or onto the grid or snap point. It always stands upright.
    - The viewport shows a footprint outline with a front arrow, plus the rotation and scale.
  - **Controls while placing:**
    - the mouse wheel turns it in 15° steps;
    - `[` / `]` scale it by ×1.1;
    - Shift-click stamps more copies;
    - right-click ends.
  - **The library's top bar** has the same Rotation and Scale, with buttons and right-click-resettable spinners, plus three checkboxes:
    - **Vary Seed** gives each copy a new seed;
    - **Drop on Surfaces**;
    - **Drag to Size** gives the old drag-a-footprint tool instead.
- **Detail: Standard / Mid** on every building and prop (`BuildingParams.detail`, `#[serde(default)]`, so old scenes load as Standard).
  - Mid gives every window a frame, a deeper sill and a mullion on wide panes, in Trim (ID 4).
  - Settlements stay Standard.
- **Destruction** (Intact → Rubble) was already on every prefab through the ruin system.
- Tests:
  - place options;
  - dropping onto an object, or onto the grid when Drop on Surfaces is off;
  - rotation and stamping;
  - Vary Seed;
  - Mid detail polygon counts;
  - old-scene load.
- **Not done yet:**
  - snap sockets, Sets, Group/Ungroup and "My Prefabs";
  - raytraced thumbnails (they are still flat-shaded);
  - the new building kinds, which come next in batches.

## 43. Prefab Library, part H2a: residential buildings
- **13 new Residential kinds** under a new **Residential** category (first in the library and the type combo). They are appended to the end of `BuildingKind`, so saved scenes and seeds keep their meaning.
  - **Houses:** Cottage, Bungalow, Farmhouse, Log Cabin, A-Frame Cabin, Villa, Victorian House, Modern House, Beach Hut.
  - **Mid density:** Row Houses / Terrace, Duplex, Walk-Up Apartments.
  - **High density:** Residential Tower.
- **What each kind has:**
  - **Cottage:** steep gable, dormers, chimney and porch.
  - **Bungalow:** brick, low hip roof and veranda.
  - **Farmhouse:** full-width porch and two chimneys.
  - **Log Cabin:** stacked logs and a stone chimney.
  - **A-Frame Cabin:** a glass gable end and a deck.
  - **Villa:** stucco, hipped roofs, a side wing, a balcony and a columned entrance.
  - **Victorian House:** steep gable, corner turret with a cone roof, two-storey bay window and porch.
  - **Modern House:** flat roofs, an upper box, a glazed ground floor and ribbon windows.
  - **Row Houses / Terrace:** one unit about every 5.5 m in alternating colours, each with its own door, stoop and chimney.
  - **Duplex:** mirrored doors and porches.
  - **Walk-Up Apartments:** stacked bay windows, a stoop, a cornice and a fire escape.
  - **Residential Tower:** glazed lobby, balconies on every floor and a plant room.
  - **Beach Hut:** plank walls, double doors and side windows.
- **Shared house builder** in `buildings/houses.rs`:
  - walled storeys with windows, front shutters and a door;
  - gable, hip or flat-with-parapet roofs that follow the longer side.
- **Lot** (`BuildingParams.lot`, `#[serde(default)]`): Garden (lawn, path, bushes), Driveway, Garage and Fence (picket fence with gaps).
  - The checkboxes show for Residential kinds.
  - Old scenes load with the default lot: garden and fence.
- Settlements count Residential kinds as houses.
- **Check renders:** `PREFAB_SHOTS=dir PREFAB_CAT=Residential cargo test --lib prefab_check_renders -- --ignored` writes one 3/4 front render per kind, which makes the thumbnail sheet.
- **Tests:**
  - every residential kind builds valid, deterministic meshes at Standard and Mid detail, within a polygon budget and with finite UVs;
  - windows and roofs are present;
  - heights make sense;
  - the lot adds geometry and the garage sits beside the house;
  - old scenes get the default lot.
- **Not done yet:**
  - the walk-up's bays and the tower's balconies are the same colour as their walls, so they read weakly in thumbnails;
  - more styles per house;
  - the next zones (commercial/office, industrial/utilities, services/transport, parks/lot details/rural).

## 44. Prefab Library, part H2b: commercial and office buildings
- **17 new kinds** under a new **Commercial & Office** category, second in the library. They are appended after Residential (indices 57–73), so saved scenes and seeds keep their meaning.
  - **Shops and places to eat:**
    - Corner Shop: wrap-around shopfront with flats above.
    - Café: terrace tables with parasols.
    - Diner: steel bands, ribbon windows and a pylon sign.
    - Restaurant: brick, tall windows, canopy and planters.
    - Bakery: bow window and an awning.
    - Pharmacy: glowing cross and parking.
  - **Big boxes:**
    - Supermarket: car park, trolley shelter and loading dock.
    - Shopping Mall: two anchor stores and a glazed atrium.
    - Hotel: podium, porte-cochère, slab tower with balconies and a crown sign.
    - Motel: outside walkways, a door per room, lobby, pylon sign and parking.
    - Bank: steps, columns and a pediment.
    - Cinema: glowing marquee, blade sign and poster cases.
  - **Car-focused:**
    - Car Dealership: glass showroom, forecourt cars and flags.
    - Drive-Through: drive lane, menu board, pay window and a tall pylon.
  - **Offices:**
    - Low-Rise Office: ribbon glazing and parking.
    - Mid-Rise Office: curtain wall and a set-back crown.
    - Office Campus: three blocks round a lawn, glazed bridges and parking.
- **Shared pieces** in `buildings/shops.rs`: shopfronts, flat boxes with parapets, signs, pylon signs, parking lots with painted bays and parked cars, café tables, planters and canopies.
- Settlements count Commercial kinds as buildings, so they take ruin damage.
- **Tests:**
  - every kind builds valid, deterministic meshes at both detail levels, within budget;
  - every kind has glazing, stands on the ground and fits its lot;
  - heights are sensible;
  - car parks have painted bays;
  - turning Signs off removes the glow.
- **Not done yet:**
  - flat roofs all use the roof colour, so the big boxes read a bit alike from above;
  - the showroom glass reads as lit white in thumbnails;
  - no interiors.

## 45. Prefab Library, part H2c: industrial and utility buildings
- **14 new kinds** under a new **Industrial & Utilities** category. They are appended after Commercial (indices 74–87), so saved scenes and seeds keep their meaning. They have no Floors slider.
  - **Industrial:**
    - Factory: gabled brick halls, an office block, chimneys and a loading dock.
    - Truck Depot: cross-dock shed with a row of dock doors, trailers backed up to them, a guard hut, a gate barrier and a fence.
    - Silo Cluster: silos with a conveyor gallery, a lattice tower, a chute and a ladder.
    - Grain Elevator: tall headhouse, twin rows of bins, and a truck bay with a spout.
    - Refinery: distillation columns with platforms, a glowing flare stack, heaters, a pipe rack and tanks.
    - Tank Farm: tanks in bund walls, joined by pipes to a header.
    - Recycling Yard: shed, scrap heaps, shipping containers, a conveyor and hopper, and a scrap crane.
  - **Utilities:**
    - Power Plant: turbine hall, boiler house, two hyperbolic cooling towers, a 90 m stack, transformers and a coal heap.
    - Wind Turbine: the Width slider is the rotor diameter; the hub sits at 1.9× the blade length.
    - Solar Farm: tilted panel rows on posts, an inverter cabin and a fence.
    - Pump Station: brick pump house, pipes with valve wheels, a surge tank and an intake basin.
    - Substation: gravel pad, transformers, gantries with insulators, a control building and a fence.
    - Power Pylon: lattice tower with three cross-arms and insulator strings.
    - Cell Tower: monopole, three-sector antennas, a dish, an equipment cabin and a fence.
- **Shared pieces** in `buildings/industry.rs`:
  - post-and-wire fences with a gate gap, tanks, silos, ladders, pipes and pipe racks, smoke stacks;
  - lattice towers (lifted so their braces never dip below ground), dock doors, heaps and shipping containers.
- `kinds.rs` was split: labels and default sizes moved to `kinds_info.rs`, keeping both files under 350 lines.
- Settlements count Industrial kinds as buildings, so they take ruin damage.
- **Tests:**
  - every kind builds valid, deterministic meshes within budget;
  - every kind stands on the ground and fits its lot;
  - heights are sensible, and the turbine scales with its rotor;
  - fenced yards have fences, and solar panels are glass.
- **Not done yet:**
  - pipes are straight runs without elbows;
  - the cooling towers have no inner shading;
  - wind turbine blades are flat beams, not twisted aerofoils.

## 46. Prefab Library, part H2d: services and transport
- **17 new kinds** under a new **Services & Transport** category. They are appended after Industrial (indices 88–104), so saved scenes and seeds keep their meaning.
  - **Services:**
    - Fire Station: brick appliance bays with glazed doors, an apron, a hose tower and an office wing.
    - Police Station: flat-roofed block behind a car park, glazed entrance, blue lamp and a flag.
    - Clinic: single storey with ribbon windows, a drop-off canopy, a glowing cross and parking.
    - University Block: long brick block with a columned entrance, a clock tower and a lawn quad with paths and trees.
    - Library: glazed ground floor under a cantilevered finned box, a clerestory and a plaza with benches.
    - Town Hall: raised centre block with a portico and domed clock tower, lower wings, steps, flags and a plaza.
    - Post Office: brick counter hall, post boxes, and a delivery van at the loading door.
    - Temple: stone platform with steps, a columned hall under two tiers of wide roofs, a gate and lanterns.
    - Cemetery: walled lawn with rows of headstones, trees and a small chapel.
  - **Transport:**
    - Bus Stop: glass shelter with a roof, bench, timetable and stop sign.
    - Bus Depot: gabled garage with roller doors, parked buses, an office and a fuel island.
    - Train Station: barrel-vaulted hall with a clock and canopy, two-storey wings, and platforms with canopies over tracks.
    - Metro Entrance: stairs down about 3 m through an opening in the paving, railings, a glass canopy and a lit sign mast.
    - Tram Stop: island platform with a canopy, bench and ticket machine, tracks on both sides and overhead-wire poles.
    - Airport Terminal: glazed hall under a sawtooth roof, drop-off road and canopy, jet bridges and an airliner at each gate.
    - Control Tower: tapered shaft with a flared glass cab, antennas and a base building. Floors make it taller.
    - Harbour Office: hipped-roof office on a quay with a lookout, a flag, bollards and water in front.
- Settlements count Services kinds as buildings, so they take ruin damage.
- **Tests:**
  - every kind builds valid, deterministic meshes within budget;
  - every kind stands on the ground and fits its lot (only the metro stairs go below ground, by at most 3.5 m);
  - heights are sensible, the harbour has water and the station has rails.
- **Not done yet:**
  - the metro stairs end at a blank wall, with no ticket hall below;
  - the airliner is simple (no windows, a flat tail);
  - the train station has no rail lines leaving the lot, and tram tracks don't join up between stops;
  - there is no hospital, school or parking garage yet; these come with the parks and rural batch or later.

## 47. Prefab Library, part H2e: parks and leisure
- **10 new kinds** under a new **Parks & Leisure** category, appended after Services & Transport (indices 105–114). They have no Floors slider.
  - Playground: safety surface, swings, a slide tower with a roof, a see-saw, a climbing dome, a sandpit, benches and a low fence.
  - Sports Court: painted hard court with lines, either a tennis net or two basketball hoops (by seed), a fence and a bench.
  - Sports Field: grass pitch with markings, penalty boxes, goals with nets, four floodlights and a clubhouse.
  - Stadium: pitch and running track inside four tiered stands, cantilever roofs over the long sides, and four 40 m floodlight masts.
  - Swimming Pool: paved deck, lane ropes, a diving board, a kiddie pool, loungers, parasols, a changing block and a fence.
  - Plaza: paved square with a ring pattern, a fountain, tree beds, benches, lamps and three flags.
  - Monument: stepped plinth in a round garden with an obelisk, a column with a figure, or a triumphal arch (by seed).
  - Park Pond: irregular pond with a gravel path, a footbridge, reeds, ducks, trees and benches.
  - Bandstand: octagonal raised stage with columns, a railing, steps, a pointed roof and deck chairs.
  - Skate Park: half-pipe with coping, a fun box with a rail and ramps, a bowl and a ledge.
- Shared helpers: `parks::lines` (court and pitch markings) and `parks::bench`.
- **Tests:**
  - every kind builds valid, deterministic meshes within budget;
  - every kind stands on the ground and fits its lot;
  - the stadium is tall, the skate park is low, the pools and pond have water, and the courts and pitches have markings.
- **Not done yet:**
  - the pond and the pools are flat water surfaces, not dug basins;
  - the skate bowl is stepped rings, not a smooth curve;
  - the stadium stands have no seats or stairways modelled.

## 48. Prefab Library, part H2f: garden and lot details
- **16 new kinds** under a new **Garden & Lot** category, appended after Parks & Leisure (indices 115–130). They are small life-sim items for dressing a house lot, and have no Floors slider. Width runs along X, so a hedge or fence stretches with Width.
  - Mailbox: post, rounded box, door and flag.
  - Garden Beds: raised timber beds with gravel paths, crops (leafy rows, shrubs or canes, by seed) and a water butt.
  - Hedge: clipped hedge with a lumpy top.
  - Picket Fence: pointed pickets on two rails, posts, and a half-open gate when the fence is long enough.
  - Patio Set: paving slabs, a table and four chairs under a parasol, a bench and potted plants.
  - Barbecue: a kettle grill or a brick barbecue with a hood and chimney (by seed).
  - Swing Set: A-frame with two swings and a slide.
  - Trampoline: legs, padded frame, mat and safety net.
  - Garden Shed: clad timber shed with a gable or pent roof, a door and a window.
  - Doghouse: kennel with an arched opening, a name board and a bowl.
  - Washing Line: a T-post line with clothes, or a rotary dryer (by seed).
  - Wheelie Bins: up to four coloured bins with lids (one may stand open) and wheels.
  - Garden Gnome: boots, body, beard, face and pointed hat, sometimes with a fishing rod.
  - Hose Reel: drum on a frame with a crank, hose and nozzle.
  - Backyard Pool: an above-ground pool with a ladder, or a paved in-ground pool with steps and grab rails (by seed).
  - Porch & Steps: raised deck with steps, posts, railings and a lean-to roof, meant to push against a house wall at +Y.
- **Tests:**
  - every kind builds valid, deterministic meshes within budget;
  - every kind stands on the ground and fits its lot;
  - the gnome is small, the shed is tall enough, the pool has water and the hedge is foliage.
- **Not done yet:**
  - no socket snapping yet: the porch, fence and hedge don't snap to houses or to each other;
  - the in-ground pool is a flat water surface, not a dug basin.

## 49. Prefab Library, part H2g: rural and farm
- **8 new kinds** under a new **Rural & Farm** category, appended after Garden & Lot (indices 131–138). None of them has a Floors slider.
  - Barn: red (or timber) barn with a gambrel roof, sliding doors with cross braces, a hay loft door with a hoist beam, and a lean-to with a tin roof when the lot is wide enough.
  - Greenhouse: glass house on a low brick wall, frame and ridge, a door, staging benches with potted plants.
  - Polytunnel: half-round plastic tunnel with hoops, a framed door and crop rows inside.
  - Henhouse & Run: coop on legs with a nesting box, ramp and a wire run, plus a few hens.
  - Haystacks: round bales (lying and standing) and a stepped stack of square bales on earth.
  - Orchard: rows of fruit trees with fruit in the crowns, a ladder and fruit crates.
  - Paddock: post-and-rail fence with an open five-bar gate, a water trough and a field shelter.
  - Beehives: a row of stacked box hives on stands among wildflowers.
- **Tests:** every kind builds valid, deterministic meshes within budget, stands on the ground and fits its lot; the barn is tall, the hives are low, and the greenhouse, paddock and orchard have glass, water and foliage.
- **Not done yet:**
  - the farmhouse, stable, windmill and silo from the plan already exist elsewhere (Farmhouse in Residential, Windmill in Medieval, Silo Cluster in Industrial); a dedicated stable is not built yet;
  - the hens and bees are simple blocks, not animals.

## 50. Prefab Library, part H3a: furniture (seating and tables)
- Second wave starts: a new **Furniture** category, appended after Rural & Farm (indices 139–148). There's no Floors slider, and the texture repeats are finer so wood grain and fabric read at furniture scale.
  - **Seating:**
    - Dining Chair: slatted or solid back.
    - Office Chair: five-star base on casters, gas lift, padded seat, back and armrests.
    - Armchair: club style.
    - Bar Stool: pedestal or four legs, with a foot ring.
    - Sofa: per-seat and back cushions, plus scatter cushions.
    - Sectional Sofa: L-shaped with a chaise.
  - **Tables:**
    - Dining Table: rectangular, or round on a pedestal for square sizes. Place mats, plates and a fruit bowl.
    - Coffee Table: with a shelf, books and a mug.
    - Side Table: a drawer, and a lamp or a potted plant on top.
    - Desk: drawer pedestal, monitor, keyboard, mouse and desk lamp.
  - Upholstery and top colours vary with the seed. At Mid detail the cushions get rounded corners and softened tops.
- **Tests:**
  - every kind builds valid, deterministic meshes within budget, stands on the floor and fits its footprint;
  - chair-back and table heights are realistic, and the coffee table is low;
  - Mid detail adds faces to the sofa.
- **Not done yet:** no snap sockets yet (chairs don't tuck under tables, sofas don't snap to walls); no room shell yet.

## 51. Prefab Library, part H3b: furniture (bedroom and storage)
- **9 more Furniture kinds** (indices 149–157). Each has a seeded wood, white or dark carcass, and drawer fronts with handles.
  - Single Bed and Double Bed: frame, headboard, footboard, mattress, one or two pillows, and a turned-down duvet.
  - Bunk Bed: corner posts, two bunks with bedding, a guard rail, and a ladder at the foot.
  - Wardrobe: two or three doors (three when wide), one mirrored door, a plinth, a cornice and a drawer.
  - Dresser: drawers in one or two columns, with a mirror on top.
  - Nightstand: two drawers, a lamp and a book.
  - Bookshelf: five shelves of seeded books, with varied heights, colours and gaps.
  - TV Stand: media unit with doors, an open bay and a console, a flat TV on a stand (the screen glows when emissive) and a sound bar.
  - Display Cabinet: cupboard base, glass front, glass shelves with plates and vases.
- **Tests:** the wardrobe and bunk bed are tall, the nightstand is low, and all the shared furniture checks pass.
- **Not done yet:** furniture uses the shared Settlement pack, so big wood panels show the weathered timber's plank bands. A clean furniture material pack (smooth wood, fabric, lacquer) is a follow-up.

## 52. Prefab Library, part H3c: kitchen & bath, lighting & decor
- **Two new categories**, appended after Furniture.
  - **Kitchen & Bath** (indices 158–164):
    - Kitchen Units: a run of base units with plinth, worktop, splashback, sink and tap, a four-ring hob and an oven.
    - Wall Cabinets: hung at 1.45 m with an under-cabinet light strip, and a cooker hood when wide.
    - Kitchen Island: cabinets, an overhanging top, a prep sink and bar stools along the front.
    - Toilet: close-coupled, with seat, cistern and flush button.
    - Vanity Sink: vanity with basin and tap, a mirror cabinet above and a light bar.
    - Bathtub: built-in, with a tiled panel, water, a mixer and a shower riser.
    - Shower Cubicle: tray, tiled walls, glass enclosure, riser rail, head and mixer.
  - **Lighting & Decor** (indices 165–170):
    - Floor Lamp: a standard lamp with a drum shade, or an arc lamp (by seed).
    - Pendant Light: ceiling rose at 2.5 m, flex, and a cone or dome shade.
    - Rug & Pouf: patterned rug with border and fringe, plus a pouf.
    - Curtains: pole with finials, pleated drapes tied back, and a sheer.
    - Wall Mirror: rectangular or round, with a shelf.
    - Floor Plant: a planter with palm blades or a small tree crown.
  - Wall and ceiling pieces are built at their real height, so they place on the floor under where they hang.
- `BuildingKind::defaults()` moved to `kinds_defaults.rs` to keep `kinds_info.rs` under 350 lines.
- **Tests:** toilet height, pendant hang height, bath water, plant foliage, and all the shared furniture checks for the 32 indoor kinds.
- **Not done yet:**
  - no room shell yet, and no wall/ceiling sockets: wall cabinets, mirrors and curtains must be placed by hand;
  - the appliance batch (fridge, washer, electronics…) is next.


## 53. Prefab Library, part H3d: appliances & electronics
- **New category "Appliances & Electronics"** (indices 171–186), appended after Lighting & Decor:
  - Fridge: fridge-freezer, or side-by-side doors with a dispenser when the lot is wide; magnets and a note.
  - Cooker: oven door with window and handle, control dials, a four-ring hob with a pan and a pot, splashback.
  - Microwave, Dishwasher, and Washing Machine: the washer has a porthole door, detergent drawer, dial and display, and is a dryer with a dark door about 30% of the time by seed.
  - Kettle & Toaster: a jug kettle on its base and a two-slot toaster.
  - Coffee Machine: an espresso machine with group head, portafilter, steam wand, drip tray and cups.
  - Television: on splayed feet or a pedestal, with a soundbar.
  - PC Setup: a tower with a glass side and lit fans, one or two monitors, keyboard, mouse mat and headphones.
  - Laptop: opened to about 110°, with a phone and a mug.
  - Speakers: two tower speakers with drivers and a port, plus an amplifier.
  - Air Conditioner: an indoor split unit at 2.1 m and the outdoor condenser, joined by a pipe.
  - Radiator: a finned panel on brackets with a valve.
  - Fan: a pedestal fan with a caged three-blade rotor.
  - Vending Machine: an open, lit display bay with shelves of products, a keypad and a pickup flap.
  - Arcade Cabinet: profiled sides, lit marquee and screen, a joystick and buttons, and a coin door.
- Counter-top pieces stand at z = 0, so Drop on Surfaces puts them on worktops and desks. Screens and displays glow unless Emissive is off.
- **Tests:** `tests_appliances.rs` covers:
  - index placement, plus valid, deterministic meshes at both detail levels;
  - floor contact, and heights for the fridge, cooker, counter-top pieces, the vending and arcade machines, and the high AC unit;
  - the glowing screens, and that they switch off.
  - The generic "stands on the ground" check allows 0.15 m tall for this category (a laptop is short).
- **Not done yet:**
  - no hinged doors or openable parts;
  - fridge and cooker have no interiors;
  - the vending machine has no glass pane, because the thumbnail renderer draws GLASS opaque and it hid the products;
  - the room-shell generator is next.

## 54. Prefab Library, part H3e: room shells
- **New category "Room Shells"** (indices 187–190), built in `rooms.rs`:
  - Room: a door with an open leaf, and windows in the back and side walls.
  - Open-Plan Room: a door, three windows, glazed patio doors, and a partition stub with a breakfast bar over a tiled kitchen floor.
  - Bathroom Shell: a tile floor, walls tiled to 1.2 m, and a small high frosted window.
  - Hallway: doors at both ends and off each side, a small window and a runner.
- **How a shell is made:**
  - The lot is the room's inside. The 0.15 m walls stand outside it, so the full width × length is usable floor.
  - A `layer` helper cuts real openings through each wall.
  - Every opening gets architraves; windows also get a sill, frame, mullion and glass. Skirting runs between the doors.
  - The floor finish tops out at z = 0, so furniture dropped in sits on it.
  - It's a cutaway: no ceiling.
- `tests_shots.rs` takes `PREFAB_PITCH` (degrees) for a higher camera, so room interiors show on the review sheet.
- **Tests:** wall height, thin slab, walls outside the lot, a floor face at z = 0, glass, bathroom tiles, and a face count above solid walls.
- **Not done yet:**
  - no ceiling option or ceiling sockets;
  - furniture doesn't snap to walls yet (sockets are a later step);
  - door and window positions are fixed per kind (no sliders);
  - rooms don't join into a floor plan.

## 55. Prefab Library, part H4a: road cars
- **New category "Vehicles"** (indices 191–198), built in `vehicles.rs` (shared body, wheels, cabin, interior, trim) and `cars.rs` (designs and extras).
- **Eight kinds, three designs each** (stored in the Floors value and picked with new **Design:** buttons in the Modify panel):
  - Sedan: Family Sedan, Executive Saloon (chrome strip), Compact Coupe.
  - Hatchback: City Hatch, Hot Hatch (spoiler), Estate (roof rails).
  - SUV: Compact Crossover, Boxy Off-Roader (spare wheel on the back), Full-Size SUV.
  - Pickup Truck: Single Cab, Double Cab, Flatbed Work Truck.
  - Van: Panel Van, Minivan, Camper Van (pop-top).
  - Sports Car: Mid-Engine (side intakes), Grand Tourer, Roadster (open, low seats).
  - Taxi: City Cab, Tall Cab, Minivan Cab, all with chequer band and roof sign.
  - Police Car: Patrol Sedan, Patrol SUV, Interceptor, with a light bar and push bar.
- **Real parts:** the side profile is extruded with semicircular wheel-arch notches and an inner wheel well; separate wheels (tyre, rim, hub, spokes at Mid); a glass cabin (GLASS ID) with roof panel and A/B/C pillars; head and tail lights (glow when Emissive is on), grille, bumpers, front and rear plate areas, mirrors, door seams and handles. Mid detail adds seats, a dashboard and a steering wheel.
- No brands, badges or copies of a specific model.
- Paint is seeded and also varies by design.
- **Tests (`tests_vehicles.rs`):** appended indices, three designs per kind, valid deterministic meshes at both details, glass and tyres, standing on the road, designs differ, Mid adds parts, and real heights (the van is taller than the sedan, the sports car lower, the police light bar on top).
- **Not done yet:**
  - doors, bonnet and boot don't open, and wheels aren't separate objects that can turn;
  - interiors only appear at Mid detail (or on the roadster);
  - the spare wheel is a simple cylinder;
  - Library thumbnails show design 1 only;
  - bus, lorry, tractor, forklift, motorbike, bicycle and scooter come next (H4b), then trains, a boat, a small plane and a helicopter (H4c).

## 56. GPU kernel parity, PR 0: groundwork
No new GPU features yet; this makes the next parity steps safe and visible.
- **Kernel split** into `src/render/gpu_pathtrace/wgsl/` (common, bvh, bsdf, env, lights, volume, integrator), joined at build time by `gpu_pathtrace::source()`.
- **Per-scene shader builds:** seven WGSL `override` switches (`FEAT_TEXTURES`, `FEAT_PRINCIPLED`, `FEAT_SSS`, `FEAT_VOLUME`, `FEAT_ENV_MAP`, `FEAT_AREA`, `FEAT_OPACITY`) guard empty hooks (`shade_hook`, `area_nee`, `volume_segment`, `opaque_at`, the env-map branch) that PR 1–4 fill in. `features::Features::of(scene)` picks the mask, and `Gpu::pipeline` compiles and caches one pipeline per mask.
- **Time-based batches:** each GPU submit traces 1–8 samples per pixel, sized from the last submit's measured time to aim for about 50 ms (`features::next_spp`), replacing the fixed 4 spp. Long renders no longer risk a driver reset, and fast cards get bigger batches.
- **Every fallback reason:** `features::reasons()` lists all CPU-only features a scene uses, and the Frame Window status shows them all ("CPU (area lights and see-through (opacity) surfaces need the CPU tracer)"). The Render tab's path-tracer note has a tooltip listing everything that still needs the CPU (`features::CPU_ONLY`).
- **Device label** now includes the driver: "GPU: card (Vulkan, driver)".
- **Tests:**
  - a parity harness (block mean error, per-channel bias, energy, coverage) run on two scenes: glass/metal/emitter, and spot + sun + fog in perspective;
  - naga (dev-dependency) parses and validates all 128 switch combinations;
  - with an adapter, all 128 pipelines build and are cached;
  - reasons, joining, constants, batch sizing and the device label.
  - On lavapipe here: energy GPU 57.11 vs CPU 57.21, and 27.28 vs 27.21 with fog.
- **Not done yet:** the hooks are empty, so the GPU still falls back for the same scenes as before. Sky/HDRI and textures come in PR 1, Principled in PR 2, volumes in PR 3, area lights / IES / opacity / catcher / toon in PR 4. Speed on a real card (her RX 9060 XT) is untested; only software Vulkan runs here.

## 57. GPU kernel parity, PR 1a: sky and HDRI on the GPU
- **Scenes with the dynamic sky or an HDRI no longer fall back to the CPU** (that reason is gone from the fallback list).
- `gpu_pathtrace/envmap.rs`:
  - an HDRI uploads as-is, keeping its rotation and strength;
  - the procedural sky (Earth or Space, with clouds) is baked into an equirect image on all cores, using the same mapping as `HdrImage::sample`;
  - both are cached, so re-renders with the same sky skip the bake.
- **Kernel:** a new binding 8 (`envmap`) and a `Params.env` vec4. `env_sample` is the same bilinear lookup as the CPU. Under `FEAT_ENV_MAP`, paths that leave the scene see the image plus the skylight term, exactly like `Tracer::environment`. Camera misses still use the CPU's exact backdrop, so stars and the sun disc stay crisp on screen.
- **Render tab → GPU Sky Bake:** 512 / 1024 (default) / 2048 / 4096 wide, saved in the render settings.
- **Tests (`tests_env.rs`):**
  - texel centres map back to their own pixel;
  - the bake equals the sky at texel centres;
  - sky and HDRI scenes switch `FEAT_ENV_MAP` on and don't fall back;
  - a repeated bake hits the cache;
  - GPU vs CPU parity under the procedural sky (energy 56.23 vs 56.14) and under a rotated, boosted HDRI (67.44 vs 67.49).
- **Not done yet:**
  - **No sky importance sampling (the "pick table"):** the CPU path tracer doesn't sample the sky directly either, so adding it to only one would break the comparison. It's a later noise improvement for both tracers.
  - **Fine detail in reflections:** stars and the sun disc are softened by the bake size.
  - **Textures and map baking (PR 1b)** are next.

## 58. GPU kernel parity, PR 1b: texture and procedural maps on the GPU
- **Mapped materials no longer send the scene to the CPU.** This covers bitmaps, map nodes, node graphs, procedural surfaces, paint layers and World / Triplanar projection.
- **`gpu_pathtrace/bake.rs`:**
  - Each mapped material is baked once over a UV grid. A texel holds 12 halves: base + alpha, emission + metallic, roughness + tangent-space normal.
  - The grid is the largest bitmap's own size, or 1024 when any map is procedural, capped at 4096.
  - Baking runs on all cores and is cached by material (the last 24), so re-renders skip it.
  - Probes decide what can be baked:
    - maps that change with the 3D position (Cracks, Grunge, 3D Noise) stay on the CPU;
    - triplanar materials need maps that repeat every 1 UV and no paint layers;
    - UV maps that don't repeat are baked over the UV bounds of the triangles that use them and clamped instead of wrapped.
- **`gpu_pathtrace/textures.rs`:** one new storage binding (9, `aux`) holds:
  - per triangle: corner UVs, a texture index and the object's projection frame;
  - a 12-word table per texture;
  - the texels.
  
  The whole store is capped at 1.5 GB and checked against the GPU's buffer limit. Past either limit the scene falls back to the CPU with the reason shown.
- **Kernel (`wgsl/textures.wgsl`):**
  - bilinear f16 lookups;
  - normal maps tilt the shading normal along the triangle's UV frame, the same as `relief::perturb` and `uv_frame`;
  - World / Triplanar blends three projections by the object-space geometric normal, with the same weights, live-side cut-off and frames as `Projection::blend` / `triplanar_normal`.
  
  Textured hits multiply by the interpolated linear vertex paint. Emitting triangles now carry the CPU's per-triangle emitter radiance, so NEE matches.
- **Tests (`tests_tex.rs`):**
  - GPU vs CPU parity:
    - UV maps: a clamped gradient floor, a brick box with checker metal and roughness, and a checker emission panel (29.93 vs 29.94);
    - procedural wood with relief (30.78 vs 30.68);
    - triplanar procedural bricks with relief (32.80 vs 32.77);
  - 3D maps stay on the CPU;
  - non-repeating triplanar maps are refused;
  - bitmap sizes, the f16 texel layout and the cache;
  - an untextured scene gets an empty store.
- **Not done yet:**
  - **Some materials still render on the CPU:** 3D (position-based) maps, triplanar with paint layers, and non-repeating triplanar maps. A later step could evaluate those per hit on the GPU.
  - **Textures over 4096 are downsampled** to 4096 for the GPU bake.
  - **Interpolated vertex paint, then lit:** vertex paint on a mapped material is turned to linear at each corner and then blended. The CPU blends first and converts afterwards, so smooth vertex-paint gradients can differ slightly.
  - **Cavity AO isn't baked:** only the CPU's ambient term uses it, and the GPU kernel has no ambient-occlusion term yet.
  - **Cut-out alpha is baked but unused:** see-through surfaces still need the CPU (PR 4).

## 59. GPU kernel parity, PR 2: Principled lobes on the GPU
- **Principled materials and Preserve Texture Appearance no longer send the scene to the CPU.** Two reasons are gone from the CPU-only list (now 8 entries).
- **Material records (`gpu_pathtrace/textures.rs`):** materials with Principled lobes or a camera glow of their own get a 16-word record in the `aux` store:
  - anisotropy and rotation;
  - clearcoat and its roughness;
  - sheen and tint;
  - SSS weight and Abbe number;
  - the Beer absorption exponent per unit length;
  - the camera glow brightness;
  - the random walk's extinction and single-scatter albedo.
  
  Each triangle's word 7 points to its record, and `Params.tex.zw` gives the table and its count. A scene with records but no maps still builds the per-triangle words.
- **Kernel (`wgsl/principled.wgsl`):** a port of `bsdf/mod.rs` and `microfacet.rs`:
  - anisotropic GGX in the same tangent frame (`Frame::new`), with the Disney alphas, Smith masking and Heitz VNDF sampling;
  - the clearcoat layer with its lobe pick and base attenuation;
  - cloth sheen on the diffuse layer;
  - the SSS entry lobe, then the random walk (exponential free flights, isotropic scattering, Russian roulette, up to 256 steps), then a white Lambert exit with NEE (`subsurface.rs`).
  
  Also covered:
  - **Glass:** `sample_glass` follows one colour channel per path with its own Cauchy IOR (weight ×3, the same as the CPU), and the integrator keeps the path's channel.
  - **Beer absorption** applies when leaving a transmissive surface.
  - **Preserve Texture Appearance:** camera hits glow with the surface's own colour × Direct Camera Brightness.
  
  `Surf` gained the frame, the alphas and the lobe weights. With the switches off, the plain BSDF code is unchanged.
- **Tests:**
  - **`tests_lobes.rs`** runs the kernel's `pr_eval` / `pr_sample` on the GPU for 48 random materials and directions (every lobe combination):
    - evaluation matches `Bsdf::eval` to 0.2%;
    - the mean sample weight (directional albedo, a white-furnace check) and the SSS entry share match the CPU's sampling.
    - Mutation checks catch a 20% sheen change, a different anisotropy aspect, a wrong clearcoat alpha and swapped VNDF alphas.
  - **`tests_principled.rs`:** GPU vs CPU renders, all within 1% energy:
    - brushed / car paint / velvet: 38.41 vs 38.26;
    - wax sphere and a thin leaf slab with SSS: 40.82 vs 40.70;
    - a dispersive, absorbing glass slab over a glowing floor: 74.34 vs 73.62. It catches a dropped ×3 spectral weight and missing absorption.
    - a Preserve Texture Appearance sign: 48.76 vs 48.80;
    - plus the record layout.
- **Not done yet:**
  - **Random-walk exits ignore opacity:** the CPU ignores it too, but once see-through surfaces come to the GPU (PR 4), the walk should match whatever the CPU does then.
  - **Absorption needs a transmissive surface:** the GPU, like the CPU, applies it only when leaving one. Volumes (PR 3) are separate.
  - **Per-pixel parity at 24 px is coarse for small highlights.** The per-lobe test carries the exact check.

## 60. GPU kernel parity, PR 3: godrays and height mist on the GPU
- **Volumetric godrays / mist no longer send the scene to the CPU.** The CPU-only list is down to 7 entries.
- **Kernel (`wgsl/volume.wgsl`):** a port of `volumetrics.rs`:
  - the same march as the CPU: one jittered sample per step, `steps` steps over the segment, and misses march out to 6× the scene radius past the eye;
  - height falloff above the mist base, and FBM wisps drifted by the wind. The value noise hashes cells exactly like `sky_noise.rs`, so the wisps land in the same places;
  - Henyey-Greenstein single scattering of every punctual light, with a shadow ray when shadows are on;
  - energy-conserving step weights and Beer-Lambert transmittance on the path throughput.
  
  It runs before the surface hit and on misses too, like the CPU integrator.
- **Shared light falloff:** `incident()` in `lights.wgsl` (range window, near/far attenuation, spot cone) now serves both surface lighting and the march.
- **Parameters:** five new vec4s at the end of `Params` (`vol0..vol4`): albedo and density; base, falloff, wisps and 1/scale; wind, g and intensity; eye and reach; steps. They are zeros without mist.
- **Tests (`tests_volume.rs`):**
  - the kernel's wisp noise and phase function match the CPU at 256 points, including negative cells;
  - a low sun behind a wall with a gap (11.0 vs 11.0 energy);
  - a spot cone and an omni in patchy, drifting, back-scattering wisps (10.6 vs 10.5).
  - Mutation checks catch: no shadow rays in the mist, g forced to 0, wisps off, and transmittance not applied.
- **Not done yet:**
  - **Area lights don't light the mist on the GPU** (they still send the scene to the CPU, PR 4).
  - **Shadow rays in the mist ignore see-through surfaces** (opacity is PR 4); the CPU lets light through them.
  - **Mist uses the plain shadow test:** no soft shadows and no shadow colour, the same as the CPU march.

## 61. GPU kernel parity, PR 4a: area lights and IES profiles on the GPU
- **Area lights and IES profiles no longer send the scene to the CPU.** The CPU-only list is down to 5 entries: Shadow & Reflection Catcher, see-through (opacity), cel/toon, maps the GPU cannot bake, and more than 1.5 GB of baked textures.
- **Kernel (`wgsl/area.wgsl`):** a port of `area_lights.rs`:
  - Rect, Disc, Sphere and Tube sampling, using the same points and estimator scales as the CPU (Sphere and Tube sample the half facing the shading point);
  - `Shadow Samples` points per area light at each shading point, each with its own soft-shadow ray and shadow colour;
  - Visible to Camera: camera rays that reach an area light before any surface see its radiance (intensity over projected area) and set alpha;
  - the five IES webs (`ies()`), which shape both spots and area lights.
- **Shared code:** `incident()` in `lights.wgsl` now handles area lights too, so area lights also light the volumetric mist on the GPU, as on the CPU. The shadow test moved into `shadow()`, which point, spot, directional and area lights share.
- **Packing:** each light is 36 words now. The area block holds the X and Y axes with the half sizes, then shape, visible, IES id and sample count. `pack()` takes the render's shadow samples.
- **Tests (`tests_area.rs`):**
  - the kernel's IES webs match the CPU at 201 angles for every profile;
  - a rect softbox and a large disc (83.2 vs 83.3 energy, mean error 0.007);
  - a sphere and a neon tube seen by the camera;
  - an IES spot (Tactical Flashlight) and an IES panel (Wall Scallop);
  - the fallback test now uses a shadow catcher and opacity.
  - Mutation checks catch: the camera not seeing lights, the wrong Tube scale, the wrong Sphere scale, a missing 1/n sample weight, IES off on area lights, and a centre-heavy disc sample.
- **Not done yet:**
  - **Opacity, shadow catcher and toon still use the CPU** (PR 4b next: opacity first).
  - **No GPU test yet for area lights in mist**, although the shared `incident()` means they light it. The march takes one area sample per step, as on the CPU.
  - **A big area light almost touching geometry** (a disc 0.2 above a box) differed in one 4×4 block (GPU 2.0 vs CPU 2.8) at the test's low sample count. I didn't investigate further; normal set-ups agree.

## 62. GPU kernel parity, PR 4c: prefab materials on the GPU (live maps)
- **Prefab Library and Settlement scenes now render on the GPU.** Their materials use 3D maps (Noise, Cracks, Grunge, Wood) or Bricks on World / Triplanar projection. A UV grid cannot hold these, so before this PR the whole scene fell back to the CPU ("maps the GPU cannot bake").
- **Live maps (`live.rs`, `wgsl/live.wgsl`).** A material that cannot be baked but uses only maps the kernel knows gets a small record instead of a texture. The kernel then evaluates its maps at every hit.
  - Map kinds the kernel knows: Noise (Perlin / Fractal / Cellular), Bricks (all four kinds), Cracks, Grunge (Dirt / Rust / Scratches / Stains), Wood, Checker, Gradient and Stripes.
  - Channels: base colour, metallic, roughness, bump (central differences, as on the CPU) and emission.
  - It works with UV mapping and with World / Triplanar blending, using the same projection frames as `relief::triplanar_normal`.
  - The noise matches the CPU exactly: the noise crate's Perlin, Fbm and Worley are ported line by line, using its own permutation tables (uploaded once per scene, 9 × 256 words). The pattern maps' hash, fbm and Voronoi are ported too.
- **Memory:** live materials bake nothing (0 bytes of textures), so they also don't count towards the 1.5 GB cap.
- **CPU-only reason, reworded:** "maps the GPU cannot bake or evaluate (3D or non-repeating maps mixed with bitmaps, node graphs, procedural surfaces or paint)".
- **Tests (`tests_live.rs`):**
  - every map kind and sub-kind (19 materials) and every prefab live material (Settlement, Medieval Castle, Modern City, Alpine Terrain) are checked on colour, emission, metallic, roughness and bump normal. Each is evaluated at 96 points by a compute entry and compared with `base_surface` and `base_tangent_normal`; at most 2 of the 96 points may differ by more than 0.01 (f32 rounding at cell borders);
  - a Settlement street (cobble road, stone facade, rusted metal, curbs and a triplanar cracks wall) renders on the GPU like the CPU;
  - a bitmap mixed with a 3D map still falls back.
  - Mutation checks catch: the Perlin scale factor, Worley's far cell, the pattern hash, Rust-style `fract` on negative coordinates, the bump sign, emission without sRGB decoding, and the fractal noise lacunarity (2 instead of 2π/3).
- **Not done yet:**
  - **3D maps mixed with bitmaps, node graphs (Levels, Color Correct, Height Blend, …), procedural surfaces or paint still use the CPU.** Live maps take single map leaves only, which is all the prefab packs use.
  - **Opacity maps** (e.g. Settlement ID 29 smoke) still need the CPU; that comes with PR 4b.
  - ~~LightMix only works with the CPU path tracer~~: done in §63.
  - **Speed:** live maps cost more per hit than a baked lookup (Cracks runs 27 Voronoi cells plus several fbm calls, and bump adds four more evaluations). I haven't timed this on her RX 9060 XT; only lavapipe here.

## 63. GPU kernel parity: Luxon LightMix on the GPU
- **LightMix now works with the GPU path tracer** (Holly's report: "Luxon LightMix only works with CPU Pathtracer right now"). With `Luxon LightMix Buffers` on, a GPU render keeps its light groups, and the Frame Window's 💡 LightMix sidebar relights it with no re-render, just like a CPU render.
- **Kernel (`FEAT_LIGHTMIX`, mask bit 128):** every place the integrator adds light also adds that share to the light's group. The groups are in the CPU's order:
  - Sun & Dynamic Sky (the sky when it is procedural);
  - Environment (HDRI or flat background, ambient, fog);
  - Emissive Materials (emission, Unlit, emitter NEE);
  - Volumetric Godrays & Mist;
  - one group per scene light (punctual, spot, sun and area NEE, and area lights seen by the camera).
  - The per-light shares are weighted by the path throughput at that vertex (`mix_beta`), including the subsurface exit.
  - A sample that isn't finite adds to no group, as on the CPU.
  - Per-pixel group sums go to a storage buffer (binding 12), read back once at the end and normalised by `extras::lightmix`, the same code the CPU uses. Coverage and the G-buffer come from the GPU image too.
- **Host (`gpu_pathtrace/mix.rs`):**
  - The group count is passed in `vol4.y` and each light's group in `misc.w`.
  - A one-word buffer is bound when LightMix is off, so the variant without LightMix costs nothing.
  - A group buffer larger than the GPU's storage-buffer limit falls back to the CPU with the reason given.
- **Tests (`tests_mix.rs`):**
  - **Red and blue omnis, a sun, a glowing panel and fog:** every group's image mean matches the CPU within 4%.
  - **A rect area light visible to the camera, a spot and mist under the Earth sky:** Sun & Sky, Volume and per-light groups match.
  - **LightMix off:** no groups and a one-word buffer.
  - **Pipeline masks:** the mask test also builds LightMix on its own and on top of every scene feature.
  - Mutation checks are listed in the PR.
- **Not done yet:**
  - **AOV passes** (Direct, Indirect, Specular, Albedo, Normal, Depth and the rest) and the **albedo-guided À-Trous denoiser** are still CPU-only. A GPU render with LightMix uses the bilateral denoiser, like any GPU render.
  - **Memory:** at 3840×2160 with 12 groups the buffer takes about 1.2 GB. A GPU whose buffer limit is smaller falls back to the CPU instead of splitting the render into tiles.
  - **Speed** with LightMix on hasn't been timed on her RX 9060 XT; only lavapipe here.
  - Opacity, the Shadow & Reflection Catcher and toon still need the CPU (PR 4b).
