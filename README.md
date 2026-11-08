# MiniMax (Holly-3D)

A clean, 3ds Max-style polygon modeler written in Rust (Bevy 0.20 + bevy_egui 0.43).
See `CLAUDE.md` for the full design spec. **Phases 1 and 2 are implemented.**

## Build & run

```bash
cargo run --release            # empty scene
cargo run --release -- --demo  # starts with a Box, Sphere and Torus
cargo test                     # headless geometry / editor tests
```

Requires Rust ≥ 1.97.1. On Linux you need the usual X11/Vulkan runtime libraries
(`libxkbcommon-x11`, a Vulkan driver). The build disables Bevy's audio and gamepad
features, so ALSA and udev headers are not needed.

## What's in Phase 1 & 2

- **5-zone layout**: top toolbar, Scene Explorer (`H`), 2x2 viewports, Command Panel
  (Create / Modify / Pivot), status + transform type-in bar. Slate `#222327` theme with
  3ds Max amber `#E59C24`. Right-click any spinner to reset it.
- **Viewports** (Z-up): Top / Front / Left / Perspective, `Alt+W` maximize, `MMB` pan,
  `Alt+MMB` orbit around the selection, wheel or `Ctrl+Alt+MMB` zoom, `Z` zoom extents
  (`Shift+Z` all views), `G` grid, `F3` wireframe, `F4` edged faces, `Alt+X` see-through,
  `Alt+Q` isolate.
- **Transforms**: `Q/W/E/R` with a 3-axis Move/Rotate/Scale gizmo, World/Local, snaps
  (`S`, `A`), `Shift+Drag` → Clone Options (Copy / Instance / Reference, N copies),
  Abs/Offset type-in, `Ctrl+Z` / `Ctrl+Y`.
- **Parametric primitives** with live parameters: Box, Sphere, Cylinder, Cone, Plane, Torus.
- **Editable Poly / Edit Poly modifier**: face-corner mesh (`slotmap` arenas), modifier
  stack with enable toggles, Convert / Collapse, sub-object modes `1–5`, click + marquee
  selection (Ctrl add, Alt subtract), Grow/Shrink, Ring/Loop (`Alt+R`/`Alt+L`),
  By Vertex, Ignore Backfacing, Soft Selection.
- **Edit Poly tools**: Extrude, Bevel, Inset, Outline, Chamfer (amount / segments /
  tension / open), Connect (segments / pinch / slide), Weld, Target Weld, Collapse,
  Remove (`Ctrl` also removes orphaned vertices), Break, Split, Insert Vertex, Cut,
  Slice Plane + Slice + Split, QuickSlice, Cap, Bridge, Flip, MSmooth, Tessellate,
  Make Planar + X/Y/Z, Attach / Detach, Material IDs, 32 smoothing-group grid + Auto Smooth,
  Repeat Last, Shift+Drag border edges to extrude.
- **Floating caddies** (`[☐]`) with live preview, OK / Apply & Continue / Cancel, and the
  **right-click Quad Menu**.

Materials, rendering, UVs, FFD, splines and the remaining modifiers are later phases;
their buttons are visible but disabled.

## Code layout

`src/mesh` (topology, ops + tests), `src/primitives`, `src/modifiers`, `src/scene`,
`src/editor` (state, undo, selection, tools, drags), `src/viewport` (camera math, picking,
Bevy render sync, line overlays), `src/ui` (egui panels, rollouts, caddies). Every `.rs`
file stays under 350 lines.
