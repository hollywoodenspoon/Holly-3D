# CLAUDE.md — MiniMax: Clean 3ds Max-Style Modeler, Painter & Renderer in Rust

## 1. Core Philosophy & Rules
This is a personal 3D polygon modeling, painting, and rendering application built in Rust.
- **3ds Max Mental Model Only:** All UX, camera controls, hotkeys (`W/E/R`, `1-5`, `M`, `F9`, `F10`, `Shift+Drag`), gizmos, and data structures MUST strictly follow **3ds Max conventions**, NEVER Blender conventions (no modal `G`/`R`/`S` keys, no mandatory node graphs for basic tasks).
- **Clean, Spacious UI:** Replicate the spatial predictability of 3ds Max (Top Toolbar, 4-Way Viewport, Right-Hand Command Panel with Modifier Stack, Compact Material Editor, Rendered Frame Window), stripped of legacy clutter. Use generous padding, clean typography, and flat slate colors.
- **Strict Modularity:** NO `.rs` file may exceed **350 lines**. Split code cleanly into submodules (`src/mesh/`, `src/modifiers/`, `src/materials/`, `src/paint/`, `src/render/`, `src/splines/`, `src/ui/`, `src/viewport/`).
- **No Cyclic Pointers:** NEVER use `Rc<RefCell<T>>` or `Arc<Mutex<T>>` for mesh topology. Use index-based storage or `slotmap` arenas.
- **Test-Driven Geometry:** Every mesh operation and modifier MUST include a headless unit test verifying vertex, edge, and polygon counts (plus normal/UV validity) so geometry math is verified via `cargo test` before wiring to the UI.

---

## 2. Tech Stack & Crates (`Cargo.toml`)
Use these crates instead of writing low-level rendering, gizmo math, booleans, or UV packers from scratch:
- `bevy = "0.20.0"` — Core engine, PBR/Transmission/Toon rendering, picking, and viewport management.
- `bevy_egui = "0.43.1"` — UI panels, Command Panel, Floating Caddies, Quad Menus, Material Editor (`M`), and Render Setup (`F10`).
- `transform-gizmo-egui` (or `transform-gizmo-bevy`) — Explicit 3ds Max-style 3-axis Move, Rotate, and Scale gizmo handles with planar squares.
- `slotmap = "1.0"` — Arena storage for `EditablePoly` vertices, edges, and polygons.
- `glam` — Linear algebra (re-exported via Bevy's `bevy::math`).
- `noise = "0.9"` — Perlin, Worley/Cellular, and FBM 3D noise for the **Rock / Crystal / Stalagmite Generator**.
- `rand = "0.8"` & `rand_chacha = "0.3"` — Deterministic seeded RNG for Greeble and Rock/Crystal generators.
- `boolmesh = "0.2.2"` — Manifold 3D Boolean operations (Union, Subtract, Intersect).
- `meshoptimizer-rs = "0.2"` (or `meshopt`) — Quadric error metric decimation for the **ProOptimizer** modifier.
- `uvunwrap = "0.2.0"` — Automatic chart generation and island packing for **Blender-style Smart UV Unwrap** inside the **Unwrap UVW** modifier.
- `image = "0.25"` — Texture canvas buffer manipulation (for 3D Viewport Painting) and PNG/EXR render output.
- `obj` / `gltf` — Mesh import and export.

---

## 3. 3ds Max Controls & Hotkeys (Non-Negotiable)

### Viewport & Camera Navigation
- `Middle Mouse Button (MMB) Drag`: Pan viewport.
- `Alt + MMB Drag`: Orbit camera around current pivot or sub-object selection center.
- `Scroll Wheel` (or `Ctrl + Alt + MMB Drag`): Zoom in/out smoothly.
- `Z`: **Zoom Extents Selected** (frames selected object, or selected vertices/edges/polygons if in sub-object mode; frames whole scene if nothing is selected).
- `Alt + W`: Toggle between **Maximized Active Viewport** and **2x2 Four-Split Layout** (`Top`, `Front`, `Left`, `Perspective`).
- `G`: Toggle Home Grid visibility in active viewport.
- `F3`: Toggle **Wireframe** vs. **Default Shading**.
- `F4`: Toggle **Edged Faces** (wireframe overlay on shaded surfaces).
- `Alt + X`: Toggle **See-Through (X-Ray)** mode on selected object.

### Transform, Cloning & Snapping Hotkeys
- `Q`: Select Object mode (cycles selection region shapes).
- `W`: Select and Move (shows explicit 3-axis translation gizmo + XY/YZ/XZ plane handles).
- `E`: Select and Rotate (shows gimbal rotation rings).
- `R`: Select and Scale (uniform + axis/plane scale handles).
- **`Shift + Drag` on Move/Rotate/Scale Gizmo:** **3ds Max Clone Action!**
  - In **Object Mode**: Dragging an object while holding `Shift` duplicates it and pops up the **Clone Options** modal (`Copy`, `Instance`, `Reference`, plus `Number of Copies` spinner).
  - In **Edit Poly Edge/Border Mode (`2` or `3`)**: Holding `Shift` while dragging open edges extrudes new adjacent polygons directly out of those edges!
- `S`: Toggle 3D Grid/Vertex/Midpoint Snap.
- `A`: Toggle Angle Snap (e.g., 5-degree increments on rotation).
- `Ctrl + Z` / `Ctrl + Y`: Undo / Redo (must cover both transform changes and mesh/modifier/paint edits).
- `Delete`: Delete selected object (in Object mode) or selected sub-objects (in Vertex/Edge/Poly mode).

### Material & Rendering Shortcuts
- `M`: Toggle **3ds Max Compact Material Editor** window.
- `F9`: **Render Last / Quick Render** — Opens the **3ds Max Rendered Frame Window (RFW)** and renders the active camera/viewport immediately.
- `F10`: Toggle **Render Setup** window (Resolution, Anti-Aliasing, Environment/Lighting, Renderer Engine selector: *Built-in High-Quality Raytracer* vs *Cycles Bridge*).
- `8`: Environment & Effects dialog (Background color, HDRI skybox, Ambient light, Fog).

### Editable Poly / Edit Poly Sub-Object Modes
When an `EditablePoly` base object or `Edit Poly` modifier is active in the stack:
- `1`: **Vertex** (renders blue vertex dots; selected vertices turn red).
- `2`: **Edge** (click to select edges; `Alt + R` for Ring, `Alt + L` for Loop, or double-click edge for Loop).
- `3`: **Border** (selects open boundary edge loops around holes).
- `4`: **Polygon** (selects whole N-gon/quad/triangle faces, highlighted with translucent red fill).
- `5`: **Element** (selects contiguous connected polygon islands).
- *Note:* Pressing the currently active number key (`1–5`) a second time returns to **Top-Level Object Mode**.

---

## 4. Mesh & Scene Architecture (`src/mesh/`)

### Face-Corner Architecture (Crucial for Edit Normals, Vertex Paint & Unwrap UVW)
To behave like 3ds Max, 3D vertex positions MUST be decoupled from per-face-corner attributes so hard edges, smoothing groups, and UV seams work without physically duplicating 3D vertices:

    pub struct Vertex {
        pub position: Vec3,
        pub color: Vec4, // RGBA for VertexPaint / direct model painting without UVs!
        pub selected: bool,
    }

    pub struct FaceCorner {
        pub vertex_id: VertexId,
        pub uv: Vec2,
        pub explicit_normal: Option<Vec3>, // Set by Edit Normals; if None, computed from smoothing_group
    }

    pub struct Polygon {
        pub corners: Vec<FaceCorner>, // Supports Triangles, Quads, and N-gons (>= 3 corners)
        pub smoothing_group: u32,     // 32-bit bitmask representing 3ds Max Smoothing Groups 1..=32
        pub material_id: u16,
        pub selected: bool,
    }

    pub struct Edge {
        pub v0: VertexId,
        pub v1: VertexId,
        pub creasing: f32,
        pub selected: bool,
    }

    pub struct EditablePoly {
        pub vertices: SlotMap<VertexId, Vertex>,
        pub edges: SlotMap<EdgeId, Edge>,
        pub polygons: SlotMap<PolyId, Polygon>,
    }

### The Modifier Stack Pipeline (`src/modifiers/`)
Every `SceneObject` contains:
1. `base_object: BaseObject` — A **Parametric Primitive** (`Box`, `Cylinder`, `Sphere`, `Plane`, `Cone`, `Torus` with live dimension/segment sliders), an **Editable Spline** (`Line`, `Circle`, `Rectangle`, `Star`, `NGon`), a **GeoGen Procedural Mesh** (`Crystal`, `Rock`, `Stalagmite`), OR a collapsed **`EditablePoly`**.
2. `modifiers: Vec<ModifierEntry>` — Ordered bottom-to-top list of non-destructive modifiers. Each `ModifierEntry` has `enabled_in_viewport: bool`, `name: String`, and `kind: ModifierKind`.
3. `evaluated_mesh: EditablePoly` — Cached result of taking `base_object.to_editable_poly()` and folding every enabled `ModifierEntry` from bottom to top. Re-evaluate and upload to Bevy's GPU `Mesh` handle ONLY when the `base_object` or a modifier parameter is dirty.

## 5. Priority 1 Core Features (Build & Polish First!)

### A. Turn to Editable Poly & Edit Poly Modifier (Complete Right-Pane Toolkit)
Must include collapsible rollouts in the Right Command Panel that dynamically adapt based on active sub-object mode (`1–5`), using `[ Action ]` (80% width interactive drag) + `[ □ ]` (20% width Floating Caddy popup) split buttons:

- **Selection Rollout:**
  - 5 Sub-Object mode buttons: `[1: Vertex]` `[2: Edge]` `[3: Border]` `[4: Polygon]` `[5: Element]`.
  - `By Vertex` checkbox, `Ignore Backfacing` checkbox.
  - `Shrink` and `Grow` buttons (expands/contracts selection across adjacent topology).
  - `Ring` and `Loop` buttons + spinner (enabled in Edge/Border mode).

- **Soft Selection Rollout:**
  - `[ ] Use Soft Selection` checkbox, `Falloff`, `Pinch`, and `Bubble` spinners (colors affected vertices with a rainbow heat gradient from red to blue).

- **Edit Vertices Rollout (Mode `1`):**
  - `Remove` (deletes vertex and merges surrounding faces), `Break` (splits vertex into one independent vertex per adjacent face).
  - `Extrude` `[□]`, `Weld` `[□]` (merges selected vertices within distance threshold), `Chamfer` `[□]` (splits vertex into a quad/N-gon with open/invert option).
  - `Target Weld` (modal tool: click Vertex A then click target Vertex B to snap and weld A into B).
  - `Connect` (creates direct new edges between selected vertices that share a polygon).
  - `Remove Isolated Vertices` & `Remove Unused Map Verts`.

- **Edit Edges & Borders Rollout (Modes `2` & `3`):**
  - **`Insert Vertex`:** Interactive tool—click anywhere along an edge in the viewport to subdivide that edge and insert a new vertex at that exact point!
  - `Remove` (removes selected edge; holding `Ctrl` while clicking `Remove` also removes the orphaned vertices cleanly!).
  - `Split` (splits mesh along selected edges), `Extrude` `[□]`, `Weld` `[□]`, `Target Weld`.
  - `Chamfer` `[□]` (Parameters: `Amount`, `Segments`, `Tension`, `Open/Invert` toggle, `Mitering: Quad/Tri`).
  - `Connect` `[□]` (Parameters: `Segments: 1..N`, `Pinch: -100..100`, `Slide: -100..100` — connects selected parallel/ringed edges with new edge loops).
  - `Bridge` `[□]` (Parameters: `Segments`, `Taper`, `Twist` — builds polygon bridge between two selected edge sets or borders).
  - `Cap` (Border mode: caps selected open boundary loop into a new polygon).
  - `Create Shape from Selection` (extracts selected edges into a new `EditableSpline` object—Linear or Smooth).

- **Edit Polygons & Elements Rollout (Modes `4` & `5`):**
  - `Insert Vertex` (click inside a polygon to poke/subdivide a new vertex connected to its corners).
  - `Extrude` `[□]` (Modes: `Group`, `Local Normal`, `By Polygon`; Parameter: `Height`).
  - `Outline` `[□]` (scales selected face loops inward/outward).
  - `Bevel` `[□]` (Modes: `Group`, `Local Normal`, `By Polygon`; Parameters: `Height`, `Outline Amount`).
  - `Inset` `[□]` (Modes: `Group`, `By Polygon`; Parameter: `Inset Amount`).
  - `Bridge` `[□]` (connects two selected polygons/elements with a tunnel or column and deletes the end caps).
  - `Flip` (reverses polygon vertex winding / face normals).
  - `Hinge From Edge` `[□]` (picks an edge as a hinge and extrudes selected polygons along an arc by `Angle` and `Segments`).
  - `Extrude Along Spline` `[□]`.

- **Edit Geometry Rollout (Available in ALL Modes `1–5`):**
  - `Repeat Last` button (repeats the previous Edit Poly action with one click).
  - `Create` (click existing or new 3D points to build a polygon manually).
  - `Collapse` (welds selected vertices/edges/polygons to their selection center).
  - `Attach` (merges another scene mesh into this EditablePoly) & `Detach` (separates selected polygons into a new Object, Element, or Clone).
  - **`Slice Plane` + `Slice` + `Split` checkbox + `Reset Plane`:**
    - Toggling `Slice Plane` activates an interactive yellow 3D wireframe plane gizmo in the viewport that can be moved/rotated with `W`/`E`.
    - Clicking `[ Slice ]` cuts new edges and vertices across the entire mesh (or selected polygons) along that plane! If `Split` is checked, it also splits the mesh along the cut.
  - **`QuickSlice`:** Interactive viewport tool—click two points on screen to draw a slicing line right across the mesh from the current camera angle.
  - **`Cut`:** Interactive viewport topology knife—click sequentially from Vertex -> Edge -> Face anywhere on the surface to draw new edges and add vertices directly.
  - `MSmooth` (applies local subdivision smoothing to selected polygons) & `Tessellate` `[□]` (`Edge` vs `Face-Center`).
  - **`Make Planar` + `[ X ]` `[ Y ]` `[ Z ]` buttons:**
    - `Make Planar` flattens selected vertices/edges/polygons onto their average plane.
    - `[ X ]`, `[ Y ]`, and `[ Z ]` immediately flatten the selection to the exact average X, Y, or Z coordinate (essential 3ds Max hard-surface workflow!).

- **Polygon: Material IDs & Smoothing Groups Rollout (Mode `4`):**
  - `Set ID` spinner (`1..=64`) + `Select ID` button (for multi/sub-object materials).
  - **3ds Max 32-Bit Smoothing Group Grid:** The iconic 4x8 grid of numbered toggle buttons (`1` through `32`), plus `[ Clear All ]` and `[ Auto Smooth ]` button with threshold angle spinner (default `45.0°`).

---

### B. FFD (Free-Form Deformation) Modifiers (`src/modifiers/ffd.rs`)
- **FFD 2x2x2, FFD 3x3x3, FFD 4x4x4:** Tricubic Bernstein polynomial lattice deformer fitted to the mesh's local bounding box.
- Expanding the FFD modifier in the Modifier Stack reveals two sub-objects: `Control Points` (`1`) and `Lattice` (`2`).
- Selecting `Control Points` renders orange control point handles connected by yellow cage lines; the user can box-select any combination of lattice points and transform them with `W`, `E`, and `R` to warp the underlying mesh smoothly in real time.
- Rollout buttons: `[ Conform to Shape ]`, `[ Reset ]`, `[ All X / All Y / All Z ]` selection filters.

---

### C. UVW Map (`uvw_map.rs`) & Unwrap UVW (`unwrap_uvw.rs`)
- **UVW Map Modifier:**
  - Mapping types: `Planar`, `Cylindrical` (with `Cap` checkbox), `Spherical`, `Shrink Wrap`, `Box`, `Face`, `XYZ to UVW`.
  - Parameters: `Length`, `Width`, `Height` spinners, `U Tile`, `V Tile`, `W Tile` spinners + `Flip` checkboxes.
  - Alignment section: `X`, `Y`, `Z` axis radio buttons + `[ Fit ]`, `[ Center ]`, `[ Bitmap Fit ]`, `[ Normal Align ]`, `[ View Align ]`, `[ Reset ]`.
  - Expanding `UVW Map` in the stack selects the **Gizmo** sub-object so the user can move, rotate, or scale the yellow projection wireframe in the viewport.
- **Unwrap UVW Modifier:**
  - `[ Open UV Editor... ]` button opens a clean floating 2D UV window with a toggleable UV Checkerboard pattern in the 3D viewport, 2D Vertex/Edge/Polygon selection, and 2D Move/Rotate/Scale/Relax/Stitch tools.
  - **Blender-Style Smart UV Project Rollout:** Powered by `uvunwrap`. Includes a single-click `[ ✨ Smart UV Unwrap ]` button with `Angle Limit` (default `66.0°`), `Island Margin` (`0.02`), and `Correct Aspect` toggle—automatically cuts seams by angle, flattens charts, and packs islands cleanly into `0..1` space.

---

### D. Materials (`M` Key) — Classic 3ds Max Slots + Presets + Outlines (`src/materials/`)
- **3ds Max Compact Material Editor (`M`):**
  - Top grid of **24 Material Sample Slots** (4x6 grid showing live shaded preview spheres/swatches).
  - Classic Material Toolbar directly beneath the slots:
    - `[ Assign Material to Selection ]` | `[ Show Shaded Material in Viewport ]` | `[ Reset Material Slot ]` | `[ Make Material Copy ]` | `[ Pick Material from Object (Eyedropper) ]`.
- **Shading Model & One-Click Preset Library:**
  - **Shading Model Selector:** `Standard / Physical PBR`, `Cel / Toon Ramp`, `Refractive Crystal / Glass`, `Unlit / Emissive`.
  - **Quick Preset Dropdown (1-Click Setup):**
    1. *3ds Max Default Modeling Clay*
    2. *Brushed Steel / Gunmetal* (`Metallic: 0.90, Roughness: 0.25`)
    3. *Polished Gold / Chrome* (`Metallic: 1.00, Roughness: 0.05`)
    4. *Matte Rubber / Vulcanized Plastic* (`Metallic: 0.00, Roughness: 0.85`)
    5. *Natural Wood / Procedural Grain* (3D wood-ring noise + warm roughness map)
    6. *Translucent Crystal / Quartz Gem* (`Specular Transmission: 0.92, IOR: 1.55, Roughness: 0.06, Thickness & Subsurface Attenuation Color`)
    7. *Cel-Shaded Anime / Tactics Style* (Quantized 2-step or 3-step light ramp, custom shadow tint, rim highlight, and ink outline enabled)
- **Universal Ink / Toon Outline Rollout (Available on EVERY Material!):**
  - Every material—whether Cel-Shaded, Realistic Metal, Wood, or Crystal—has an `[✓] Render Ink Outline` checkbox with `Outline Width` (`0.0..=10.0`), `Outline Color` picker, and `[✓] Draw Crease / Sharp Angle Lines` (with angle threshold spinner).
  - Implemented via an inverted-hull back-face extrusion pass + sharp-edge wireframe pass so outlines work seamlessly on realistic PBR materials and toon materials alike.
- **Maps Rollout & Optional Slate Node View:**
  - Classic 3ds Max **Maps Rollout** with checkboxes, amount spinners (`0..100`), and file/procedural map slots for `Base Color / Diffuse`, `Metallic`, `Roughness`, `Bump / Normal`, `Opacity`, and `Emission`, plus a 1-click `[ Import PBR Texture Folder ]` button.
  - Toggle button `[ Mode: Compact | Slate Node Graph ]` for users who want to wire procedural `Noise`, `Checker`, `Gradient Ramp`, or `Color Correction` nodes into material inputs.

---

### E. Direct 3D Model Painting (`src/paint/`)
Accessible via the `[ 🖌 Viewport Paint ]` button on the top toolbar or by adding a **VertexPaint Modifier**:
- **Mode 1: 3ds Max VertexPaint (Zero UVs Required):**
  - Paint RGBA color, metallic, or roughness directly onto mesh vertices (`Vertex.color`).
  - Floating **PaintBox Palette** (just like 3ds Max's VertexPaint floater): Brush `Size`, `Opacity`, `Falloff` curve, `Color Swatch`, `[ All / Selected Faces Only ]` mask, `[ Fill Object ]` bucket, `[ Blur ]` brush, and `Layer Opacity` slider.
- **Mode 2: Direct 3D Texture Projection Paint:**
  - Paint directly onto a `2048x2048` (or `4096x4096`) BaseColor texture map in the 3D viewport via raycast UV lookup.
  - If the model has no UVs yet, clicking `[ Auto-Setup Paint Canvas ]` automatically runs a non-overlapping Smart UV Project and attaches a fresh blank texture so you can start painting on any raw mesh in 1 second.

---

### F. Rendering (`F9` & `F10` — 3ds Max Render Setup & Frame Buffer) (`src/render/`)
- **`F10` Render Setup Dialog:**
  - **Common Tab:** Output Resolution presets (`800x600`, `1280x720`, `1920x1080`, `2048x2048`, `3840x2160`), `Image Aspect` lock, `[✓] Transparent Background (Alpha)` toggle, Frame/Camera selector.
  - **Renderer Engine Selector:**
    1. **Built-in High-Quality Raytracer / PBR Engine (Default):** High-sample offscreen render target with Soft Shadows, Screen-Space Reflections/GI, Refraction for Crystals, Anti-Aliasing, Tonemapping, and the Universal Outline Pass.
    2. **Cycles Standalone Bridge:** Exports the evaluated scene, lights, camera, and PBR materials to a temporary scene description and invokes headless Cycles CLI in the background, streaming the rendered image directly into the Frame Buffer window.
- **`F9` Rendered Frame Window (RFW):**
  - Replicates the classic **3ds Max Rendered Frame Window** with top toolbar controls:
    - `[ Render ]` button | `Viewport / Camera` dropdown | `[ Region / Crop ]` toggle | `[ 💾 Save Image (.PNG / .EXR / .JPG) ]` | `[ Copy to Clipboard ]` | `[ Clone Render Window ]` (to compare two renders side-by-side) | `[ R ] [ G ] [ B ] [ A ] [ Monochrome ]` channel buttons.

## 6. Priority 2 Features (Splines, Clone/Array & Secondary Modifiers)

### A. Splines + Lathe / Extrude (`src/splines/`)
- **Create Tab -> Shapes (Splines):**
  - `Line` (click to place sharp Corner points, click-and-drag to pull smooth Bezier handles, right-click to finish, prompt to close spline if clicking first vertex).
  - Parametric Shapes: `Circle`, `Arc`, `NGon`, `Rectangle`, `Ellipse`, `Star`, `Helix`.
  - **Rendering Rollout (on every Spline):** Classic 3ds Max `[ ] Enable in Renderer` and `[ ] Enable in Viewport` checkboxes with `Radial` (`Thickness`, `Sides`, `Angle`) vs `Rectangular` (`Length`, `Width`, `Angle`) profile options so any spline can immediately become a 3D pipe, cable, or bar!
- **Editable Spline Sub-Objects:**
  - `1: Vertex` (Right-click to switch vertex tangent type: `Bezier Corner`, `Bezier`, `Corner`, `Smooth`; plus `Fillet` `[□]`, `Chamfer` `[□]`, `Weld`, `Refine / Insert`, `Make First`).
  - `2: Segment` (`Divide`).
  - `3: Spline` (`Outline` `[□]`, `Boolean: Union/Subtract/Intersect`, `Mirror`).
- **Lathe Modifier (`src/modifiers/lathe.rs`):**
  - Revolves a 2D spline profile around an axis into a 3D mesh (vases, pillars, goblets, wine glasses, turret barrels).
  - Parameters: `Degrees` (`0.0..=360.0`), `Segments` (`3..=128`), `[✓] Weld Core`, `[✓] Flip Normals`, `Cap Start / Cap End`, Direction (`X`, `Y`, `Z`), and Alignment buttons (`[ Min ]`, `[ Center ]`, `[ Max ]`) + interactive `Axis` sub-object.
- **Spline Extrude Modifier (`src/modifiers/spline_extrude.rs`):**
  - Extrudes any closed 2D spline into a 3D mesh with `Amount`, `Segments`, and `Cap Start / Cap End`.

---

### B. 3ds Max Array Dialog (`src/ui/array_dialog.rs`)
- Accessible from `Tools -> Array...` or the top toolbar `[ Array... ]` button.
- Replicates the classic **3ds Max Array Dialog**:
  - **Array Transformation Grid:** Two side-by-side sections for **Incremental** (step per copy) and **Totals** (total offset across all copies) with `X`, `Y`, `Z` spinners for `Move`, `Rotate`, and `Scale` + `[ Re-Orient ]` and `[ Uniform ]` checkboxes.
  - **Type of Object:** `Copy` | `Instance` | `Reference`.
  - **Array Dimensions:**
    - `1D`: `Count` spinner (radial or linear arrays).
    - `2D`: `Count` + `Incremental Row Offsets (X/Y/Z)`.
    - `3D`: `Count` + `Incremental Layer Offsets (X/Y/Z)`.
  - **`[✓] Preview` Checkbox:** Live-updates the array instances in the 3D viewport while scrubbing spinners before clicking `[ OK ]` or `[ Cancel ]`.

---

### C. Procedural Generators & Remaining Stack Modifiers (`src/modifiers/`)

1. **GeoGen — Procedural Rock, Crystal & Stalagmite Generator (`geogen.rs`):**
   - Available both in `Create -> Geometry -> GeoGen` (spawns a standalone procedural object) AND in the `Modifier List` (grows crystals/rocks/stalagmites on selected polygons or deforms an existing mesh).
   - **Generator Mode Selector:**
     - `Quartz Crystal Cluster`: Hexagonal/polygonal prism spires growing from a cluster base with random tilt angles, varied heights, sharp pyramidal termination tips, and optional side-shard offshoots.
     - `Jagged Rock / Boulder`: Subdivided convex base deformed by 3D Worley/FBM noise (`noise` crate) and carved by seeded random 3D Voronoi planar slices to produce crisp, flat fractured rock planes and sharp ridges.
     - `Layered Cliff / Slate`: Stratified horizontal terrace stepping + vertical fracture cracks.
     - `Stalagmite / Stalactite`: Tapered vertical limestone drip formations with fluted vertical ridges and stepped ring bulges.
   - **Parameters Rollout:**
     - `Random Seed` spinner + `[ 🎲 New Seed ]` button.
     - `Count / Cluster Spires` (`1..=50`), `Spread Angle` (`0°..=90°`).
     - `Min Height`, `Max Height`, `Base Radius`, `Tip Taper`.
     - `Facet Sharpness / Planar Cuts` (`0..=30`), `Noise Strength`, `Noise Scale`, `Auto-Assign Smoothing Groups` toggle (keeps crystal facets sharp and rock ridges defined).

2. **Greeble (`greeble.rs`):**
   - Classic Tom Hudson-style sci-fi hull/city detail generator applied to all polygons (or selected polygons passed up the stack).
   - **Panels Rollout:** `Random Seed`, `Min Height`, `Max Height`, `Taper` (inward bevel on panel tops), `[✓] Select Tops`, `[✓] Generate Panels`.
   - **Widgets / Nurnies Rollout:** `Subdivide Levels` (`1..=3`), `Widget Density` (`0.0..=1.0`), `Min/Max Widget Size`, Widget Types (`Boxes`, `Cylinders`, `Ribs/Ledges`), and separate `Panel Material ID` vs `Widget Material ID`.

3. **Chamfer Modifier (`chamfer.rs`):**
   - Non-destructive stack modifier beveling edges across the mesh.
   - Parameters: `Amount`, `Segments`, `Tension` (`0.0` flat to `1.0` curved), `Selection Filter` (`All Edges`, `Selected Edges`, `By Smoothing Group Boundaries`, or `By Angle Threshold: Min/Max Angle`).

4. **Symmetry (`symmetry.rs`):**
   - Parameters: `Mirror Axis` (`X`, `Y`, `Z`), `[ ] Flip`, `[✓] Slice Along Mirror`, `[✓] Weld Seam` + `Threshold` spinner.
   - Expanding `Symmetry` in the stack selects the **Mirror** sub-object gizmo so the user can move and rotate the symmetry plane with `W` and `E`.

5. **Shell (`shell.rs`):**
   - Parameters: `Inner Amount`, `Outer Amount`, `Segments`, `[✓] Straighten Corners`, `[ ] Override Edge Material ID`, `Select Inner/Outer/Edge Faces` flags.

6. **TurboSmooth (`turbosmooth.rs`):**
   - Catmull-Clark subdivision surface algorithm.
   - Parameters: `Iterations` (`0..=4`), `[✓] Isoline Display` (draws only simplified cage edges in wireframe overlay), `[ ] Separate by Smoothing Groups` (keeps hard smoothing-group boundaries crisp before smoothing), `[ ] Separate by Materials`.

7. **Classic Deformers (`deformers.rs`):**
   - **Bend:** `Angle`, `Direction`, `Bend Axis` (`X/Y/Z`), `[ ] Limit Effect` + `Upper/Lower Limit`, movable `Gizmo`/`Center` sub-object.
   - **Twist:** `Angle`, `Bias` (`-100..100`), `Twist Axis` (`X/Y/Z`), `Limits`.
   - **Taper:** `Amount` (`-10..10`), `Curve`, `Primary Axis`, `Effect Axis`, `[ ] Symmetry`.
   - **Push:** `Push Value` spinner (offsets every vertex along its averaged vertex normal).

8. **Edit Normals (`edit_normals.rs`):**
   - Draws cyan (unspecified), blue (unified), and green (explicit) normal vector lines from every `FaceCorner` in the viewport with a `Display Length` spinner.
   - Selection modes: `By Normal`, `By Vertex`, `By Edge`, `By Face`.
   - Operations: `[ Break ]` (splits shared vertex normals into independent corner normals for hard shading), `[ Unify / Average ]` (smooths selected normals), `[ Reset ]` (returns normals to Smoothing Group control), `[ Make Explicit ]`, plus direct rotation using the `E` Rotate Gizmo.

9. **Boolean (`boolean.rs` via `boolmesh`) & ProOptimizer (`pro_optimizer.rs` via `meshoptimizer-rs`):**
   - **Boolean:** `Union`, `Subtract (A-B)`, `Subtract (B-A)`, `Intersect`, `[ Pick Operand B ]` button, plus a live operand list so multiple boolean operands can be stacked like 3ds Max `ProBoolean`.
   - **ProOptimizer:** `[ Calculate ]` button, `Vertex %` (`1.0%..=100.0%`) and `Vertex Count` linked spinners, `[✓] Keep Material Boundaries`, `[✓] Keep UV Boundaries`, `[✓] Keep Vertex Colors`, `[✓] Keep Normals`.

## 7. UI & Visual Design ("Clean 3ds Max" Specification)

Configure `egui::Style` and `egui::Visuals` on startup in `src/ui/theme.rs`:

### Color Palette & Styling
- **Main Panel Fill (`panel_fill`):** `#222327` (Clean dark neutral slate).
- **Rollout Header / Card Fill (`faint_bg_color`):** `#2B2D31`.
- **Input / Stack Box Fill (`extreme_bg_color`):** `#18191C`.
- **3ds Max Signature Amber (`#E59C24`):** Use for active `Q/W/E/R` buttons, active Sub-Object buttons (`1–5`), active Snap toggles, and the currently highlighted row in the Modifier Stack.
- **Secondary Accent (`#3A7AFE`):** Crisp blue for hover states.
- **Spacing & Corners:** `item_spacing = vec2(6.0, 6.0)`, `button_padding = vec2(8.0, 5.0)`, `corner_radius = 4.0`.
- **3ds Max Spinner Behavior (Crucial Helper Function):**
  Wrap every numeric `egui::DragValue` in a helper `max_spinner(ui, &mut value, speed, default_val)` so that **Right-Clicking any numeric spinner immediately resets its value to `default_val` (usually `0.0`)**.

### Fixed 5-Zone Layout (`src/ui/layout.rs`)
1. **Top Main Toolbar (`36px` height):**
   - Left: `File` (New, Open, Save, Import OBJ/glTF, Export OBJ/glTF), `Edit` (Undo, Redo, Clone, Delete), `Tools` (Array, Mirror, Align).
   - Center: `[Q]` `[W]` `[E]` `[R]` mode buttons | Coordinate System (`View`, `World`, `Local`) | Pivot Mode (`Pivot Point`, `Selection Center`, `Transform Coordinate Center`) | `[S] Snap` `[A] Angle Snap` | `[Mirror]` `[Array...]` `[🖌 Paint]`.
   - Right: `[ ⚪ Material Editor (M) ]` | `[ ⚙ Render Setup (F10) ]` | `[  teapot/camera Quick Render (F9) ]`.
2. **Left Scene Explorer (`200px` width, collapsible with `H`):**
   - Clean outliner of Scene Objects with `[👁]` visibility toggle, `[❄]` freeze toggle, object type icon (Mesh, Spline, Light, Camera), and search filter.
3. **Center 3D Viewport(s):**
   - Top-Left overlay text buttons matching 3ds Max: `[ + ] [ Perspective ] [ Default Shading + Edged Faces ]`.
   - Supports single maximized view or 2x2 split (`Alt + W`) with a gold border (`#E59C24`) around the active quadrant and an `X/Y/Z` axis tripod in the bottom-left corner.
4. **Right Command Panel (`310px` fixed width):**
   - **Top Tab Row:** `[ ➕ Create ]` | `[ 🌈 Modify ]` | `[ ✥ Pivot ]`.
   - **Create Tab:** Sub-category icons for `Geometry` (`Box`, `Sphere`, `Cylinder`, `Plane`, `Torus`, `Cone`, `GeoGen Rock/Crystal`), `Shapes / Splines` (`Line`, `Circle`, `Rectangle`, `Star`, `NGon`), and `Lights / Cameras` (`Directional`, `Point`, `Spot`, `Target Camera`).
   - **Modify Tab:**
     1. Object Name text box + Wireframe Color swatch on the right.
     2. **Searchable Modifier List Dropdown:** Typing filters all modifiers; clicking adds it to the top of the stack.
     3. **Modifier Stack Box (`150px` height):** Renders top-to-bottom. Each row has `[👁]` toggle, `[▸]` sub-object expander (`Vertex`, `Edge`, `Border`, `Polygon`, `Element`, or `Gizmo`/`Mirror Plane`/`Control Points`), and Modifier Name.
     4. **Stack Action Row:** `[ 📌 Pin ]` `[ Show End Result ]` `[ Make Unique ]` `[ 🗑 Delete Modifier ]` `[ Collapse To Editable Poly ]`.
     5. **Collapsible Rollouts (`egui::CollapsingHeader`):**
        - *Selection Rollout:* 5 big, clean buttons `[1: Vtx]` `[2: Edg]` `[3: Brd]` `[4: Pol]` `[5: Elm]` that light up in `#E59C24` when active.
        - *Edit Geometry / Sub-Object Rollouts:* 2-column grid of **Split Caddy Buttons** (`80%` width main button `[ Extrude ]` + `20%` width settings button `[ □ ]`).
        - *Parameters Rollout:* Sliders and right-click-resettable spinners for the currently selected modifier or base primitive.
   - **Pivot Tab:** `[ Affect Pivot Only ]` toggle, `[ Center to Object ]`, `[ Align to World ]`, `[ Bottom-Center Pivot (Z=0) ]`, and `[ Reset XForm ]`.
5. **Bottom Status & Transform Type-In Bar (`28px` height):**
   - Left: Mesh statistics (`Selected: 4 Polygons | Verts: 512 | Edges: 1,024 | Polys: 514`).
   - Right: `[Abs / Off]` mode toggle + `X: [ 0.000 ]` `Y: [ 0.000 ]` `Z: [ 0.000 ]` spinners (right-click any spinner to zero out).

### Floating In-Viewport Caddy & Right-Click Quad Menu
- **Floating Caddy (`[ □ ]`):** Clicking `[ □ ]` next to any Edit Poly operation opens a small, sleek floating `egui::Window` offset 60px from the viewport center with live preview spinners, `[ ✓ OK ]`, `[ ➕ Apply & Continue ]`, and `[ ✕ Cancel ]`.
- **Right-Click Quad Menu:** Right-clicking in the viewport opens a clean 4-quadrant context popup at the mouse cursor:
  - *Tools 1 (Top-Left):* Extrude, Chamfer, Bevel, Inset, Connect, Target Weld, Cut, QuickSlice.
  - *Transform (Bottom-Right):* Move, Rotate, Scale, Clone, Align.
  - *Display (Top-Right):* Hide Selected, Unhide All, Freeze Selected, Isolate Selection (`Alt + Q`).
  - *Convert To (Bottom-Left):* **Convert to Editable Poly**, **Convert to Editable Spline**.

---

## 8. Phased Build Order (Strictly Prioritized for User Workflow)
When building or extending the application, execute these phases in exact sequence and verify compilation (`cargo check` and `cargo test`) after each step:

- **Phase 1 (Viewport, Clean 3ds Max UI & Base Primitives):**
  Scaffold Cargo project, custom `#222327` + `#E59C24` theme, 5-zone UI layout, 3ds Max Camera (`MMB` Pan, `Alt+MMB` Orbit, `Z` Zoom Extents, `Alt+W` 4-view, `F3`/`F4` Wireframe/Edged Faces), `Q/W/E/R` gizmos, `Shift+Drag` Clone dialog, and Parametric Primitives (`Box`, `Cylinder`, `Sphere`, `Plane`, `Cone`, `Torus`).

- **Phase 2 (Turn to Editable Poly & Complete Right-Pane Edit Poly Toolkit):**
  Implement `EditablePoly` Face-Corner structure, Modifier Stack UI, `Convert to Editable Poly`, **Edit Poly modifier**, sub-object raycast selection (`1–5`), and the full right-pane modeling tools: **Extrude, Chamfer, Bevel, Inset, Connect, Target Weld, Weld, Break, Remove, Insert Vertex (on Edges/Polys), Cut, Slice Plane, QuickSlice, Cap, Bridge, Make Planar X/Y/Z, Attach/Detach, and 1–32 Smoothing Groups**. Include Floating `[□]` Caddies and Right-Click Quad Menu.

- **Phase 3 (FFD, UVW Map & Unwrap UVW):**
  Implement **FFD 2x2x2 / 3x3x3 / 4x4x4** (with interactive lattice control points), **UVW Map** (Planar, Cylindrical, Spherical, Box with interactive gizmo), and **Unwrap UVW** (2D UV Editor window + 1-click **Smart UV Project** via `uvunwrap`).

- **Phase 4 (Materials `M`, 3D Model Painting & `F9`/`F10` Rendering):**
  Implement the **Compact Material Editor (`M`)** with 24 sample slots, PBR/Crystal/Wood/Rubber/Metal/Cel-Shade presets, Universal Ink Outlines on any material, **3D Viewport Vertex & Texture Painting**, **Render Setup (`F10`)**, and the **Rendered Frame Window (`F9`)** with PNG/EXR export + Cycles CLI bridge option.

- **Phase 5 (Splines, Lathe, Array, Rock/Crystal Generator & Remaining Modifiers):**
  Implement **Splines + Lathe + Spline Extrude**, the **3ds Max Array Dialog**, **GeoGen (Crystals, Rocks, Stalagmites)**, **Greeble**, **Symmetry**, **Shell**, **TurboSmooth**, **Bend/Twist/Taper/Push**, **Edit Normals**, **Boolean (`boolmesh`)**, and **ProOptimizer (`meshoptimizer-rs`)**.
