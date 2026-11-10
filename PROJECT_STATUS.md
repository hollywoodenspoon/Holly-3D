# MiniMax (Holly-3D): Project Status Report

_Status as of 9 November 2026, measured against the spec in `CLAUDE.md`._

## At a glance

| | |
|---|---|
| Build phases in `CLAUDE.md` | **5 of 5 done** (merged to `main` in PRs #1, #2 and #3) |
| Source | 162 `.rs` files, about 25,100 lines; the largest is 347 lines (limit 350) |
| Tests | 165 headless tests, all passing (`cargo test --lib`) |
| Lint | `cargo clippy --all-targets` reports no warnings |
| Stack | Bevy 0.20, bevy_egui 0.43.1, transform-gizmo-egui, slotmap, boolmesh, meshopt, uvunwrap, noise, rand/rand_chacha, image |

**Phase 6 in progress:** closing the §9 gaps one by one (✅ marks below). File I/O is done: `.mmax` scenes plus OBJ, glTF/GLB and FBX import and export.

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
| `Q` cycles selection-region shapes | ❌ | Rectangle marquee only; no circle, fence or lasso |
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
- ❌ **Bitmap Fit** is greyed out.
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
  - Choice of the built-in raytracer (BVH, soft shadows, reflection and refraction, anti-aliasing, tonemapping, outline pass) or the Cycles bridge, which writes a Cycles XML scene and runs the Cycles command-line tool.
- ✅ **Rendered Frame Window:**
  - Render, a viewport/camera picker, Region.
  - Save as PNG, EXR or JPG; Copy to Clipboard; Clone Render Window.
  - R, G, B, A and Mono channel buttons.
- ✅ **Cameras:** Target and Free Camera objects; Render Setup and the Frame Window list them next to the viewports.
- 🟡 The Cycles bridge needs a Cycles executable installed; its path is set in F10. It has never been run against a real Cycles install here.

### G. Lighting
- ✅ The default two-light headlight rig, with `Ctrl+L` and automatic switching to scene lights.
- ✅ Light types: Omni, Target Spot, Free Spot, Target Directional, Free Directional, Skylight.
- ✅ Light Parameters: on/off, type, shadows, multiplier, colour plus Kelvin presets, decay/range, Show Attenuation, hotspot and falloff, shadow bias, softness and colour.
- ✅ Hotspot and falloff cone gizmos.
- 🟡 Attenuation has a far range only; there is no separate **near** attenuation.
- ✅ The Environment dialog (`8`): five lighting rig presets, background colour, ambient tint, exponential fog.
- ❌ **HDRI skybox** (an image as the environment background).

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
| Edit Normals | ✅ | Cyan, blue and green lines; select by Normal, Vertex, Edge or Face; Break, Unify, Reset, Make Explicit. 🟡 The `E` rotation is covered by tests only and hasn't been tried in the app |
| Boolean (boolmesh) | ✅ | Union, A−B, B−A, Intersect; Pick Operand B; an operand list with Remove, Extract and on/off toggles; Merge Coplanar Faces. Open meshes are skipped with a warning |
| ProOptimizer (meshopt) | ✅ | Calculate, linked Vertex % and Vertex Count, keeps material, UV, colour and normal boundaries, Protect Borders |

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
| Stack action row | 🟡 | Make Unique, Delete and Collapse work. ❌ **Pin Stack**. Show End Result is greyed out because the end result is always shown |
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

---

## 9. Remaining work, by impact

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
8. **Selection-region shapes** cycled with `Q`: circle, fence, lasso.
9. **Smaller items:** HDRI environment background, near attenuation on lights, Bitmap Fit in UVW Map, Pin Stack.
10. **Checks still to do:** try Edit Normals `E` rotation in the app, and run the Cycles bridge against a real Cycles install.
