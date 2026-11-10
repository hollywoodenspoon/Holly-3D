# MiniMax Phase 9 Plan: 3ds Max Workflow, Materials and GhostTown

_Agreed 10 November 2026. Each step is one PR, merged before the next one starts._

## Ground rules

- **A button for everything.** Every feature must be reachable from a visible button, dropdown or menu entry. Hotkeys are shortcuts only, never the only way in.
  - This rule goes into `CLAUDE.md` §1.
  - A test enforces it: every action lives in one action table (which also feeds the hotkey editor). The test fails if any action has no button or menu entry.
- **The usual rules for every PR:**
  - every `.rs` file stays at 350 lines or fewer
  - `cargo clippy --all-targets` reports no warnings
  - headless tests cover each feature
  - `PROJECT_STATUS.md` is updated
  - the work is pushed to `claude/3ds-max-ui-tools-setup-558yf8` with a PR to merge
- **Old scenes keep loading.** New fields get defaults, and old presets keep their names.

## Order

| # | PR | Contents |
|---|---|---|
| 1 | Button-rich interface | Main toolbar, top menus, Command Panel tabs, Modeling Ribbon, viewport navigation buttons, the full grouped Modifier List, and the "button for everything" rule with its test |
| 2 | Preferences | Themes, the hotkey editor, and viewport, general and rendering settings, plus the Assets tab (its downloads arrive with PR 5) |
| 3 | Click-and-drag creation | Every Create button builds its object with the mouse, as in 3ds Max |
| 4 | Help | A Help window with step-by-step pages and screenshots of the real program |
| 5 | Material library | About 280 presets (procedural and photo), new pattern maps, triplanar mapping, the asset downloader |
| 6 | GhostTown variety | New settlement types and building types, building and part toggles, rivers and bridges, the emissive toggle, prop styles |
| 7 | Destruction | Much heavier destruction: toppled buildings, twisted rebar and more |

The general basics of how the program works (PRs 1–4) come first. Help follows them so its screenshots show the finished interface, and each later PR adds its own Help pages.

---

## PR 1: A button-rich, 3ds Max-style interface

### Main toolbar (icon buttons with tooltips that show the hotkey)
- **Undo / Redo**
- **Selection:**
  - Select Object
  - Select by Name, which opens a pick list
  - a region shape dropdown (rectangle, circle, fence, lasso)
  - a Window / Crossing toggle
  - a Selection Filter dropdown (All, Geometry, Shapes, Lights, Cameras)
- **Transforms:**
  - Move, Rotate, Scale
  - a Reference Coordinate dropdown
  - a Pivot Centre dropdown
- **Snaps:** Snap, Angle Snap, Percent Snap, Snap Settings
- **Tools:** Mirror, an Align dropdown, Array, Clone
- **Lighting:** Default Lights, ☀ Environment (sky and deep space)
- **Rendering:** Material Editor, Render Setup, Rendered Frame Window, Render
- **App:** Help, Preferences

### Top menus
**File · Edit · Tools · Views · Create · Modifiers · Rendering · Help**
- **Views:**
  - Grid, Wireframe, Edged Faces, See-Through
  - Zoom Extents, Zoom Extents All
  - Maximize Viewport, Scene Explorer
- **Create:** every object type, grouped.
- **Modifiers:** every modifier, grouped.
- **Rendering:**
  - Environment, with sky presets
  - Default Lights
  - Render Setup, Render
- **Help:** the Help window, the Getting Started tour, the hotkey cheat sheet, Credits.
- **Hotkey names:** every menu entry shows its hotkey.

### Features that today work only from the keyboard, and where they go

| Hotkey | Where it will be |
|---|---|
| Environment / Sky / Deep Space (`8`) | Toolbar button, Rendering menu, and a Sky dropdown with presets (Clear Day, Sunset, Overcast, Night, Deep Space, Nebula) |
| Default Lights (`Ctrl+L`) | Toolbar toggle and Rendering menu |
| Grid (`G`), Wireframe (`F3`), Edged Faces (`F4`), See-Through (`Alt+X`) | The viewport shading dropdown and the Views menu |
| Zoom Extents (`Z`, `Shift+Z`) | Views menu and the navigation buttons |
| Scene Explorer (`H`) | Views menu and a toolbar button |
| Joint tool (`J`) | Assembly rollout and Tools menu |
| Undo / Redo, Save / Open | Toolbar buttons plus the menus |

### Viewport navigation buttons (bottom-right, like 3ds Max)
- Zoom, Zoom All
- Zoom Extents, Zoom Extents All
- Field of View, Pan, Orbit
- Maximize Viewport Toggle

### Command Panel tabs
**Create · Modify · Hierarchy** (today's Pivot tab) **· Display · Utilities**
- **Display:**
  - Hide and Freeze by category
  - Hide Selected, Unhide All
  - Freeze, Unfreeze
  - display properties
- **Utilities:** Reset XForm, Collapse, Measure, Asset Packs.

### Modeling Ribbon (optional, toggled from a button)
A bar of big buttons for every Edit Poly tool, grouped like the ribbon in 3ds Max. The buttons change with the sub-object level you're in (1–5).

### Status bar
- Lock Selection, Absolute / Offset
- X / Y / Z boxes for typing values
- the grid size
- a prompt line that says what the current tool expects next

### The Modifier List
- **A dropdown button, "Modifier List ▾",** opens **every modifier** in a tall list you can scroll, so you never have to search.
- **The list is grouped under headings:**

| Group | Modifiers |
|---|---|
| Mesh Editing | Edit Poly, Chamfer, Symmetry, Shell, Panel Groove, Edit Normals, ProOptimizer |
| Subdivision | TurboSmooth |
| Free-Form Deformers | FFD 2x2x2, FFD 3x3x3, FFD 4x4x4 |
| Parametric Deformers | Bend, Twist, Taper, Push |
| UV Coordinates | UVW Map, Unwrap UVW |
| Paint | VertexPaint |
| Spline → Mesh | Lathe, Extrude |
| Booleans | Boolean |
| Generators | GeoGen, Greeble, Terrain, Ocean |
| Architecture & Destruction | GhostTown, Medieval Architect, Battlefield / Ruin |

- **One click adds a modifier.** Hovering over one shows a single-line description of it.
- **The search box stays at the top, but you never have to use it.**
- **Modifiers that don't fit the selected object are dimmed.** Hovering says why (for example, Lathe needs a spline).
- **"Show Buttons" (modifier sets):** a grid of buttons for your favourite modifiers, always visible under the stack. You choose which ones appear.

### Tests
- Every modifier appears in exactly one group, and every list entry creates a modifier.
- Every action has a button or menu entry (the rule test).
- Every menu action runs.

---

## PR 2: Preferences / Settings

Open it from the **⚙ Preferences** toolbar button or **Edit → Preferences**. Settings are saved to `~/.config/minimax/preferences.ron` and kept between sessions.

| Tab | What you can change |
|---|---|
| **Theme** | Presets: Slate & Amber (today's), 3ds Max Classic Gray, Midnight Blue, Light, High Contrast. Or Custom: pick the accent, panel and text colours. Also UI size (90–150 %) and font size. |
| **Hotkeys** | Every action in one searchable list. Click a key, press a new one, and it's changed. Conflicts are flagged ("F is used by Zoom Extents, swap?"). Reset one key or all. Key sets: MiniMax default and 3ds Max standard. Export and import keymaps. |
| **Viewports** | Background colour or gradient, grid colour and spacing, selection colours, invert zoom, orbit and zoom speed |
| **General** | Autosave every N minutes, number of undo steps, units (m / cm / ft), show the tour on startup |
| **Rendering** | Default renderer, Blender location, default resolution |
| **Assets** | The three download packs (see PR 5) |

- **Under the hood:** theme colours and hotkeys move from fixed code into runtime tables, so changes apply instantly.
- **Everything shows your current keys:** menus, tooltips and the Help cheat sheet.
- **Tests:**
  - rebinding a key works and conflicts are caught
  - settings save and load correctly
  - every theme keeps text readable (a contrast check)

---

## PR 3: Click-and-drag creation (like 3ds Max)

| Object | Step 1: drag | Step 2: move, then click | Step 3: move, then click |
|---|---|---|---|
| Box | Base, corner to corner (length × width) | Height | — |
| Plane | Rectangle; releasing finishes it | — | — |
| Sphere | Radius; releasing finishes it | — | — |
| Cylinder | Radius | Height | — |
| Cone | Bottom radius | Height | Top radius |
| Torus | Main radius | Tube thickness | — |
| GeoGen (rock, crystal, cliff, stalagmite) | Base radius | Height | — |
| Terrain, Ocean | Area rectangle | Height / wave size | — |
| Planet | Radius | — | — |
| Settlements | Town area | — | — |
| Mech (gear, piston, hose) | Size | (length, where it applies) | — |

- **The view decides the plane.** Objects drawn in Top sit on the ground; in Front or Left they stand on that view's plane; in Perspective they land on the ground grid.
- **Snaps (`S`) work while dragging.**
- **Creating:**
  - right-click cancels a half-made object
  - `Ctrl+Z` removes a finished one in a single step
  - the tool stays active, so you can draw several objects in a row
- **Creation Method:** *Box / Cube* for the Box; *Edge / Center* for round shapes.
- **Right-clicking a Create button still places a default-size object at the origin.**
- **The status bar prompt** names the next step.
- **Tests:** simulated mouse presses and moves (no screen needed) check each object's final size and position.

---

## PR 4: Help

- **How to open it:** a ❓ toolbar button and the **Help** menu open the Help window. **F1** opens the page for the panel under the mouse.
- **Layout:** topics on the left and the page on the right, with search over all pages.
- **Step-by-step pages** with **screenshots of the real program**. Arrows and circles point at the buttons you need.
- **"Show me" buttons** open the panel a page talks about.
- **A Getting Started tour** (about 8 screens) runs on first launch and can be reopened from the Help menu.
- **The hotkey cheat sheet** always shows your current keys.
- **Credits** name Poly Haven, Kenney and Quaternius.

**Topics:**
1. Getting Started: the screen layout, moving around the viewports
2. Creating objects: drag to create, primitives, GeoGen, Worlds, Settlements, Mech
3. Selecting and transforming: Q W E R, snaps, cloning, Array, Mirror, Align
4. Editable Poly modelling: every tool with a before / after picture
5. Modifiers: one page each
6. Splines, Lathe and Extrude
7. Materials and the preset library
8. Painting
9. Lights, cameras, sky and deep space
10. Rendering: F9 / F10, Cycles through Blender
11. GhostTown, Medieval Architect and destruction
12. Mech assembly
13. Files: open, save, import, export
14. Preferences and hotkeys

**Screenshots** are made by a script that launches MiniMax, sets up each scene, opens the right panel and saves the picture. One command remakes them all when the interface changes. They're built into the program, so Help works offline.

**Open question:** I still need to check whether MiniMax can run on the cloud build machine with a software graphics driver. If it can't, the fallback is a small script run once on the PC to take all the screenshots.

---

## PR 5: Material library (~280 presets, procedural and photo)

### New procedural pattern maps
- **Bricks / Tiles:** bricks, tiles, paving, planks, roof tiles, cobbles.
- **Stripes:** road markings, corrugated metal.
- **Cracks / Voronoi:** cracked earth, worn asphalt, flagstones.
- **Grunge / Wear:** dirt, rust, scratches, worn edges.

### Categories

| Category | Examples | About |
|---|---|---|
| Ground & Terrain | Grass (fresh, dry, patchy), mud (dry, wet), gravel, pebbles, sand (beach, desert, wet), red clay, cracked earth, forest floor, dirt path, rocky ground, snow, slush, ice, moss, lava | 30 |
| Roads & Urban Ground | Asphalt (new, worn, cracked, wet, patched), concrete road, lane lines (white, yellow, dashed), crosswalk, sidewalk slabs, curb stone, brick paving, cobbles (modern, medieval), flagstone, manhole iron, tactile paving, parking lot, gravel road, dirt road | 30 |
| Walls & Facades | Bricks (red, yellow, white-washed, burnt, mossy), plaster, stucco, painted render, wood siding, half-timber, stone blocks, cinder block, tiled facade, glass curtain wall | 30 |
| Roofs | Terracotta, slate, thatch, wood shingles, corrugated tin (clean, rusty), tar paper, copper with green patina, zinc, solar panel | 15 |
| Metals | Steel, stainless, aluminium, copper, brass, bronze, iron, rust (light, heavy), chrome, silver, titanium, anodised colours, galvanised, diamond plate, worn painted metal | 30 |
| Wood | Oak, walnut, pine, cherry, mahogany, ebony, birch, bamboo, planks, weathered, charred, painted, plywood, driftwood | 20 |
| Stone & Concrete | Marble (white, black, green), granite, slate, sandstone, limestone, basalt, obsidian, concrete (smooth, rough, stained, cracked), terrazzo | 20 |
| Glass & Gems | Clear, frosted, tinted, dirty and broken glass, stained glass, ruby, emerald, sapphire, diamond, amethyst, amber, ice, water | 20 |
| Plastic & Rubber | Gloss, matte, soft-touch, tyre, silicone, toy plastic | 15 |
| Fabric & Leather | Cotton, denim, velvet, silk, wool, canvas, sacking, leather, suede | 12 |
| Nature & Organic | Skin, wax, bone, bark, leaves, hay, coal, ash | 15 |
| Glow / Emissive | Neon colours, LED, screen, lit night windows, fire, embers, molten metal | 15 |
| Toon & Stylised | Several cel styles, comic ink, pastel, flat colour, retro palette | 12 |
| Sci-Fi | Carbon fibre, hologram, force field, alien chitin, worn hull panel | 10 |

### Procedural and photo versions
- **Every** ground, road, wall, roof, wood, stone and metal preset has a procedural version that always works.
- **A 📷 Photo version** is added when the photo pack is installed.
- **The browser has a [ Procedural | Photo | Both ] filter.**
- **New "World / Triplanar" mapping** projects photos from three sides, so they tile on generated objects that have no UVs (GhostTown, Terrain, rocks). It works in the viewport, the built-in renderer and Cycles.

### The Material Editor (`M`)
- A preset browser with category tabs, search and preview spheres, plus a ★ Favourites list.
- The original 7 presets stay, and old scenes still load.

### Asset downloader (Preferences → Assets)
Each pack has its own ✓ checkbox, **[ Download ]** and **[ Remove ]** buttons, and a progress bar. Files go to `~/.local/share/minimax/assets/`, not into the git repository, so clones stay small. All three sources are CC0 (free, no credit required).

| Pack | Contents | Size (estimate) |
|---|---|---|
| 📷 Photo Textures (1K) | ~60 ground, road, wall, roof, wood, stone and metal materials from Poly Haven | ~150–250 MB |
| 🧊 Low-poly Models | Kenney and Quaternius: cars, trees, street furniture, medieval props | ~70–180 MB |
| 🗿 Realistic Models | Poly Haven: barrels, crates, furniture, rocks, plants | ~100–300 MB |

- **1K is the default resolution.** A 2K option is in Preferences.
- **Note:** these sites are blocked from the cloud build machine. While building this PR, I'll ask for them to be allowed (a one-click approval) so I can test with the real files.

### Tests
- Every preset is valid, names are unique, and every category has entries.
- Old scenes load.
- Triplanar mapping gives seamless coordinates.
- The downloader handles missing, partial and installed packs.

---

## PR 6: GhostTown variety

### New settlement types (in addition to today's four)

| Modern | Medieval / Historic |
|---|---|
| Grid Downtown *(today)* | Round Walled City *(today)* |
| Suburbs *(today)* | **Square Walled City:** corner towers, gatehouses, a central keep |
| **River City:** a river through the middle, stone / steel / draw bridges, quays, riverside paths | **River Town:** river, stone arch bridges, watermills, docks |
| **Harbour Town:** waterfront, piers, warehouses, cranes | **Castle & Town:** a hill castle above a town |
| **Industrial District:** factories, chimneys, rail yard, tanks | **Star Fort:** bastioned walls with a moat |
| **Old Town + Modern Sprawl:** a historic core with new buildings around it | Village *(today)*, **Crossroads Hamlet**, **Monastery** |

### New building types
- **Modern:**
  - glass skyscraper, office block, brick tenement, apartment block
  - shops with awnings, gas station, warehouse, factory with a sawtooth roof
  - parking garage, school, hospital, church, water tower, kiosk
- **Medieval:**
  - keep, guild hall, market hall, tavern, chapel
  - windmill, watermill, blacksmith, stable, granary, gatehouse, well house

### On / off toggles
- **Building types:** a checkbox per type.
- **Building parts:**
  - balconies, fire escapes, awnings, chimneys, dormers, shutters
  - rooftop clutter (AC units, tanks, antennas), signs, stairs and porches
- **Town features:**
  - walls, gates, river, bridges, trees, street lamps, cars and carts
  - market stalls, fences, road markings, parks

### Emissive lights toggle
`[✓] Emissive Lights` controls lit windows, street lamps, lanterns, neon signs and fires. With it off, windows are plain glass and nothing glows.

### Materials and props
- **The GhostTown material pack is rebuilt from the PR 5 library**, growing from 12 to about 24 IDs. New IDs include:
  - asphalt, painted lane lines
  - sidewalk slabs, curbs, cobbles
  - river water
  - brick, plaster and roof variations
  - rusted metal, broken glass
- **Material Style:** Procedural or Photo (Photo when the pack is installed).
- **Prop Style:** Generated, Low-poly models or Realistic models (each when its pack is installed). It falls back to generated props.
- **Buildings stay generated by code,** so they can be resized, toggled, given material IDs and destroyed.

### Tests
- Every settlement type and building type is valid, deterministic, depends on the seed, and stays inside its area.
- Toggles remove their parts.
- Rivers are crossed by bridges.
- With the emissive toggle off, no emissive IDs are used.

---

## PR 7: Much heavier destruction

**Modern:**
- **Collapse and lean:**
  - toppled buildings falling across streets, whole or snapped in two
  - leaning buildings
  - facades peeled off to show the floor slabs
  - pancaked floors
  - slabs hanging by their rebar
- **Twisted metal:**
  - twisted, bent and frayed rebar (curved, splayed bundles)
  - bent steel beams
- **Wall damage:** shell holes punched through walls, scorch marks.
- **Streets:**
  - burnt-out cars
  - fallen power poles and street lights
  - broken bridge spans dropped into the river
  - rubble mounds that pile up in the blast direction
  - tank traps, sandbags, barbed wire

**Medieval:**
- collapsed towers
- toppled wall sections
- gutted timber frames still standing
- fallen siege towers and broken trebuchets
- bridges with missing arches

**Ancient:**
- toppled colonnades
- half-buried statues
- roots splitting walls

**New controls:**
- Collapse Chance, Topple Chance
- Rebar Twist, Debris Spread
- Fire Damage
- Destruction Level presets: Light, Heavy, Warzone, Apocalyptic

**Tests:**
- Each mode's signature parts are present.
- Toppled buildings lie on the ground and stay inside the area.
- Rebar is bent (not straight).
- Zero intensity changes nothing.
- User Blast Zones still limit where damage happens.
