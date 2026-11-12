# Ilex 3D (Holly-3D)

_Formerly **MiniMax**. Scenes saved as `.mmax` (and `.mmaxpose` poses) still open; new
scenes save as `.ilex`, and the old `minimax` command still starts the app._

A clean, 3ds Max-style polygon modeler written in Rust (Bevy 0.20 + bevy_egui 0.43).
See `CLAUDE.md` for the full design spec and `PROJECT_STATUS.md` for progress.
**Phases 1–8C are implemented and merged.**

## Install

The app is built from source with Cargo. You need **Rust 1.97.1 or newer**, a
**Vulkan driver** and the **X11 libraries** (on a Wayland desktop it runs through
XWayland automatically). Bevy's audio and gamepad features are off, so ALSA and udev
headers are not needed.

### CachyOS / Arch Linux (fish shell)

```fish
# 1. Build tools, X11 / Vulkan libraries and the file-dialog portal
sudo pacman -S --needed base-devel git rustup pkgconf \
    vulkan-icd-loader libxkbcommon-x11 libx11 libxcursor libxrandr libxi \
    xdg-desktop-portal

# 2. The Vulkan driver for your GPU (pick one)
sudo pacman -S --needed vulkan-radeon   # AMD
sudo pacman -S --needed nvidia-utils    # NVIDIA
sudo pacman -S --needed vulkan-intel    # Intel

# 3. Rust (fish_add_path is permanent, so run it once)
rustup default stable
fish_add_path ~/.cargo/bin
rustc --version

# 4. Get the code
git clone https://github.com/hollywoodenspoon/Holly-3D.git
cd Holly-3D

# 5. Build and run (the first build compiles Bevy and takes several minutes)
cargo run --release
```

To install it as a command instead, run `cargo install --path .` and start it with
`ilex`. To update later: `git pull` and then `cargo install --path .` again.

On bash or zsh the steps are the same, except step 3: add
`export PATH="$HOME/.cargo/bin:$PATH"` to your shell's startup file instead of
running `fish_add_path`.

### As a pacman package with a local repository (CachyOS / Arch)

`packaging/` builds a real pacman package. It installs `/usr/bin/ilex` (plus a
`/usr/bin/minimax` symlink for old scripts), a menu entry, the white Holly tree
app icon, and the `application/x-ilex-scene` and legacy `application/x-minimax-scene`
file types, so double-clicking a `.ilex` or `.mmax` file in Dolphin or Nautilus opens
it in Ilex. The package replaces an installed `minimax` package on upgrade. Do steps 1–4 above first (packages, driver, Rust, clone), then:

```fish
packaging/publish-repo.sh
```

The script runs `makepkg -sf`, copies the package into
`~/.local/share/minimax-repo/` and updates the repository database. The first
time, it prints three lines to add to the end of `/etc/pacman.conf`:

```ini
[minimax]
SigLevel = Optional TrustAll
Server = file:///home/<you>/.local/share/minimax-repo
```

Then install with `sudo pacman -Syu ilex`. (The repository keeps its `[minimax]`
name so an existing `pacman.conf` entry keeps working.) To upgrade later, run `git pull`,
`packaging/publish-repo.sh` and `sudo pacman -Syu`.

- **Version:** the package version is `0.<commit count>.g<short hash>`, so every new
  commit counts as an upgrade. Only **committed** changes on the checked-out branch
  are built.
- **Extra flags** go to makepkg, e.g. `packaging/publish-repo.sh --nocheck` skips
  the tests.
- **CPU:** the package targets Zen 3 (`znver3`). On another CPU, run
  `env MINIMAX_TARGET_CPU=native packaging/publish-repo.sh`.
- **Permissions:** pacman 7 reads repositories as the `alpm` user. If your home
  folder is private, the script prints a warning with a `chmod o+x` fix (traverse
  only), or you can choose another folder with `MINIMAX_REPO_DIR`.
- **Build once without a repository:** `cd packaging; makepkg -si`.

### Build & run

```bash
cargo run --release            # empty scene
cargo run --release -- --demo  # starts with a Box, Sphere and Torus
cargo test --lib               # headless geometry / editor tests
```

### Troubleshooting

- **"Illegal instruction" crash:** `.cargo/config.toml` builds for a Ryzen 5000 (Zen 3)
  CPU (`target-cpu=znver3`). On an older AMD or an Intel CPU, change `znver3` to
  `native` in that file and rebuild.
- **Build runs out of memory:** the config runs 10 compile jobs at once, sized for
  16 GB of RAM. With less memory, build with fewer jobs: `cargo run --release -j 4`.
- **Cycles rendering:** the Cycles option in Render Setup (`F10`) needs Blender
  (`sudo pacman -S blender`). The built-in raytracer needs nothing extra.

## What's in it

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

- **Phase 3**: FFD 2x2x2 / 3x3x3 / 4x4x4 (control points + lattice), UVW Map (with gizmo)
  and Unwrap UVW (2D UV editor, Smart UV Unwrap).
- **Phase 4**: Compact Material Editor (`M`) with presets and ink outlines, vertex and
  texture painting, lights + Default Lights (`Ctrl+L`), Render Setup (`F10`) and the
  Rendered Frame Window (`F9`).
- **Phase 5**: splines (Line, Circle, Arc, NGon, Rectangle, Ellipse, Star, Helix) with
  rendering profiles, Lathe and Extrude, the Array dialog and Mirror, GeoGen
  (crystals, rocks, cliffs, stalagmites), Greeble, Chamfer, Symmetry, Shell, TurboSmooth,
  Bend / Twist / Taper / Push, Edit Normals (Break / Unify / Reset / Make Explicit,
  `E` to rotate), Boolean (`boolmesh`, ProBoolean-style operand list with Pick / Remove /
  Extract) and ProOptimizer (`meshopt`, keeps material / UV / colour / normal seams).

- **Phase 6**: file I/O (`.ilex` scenes, formerly `.mmax`, OBJ, glTF/GLB and FBX import and export),
  target and free cameras, the Align dialog, interactive gizmos for the deformers and
  the Symmetry mirror, spline Booleans and Refine, 3D snaps, selection region shapes,
  HDRI environments and Cycles rendering through Blender.
- **Phase 7**: bone-free mech assembly (joints, FK / IK drag, Mirror Mech Limb, Contact
  Sets, Motion Links), Gear / Piston / Hose primitives and Panel Groove.
- **Phase 8A**: Multi/Sub-Object materials, emissive area lights, AO / GI / denoiser /
  bloom / depth of field, and a dynamic sky with clouds and deep space.
- **Phase 8B**: procedural worlds under `Create -> Geometry -> Worlds`: biome Terrain
  with erosion, a Gerstner Ocean with crest foam, and Planet Studio.
- **Phase 8C**: GhostTown settlements under `Create -> Geometry -> Settlement` (Modern
  Downtown, Suburbs, Medieval Walled City, Village), the scale-aware Medieval Architect
  modifier, and Battlefield / Ruin destruction (Modern, Medieval Siege, Ancient Ruins)
  with movable Blast Zones.

## Code layout

`src/mesh` (topology, ops + tests), `src/primitives`, `src/modifiers`, `src/scene`,
`src/generators` (worlds and settlements), `src/editor` (state, undo, selection, tools,
drags), `src/viewport` (camera math, picking, Bevy render sync, line overlays), `src/ui`
(egui panels, rollouts, caddies). Every `.rs` file stays under 350 lines.
