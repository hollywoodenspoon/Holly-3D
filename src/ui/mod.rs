//! egui front-end: header row, viewports, the right Properties column,
//! the status bar and floating windows.

pub mod action_ui;
pub mod actions;
pub mod align_dialog;
pub mod buildings_browser;
pub mod character_studio;
pub mod array_dialog;
pub mod caddy;
pub mod clone_dialog;
pub mod crash_dialog;
pub mod env_dialog;
pub mod explorer;
pub mod face_cloner;
pub mod file_menu;
pub mod fonts;
pub mod help;
pub mod logo;
pub mod header;
pub mod hotkeys;
pub mod material_editor;
pub mod menubar;
pub mod mirror_dialog;
pub mod paintbox;
pub mod popout;
pub mod popout_windows;
pub mod prefs;
pub mod properties;
pub mod quad_menu;
pub mod render_fx;
pub mod render_lens;
pub mod render_sheet;
pub mod render_grading;
pub mod render_lightmix;
pub mod render_path;
pub mod render_setup;
pub mod render_window;
pub mod ribbon;
pub mod rollouts;
pub mod sky_dialog;
pub mod weather_rollout;
pub mod volume_dialog;
pub mod snaps;
pub mod status_bar;
pub mod theme;
pub mod tool_windows;
pub mod uv_editor;
pub mod viewport_ui;
pub mod widgets;

use crate::editor::Editor;
use bevy::prelude::*;
use bevy_egui::egui::{self, LayerId, Ui, UiBuilder};
use bevy_egui::{EguiContexts, EguiPrimaryContextPass};
use popout::{Dock, Host};
use std::collections::{HashMap, HashSet};

type Themed = (crate::prefs::ThemePrefs, crate::prefs::ViewportPrefs);

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>().add_systems(EguiPrimaryContextPass, ui_system);
        popout_windows::register(app);
    }
}

/// Transient UI-only state (not part of undo).
#[derive(Resource, Default)]
pub struct UiState {
    /// Theme settings last applied to egui.
    themed: Option<Themed>,
    /// The same for each pop-out window's egui context.
    pop_themed: HashMap<Dock, Themed>,
    /// Dialogs that are open and popped out: each needs an OS window.
    pub pop_want: HashSet<Dock>,
    pub prefs: prefs::PrefsUi,
    pub viewport: viewport_ui::ViewportUiState,
    pub modifier_list_open: bool,
    pub uv: uv_editor::UvEditorState,
    pub mat: material_editor::MatEditorUi,
    pub rfw: render_window::RfwState,
    pub help: help::HelpUi,
    pub face: face_cloner::FaceUi,
}

fn ui_system(mut contexts: EguiContexts, mut ed: ResMut<Editor>, mut st: ResMut<UiState>) -> Result {
    let ctx = contexts.ctx_mut()?.clone();
    ensure_theme(&ctx, &ed, &mut st.themed);
    let ed = &mut *ed;
    let st = &mut *st;
    hotkeys::handle(&ctx, ed);
    popout::begin_main(&ed.prefs.layout.popouts);
    for d in Dock::ALL {
        if popout::arriving(d, Host::Main) {
            clear_dock_textures(st, d);
        }
    }
    let mut root =
        Ui::new(ctx.clone(), "minimax_root".into(), UiBuilder::new().layer_id(LayerId::background()).max_rect(ctx.viewport_rect()));
    let screen = ctx.viewport_rect();
    let mut top = header::show(&mut root, ed).max.y;
    if ed.iface.ribbon {
        top = ribbon::show(&mut root, ed).max.y;
    }
    let bottom = status_bar::show(&mut root, ed).min.y;
    let right = properties::show(&mut root, ed, st).min.x.clamp(screen.min.x, screen.max.x);
    let area = egui::Rect::from_min_max(egui::pos2(screen.min.x, top), egui::pos2(right, bottom));
    viewport_ui::show(&mut root, ed, &mut st.viewport, area);
    caddy::show(&ctx, ed);
    clone_dialog::show(&ctx, ed);
    array_dialog::show(&ctx, ed);
    mirror_dialog::show(&ctx, ed);
    align_dialog::show(&ctx, ed);
    quad_menu::show(&ctx, ed);
    tool_windows::show(&ctx, ed);
    prefs::show(&ctx, ed, &mut st.prefs);
    ed.autosave_tick(ctx.input(|i| i.time));
    ed.poll_downloads();
    uv_editor::show(&ctx, ed, &mut st.uv);
    material_editor::show(&ctx, ed, &mut st.mat);
    material_editor::library::show(&ctx, ed, &mut st.mat.library);
    buildings_browser::show(&ctx, ed);
    character_studio::show(&ctx, ed);
    face_cloner::show(&ctx, ed, &mut st.face);
    paintbox::show(&ctx, ed);
    render_window::show(&ctx, ed, &mut st.rfw);
    help::show(&ctx, ed, &mut st.help);
    crash_dialog::show(&ctx, ed);
    let (saved, want) = popout::end_main();
    if let Some(v) = saved {
        ed.prefs.layout.popouts = v;
        ed.iface.layout_dirty = true;
    }
    st.pop_want = want;
    crate::diagnostics::state::tick(ed, ctx.input(|i| i.time));
    ed.update_caddy_preview();
    ed.scene.solve_assembly();
    ed.scene.update_hoses();
    Ok(())
}

/// Installs the font and applies the theme to `ctx` when they changed.
fn ensure_theme(ctx: &egui::Context, ed: &Editor, themed: &mut Option<Themed>) {
    let want = (ed.prefs.theme.clone(), ed.prefs.viewport.clone());
    if fonts::installed(ctx) != Some(want.0.font) {
        // egui loads new fonts at the start of the next frame; the theme (which
        // names the bold face) is applied then.
        fonts::install(ctx, want.0.font);
        *themed = None;
        ctx.request_repaint();
    } else if themed.as_ref() != Some(&want) {
        let first = themed.is_none();
        theme::apply(ctx, &want.0, &want.1);
        *themed = Some(want);
        // egui's debug options only exist in debug builds.
        #[cfg(debug_assertions)]
        if first && std::env::var("MINIMAX_DEBUG_UI").is_ok() {
            ctx.global_style_mut(|s| s.debug.show_expand_width = true);
        }
        #[cfg(not(debug_assertions))]
        let _ = first;
    }
}

/// Draws one pop-out-capable dialog (used by its pop-out window's pass).
fn draw_dock(ctx: &egui::Context, ed: &mut Editor, st: &mut UiState, d: Dock) {
    match d {
        Dock::MaterialEditor => material_editor::show(ctx, ed, &mut st.mat),
        Dock::MaterialLibrary => material_editor::library::show(ctx, ed, &mut st.mat.library),
        Dock::FrameWindow => render_window::show(ctx, ed, &mut st.rfw),
        Dock::UvEditor => uv_editor::show(ctx, ed, &mut st.uv),
        Dock::CharacterStudio => character_studio::show(ctx, ed),
        Dock::FaceCloner => face_cloner::show(ctx, ed, &mut st.face),
        Dock::BuildingsBrowser => buildings_browser::show(ctx, ed),
        Dock::PaintBox => paintbox::show(ctx, ed),
        Dock::Help => help::show(ctx, ed, &mut st.help),
        Dock::Preferences => prefs::show(ctx, ed, &mut st.prefs),
    }
}

/// Textures belong to the egui context that loaded them: drop a dialog's
/// cached textures when it moves to another window.
fn clear_dock_textures(st: &mut UiState, d: Dock) {
    match d {
        Dock::MaterialEditor => st.mat.clear_textures(),
        Dock::MaterialLibrary => st.mat.library.clear_textures(),
        Dock::FrameWindow => st.rfw.clear_textures(),
        Dock::FaceCloner => st.face.pins.clear_textures(),
        Dock::Help => st.help.textures.clear(),
        _ => {}
    }
}
